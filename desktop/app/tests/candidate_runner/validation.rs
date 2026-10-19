//! Candidate validation against the real preview worker; the old preview keeps playing.

use super::*;

#[test]
fn passing_candidate_is_validated_and_staged_while_the_old_preview_keeps_playing() {
    let mut w = world();
    let (coordinator, mut state) = display_base(&mut w);
    let displayed = state.displayed().unwrap().clone();
    let owned_before = coordinator.metrics().owned_processes;
    let artifact_before = w.service.stats().artifact_bytes;
    let context = w.start();
    worker_config(
        &context.draft,
        json!({"frames": 120, "tracks": [[0.0, 2.0]], "audio": "tone", "pixel": 40}),
    );
    fs::write(context.draft.join("src/lib.rs"), "// new scene\n").unwrap();
    let captured = w.capture(&context);
    let run = w.run(&captured, 0);
    assert!(run.report.passed(), "{:?}", run.report.failure());
    let staged = run.staged.as_ref().expect("a passing candidate is staged");
    // The staged playback matches the candidate, not the task base.
    let candidate = captured.candidate().revision().as_str();
    assert_eq!(staged.ready().identity().source_revision, candidate);
    assert_ne!(candidate, captured.source_base().revision().as_str());
    assert_eq!(staged.ready().timeline.total_frames, 120);
    assert!(staged.ready().frame.is_some());
    assert!(staged.ready().audio_source.is_some());
    assert_eq!(run.report.audio().unwrap().sample_rate, 48_000);
    assert!(run.report.build().is_some());
    // The candidate's prepared PCM is part of the shared build budget while the staged
    // source (an open file) exists...
    let pcm_bytes = run.report.audio().unwrap().sample_count * 8;
    assert!(pcm_bytes > 0);
    assert_eq!(
        w.service.stats().artifact_bytes,
        artifact_before + pcm_bytes
    );
    assert_eq!(run.report.candidate(), captured.candidate());
    // Nothing was published: the displayed preview is unchanged and still seekable.
    assert_eq!(state.displayed().unwrap(), &displayed);
    assert!(coordinator.events().ready.is_none());
    seek_and_wait(&coordinator, &mut state, 7);
    // The validation worker lives in its own scope, not in the coordinator's lanes.
    assert_eq!(coordinator.metrics().owned_processes, owned_before);
    drop(run);
    // ...and is returned to it once the staged candidate is dropped (its worker and lease
    // are released by the background teardown owner, never on the dropping thread).
    assert!(
        studio_engine::services::teardown::Teardown::global().wait_idle(Duration::from_secs(30))
    );
    assert_eq!(w.service.stats().artifact_bytes, artifact_before);
    coordinator.close();
}

/// Run a passing candidate whose compiled timeline is `width`x`height`; returns the run.
fn run_sized(width: u32, height: u32) -> studio_engine::services::candidate_runner::CandidateRun {
    let mut w = world();
    let context = w.start();
    worker_config(
        &context.draft,
        json!({"frames": 60, "width": width, "height": height, "vary_pixels": true}),
    );
    fs::write(context.draft.join("src/lib.rs"), "// sized scene\n").unwrap();
    let captured = w.capture(&context);
    w.run(&captured, 0)
}

#[test]
fn candidates_above_the_preview_cap_validate_and_stage_at_the_effective_scale() {
    // (compiled size, clamped preview size, effective scale): the real worker's clamp is
    // min(1280/w, 720/h, 1) and its response carries that scale.
    type Case = ((u32, u32), (u32, u32), f64);
    let cases: [Case; 4] = [
        ((640, 360), (640, 360), 1.),
        ((1920, 1080), (1280, 720), 1280. / 1920.),
        ((3840, 2160), (1280, 720), 1280. / 3840.),
        // Height binds, not width.
        ((1280, 1440), (640, 720), 0.5),
    ];
    for ((width, height), clamped, scale) in cases {
        let run = run_sized(width, height);
        assert!(
            run.report.passed(),
            "{width}x{height}: {:?}",
            run.report.failure()
        );
        assert!(!run.report.frames().is_empty());
        for frame in run.report.frames() {
            assert_eq!(
                (frame.width, frame.height),
                clamped,
                "{width}x{height} representative frame {}",
                frame.frame
            );
        }
        let staged = run.staged.as_ref().expect("a passing candidate is staged");
        let ready = staged.ready();
        assert_eq!(
            (ready.timeline.width, ready.timeline.height),
            (width as usize, height as usize)
        );
        let first = ready.frame.as_ref().expect("staged playhead frame");
        assert_eq!(first.response.scale, scale, "{width}x{height}");
        assert_eq!(
            (first.response.header.width, first.response.header.height),
            clamped
        );
    }
}

#[test]
fn compile_failure_reports_bounded_context_and_old_playback_survives() {
    let mut w = world();
    let (coordinator, mut state) = display_base(&mut w);
    let context = w.start();
    fs::write(context.draft.join("src/lib.rs"), "fn broken(").unwrap();
    let captured = w.capture(&context);
    w.compiler.fail.store(true, Ordering::SeqCst);
    let run = w.run(&captured, 0);
    assert!(run.staged.is_none());
    assert!(!run.report.passed());
    let failure = run.report.failure().unwrap();
    assert_eq!(failure.stage, ValidationStage::Compile);
    assert_eq!(failure.kind, FailureKind::Source);
    assert!(failure.output_tail.contains("E0425"));
    assert!(failure.changed_paths.contains(&"src/lib.rs".to_owned()));
    assert_eq!(failure.candidate, captured.candidate().revision().as_str());
    assert!(failure.build_key.is_some());
    // The failed validation is a repair candidate through the real Stage 1 budget.
    let budget = w.controller.agent_task().unwrap().repair().clone();
    assert!(matches!(
        next_step(&run.report, &budget),
        NextStep::Repair { attempt: 1, .. }
    ));
    seek_and_wait(&coordinator, &mut state, 11);
    coordinator.close();
}

#[test]
fn inspection_errors_worker_start_failures_and_cancellation_route_deterministically() {
    let mut w = world();
    let context = w.start();
    worker_config(
        &context.draft,
        json!({"inspect": [{"frame": 0, "severity": "error", "key": "missing_asset", "message": "media/absent.jpg"}]}),
    );
    let captured = w.capture(&context);
    let run = w.run(&captured, 0);
    let failure = run.report.failure().unwrap();
    assert_eq!(failure.stage, ValidationStage::Inspection);
    assert!(failure.summary.contains("absent.jpg"));
    assert!(run.staged.is_none());
    assert_eq!(
        w.controller.processes.active_count(),
        0,
        "worker reaped after failure"
    );

    // A worker that cannot start is the candidate's problem and may be repaired.
    let mut w = world();
    let context = w.start();
    worker_config(&context.draft, json!({"fail_start": true}));
    let captured = w.capture(&context);
    let run = w.run(&captured, 0);
    let failure = run.report.failure().unwrap();
    assert_eq!(
        (failure.stage, failure.kind),
        (ValidationStage::Worker, FailureKind::Source)
    );
    assert_eq!(w.controller.processes.active_count(), 0);

    // Cancelling the compile scope is environmental: it never spends the repair.
    let mut w = world();
    let context = w.start();
    fs::write(context.draft.join("src/lib.rs"), "// slow\n").unwrap();
    let captured = w.capture(&context);
    w.compiler.release.store(false, Ordering::SeqCst);
    let scopes = w.scopes();
    let compiler_scope = scopes.compiler.clone();
    let canceller = std::thread::spawn({
        let compiler = w.compiler.clone();
        move || {
            compiler.wait_started(1);
            compiler_scope.shutdown(Duration::ZERO);
        }
    });
    let run = run_candidate_validation(&captured, &w.checkpoints(), &w.config(), &scopes, 0, 0);
    canceller.join().unwrap();
    let failure = run.report.failure().unwrap();
    assert_eq!(failure.kind, FailureKind::Environment);
    let budget = w.controller.agent_task().unwrap().repair().clone();
    assert!(matches!(
        next_step(&run.report, &budget),
        NextStep::Retain { .. }
    ));
    let deadline = Instant::now() + Duration::from_secs(5);
    while w.compiler.killed.load(Ordering::SeqCst) == 0 {
        assert!(
            Instant::now() < deadline,
            "the abandoned compile was not killed"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn one_automatic_repair_then_terminal_with_the_second_failure_retained() {
    let mut w = world();
    let context = w.start();
    worker_config(&context.draft, json!({"render_fail_frame": 0}));
    let first = w.capture(&context);
    let run = w.run(&first, 0);
    assert_eq!(run.report.failure().unwrap().stage, ValidationStage::Render);
    assert!(matches!(
        w.controller
            .agent_validation_failed(&context.identity, "render failed")
            .unwrap(),
        RepairDecision::Repair { attempt: 1 }
    ));
    w.controller.agent_begin_repair(&context.identity).unwrap();
    // The repair reopens the same stable draft and captures a NEW immutable candidate.
    w.reopen(&context);
    worker_config(&context.draft, json!({"render_fail_frame": 89}));
    let second = w.capture(&context);
    assert_ne!(first.candidate(), second.candidate());
    let spent = w.controller.agent_task().unwrap().repair().clone();
    assert_eq!(spent.used(), 1);
    let run = w.run(&second, spent.used());
    assert_eq!(run.report.repair_count(), 1);
    match next_step(&run.report, &spent) {
        NextStep::Retain {
            context: Some(c), ..
        } => assert_eq!(c.repair_count, 1),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        w.controller
            .agent_validation_failed(&context.identity, "still failing")
            .unwrap(),
        RepairDecision::Exhausted
    );
    assert_eq!(
        w.controller.agent_task().unwrap().state(),
        TaskState::Failed
    );
}

#[test]
fn audio_only_edit_with_unchanged_video_passes_and_misplaced_audio_fails() {
    let mut w = world();
    let context = w.start();
    // Baseline: video only.
    worker_config(&context.draft, json!({"frames": 90, "pixel": 9}));
    let baseline = w.capture(&context);
    let baseline_run = w.run(&baseline, 0);
    assert!(baseline_run.report.passed());
    let frame_zero = baseline_run.report.frames()[0].sha256.clone();
    drop(baseline_run);
    w.controller
        .agent_validation_failed(&context.identity, "retry with audio")
        .unwrap();
    w.controller.agent_begin_repair(&context.identity).unwrap();
    w.reopen(&context);
    // Audio-only change: same pixels, a track now exists.
    worker_config(
        &context.draft,
        json!({"frames": 90, "pixel": 9, "tracks": [[0.5, 2.5]], "audio": "tone"}),
    );
    fs::write(context.draft.join("media/cue.wav"), b"RIFF new cue").unwrap();
    let audio_edit = w.capture(&context);
    let run = w.run(&audio_edit, 1);
    assert!(run.report.passed(), "{:?}", run.report.failure());
    assert_eq!(
        run.report.frames()[0].sha256,
        frame_zero,
        "frame zero is unchanged and that is fine"
    );
    let audio = run.report.audio().unwrap();
    assert!(audio.tracks[0].audible);
    assert!(audio.checks_passed.iter().any(|c| c == "track_placement"));
    drop(run);

    // Audible content outside every declared track window fails the placement check.
    let mut w = world();
    let context = w.start();
    worker_config(
        &context.draft,
        json!({"frames": 150, "tracks": [[0.0, 1.0]], "audio": "misplaced"}),
    );
    let captured = w.capture(&context);
    let run = w.run(&captured, 0);
    let failure = run.report.failure().unwrap();
    assert_eq!(failure.stage, ValidationStage::Audio);
    assert!(failure.summary.contains("outside"));
    assert!(run.staged.is_none());
}

#[test]
fn clipping_is_a_visible_warning_and_a_custom_main_import_needs_no_video_cli() {
    let mut w = world();
    let context = w.start();
    // The project's own main is not a video CLI: only the worker entry matters.
    fs::write(
        context.draft.join("src/main.rs"),
        "fn main() { println!(\"custom tool\"); }\n",
    )
    .unwrap();
    worker_config(
        &context.draft,
        json!({"tracks": [[0.0, 2.0]], "audio": "clip"}),
    );
    let captured = w.capture(&context);
    let run = w.run(&captured, 0);
    assert!(run.report.passed(), "{:?}", run.report.failure());
    assert!(
        run.report
            .diagnostics()
            .iter()
            .any(|d| d.key == "audio_clipping")
    );
    assert!(
        run.report
            .warnings()
            .iter()
            .any(|m| m.contains("full scale"))
    );
}

#[test]
fn an_unchanged_candidate_joins_the_ui_build_instead_of_recompiling() {
    let mut w = world();
    let (coordinator, _state) = display_base(&mut w);
    assert_eq!(w.compiler.started.load(Ordering::SeqCst), 1);
    let context = w.start();
    let captured = w.capture(&context);
    assert_eq!(
        captured.candidate().revision(),
        captured.source_base().revision()
    );
    let run = w.run(&captured, 0);
    assert!(run.report.passed(), "{:?}", run.report.failure());
    assert_eq!(
        w.compiler.started.load(Ordering::SeqCst),
        1,
        "equal key shared with the UI"
    );
    assert_eq!(w.service.stats().cache_hits, 1);
    coordinator.close();
}

#[test]
fn an_incompatible_worker_is_an_environment_failure_no_repair_can_fix() {
    let mut w = world();
    let (coordinator, mut state) = display_base(&mut w);
    let context = w.start();
    worker_config(&context.draft, json!({"backend": "gpu"}));
    let captured = w.capture(&context);
    let run = w.run(&captured, 0);
    let failure = run.report.failure().expect("negotiation must fail");
    assert_eq!(
        (failure.stage, failure.kind),
        (ValidationStage::Worker, FailureKind::Environment)
    );
    assert!(failure.output_tail.contains("Incompatible preview SDK"));
    let budget = w.controller.agent_task().unwrap().repair().clone();
    assert!(matches!(
        next_step(&run.report, &budget),
        NextStep::Retain { .. }
    ));
    assert!(run.staged.is_none());
    seek_and_wait(&coordinator, &mut state, 4);
    coordinator.close();
}

#[test]
fn the_workers_declared_shader_gap_reaches_the_report_as_a_visible_warning() {
    let mut w = world();
    let context = w.start();
    worker_config(
        &context.draft,
        json!({"capability_gaps": ["shader_preview"]}),
    );
    let captured = w.capture(&context);
    let run = w.run(&captured, 0);
    assert!(run.report.passed(), "{:?}", run.report.failure());
    assert_eq!(run.report.capability_gaps(), ["shader_preview".to_owned()]);
    assert!(
        run.report
            .warnings()
            .iter()
            .any(|m| m.contains("Unsupported shaders") && m.contains("`cpu`"))
    );
    // The preview still stages: the limitation is visible, not fatal.
    assert!(run.staged.is_some());
}

#[test]
fn a_skia_worker_without_the_shader_gap_reports_no_shader_warning() {
    let mut w = world();
    let context = w.start();
    worker_config(&context.draft, json!({"backend": "skia-cpu"}));
    let captured = w.capture(&context);
    let run = w.run(&captured, 0);
    assert!(run.report.passed(), "{:?}", run.report.failure());
    assert!(run.report.capability_gaps().is_empty());
    assert!(
        run.report.warnings().iter().all(|m| !m.contains("shader")),
        "{:?}",
        run.report.warnings()
    );
    assert!(run.staged.is_some());
}

#[test]
fn stop_during_validation_is_environmental_and_leaves_the_old_preview_alone() {
    let mut w = world();
    let (coordinator, mut state) = display_base(&mut w);
    let displayed = state.displayed().unwrap().clone();
    let context = w.start();
    // Every worker request takes 40 ms, so the run is still rendering when Stop lands.
    worker_config(&context.draft, json!({"delay_ms": 40}));
    let captured = w.capture(&context);
    let scopes = w.scopes();
    let stop = scopes.worker.clone();
    let stopper = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        stop.shutdown(Duration::ZERO);
    });
    let run = run_candidate_validation(&captured, &w.checkpoints(), &w.config(), &scopes, 0, 0);
    stopper.join().unwrap();
    let failure = run.report.failure().expect("stopped");
    assert_eq!(failure.kind, FailureKind::Environment);
    assert!(run.staged.is_none());
    let budget = w.controller.agent_task().unwrap().repair().clone();
    assert!(matches!(
        next_step(&run.report, &budget),
        NextStep::Retain { .. }
    ));
    assert_eq!(budget.used(), 0);
    assert_eq!(state.displayed().unwrap(), &displayed);
    seek_and_wait(&coordinator, &mut state, 9);
    coordinator.close();
}

#[test]
fn old_playback_survives_every_kind_of_validation_failure() {
    let mut w = world();
    let (coordinator, mut state) = display_base(&mut w);
    let displayed = state.displayed().unwrap().clone();
    let cases: Vec<(Value, ValidationStage, FailureKind)> = vec![
        (
            json!({"inspect": [{"frame": 0, "severity": "error", "key": "missing_asset", "message": "media/absent.jpg"}]}),
            ValidationStage::Inspection,
            FailureKind::Source,
        ),
        (
            json!({"render_fail_frame": 0}),
            ValidationStage::Render,
            FailureKind::Source,
        ),
        (
            json!({"frames": 150, "tracks": [[0.0, 1.0]], "audio": "misplaced"}),
            ValidationStage::Audio,
            FailureKind::Source,
        ),
        (
            json!({"fail_start": true}),
            ValidationStage::Worker,
            FailureKind::Source,
        ),
        (
            json!({"backend": "gpu"}),
            ValidationStage::Worker,
            FailureKind::Environment,
        ),
    ];
    for (i, (config, stage, kind)) in cases.into_iter().enumerate() {
        let context = w.start();
        worker_config(&context.draft, config);
        fs::write(context.draft.join("src/lib.rs"), format!("// case {i}\n")).unwrap();
        let captured = w.capture(&context);
        let run = w.run(&captured, 0);
        let failure = run.report.failure().expect("the case must fail");
        assert_eq!((failure.stage, failure.kind), (stage, kind), "case {i}");
        assert!(run.staged.is_none());
        assert_eq!(state.displayed().unwrap(), &displayed, "case {i}");
        assert!(coordinator.events().ready.is_none(), "case {i}");
        seek_and_wait(&coordinator, &mut state, 3 + i);
        drop(run);
        w.controller
            .finish_agent_task(&context.identity, TaskState::Failed, "case done")
            .unwrap();
    }
    coordinator.close();
}

#[test]
fn a_draft_that_changes_during_capture_fails_capture_and_keeps_the_old_preview() {
    let mut w = world();
    let (coordinator, mut state) = display_base(&mut w);
    let displayed = state.displayed().unwrap().clone();
    let context = w.start();
    let ticket = w.ticket(&context);
    let draft = context.draft.clone();
    let outcome = capture_candidate_observed(&ticket, &w.checkpoints(), &mut |phase| {
        if phase == CapturePhase::Scanned {
            fs::write(draft.join("src/lib.rs"), "// changed under the capture\n").unwrap();
        }
    });
    assert_eq!(outcome.err(), Some(CandidateError::DraftChanged));
    // No candidate was registered and nothing about playback moved.
    assert!(w.controller.agent_task().unwrap().candidate().is_none());
    assert_eq!(state.displayed().unwrap(), &displayed);
    seek_and_wait(&coordinator, &mut state, 5);
    coordinator.close();
}
