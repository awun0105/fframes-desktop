//! Manual review and the review policy.

use super::*;

#[test]
fn manual_review_retains_the_candidate_and_apply_publishes_it() {
    let w = World::new();
    w.wf()
        .set_review_policy(studio_engine::ReviewPolicy::ManualReview)
        .unwrap();
    w.wait("the policy", |s| {
        s.review_policy == studio_engine::ReviewPolicy::ManualReview
    });
    let original = w.source("src/lib.rs");
    w.submit(&good("manual"));
    let task = w.wait_phase(TaskPhase::AwaitingReview);
    assert_eq!(task.engine_state, TaskState::CandidateReady);
    assert_eq!(
        w.source("src/lib.rs"),
        original,
        "nothing is published before review"
    );
    assert!(task.review.as_ref().unwrap().apply_blocked.is_none());
    assert!(task.validation.as_ref().unwrap().passed);
    assert!(task.changes.is_some());
    assert_eq!(w.snap().history.len(), 0);
    // Export gives an independent copy of the candidate without publishing it.
    let export = w._temp.path().join("exported");
    w.wf().export_candidate(&export).unwrap();
    w.wait("the export", |s| {
        s.rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Notice { text, .. } if text.starts_with("Exported the candidate")))
    });
    assert_eq!(
        fs::read_to_string(export.join("src/lib.rs")).unwrap(),
        "// manual\n"
    );
    assert_eq!(w.source("src/lib.rs"), original);
    // The candidate's staged playback worker is retained while the review waits.
    assert_eq!(w.engine_state(), Some(TaskState::CandidateReady));
    w.wf().apply().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(w.source("src/lib.rs"), "// manual\n");
    assert_eq!(w.snap().history.len(), 1);
    w.assert_clean();
}

#[test]
fn discarding_a_reviewed_candidate_keeps_the_source_and_the_working_copy() {
    let w = World::new();
    w.wf()
        .set_review_policy(studio_engine::ReviewPolicy::ManualReview)
        .unwrap();
    w.wait("the policy", |s| {
        s.review_policy == studio_engine::ReviewPolicy::ManualReview
    });
    let original = w.source("src/lib.rs");
    w.submit(&good("discard me"));
    w.wait_phase(TaskPhase::AwaitingReview);
    // Stop is a notice, not a discard, while only a review is pending.
    w.wf().stop().unwrap();
    w.wait("the stop notice", |s| {
        s.rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Notice { text, .. } if text.starts_with("Nothing is running")))
    });
    assert_eq!(
        w.snap().task.as_ref().unwrap().phase,
        TaskPhase::AwaitingReview
    );
    w.wf().discard().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Cancelled);
    assert_eq!(w.source("src/lib.rs"), original);
    assert!(matches!(
        task.draft_state,
        Some(DraftState::Retained { .. })
    ));
    assert!(
        w.snap()
            .rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Outcome(o) if o.kind == OutcomeKind::Discarded))
    );
    // The retained working copy can be exported once no writer can remain.
    let export = w._temp.path().join("draft-export");
    w.wf().export_draft(&export).unwrap();
    w.wait("the draft export", |s| {
        s.rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Notice { text, .. } if text.starts_with("Exported the working copy")))
    });
    assert_eq!(
        fs::read_to_string(export.join("src/lib.rs")).unwrap(),
        "// discard me\n"
    );
    w.assert_clean();
}

#[test]
fn a_policy_change_mid_task_only_affects_the_next_task() {
    let w = World::new();
    let mut script = good("running");
    script["wait_file"] = json!("go");
    w.submit(&script);
    w.wait_phase(TaskPhase::Editing);
    w.wait_prompts(1);
    w.wf()
        .set_review_policy(studio_engine::ReviewPolicy::ManualReview)
        .unwrap();
    let s = w.wait("the policy", |s| {
        s.review_policy == studio_engine::ReviewPolicy::ManualReview
    });
    assert_eq!(
        s.task.as_ref().unwrap().review_policy,
        studio_engine::ReviewPolicy::AutoApply,
        "the running task froze auto-Apply when it started"
    );
    w.release("go");
    let first = w.wait_task(None);
    assert_eq!(
        first.phase,
        TaskPhase::Accepted,
        "the running task still auto-applied"
    );
    w.submit(&good("next"));
    let second = w.wait_phase(TaskPhase::AwaitingReview);
    assert_ne!(second.id, first.id);
    assert_eq!(
        second.review_policy,
        studio_engine::ReviewPolicy::ManualReview
    );
}
