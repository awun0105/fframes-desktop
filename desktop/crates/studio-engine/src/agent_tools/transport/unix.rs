use super::Accepted;
use crate::agent_tools::broker::BrokerError;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{
            ffi::OsStrExt,
            fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

pub type Stream = UnixStream;

/// Longest filesystem path a unix socket address can name (macOS is the strictest).
const MAX_SOCKET_PATH: usize = 100;

fn io_error(context: &'static str) -> impl FnOnce(io::Error) -> BrokerError {
    move |source| BrokerError::Io { context, source }
}

fn euid() -> u32 {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() }
}

fn insecure(path: &Path, reason: impl Into<String>) -> BrokerError {
    BrokerError::InsecureRuntimeDir {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn verify_private_dir(path: &Path) -> Result<(), BrokerError> {
    let meta = fs::symlink_metadata(path).map_err(io_error("inspect runtime dir"))?;
    if meta.file_type().is_symlink() {
        return Err(insecure(path, "is a symlink"));
    }
    if !meta.is_dir() {
        return Err(insecure(path, "is not a directory"));
    }
    if meta.uid() != euid() {
        return Err(insecure(path, "is not owned by the current user"));
    }
    if meta.mode() & 0o077 != 0 {
        return Err(insecure(
            path,
            format!(
                "is accessible by group or others (mode {:o})",
                meta.mode() & 0o777
            ),
        ));
    }
    Ok(())
}

/// Creates the runtime directory 0700 when it is missing, verifies it and returns its
/// canonical path.
pub fn prepare_runtime_dir(dir: &Path) -> Result<PathBuf, BrokerError> {
    match fs::symlink_metadata(dir) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(dir)
                .map_err(io_error("create runtime dir"))?;
        }
        Err(error) => return Err(io_error("inspect runtime dir")(error)),
    }
    verify_private_dir(dir)?;
    fs::canonicalize(dir).map_err(io_error("resolve runtime dir"))
}

/// A previous run's socket and capability files are dead: its sockets are gone.
fn clear_stale(runtime_dir: &Path, socket: &Path) -> Result<(), BrokerError> {
    match fs::symlink_metadata(socket) {
        Ok(meta) if meta.file_type().is_socket() && meta.uid() == euid() => {
            if UnixStream::connect(socket).is_ok() {
                return Err(BrokerError::AlreadyRunning(socket.to_path_buf()));
            }
            fs::remove_file(socket).map_err(io_error("remove stale socket"))?;
        }
        Ok(_) => return Err(insecure(runtime_dir, "contains an unexpected broker.sock")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("inspect socket")(error)),
    }
    for entry in fs::read_dir(runtime_dir).map_err(io_error("list runtime dir"))? {
        let entry = entry.map_err(io_error("list runtime dir"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("cap-") && name.ends_with(".json") {
            let path = entry.path();
            if let Ok(meta) = fs::symlink_metadata(&path)
                && meta.is_file()
                && meta.uid() == euid()
            {
                let _ = fs::remove_file(path);
            }
        }
    }
    Ok(())
}

fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // SAFETY: zeroed ucred is a valid out-parameter; `length` is its size.
        let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
        let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let rc = unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                std::ptr::addr_of_mut!(cred).cast(),
                &mut length,
            )
        };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(cred.uid)
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let (mut uid, mut gid): (libc::uid_t, libc::gid_t) = (0, 0);
        // SAFETY: valid descriptor and out-parameters.
        if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(uid)
    }
}

/// The broker's listening socket `broker.sock` (0600) in the runtime directory.
pub struct Listener {
    listener: UnixListener,
    endpoint: PathBuf,
}

impl Listener {
    /// Binds in a runtime directory returned by [`prepare_runtime_dir`], after clearing
    /// a previous run's dead socket and capability files.
    pub fn bind(runtime_dir: &Path) -> Result<Self, BrokerError> {
        let socket = runtime_dir.join("broker.sock");
        if socket.as_os_str().len() >= MAX_SOCKET_PATH || socket.to_str().is_none() {
            return Err(BrokerError::SocketPathTooLong(socket));
        }
        clear_stale(runtime_dir, &socket)?;
        let listener = UnixListener::bind(&socket).map_err(io_error("bind socket"))?;
        let setup = (|| {
            fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))
                .map_err(io_error("restrict socket"))?;
            listener
                .set_nonblocking(true)
                .map_err(io_error("configure socket"))
        })();
        if let Err(error) = setup {
            let _ = fs::remove_file(&socket);
            return Err(error);
        }
        Ok(Self {
            listener,
            endpoint: socket,
        })
    }

    pub fn endpoint(&self) -> &Path {
        &self.endpoint
    }

    /// Waits up to `tick` for a connection; only peers running as our uid are handed out.
    pub fn accept(&self, tick: Duration) -> Accepted {
        let mut poll = libc::pollfd {
            fd: self.listener.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one valid pollfd.
        let ready = unsafe { libc::poll(&mut poll, 1, tick.as_millis() as i32) };
        if ready <= 0 {
            return Accepted::Idle;
        }
        let stream = match self.listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Accepted::Idle,
            Err(_) => {
                thread::sleep(Duration::from_millis(10));
                return Accepted::Idle;
            }
        };
        // BSDs hand out sockets that inherit O_NONBLOCK from the listener.
        if stream.set_nonblocking(false).is_err() {
            return Accepted::Idle;
        }
        if peer_uid(&stream).ok() != Some(euid()) {
            return Accepted::Refused;
        }
        Accepted::Peer(stream)
    }
}

/// Removes the socket file once the broker stopped serving it.
pub fn remove_endpoint(endpoint: &Path) {
    let _ = fs::remove_file(endpoint);
}

/// The peer was verified when it was accepted.
pub fn verify_peer(_stream: &Stream) -> bool {
    true
}

/// Cuts a stream off from another thread: a blocked read or write returns at once.
pub fn close(stream: &Stream) {
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

/// Ends a connection after its replies were written.
pub fn finish(stream: Stream) {
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

/// Writes a new 0600 file, never following a symlink and never replacing a file.
pub fn create_private_file(path: &Path, body: &[u8]) -> io::Result<()> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .and_then(|mut handle| handle.write_all(body).and_then(|()| handle.flush()))
}

/// Opens a file only the current user can read: no symlink, a regular file owned by us,
/// no group or world bits. The checks run on the opened descriptor, so nothing can be
/// swapped between check and read.
pub fn open_private_file(path: &Path) -> Result<File, String> {
    // O_NOFOLLOW refuses a symlink at the final component.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|error| {
            format!(
                "cannot open {} (symlinks are refused): {error}",
                path.display()
            )
        })?;
    let meta = file
        .metadata()
        .map_err(|error| format!("cannot stat {}: {error}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    if meta.uid() != euid() {
        return Err(format!(
            "{} is not owned by the current user",
            path.display()
        ));
    }
    if meta.mode() & 0o077 != 0 {
        return Err(format!(
            "{} is accessible by group or others (mode {:o}); expected 0600",
            path.display(),
            meta.mode() & 0o777
        ));
    }
    Ok(file)
}

/// `connect(2)` with a deadline (std has no timeout for unix sockets).
pub fn connect(path: &Path, timeout: Duration) -> io::Result<Stream> {
    let bytes = path.as_os_str().as_bytes();
    // SAFETY: an all-zero sockaddr_un is a valid value.
    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.len() >= addr.sun_path.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "socket path is too long",
        ));
    }
    addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (dst, src) in addr.sun_path.iter_mut().zip(bytes) {
        *dst = *src as libc::c_char;
    }
    // SAFETY: plain syscalls on a descriptor we create and own; `fd` closes it on
    // every exit path.
    let (fd, raw) = unsafe {
        let raw = libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0);
        if raw < 0 {
            return Err(io::Error::last_os_error());
        }
        (OwnedFd::from_raw_fd(raw), raw)
    };
    set_flags(raw, true)?;
    let deadline = Instant::now() + timeout;
    let length = std::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;
    loop {
        // SAFETY: `addr` is a fully initialized sockaddr_un of `length` bytes.
        let rc = unsafe {
            libc::connect(
                raw,
                std::ptr::addr_of!(addr).cast::<libc::sockaddr>(),
                length,
            )
        };
        if rc == 0 {
            break;
        }
        let error = io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EINTR) => {}
            // A full listen backlog; retry until the deadline.
            Some(libc::EAGAIN) => {
                if Instant::now() >= deadline {
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out"));
                }
                thread::sleep(Duration::from_millis(5));
            }
            Some(libc::EINPROGRESS) => {
                wait_connected(raw, deadline)?;
                break;
            }
            _ => return Err(error),
        }
    }
    set_flags(raw, false)?;
    Ok(UnixStream::from(fd))
}

fn set_flags(raw: i32, nonblocking: bool) -> io::Result<()> {
    // SAFETY: fcntl on a valid descriptor with integer arguments.
    unsafe {
        if libc::fcntl(raw, libc::F_SETFD, libc::FD_CLOEXEC) < 0 {
            return Err(io::Error::last_os_error());
        }
        let flags = libc::fcntl(raw, libc::F_GETFL);
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        let flags = if nonblocking {
            flags | libc::O_NONBLOCK
        } else {
            flags & !libc::O_NONBLOCK
        };
        if libc::fcntl(raw, libc::F_SETFL, flags) < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

fn wait_connected(raw: i32, deadline: Instant) -> io::Result<()> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let mut poll = libc::pollfd {
            fd: raw,
            events: libc::POLLOUT,
            revents: 0,
        };
        // SAFETY: one valid pollfd.
        let ready = unsafe { libc::poll(&mut poll, 1, remaining.as_millis().min(60_000) as i32) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if ready == 0 {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out"));
        }
        let mut code: libc::c_int = 0;
        let mut length = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
        // SAFETY: `code`/`length` are valid out-parameters for SO_ERROR.
        let rc = unsafe {
            libc::getsockopt(
                raw,
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                std::ptr::addr_of_mut!(code).cast(),
                &mut length,
            )
        };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        return if code == 0 {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(code))
        };
    }
}
