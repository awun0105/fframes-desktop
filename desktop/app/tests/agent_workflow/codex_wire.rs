//! The workflow over the Codex app-server wire (`fake-codex.py`): a command still running
//! when its turn ends holds capture back until it completes, and Stop during that wait
//! captures nothing.

use super::*;

fn codex(mode: &'static str) -> World {
    World::with(Options {
        fake_cli: Some((studio_harness::AgentWire::CodexAppServer, mode)),
        ..Options::default()
    })
}

#[test]
fn capture_waits_for_a_command_that_outlives_its_turn() {
    let w = codex("turns");
    w.cli_writes("writes.json", "first");
    w.cli_writes("late-writes.json", "late");
    w.wf().submit("BACKGROUND please make the edit").unwrap();
    w.wait_phase(TaskPhase::WaitingBackground);
    assert_eq!(
        w.compiler.started.load(Ordering::SeqCst),
        0,
        "the pipeline started while the command was still running"
    );
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(task.turns, 1);
    assert_eq!(
        w.source("src/lib.rs"),
        "// late\n",
        "the candidate holds what the command wrote before it completed"
    );
    w.assert_clean();
}

#[test]
fn stop_while_a_command_outlives_its_turn_captures_nothing_and_leaves_no_process() {
    let w = codex("turns");
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
