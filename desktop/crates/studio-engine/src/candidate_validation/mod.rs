//! Immutable candidate capture and acceptance validation.
//!
//! A quiesced draft (proved by a [`CaptureTicket`]) is captured into content-addressed
//! checkpoint objects ([`capture_candidate`]): the draft is scanned, captured and
//! reloaded/verified, then scanned again; any difference fails the capture so a changing
//! tree is never validated. The resulting [`CapturedCandidate`] names the task, its
//! source base, the candidate revision and the manifest/object hashes.
//!
//! [`validate_candidate`] turns what an owned preview worker reports (through the
//! [`CandidateProbe`] trait) into an immutable [`ValidationReport`]: build identity,
//! diagnostics, coverage, representative frame hashes and prepared-audio identity. A
//! cached compiler success alone never passes: the report is `Passed` only when the
//! worker negotiated, the timeline is valid, no critical inspection error exists and
//! real frames and (checked) audio were produced. There is deliberately no pixel
//! difference or title-anchor requirement: an audio-only edit or an unchanged frame zero
//! is legitimate.
//!
//! Failure routing is deterministic ([`next_step`]): a source-attributable failure may
//! spend the single automatic repair of [`RepairBudget`]; environment failures never do.
use crate::{
    CandidateRevision, CaptureTicket, PreviewFrame, RepairBudget, TaskIdentity, TaskScope,
    TaskSourceBase, validate_preview_timeline,
};
use fframes_studio_protocol::{
    DiagnosticSeverity, InspectResponse, MAX_INSPECT_FRAMES, PreparedAudioDescriptor,
    PreviewTimelineResponse,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use studio_project::{
    Manifest, OpenProject, SourceRevision,
    checkpoint::Checkpoints,
    revision::{FileKind, SourceInventory},
};

/// Evenly spaced representative sample (initial maximum).
pub const MAX_SAMPLE_FRAMES: usize = 24;
/// Frames the validator renders and hashes (boundaries + playhead + sample).
pub const MAX_RENDERED_FRAMES: usize = 64;
/// Upper bound of a broadened (full-video) inspection; longer videos are strided and
/// reported as incomplete coverage rather than silently omitted.
pub const MAX_BROADENED_FRAMES: usize = 20_000;
pub const MAX_CHANGED_PATHS: usize = 256;
pub const MAX_REPORT_DIAGNOSTICS: usize = 128;
pub const MAX_FAILURE_PATHS: usize = 128;
pub const MAX_FAILURE_DIAGNOSTICS: usize = 64;
pub const MAX_COMPILER_OUTPUT_BYTES: usize = 16 * 1024;
pub const MAX_REPAIR_PROMPT_BYTES: usize = 24 * 1024;
const MAX_MESSAGE_BYTES: usize = 1024;
const MAX_ACTIVITY_RANGES: usize = 4096;
/// Peak below which a 10 ms bucket counts as silent (about -80 dBFS).
const SILENCE_PEAK: f32 = 1e-4;

mod capture;
mod coverage;
mod probe;
mod report;
mod validation;

pub use capture::*;
pub use coverage::*;
pub use probe::*;
pub use report::*;
pub use validation::*;

// ---- routing ---------------------------------------------------------------------------

/// What the transaction does next with a validation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextStep {
    /// The candidate is validated and may be handed to promotion.
    Accept,
    /// Spend the single automatic repair: reopen the stable draft with this context,
    /// reap the writer again and capture a new immutable candidate.
    Repair {
        attempt: u32,
        context: FailureContext,
    },
    /// Retain the draft for user review; no automatic repair applies.
    Retain {
        reason: String,
        context: Option<FailureContext>,
    },
}

/// Deterministic repair routing over the Stage 1 [`RepairBudget`] (one attempt maximum).
pub fn next_step(report: &ValidationReport, budget: &RepairBudget) -> NextStep {
    let Some(context) = report.failure() else {
        // No recorded failure is not acceptance: the report must be a passing one with
        // complete evidence for exactly its candidate.
        return match report.acceptance_gap() {
            None => NextStep::Accept,
            Some(gap) => NextStep::Retain {
                reason: format!("the validation report is not acceptance evidence: {gap}"),
                context: None,
            },
        };
    };
    match context.kind {
        FailureKind::Environment => NextStep::Retain {
            reason: format!("validation could not complete: {}", context.summary),
            context: Some(context.clone()),
        },
        FailureKind::Source if budget.remaining() > 0 => NextStep::Repair {
            attempt: budget.used() + 1,
            context: context.clone(),
        },
        FailureKind::Source => NextStep::Retain {
            reason: format!(
                "validation failed after the automatic repair attempt: {}",
                context.summary
            ),
            context: Some(context.clone()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Controller, EngineError, QuiescenceEvidence, TaskError, TaskState, TurnCompletion,
        WriterObservation, app_paths::AppPaths, build_materialization::sdk_pin,
    };
    use studio_bootstrap::WriterOwnership;
    use studio_sdk::CompatibilityManifest;

    fn qualified() -> WriterOwnership {
        WriterOwnership::ProcessGroupContained {
            qualification: "test-adapter".into(),
        }
    }

    /// A controller whose task is `Validating` a captured, unchanged candidate.
    fn validating() -> (
        tempfile::TempDir,
        Controller,
        TaskIdentity,
        CapturedCandidate,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("video");
        let paths = AppPaths::new(temp.path().join("history")).unwrap();
        let compatibility = CompatibilityManifest::default_linux_x64();
        studio_project::create(
            &root,
            "Video",
            sdk_pin(&compatibility),
            &compatibility.fframes_version,
            "0.1.0",
        )
        .unwrap();
        let mut controller = Controller::open(&root, &paths).unwrap();
        let context = controller.begin_agent_task("edit").unwrap();
        let id = context.identity.clone();
        let writer = controller.agent_writer_started(&id, qualified()).unwrap();
        controller
            .agent_task_transition(&id, TaskState::Editing)
            .unwrap();
        controller
            .agent_task_transition(&id, TaskState::Quiescing)
            .unwrap();
        let ticket = controller
            .agent_complete_quiescence(
                &id,
                &QuiescenceEvidence {
                    identity: id.clone(),
                    provider_session: None,
                    writer,
                    completion: TurnCompletion::EndTurn,
                    cancel_requested: false,
                    unresolved_requests: 0,
                    observed: WriterObservation {
                        ownership: qualified(),
                        escaped_pids: vec![],
                    },
                },
            )
            .unwrap();
        let captured = controller.agent_capture_candidate(ticket).unwrap();
        (temp, controller, id, captured)
    }

    fn failed_report(captured: &CapturedCandidate) -> ValidationReport {
        validate_candidate(
            captured,
            BuildOutcome::Failed {
                kind: FailureKind::Source,
                stage: ValidationStage::Compile,
                output: "error".into(),
                build: None,
            },
            0,
            0,
            None,
        )
    }

    #[test]
    fn a_report_for_another_manifest_or_candidate_never_advances_the_task() {
        let (_temp, mut controller, id, captured) = validating();
        let report = failed_report(&captured);

        let mut wrong_manifest = report.clone();
        wrong_manifest.manifest_sha256 = "0".repeat(64);
        assert!(matches!(
            controller.agent_apply_validation(&id, &wrong_manifest),
            Err(EngineError::Task(TaskError::ReportMismatch))
        ));
        let mut wrong_candidate = report.clone();
        wrong_candidate.candidate = CandidateRevision::new(
            studio_project::SourceRevision::try_from("9".repeat(64)).unwrap(),
        );
        assert!(matches!(
            controller.agent_apply_validation(&id, &wrong_candidate),
            Err(EngineError::Task(TaskError::ReportMismatch))
        ));
        assert_eq!(
            controller.agent_task().unwrap().state(),
            TaskState::Validating
        );
        // The untampered report routes normally (a source failure spends the repair).
        assert!(matches!(
            controller.agent_apply_validation(&id, &report).unwrap(),
            NextStep::Repair { .. }
        ));
    }

    #[test]
    fn a_failed_report_forged_into_a_pass_has_no_evidence_and_is_not_accepted() {
        let (_temp, mut controller, id, captured) = validating();
        let budget = RepairBudget::automatic();
        let mut forged = failed_report(&captured);
        forged.failure = None;
        // Still verdict Failed with no recorded failure: not acceptance.
        assert!(matches!(
            next_step(&forged, &budget),
            NextStep::Retain { .. }
        ));
        // Verdict flipped as well, but no build/coverage/frames/audio exist.
        forged.verdict = Verdict::Passed;
        assert!(forged.acceptance_gap().is_some());
        assert!(matches!(
            next_step(&forged, &budget),
            NextStep::Retain { .. }
        ));
        assert!(matches!(
            controller.agent_apply_validation(&id, &forged).unwrap(),
            NextStep::Retain { .. }
        ));
        assert_ne!(
            controller.agent_task().map(|t| t.state()),
            Some(TaskState::CandidateReady)
        );
    }
}
