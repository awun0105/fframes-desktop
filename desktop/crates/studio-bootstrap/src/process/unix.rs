//! Process-group membership on Linux and macOS: `/proc` and `proc_listpids` scans and
//! the pure decisions over them.

use super::*;

/// One consistent look at the process table. `complete` is false whenever any part of
/// the enumeration could not be read, so an absent member can never be mistaken for a
/// vanished one.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Default)]
pub(super) struct ProcScan {
    pub(super) entries: Vec<ProcEntry>,
    pub(super) complete: bool,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub(super) struct ProcEntry {
    pub(super) pid: u32,
    pub(super) ppid: u32,
    pub(super) pgrp: i32,
    pub(super) start: u64,
    pub(super) zombie: bool,
}

#[cfg(target_os = "linux")]
pub(super) fn parse_proc_stat(pid: u32, stat: &str) -> Option<ProcEntry> {
    // The command name may contain spaces and parentheses; fields follow the last ')'.
    let rest = stat.get(stat.rfind(')')? + 1..)?;
    let mut fields = rest.split_whitespace();
    let state = fields.next()?;
    let ppid = fields.next()?.parse().ok()?;
    let pgrp = fields.next()?.parse().ok()?;
    // starttime is field 22 of the line, i.e. index 19 after the state field.
    let start = fields.nth(16)?.parse().ok()?;
    Some(ProcEntry {
        pid,
        ppid,
        pgrp,
        start,
        zombie: state == "Z" || state == "X",
    })
}

/// Snapshot of /proc. Any unreadable piece makes the scan incomplete.
#[cfg(target_os = "linux")]
pub(super) fn scan_proc() -> ProcScan {
    let mut scan = ProcScan {
        entries: Vec::new(),
        complete: true,
    };
    let Ok(dir) = std::fs::read_dir("/proc") else {
        scan.complete = false;
        return scan;
    };
    for entry in dir {
        let Ok(entry) = entry else {
            scan.complete = false;
            continue;
        };
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        match std::fs::read_to_string(entry.path().join("stat")) {
            Ok(stat) => match parse_proc_stat(pid, &stat) {
                Some(parsed) => scan.entries.push(parsed),
                None => scan.complete = false,
            },
            // The process exited between listing and reading: genuinely gone.
            Err(e) if matches!(e.raw_os_error(), Some(libc::ENOENT | libc::ESRCH)) => {}
            Err(_) => scan.complete = false,
        }
    }
    scan
}

#[cfg(target_os = "linux")]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ScanVerdict {
    Members(Vec<u32>),
    /// Only zombies (or nothing) remain in the group.
    Quiet,
    /// The table was not fully readable: nothing can be concluded.
    Incomplete,
    /// The group id now belongs to an unrelated process family.
    Foreign,
}

#[cfg(target_os = "linux")]
pub(super) fn classify_scan(scan: &ProcScan, pgid: i32, leader_start: Option<u64>) -> ScanVerdict {
    if !scan.complete {
        return ScanVerdict::Incomplete;
    }
    // A live-or-zombie process owning the group id that is not our leader means the id
    // was recycled; that group is not ours and is never signalled.
    if let (Some(expected), Some(leader)) = (
        leader_start,
        scan.entries.iter().find(|p| p.pid as i32 == pgid),
    ) && leader.start != expected
    {
        return ScanVerdict::Foreign;
    }
    let members: Vec<u32> = scan
        .entries
        .iter()
        .filter(|p| p.pgrp == pgid && !p.zombie)
        .map(|p| p.pid)
        .collect();
    if members.is_empty() {
        ScanVerdict::Quiet
    } else {
        ScanVerdict::Members(members)
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Probe {
    Exists,
    Gone,
    Unknown,
}

#[cfg(unix)]
pub(super) fn probe_group(pgid: i32) -> Probe {
    // SAFETY: signal 0 only probes the group this tracker owns.
    if unsafe { libc::kill(-pgid, 0) } == 0 {
        Probe::Exists
    } else if std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        Probe::Gone
    } else {
        Probe::Unknown
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Default)]
pub(super) struct MacProcScan {
    pub(super) entries: Vec<MacProcEntry>,
    pub(super) complete: bool,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub(super) struct MacProcEntry {
    pub(super) pid: u32,
    pub(super) pgrp: u32,
    pub(super) start: (u64, u64),
    pub(super) zombie: bool,
}

#[cfg(target_os = "macos")]
pub(super) fn mac_process_start_time(pid: libc::pid_t) -> Option<(u64, u64)> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    // SAFETY: `info` points to a correctly-sized output buffer for PROC_PIDTBSDINFO.
    let read = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int,
        )
    };
    if read != std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int {
        return None;
    }
    // SAFETY: proc_pidinfo returned exactly the initialized structure size.
    let info = unsafe { info.assume_init() };
    Some((info.pbi_start_tvsec, info.pbi_start_tvusec))
}

#[cfg(target_os = "macos")]
pub(super) fn mac_process_group_list_failed(count: libc::c_int, errno: libc::c_int) -> bool {
    count < 0 || (count == 0 && errno != 0)
}

#[cfg(target_os = "macos")]
pub(super) fn mac_process_info_confirms_exit(read: libc::c_int, errno: libc::c_int) -> bool {
    read == 0 && errno == libc::ESRCH
}

/// Lists every process in a group, including zombies, via libproc. macOS does not
/// expose `/proc`, and `killpg(..., 0)` cannot tell a zombie-only group from a live one
/// (it fails with EPERM while the group holds only unreaped zombies), so termination
/// verification needs process state as well as the probe. Members owned by another user
/// are listed but refuse `proc_pidinfo` with EPERM, which leaves the scan incomplete.
#[cfg(target_os = "macos")]
pub(super) fn scan_process_group(pgid: i32) -> MacProcScan {
    let mut scan = MacProcScan {
        entries: Vec::new(),
        complete: true,
    };
    let mut capacity = 64usize;
    let pids = loop {
        let Some(bytes) = capacity.checked_mul(std::mem::size_of::<libc::pid_t>()) else {
            scan.complete = false;
            return scan;
        };
        let Ok(buffer_size) = libc::c_int::try_from(bytes) else {
            scan.complete = false;
            return scan;
        };
        let mut pids = vec![0 as libc::pid_t; capacity];
        // SAFETY: libproc writes at most `buffer_size` bytes to this allocated pid buffer.
        // SAFETY: `__error` returns this thread's errno storage.
        unsafe { *libc::__error() = 0 };
        let count = unsafe { libc::proc_listpgrppids(pgid, pids.as_mut_ptr().cast(), buffer_size) };
        let errno = unsafe { *libc::__error() };
        if mac_process_group_list_failed(count, errno) {
            scan.complete = false;
            return scan;
        }
        let count = count as usize;
        if count < capacity {
            pids.truncate(count);
            break pids;
        }
        if capacity >= 1_048_576 {
            scan.complete = false;
            return scan;
        }
        capacity *= 2;
    };

    for pid in pids {
        let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
        // A nonzero arg lets libproc read entries that have moved to zombproc but have
        // not yet been reaped by their parent.
        // SAFETY: `info` points to a correctly-sized output buffer for PROC_PIDTBSDINFO.
        // SAFETY: `__error` returns this thread's errno storage.
        unsafe { *libc::__error() = 0 };
        let read = unsafe {
            libc::proc_pidinfo(
                pid,
                libc::PROC_PIDTBSDINFO,
                1,
                info.as_mut_ptr().cast(),
                std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int,
            )
        };
        let errno = unsafe { *libc::__error() };
        if read == 0 {
            // libproc maps lookup failures to zero and leaves errno set. ESRCH means this
            // PID left the process table after enumeration; other failures make the scan
            // incomplete and cannot be counted as an exited process.
            if !mac_process_info_confirms_exit(read, errno) {
                scan.complete = false;
            }
            continue;
        }
        if read != std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int {
            scan.complete = false;
            continue;
        }
        // SAFETY: proc_pidinfo returned exactly the initialized structure size.
        let info = unsafe { info.assume_init() };
        scan.entries.push(MacProcEntry {
            pid: info.pbi_pid,
            pgrp: info.pbi_pgid,
            start: (info.pbi_start_tvsec, info.pbi_start_tvusec),
            zombie: info.pbi_status == libc::SZOMB,
        });
    }
    scan
}

#[cfg(target_os = "macos")]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum MacScanVerdict {
    Members(Vec<u32>),
    Quiet,
    Incomplete,
    Foreign,
}

#[cfg(target_os = "macos")]
pub(super) fn classify_mac_scan(
    scan: &MacProcScan,
    pgid: i32,
    leader_start: Option<(u64, u64)>,
) -> MacScanVerdict {
    if !scan.complete {
        return MacScanVerdict::Incomplete;
    }
    if let (Some(expected), Some(leader)) = (
        leader_start,
        scan.entries
            .iter()
            .find(|process| process.pid as i32 == pgid),
    ) && leader.start != expected
    {
        return MacScanVerdict::Foreign;
    }
    let members: Vec<u32> = scan
        .entries
        .iter()
        .filter(|process| process.pgrp as i32 == pgid && !process.zombie)
        .map(|process| process.pid)
        .collect();
    if members.is_empty() {
        MacScanVerdict::Quiet
    } else {
        MacScanVerdict::Members(members)
    }
}

#[cfg(target_os = "macos")]
pub(super) fn mac_membership_with(
    pgid: i32,
    leader_start: Option<(u64, u64)>,
    probe: &mut dyn FnMut() -> Probe,
    scan: &mut dyn FnMut() -> MacProcScan,
    pause: &dyn Fn(),
) -> GroupMembership {
    // macOS answers `kill(-pgid, 0)` with EPERM, not ESRCH, while every member is an
    // unreaped zombie (the leader before `wait`), so an inconclusive probe is settled by
    // the scan; an incomplete scan still counts as present.
    if probe() == Probe::Gone {
        return GroupMembership::Empty;
    }
    let mut quiet = 0;
    for _ in 0..4 {
        match classify_mac_scan(&scan(), pgid, leader_start) {
            MacScanVerdict::Incomplete => return GroupMembership::Present,
            MacScanVerdict::Foreign => return GroupMembership::Empty,
            MacScanVerdict::Members(pids) => return GroupMembership::Members(pids),
            MacScanVerdict::Quiet => {
                quiet += 1;
                if quiet == 2 {
                    return GroupMembership::Empty;
                }
                pause();
            }
        }
    }
    GroupMembership::Present
}

/// Pure membership decision over injectable probe/scan sources.
///
/// A group is `Empty` only when the kernel says it is gone, or when two consecutive
/// complete scans show nothing but zombies. An incomplete scan is `Present`: a member
/// that merely could not be seen is never treated as having exited.
#[cfg(target_os = "linux")]
pub(super) fn membership_with(
    pgid: i32,
    leader_start: Option<u64>,
    probe: &mut dyn FnMut() -> Probe,
    scan: &mut dyn FnMut() -> ProcScan,
    pause: &dyn Fn(),
) -> GroupMembership {
    match probe() {
        Probe::Gone => return GroupMembership::Empty,
        Probe::Unknown => return GroupMembership::Present,
        Probe::Exists => {}
    }
    let mut quiet = 0;
    for _ in 0..4 {
        match classify_scan(&scan(), pgid, leader_start) {
            ScanVerdict::Incomplete => return GroupMembership::Present,
            ScanVerdict::Foreign => return GroupMembership::Empty,
            ScanVerdict::Members(pids) => return GroupMembership::Members(pids),
            ScanVerdict::Quiet => {
                quiet += 1;
                if quiet == 2 {
                    return GroupMembership::Empty;
                }
                pause();
            }
        }
    }
    GroupMembership::Present
}

#[cfg(target_os = "linux")]
pub(super) fn escaped_in_scan(
    scan: &ProcScan,
    root_pid: u32,
    pgid: i32,
    leader_start: Option<u64>,
) -> Vec<u32> {
    if !scan.complete {
        return Vec::new();
    }
    if let (Some(expected), Some(root)) = (
        leader_start,
        scan.entries.iter().find(|p| p.pid == root_pid),
    ) && root.start != expected
    {
        return Vec::new();
    }
    let mut descendants = vec![root_pid];
    let mut escaped = Vec::new();
    let mut index = 0;
    while index < descendants.len() {
        let parent = descendants[index];
        index += 1;
        for process in scan.entries.iter().filter(|p| p.ppid == parent) {
            if descendants.contains(&process.pid) {
                continue;
            }
            descendants.push(process.pid);
            if process.pgrp != pgid && !process.zombie {
                escaped.push(process.pid);
            }
        }
    }
    escaped
}
