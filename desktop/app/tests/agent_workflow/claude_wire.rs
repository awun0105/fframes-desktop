//! The workflow over the Claude Code stream-json wire (`fake-claude.py`): capture waits
//! while background work the agent started is live and for the turn the agent continues
//! with by itself.

use super::*;

fn claude(mode: &'static str) -> World {
    World::with(Options {
        fake_cli: Some((studio_harness::AgentWire::ClaudeStreamJson, mode)),
        ..Options::default()
    })
}

impl World {
    /// Has the fake native CLI write `src/lib.rs` (with `marker`) and a good config into
    /// the draft when it reads `name` from its agent directory.
    pub(super) fn cli_writes(&self, name: &str, marker: &str) {
        fs::write(
            self.agent.join(name),
            json!({"src/lib.rs": format!("// {marker}\n"), FAKE_CONFIG: GOOD_CONFIG}).to_string(),
        )
        .unwrap();
    }
}

#[test]
fn capture_waits_for_the_turn_the_agent_continues_with_and_its_permission_card() {
    let w = claude("turns");
    w.cli_writes("writes.json", "first");
    w.cli_writes("late-writes.json", "late");
    w.wf()
        .submit("BACKGROUND GATED please make the edit")
        .unwrap();
    // The agent's own turn asks before its edit; nothing was captured yet.
    let s = w.wait_phase_snapshot(TaskPhase::WaitingPermission);
    assert!(
        s.rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Notice { text, .. }
            if text.contains("continued after a background task"))),
        "{:?}",
        s.rows
    );
    assert_eq!(
        w.compiler.started.load(Ordering::SeqCst),
        0,
        "the pipeline started before the agent's own turn ended"
    );
    let card = s.open_permissions()[0].clone();
    w.wf()
        .reply_permission(&card.reference, PermissionAnswer::Select("allow".into()))
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(task.turns, 2, "the continued turn is a turn of its own");
    assert_eq!(
        w.source("src/lib.rs"),
        "// late\n",
        "the candidate holds the continued turn's edit"
    );
    w.assert_clean();
}

#[test]
fn capture_starts_once_the_agent_does_not_continue_after_its_background_task() {
    let w = claude("no-continue");
    w.cli_writes("writes.json", "first");
    w.wf().submit("BACKGROUND please make the edit").unwrap();
    w.wait_phase(TaskPhase::WaitingBackground);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(task.turns, 1);
    assert_eq!(w.source("src/lib.rs"), "// first\n");
    w.assert_clean();
}

#[test]
fn stop_while_waiting_for_a_background_task_captures_nothing_and_leaves_no_process() {
    let w = claude("turns");
    w.cli_writes("writes.json", "first");
    let before = w.source("src/lib.rs");
    w.wf()
        .submit("BACKGROUND HOLD please make the edit")
        .unwrap();
    w.wait_phase(TaskPhase::WaitingBackground);
    w.wf().stop().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Cancelled, "{:?}", task.error);
    assert_eq!(w.source("src/lib.rs"), before, "nothing was captured");
    assert_eq!(w.compiler.started.load(Ordering::SeqCst), 0);
    w.assert_clean();
}
