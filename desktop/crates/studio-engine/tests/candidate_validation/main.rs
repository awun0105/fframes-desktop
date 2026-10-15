use fframes_studio_protocol::*;
use std::{fs, path::Path};
use studio_bootstrap::WriterOwnership;
use studio_engine::{
    AgentTaskContext, CaptureTicket, CompiledScope, Controller, PreviewFrame, QuiescenceEvidence,
    RepairDecision, ScopeSelection, ScopedScene, TaskScope, TaskState, TurnCompletion,
    WriterObservation, app_paths::AppPaths, build_materialization::sdk_pin,
    candidate_validation::*,
};
use studio_project::checkpoint::Checkpoints;
use studio_sdk::CompatibilityManifest;

mod audio;
mod capture;
mod controller;
mod coverage;
mod failures;

struct Fixture {
    _temp: tempfile::TempDir,
    root: std::path::PathBuf,
    paths: AppPaths,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("video");
    let paths = AppPaths::new(temp.path().join("history")).unwrap();
    let compatibility = CompatibilityManifest::default_linux_x64();
    studio_project::create(
        &root,
        "Video",
        sdk_pin(&compatibility),
        &compatibility.fframes_version,
        compatibility.studio_runtime_version(),
    )
    .unwrap();
    Fixture {
        _temp: temp,
        root,
        paths,
    }
}

fn qualified() -> WriterOwnership {
    WriterOwnership::ProcessGroupContained {
        qualification: "test-adapter".into(),
    }
}

fn evidence(controller: &Controller, context: &AgentTaskContext) -> QuiescenceEvidence {
    QuiescenceEvidence {
        identity: context.identity.clone(),
        provider_session: None,
        writer: controller
            .agent_task()
            .unwrap()
            .writer_generation()
            .expect("a writer was started"),
        completion: TurnCompletion::EndTurn,
        cancel_requested: false,
        unresolved_requests: 0,
        observed: WriterObservation {
            ownership: qualified(),
            escaped_pids: vec![],
        },
    }
}

fn start(controller: &mut Controller) -> AgentTaskContext {
    let context = controller.begin_agent_task("edit").unwrap();
    controller
        .agent_writer_started(&context.identity, qualified())
        .unwrap();
    controller
        .agent_task_transition(&context.identity, TaskState::Editing)
        .unwrap();
    controller
        .agent_task_transition(&context.identity, TaskState::Quiescing)
        .unwrap();
    context
}

fn ticket(controller: &mut Controller, context: &AgentTaskContext) -> CaptureTicket {
    let evidence = evidence(controller, context);
    controller
        .agent_complete_quiescence(&context.identity, &evidence)
        .unwrap()
}

fn checkpoints(f: &Fixture, controller: &Controller) -> Checkpoints {
    Checkpoints::new(&f.paths.project(&controller.project.manifest.project_id)).unwrap()
}

// ---- validation with a deterministic probe ------------------------------------------------

fn identity(revision: &str) -> PreviewIdentity {
    PreviewIdentity {
        project_id: "p".into(),
        open_session: "s".into(),
        source_revision: revision.into(),
        worker_generation: 1,
    }
}

fn envelope(revision: &str) -> PreviewEnvelope {
    PreviewEnvelope {
        contract_version: PREVIEW_CONTRACT_VERSION,
        identity: identity(revision),
        request_id: 1,
    }
}

fn timeline(
    revision: &str,
    total: usize,
    scenes: &[(usize, usize)],
    tracks: &[(f64, f64)],
) -> PreviewTimelineResponse {
    PreviewTimelineResponse {
        timing_generation: 0,
        envelope: envelope(revision),
        fps: 30,
        width: 4,
        height: 2,
        total_frames: total,
        duration_seconds: total as f32 / 30.,
        scenes: scenes
            .iter()
            .enumerate()
            .map(|(i, (s, e))| PreviewSceneInfo {
                clip: None,
                instance_id: format!("scene-{i}"),
                editor_instance_key: None,
                index: i,
                name: format!("s{i}"),
                full_name: format!("s{i}"),
                start_frame: *s,
                end_frame: *e,
                start_seconds: *s as f32 / 30.,
                end_seconds: *e as f32 / 30.,
            })
            .collect(),
        audio_tracks: tracks
            .iter()
            .map(|(s, e)| PreviewAudioTrackInfo {
                file: "cue.wav".into(),
                start_seconds: *s,
                end_seconds: *e,
                mix: TrackMixInfo {
                    gain_db: 0.,
                    pan: 0.,
                    fade_in: 0.,
                    fade_out: 0.,
                    offset: 0.,
                    voice: false,
                    duck_under_voice: false,
                },
            })
            .collect(),
    }
}

fn frame(revision: &str, index: usize, value: u8) -> PreviewFrame {
    let id = identity(revision);
    let header = FrameHeader::new_straight_rgba(revision, 1, 1, index, 4, 2).unwrap();
    PreviewFrame {
        response: ScaledFrameResponse {
            envelope: envelope(revision),
            frame_index: index,
            seek_serial: 0,
            scale: 1.,
            header,
            render_duration_micros: 1,
            record: BinaryRecordHeader {
                kind: BinaryRecordKind::FrameRgba8,
                identity: id,
                request_id: 1,
                offset: 0,
                payload_len: 32,
            },
            editor_metadata: None,
        },
        pixels: vec![value; 32],
    }
}

fn silent_audio(t: &PreviewTimelineResponse, _revision: &str) -> AudioProbe {
    let samples = (t.total_frames as u64 * 48_000).div_ceil(30);
    AudioProbe {
        descriptor: PreparedAudioDescriptor {
            envelope: t.envelope.clone(),
            artifact_id: "pcm-abc".into(),
            sample_rate: 48_000,
            channels: 2,
            sample_count: samples,
            byte_count: samples * 8,
            sha256: "a".repeat(64),
            silent: t.audio_tracks.is_empty(),
        },
        stats: PcmStats {
            sample_frames: samples,
            checked_windows: placement_windows(t, 48_000),
            ..Default::default()
        },
    }
}

struct Fake {
    revision: String,
    timeline: PreviewTimelineResponse,
    timeline_error: Option<ProbeError>,
    diagnostics: Vec<PreviewDiagnostic>,
    truncated: bool,
    render_error_at: Option<usize>,
    audio: Result<AudioProbe, ProbeError>,
    inspect_batches: Vec<usize>,
    inspected: Vec<usize>,
    rendered: Vec<usize>,
    pixel: u8,
    gaps: Vec<String>,
    /// Reports `cancelled()` once this many frames have been rendered (0: at once).
    cancel_after_renders: Option<usize>,
    /// Diagnostics returned by the nth inspection batch (overrides `diagnostics`).
    batch_diagnostics: Vec<(usize, Vec<PreviewDiagnostic>)>,
    /// Batch index whose response claims truncation.
    truncate_batch: Option<usize>,
    /// Frames carry this revision instead of the probed one.
    frame_revision: Option<String>,
}

impl Fake {
    fn new(revision: &str, total: usize) -> Self {
        let t = timeline(revision, total, &[(0, total / 2), (total / 2, total)], &[]);
        Self {
            revision: revision.into(),
            audio: Ok(silent_audio(&t, revision)),
            timeline: t,
            timeline_error: None,
            diagnostics: vec![],
            truncated: false,
            render_error_at: None,
            inspect_batches: vec![],
            inspected: vec![],
            rendered: vec![],
            pixel: 7,
            gaps: vec![],
            cancel_after_renders: None,
            batch_diagnostics: vec![],
            truncate_batch: None,
            frame_revision: None,
        }
    }
    fn with_timeline(mut self, t: PreviewTimelineResponse) -> Self {
        self.audio = Ok(silent_audio(&t, &self.revision));
        self.timeline = t;
        self
    }
}

impl CandidateProbe for Fake {
    fn timeline(&mut self) -> Result<PreviewTimelineResponse, ProbeError> {
        match &self.timeline_error {
            Some(e) => Err(e.clone()),
            None => Ok(self.timeline.clone()),
        }
    }
    fn inspect(&mut self, frames: &[usize]) -> Result<InspectResponse, ProbeError> {
        assert!(frames.len() <= MAX_INSPECT_FRAMES);
        let batch = self.inspect_batches.len();
        self.inspect_batches.push(frames.len());
        self.inspected.extend_from_slice(frames);
        let diagnostics = self
            .batch_diagnostics
            .iter()
            .find(|(n, _)| *n == batch)
            .map_or_else(|| self.diagnostics.clone(), |(_, d)| d.clone());
        Ok(InspectResponse {
            envelope: envelope(&self.revision),
            diagnostics,
            truncated: self.truncated || self.truncate_batch == Some(batch),
        })
    }
    fn render(&mut self, frame_index: usize) -> Result<PreviewFrame, ProbeError> {
        if self.render_error_at == Some(frame_index) {
            return Err(ProbeError::source(format!("panic rendering {frame_index}")));
        }
        self.rendered.push(frame_index);
        let revision = self.frame_revision.as_deref().unwrap_or(&self.revision);
        Ok(frame(revision, frame_index, self.pixel))
    }
    fn audio(&mut self) -> Result<AudioProbe, ProbeError> {
        self.audio.clone()
    }
    fn capability_gaps(&self) -> Vec<String> {
        self.gaps.clone()
    }
    fn cancelled(&self) -> bool {
        self.cancel_after_renders
            .is_some_and(|after| self.rendered.len() >= after)
    }
}

fn build_identity(captured: &CapturedCandidate) -> BuildIdentity {
    BuildIdentity {
        key_digest: "k".repeat(64),
        source_revision: captured.candidate().revision().as_str().into(),
        sdk_id: "sdk".into(),
        compatibility_digest: "d".repeat(64),
        toolchain: "1.98.1".into(),
        target_triple: "x86_64-unknown-linux-gnu".into(),
        package: "video".into(),
        worker_target: "worker".into(),
        profile: "debug".into(),
        backend: "cpu".into(),
    }
}

/// A captured candidate with the given draft edits, plus the live controller (kept for the
/// repair-budget tests).
fn captured_with(
    f: &Fixture,
    edits: impl FnOnce(&Path),
) -> (Controller, AgentTaskContext, CapturedCandidate) {
    let mut controller = Controller::open(&f.root, &f.paths).unwrap();
    let context = start(&mut controller);
    edits(&context.draft);
    let t = ticket(&mut controller, &context);
    let captured = capture_candidate(&t, &checkpoints(f, &controller)).unwrap();
    controller.agent_record_candidate(t, &captured).unwrap();
    (controller, context, captured)
}

fn run(f: &Fixture, probe: &mut Fake, edits: impl FnOnce(&Path)) -> (ValidationReport, Controller) {
    let (controller, _ctx, captured) = captured_with(f, edits);
    let revision = captured.candidate().revision().as_str().to_owned();
    // The worker reports the identity of the revision it was asked to build.
    probe.revision = revision.clone();
    probe.timeline.envelope = envelope(&revision);
    if let Ok(audio) = &mut probe.audio {
        audio.descriptor.envelope = probe.timeline.envelope.clone();
    }
    let report = validate_candidate(
        &captured,
        BuildOutcome::Built(build_identity(&captured)),
        0,
        0,
        Some(probe),
    );
    (report, controller)
}
