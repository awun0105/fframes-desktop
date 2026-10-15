use super::*;
#[cfg(unix)]
use std::sync::Barrier;

#[test]
fn test_child_environment_allowlist_isolation() {
    let env = ChildEnvironment::default_allowlist();
    assert!(env.get("PATH").is_some() || env.get("Path").is_some());

    let mut cmd = Command::new("sh");
    env.apply_to_command(&mut cmd);
    // Ensure no panics, env applied
}

#[test]
fn test_child_environment_prepend_path() {
    let mut env = ChildEnvironment::empty();
    env.set("PATH", "/usr/bin");
    env.prepend_path("/custom/bin");

    #[cfg(unix)]
    assert_eq!(env.get("PATH"), Some("/custom/bin:/usr/bin"));
}

/// The host allowlist stores `PATH`; on Windows every prepend must build on the last
/// one (toolchain, then FFmpeg DLLs) and leave a single spelling for the child.
#[cfg(windows)]
#[test]
fn test_child_environment_repeated_prepend_keeps_every_entry_on_windows() {
    let mut env = ChildEnvironment::empty();
    env.set("PATH", r"C:\host");
    env.prepend_path(r"C:\sdk\toolchain\bin");
    env.prepend_path(r"C:\sdk\ffmpeg\bin");

    assert_eq!(
        env.get("Path"),
        Some(r"C:\sdk\ffmpeg\bin;C:\sdk\toolchain\bin;C:\host")
    );
    assert_eq!(
        env.iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case("PATH"))
            .count(),
        1
    );
}

#[test]
fn test_child_environment_redaction() {
    let mut env = ChildEnvironment::empty();
    env.set("NORMAL_VAR", "value123");
    env.set("API_KEY", "super_secret_key");
    env.set("MY_TOKEN", "bearer_secret");

    let redacted = env.export_redacted();
    assert_eq!(redacted.get("NORMAL_VAR"), Some(&"value123".to_string()));
    assert_eq!(redacted.get("API_KEY"), Some(&"[REDACTED]".to_string()));
    assert_eq!(redacted.get("MY_TOKEN"), Some(&"[REDACTED]".to_string()));

    // A proxy URL can carry credentials.
    env.set("HTTPS_PROXY", "http://user:pass@proxy:8080");
    assert_eq!(
        env.export_redacted().get("HTTPS_PROXY"),
        Some(&"[REDACTED]".to_string())
    );
}

#[test]
#[cfg(unix)]
fn test_tracked_child_graceful_termination() {
    let mut opts = SpawnOptions::new("sleep");
    opts.arg("30");
    let mut child = spawn_tracked(opts).expect("spawn sleep");
    assert!(child.pid() > 0);

    // Terminate within 500ms
    let start = Instant::now();
    child
        .terminate_gracefully(Duration::from_millis(500))
        .expect("terminate gracefully");
    assert!(start.elapsed() < Duration::from_secs(2));

    let status = child.try_wait().expect("wait status");
    assert!(status.is_some());
}

#[test]
#[cfg(unix)]
fn a_launch_reports_the_containment_this_platform_can_verify() {
    let mut opts = SpawnOptions::new("sleep");
    opts.arg("30");
    let mut child = spawn_tracked(opts).expect("spawn sleep");
    let ownership = child.containment();
    if cfg!(any(target_os = "linux", target_os = "macos")) {
        // Its own process group, watched by the escape scan.
        assert!(
            matches!(&ownership, WriterOwnership::ProcessGroupContained { qualification }
                if qualification.starts_with("runtime:")),
            "{ownership:?}"
        );
        // SAFETY: getpgid has no memory effects.
        assert_eq!(
            unsafe { libc::getpgid(child.pid() as i32) },
            child.pid() as i32
        );
    } else {
        // No descendant scan rules out a writer that left the group.
        assert_eq!(ownership, WriterOwnership::Unknown);
    }
    child
        .terminate_gracefully(Duration::from_millis(500))
        .expect("terminate gracefully");
}

#[test]
#[cfg(unix)]
fn test_process_tree_manager_cleanup() {
    let manager = ProcessTreeManager::new();
    let mut opts1 = SpawnOptions::new("sleep");
    opts1.arg("30");
    let _child1 = manager.spawn(opts1).expect("spawn child 1");

    let mut opts2 = SpawnOptions::new("sleep");
    opts2.arg("30");
    let _child2 = manager.spawn(opts2).expect("spawn child 2");

    assert_eq!(manager.active_count(), 2);
    manager.terminate_all(Duration::from_millis(200));
    assert_eq!(manager.active_count(), 0);
}

#[test]
#[cfg(unix)]
fn terminal_shutdown_reaps_and_prevents_background_clone_from_spawning() {
    let manager = ProcessTreeManager::new();
    let background = manager.clone();
    let mut options = SpawnOptions::new("sleep");
    options.arg("30");
    let child = background.spawn(options).unwrap();
    manager.shutdown(Duration::ZERO);
    assert!(child.lock().try_wait().unwrap().is_some());
    assert!(background.spawn(SpawnOptions::new("sleep")).is_err());
    assert_eq!(background.active_count(), 0);
}

#[test]
#[cfg(unix)]
fn concurrent_scoped_spawn_and_parent_shutdown_never_leaves_a_live_child() {
    for _ in 0..32 {
        let parent = ProcessTreeManager::new();
        let scoped = parent.sub_manager();
        let barrier = Arc::new(Barrier::new(2));
        let spawn_barrier = Arc::clone(&barrier);
        let spawn_thread = std::thread::spawn(move || {
            let mut options = SpawnOptions::new("sleep");
            options.arg("30");
            spawn_barrier.wait();
            scoped.spawn(options)
        });

        barrier.wait();
        std::thread::sleep(Duration::from_micros(100));
        parent.shutdown(Duration::ZERO);
        let spawned = spawn_thread.join().expect("spawn thread panicked");

        // Inspect the first shutdown's outcome before cleanup: a second sweep
        // could hide a child published after the first sweep returned.
        let child_exited = spawned
            .as_ref()
            .map(|child| child.lock().try_wait().expect("wait status").is_some())
            .unwrap_or(true);
        let tracked = parent.active_count();
        // Always clean up before assertions so a failed expectation cannot leak a process.
        parent.shutdown(Duration::ZERO);

        assert!(child_exited, "successful concurrent spawn remained alive");
        assert_eq!(tracked, 0, "parent retained a live concurrent child");
    }
}

#[test]
#[cfg(unix)]
fn sub_manager_scopes_cleanup_while_parent_retains_shutdown_ownership() {
    let parent = ProcessTreeManager::new();
    let mut p_opts = SpawnOptions::new("sleep");
    p_opts.arg("30");
    let parent_child = parent.spawn(p_opts).unwrap();

    let sub = parent.sub_manager();
    let mut s_opts = SpawnOptions::new("sleep");
    s_opts.arg("30");
    let sub_child = sub.spawn(s_opts).unwrap();

    // Sub-manager terminates only its own scoped child
    sub.terminate_all(Duration::from_millis(100));
    let sub_exited = sub_child.lock().try_wait().unwrap().is_some();
    let parent_survived = parent_child.lock().try_wait().unwrap().is_none();
    let sub_count = sub.active_count();
    let parent_count = parent.active_count();

    // Parent shutdown terminates remaining children and cancels sub
    parent.shutdown(Duration::ZERO);
    let parent_exited = parent_child.lock().try_wait().unwrap().is_some();

    assert!(sub_exited);
    assert!(parent_survived);
    assert_eq!(sub_count, 0);
    assert_eq!(parent_count, 1);
    assert!(parent_exited);
    assert!(sub.is_shutdown());
    assert!(sub.spawn(SpawnOptions::new("sleep")).is_err());
}

#[test]
#[cfg(unix)]
fn nested_scope_cleanup_preserves_unrelated_children() {
    let parent = ProcessTreeManager::new();
    let sibling = parent.sub_manager();
    let nested = parent.sub_manager().sub_manager();

    let mut sibling_options = SpawnOptions::new("sleep");
    sibling_options.arg("30");
    let sibling_child = sibling.spawn(sibling_options).unwrap();
    let mut nested_options = SpawnOptions::new("sleep");
    nested_options.arg("30");
    let nested_child = nested.spawn(nested_options).unwrap();

    nested.terminate_all(Duration::ZERO);
    let nested_exited = nested_child.lock().try_wait().unwrap().is_some();
    let sibling_survived = sibling_child.lock().try_wait().unwrap().is_none();
    let parent_count = parent.active_count();

    // Clean up both scopes before asserting observations.
    parent.shutdown(Duration::ZERO);
    let sibling_exited = sibling_child.lock().try_wait().unwrap().is_some();

    assert!(nested_exited);
    assert!(sibling_survived);
    assert_eq!(parent_count, 1);
    assert!(sibling_exited);
}

#[test]
#[cfg(unix)]
fn terminal_operation_scope_reaps_nested_children_without_cancelling_displayed_scope() {
    let root = ProcessTreeManager::new();
    let operation = root.sub_manager();
    let nested = operation.sub_manager();
    let displayed = root.sub_manager();
    let mut opts = SpawnOptions::new("sleep");
    opts.arg("30");
    let child = nested.spawn(opts).unwrap();
    operation.shutdown(Duration::ZERO);
    assert!(child.lock().try_wait().unwrap().is_some());
    assert!(nested.is_shutdown());
    assert!(nested.spawn(SpawnOptions::new("sleep")).is_err());
    assert!(!displayed.is_shutdown());
    let mut opts = SpawnOptions::new("sleep");
    opts.arg("30");
    let live = displayed.spawn(opts).unwrap();
    root.shutdown(Duration::ZERO);
    assert!(live.lock().try_wait().unwrap().is_some());
    assert!(displayed.is_shutdown());
    assert_eq!(root.active_count(), 0);
}

#[cfg(unix)]
fn shell(script: &str) -> TrackedChild {
    let mut opts = SpawnOptions::new("sh");
    opts.arg("-c").arg(script);
    spawn_tracked(opts).expect("spawn shell")
}

#[cfg(unix)]
fn read_pid_line(child: &mut TrackedChild) -> i32 {
    use std::io::{BufRead, BufReader};
    let stdout = child.child_mut().stdout.take().expect("piped stdout");
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line).unwrap();
    line.trim().parse().expect("helper pid line")
}

#[cfg(unix)]
fn pid_alive(pid: i32) -> bool {
    // A zombie still answers signal 0 but cannot write; its state letter tells it apart.
    // SAFETY: signal 0 probes existence only.
    let exists = unsafe { libc::kill(pid, 0) } == 0;
    exists && process_state(pid).is_some_and(|state| !state.starts_with('Z'))
}

#[cfg(target_os = "linux")]
fn process_state(pid: i32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit(") ").next().map(str::to_owned)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn process_state(pid: i32) -> Option<String> {
    let output = std::process::Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let state = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!state.is_empty()).then_some(state)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn wait_for_escaped_descendant(mut observe: impl FnMut() -> Vec<u32>, helper: u32) -> Vec<u32> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let escaped = observe();
        if escaped.contains(&helper) {
            return escaped;
        }
        assert!(
            Instant::now() < deadline,
            "setsid helper {helper} never left its parent's process group"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[cfg(unix)]
fn forced_termination_verifies_term_ignoring_group_members_are_gone() {
    let mut child = shell("trap '' TERM; sleep 30 & echo $!; wait");
    let helper = read_pid_line(&mut child);
    let report = child
        .terminate_verified(Duration::from_millis(100))
        .expect("terminates");
    assert!(report.forced, "TERM-ignoring tree needs a forced kill");
    assert!(report.verified(), "{report:?}");
    assert!(report.remaining.is_empty());
    assert!(!pid_alive(helper));
}

#[test]
#[cfg(unix)]
fn dead_leader_with_surviving_group_member_is_not_reported_clean() {
    let mut child = shell("sleep 30 & echo $!; exit 0");
    let helper = read_pid_line(&mut child);
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    // The direct child has exited but a group member is alive.
    let membership = child.group_membership();
    assert!(matches!(
        &membership,
        GroupMembership::Members(pids) if pids.contains(&(helper as u32))
    ));
    assert!(child.is_alive());
    let report = child
        .terminate_verified(Duration::from_millis(500))
        .expect("terminates");
    assert!(report.verified(), "{report:?}");
    assert!(!pid_alive(helper));
    assert!(!child.is_alive());
}

#[test]
#[cfg(unix)]
fn setsid_escaped_helper_survives_group_verification_and_is_observable_only_while_parented() {
    // macOS has no setsid command, so the helper leaves the group through Python and
    // reports its pid only once it has left.
    let mut child = shell(
        "python3 -c 'import os, sys; os.setsid(); print(os.getpid(), flush=True); os.execvp(sys.argv[1], sys.argv[1:])' sleep 30 & wait",
    );
    let helper = read_pid_line(&mut child);
    // The escape scan sees the helper while it is still a descendant.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    assert_eq!(
        wait_for_escaped_descendant(
            || match child.escaped_descendants() {
                EscapeScan::Escaped(pids) => pids,
                _ => Vec::new(),
            },
            helper as u32
        ),
        vec![helper as u32]
    );
    let report = child
        .terminate_verified(Duration::from_millis(500))
        .expect("terminates");
    let survived = pid_alive(helper);
    // Clean up only the known test-owned helper before asserting.
    // SAFETY: helper is the pid this test spawned and read from the shell.
    unsafe {
        libc::kill(helper, libc::SIGKILL);
    }
    assert!(
        report.verified(),
        "group verification only covers the group: {report:?}"
    );
    assert!(survived, "setsid helper escapes the process group");
}

#[test]
fn writer_ownership_is_qualified_only_for_verified_containment() {
    assert!(
        WriterOwnership::ProcessGroupContained {
            qualification: "fixture".into()
        }
        .is_qualified()
    );
    assert!(
        WriterOwnership::JobObjectContained {
            verification: "fixture".into()
        }
        .is_qualified()
    );
    assert!(!WriterOwnership::Detached.is_qualified());
    assert!(!WriterOwnership::Unknown.is_qualified());
}

#[cfg(target_os = "linux")]
fn stat_line(pid: u32, comm: &str, state: &str, ppid: u32, pgrp: i32, start: u64) -> String {
    // pid (comm) state ppid pgrp session tty tpgid flags minflt cminflt majflt cmajflt
    // utime stime cutime cstime priority nice threads itrealvalue starttime ...
    format!("{pid} ({comm}) {state} {ppid} {pgrp} 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 {start} 0 0")
}

#[cfg(target_os = "linux")]
fn entry(pid: u32, ppid: u32, pgrp: i32, start: u64, zombie: bool) -> ProcEntry {
    ProcEntry {
        pid,
        ppid,
        pgrp,
        start,
        zombie,
    }
}

#[cfg(target_os = "linux")]
fn complete(entries: Vec<ProcEntry>) -> ProcScan {
    ProcScan {
        entries,
        complete: true,
    }
}

#[test]
#[cfg(target_os = "linux")]
fn proc_stat_parser_handles_spaces_parentheses_and_reads_start_time() {
    let entry = parse_proc_stat(7, &stat_line(7, "we ird) name", "S", 1, 42, 9001)).unwrap();
    assert_eq!(
        (entry.ppid, entry.pgrp, entry.start, entry.zombie),
        (1, 42, 9001, false)
    );
    assert!(
        parse_proc_stat(8, &stat_line(8, "x", "Z", 1, 9, 5))
            .unwrap()
            .zombie
    );
    // A truncated line is a parse failure, which makes a scan incomplete.
    assert!(parse_proc_stat(8, "8 (x) Z 1 9 9").is_none());
}

#[test]
#[cfg(target_os = "linux")]
fn unreadable_scan_is_unverified_present_never_empty() {
    let mut probes = 0;
    let membership = membership_with(
        500,
        Some(1),
        &mut || {
            probes += 1;
            Probe::Exists
        },
        // Injected scan failure: the group is known to exist but cannot be enumerated.
        &mut || ProcScan {
            entries: vec![entry(1, 0, 1, 1, false)],
            complete: false,
        },
        &|| {},
    );
    assert_eq!(membership, GroupMembership::Present);
    // The same scan that merely lacks the group still cannot clear it.
    assert_eq!(
        classify_scan(
            &ProcScan {
                entries: vec![],
                complete: false
            },
            500,
            None
        ),
        ScanVerdict::Incomplete
    );
}

#[test]
#[cfg(target_os = "linux")]
fn zombie_only_group_needs_two_quiet_scans_and_a_fork_between_them_is_seen() {
    // Scan 1 sees only a zombie leader; before scan 2 a live member appears
    // (a fork racing the enumeration). The group is not empty.
    let mut scans = vec![
        complete(vec![entry(500, 1, 500, 77, true)]),
        complete(vec![
            entry(500, 1, 500, 77, true),
            entry(501, 500, 500, 90, false),
        ]),
    ]
    .into_iter();
    let membership = membership_with(
        500,
        Some(77),
        &mut || Probe::Exists,
        &mut || scans.next().expect("no more than two scans"),
        &|| {},
    );
    assert_eq!(membership, GroupMembership::Members(vec![501]));

    // Two consecutive quiet scans clear it.
    let quiet = complete(vec![entry(500, 1, 500, 77, true)]);
    let membership = membership_with(
        500,
        Some(77),
        &mut || Probe::Exists,
        &mut || quiet.clone(),
        &|| {},
    );
    assert_eq!(membership, GroupMembership::Empty);
    // Kernel says gone: empty without scanning at all.
    let membership = membership_with(
        500,
        Some(77),
        &mut || Probe::Gone,
        &mut || panic!("no scan needed"),
        &|| {},
    );
    assert_eq!(membership, GroupMembership::Empty);
    let membership = membership_with(
        500,
        Some(77),
        &mut || Probe::Unknown,
        &mut || panic!("no scan needed"),
        &|| {},
    );
    assert_eq!(membership, GroupMembership::Present);
}

#[test]
#[cfg(target_os = "linux")]
fn a_recycled_group_id_is_foreign_and_never_ours() {
    // Same numeric id, different leader start time: an unrelated process family.
    let recycled = complete(vec![entry(500, 1, 500, 99_999, false)]);
    assert_eq!(
        classify_scan(&recycled, 500, Some(77)),
        ScanVerdict::Foreign
    );
    let membership = membership_with(
        500,
        Some(77),
        &mut || Probe::Exists,
        &mut || recycled.clone(),
        &|| {},
    );
    assert_eq!(membership, GroupMembership::Empty);
    assert!(escaped_in_scan(&recycled, 500, 500, Some(77)).is_empty());
    // Our own leader is recognised.
    let ours = complete(vec![entry(500, 1, 500, 77, false)]);
    assert_eq!(
        classify_scan(&ours, 500, Some(77)),
        ScanVerdict::Members(vec![500])
    );
}

#[test]
#[cfg(target_os = "linux")]
fn escape_detection_follows_descendants_and_ignores_incomplete_scans() {
    let scan = complete(vec![
        entry(500, 1, 500, 77, false),
        entry(501, 500, 500, 78, false),
        entry(502, 501, 700, 79, false),
        entry(503, 501, 700, 80, true),
    ]);
    assert_eq!(escaped_in_scan(&scan, 500, 500, Some(77)), vec![502]);
    let mut partial = scan;
    partial.complete = false;
    assert!(escaped_in_scan(&partial, 500, 500, Some(77)).is_empty());
}

#[test]
#[cfg(unix)]
fn a_verified_terminated_tracker_is_inert_and_never_probes_or_signals_again() {
    let mut child = shell("sleep 30 & wait");
    let report = child
        .terminate_verified(Duration::from_millis(200))
        .unwrap();
    assert!(report.verified());
    assert!(child.is_terminated());
    let calls = child.os_calls();
    assert!(calls > 0);
    // Everything below runs while the numeric group id could already belong to
    // someone else. None of it may touch the OS again.
    assert_eq!(
        child.terminate_verified(Duration::from_millis(50)).unwrap(),
        report
    );
    child
        .terminate_gracefully(Duration::from_millis(50))
        .unwrap();
    child.kill_forcefully().unwrap();
    assert!(!child.is_alive());
    assert_eq!(child.group_membership(), GroupMembership::Empty);
    assert_eq!(child.escaped_descendants(), EscapeScan::Clean);
    assert!(child.try_wait().unwrap().is_some());
    assert_eq!(
        child.os_calls(),
        calls,
        "no probe or signal after the latch"
    );
    drop(child);
}

#[test]
#[cfg(unix)]
fn naturally_exited_group_latches_when_observed_and_is_never_signalled() {
    let mut child = shell("exit 0");
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.is_alive() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(child.try_wait().unwrap().is_some());
    assert!(child.is_terminated());
    let calls = child.os_calls();
    child.kill_forcefully().unwrap();
    assert_eq!(child.os_calls(), calls);
}

#[test]
#[cfg(unix)]
fn termination_is_bounded_even_when_the_leader_ignores_term() {
    // The shell reports its pid once the trap is in place, so TERM cannot win the race.
    let mut child = shell("trap '' TERM; echo $$; while :; do sleep 1; done");
    read_pid_line(&mut child);
    let started = Instant::now();
    let report = child
        .terminate_verified(Duration::from_millis(100))
        .unwrap();
    assert!(report.forced && report.verified(), "{report:?}");
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
#[cfg(unix)]
fn verified_scope_shutdown_seals_first_and_reports_every_tree() {
    let scope = ProcessTreeManager::new().sub_manager();
    let background = scope.clone();
    let mut options = SpawnOptions::new("sh");
    options.arg("-c").arg("sleep 30 & wait");
    let child = scope.spawn(options).unwrap();
    let report = scope.shutdown_verified(Duration::from_millis(200));
    assert_eq!(report.children.len(), 1);
    assert!(report.verified(), "{report:?}");
    assert!(child.lock().is_terminated());
    // A clone held elsewhere can no longer spawn into the sealed scope.
    assert!(background.spawn(SpawnOptions::new("sleep")).is_err());
    assert!(background.is_shutdown());
    // Sealing alone terminates nothing.
    let other = ProcessTreeManager::new();
    let mut options = SpawnOptions::new("sleep");
    options.arg("30");
    let live = other.spawn(options).unwrap();
    other.seal();
    assert!(other.spawn(SpawnOptions::new("sleep")).is_err());
    assert!(live.lock().try_wait().unwrap().is_none());
    other.shutdown(Duration::ZERO);
}

#[test]
#[cfg(unix)]
fn scope_observation_reports_live_members_without_terminating_anything() {
    let scope = ProcessTreeManager::new().sub_manager();
    assert!(scope.observe().is_clean(), "an empty scope is clean");
    let mut options = SpawnOptions::new("sleep");
    options.arg("30");
    let child = scope.spawn(options).unwrap();
    let observed = scope.observe();
    assert_eq!(observed.live_children, 1);
    assert!(!observed.is_clean());
    assert!(!observed.termination.group_empty);
    assert!(!observed.termination.direct_child_exited);
    assert_eq!(observed.termination.remaining, vec![child.lock().pid()]);
    assert!(
        child.lock().try_wait().unwrap().is_none(),
        "observing never signals or reaps"
    );
    let merged = scope.shutdown_verified(Duration::from_millis(200)).merged();
    assert!(
        merged.verified() && merged.remaining.is_empty(),
        "{merged:?}"
    );
    assert!(scope.observe().is_clean());
}

#[test]
#[cfg(target_os = "linux")]
fn scope_observation_sees_a_setsid_escape_before_the_kill() {
    let scope = ProcessTreeManager::new().sub_manager();
    let mut options = SpawnOptions::new("sh");
    options.arg("-c").arg("setsid sleep 30 & echo $!; wait");
    let child = scope.spawn(options).unwrap();
    let helper = {
        use std::io::{BufRead, BufReader};
        let stdout = child
            .lock()
            .child_mut()
            .stdout
            .take()
            .expect("piped stdout");
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line).unwrap();
        line.trim().parse::<u32>().expect("helper pid line")
    };
    let observed = wait_for_escaped_descendant(|| scope.observe().escaped, helper);
    assert_eq!(observed, vec![helper]);
    assert!(!scope.observe().is_clean());
    let termination = scope.shutdown_verified(Duration::from_millis(500));
    // SAFETY: helper is the pid this test spawned and read from the shell.
    unsafe {
        libc::kill(helper as i32, libc::SIGKILL);
    }
    assert!(
        termination.verified(),
        "group verification alone cannot see the escape: {termination:?}"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn macos_membership_ignores_zombies_but_requires_two_complete_scans() {
    let zombie_group = MacProcScan {
        entries: vec![MacProcEntry {
            pid: 500,
            pgrp: 500,
            start: (10, 20),
            zombie: true,
        }],
        complete: true,
    };
    let mut scans = 0;
    let membership = mac_membership_with(
        500,
        Some((10, 20)),
        &mut || Probe::Exists,
        &mut || {
            scans += 1;
            zombie_group.clone()
        },
        &|| {},
    );
    assert_eq!(membership, GroupMembership::Empty);
    assert_eq!(scans, 2, "one quiet snapshot is not enough to prove empty");

    // The kernel's EPERM for a zombie-only group defers to the scan.
    let membership = mac_membership_with(
        500,
        Some((10, 20)),
        &mut || Probe::Unknown,
        &mut || zombie_group.clone(),
        &|| {},
    );
    assert_eq!(membership, GroupMembership::Empty);
    let incomplete = MacProcScan {
        entries: Vec::new(),
        complete: false,
    };
    assert_eq!(
        mac_membership_with(
            500,
            Some((10, 20)),
            &mut || Probe::Unknown,
            &mut || incomplete.clone(),
            &|| {},
        ),
        GroupMembership::Present
    );
    let live_member = MacProcScan {
        entries: vec![MacProcEntry {
            pid: 501,
            pgrp: 500,
            start: (10, 21),
            zombie: false,
        }],
        complete: true,
    };
    assert_eq!(
        mac_membership_with(
            500,
            Some((10, 20)),
            &mut || Probe::Unknown,
            &mut || live_member.clone(),
            &|| {},
        ),
        GroupMembership::Members(vec![501])
    );
}

#[test]
#[cfg(target_os = "macos")]
fn macos_membership_preserves_live_and_unverifiable_groups() {
    let live_group = MacProcScan {
        entries: vec![MacProcEntry {
            pid: 501,
            pgrp: 500,
            start: (10, 21),
            zombie: false,
        }],
        complete: true,
    };
    assert_eq!(
        mac_membership_with(
            500,
            Some((10, 20)),
            &mut || Probe::Exists,
            &mut || live_group.clone(),
            &|| {},
        ),
        GroupMembership::Members(vec![501])
    );

    let incomplete = MacProcScan {
        entries: Vec::new(),
        complete: false,
    };
    assert_eq!(
        mac_membership_with(
            500,
            Some((10, 20)),
            &mut || Probe::Exists,
            &mut || incomplete.clone(),
            &|| {},
        ),
        GroupMembership::Present
    );
}

#[test]
#[cfg(target_os = "macos")]
fn macos_membership_does_not_claim_a_recycled_group_id() {
    let recycled = MacProcScan {
        entries: vec![MacProcEntry {
            pid: 500,
            pgrp: 500,
            start: (99, 1),
            zombie: false,
        }],
        complete: true,
    };
    assert_eq!(
        mac_membership_with(
            500,
            Some((10, 20)),
            &mut || Probe::Exists,
            &mut || recycled.clone(),
            &|| {},
        ),
        GroupMembership::Empty
    );
}

#[test]
#[cfg(target_os = "macos")]
fn macos_libproc_zero_returns_are_disambiguated_by_errno() {
    assert!(!mac_process_group_list_failed(0, 0));
    assert!(mac_process_group_list_failed(0, libc::EACCES));
    assert!(mac_process_group_list_failed(-1, 0));
    assert!(mac_process_info_confirms_exit(0, libc::ESRCH));
    assert!(!mac_process_info_confirms_exit(0, libc::EACCES));
    assert!(!mac_process_info_confirms_exit(0, 0));
}

#[test]
fn merged_termination_is_clean_only_if_every_tree_is() {
    let clean = TerminationReport {
        forced: false,
        direct_child_exited: true,
        group_empty: true,
        remaining: vec![],
    };
    assert!(ScopeTermination { children: vec![] }.merged().verified());
    let dirty = TerminationReport {
        forced: true,
        direct_child_exited: true,
        group_empty: false,
        remaining: vec![9],
    };
    let merged = ScopeTermination {
        children: vec![clean, dirty],
    }
    .merged();
    assert!(merged.forced && !merged.verified());
    assert_eq!(merged.remaining, vec![9]);
}
