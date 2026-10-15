//! Failures, writer containment and conflicts.

use super::*;

#[test]
fn a_provider_crash_fails_the_task_with_the_draft_retained_and_no_writer() {
    let w = World::new();
    let original = w.source("src/lib.rs");
    w.submit(
        &json!({"text": "about to crash", "write": {"src/lib.rs": "// crashed\n"}, "crash": 7}),
    );
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Failed);
    let error = task.error.as_ref().unwrap();
    assert!(error.code.starts_with("provider_"), "{}", error.code);
    assert!(error.retained);
    assert!(error.action.is_some());
    assert_eq!(
        w.source("src/lib.rs"),
        original,
        "a crash never reaches the project"
    );
    assert_eq!(w.engine_state(), Some(TaskState::Failed));
    assert!(matches!(
        task.draft_state,
        Some(DraftState::Retained { .. })
    ));
    assert!(
        w.ctl()
            .lock()
            .agent_draft_store()
            .path()
            .join("src/lib.rs")
            .exists()
    );
    assert_eq!(w.snap().history.len(), 0);
    w.assert_clean();
}

#[test]
fn an_adapter_that_dies_during_startup_fails_the_task_cleanly() {
    let w = World::with(Options {
        wrapper: vec!["env".into(), "SCRIPTED_AGENT_INIT=exit".into()],
        ..Options::default()
    });
    w.submit(&good("never"));
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Failed);
    assert!(task.error.as_ref().unwrap().code.starts_with("provider_"));
    assert!(w.evidence("prompts.jsonl").is_empty());
    w.assert_clean();
}

#[test]
fn an_unqualified_adapter_blocks_capture_and_retains_the_draft_locked() {
    let w = World::with(Options {
        ownership: WriterOwnership::Unknown,
        ..Options::default()
    });
    let original = w.source("src/lib.rs");
    w.submit(&good("unqualified"));
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Failed);
    assert_eq!(task.error.as_ref().unwrap().code, "quiescence_blocked");
    assert_eq!(w.source("src/lib.rs"), original);
    assert!(matches!(
        task.draft_state,
        Some(DraftState::UnsafeWriter { .. })
    ));
    w.assert_clean();
}

#[cfg(unix)]
#[test]
fn an_escaped_writer_blocks_apply_retains_the_draft_and_locks_it_until_acknowledged() {
    let w = World::new();
    let original = w.source("src/lib.rs");
    let mut script = good("escaped");
    script["escape"] = json!(true);
    w.submit(&script);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Failed);
    assert_eq!(
        task.error.as_ref().unwrap().code,
        "quiescence_blocked",
        "{:?}",
        task.error
    );
    assert_eq!(w.source("src/lib.rs"), original, "nothing was applied");
    assert_eq!(w.snap().history.len(), 0);
    assert!(matches!(
        task.draft_state,
        Some(DraftState::UnsafeWriter { .. })
    ));
    let pids: Vec<i32> = fs::read_to_string(w.agent.join("escape.pid"))
        .unwrap()
        .split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect();
    assert!(
        !pids.is_empty() && pids.iter().all(|p| pid_alive(*p)),
        "the escaped helper is still running"
    );
    // The next task is refused while the draft may still be written.
    w.wf().submit(&World::brief(&good("blocked"))).unwrap();
    w.wait("the draft lock", |s| has_error(s, "draft_unsafe"));
    assert!(w.snap().recovery.draft.as_ref().unwrap().can_acknowledge);
    assert_eq!(
        w.snap().task.as_ref().unwrap().id,
        task.id,
        "no new task was created"
    );
    // The user confirms after dealing with the helper; then work continues.
    for pid in &pids {
        unsafe { libc::kill(*pid, libc::SIGKILL) };
    }
    w.wf().acknowledge_writer_gone().unwrap();
    w.wait("the unlocked draft", |s| {
        matches!(
            s.recovery.draft.as_ref().map(|d| &d.state),
            Some(DraftState::Retained { .. })
        )
    });
    w.submit(&good("after unlock"));
    let next = w.wait_task(Some(&task.id));
    assert_eq!(next.phase, TaskPhase::Accepted, "{:?}", next.error);
}

#[cfg(unix)]
#[test]
fn a_plain_helper_inside_the_process_group_is_reaped_and_does_not_block_capture() {
    let w = World::new();
    let mut script = good("with helper");
    script["helper"] = json!(true);
    w.submit(&script);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let pid: i32 = fs::read_to_string(w.agent.join("helper.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while pid_alive(pid) && Instant::now() < deadline {
        // Zombies of a killed non-child still answer kill(0) until reaped by init.
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
        if stat.is_empty() || stat.contains(") Z ") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    assert!(
        stat.is_empty() || stat.contains(") Z "),
        "the helper outlived the task: {stat}"
    );
    w.assert_clean();
}

#[test]
fn a_source_edit_during_review_ends_in_conflict_with_paths_and_the_candidate_kept() {
    let w = World::new();
    w.wf()
        .set_review_policy(studio_engine::ReviewPolicy::ManualReview)
        .unwrap();
    w.wait("the policy", |s| {
        s.review_policy == studio_engine::ReviewPolicy::ManualReview
    });
    w.submit(&good("candidate"));
    w.wait_phase(TaskPhase::AwaitingReview);
    fs::write(w.root.join("src/lib.rs"), "// an editor changed this\n").unwrap();
    w.wf().apply().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Conflict, "{:?}", task.error);
    let conflict = task.conflict.as_ref().expect("conflict paths");
    assert!(
        conflict.external_paths.iter().any(|p| p == "src/lib.rs"),
        "{conflict:?}"
    );
    assert!(conflict.candidate_paths.iter().any(|p| p == "src/lib.rs"));
    assert!(conflict.overlapping.iter().any(|p| p == "src/lib.rs"));
    assert_eq!(task.error.as_ref().unwrap().code, "source_conflict");
    assert_eq!(
        w.source("src/lib.rs"),
        "// an editor changed this\n",
        "the editor's bytes stay"
    );
    assert_eq!(w.snap().history.len(), 0);
    assert!(matches!(
        task.draft_state,
        Some(DraftState::Retained { .. })
    ));
    w.assert_clean();
}
