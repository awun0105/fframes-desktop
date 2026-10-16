//! Per-file review through the workflow: rejecting files of a candidate before Apply and
//! reverting one file of an applied revision.
#![allow(dead_code, unused_imports)]
include!("../../crates/studio-engine/tests/support/workflow_world.rs");
use studio_engine::ReviewPolicy;

fn three_files(marker: &str) -> Value {
    json!({
        "text": "done",
        "write": {
            "src/lib.rs": format!("// {marker}\n"),
            FAKE_CONFIG: GOOD_CONFIG,
            "notes.txt": "kept out of the project\n",
        },
    })
}

fn manual_world() -> World {
    let w = World::new();
    w.wf()
        .set_review_policy(ReviewPolicy::ManualReview)
        .unwrap();
    w.wait("the policy", |s| {
        s.review_policy == ReviewPolicy::ManualReview
    });
    w
}

#[test]
fn rejected_files_are_left_out_of_what_apply_publishes() {
    let w = manual_world();
    let original = w.source("src/lib.rs");
    w.submit(&three_files("reviewed"));
    w.wait_phase(TaskPhase::AwaitingReview);

    let diffs = w.wf().candidate_file_diffs().unwrap();
    assert!(diffs.iter().any(|d| d.path == "notes.txt"), "{diffs:?}");

    w.wf().reject_files(&["notes.txt".to_owned()]).unwrap();
    // Wait for the replacement to settle: its After evidence is requested again, so the
    // candidate listing no longer has the rejected file.
    w.poll("the replacement candidate", || {
        w.wf()
            .candidate_file_diffs()
            .is_ok_and(|d| !d.is_empty() && d.iter().all(|d| d.path != "notes.txt"))
            && w.snap()
                .task
                .as_ref()
                .is_some_and(|t| t.phase == TaskPhase::AwaitingReview)
    });
    assert_eq!(w.engine_state(), Some(TaskState::CandidateReady));

    w.wf().apply().unwrap();
    assert_eq!(w.wait_task(None).phase, TaskPhase::Accepted);
    assert_eq!(w.source("src/lib.rs"), "// reviewed\n");
    assert_ne!(w.source("src/lib.rs"), original);
    assert!(!w.root.join("notes.txt").exists());
    w.assert_clean();
}

#[test]
fn stopping_while_the_remaining_files_validate_returns_to_review_of_the_original() {
    let w = manual_world();
    let original = w.source("src/lib.rs");
    w.submit(&three_files("stopped"));
    w.wait_phase(TaskPhase::AwaitingReview);
    let candidate = w.ctl().lock().agent_task().unwrap().candidate().cloned();

    w.compiler.release.store(false, Ordering::SeqCst);
    let before = w.compiler.started.load(Ordering::SeqCst);
    w.wf().reject_files(&["notes.txt".to_owned()]).unwrap();
    w.compiler.wait_started(before + 1);
    w.wait_phase(TaskPhase::Validating);
    w.wf().stop().unwrap();

    // Only the check of the remaining files ended: the task is back in review of the
    // original candidate, which still includes every file.
    w.wait_phase(TaskPhase::AwaitingReview);
    assert_eq!(w.engine_state(), Some(TaskState::CandidateReady));
    assert_eq!(
        w.ctl().lock().agent_task().unwrap().candidate().cloned(),
        candidate
    );
    assert_eq!(w.source("src/lib.rs"), original);
    assert!(!w.root.join("notes.txt").exists());
    let diffs = w.wf().candidate_file_diffs().unwrap();
    assert!(diffs.iter().any(|d| d.path == "notes.txt"), "{diffs:?}");

    w.compiler.release.store(true, Ordering::SeqCst);
    w.wf().apply().unwrap();
    assert_eq!(w.wait_task(None).phase, TaskPhase::Accepted);
    assert_eq!(w.source("src/lib.rs"), "// stopped\n");
    assert_eq!(w.source("notes.txt"), "kept out of the project\n");
    w.assert_clean();
}

#[test]
fn rejecting_every_file_discards_the_candidate() {
    let w = manual_world();
    let original = w.source("src/lib.rs");
    w.submit(&three_files("all rejected"));
    w.wait_phase(TaskPhase::AwaitingReview);
    let every: Vec<String> = w
        .wf()
        .candidate_file_diffs()
        .unwrap()
        .into_iter()
        .map(|d| d.path)
        .collect();
    w.wf().reject_files(&every).unwrap();
    assert_eq!(w.wait_task(None).phase, TaskPhase::Cancelled);
    assert_eq!(w.source("src/lib.rs"), original);
    w.assert_clean();
}

#[test]
fn one_file_of_an_applied_revision_can_be_reverted_alone() {
    let w = World::new();
    let original = w.source("src/lib.rs");
    w.submit(&three_files("applied"));
    assert_eq!(w.wait_task(None).phase, TaskPhase::Accepted);
    assert_eq!(w.source("src/lib.rs"), "// applied\n");
    assert!(w.root.join("notes.txt").exists());

    let snapshot = w.wait("the revision in history", |s| !s.history.is_empty());
    let revision = snapshot.history.last().unwrap().id.clone();
    w.wf().revert_path(&revision, "src/lib.rs").unwrap();
    w.wait("the reverted path in history", |s| {
        s.history
            .iter()
            .any(|h| h.reverted_paths.iter().any(|p| p == "src/lib.rs"))
    });
    assert_eq!(w.source("src/lib.rs"), original);
    assert_eq!(w.source("notes.txt"), "kept out of the project\n");
    w.assert_clean();
}
