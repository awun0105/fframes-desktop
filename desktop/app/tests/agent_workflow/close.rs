//! Closing the workflow mid-task.

use super::*;

#[test]
fn closing_the_workflow_mid_task_reaps_everything() {
    let w = World::new();
    w.submit(&json!({"hang": true}));
    w.wait_phase(TaskPhase::Editing);
    w.wait_prompts(1);
    assert_eq!(w.snap().resources.broker_grants, 1);
    w.wf().close();
    assert_eq!(w.processes(), 0);
    assert_eq!(w.engine_state(), Some(TaskState::Interrupted));
    assert!(matches!(w.draft_state(), Some(DraftState::Retained { .. })));
    let s = w.snap();
    assert!(s.closed);
    assert_eq!(s.task.as_ref().unwrap().phase, TaskPhase::Interrupted);
    assert_eq!(s.resources.broker_grants, 0);
    assert!(
        fs::read_dir(w.runtime.path().join("rt"))
            .map(|d| d.count())
            .unwrap_or(0)
            == 0,
        "no socket or capability file remains"
    );
    assert_eq!(w.service.stats().leased_entries, 0);
    w.wf().close();
}

#[test]
fn closing_during_validation_and_during_review_leaves_nothing_behind() {
    let w = World::with(Options {
        block_compiler: true,
        ..Options::default()
    });
    w.submit(&good("closing"));
    w.compiler.wait_started(1);
    w.wait_phase(TaskPhase::Validating);
    w.wf().close();
    assert_eq!(w.processes(), 0);
    assert_eq!(w.engine_state(), Some(TaskState::Interrupted));
    assert_eq!(w.service.stats().in_flight, 0);

    let w = World::new();
    w.wf()
        .set_review_policy(studio_engine::ReviewPolicy::ManualReview)
        .unwrap();
    w.wait("the policy", |s| {
        s.review_policy == studio_engine::ReviewPolicy::ManualReview
    });
    w.submit(&good("review then close"));
    w.wait_phase(TaskPhase::AwaitingReview);
    assert!(
        w.processes() > 0,
        "the staged preview worker is alive during review"
    );
    w.wf().close();
    assert_eq!(w.processes(), 0);
    assert_eq!(w.engine_state(), Some(TaskState::Interrupted));
    w.assert_clean();
}
