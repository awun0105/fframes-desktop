
use super::*;

fn p(pid: u32, ppid: u32) -> WinProcess {
    WinProcess { pid, ppid }
}

#[test]
fn a_child_outside_the_job_of_an_exited_member_is_escaped() {
    // Member 10 (created at 100) already exited; its child 20 survives outside the job.
    let snapshot = [p(20, 10), p(30, 20), p(40, 1)];
    let escaped = escaped_from_members(&snapshot, &[(10, 100)], &mut |pid| match pid {
        20 => JobCheck::Outside { created: 150 },
        30 => JobCheck::Outside { created: 160 },
        _ => JobCheck::InJob,
    });
    assert_eq!(escaped, [20, 30], "grandchildren of an escapee count too");
}

#[test]
fn a_recycled_parent_pid_does_not_attribute_an_older_process() {
    let snapshot = [p(20, 10)];
    let escaped = escaped_from_members(&snapshot, &[(10, 500)], &mut |_| JobCheck::Outside {
        created: 100,
    });
    assert!(escaped.is_empty());
}

#[test]
fn exited_and_contained_children_are_not_escaped_but_uninspectable_ones_are() {
    let snapshot = [p(20, 10), p(21, 10), p(22, 10)];
    let escaped = escaped_from_members(&snapshot, &[(10, 100)], &mut |pid| match pid {
        20 => JobCheck::Gone,
        21 => JobCheck::InJob,
        _ => JobCheck::Denied,
    });
    assert_eq!(escaped, [22]);
}

#[test]
fn processes_of_unrelated_parents_are_ignored() {
    let snapshot = [p(20, 99)];
    let escaped = escaped_from_members(&snapshot, &[(10, 100)], &mut |_| JobCheck::Denied);
    assert!(escaped.is_empty());
}

#[test]
fn a_reused_candidate_pid_is_not_counted_as_an_escape() {
    // The snapshot saw 20 as a child of member 10; by the time 20 was opened it had
    // exited and its pid named a newer, unrelated process with another parent.
    let snapshot = [p(20, 10)];
    let reused = CandidateInfo {
        in_job: false,
        created: 150,
        parent: 77,
    };
    assert_eq!(classify_candidate(10, reused), JobCheck::Gone);
    let escaped = escaped_from_members(&snapshot, &[(10, 100)], &mut |_| {
        classify_candidate(10, reused)
    });
    assert!(escaped.is_empty());

    let genuine = CandidateInfo {
        parent: 10,
        ..reused
    };
    assert_eq!(
        classify_candidate(10, genuine),
        JobCheck::Outside { created: 150 }
    );
    assert_eq!(
        classify_candidate(
            10,
            CandidateInfo {
                in_job: true,
                ..genuine
            }
        ),
        JobCheck::InJob
    );
}

#[test]
fn an_uninspectable_escapee_does_not_attribute_its_own_children() {
    // 20 cannot be inspected: it counts as escaped, but without its creation time a
    // process naming pid 20 as parent may descend from an older holder of that pid.
    let snapshot = [p(20, 10), p(30, 20)];
    let escaped = escaped_from_members(&snapshot, &[(10, 100)], &mut |pid| match pid {
        20 => JobCheck::Denied,
        _ => JobCheck::Outside { created: 50 },
    });
    assert_eq!(escaped, [20]);
}

fn opened(created: u64) -> MemberOpen<()> {
    MemberOpen::Opened {
        created,
        handle: (),
    }
}

#[test]
fn a_failed_member_record_is_unverifiable_never_clean() {
    let mut history = MemberHistory::<()>::new();
    history.record(Ok(vec![10, 11]), &mut |pid| opened(u64::from(pid)));
    assert_eq!(history.ancestry(), [(10, 10), (11, 11)]);
    assert_eq!(history.observed(10), Some(1));
    assert_eq!(
        escape_verdict(Vec::new(), history.gap.as_deref()),
        EscapeScan::Clean
    );

    // A member that exited before it was opened is a sampling miss, not a gap.
    history.record(Ok(vec![12]), &mut |_| MemberOpen::Gone);
    assert_eq!(history.observed(10), Some(1));

    // A live member that could not be recorded is a permanent gap.
    history.record(Ok(vec![13]), &mut |_| MemberOpen::Failed("denied".into()));
    history.record(Ok(vec![14]), &mut |pid| opened(u64::from(pid)));
    assert_eq!(history.observed(10), None);
    assert!(matches!(
        escape_verdict(Vec::new(), history.gap.as_deref()),
        EscapeScan::Unverifiable(reason) if reason.contains("13")
    ));
    // Escapes found anyway are still reported as such.
    assert_eq!(
        escape_verdict(vec![40], history.gap.as_deref()),
        EscapeScan::Escaped(vec![40])
    );

    let mut unreadable = MemberHistory::<()>::new();
    unreadable.record(Err("job query failed".into()), &mut |pid| {
        opened(u64::from(pid))
    });
    assert_eq!(unreadable.observed(10), None);
    assert_eq!(
        escape_verdict(Vec::new(), unreadable.gap.as_deref()),
        EscapeScan::Unverifiable("job query failed".into())
    );
}

#[test]
fn reaching_the_member_cap_makes_the_record_incomplete() {
    let mut history = MemberHistory::<()>::new();
    let pids: Vec<u32> = (1..=MAX_JOB_MEMBERS as u32).collect();
    history.record(Ok(pids.clone()), &mut |pid| opened(u64::from(pid)));
    assert_eq!(history.observed(0), Some(MAX_JOB_MEMBERS));
    // Members already recorded never count against the cap again.
    history.record(Ok(pids), &mut |pid| opened(u64::from(pid)));
    assert_eq!(history.observed(0), Some(MAX_JOB_MEMBERS));

    history.record(Ok(vec![MAX_JOB_MEMBERS as u32 + 1]), &mut |pid| {
        opened(u64::from(pid))
    });
    assert_eq!(history.members.len(), MAX_JOB_MEMBERS);
    assert_eq!(history.observed(0), None);
    assert!(matches!(
        escape_verdict(Vec::new(), history.gap.as_deref()),
        EscapeScan::Unverifiable(_)
    ));
}

/// Starts `ping` as a child of `parent` (a job member) outside the job: the job must
/// permit breakaway. Returns the process handle and pid.
#[cfg(windows)]
fn spawn_escapee(parent: u32) -> (windows_sys::Win32::Foundation::HANDLE, u32) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::*;
    let mut command: Vec<u16> = std::ffi::OsStr::new("ping -n 30 127.0.0.1")
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: test-only Win32 setup of one attribute list; every handle is closed.
    unsafe {
        let mut parent_handle = OpenProcess(PROCESS_CREATE_PROCESS, 0, parent);
        assert!(!parent_handle.is_null(), "open the job member");
        let mut size = 0usize;
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut size);
        let mut storage = vec![0u8; size];
        let list: LPPROC_THREAD_ATTRIBUTE_LIST = storage.as_mut_ptr().cast();
        assert_ne!(InitializeProcThreadAttributeList(list, 1, 0, &mut size), 0);
        assert_ne!(
            UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as usize,
                (&mut parent_handle as *mut windows_sys::Win32::Foundation::HANDLE).cast(),
                std::mem::size_of_val(&parent_handle),
                std::ptr::null_mut(),
                std::ptr::null(),
            ),
            0
        );
        let mut startup: STARTUPINFOEXW = std::mem::zeroed();
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        startup.lpAttributeList = list;
        let mut info: PROCESS_INFORMATION = std::mem::zeroed();
        let created = CreateProcessW(
            std::ptr::null(),
            command.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_BREAKAWAY_FROM_JOB | CREATE_NO_WINDOW,
            std::ptr::null(),
            std::ptr::null(),
            &startup.StartupInfo,
            &mut info,
        );
        DeleteProcThreadAttributeList(list);
        CloseHandle(parent_handle);
        assert_ne!(created, 0, "start the escapee");
        CloseHandle(info.hThread);
        (info.hProcess, info.dwProcessId)
    }
}

#[cfg(windows)]
#[test]
fn a_real_process_outside_the_job_is_reported_and_in_job_children_are_not() {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::TerminateProcess;

    // cmd starts ping inside the job: an ordinary child, never an escape.
    let mut opts = SpawnOptions::new("cmd");
    opts.args(["/c", "ping -n 30 127.0.0.1 > nul"]);
    opts.breakaway_ok = true;
    let mut child = spawn_tracked(opts).expect("spawn cmd");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(child.group_membership(), GroupMembership::Present);
    assert_eq!(child.escaped_descendants(), EscapeScan::Clean);

    let (helper, helper_pid) = spawn_escapee(child.pid());
    // The parent check that rejects a reused pid must actually read the parent: a
    // failing query would turn the escapee into an uninspectable one and still pass.
    let helper_parent = process_parent_pid(helper);
    // The production path, through a limited query handle, must classify it too.
    let probed = probe_candidate(helper_pid, child.pid(), child.scope.job);
    let scan = child.escaped_descendants();
    // SAFETY: the test owns the helper handle.
    unsafe {
        TerminateProcess(helper, 1);
        CloseHandle(helper);
    }
    assert_eq!(helper_parent, Some(child.pid()));
    assert!(matches!(probed, JobCheck::Outside { .. }), "{probed:?}");
    let EscapeScan::Escaped(escaped) = scan else {
        panic!("the escapee is not reported: {scan:?}");
    };
    // The escapee's own console host is its child, so it is outside the job too.
    assert_eq!(escaped.first(), Some(&helper_pid), "{escaped:?}");
    let snapshot = windows_process_snapshot().unwrap();
    for pid in &escaped[1..] {
        let parent = snapshot.iter().find(|p| p.pid == *pid).map(|p| p.ppid);
        assert!(
            parent.is_none() || escaped.contains(&parent.unwrap()),
            "{pid} is not under the escapee"
        );
    }
    child.kill_forcefully().unwrap();
}

#[cfg(windows)]
#[test]
fn a_silent_short_lived_member_is_sampled_without_any_event() {
    // The first ping lives about a second and exits before anything is asked of the
    // tracker; only the background sampler can have recorded it.
    let mut opts = SpawnOptions::new("cmd");
    opts.args([
        "/c",
        "ping -n 2 127.0.0.1 > nul & ping -n 30 127.0.0.1 > nul",
    ]);
    let mut child = spawn_tracked(opts).expect("spawn cmd");
    std::thread::sleep(Duration::from_millis(2500));
    // Members alive now (the second ping and console hosts) are recorded by the call
    // below anyway; only a recording during the silent sleep adds the first ping.
    let live_now = job_member_pids(child.scope.job)
        .expect("job member list")
        .into_iter()
        .filter(|pid| *pid != child.pid())
        .count();
    let observed = child
        .observed_descendants()
        .expect("a complete member record");
    assert!(
        observed > live_now,
        "{observed} descendants recorded, {live_now} alive now"
    );
    assert!(child.sampler.is_some());
    child.kill_forcefully().unwrap();
    assert!(child.is_terminated());
    assert!(child.sampler.is_none(), "termination stops the sampler");
    assert_eq!(child.escaped_descendants(), EscapeScan::Clean);
}

#[cfg(windows)]
#[test]
fn the_sampler_ends_with_its_tree_and_on_drop() {
    let mut opts = SpawnOptions::new("cmd");
    opts.args(["/c", "exit 0"]);
    let child = spawn_tracked(opts).expect("spawn cmd");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !child.sampler.as_ref().is_some_and(|(_, t)| t.is_finished()) {
        assert!(
            Instant::now() < deadline,
            "the sampler outlived an empty job"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(child);

    let mut opts = SpawnOptions::new("cmd");
    opts.args(["/c", "ping -n 30 127.0.0.1 > nul"]);
    let child = spawn_tracked(opts).expect("spawn cmd");
    let started = Instant::now();
    drop(child);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "drop joins promptly"
    );
}
