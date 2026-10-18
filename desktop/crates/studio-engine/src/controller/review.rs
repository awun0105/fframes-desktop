//! Review of a validated candidate before Apply: reading it file by file and replacing
//! it with a subset that must pass validation again.

use super::Controller;
use crate::{
    EngineError, FileDelta, TaskError, TaskState,
    agent_task::ReplacedCandidate,
    candidate_validation::{
        CapturedCandidate, FileDiff, NextStep, ValidationReport, candidate_file_diffs,
        capture_replacement_candidate, delta_file_diffs, file_changes, next_step,
    },
    edit_transaction::FileState,
};
use std::{collections::BTreeSet, fs, path::PathBuf};
use studio_project::{SourceRevision, checkpoint::Checkpoints};

/// A replacement candidate formed from a task's validated candidate without some of its
/// files, waiting for its own validation report.
///
/// While it exists the task is `Validating` the replacement. It must be settled with
/// [`Controller::agent_settle_rejection`] (or abandoned with
/// [`Controller::agent_abandon_rejection`]) to leave that state.
#[derive(Debug)]
pub struct Rejection {
    captured: CapturedCandidate,
    replaced: ReplacedCandidate,
    rejected: Vec<String>,
}

impl Rejection {
    /// The replacement to validate.
    pub fn captured(&self) -> &CapturedCandidate {
        &self.captured
    }
    /// The paths left out of the replacement.
    pub fn rejected(&self) -> &[String] {
        &self.rejected
    }
}

/// Deletes a scratch tree when dropped.
struct RemoveOnDrop(PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl Controller {
    /// Forms the replacement for the task's validated candidate `current`: the task's
    /// source base plus the after state of every changed file that is not in `rejected`.
    ///
    /// The replacement keeps the task's identity, source base and scope, so once it
    /// passes validation Apply and its authorization checks accept it unchanged. The
    /// task moves from `CandidateReady` to `Validating` and returns only through
    /// [`Controller::agent_settle_rejection`] or [`Controller::agent_abandon_rejection`];
    /// the agent's working copy is untouched. Rejecting every file (nothing would be
    /// applied) or a path the candidate did not change is refused.
    pub fn agent_reject_files(
        &mut self,
        current: &CapturedCandidate,
        rejected: &[String],
    ) -> Result<Rejection, EngineError> {
        let identity = current.identity().clone();
        let task = self.agent.validate(&identity)?;
        if task.state() != TaskState::CandidateReady {
            return Err(TaskError::WrongState {
                expected: TaskState::CandidateReady,
                actual: task.state(),
            }
            .into());
        }
        if task.candidate() != Some(current.candidate())
            || task.candidate_manifest() != Some(current.manifest_sha256())
        {
            return Err(TaskError::ReportMismatch.into());
        }
        let context = task.context();
        let (scope, source_base) = (context.scope.clone(), context.source_base.clone());

        let base = self.checkpoints.load(source_base.revision())?;
        let candidate = self.checkpoints.load(current.candidate().revision())?;
        let changes = file_changes(&base, &candidate);
        let rejected_set: BTreeSet<&str> = rejected.iter().map(String::as_str).collect();
        if let Some(unknown) = rejected_set
            .iter()
            .find(|path| !changes.iter().any(|c| c.path == **path))
        {
            return Err(EngineError::Diagnostic(format!(
                "{unknown} is not a file this candidate changes"
            )));
        }
        let accepted: Vec<_> = changes
            .iter()
            .filter(|change| !rejected_set.contains(change.path))
            .collect();
        if rejected_set.is_empty() {
            return Err(EngineError::Diagnostic("no file was rejected".into()));
        }
        if accepted.is_empty() {
            return Err(EngineError::Diagnostic(
                "every file was rejected; discard the candidate instead".into(),
            ));
        }

        let draft = self.checkpoints.draft(
            source_base.revision(),
            &format!("reject-{}", uuid::Uuid::new_v4().simple()),
        )?;
        // Removed on every exit, a panic included.
        let draft_guard = RemoveOnDrop(draft.clone());
        let formed = (|| -> Result<CapturedCandidate, EngineError> {
            // Deletions first, then the directories they emptied, then the writes: a path
            // may turn from a file into a directory (or back) within one candidate.
            for change in accepted.iter().filter(|c| c.after.is_none()) {
                let target = draft.join(change.path);
                match fs::remove_file(&target) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.into()),
                }
                let mut parent = target.parent();
                while let Some(dir) = parent {
                    if dir == draft || !dir.starts_with(&draft) || fs::remove_dir(dir).is_err() {
                        break;
                    }
                    parent = dir.parent();
                }
            }
            for change in &accepted {
                if let Some(after) = change.after {
                    self.write_object(&draft.join(change.path), &FileState::from(after))?;
                }
            }
            Ok(capture_replacement_candidate(
                &identity,
                &source_base,
                scope,
                &draft,
                &self.checkpoints,
            )?)
        })();
        drop(draft_guard);
        let captured = formed?;
        // The replacement must differ from the base by exactly the accepted files: a
        // rejected file that an accepted one cannot exist without (a directory a new
        // file lives in) would otherwise be silently carried along or dropped.
        let replacement = self.checkpoints.load(captured.candidate().revision())?;
        let formed_paths: BTreeSet<&str> = file_changes(&base, &replacement)
            .into_iter()
            .map(|change| change.path)
            .collect();
        let expected: BTreeSet<&str> = accepted.iter().map(|change| change.path).collect();
        if formed_paths != expected {
            return Err(EngineError::Diagnostic(
                "the remaining files cannot be separated from the rejected ones".into(),
            ));
        }
        let replaced = self.agent.replace_candidate(
            &identity,
            captured.candidate().clone(),
            captured.manifest_sha256().to_owned(),
        )?;
        Ok(Rejection {
            captured,
            replaced,
            rejected: rejected_set.into_iter().map(str::to_owned).collect(),
        })
    }

    /// Checks a replacement against its validation report. `true`: it passed and the task
    /// is `CandidateReady` with the replacement as its candidate; the rejection is spent.
    /// `false`: it did not pass; the task stays `Validating` until the caller gives the
    /// rejection to [`Controller::agent_abandon_rejection`], which reinstates the
    /// candidate it replaced (Apply of the failed replacement is refused by the ordinary
    /// authorization). A report for anything but the replacement is an error and likewise
    /// leaves the task `Validating`. Neither outcome spends the task's repair.
    pub fn agent_settle_rejection(
        &mut self,
        rejection: &Rejection,
        report: &ValidationReport,
    ) -> Result<bool, EngineError> {
        let identity = rejection.captured.identity().clone();
        let task = self.agent.validate(&identity)?;
        if task.state() != TaskState::Validating {
            return Err(TaskError::WrongState {
                expected: TaskState::Validating,
                actual: task.state(),
            }
            .into());
        }
        if report.task() != &identity
            || task.candidate() != Some(report.candidate())
            || report.source_base() != &task.context().source_base
            || task.candidate_manifest() != Some(report.manifest_sha256())
        {
            return Err(TaskError::ReportMismatch.into());
        }
        if matches!(next_step(report, task.repair()), NextStep::Accept) {
            self.agent
                .candidate_validated(&identity, &report.candidate().clone())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Gives up on a replacement (it failed validation, Stop, an error): the candidate it
    /// replaced is the task's again.
    pub fn agent_abandon_rejection(&mut self, rejection: Rejection) -> Result<(), EngineError> {
        Ok(self.agent.restore_candidate(rejection.replaced)?)
    }

    /// Where the task's current candidate and its source base live, for reading them
    /// without holding the controller: `None` when no candidate was captured.
    pub fn candidate_diff_source(&self) -> Option<CandidateDiffSource> {
        let task = self.agent.current()?;
        if task.state() != TaskState::CandidateReady {
            return None;
        }
        Some(CandidateDiffSource {
            history: self.checkpoints.root.clone(),
            base: task.context().source_base.revision().clone(),
            candidate: task.candidate()?.revision().clone(),
        })
    }

    /// The recorded changes of an accepted revision, with the checkpoint store holding
    /// their objects: `None` when `revision` is not in the task history.
    pub fn revision_diff_source(&self, revision: &str) -> Option<RevisionDiffSource> {
        let record = self.record.state.task_history().get(revision)?;
        Some(RevisionDiffSource {
            history: self.checkpoints.root.clone(),
            changes: record.changes.clone(),
        })
    }
}

/// What reading a candidate's files needs, detached from the controller.
#[derive(Debug, Clone)]
pub struct CandidateDiffSource {
    history: PathBuf,
    base: SourceRevision,
    candidate: SourceRevision,
}

impl CandidateDiffSource {
    /// Per-file text of the candidate. Reads files: never call it on a UI thread.
    pub fn read(&self) -> Result<Vec<FileDiff>, EngineError> {
        let checkpoints = Checkpoints::new(&self.history)?;
        Ok(candidate_file_diffs(
            &checkpoints,
            &self.base,
            &self.candidate,
        )?)
    }
}

/// What reading an accepted revision's files needs, detached from the controller.
#[derive(Debug, Clone)]
pub struct RevisionDiffSource {
    history: PathBuf,
    changes: Vec<FileDelta>,
}

impl RevisionDiffSource {
    /// Per-file text of the revision. Reads files: never call it on a UI thread.
    pub fn read(&self) -> Result<Vec<FileDiff>, EngineError> {
        let checkpoints = Checkpoints::new(&self.history)?;
        Ok(delta_file_diffs(&checkpoints, &self.changes))
    }
}
