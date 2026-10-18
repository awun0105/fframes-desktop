//! Job-object membership on Windows: snapshots, member history and the escape decision.

use super::*;

/// One live process of a Windows snapshot.
#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WinProcess {
    pub(super) pid: u32,
    pub(super) ppid: u32,
}

/// Whether a candidate process is in the task's job object.
#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JobCheck {
    InJob,
    Outside {
        created: u64,
    },
    /// Exited since the snapshot (its pid may already name another process): not a
    /// surviving writer.
    Gone,
    /// Alive but not inspectable: never assumed contained.
    Denied,
}

/// What an opened candidate handle reported.
#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy)]
pub(super) struct CandidateInfo {
    pub(super) in_job: bool,
    pub(super) created: u64,
    /// Parent pid read from the handle, not from the snapshot.
    pub(super) parent: u32,
}

/// Classifies a candidate opened after the snapshot was taken. A parent other than the
/// one the snapshot recorded means the snapshot's process exited and its pid was reused
/// in between: the opened process is someone else's.
#[cfg(any(windows, test))]
pub(super) fn classify_candidate(snapshot_ppid: u32, info: CandidateInfo) -> JobCheck {
    if info.parent != snapshot_ppid {
        JobCheck::Gone
    } else if info.in_job {
        JobCheck::InJob
    } else {
        JobCheck::Outside {
            created: info.created,
        }
    }
}

/// Processes outside the job whose parent is a recorded job member (or another escapee)
/// created no later than them. Members are `(pid, creation time)` seen in the job at any
/// earlier scan, so a child whose parent already exited is still attributed; the creation
/// order rules out a recycled parent pid. An exited process is not escaped; one that
/// cannot be inspected is, but its own children are not attributed through it: without
/// its creation time a recycled pid cannot be ruled out.
#[cfg(any(windows, test))]
pub(super) fn escaped_from_members(
    snapshot: &[WinProcess],
    members: &[(u32, u64)],
    check: &mut impl FnMut(u32) -> JobCheck,
) -> Vec<u32> {
    let mut ancestors: Vec<(u32, Option<u64>)> = members
        .iter()
        .map(|&(pid, created)| (pid, Some(created)))
        .collect();
    let mut escaped = Vec::new();
    let mut seen = std::collections::HashSet::new();
    loop {
        let mut found = false;
        for process in snapshot {
            if seen.contains(&process.pid) || !ancestors.iter().any(|(pid, _)| *pid == process.ppid)
            {
                continue;
            }
            let created = match check(process.pid) {
                JobCheck::Gone | JobCheck::InJob => {
                    seen.insert(process.pid);
                    continue;
                }
                JobCheck::Denied => None,
                JobCheck::Outside { created } => Some(created),
            };
            let parented = ancestors.iter().any(|(pid, parent_created)| {
                *pid == process.ppid
                    && parent_created.is_some_and(|parent| created.is_none_or(|c| parent <= c))
            });
            seen.insert(process.pid);
            if parented {
                escaped.push(process.pid);
                ancestors.push((process.pid, created));
                found = true;
            }
        }
        if !found {
            return escaped;
        }
    }
}

/// Folds the escapes found by one scan and the member history's gap into one answer.
/// Found escapes win over a gap: they are the stronger, more useful fact.
#[cfg(any(windows, test))]
pub(super) fn escape_verdict(escaped: Vec<u32>, gap: Option<&str>) -> EscapeScan {
    match gap {
        _ if !escaped.is_empty() => EscapeScan::Escaped(escaped),
        Some(reason) => EscapeScan::Unverifiable(reason.to_owned()),
        None => EscapeScan::Clean,
    }
}

/// A process seen in the task's job. On Windows its open handle (`H`) keeps the pid from
/// being given to another process, so attributing a child to this pid stays exact after
/// it exits.
#[cfg(any(windows, test))]
pub(super) struct JobMember<H> {
    pub(super) pid: u32,
    pub(super) created: u64,
    pub(super) _handle: H,
}

/// Result of opening a pid listed in the job for recording.
#[cfg(any(windows, test))]
pub(super) enum MemberOpen<H> {
    Opened {
        created: u64,
        handle: H,
    },
    /// Exited before it could be opened, or its pid now names a process outside the job.
    Gone,
    /// Alive in the job but not recordable.
    Failed(String),
}

/// Every member seen in the task's job, plus why that record may be missing one. A gap
/// is permanent: a member that was never recorded can have started an escapee at any
/// time. A member that exited before it was sampled is not a gap; sampling every
/// `MEMBER_NOTE_INTERVAL` bounds that window instead.
#[cfg(any(windows, test))]
pub(super) struct MemberHistory<H> {
    pub(super) members: Vec<JobMember<H>>,
    pub(super) gap: Option<String>,
}

#[cfg(any(windows, test))]
impl<H> MemberHistory<H> {
    pub(super) fn new() -> Self {
        Self {
            members: Vec::new(),
            gap: None,
        }
    }

    /// Adds the job's current members. A pid already recorded is the same process: its
    /// held handle keeps the pid from being reused.
    pub(super) fn record(
        &mut self,
        pids: Result<Vec<u32>, String>,
        open: &mut impl FnMut(u32) -> MemberOpen<H>,
    ) {
        let pids = match pids {
            Ok(pids) => pids,
            Err(reason) => return self.note_gap(reason),
        };
        for pid in pids {
            if self.members.iter().any(|m| m.pid == pid) {
                continue;
            }
            if self.members.len() >= MAX_JOB_MEMBERS {
                return self.note_gap(format!(
                    "more than {MAX_JOB_MEMBERS} processes ran in the task scope"
                ));
            }
            match open(pid) {
                MemberOpen::Opened { created, handle } => self.members.push(JobMember {
                    pid,
                    created,
                    _handle: handle,
                }),
                MemberOpen::Gone => {}
                MemberOpen::Failed(reason) => {
                    self.note_gap(format!("job member {pid} could not be recorded: {reason}"))
                }
            }
        }
    }

    /// Keeps the first reason: it is where the record stopped being complete.
    pub(super) fn note_gap(&mut self, reason: String) {
        self.gap.get_or_insert(reason);
    }

    /// `(pid, creation time)` of every recorded member.
    pub(super) fn ancestry(&self) -> Vec<(u32, u64)> {
        self.members.iter().map(|m| (m.pid, m.created)).collect()
    }

    /// Recorded members other than `root`; `None` once the record has a gap.
    pub(super) fn observed(&self, root: u32) -> Option<usize> {
        self.gap
            .is_none()
            .then(|| self.members.iter().filter(|m| m.pid != root).count())
    }
}

#[cfg(windows)]
pub(super) fn process_creation_time(handle: windows_sys::Win32::Foundation::HANDLE) -> Option<u64> {
    use windows_sys::Win32::Foundation::FILETIME;
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
    // SAFETY: `handle` is a live process handle with query rights.
    let ok = unsafe {
        windows_sys::Win32::System::Threading::GetProcessTimes(
            handle,
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    };
    (ok != 0).then(|| (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
}

/// Parent pid of the process behind `handle`, read from the kernel rather than a snapshot.
#[cfg(windows)]
pub(super) fn process_parent_pid(handle: windows_sys::Win32::Foundation::HANDLE) -> Option<u32> {
    use windows_sys::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};
    use windows_sys::Win32::System::Threading::PROCESS_BASIC_INFORMATION;
    // SAFETY: plain-data struct; all-zero is a valid value.
    let mut info: PROCESS_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: `handle` is a live process handle with query rights; `info` is writable and
    // exactly the size passed.
    let status = unsafe {
        NtQueryInformationProcess(
            handle,
            ProcessBasicInformation,
            (&mut info as *mut PROCESS_BASIC_INFORMATION).cast(),
            std::mem::size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            std::ptr::null_mut(),
        )
    };
    // NTSTATUS: negative values are errors.
    (status >= 0).then_some(info.InheritedFromUniqueProcessId as u32)
}

/// Whether the process behind `handle` is in `job`; `None` when that cannot be told.
#[cfg(windows)]
pub(super) fn process_in_job(
    handle: windows_sys::Win32::Foundation::HANDLE,
    job: windows_sys::Win32::Foundation::HANDLE,
) -> Option<bool> {
    let mut in_job = 0;
    // SAFETY: both handles are live for the duration of the call.
    let ok =
        unsafe { windows_sys::Win32::System::JobObjects::IsProcessInJob(handle, job, &mut in_job) };
    (ok != 0).then_some(in_job != 0)
}

/// A query handle to `pid`; `Err(gone)` when it cannot be opened. Only
/// `ERROR_INVALID_PARAMETER` (no such pid) means gone; any other failure is a process
/// that exists but cannot be inspected.
#[cfg(windows)]
pub(super) fn open_process(pid: u32) -> Result<std::os::windows::io::OwnedHandle, bool> {
    use std::os::windows::io::{FromRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::{ERROR_INVALID_PARAMETER, GetLastError};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    // SAFETY: plain query open; ownership of the handle moves into `OwnedHandle`.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        // SAFETY: reads the calling thread's last error.
        let gone = unsafe { GetLastError() } == ERROR_INVALID_PARAMETER;
        return Err(gone);
    }
    // SAFETY: `handle` is a fresh, valid handle nobody else owns.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle as _) })
}

/// Opens `pid` for query and runs `f` on the handle; `Err(gone)` when it cannot be opened.
#[cfg(windows)]
pub(super) fn with_process<T>(
    pid: u32,
    f: impl FnOnce(windows_sys::Win32::Foundation::HANDLE) -> T,
) -> Result<T, bool> {
    use std::os::windows::io::AsRawHandle;
    let handle = open_process(pid)?;
    Ok(f(handle.as_raw_handle() as _))
}

/// Inspects a snapshot process that names a recorded member (or escapee) as its parent.
#[cfg(windows)]
pub(super) fn probe_candidate(
    pid: u32,
    snapshot_ppid: u32,
    job: windows_sys::Win32::Foundation::HANDLE,
) -> JobCheck {
    let info = with_process(pid, |handle| {
        Some(CandidateInfo {
            in_job: process_in_job(handle, job)?,
            created: process_creation_time(handle)?,
            parent: process_parent_pid(handle)?,
        })
    });
    match info {
        Ok(Some(info)) => classify_candidate(snapshot_ppid, info),
        Ok(None) | Err(false) => JobCheck::Denied,
        Err(true) => JobCheck::Gone,
    }
}

/// Opens a pid listed in `job` and keeps its handle for the member history.
#[cfg(windows)]
pub(super) fn open_member(
    pid: u32,
    job: windows_sys::Win32::Foundation::HANDLE,
) -> MemberOpen<std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::AsRawHandle;
    let handle = match open_process(pid) {
        Ok(handle) => handle,
        Err(true) => return MemberOpen::Gone,
        Err(false) => return MemberOpen::Failed("it could not be opened".into()),
    };
    let raw = handle.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
    match process_in_job(raw, job) {
        // The member exited after the list was read and its pid was reused.
        Some(false) => return MemberOpen::Gone,
        Some(true) => {}
        None => return MemberOpen::Failed("its job membership could not be read".into()),
    }
    match process_creation_time(raw) {
        Some(created) => MemberOpen::Opened { created, handle },
        None => MemberOpen::Failed("its creation time could not be read".into()),
    }
}

/// Most members remembered per task (each holds one handle).
#[cfg(any(windows, test))]
pub(super) const MAX_JOB_MEMBERS: usize = 4096;
/// Time between two member recordings: the background sampler's period and the least
/// gap between two recordings driven by the event stream.
#[cfg(windows)]
pub(super) const MEMBER_NOTE_INTERVAL: Duration = Duration::from_millis(100);

/// Every live process with its parent, from one Toolhelp snapshot.
#[cfg(windows)]
pub(super) fn windows_process_snapshot() -> Option<Vec<WinProcess>> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
    // SAFETY: snapshot handle is closed below; entries are plain structs.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut processes = Vec::new();
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                processes.push(WinProcess {
                    pid: entry.th32ProcessID,
                    ppid: entry.th32ParentProcessID,
                });
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        Some(processes)
    }
}

/// Pids currently assigned to `job`. A list that does not fit is asked for again with a
/// buffer sized from the reported count; `Err` when it cannot be read in full.
#[cfg(windows)]
pub(super) fn job_member_pids(
    job: windows_sys::Win32::Foundation::HANDLE,
) -> Result<Vec<u32>, String> {
    use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, GetLastError};
    use windows_sys::Win32::System::JobObjects::{
        JOBOBJECT_BASIC_PROCESS_ID_LIST, JobObjectBasicProcessIdList, QueryInformationJobObject,
    };
    const ATTEMPTS: usize = 4;
    // The two counts, in pointer-sized words, before the first id.
    let header = std::mem::offset_of!(JOBOBJECT_BASIC_PROCESS_ID_LIST, ProcessIdList)
        .div_ceil(std::mem::size_of::<usize>());
    let mut capacity = 256;
    for _ in 0..ATTEMPTS {
        // `usize` words keep the buffer aligned for the struct and its ids.
        let mut buffer = vec![0usize; header + capacity];
        let bytes = u32::try_from(std::mem::size_of_val(buffer.as_slice()))
            .map_err(|_| "the job's process list does not fit a query".to_owned())?;
        // SAFETY: the buffer is writable, suitably aligned and `bytes` long.
        let ok = unsafe {
            QueryInformationJobObject(
                job,
                JobObjectBasicProcessIdList,
                buffer.as_mut_ptr().cast(),
                bytes,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            // SAFETY: reads the calling thread's last error right after the failed call.
            let error = unsafe { GetLastError() };
            if error != ERROR_MORE_DATA {
                return Err(format!(
                    "the job's process list could not be read (error {error})"
                ));
            }
        }
        let raw = buffer.as_ptr().cast::<JOBOBJECT_BASIC_PROCESS_ID_LIST>();
        // SAFETY: the header lies inside the buffer; it is read through the raw pointer,
        // whose provenance covers the whole buffer.
        let (assigned, listed) = unsafe {
            (
                (*raw).NumberOfAssignedProcesses as usize,
                (*raw).NumberOfProcessIdsInList as usize,
            )
        };
        let listed = listed.min(capacity);
        if ok != 0 && (listed < capacity || assigned <= listed) {
            // SAFETY: `ProcessIdList` starts inside the buffer. Its address is taken
            // without a reference to the one-element array, so reading the `listed` ids
            // written after it stays within the buffer's provenance.
            let ids = unsafe { std::ptr::addr_of!((*raw).ProcessIdList) }.cast::<usize>();
            return Ok((0..listed)
                // SAFETY: `i < listed <= capacity`: every such id is in the buffer.
                .map(|i| unsafe { ids.add(i).read() } as u32)
                .collect());
        }
        // Too small, or members joined since the count was taken: grow past it.
        capacity = (assigned + 64).max(capacity * 2);
    }
    Err("the job's process list kept outgrowing its buffer".into())
}

#[cfg(windows)]
pub(super) fn job_has_active_processes(job: windows_sys::Win32::Foundation::HANDLE) -> bool {
    // SAFETY: queries a job object handle owned by the tracker.
    unsafe {
        let mut accounting: windows_sys::Win32::System::JobObjects::JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = std::mem::zeroed();
        let mut ret_len = 0;
        let res = windows_sys::Win32::System::JobObjects::QueryInformationJobObject(
            job,
            windows_sys::Win32::System::JobObjects::JobObjectBasicAccountingInformation,
            &mut accounting as *mut _ as _,
            std::mem::size_of_val(&accounting) as u32,
            &mut ret_len,
        );
        // A failed query cannot prove the owned descendants exited.
        res == 0 || accounting.ActiveProcesses > 0
    }
}

/// The task's job object and its member history, shared with the background sampler.
#[cfg(windows)]
pub(super) struct JobScope {
    pub(super) job: windows_sys::Win32::Foundation::HANDLE,
    pub(super) history: Mutex<MemberHistory<std::os::windows::io::OwnedHandle>>,
}

// SAFETY: a job object handle may be used from any thread; it is closed only in `Drop`,
// once no thread holds the scope any more.
#[cfg(windows)]
unsafe impl Send for JobScope {}
#[cfg(windows)]
unsafe impl Sync for JobScope {}

#[cfg(windows)]
impl JobScope {
    pub(super) fn record(&self) {
        let mut history = self.history.lock();
        let job = self.job;
        history.record(job_member_pids(job), &mut |pid| open_member(pid, job));
    }
}

#[cfg(windows)]
impl Drop for JobScope {
    fn drop(&mut self) {
        if !self.job.is_null() {
            // SAFETY: the scope owns the job handle and this is its last user.
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.job);
            }
        }
    }
}

/// Stop signal and thread of a running member sampler.
#[cfg(windows)]
pub(super) type MemberSampler = (std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>);

/// Records the job's members every `MEMBER_NOTE_INTERVAL` until the sender is dropped or
/// the job has no process left (nothing can join an empty job again), so a long tool call
/// that emits no agent event is still sampled. A sampler that cannot start is a gap.
#[cfg(windows)]
pub(super) fn start_member_sampler(scope: &Arc<JobScope>) -> Option<MemberSampler> {
    use std::sync::mpsc::{RecvTimeoutError, channel};
    let (stop, stopped) = channel::<()>();
    let sampled = Arc::clone(scope);
    let spawned = std::thread::Builder::new()
        .name("job-member-sampler".into())
        .spawn(move || {
            while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(MEMBER_NOTE_INTERVAL) {
                sampled.record();
                if !job_has_active_processes(sampled.job) {
                    break;
                }
            }
        });
    match spawned {
        Ok(thread) => Some((stop, thread)),
        Err(error) => {
            scope
                .history
                .lock()
                .note_gap(format!("the job member sampler could not start: {error}"));
            None
        }
    }
}
