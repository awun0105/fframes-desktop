//! Stop in every task state, and New chat.

use super::*;

#[test]
fn new_chat_is_refused_while_a_task_runs_and_archives_the_transcript_when_idle() {
    let w = World::new();
    w.submit(&json!({"text": "starting", "hang": true}));
    w.wait_phase(TaskPhase::Editing);
    w.wf().new_chat().unwrap();
    w.wait("the refusal", |s| {
        s.rows.iter().any(|r| {
            matches!(&r.kind, RowKind::Notice { text, .. } if text.contains("before starting a new chat"))
        })
    });
    assert_eq!(w.snap().chat, 0);

    w.wf().stop().unwrap();
    w.wait_task(None);
    let before = w.snap();
    let last_id = before.rows.last().unwrap().id;
    let history = before.history.len();
    w.wf().new_chat().unwrap();
    let s = w.wait("the new chat", |s| s.chat == 1);
    assert!(s.rows.is_empty(), "{:?}", s.rows);
    assert!(!s.older_rows);
    assert!(s.task.is_none());
    assert!(s.session_restore.is_none());
    assert_eq!(
        s.history.len(),
        history,
        "undo history is not part of the chat"
    );
    let dir = w.paths.project(&s.project);
    let archived = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.starts_with("conversation-") && name.ends_with(".jsonl")
        })
        .count();
    assert_eq!(archived, 1);

    w.submit(&good("after"));
    w.wait_task(None);
    let s = w.snap();
    assert!(
        s.rows.first().unwrap().id > last_id,
        "row ids keep counting up"
    );
    w.assert_clean();
}

#[test]
fn stop_during_editing_reaps_the_adapter_and_retains_the_draft() {
    let w = World::new();
    w.submit(&json!({"text": "starting", "write": {"partial.txt": "half"}, "hang": true}));
    w.wait_phase(TaskPhase::Editing);
    w.poll("the partial write", || {
        w.ctl()
            .lock()
            .agent_draft_store()
            .path()
            .join("partial.txt")
            .exists()
    });
    w.wf().stop().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Cancelled);
    assert!(task.stop_requested);
    assert_eq!(w.engine_state(), Some(TaskState::Cancelled));
    assert!(matches!(w.draft_state(), Some(DraftState::Retained { .. })));
    assert!(
        w.agent.join("cancel.json").exists(),
        "the prompt was cancelled before reaping"
    );
    assert!(
        w.ctl()
            .lock()
            .agent_draft_store()
            .path()
            .join("partial.txt")
            .exists()
    );
    assert_eq!(w.snap().history.len(), 0);
    w.assert_clean();
}

#[test]
fn stop_while_a_permission_is_open_closes_it_as_cancelled() {
    let w = World::new();
    w.submit(&json!({"permission": {"title": "Edit title"}, "write": {"x.txt": "x"}}));
    let s = w.wait_phase_snapshot(TaskPhase::WaitingPermission);
    assert_eq!(s.open_permissions().len(), 1);
    assert_eq!(s.task.as_ref().unwrap().engine_state, TaskState::Waiting);
    let reference = s.open_permissions()[0].reference.clone();
    w.wf().stop().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Cancelled);
    let s = w.snap();
    assert!(s.open_permissions().is_empty());
    assert!(s.rows.iter().any(
        |r| matches!(&r.kind, RowKind::Permission(c) if c.state == PermissionState::Cancelled)
    ));
    // The agent saw the cancelled outcome; a late click is stale, never delivered.
    let outcome = fs::read_to_string(w.agent.join("permission-perm-1.json")).unwrap();
    assert!(outcome.contains("cancelled"), "{outcome}");
    assert_eq!(
        w.wf()
            .reply_permission(&reference, PermissionAnswer::Select("allow".into()))
            .unwrap_err(),
        WorkflowError::StalePermission
    );
    w.assert_clean();
}

#[test]
fn stop_while_waiting_for_an_answer_ends_the_task() {
    let w = World::new();
    w.submit(&json!({"text": "Which title should I use?"}));
    let task = w.wait_phase(TaskPhase::WaitingClarification);
    assert_eq!(task.engine_state, TaskState::Waiting);
    w.wf().stop().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Cancelled);
    assert_eq!(
        w.wf()
            .reply_clarification(&task.id, "too late")
            .unwrap_err(),
        WorkflowError::NotWaiting
    );
    w.assert_clean();
}

#[test]
fn stop_while_validating_cancels_the_compile_and_keeps_the_draft() {
    let w = World::with(Options {
        block_compiler: true,
        ..Options::default()
    });
    w.submit(&good("never compiled"));
    w.compiler.wait_started(1);
    w.wait_phase(TaskPhase::Validating);
    w.wf().stop().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Cancelled);
    assert_eq!(w.engine_state(), Some(TaskState::Cancelled));
    assert!(matches!(w.draft_state(), Some(DraftState::Retained { .. })));
    w.poll("the shared compile to observe cancellation", || {
        w.compiler.killed.load(Ordering::SeqCst) >= 1
    });
    assert!(
        w.compiler.killed.load(Ordering::SeqCst) >= 1,
        "the shared compile saw the cancel"
    );
    assert_eq!(w.snap().history.len(), 0);
    w.compiler.release.store(true, Ordering::SeqCst);
    w.assert_clean();
}

#[test]
fn stop_during_the_repair_turn_ends_the_task() {
    let w = World::new();
    w.submit_plan(&[broken(), json!({"hang": true})]);
    let task = w.wait_phase(TaskPhase::Repairing);
    assert_eq!(task.repair.used, 1);
    assert!(task.repair.in_progress);
    w.wait_prompts(2);
    w.wf().stop().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Cancelled);
    assert_eq!(task.repair.used, 1);
    assert!(matches!(w.draft_state(), Some(DraftState::Retained { .. })));
    w.assert_clean();
}
