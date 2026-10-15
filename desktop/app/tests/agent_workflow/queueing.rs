//! Queued briefs: waiting for the writer, bounds, cancellation and Stop.

use super::*;

#[test]
fn a_second_edit_waits_for_the_first_writer_and_starts_from_the_published_source() {
    let w = World::new();
    let mut first = good("one");
    first["wait_file"] = json!("go");
    w.submit(&first);
    w.wait_phase(TaskPhase::Editing);
    w.wait_prompts(1);
    w.submit(&good("two"));
    let s = w.wait("the queued brief", |s| s.queue.len() == 1);
    assert_eq!(
        s.task.as_ref().unwrap().phase,
        TaskPhase::Editing,
        "the first task still runs"
    );
    assert!(s.queue[0].summary.contains("SCRIPT"));
    assert_eq!(
        w.evidence("prompts.jsonl").len(),
        1,
        "the second task did not start yet"
    );
    w.release("go");
    let s = w.wait("both edits accepted", |s| {
        s.history.len() == 2
            && s.task
                .as_ref()
                .is_some_and(|t| t.phase == TaskPhase::Accepted)
            && s.queue.is_empty()
    });
    assert_eq!(w.source("src/lib.rs"), "// two\n");
    // The second task was frozen against the first one's published revision.
    assert_eq!(s.task.as_ref().unwrap().source_base, s.history[0].published);
    let prompts = w.evidence("prompts.jsonl");
    assert_eq!(prompts.len(), 2);
    assert_eq!(
        prompts[0]["cwd"], prompts[1]["cwd"],
        "the stable draft cwd is reused"
    );
    w.assert_clean();
}

#[test]
fn the_queue_is_bounded_cancellable_and_cleared_by_stop() {
    let w = World::new();
    w.submit(&json!({"hang": true}));
    w.wait_phase(TaskPhase::Editing);
    let mut ids = Vec::new();
    for i in 0..MAX_QUEUED_BRIEFS {
        ids.push(w.wf().submit(&format!("queued {i}")).unwrap());
    }
    assert_eq!(
        w.wf().submit("one too many").unwrap_err(),
        WorkflowError::QueueFull
    );
    w.wait("the full queue", |s| s.queue.len() == MAX_QUEUED_BRIEFS);
    w.wf().cancel_queued(ids[0]).unwrap();
    w.wait("a cancelled queue entry", |s| {
        s.queue.len() == MAX_QUEUED_BRIEFS - 1
    });
    w.wf().submit("fits again").unwrap();
    w.wf().stop().unwrap();
    let s = w.wait("the cleared queue", |s| {
        s.queue.is_empty()
            && s.task
                .as_ref()
                .is_some_and(|t| t.phase == TaskPhase::Cancelled)
    });
    assert_eq!(s.task.as_ref().unwrap().phase, TaskPhase::Cancelled);
    // Nothing queued behind it ever started: an actor round trip (a refresh dispatched
    // after Stop and whatever Stop finished) is the barrier, not a sleep.
    let revision = s.revision;
    w.wf().refresh().unwrap();
    let after = w.wait("an actor round trip", |s| s.revision > revision);
    assert!(after.queue.is_empty());
    assert_eq!(
        after.task.as_ref().map(|t| &t.id),
        s.task.as_ref().map(|t| &t.id),
        "no new task replaced the stopped one"
    );
    assert_eq!(w.evidence("prompts.jsonl").len(), 1);
    w.assert_clean();
}
