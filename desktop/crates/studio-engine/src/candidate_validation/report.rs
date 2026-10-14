//! The immutable validation report and its parts.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStage {
    Capture,
    Compile,
    Worker,
    Timeline,
    Inspection,
    Render,
    Audio,
    /// Required coverage exceeds what the bounded validator can inspect or render.
    Coverage,
}

/// Whether the agent could plausibly fix the failure by editing the draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Source,
    /// SDK, cancellation, cache or I/O problems: a repair turn cannot help.
    Environment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportDiagnostic {
    pub stage: ValidationStage,
    pub severity: DiagnosticSeverity,
    pub frame: Option<usize>,
    pub key: String,
    pub message: String,
}

/// Build identity of the compile that produced the validated bytes (from the shared
/// compile service's key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildIdentity {
    pub key_digest: String,
    pub source_revision: String,
    pub sdk_id: String,
    pub compatibility_digest: String,
    pub toolchain: String,
    pub target_triple: String,
    pub package: String,
    pub worker_target: String,
    pub profile: String,
    pub backend: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BroadenReason {
    /// Rust, Cargo, style or configuration changed: shared code may affect any scene.
    SharedSourceChanged,
    /// An existing file was deleted or modified (assets may be shared between scenes).
    ExistingFilesChanged,
    ChangeListTruncated,
    /// Added/changed files of an unclassified kind (fonts, data): their users are unknown.
    OtherFilesChanged,
    /// The compiled timeline names no scenes, or its scenes do not tile the video, so a
    /// scene-targeted sample cannot be trusted to reach every part of it.
    ScenesUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    pub total_frames: usize,
    /// Requested scope and its original half-open interval, if a compiled selection was
    /// attached to the task.
    pub requested_scope: String,
    pub requested_interval: Option<[usize; 2]>,
    /// Scope-specific samples and adjacent boundaries required by validation.
    pub requested_frames: Vec<usize>,
    pub boundary_frames: Vec<usize>,
    pub playhead: usize,
    pub sample_frames: Vec<usize>,
    pub rendered_frames: Vec<usize>,
    pub inspected_frames: usize,
    pub inspected_frame_indexes: Vec<usize>,
    pub inspection_batches: usize,
    pub broadened: Option<BroadenReason>,
    /// False when the video was longer than the broadened bound or rendering was capped.
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameArtifact {
    pub frame: usize,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackCheck {
    pub file: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    /// Non-silent PCM was found inside the track's window.
    pub audible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioReport {
    pub artifact_id: String,
    pub sha256: String,
    pub sample_rate: u32,
    pub channels: u8,
    pub sample_count: u64,
    pub silent: bool,
    pub peak: f32,
    pub clipped_samples: u64,
    pub tracks: Vec<TrackCheck>,
    /// True only when the streaming scan checked every active PCM bucket against exactly
    /// the track windows of the compiled timeline. False is never a pass.
    pub placement_verified: bool,
    /// False when the bounded activity map overflowed: `tracks[].audible` is then
    /// incomplete (never used for acceptance).
    pub audibility_complete: bool,
    /// Names of the local checks that passed.
    pub checks_passed: Vec<String>,
}

/// Bounded context given to the repair turn and to the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureContext {
    pub stage: ValidationStage,
    pub kind: FailureKind,
    pub summary: String,
    /// Tail of the compiler/worker output (at most 16 KiB).
    pub output_tail: String,
    pub diagnostics: Vec<ReportDiagnostic>,
    pub changed_paths: Vec<String>,
    pub candidate: String,
    pub source_base: String,
    pub build_key: Option<String>,
    pub repair_count: u32,
}

impl FailureContext {
    /// Deterministic, bounded text for the automatic repair turn.
    pub fn repair_prompt(&self) -> String {
        let mut text = format!(
            "The previous edit failed validation at the {:?} stage (automatic repair attempt {} of 1).\n\nSummary: {}\n",
            self.stage,
            self.repair_count + 1,
            self.summary
        );
        if !self.changed_paths.is_empty() {
            text.push_str("\nChanged paths:\n");
            for path in &self.changed_paths {
                text.push_str("- ");
                text.push_str(path);
                text.push('\n');
            }
        }
        if !self.diagnostics.is_empty() {
            text.push_str("\nDiagnostics:\n");
            for d in &self.diagnostics {
                text.push_str(&format!(
                    "- [{:?}] frame {}: {}: {}\n",
                    d.severity,
                    d.frame.map_or_else(|| "-".to_owned(), |f| f.to_string()),
                    d.key,
                    d.message
                ));
            }
        }
        if !self.output_tail.is_empty() {
            text.push_str("\nOutput:\n");
            text.push_str(&self.output_tail);
            text.push('\n');
        }
        text.push_str("\nFix the problem in the working directory and finish the turn. Do not change the project id or SDK pin.\n");
        bound_tail(&text, MAX_REPAIR_PROMPT_BYTES, false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Passed,
    Failed,
}

/// Local audio checks every passing report must have passed.
pub(super) const REQUIRED_AUDIO_CHECKS: [&str; 8] = [
    "revision",
    "geometry",
    "duration",
    "pcm_length",
    "finite",
    "silent_flag",
    "track_bounds",
    "track_placement",
];

/// The immutable, revision-bound acceptance record of one candidate.
///
/// Only [`validate_candidate`] can construct one, and it is deliberately **not**
/// `Deserialize`: a report read back from storage or a wire is data for display, never
/// an acceptance proof. [`next_step`] and the controller re-check the evidence anyway.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValidationReport {
    pub(super) task: TaskIdentity,
    pub(super) source_base: TaskSourceBase,
    pub(super) candidate: CandidateRevision,
    pub(super) manifest_sha256: String,
    pub(super) build: Option<BuildIdentity>,
    pub(super) verdict: Verdict,
    /// Display buffer (bounded); acceptance never reads it.
    pub(super) diagnostics: Vec<ReportDiagnostic>,
    pub(super) diagnostics_total: usize,
    /// First error-severity diagnostics, retained regardless of the display bound.
    pub(super) errors: Vec<ReportDiagnostic>,
    pub(super) errors_total: usize,
    pub(super) warnings: Vec<String>,
    pub(super) capability_gaps: Vec<String>,
    pub(super) coverage: Option<Coverage>,
    pub(super) frames: Vec<FrameArtifact>,
    pub(super) audio: Option<AudioReport>,
    pub(super) changes: ChangeSet,
    pub(super) repair_count: u32,
    pub(super) failure: Option<FailureContext>,
}

impl ValidationReport {
    pub fn task(&self) -> &TaskIdentity {
        &self.task
    }
    pub fn source_base(&self) -> &TaskSourceBase {
        &self.source_base
    }
    pub fn candidate(&self) -> &CandidateRevision {
        &self.candidate
    }
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }
    pub fn build(&self) -> Option<&BuildIdentity> {
        self.build.as_ref()
    }
    pub fn verdict(&self) -> Verdict {
        self.verdict
    }
    pub fn passed(&self) -> bool {
        self.verdict == Verdict::Passed
    }
    pub fn diagnostics(&self) -> &[ReportDiagnostic] {
        &self.diagnostics
    }
    pub fn diagnostics_total(&self) -> usize {
        self.diagnostics_total
    }
    /// Error-severity diagnostics (bounded list) kept independently of the display
    /// buffer, so a late critical error is never hidden by earlier warnings.
    pub fn errors(&self) -> &[ReportDiagnostic] {
        &self.errors
    }
    /// Count of every error-severity diagnostic seen, including beyond [`Self::errors`].
    pub fn errors_total(&self) -> usize {
        self.errors_total
    }
    /// Visible warnings (unsupported shaders, clipping, silence, partial coverage...).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
    pub fn coverage(&self) -> Option<&Coverage> {
        self.coverage.as_ref()
    }
    pub fn frames(&self) -> &[FrameArtifact] {
        &self.frames
    }
    pub fn audio(&self) -> Option<&AudioReport> {
        self.audio.as_ref()
    }
    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }
    pub fn repair_count(&self) -> u32 {
        self.repair_count
    }
    /// Capabilities the preview worker declared it lacks (for example `shader_preview`).
    pub fn capability_gaps(&self) -> &[String] {
        &self.capability_gaps
    }
    pub fn failure(&self) -> Option<&FailureContext> {
        self.failure.as_ref()
    }

    /// Why this report cannot be accepted as complete evidence for its candidate, if it
    /// cannot: not passed, a failure recorded, an error seen, a build that does not name
    /// the candidate, incomplete required coverage, missing/mismatched frames or unproven
    /// audio checks. `None` means every acceptance requirement is evidenced.
    pub fn acceptance_gap(&self) -> Option<String> {
        if self.verdict != Verdict::Passed {
            return Some("the verdict is not Passed".into());
        }
        if self.failure.is_some() {
            return Some("a failure is recorded".into());
        }
        if self.errors_total > 0 || !self.errors.is_empty() {
            return Some(format!(
                "{} error diagnostic(s) were reported",
                self.errors_total
            ));
        }
        let Some(build) = &self.build else {
            return Some("no build identity".into());
        };
        let candidate = self.candidate.revision().as_str();
        if build.source_revision != candidate || build.backend != VALIDATED_BACKEND {
            return Some("the build identity does not name this candidate".into());
        }
        let Some(coverage) = &self.coverage else {
            return Some("no coverage".into());
        };
        if !coverage.complete {
            return Some("required coverage is incomplete".into());
        }
        if coverage.total_frames == 0
            || coverage.rendered_frames.is_empty()
            || self.frames.len() != coverage.rendered_frames.len()
            || self
                .frames
                .iter()
                .zip(&coverage.rendered_frames)
                .any(|(f, want)| {
                    f.frame != *want
                        || f.width == 0
                        || f.height == 0
                        || f.bytes != u64::from(f.width) * u64::from(f.height) * 4
                        || f.sha256.is_empty()
                })
        {
            return Some("the representative frames are missing or do not match the plan".into());
        }
        let Some(audio) = &self.audio else {
            return Some("no prepared-audio evidence".into());
        };
        if !audio.placement_verified {
            return Some("audio track placement was not verified".into());
        }
        if let Some(missing) = REQUIRED_AUDIO_CHECKS
            .iter()
            .find(|name| !audio.checks_passed.iter().any(|c| c == *name))
        {
            return Some(format!("the audio check `{missing}` did not pass"));
        }
        None
    }
}
