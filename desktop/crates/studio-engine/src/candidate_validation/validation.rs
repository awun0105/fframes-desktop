//! Turning a probe's results into a validation report.

use super::*;

/// What the compile/launch step produced before the probe could run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildOutcome {
    Built(BuildIdentity),
    Failed {
        kind: FailureKind,
        stage: ValidationStage,
        /// Compiler, materialization or worker negotiation output.
        output: String,
        build: Option<BuildIdentity>,
    },
}

/// Keep at most `max` bytes at a char boundary: the tail (compiler errors come last) or
/// the head.
pub fn bound_tail(text: &str, max: usize, tail: bool) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    if tail {
        let mut start = text.len() - max;
        while !text.is_char_boundary(start) {
            start += 1;
        }
        text[start..].to_owned()
    } else {
        let mut end = max;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text[..end].to_owned()
    }
}

/// Collects diagnostics. The bounded `diagnostics` list is a *display* buffer; the
/// sticky `errors`/`errors_total`/`first_inspection_error` facts are recorded before any
/// display truncation and are what acceptance reads.
pub(super) struct Collector {
    pub(super) diagnostics: Vec<ReportDiagnostic>,
    pub(super) total: usize,
    /// First [`MAX_FAILURE_DIAGNOSTICS`] error-severity diagnostics, whenever they arrive.
    pub(super) errors: Vec<ReportDiagnostic>,
    pub(super) errors_total: usize,
    pub(super) first_inspection_error: Option<ReportDiagnostic>,
    pub(super) warnings: Vec<String>,
    pub(super) gaps: Vec<String>,
}

impl Collector {
    pub(super) fn new() -> Self {
        Self {
            diagnostics: vec![],
            total: 0,
            errors: vec![],
            errors_total: 0,
            first_inspection_error: None,
            warnings: vec![],
            gaps: vec![],
        }
    }
    pub(super) fn push(&mut self, diagnostic: ReportDiagnostic) {
        self.total += 1;
        if diagnostic.severity == DiagnosticSeverity::Error {
            self.errors_total += 1;
            if diagnostic.stage == ValidationStage::Inspection
                && self.first_inspection_error.is_none()
            {
                self.first_inspection_error = Some(diagnostic.clone());
            }
            if self.errors.len() < MAX_FAILURE_DIAGNOSTICS {
                self.errors.push(diagnostic.clone());
            }
        }
        if self.diagnostics.len() < MAX_REPORT_DIAGNOSTICS {
            self.diagnostics.push(diagnostic);
        }
    }
    pub(super) fn note(
        &mut self,
        stage: ValidationStage,
        severity: DiagnosticSeverity,
        frame: Option<usize>,
        key: &str,
        message: impl Into<String>,
    ) {
        self.push(ReportDiagnostic {
            stage,
            severity,
            frame,
            key: key.to_owned(),
            message: bound_tail(&message.into(), MAX_MESSAGE_BYTES, false),
        });
    }
    pub(super) fn warn(&mut self, message: impl Into<String>) {
        let message = bound_tail(&message.into(), MAX_MESSAGE_BYTES, false);
        if !self.warnings.contains(&message) && self.warnings.len() < MAX_REPORT_DIAGNOSTICS {
            self.warnings.push(message);
        }
    }
}

/// A user Stop or shutdown is never the candidate's fault.
pub(super) fn cancelled_failure(stage: ValidationStage) -> Failure {
    Failure {
        stage,
        kind: FailureKind::Environment,
        summary: "validation was cancelled".into(),
        output: String::new(),
    }
}

pub(super) struct Failure {
    pub(super) stage: ValidationStage,
    pub(super) kind: FailureKind,
    pub(super) summary: String,
    pub(super) output: String,
}

pub(super) fn is_unsupported_shader(key: &str, message: &str) -> bool {
    let text = format!("{key} {message}").to_lowercase();
    text.contains("shader")
        && (text.contains("unsupported")
            || text.contains("not supported")
            || text.contains("fallback")
            || text.contains("cpu"))
}

/// The only backend the owned preview worker validates.
pub const VALIDATED_BACKEND: &str = "cpu";

/// A compile that does not name exactly the captured candidate (or the validated
/// backend) proves nothing about it.
pub(super) fn build_identity_defect(
    captured: &CapturedCandidate,
    identity: &BuildIdentity,
) -> Option<String> {
    let candidate = captured.build_revision.as_str();
    if identity.source_revision != candidate {
        return Some(format!(
            "the build names source revision {} but the candidate is {candidate}",
            identity.source_revision
        ));
    }
    if identity.backend != VALIDATED_BACKEND {
        return Some(format!(
            "the build used backend `{}` but candidates are validated on `{VALIDATED_BACKEND}`",
            identity.backend
        ));
    }
    None
}

/// Run the acceptance checks. `playhead` is the user's current playhead; `repair_used` is
/// [`RepairBudget::used`]. The report never passes without a probe, a build that names
/// the candidate, complete required coverage and real frames and audio.
pub fn validate_candidate(
    captured: &CapturedCandidate,
    build: BuildOutcome,
    playhead: usize,
    repair_used: u32,
    probe: Option<&mut dyn CandidateProbe>,
) -> ValidationReport {
    let mut collector = Collector::new();
    let mut coverage = None;
    let mut frames = vec![];
    let mut audio = None;
    let (identity, failure) = match build {
        BuildOutcome::Failed {
            kind,
            stage,
            output,
            build,
        } => (
            build,
            Some(Failure {
                stage,
                kind,
                summary: format!("{stage:?} failed"),
                output,
            }),
        ),
        BuildOutcome::Built(identity) => {
            let failure = if let Some(defect) = build_identity_defect(captured, &identity) {
                Some(Failure {
                    stage: ValidationStage::Compile,
                    kind: FailureKind::Environment,
                    summary: defect,
                    output: String::new(),
                })
            } else {
                match probe {
                    Some(probe) => run_probe(
                        captured,
                        probe,
                        playhead,
                        &mut collector,
                        &mut coverage,
                        &mut frames,
                        &mut audio,
                    ),
                    None => Some(Failure {
                        stage: ValidationStage::Worker,
                        kind: FailureKind::Environment,
                        summary: "no preview worker was available to validate the candidate".into(),
                        output: String::new(),
                    }),
                }
            };
            (Some(identity), failure)
        }
    };
    let context_for = |f: Failure, collector: &Collector| FailureContext {
        stage: f.stage,
        kind: f.kind,
        summary: bound_tail(&f.summary, MAX_MESSAGE_BYTES * 2, false),
        output_tail: bound_tail(&f.output, MAX_COMPILER_OUTPUT_BYTES, true),
        diagnostics: collector.errors.clone(),
        changed_paths: captured
            .changes
            .entries
            .iter()
            .take(MAX_FAILURE_PATHS)
            .map(|c| c.path.clone())
            .collect(),
        candidate: captured.candidate.revision().as_str().to_owned(),
        source_base: captured.source_base.revision().as_str().to_owned(),
        build_key: identity.as_ref().map(|b| b.key_digest.clone()),
        repair_count: repair_used,
    };
    let failure_context = failure.map(|f| context_for(f, &collector));
    let mut report = ValidationReport {
        task: captured.identity.clone(),
        source_base: captured.source_base.clone(),
        candidate: captured.candidate.clone(),
        build_revision: captured.build_revision.clone(),
        manifest_sha256: captured.manifest_sha256.clone(),
        build: identity.clone(),
        verdict: if failure_context.is_some() {
            Verdict::Failed
        } else {
            Verdict::Passed
        },
        errors: collector.errors.clone(),
        errors_total: collector.errors_total,
        diagnostics: std::mem::take(&mut collector.diagnostics),
        diagnostics_total: collector.total,
        warnings: std::mem::take(&mut collector.warnings),
        capability_gaps: std::mem::take(&mut collector.gaps),
        coverage,
        frames,
        audio,
        changes: captured.changes.clone(),
        repair_count: repair_used,
        failure: failure_context,
    };
    // Last line of defence: whatever the probe claimed, a passing report must carry
    // complete evidence for exactly this candidate.
    if report.verdict == Verdict::Passed
        && let Some(gap) = report.acceptance_gap()
    {
        report.failure = Some(context_for(
            Failure {
                stage: ValidationStage::Worker,
                kind: FailureKind::Environment,
                summary: format!("validation evidence is incomplete: {gap}"),
                output: String::new(),
            },
            &collector,
        ));
        report.verdict = Verdict::Failed;
    }
    report
}

pub(super) fn run_probe(
    captured: &CapturedCandidate,
    probe: &mut dyn CandidateProbe,
    playhead: usize,
    collector: &mut Collector,
    coverage_out: &mut Option<Coverage>,
    frames_out: &mut Vec<FrameArtifact>,
    audio_out: &mut Option<AudioReport>,
) -> Option<Failure> {
    let revision = captured.candidate.revision().as_str();
    let fail = |stage, error: ProbeError, what: &str| Failure {
        stage,
        kind: error.kind,
        summary: format!(
            "{what}: {}",
            bound_tail(&error.message, MAX_MESSAGE_BYTES, false)
        ),
        output: error.message,
    };
    let timeline = match probe.timeline() {
        Ok(t) => t,
        Err(e) => {
            return Some(fail(
                ValidationStage::Timeline,
                e,
                "the compiled timeline is unavailable",
            ));
        }
    };
    if timeline.envelope.identity.source_revision != revision {
        return Some(Failure {
            stage: ValidationStage::Timeline,
            kind: FailureKind::Environment,
            summary: "the timeline was produced for a different revision than the candidate".into(),
            output: String::new(),
        });
    }
    if let Err(message) = validate_preview_timeline(&timeline) {
        collector.note(
            ValidationStage::Timeline,
            DiagnosticSeverity::Error,
            None,
            "timeline_invalid",
            &message,
        );
        return Some(Failure {
            stage: ValidationStage::Timeline,
            kind: FailureKind::Source,
            summary: message.clone(),
            output: message,
        });
    }
    if timeline.total_frames == 0 {
        collector.note(
            ValidationStage::Timeline,
            DiagnosticSeverity::Error,
            None,
            "timeline_empty",
            "the compiled video has no frames",
        );
        return Some(Failure {
            stage: ValidationStage::Timeline,
            kind: FailureKind::Source,
            summary: "the compiled video has no frames".into(),
            output: String::new(),
        });
    }
    for gap in probe.capability_gaps().into_iter().take(16) {
        let gap = bound_tail(&gap, 128, false);
        if gap.to_lowercase().contains("shader") {
            collector.warn(format!(
                "Unsupported shaders: the CPU preview worker reports the capability gap `{gap}`; shader effects in this video cannot be shown faithfully."
            ));
        }
        collector.note(
            ValidationStage::Worker,
            DiagnosticSeverity::Warning,
            None,
            "capability_gap",
            format!("the preview worker declares `{gap}` unavailable"),
        );
        collector.gaps.push(gap);
    }
    let plan = plan_coverage_scoped(
        &timeline,
        playhead,
        &captured.changes,
        Some(captured.scope()),
    );
    let playhead = playhead.min(timeline.total_frames - 1);
    // Inspection in bounded batches.
    let mut batches = 0;
    for batch in plan.inspect_frames.chunks(MAX_INSPECT_FRAMES) {
        if probe.cancelled() {
            return Some(cancelled_failure(ValidationStage::Inspection));
        }
        let response = match probe.inspect(batch) {
            Ok(r) => r,
            Err(e) => return Some(fail(ValidationStage::Inspection, e, "inspection failed")),
        };
        batches += 1;
        if response.truncated {
            collector.note(
                ValidationStage::Inspection,
                DiagnosticSeverity::Error,
                None,
                "inspection_truncated",
                "inspection output was truncated; critical findings cannot be ruled out",
            );
        }
        for d in response.diagnostics {
            if is_unsupported_shader(&d.key, &d.message) {
                collector.warn(format!(
                    "Unsupported shader at frame {}: {} ({}); the CPU preview cannot show it faithfully.",
                    d.frame, d.message, d.key
                ));
            }
            collector.note(
                ValidationStage::Inspection,
                d.severity,
                Some(d.frame),
                &d.key,
                d.message,
            );
        }
    }
    *coverage_out = Some(Coverage {
        total_frames: timeline.total_frames,
        requested_scope: plan.requested_scope.clone(),
        requested_interval: plan.requested_interval,
        requested_frames: plan.requested_frames.clone(),
        boundary_frames: plan.boundary_frames.clone(),
        playhead,
        sample_frames: plan.sample_frames.clone(),
        rendered_frames: plan.rendered_frames.clone(),
        inspected_frames: plan.inspect_frames.len(),
        inspected_frame_indexes: plan.inspect_frames.clone(),
        inspection_batches: batches,
        broadened: plan.broadened,
        complete: plan.complete,
    });
    if !plan.complete {
        collector.warn(
            "Coverage is partial: the video is longer than the bounded full inspection or more frames were representative than the render cap allows.",
        );
    }
    if let Some(first) = collector.first_inspection_error.as_ref() {
        let summary = format!(
            "critical inspection error at frame {}: {}",
            first
                .frame
                .map_or_else(|| "-".to_owned(), |f| f.to_string()),
            first.message
        );
        return Some(Failure {
            stage: ValidationStage::Inspection,
            kind: FailureKind::Source,
            summary,
            output: String::new(),
        });
    }
    // Representative frames.
    for frame in &plan.rendered_frames {
        if probe.cancelled() {
            return Some(cancelled_failure(ValidationStage::Render));
        }
        let rendered = match probe.render(*frame) {
            Ok(f) => f,
            Err(e) => {
                collector.note(
                    ValidationStage::Render,
                    DiagnosticSeverity::Error,
                    Some(*frame),
                    "render_failed",
                    &e.message,
                );
                return Some(fail(
                    ValidationStage::Render,
                    e,
                    &format!("rendering frame {frame} failed"),
                ));
            }
        };
        let header = &rendered.response.header;
        let expected = u64::from(header.width) * u64::from(header.height) * 4;
        if header.width == 0
            || header.height == 0
            || rendered.pixels.len() as u64 != expected
            || rendered.response.frame_index != *frame
        {
            let message = format!("frame {frame} has invalid geometry or index");
            collector.note(
                ValidationStage::Render,
                DiagnosticSeverity::Error,
                Some(*frame),
                "render_invalid",
                &message,
            );
            return Some(Failure {
                stage: ValidationStage::Render,
                kind: FailureKind::Source,
                summary: message.clone(),
                output: message,
            });
        }
        if let Err(message) = rendered.validate(&timeline.envelope.identity) {
            let message =
                format!("frame {frame} does not belong to the candidate timeline: {message}");
            collector.note(
                ValidationStage::Render,
                DiagnosticSeverity::Error,
                Some(*frame),
                "render_identity",
                &message,
            );
            return Some(Failure {
                stage: ValidationStage::Render,
                kind: FailureKind::Environment,
                summary: message.clone(),
                output: message,
            });
        }
        frames_out.push(FrameArtifact {
            frame: *frame,
            width: header.width,
            height: header.height,
            bytes: rendered.pixels.len() as u64,
            sha256: format!("{:x}", Sha256::digest(&rendered.pixels)),
        });
    }
    // Prepared audio and local PCM/timeline checks.
    if probe.cancelled() {
        return Some(cancelled_failure(ValidationStage::Audio));
    }
    let probed = match probe.audio() {
        Ok(a) => a,
        Err(e) => {
            collector.note(
                ValidationStage::Audio,
                DiagnosticSeverity::Error,
                None,
                "audio_unavailable",
                &e.message,
            );
            return Some(fail(ValidationStage::Audio, e, "audio preparation failed"));
        }
    };
    let (report, errors) = check_audio(&timeline, &probed, revision);
    for (key, message) in &errors {
        collector.note(
            ValidationStage::Audio,
            DiagnosticSeverity::Error,
            None,
            key,
            message,
        );
    }
    for (severity, key, message) in audio_diagnostics(&report) {
        let warn = severity != DiagnosticSeverity::Info;
        collector.note(ValidationStage::Audio, severity, None, &key, &message);
        if warn {
            collector.warn(message);
        }
    }
    let placement_unverified = !report.placement_verified;
    if placement_unverified {
        let message = "Track placement was not verified: the PCM scan did not check the compiled audio track windows, so audible content outside them cannot be ruled out.";
        collector.note(
            ValidationStage::Audio,
            DiagnosticSeverity::Error,
            None,
            "audio_placement_unverified",
            message,
        );
        collector.warn(message);
    }
    *audio_out = Some(report);
    if let Some((_, message)) = errors.first() {
        return Some(Failure {
            stage: ValidationStage::Audio,
            kind: FailureKind::Source,
            summary: message.clone(),
            output: errors
                .iter()
                .map(|(_, m)| m.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        });
    }
    if placement_unverified {
        return Some(Failure {
            stage: ValidationStage::Audio,
            kind: FailureKind::Environment,
            summary: "audio track placement could not be verified".into(),
            output: String::new(),
        });
    }
    // Every other check passed on the frames that were examined, but a change whose
    // required coverage exceeds the bounded validator is retained with this limitation
    // instead of being accepted.
    if !plan.complete {
        return Some(Failure {
            stage: ValidationStage::Coverage,
            kind: FailureKind::Environment,
            summary: format!(
                "bounded validation limitation: this change requires more coverage than the validator supports ({} frames; at most {MAX_RENDERED_FRAMES} representative frames are rendered and at most {MAX_BROADENED_FRAMES} frames are inspected). The candidate is retained for review, not accepted.",
                timeline.total_frames
            ),
            output: String::new(),
        });
    }
    None
}

pub(super) fn audio_diagnostics(report: &AudioReport) -> Vec<(DiagnosticSeverity, String, String)> {
    let mut out = vec![];
    if report.clipped_samples > 0 {
        out.push((
            DiagnosticSeverity::Warning,
            "audio_clipping".to_owned(),
            format!(
                "{} samples reach or exceed full scale (peak {:.3})",
                report.clipped_samples, report.peak
            ),
        ));
    }
    if !report.silent && report.peak <= SILENCE_PEAK {
        out.push((
            DiagnosticSeverity::Warning,
            "audio_silent_mix".to_owned(),
            "the mix declares audio tracks but the prepared PCM is silent".to_owned(),
        ));
    }
    for track in report
        .tracks
        .iter()
        .filter(|t| !t.audible && t.end_seconds > t.start_seconds)
        .filter(|_| report.audibility_complete)
    {
        if report.peak > SILENCE_PEAK {
            out.push((
                DiagnosticSeverity::Warning,
                "audio_silent_track".to_owned(),
                format!(
                    "track {} is silent inside its window {:.2}s-{:.2}s",
                    track.file, track.start_seconds, track.end_seconds
                ),
            ));
        }
    }
    out
}

/// Bounded local checks of the prepared PCM against the compiled timeline. Returns the
/// report and the failing checks (`(key, message)`).
pub fn check_audio(
    timeline: &PreviewTimelineResponse,
    probed: &AudioProbe,
    candidate_revision: &str,
) -> (AudioReport, Vec<(String, String)>) {
    let d = &probed.descriptor;
    let stats = &probed.stats;
    let mut errors: Vec<(String, String)> = vec![];
    let mut passed: Vec<String> = vec![];
    let mut check = |name: &str, ok: bool, message: String| {
        if ok {
            passed.push(name.to_owned());
        } else {
            errors.push((format!("audio_{name}"), message));
        }
    };
    check(
        "revision",
        d.envelope.identity.source_revision == candidate_revision
            && timeline.envelope.identity == d.envelope.identity,
        "prepared audio was produced for a different revision/worker than the candidate timeline"
            .into(),
    );
    check(
        "geometry",
        (8000..=192_000).contains(&d.sample_rate) && d.channels == 2,
        format!(
            "unsupported audio geometry {} Hz / {} channels",
            d.sample_rate, d.channels
        ),
    );
    let expected = (timeline.total_frames as u128 * u128::from(d.sample_rate))
        .div_ceil(timeline.fps.max(1) as u128);
    check(
        "duration",
        u128::from(d.sample_count) == expected
            && d.sample_count.checked_mul(8) == Some(d.byte_count),
        format!(
            "prepared audio has {} samples; the compiled duration needs {expected}",
            d.sample_count
        ),
    );
    check(
        "pcm_length",
        stats.sample_frames == d.sample_count,
        format!(
            "scanned PCM has {} sample frames, descriptor says {}",
            stats.sample_frames, d.sample_count
        ),
    );
    check(
        "finite",
        stats.non_finite == 0,
        format!(
            "prepared PCM contains {} non-finite samples",
            stats.non_finite
        ),
    );
    check(
        "silent_flag",
        d.silent == timeline.audio_tracks.is_empty(),
        "the silent flag disagrees with the compiled audio tracks".into(),
    );
    let duration = timeline.total_frames as f64 / timeline.fps.max(1) as f64;
    let tolerance = 1.0 / timeline.fps.max(1) as f64 + 0.05;
    let rate = f64::from(d.sample_rate);
    let in_bounds = timeline.audio_tracks.iter().all(|t| {
        t.start_seconds >= 0.
            && t.end_seconds >= t.start_seconds
            && t.start_seconds <= duration + tolerance
    });
    check(
        "track_bounds",
        in_bounds,
        "an audio track starts beyond the compiled duration".into(),
    );
    // Placement is evidenced by the streaming scan, which checked every active bucket
    // against these windows; it never depends on the bounded activity map. A scan that
    // checked different windows (or none) cannot vouch for placement: not a pass.
    let windows = placement_windows(timeline, d.sample_rate);
    let placement_verified = stats.checked_windows == windows;
    if placement_verified {
        check(
            "track_placement",
            stats.first_outside.is_none(),
            match stats.first_outside {
                Some((start, end)) => format!(
                    "prepared PCM has audible content at {:.2}s-{:.2}s, outside every compiled audio track window",
                    start as f64 / rate,
                    end as f64 / rate
                ),
                None => String::new(),
            },
        );
    }
    let tracks = timeline
        .audio_tracks
        .iter()
        .map(|t| {
            let s = (t.start_seconds * rate) as u64;
            let e = (t.end_seconds * rate) as u64;
            TrackCheck {
                file: bound_tail(&t.file, 256, false),
                start_seconds: t.start_seconds,
                end_seconds: t.end_seconds,
                audible: stats.active.iter().any(|&(a, b)| a < e && b > s),
            }
        })
        .collect();
    (
        AudioReport {
            artifact_id: d.artifact_id.clone(),
            sha256: d.sha256.clone(),
            sample_rate: d.sample_rate,
            channels: d.channels,
            sample_count: d.sample_count,
            silent: d.silent,
            peak: stats.peak,
            clipped_samples: stats.clipped,
            tracks,
            placement_verified,
            audibility_complete: !stats.activity_truncated,
            checks_passed: passed,
        },
        errors,
    )
}
