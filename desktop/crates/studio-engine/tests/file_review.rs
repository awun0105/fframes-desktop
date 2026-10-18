//! Reviewing a candidate file by file: per-file text, rejecting files before Apply (the
//! replacement must pass the full validation again) and reverting one file after Apply.

#[path = "support/tx_fixture.rs"]
mod tx_fixture;

use std::fs;
use studio_engine::{
    Controller, EngineError, TaskState,
    candidate_validation::{
        BuildOutcome, CapturedCandidate, ChangeKind, FailureKind, FileText, NotShown,
        ValidationReport, ValidationStage, validate_candidate,
    },
};
use tx_fixture::*;

fn apply(c: &mut Controller, captured: &CapturedCandidate, report: &ValidationReport) -> String {
    c.apply_candidate(captured, report).unwrap().record.id
}

fn revert(c: &mut Controller, target: &str, path: &str) -> Result<(), EngineError> {
    let preparation = c.prepare_revert_path(target, path)?;
    let report = passing_probe::passing_report(preparation.captured());
    c.undo_task(&preparation, &report).map(|_| ())
}

fn two_file_edit(draft: &std::path::Path) {
    fs::write(draft.join("notes.txt"), "edited by the agent\n").unwrap();
    fs::write(draft.join("added.txt"), "added\n").unwrap();
}

/// A report for `captured` whose build failed on the source, as when a kept file needs
/// one the review left out.
fn failing_report(captured: &CapturedCandidate) -> ValidationReport {
    let report = validate_candidate(
        captured,
        BuildOutcome::Failed {
            kind: FailureKind::Source,
            stage: ValidationStage::Compile,
            output: "error[E0583]: file not found for module `scene`".into(),
            build: None,
        },
        0,
        0,
        None,
    );
    assert!(!report.passed());
    report
}

fn paths(c: &Controller, captured: &CapturedCandidate) -> Vec<String> {
    let _ = captured;
    let mut paths: Vec<_> = c
        .candidate_diff_source()
        .unwrap()
        .read()
        .unwrap()
        .into_iter()
        .map(|d| d.path)
        .collect();
    paths.sort();
    paths
}

#[test]
fn rejecting_a_file_the_rest_depends_on_fails_validation_and_blocks_apply() {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let (_, captured, report) = ready(&f, &mut c, "two files", two_file_edit);
    let original = captured.candidate().clone();

    let rejection = c
        .agent_reject_files(&captured, &["added.txt".to_owned()])
        .unwrap();
    assert_eq!(c.agent_task().unwrap().state(), TaskState::Validating);
    let subset = rejection.captured().clone();
    let failed = failing_report(&subset);
    // The report is genuine and for the replacement; it just does not pass.
    assert!(!c.agent_settle_rejection(&rejection, &failed).unwrap());
    // The replacement cannot be applied while it is unsettled ...
    assert!(c.apply_candidate(&subset, &failed).is_err());
    c.agent_abandon_rejection(rejection).unwrap();

    // ... nor afterwards; the task is not in repair and keeps its original candidate.
    let task = c.agent_task().unwrap();
    assert_eq!(task.state(), TaskState::CandidateReady);
    assert_eq!(task.candidate(), Some(&original));
    assert_eq!(task.repair().used(), 0);
    assert!(c.apply_candidate(&subset, &failed).is_err());
    assert!(
        c.apply_candidate(&subset, &passing_probe::passing_report(&subset))
            .is_err(),
        "a subset that never passed validation is never authorized"
    );
    // Nothing was applied, and the original is still applicable.
    assert_eq!(
        fs::read_to_string(f.root.join("notes.txt")).unwrap(),
        "keep me\n"
    );
    assert!(!f.root.join("added.txt").exists());
    apply(&mut c, &captured, &report);
    assert!(f.root.join("added.txt").is_file());
}

#[test]
fn a_valid_subset_applies_exactly_the_accepted_files() {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let (_, captured, _) = ready(&f, &mut c, "many files", |draft| {
        multi_file_edit(draft);
    });
    let rejected = ["added.txt".to_owned(), "old.txt".to_owned()];
    let rejection = c.agent_reject_files(&captured, &rejected).unwrap();
    assert_eq!(rejection.rejected(), ["added.txt", "old.txt"]);
    let subset = rejection.captured().clone();
    let report = passing_probe::passing_report(&subset);
    assert!(c.agent_settle_rejection(&rejection, &report).unwrap());
    assert_eq!(c.agent_task().unwrap().state(), TaskState::CandidateReady);
    assert_eq!(c.agent_task().unwrap().repair().used(), 0);

    // The original candidate is no longer the task's.
    assert!(
        c.apply_candidate(&captured, &passing_probe::passing_report(&captured))
            .is_err()
    );
    let record = c.apply_candidate(&subset, &report).unwrap().record;
    // A mode-only change (run.sh) is a change only where the platform has file modes.
    let mut changed: Vec<_> = record
        .changes
        .iter()
        .map(|d| d.path.as_str())
        .filter(|p| *p != "run.sh")
        .collect();
    changed.sort();
    assert_eq!(changed, ["media/new/clip.txt", "notes.txt"]);
    assert_eq!(
        fs::read_to_string(f.root.join("notes.txt")).unwrap(),
        "edited by the agent\n"
    );
    assert!(f.root.join("media/new/clip.txt").is_file());
    assert!(!f.root.join("added.txt").exists(), "a rejected addition");
    assert!(f.root.join("old.txt").is_file(), "a rejected deletion");
}

#[test]
fn rejecting_everything_or_an_unchanged_path_is_refused_and_changes_nothing() {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let (_, captured, _) = ready(&f, &mut c, "two files", two_file_edit);
    let all = ["added.txt".to_owned(), "notes.txt".to_owned()];
    assert!(c.agent_reject_files(&captured, &all).is_err());
    assert!(
        c.agent_reject_files(&captured, &["private.txt".to_owned()])
            .is_err()
    );
    assert!(c.agent_reject_files(&captured, &[]).is_err());
    assert_eq!(c.agent_task().unwrap().state(), TaskState::CandidateReady);
    assert_eq!(
        c.agent_task().unwrap().candidate(),
        Some(captured.candidate())
    );
}

#[test]
fn reverting_one_file_then_undoing_the_revision_inverts_only_the_rest_across_a_reopen() {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let (_, captured, report) = ready(&f, &mut c, "two files", two_file_edit);
    let id = apply(&mut c, &captured, &report);

    revert(&mut c, &id, "notes.txt").unwrap();
    assert_eq!(
        fs::read_to_string(f.root.join("notes.txt")).unwrap(),
        "keep me\n"
    );
    assert!(f.root.join("added.txt").is_file(), "the other file stays");
    assert_eq!(c.state().task_history().reverted_paths(&id), ["notes.txt"]);
    // The same file cannot be reverted twice.
    assert!(revert(&mut c, &id, "notes.txt").is_err());
    // A path the revision did not change is refused.
    assert!(revert(&mut c, &id, "private.txt").is_err());

    c.close().unwrap();
    drop(c);
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    assert_eq!(c.state().task_history().reverted_paths(&id), ["notes.txt"]);

    let preparation = c.prepare_undo(Some(&id)).unwrap();
    let report = passing_probe::passing_report(preparation.captured());
    c.undo_task(&preparation, &report).unwrap();
    assert_eq!(
        fs::read_to_string(f.root.join("notes.txt")).unwrap(),
        "keep me\n"
    );
    assert!(!f.root.join("added.txt").exists());
    // Everything of the revision is reversed now.
    assert!(c.prepare_undo(Some(&id)).is_err());
}

fn undo_revision(c: &mut Controller, target: &str) -> Result<(), EngineError> {
    let preparation = c.prepare_undo(Some(target))?;
    let report = passing_probe::passing_report(preparation.captured());
    c.undo_task(&preparation, &report).map(|_| ())
}

#[test]
fn undoing_a_revert_after_a_whole_undo_puts_that_file_back_in_effect() {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let (_, captured, report) = ready(&f, &mut c, "two files", two_file_edit);
    let id = apply(&mut c, &captured, &report);

    revert(&mut c, &id, "notes.txt").unwrap();
    let revert_id = c.state().task_history().head_id().unwrap();
    // The whole Undo inverts only what is still in effect and records exactly that.
    undo_revision(&mut c, &id).unwrap();
    let whole = c.state().task_history().head().unwrap().clone();
    assert_eq!(whole.undoes_paths, Some(vec!["added.txt".to_owned()]));
    assert!(!f.root.join("added.txt").exists());
    let mut reverted = c.state().task_history().reverted_paths(&id);
    reverted.sort();
    assert_eq!(reverted, ["added.txt", "notes.txt"]);
    assert!(
        undo_revision(&mut c, &id).is_err(),
        "everything is reversed"
    );

    // Undoing the revert of notes.txt puts the agent's edit back in effect.
    undo_revision(&mut c, &revert_id).unwrap();
    assert_eq!(
        fs::read_to_string(f.root.join("notes.txt")).unwrap(),
        "edited by the agent\n"
    );
    assert_eq!(c.state().task_history().reverted_paths(&id), ["added.txt"]);
    // So it can be reverted again, and a whole Undo has exactly that file left.
    revert(&mut c, &id, "notes.txt").unwrap();
    assert_eq!(
        fs::read_to_string(f.root.join("notes.txt")).unwrap(),
        "keep me\n"
    );
    assert!(undo_revision(&mut c, &id).is_err());
}

#[test]
fn undoing_a_per_file_revert_alone_restores_that_file_and_allows_reverting_it_again() {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let (_, captured, report) = ready(&f, &mut c, "two files", two_file_edit);
    let id = apply(&mut c, &captured, &report);

    revert(&mut c, &id, "notes.txt").unwrap();
    let revert_id = c.state().task_history().head_id().unwrap();
    assert_eq!(
        c.state().task_history().head().unwrap().undoes_paths,
        Some(vec!["notes.txt".to_owned()])
    );
    undo_revision(&mut c, &revert_id).unwrap();
    assert_eq!(
        fs::read_to_string(f.root.join("notes.txt")).unwrap(),
        "edited by the agent\n"
    );
    assert!(c.state().task_history().reverted_paths(&id).is_empty());
    revert(&mut c, &id, "notes.txt").unwrap();
    assert_eq!(c.state().task_history().reverted_paths(&id), ["notes.txt"]);
}

#[test]
fn a_rejection_cannot_be_abandoned_once_the_task_has_ended() {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let (_, captured, _) = ready(&f, &mut c, "two files", two_file_edit);
    let rejection = c
        .agent_reject_files(&captured, &["added.txt".to_owned()])
        .unwrap();
    c.finish_agent_task(captured.identity(), TaskState::Cancelled, "stopped")
        .unwrap();
    // The caller learns the original candidate could not be reinstated (the actor then
    // ends its own task instead of offering the original for review).
    assert!(c.agent_abandon_rejection(rejection).is_err());
}

#[test]
fn reverting_the_last_remaining_files_one_by_one_reverses_the_revision() {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let (_, captured, report) = ready(&f, &mut c, "two files", two_file_edit);
    let id = apply(&mut c, &captured, &report);
    revert(&mut c, &id, "notes.txt").unwrap();
    revert(&mut c, &id, "added.txt").unwrap();
    assert!(!f.root.join("added.txt").exists());
    assert!(c.prepare_undo(Some(&id)).is_err(), "nothing left to undo");
}

#[test]
fn reverting_a_file_edited_outside_the_task_is_refused_with_a_reason() {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let (_, captured, report) = ready(&f, &mut c, "two files", two_file_edit);
    let id = apply(&mut c, &captured, &report);
    fs::write(f.root.join("notes.txt"), "the user edited this\n").unwrap();

    let error = revert(&mut c, &id, "notes.txt").unwrap_err().to_string();
    assert!(error.contains("notes.txt"), "{error}");
    assert!(error.contains("later changes"), "{error}");
    assert_eq!(
        fs::read_to_string(f.root.join("notes.txt")).unwrap(),
        "the user edited this\n"
    );
    // The untouched file can still be reverted on its own.
    revert(&mut c, &id, "added.txt").unwrap();
    assert!(!f.root.join("added.txt").exists());
}

#[test]
fn per_file_text_covers_added_deleted_modified_and_binary_files() {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let (_, captured, report) = ready(&f, &mut c, "all kinds", |draft| {
        fs::write(draft.join("notes.txt"), "edited by the agent\n").unwrap();
        fs::write(draft.join("added.txt"), "added\n").unwrap();
        fs::remove_file(draft.join("old.txt")).unwrap();
        fs::write(draft.join("blob.bin"), [0u8, 159, 146, 150, 0, 1]).unwrap();
    });
    assert_eq!(
        paths(&c, &captured),
        ["added.txt", "blob.bin", "notes.txt", "old.txt"]
    );
    let diffs = c.candidate_diff_source().unwrap().read().unwrap();
    let get = |name: &str| diffs.iter().find(|d| d.path == name).unwrap();

    let notes = get("notes.txt");
    assert_eq!(notes.change, ChangeKind::Modified);
    assert_eq!(notes.before, FileText::Text("keep me\n".into()));
    assert_eq!(notes.after, FileText::Text("edited by the agent\n".into()));
    let added = get("added.txt");
    assert_eq!(added.change, ChangeKind::Added);
    assert_eq!(added.before, FileText::Absent);
    assert_eq!(added.after, FileText::Text("added\n".into()));
    let old = get("old.txt");
    assert_eq!(old.change, ChangeKind::Deleted);
    assert_eq!(old.before, FileText::Text("to be deleted\n".into()));
    assert_eq!(old.after, FileText::Absent);
    let blob = get("blob.bin");
    assert_eq!(blob.change, ChangeKind::Added);
    assert_eq!(blob.before, FileText::Absent);
    assert_eq!(blob.after, FileText::NotShown(NotShown::Binary));
    assert!(NotShown::Binary.reason().contains("binary"));

    // The accepted revision reads the same way, from its recorded changes.
    let id = apply(&mut c, &captured, &report);
    let revision = c.revision_diff_source(&id).unwrap().read().unwrap();
    let get = |name: &str| revision.iter().find(|d| d.path == name).unwrap();
    assert_eq!(
        get("old.txt").before,
        FileText::Text("to be deleted\n".into())
    );
    assert_eq!(get("old.txt").after, FileText::Absent);
    assert_eq!(get("blob.bin").after, FileText::NotShown(NotShown::Binary));
    assert!(c.revision_diff_source("no-such-revision").is_none());
}
