//! Named-pipe transport.
//!
//! - The pipe `\\.\pipe\fframes-studio-<random>` is created with
//!   `FILE_FLAG_FIRST_PIPE_INSTANCE` (nobody else holds the name), `PIPE_REJECT_REMOTE_CLIENTS`
//!   and a protected DACL that grants only the current user's SID, who also owns it. Only
//!   the owner may add instances, so the name cannot be served by anyone else either.
//! - After the hello line, the server reads the client's token through
//!   `ImpersonateNamedPipeClient` (identification level: the client opens the pipe with
//!   `SECURITY_IDENTIFICATION`) and refuses another user, a restricted token and an
//!   AppContainer. The client checks that the server process runs as the current user.
//! - The runtime directory and the capability files are created with the same kind of
//!   descriptor (owner and only ACE: the current user) and verified on the opened handle.
//! - An exclusive, delete-on-close `broker.lock` in the runtime directory marks a live
//!   broker: the pipe name is random, so it cannot tell a second broker apart.
//!
//! Streams use overlapped I/O, so reads and writes honour their timeouts and another
//! thread can cut them off ([`close`]).
use super::Accepted;
use crate::agent_tools::broker::BrokerError;
use std::{
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    ptr,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_INSUFFICIENT_BUFFER, ERROR_IO_PENDING, ERROR_NO_DATA,
        ERROR_OPERATION_ABORTED, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, ERROR_PIPE_NOT_CONNECTED,
        ERROR_SHARING_VIOLATION, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LocalFree,
        WAIT_TIMEOUT,
    },
    Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL,
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            GetSecurityInfo, SDDL_REVISION_1, SE_FILE_OBJECT, SE_OBJECT_TYPE,
        },
        DACL_SECURITY_INFORMATION, GetAce, GetLengthSid, GetTokenInformation, IsTokenRestricted,
        OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, RevertToSelf, SECURITY_ATTRIBUTES,
        TOKEN_QUERY, TOKEN_USER, TokenIsAppContainer, TokenUser,
    },
    Storage::FileSystem::{
        CREATE_NEW, CreateDirectoryW, CreateFileW, DELETE, FILE_ATTRIBUTE_NORMAL,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_DELETE_ON_CLOSE,
        FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_OVERLAPPED,
        FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES, OPEN_EXISTING,
        PIPE_ACCESS_DUPLEX, READ_CONTROL, ReadFile, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
        WriteFile,
    },
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeServerProcessId,
            ImpersonateNamedPipeClient, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT, WaitNamedPipeW,
        },
        SystemServices::{ACCESS_ALLOWED_ACE_TYPE, ACCESS_DENIED_ACE_TYPE},
        Threading::{
            CreateEventW, GetCurrentThread, INFINITE, OpenProcess, OpenProcessToken,
            OpenThreadToken, PROCESS_QUERY_LIMITED_INFORMATION, WaitForSingleObject,
        },
    },
};

const PIPE_PREFIX: &str = r"\\.\pipe\";
const PIPE_BUFFER: u32 = 64 * 1024;
/// Deny ACE kinds (plain, object, callback, callback object) only ever take access away.
const DENY_ACE_TYPES: [u32; 4] = [ACCESS_DENIED_ACE_TYPE, 6, 10, 12];

fn io_error(context: &'static str) -> impl FnOnce(io::Error) -> BrokerError {
    move |source| BrokerError::Io { context, source }
}

fn insecure(path: &Path, reason: impl Into<String>) -> BrokerError {
    BrokerError::InsecureRuntimeDir {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain(Some(0)).collect()
}

fn last_error() -> io::Error {
    io::Error::last_os_error()
}

fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(last_error());
    }
    // SAFETY: a valid handle we just created and nobody else owns.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

/// Memory the system allocated with `LocalAlloc`, freed on drop.
struct LocalMemory(*mut core::ffi::c_void);

impl Drop for LocalMemory {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: allocated by the system call that returned it, freed once.
            unsafe { LocalFree(self.0) };
        }
    }
}

/// The SID bytes `sid` points to.
///
/// # Safety
/// `sid` must point to a valid SID.
unsafe fn sid_bytes(sid: PSID) -> Vec<u8> {
    // SAFETY: the caller guarantees a valid SID of `GetLengthSid` bytes.
    unsafe {
        let length = GetLengthSid(sid) as usize;
        std::slice::from_raw_parts(sid.cast::<u8>(), length).to_vec()
    }
}

fn sid_string(sid: &[u8]) -> String {
    let mut text: *mut u16 = ptr::null_mut();
    // SAFETY: `sid` holds a valid SID; the string is freed by `LocalMemory`.
    if unsafe { ConvertSidToStringSidW(sid.as_ptr() as PSID, &mut text) } == 0 {
        return "<unreadable SID>".into();
    }
    let _free = LocalMemory(text.cast());
    // SAFETY: a NUL-terminated wide string from the system.
    let length = (0..).take_while(|&i| unsafe { *text.add(i) } != 0).count();
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, length) })
}

/// The user SID of a token opened with `TOKEN_QUERY`.
fn token_user(token: HANDLE) -> io::Result<Vec<u8>> {
    let mut length = 0u32;
    // SAFETY: a size query with no buffer.
    unsafe { GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut length) };
    // SAFETY: GetLastError has no preconditions.
    if unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
        return Err(last_error());
    }
    // u64 words keep the buffer aligned for TOKEN_USER.
    let mut buffer = vec![0u64; (length as usize).div_ceil(8)];
    // SAFETY: `buffer` holds `length` bytes.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            length,
            &mut length,
        )
    };
    if ok == 0 {
        return Err(last_error());
    }
    // SAFETY: the call filled a TOKEN_USER whose SID points into `buffer`.
    Ok(unsafe { sid_bytes((*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid) })
}

/// The SID of the user this process runs as.
fn current_user() -> io::Result<&'static [u8]> {
    static USER: OnceLock<Vec<u8>> = OnceLock::new();
    if let Some(user) = USER.get() {
        return Ok(user);
    }
    let mut token: HANDLE = ptr::null_mut();
    // SAFETY: the pseudo handle of this process and a valid out-parameter.
    if unsafe {
        OpenProcessToken(
            windows_sys::Win32::System::Threading::GetCurrentProcess(),
            TOKEN_QUERY,
            &mut token,
        )
    } == 0
    {
        return Err(last_error());
    }
    let token = owned(token)?;
    let user = token_user(token.as_raw_handle())?;
    Ok(USER.get_or_init(|| user))
}

/// Whether a token is the current user's, unrestricted and outside an AppContainer.
pub(crate) fn token_is_current_user(token: HANDLE) -> bool {
    let Ok(user) = current_user() else {
        return false;
    };
    if token_user(token).ok().as_deref() != Some(user) {
        return false;
    }
    // SAFETY: a token handle opened with TOKEN_QUERY.
    if unsafe { IsTokenRestricted(token) } != 0 {
        return false;
    }
    let mut app_container = 0u32;
    let mut length = 0u32;
    // SAFETY: a u32 out-parameter of the size passed.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenIsAppContainer,
            (&raw mut app_container).cast(),
            4,
            &mut length,
        )
    };
    ok != 0 && app_container == 0
}

/// A self-relative security descriptor: owner and only ACE the current user.
struct PrivateDescriptor(LocalMemory);

impl PrivateDescriptor {
    /// `rights` is an SDDL right ("GA", "FA"); `inherit` adds `OICI` for directories.
    fn new(rights: &str, inherit: bool) -> io::Result<Self> {
        let sid = sid_string(current_user()?);
        let flags = if inherit { "OICI" } else { "" };
        let sddl = format!("O:{sid}D:P(A;{flags};{rights};;;{sid})");
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
        // SAFETY: a NUL-terminated SDDL string and a valid out-parameter.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide(OsStr::new(&sddl)).as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(last_error());
        }
        Ok(Self(LocalMemory(descriptor)))
    }

    fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.0.0,
            bInheritHandle: 0,
        }
    }
}

/// Why the object behind `handle` is not private to the current user, if it is not: it
/// must be owned by the current user, and every ACE that grants access must name them.
pub(crate) fn not_private(handle: HANDLE, kind: SE_OBJECT_TYPE) -> Option<String> {
    let user = match current_user() {
        Ok(user) => user,
        Err(error) => return Some(format!("cannot read the current user: {error}")),
    };
    let mut owner: PSID = ptr::null_mut();
    let mut dacl: *mut ACL = ptr::null_mut();
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    // SAFETY: valid out-parameters; the descriptor is freed by `LocalMemory`.
    let status = unsafe {
        GetSecurityInfo(
            handle,
            kind,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut dacl,
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if status != 0 {
        return Some(format!(
            "cannot read its security descriptor: {}",
            io::Error::from_raw_os_error(status as i32)
        ));
    }
    let _free = LocalMemory(descriptor);
    // SAFETY: `owner` points into the descriptor, which lives until the end.
    if owner.is_null() || unsafe { sid_bytes(owner) } != user {
        return Some("is not owned by the current user".into());
    }
    if dacl.is_null() {
        return Some("has no DACL, so everyone can open it".into());
    }
    // SAFETY: `dacl` points into the descriptor.
    let count = unsafe { (*dacl).AceCount };
    for index in 0..u32::from(count) {
        let mut ace: *mut core::ffi::c_void = ptr::null_mut();
        // SAFETY: `index` is below the ACE count.
        if unsafe { GetAce(dacl, index, &mut ace) } == 0 {
            return Some("has an unreadable access entry".into());
        }
        // SAFETY: every ACE starts with an ACE_HEADER.
        let kind = u32::from(unsafe { (*ace.cast::<ACE_HEADER>()).AceType });
        if kind == ACCESS_ALLOWED_ACE_TYPE {
            // SAFETY: an ACCESS_ALLOWED_ACE whose SID starts at `SidStart`.
            let sid = unsafe {
                sid_bytes((&raw mut (*ace.cast::<ACCESS_ALLOWED_ACE>()).SidStart).cast())
            };
            if sid != user {
                return Some(format!("grants access to {}", sid_string(&sid)));
            }
        } else if !DENY_ACE_TYPES.contains(&kind) {
            return Some(format!(
                "has an access entry of type {kind} that may grant access"
            ));
        }
    }
    None
}

fn verify_private_dir(path: &Path) -> Result<(), BrokerError> {
    let dir = OpenOptions::new()
        .access_mode(READ_CONTROL | FILE_READ_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(io_error("inspect runtime dir"))?;
    let meta = dir.metadata().map_err(io_error("inspect runtime dir"))?;
    if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(insecure(path, "is a symlink or junction"));
    }
    if !meta.is_dir() {
        return Err(insecure(path, "is not a directory"));
    }
    match not_private(dir.as_raw_handle(), SE_FILE_OBJECT) {
        Some(reason) => Err(insecure(path, reason)),
        None => Ok(()),
    }
}

/// Creates the runtime directory private to the current user when it is missing (its
/// parents get default permissions), verifies it and returns its absolute path. The path
/// is not canonicalized: that would add a `\\?\` prefix to everything built from it.
pub fn prepare_runtime_dir(dir: &Path) -> Result<PathBuf, BrokerError> {
    match fs::symlink_metadata(dir) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = dir.parent() {
                fs::create_dir_all(parent).map_err(io_error("create runtime dir"))?;
            }
            let descriptor =
                PrivateDescriptor::new("FA", true).map_err(io_error("create runtime dir"))?;
            let attributes = descriptor.attributes();
            // SAFETY: a NUL-terminated path and a valid descriptor.
            if unsafe { CreateDirectoryW(wide(dir.as_os_str()).as_ptr(), &attributes) } == 0 {
                return Err(io_error("create runtime dir")(last_error()));
            }
        }
        Err(error) => return Err(io_error("inspect runtime dir")(error)),
    }
    verify_private_dir(dir)?;
    std::path::absolute(dir).map_err(io_error("resolve runtime dir"))
}

/// A previous run's capability files are dead: its pipe is gone with its process.
fn clear_stale(runtime_dir: &Path) -> Result<(), BrokerError> {
    for entry in fs::read_dir(runtime_dir).map_err(io_error("list runtime dir"))? {
        let entry = entry.map_err(io_error("list runtime dir"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("cap-") && name.ends_with(".json") {
            let path = entry.path();
            if let Ok(meta) = fs::symlink_metadata(&path)
                && meta.is_file()
                && meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
            {
                let _ = fs::remove_file(path);
            }
        }
    }
    Ok(())
}

/// One pipe instance waiting for a client.
struct Listening {
    pipe: Option<OwnedHandle>,
    event: OwnedHandle,
    /// Boxed: the system writes to it until the connect completes or is cancelled.
    overlapped: Box<OVERLAPPED>,
    /// The connect is no longer in flight (a client connected or it failed).
    settled: bool,
}

// SAFETY: the raw pointers inside OVERLAPPED are the event handle, owned by this value.
unsafe impl Send for Listening {}

impl Listening {
    fn start(name: &[u16], first: bool) -> io::Result<Self> {
        let descriptor = PrivateDescriptor::new("GA", false)?;
        let attributes = descriptor.attributes();
        let first = if first {
            FILE_FLAG_FIRST_PIPE_INSTANCE
        } else {
            0
        };
        // SAFETY: a NUL-terminated name and a valid descriptor.
        let pipe = owned(unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | first,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_UNLIMITED_INSTANCES,
                PIPE_BUFFER,
                PIPE_BUFFER,
                0,
                &attributes,
            )
        })?;
        // SAFETY: a manual-reset, initially unsignalled, unnamed event.
        let event = owned(unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) })?;
        // SAFETY: an all-zero OVERLAPPED is valid.
        let mut overlapped: Box<OVERLAPPED> = Box::new(unsafe { std::mem::zeroed() });
        overlapped.hEvent = event.as_raw_handle();
        let mut listening = Self {
            pipe: Some(pipe),
            event,
            overlapped,
            settled: false,
        };
        let handle = listening.handle();
        // SAFETY: the pipe and the boxed OVERLAPPED outlive the operation (see Drop).
        if unsafe { ConnectNamedPipe(handle, &mut *listening.overlapped) } == 0 {
            // SAFETY: GetLastError has no preconditions.
            match unsafe { GetLastError() } {
                ERROR_IO_PENDING => {}
                ERROR_PIPE_CONNECTED => listening.settled = true,
                code => {
                    listening.settled = true;
                    return Err(io::Error::from_raw_os_error(code as i32));
                }
            }
        } else {
            listening.settled = true;
        }
        Ok(listening)
    }

    fn handle(&self) -> HANDLE {
        self.pipe
            .as_ref()
            .map_or(ptr::null_mut(), |pipe| pipe.as_raw_handle())
    }

    /// Waits up to `tick`: `Some(true)` a client connected, `Some(false)` one came and
    /// went (the instance is spent), `None` nothing yet.
    fn wait(&mut self, tick: Duration) -> Option<bool> {
        if self.settled {
            return Some(true);
        }
        // SAFETY: a valid event handle.
        if unsafe { WaitForSingleObject(self.event.as_raw_handle(), millis(tick)) } == WAIT_TIMEOUT
        {
            return None;
        }
        self.settled = true;
        let mut transferred = 0u32;
        // SAFETY: the operation completed; the OVERLAPPED is the one it used.
        let ok =
            unsafe { GetOverlappedResult(self.handle(), &*self.overlapped, &mut transferred, 0) };
        Some(ok != 0)
    }
}

impl Drop for Listening {
    fn drop(&mut self) {
        if !self.settled {
            let handle = self.handle();
            let mut transferred = 0u32;
            // SAFETY: cancel the connect and wait until the system no longer touches the
            // OVERLAPPED before it is freed.
            unsafe {
                CancelIoEx(handle, &*self.overlapped);
                GetOverlappedResult(handle, &*self.overlapped, &mut transferred, 1);
            }
        }
    }
}

/// Milliseconds for a wait, rounded up so a short timeout never becomes "do not wait".
fn millis(duration: Duration) -> u32 {
    let millis = duration.as_nanos().div_ceil(1_000_000);
    u32::try_from(millis)
        .unwrap_or(INFINITE - 1)
        .min(INFINITE - 1)
}

/// The broker's named pipe. One instance always listens, so the name never disappears
/// between two clients.
pub struct Listener {
    name: Vec<u16>,
    endpoint: PathBuf,
    listening: Mutex<Option<Listening>>,
    /// Exclusive and deleted on close: marks the live broker of this runtime directory.
    _lock: File,
}

impl Listener {
    /// Creates the pipe for a runtime directory returned by [`prepare_runtime_dir`],
    /// after taking its lock and clearing a previous run's capability files.
    pub fn bind(runtime_dir: &Path) -> Result<Self, BrokerError> {
        let lock_path = runtime_dir.join("broker.lock");
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .access_mode(FILE_GENERIC_WRITE | DELETE)
            .share_mode(0)
            .custom_flags(FILE_FLAG_DELETE_ON_CLOSE | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&lock_path)
            .map_err(|error| {
                if error.raw_os_error() == Some(ERROR_SHARING_VIOLATION as i32) {
                    BrokerError::AlreadyRunning(lock_path.clone())
                } else {
                    io_error("lock runtime dir")(error)
                }
            })?;
        clear_stale(runtime_dir)?;
        let endpoint = PathBuf::from(format!(
            "{PIPE_PREFIX}fframes-studio-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let name = wide(endpoint.as_os_str());
        let first = Listening::start(&name, true).map_err(io_error("create pipe"))?;
        Ok(Self {
            name,
            endpoint,
            listening: Mutex::new(Some(first)),
            _lock: lock,
        })
    }

    pub fn endpoint(&self) -> &Path {
        &self.endpoint
    }

    /// Waits up to `tick` for a client. Its identity is checked after its first line
    /// ([`verify_peer`]): a pipe client cannot be impersonated before it sent anything.
    pub fn accept(&self, tick: Duration) -> Accepted {
        let mut slot = self
            .listening
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot.is_none() {
            match Listening::start(&self.name, false) {
                Ok(listening) => *slot = Some(listening),
                Err(_) => {
                    drop(slot);
                    thread::sleep(Duration::from_millis(10));
                    return Accepted::Idle;
                }
            }
        }
        let listening = slot.as_mut().expect("an instance listens");
        match listening.wait(tick) {
            None => Accepted::Idle,
            Some(false) => {
                *slot = None;
                Accepted::Idle
            }
            Some(true) => {
                let mut connected = slot.take().expect("an instance listens");
                *slot = Listening::start(&self.name, false).ok();
                let pipe = connected.pipe.take().expect("a connected pipe");
                Accepted::Peer(Stream::new(pipe, true))
            }
        }
    }
}

/// Nothing to remove: the pipe disappears with its last handle and the lock file is
/// deleted when it closes.
pub fn remove_endpoint(_endpoint: &Path) {}

/// The client's token, read through identification-level impersonation, must be the
/// current user's, unrestricted and outside an AppContainer.
pub fn verify_peer(stream: &Stream) -> bool {
    let handle = stream.pipe.handle.as_raw_handle();
    // SAFETY: a connected server end of a pipe.
    if unsafe { ImpersonateNamedPipeClient(handle) } == 0 {
        return false;
    }
    let mut token: HANDLE = ptr::null_mut();
    // SAFETY: this thread's impersonation token, opened with the process's own identity.
    let opened = unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut token) };
    // SAFETY: ends the impersonation started above.
    if unsafe { RevertToSelf() } == 0 {
        // Never go on running as the client.
        std::process::abort();
    }
    if opened == 0 {
        return false;
    }
    owned(token).is_ok_and(|token| token_is_current_user(token.as_raw_handle()))
}

/// Cuts a stream off from another thread: in-flight I/O is cancelled and the next call
/// fails at once; a server end is also disconnected, so the client sees the close.
pub fn close(stream: &Stream) {
    stream.shutdown(std::net::Shutdown::Both).ok();
}

/// Ends a connection after its replies were written. Only the handle is closed:
/// disconnecting would discard replies the client has not read yet.
pub fn finish(stream: Stream) {
    drop(stream);
}

/// Writes a new file private to the current user, never through a reparse point and
/// never replacing a file.
pub fn create_private_file(path: &Path, body: &[u8]) -> io::Result<()> {
    let descriptor = PrivateDescriptor::new("FA", false)?;
    let attributes = descriptor.attributes();
    // SAFETY: a NUL-terminated path and a valid descriptor.
    let handle = owned(unsafe {
        CreateFileW(
            wide(path.as_os_str()).as_ptr(),
            FILE_GENERIC_WRITE,
            0,
            &attributes,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            ptr::null_mut(),
        )
    })?;
    let mut file = File::from(handle);
    file.write_all(body).and_then(|()| file.flush())
}

/// Opens a file only the current user can read: not a reparse point, a regular file
/// owned by the current user whose DACL grants nobody else. The checks run on the opened
/// handle, so nothing can be swapped between check and read.
pub fn open_private_file(path: &Path) -> Result<File, String> {
    let file = OpenOptions::new()
        .access_mode(FILE_GENERIC_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let meta = file
        .metadata()
        .map_err(|error| format!("cannot stat {}: {error}", path.display()))?;
    if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(format!(
            "{} is a symlink or junction (refused)",
            path.display()
        ));
    }
    if !meta.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    if let Some(reason) = not_private(file.as_raw_handle(), SE_FILE_OBJECT) {
        return Err(format!("{} {reason}", path.display()));
    }
    Ok(file)
}

/// Connects to a broker pipe, waiting while every instance is busy, and checks that the
/// server runs as the current user. The client is only identifiable, never impersonable.
pub fn connect(endpoint: &Path, timeout: Duration) -> io::Result<Stream> {
    let is_pipe = endpoint
        .to_str()
        .and_then(|text| text.get(..PIPE_PREFIX.len()))
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(PIPE_PREFIX));
    if !is_pipe {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the broker endpoint is not a local named pipe",
        ));
    }
    let name = wide(endpoint.as_os_str());
    let deadline = Instant::now() + timeout;
    let pipe = loop {
        // SAFETY: a NUL-terminated name; no security attributes or template.
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                0,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                ptr::null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            break owned(handle)?;
        }
        let error = last_error();
        if error.raw_os_error() != Some(ERROR_PIPE_BUSY as i32) {
            return Err(error);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out"));
        }
        // SAFETY: a NUL-terminated name.
        unsafe { WaitNamedPipeW(name.as_ptr(), millis(remaining)) };
    };
    if !server_is_current_user(pipe.as_raw_handle()) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the broker pipe is not served by the current user",
        ));
    }
    Ok(Stream::new(pipe, false))
}

fn server_is_current_user(pipe: HANDLE) -> bool {
    let mut pid = 0u32;
    // SAFETY: a client end of a pipe and a valid out-parameter.
    if unsafe { GetNamedPipeServerProcessId(pipe, &mut pid) } == 0 {
        return false;
    }
    // SAFETY: plain calls with valid out-parameters; the handles are owned below.
    let Ok(process) = owned(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) })
    else {
        return false;
    };
    let mut token: HANDLE = ptr::null_mut();
    if unsafe { OpenProcessToken(process.as_raw_handle(), TOKEN_QUERY, &mut token) } == 0 {
        return false;
    }
    let Ok(token) = owned(token) else {
        return false;
    };
    let user = current_user().ok();
    token_user(token.as_raw_handle()).ok().as_deref() == user
}

struct Pipe {
    handle: OwnedHandle,
    server: bool,
    closed: AtomicBool,
}

/// One end of a connected pipe. Clones share the handle; timeouts are per clone.
pub struct Stream {
    pipe: Arc<Pipe>,
    /// Nanoseconds; 0 means no timeout.
    read_timeout: AtomicU64,
    write_timeout: AtomicU64,
}

// SAFETY: HANDLE is a kernel handle, usable from any thread.
unsafe impl Send for Pipe {}
unsafe impl Sync for Pipe {}

impl std::fmt::Debug for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stream")
            .field("server", &self.pipe.server)
            .finish_non_exhaustive()
    }
}

fn timeout_nanos(timeout: Option<Duration>) -> io::Result<u64> {
    match timeout {
        None => Ok(0),
        Some(duration) if duration.is_zero() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot set a 0 duration timeout",
        )),
        Some(duration) => Ok(u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)),
    }
}

impl Stream {
    fn new(handle: OwnedHandle, server: bool) -> Self {
        Self {
            pipe: Arc::new(Pipe {
                handle,
                server,
                closed: AtomicBool::new(false),
            }),
            read_timeout: AtomicU64::new(0),
            write_timeout: AtomicU64::new(0),
        }
    }

    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.read_timeout
            .store(timeout_nanos(timeout)?, Ordering::SeqCst);
        Ok(())
    }

    pub fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.write_timeout
            .store(timeout_nanos(timeout)?, Ordering::SeqCst);
        Ok(())
    }

    pub fn try_clone(&self) -> io::Result<Stream> {
        Ok(Stream {
            pipe: self.pipe.clone(),
            read_timeout: AtomicU64::new(self.read_timeout.load(Ordering::SeqCst)),
            write_timeout: AtomicU64::new(self.write_timeout.load(Ordering::SeqCst)),
        })
    }

    /// Cancels in-flight I/O of every clone; later calls fail (reads see end of stream).
    /// A server end is disconnected as well, which discards what the client did not read.
    pub fn shutdown(&self, _how: std::net::Shutdown) -> io::Result<()> {
        let handle = self.pipe.handle.as_raw_handle();
        self.pipe.closed.store(true, Ordering::SeqCst);
        // SAFETY: a valid handle; cancels the I/O of every thread on it.
        unsafe { CancelIoEx(handle, ptr::null()) };
        if self.pipe.server {
            // SAFETY: a server end of a pipe.
            unsafe { DisconnectNamedPipe(handle) };
        }
        Ok(())
    }

    fn closed(&self, reading: bool) -> io::Result<usize> {
        if reading {
            Ok(0)
        } else {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
    }

    /// One overlapped read or write, bounded by the stream's timeout. A timed-out
    /// operation is cancelled; bytes it moved before the cancel are still returned.
    fn transfer(&self, reading: bool, buffer: *mut u8, length: usize) -> io::Result<usize> {
        if self.pipe.closed.load(Ordering::SeqCst) {
            return self.closed(reading);
        }
        let handle = self.pipe.handle.as_raw_handle();
        let length = u32::try_from(length).unwrap_or(u32::MAX);
        // SAFETY: a manual-reset, initially unsignalled, unnamed event.
        let event = owned(unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) })?;
        // SAFETY: an all-zero OVERLAPPED is valid.
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
        overlapped.hEvent = event.as_raw_handle();
        // SAFETY: `buffer` holds `length` bytes and, like `overlapped`, outlives the
        // operation: every path below waits for its completion.
        let started = unsafe {
            if reading {
                ReadFile(handle, buffer, length, ptr::null_mut(), &mut overlapped)
            } else {
                WriteFile(handle, buffer, length, ptr::null_mut(), &mut overlapped)
            }
        };
        if started == 0 {
            // SAFETY: GetLastError has no preconditions.
            let code = unsafe { GetLastError() };
            if code != ERROR_IO_PENDING {
                return self.failed(reading, code);
            }
            let timeout = if reading {
                &self.read_timeout
            } else {
                &self.write_timeout
            };
            let wait = match timeout.load(Ordering::SeqCst) {
                0 => INFINITE,
                nanos => millis(Duration::from_nanos(nanos)),
            };
            // SAFETY: a valid event handle.
            if unsafe { WaitForSingleObject(event.as_raw_handle(), wait) } == WAIT_TIMEOUT {
                // SAFETY: cancels only this operation.
                unsafe { CancelIoEx(handle, &overlapped) };
            }
        }
        let mut transferred = 0u32;
        // SAFETY: waits until the system is done with `overlapped` and `buffer`.
        if unsafe { GetOverlappedResult(handle, &overlapped, &mut transferred, 1) } != 0 {
            return Ok(transferred as usize);
        }
        // SAFETY: GetLastError has no preconditions.
        let code = unsafe { GetLastError() };
        if code == ERROR_OPERATION_ABORTED {
            if transferred > 0 {
                return Ok(transferred as usize);
            }
            if self.pipe.closed.load(Ordering::SeqCst) {
                return self.closed(reading);
            }
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "pipe operation timed out",
            ));
        }
        self.failed(reading, code)
    }

    fn failed(&self, reading: bool, code: u32) -> io::Result<usize> {
        match code {
            ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_NO_DATA => self.closed(reading),
            code => Err(io::Error::from_raw_os_error(code as i32)),
        }
    }
}

impl Read for Stream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        self.transfer(true, buffer.as_mut_ptr(), buffer.len())
    }
}

impl Write for Stream {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        // WriteFile only reads from the buffer.
        self.transfer(false, buffer.as_ptr().cast_mut(), buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "windows_tests.rs"]
mod tests;
