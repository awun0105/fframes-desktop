//! Failure routing: repairs, environment failures and what never passes.

use super::*;

#[test]
fn compile_failure_routes_to_one_repair_then_retains_with_bounded_context() {
    let f = fixture();
    let (mut controller, context, captured) = captured_with(&f, |draft| {
        fs::write(draft.join("src/lib.rs"), "fn broken(").unwrap();
        for i in 0..300 {
            fs::write(draft.join(format!("media/extra-{i:03}.txt")), "x").unwrap();
        }
    });
    let mut output = String::from("HEAD-MARKER ");
    output.push_str(&"error[E0001]: expected `)`\n".repeat(200_000));
    output.push_str("TAIL-MARKER");
    let failed = |used| {
        validate_candidate(
            &captured,
            BuildOutcome::Failed {
                kind: FailureKind::Source,
                stage: ValidationStage::Compile,
                output: output.clone(),
                build: Some(build_identity(&captured)),
            },
            0,
            used,
            None,
        )
    };
    let report = failed(0);
    assert!(!report.passed());
    let ctx = report.failure().unwrap();
    assert_eq!(ctx.stage, ValidationStage::Compile);
    assert!(ctx.output_tail.len() <= MAX_COMPILER_OUTPUT_BYTES);
    assert!(ctx.output_tail.ends_with("TAIL-MARKER"));
    assert!(!ctx.output_tail.contains("HEAD-MARKER"));
    assert!(ctx.changed_paths.len() <= MAX_FAILURE_PATHS);
    assert_eq!(ctx.candidate, captured.candidate().revision().as_str());
    assert_eq!(ctx.source_base, context.source_base.revision().as_str());
    assert_eq!(ctx.build_key.as_deref(), Some("k".repeat(64).as_str()));
    assert_eq!(ctx.repair_count, 0);
    assert!(report.changes().truncated || report.changes().total > MAX_FAILURE_PATHS);
    let prompt = ctx.repair_prompt();
    assert!(prompt.len() <= MAX_REPAIR_PROMPT_BYTES);
    assert!(prompt.contains("TAIL-MARKER") || prompt.contains("expected `)`"));

    // Real Stage 1 budget: first failure repairs, the second exhausts the single repair.
    let task_budget = controller.agent_task().unwrap().repair().clone();
    match next_step(&report, &task_budget) {
        NextStep::Repair { attempt: 1, .. } => {}
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        controller
            .agent_validation_failed(&context.identity, "compile")
            .unwrap(),
        RepairDecision::Repair { attempt: 1 }
    ));
    controller.agent_begin_repair(&context.identity).unwrap();
    let spent = controller.agent_task().unwrap().repair().clone();
    assert_eq!(spent.used(), 1);
    let second = failed(spent.used());
    assert_eq!(second.failure().unwrap().repair_count, 1);
    match next_step(&second, &spent) {
        NextStep::Retain { reason, .. } => assert!(reason.contains("after the automatic repair")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn environment_failures_never_spend_the_repair_and_a_missing_probe_never_passes() {
    let f = fixture();
    let (controller, _ctx, captured) = captured_with(&f, |_| {});
    let budget = controller.agent_task().unwrap().repair().clone();
    let cancelled = validate_candidate(
        &captured,
        BuildOutcome::Failed {
            kind: FailureKind::Environment,
            stage: ValidationStage::Compile,
            output: "Build cancelled".into(),
            build: None,
        },
        0,
        0,
        None,
    );
    assert!(matches!(
        next_step(&cancelled, &budget),
        NextStep::Retain { .. }
    ));
    // A cached compiler success without a worker probe is not acceptance.
    let unprobed = validate_candidate(
        &captured,
        BuildOutcome::Built(build_identity(&captured)),
        0,
        0,
        None,
    );
    assert!(!unprobed.passed());
    assert_eq!(unprobed.failure().unwrap().kind, FailureKind::Environment);
    assert!(matches!(
        next_step(&unprobed, &budget),
        NextStep::Retain { .. }
    ));
}

#[test]
fn inspection_errors_truncation_and_unsupported_shaders() {
    let f = fixture();
    // Critical error => failure routed to repair.
    let mut probe = Fake::new("x", 60);
    probe.diagnostics = vec![PreviewDiagnostic {
        frame: 12,
        severity: DiagnosticSeverity::Error,
        key: "missing_asset".into(),
        message: "media/absent.jpg".into(),
    }];
    let (report, controller) = run(&f, &mut probe, |_| {});
    assert!(!report.passed());
    let ctx = report.failure().unwrap();
    assert_eq!(ctx.stage, ValidationStage::Inspection);
    assert_eq!(ctx.kind, FailureKind::Source);
    assert!(ctx.summary.contains("absent.jpg"));
    assert!(
        probe.rendered.is_empty(),
        "no rendering after a critical error"
    );
    assert!(matches!(
        next_step(&report, controller.agent_task().unwrap().repair()),
        NextStep::Repair { .. }
    ));
    drop(controller);

    // Truncated inspection cannot rule out critical findings.
    let f2 = fixture();
    let mut probe = Fake::new("x", 60);
    probe.truncated = true;
    let (report, _c) = run(&f2, &mut probe, |_| {});
    assert!(!report.passed());
    assert_eq!(report.failure().unwrap().stage, ValidationStage::Inspection);

    // Unsupported shader on a renderer with the shader gap: a visible warning, still a pass.
    let shader_diagnostic = PreviewDiagnostic {
        frame: 3,
        severity: DiagnosticSeverity::Warning,
        key: "shader_unsupported".into(),
        message: "custom shader falls back to CPU preview".into(),
    };
    let f3 = fixture();
    let mut probe = Fake::new("x", 60);
    probe.gaps = vec!["shader_preview".into()];
    probe.diagnostics = vec![shader_diagnostic.clone()];
    let (report, _c) = run(&f3, &mut probe, |_| {});
    assert!(report.passed());
    assert!(
        report
            .warnings()
            .iter()
            .any(|w| w.starts_with("Unsupported shader at frame 3") && w.contains("`cpu`"))
    );
    assert!(report.diagnostics_total() >= 1);

    // A Skia worker draws shaders: the same diagnostic is no warning.
    let f4 = fixture();
    let mut probe = Fake::new("x", 60);
    probe.backend = "skia-cpu".into();
    probe.diagnostics = vec![shader_diagnostic];
    let (report, _c) = run(&f4, &mut probe, |_| {});
    assert!(report.passed());
    assert!(
        report.warnings().iter().all(|w| !w.contains("shader")),
        "{:?}",
        report.warnings()
    );
}

#[test]
fn render_failures_timeline_problems_and_wrong_revision_fail_closed() {
    let f = fixture();
    let mut probe = Fake::new("x", 60);
    probe.render_error_at = Some(59);
    let (report, _c) = run(&f, &mut probe, |_| {});
    let ctx = report.failure().unwrap();
    assert_eq!(ctx.stage, ValidationStage::Render);
    assert!(ctx.summary.contains("59"));
    assert!(report.frames().iter().all(|a| a.frame != 59));

    let f = fixture();
    let mut probe = Fake::new("x", 0);
    let (report, _c) = run(&f, &mut probe, |_| {});
    assert_eq!(report.failure().unwrap().stage, ValidationStage::Timeline);
    assert_eq!(report.failure().unwrap().kind, FailureKind::Source);

    let f = fixture();
    let mut probe = Fake::new("x", 60);
    probe.timeline_error = Some(ProbeError::environment("worker crashed on startup"));
    let (report, _c) = run(&f, &mut probe, |_| {});
    assert_eq!(report.failure().unwrap().kind, FailureKind::Environment);

    // A timeline for another revision is never accepted for this candidate.
    let f = fixture();
    let (_c, _ctx, captured) = captured_with(&f, |_| {});
    let mut probe = Fake::new("someone-elses-revision", 60);
    let report = validate_candidate(
        &captured,
        BuildOutcome::Built(build_identity(&captured)),
        0,
        0,
        Some(&mut probe),
    );
    assert!(!report.passed());
    assert!(
        report
            .failure()
            .unwrap()
            .summary
            .contains("different revision")
    );
}

#[test]
fn a_declared_shader_capability_gap_is_a_visible_warning_recorded_in_the_report() {
    let f = fixture();
    let mut probe = Fake::new("x", 60);
    probe.gaps = vec!["shader_preview".into()];
    let (report, _c) = run(&f, &mut probe, |_| {});
    assert!(report.passed(), "{:?}", report.failure());
    assert_eq!(report.capability_gaps(), ["shader_preview".to_owned()]);
    assert!(
        report
            .warnings()
            .iter()
            .any(|w| w.contains("Unsupported shaders") && w.contains("shader_preview"))
    );
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|d| d.key == "capability_gap")
        .expect("gap is a diagnostic");
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Warning);
    assert_eq!(diagnostic.stage, ValidationStage::Worker);

    // Other gaps are recorded but are not shader warnings.
    let f = fixture();
    let mut probe = Fake::new("x", 60);
    probe.gaps = vec!["gpu_backend".into()];
    let (report, _c) = run(&f, &mut probe, |_| {});
    assert!(report.passed());
    assert_eq!(report.capability_gaps(), ["gpu_backend".to_owned()]);
    assert!(report.warnings().iter().all(|w| !w.contains("shader")));
}

#[test]
fn stop_between_worker_requests_is_an_environment_failure_that_spends_no_repair() {
    for (cancel_after, stage) in [
        (0usize, ValidationStage::Inspection),
        (2, ValidationStage::Render),
    ] {
        let f = fixture();
        let mut probe = Fake::new("x", 60);
        probe.cancel_after_renders = Some(cancel_after);
        let (report, controller) = run(&f, &mut probe, |_| {});
        let failure = report.failure().expect("cancelled");
        assert_eq!(failure.kind, FailureKind::Environment);
        assert_eq!(failure.stage, stage);
        assert!(failure.summary.contains("cancelled"));
        assert!(probe.rendered.len() <= cancel_after.max(1));
        assert!(matches!(
            next_step(&report, controller.agent_task().unwrap().repair()),
            NextStep::Retain { .. }
        ));
    }
}

fn warnings_then(errors: Vec<PreviewDiagnostic>) -> Vec<PreviewDiagnostic> {
    let mut diagnostics: Vec<PreviewDiagnostic> = (0..200)
        .map(|i| PreviewDiagnostic {
            frame: i,
            severity: DiagnosticSeverity::Warning,
            key: format!("noise_{i}"),
            message: "harmless".into(),
        })
        .collect();
    diagnostics.extend(errors);
    diagnostics
}

#[test]
fn a_critical_inspection_fact_survives_the_bounded_display_buffer() {
    // Broadened (shared source change) => four inspection batches. Batch 0 fills the
    // 128-entry display buffer with warnings; an interior Error arrives in batch 3.
    let edit = |draft: &Path| fs::write(draft.join("src/lib.rs"), "// shared").unwrap();
    let f = fixture();
    let mut probe = Fake::new("x", 1000);
    probe.batch_diagnostics = vec![
        (0, warnings_then(vec![])),
        (
            3,
            vec![PreviewDiagnostic {
                frame: 777,
                severity: DiagnosticSeverity::Error,
                key: "missing_asset".into(),
                message: "media/late.jpg".into(),
            }],
        ),
    ];
    let (report, controller) = run(&f, &mut probe, edit);
    assert_eq!(probe.inspect_batches.len(), 4);
    assert!(!report.passed());
    let failure = report.failure().unwrap();
    assert_eq!(failure.stage, ValidationStage::Inspection);
    assert_eq!(failure.kind, FailureKind::Source);
    assert!(failure.summary.contains("late.jpg"));
    assert!(failure.diagnostics.iter().any(|d| d.frame == Some(777)));
    // The display buffer is full of warnings and does not contain the error...
    assert_eq!(report.diagnostics().len(), MAX_REPORT_DIAGNOSTICS);
    assert!(report.diagnostics_total() > MAX_REPORT_DIAGNOSTICS);
    assert!(
        report
            .diagnostics()
            .iter()
            .all(|d| d.severity != DiagnosticSeverity::Error)
    );
    // ...but the sticky error facts do.
    assert_eq!(report.errors_total(), 1);
    assert_eq!(report.errors()[0].frame, Some(777));
    assert!(
        probe.rendered.is_empty(),
        "no rendering after a critical error"
    );
    assert!(matches!(
        next_step(&report, controller.agent_task().unwrap().repair()),
        NextStep::Repair { .. }
    ));

    // A truncated later batch after the buffer filled up is just as fatal.
    let f = fixture();
    let mut probe = Fake::new("x", 1000);
    probe.batch_diagnostics = vec![(0, warnings_then(vec![]))];
    probe.truncate_batch = Some(3);
    let (report, _c) = run(&f, &mut probe, edit);
    assert!(!report.passed());
    let failure = report.failure().unwrap();
    assert_eq!(failure.stage, ValidationStage::Inspection);
    assert!(failure.summary.contains("truncated"));
    assert_eq!(report.errors_total(), 1);
    assert_eq!(report.diagnostics().len(), MAX_REPORT_DIAGNOSTICS);
}

#[test]
fn incomplete_required_coverage_is_retained_not_accepted() {
    // 40 scenes => 80 required boundary frames, more than the 64-frame render cap.
    let scenes: Vec<(usize, usize)> = (0..40).map(|i| (i * 25, (i + 1) * 25)).collect();
    let f = fixture();
    let t = timeline("x", 1000, &scenes, &[]);
    let mut probe = Fake::new("x", 1000).with_timeline(t);
    let (report, controller) = run(&f, &mut probe, |_| {});
    assert!(!report.passed());
    let failure = report.failure().unwrap();
    assert_eq!(failure.stage, ValidationStage::Coverage);
    assert_eq!(failure.kind, FailureKind::Environment);
    assert!(failure.summary.contains("bounded validation limitation"));
    assert!(!report.coverage().unwrap().complete);
    assert!(report.frames().len() <= MAX_RENDERED_FRAMES);
    match next_step(&report, controller.agent_task().unwrap().repair()) {
        NextStep::Retain { reason, context } => {
            assert!(reason.contains("bounded validation limitation"), "{reason}");
            assert!(context.is_some());
        }
        other => panic!("{other:?}"),
    }
    // Optional sampled frames beyond the cap do not count as missing coverage: 20 scenes
    // need 40 required frames and still complete.
    let scenes: Vec<(usize, usize)> = (0..20).map(|i| (i * 50, (i + 1) * 50)).collect();
    let plan = plan_coverage(
        &timeline("x", 1000, &scenes, &[]),
        10,
        &ChangeSet::default(),
    );
    assert!(plan.complete);

    // A broadened video beyond the inspection bound: the defect sits in a frame the
    // strided inspection never visits, so the only safe verdict is "not accepted".
    let f = fixture();
    let total = MAX_BROADENED_FRAMES + 5000;
    let mut probe = Fake::new("x", total);
    let (report, controller) = run(&f, &mut probe, |draft| {
        fs::write(draft.join("src/lib.rs"), "// shared").unwrap();
    });
    let omitted = (0..total)
        .find(|frame| !probe.inspected.contains(frame))
        .expect("a strided inspection omits frames");
    assert!(!probe.inspected.contains(&omitted));
    assert!(!report.passed());
    let failure = report.failure().unwrap();
    assert_eq!(failure.stage, ValidationStage::Coverage);
    assert!(!report.coverage().unwrap().complete);
    assert!(matches!(
        next_step(&report, controller.agent_task().unwrap().repair()),
        NextStep::Retain { .. }
    ));
}

#[test]
fn a_build_that_does_not_name_the_candidate_proves_nothing() {
    let f = fixture();
    let (_c, _ctx, captured) = captured_with(&f, |_| {});
    let revision = captured.candidate().revision().as_str().to_owned();
    for mutate in [
        (|b: &mut BuildIdentity| b.source_revision = "e".repeat(64)) as fn(&mut BuildIdentity),
        |b| b.backend = "gpu".into(),
    ] {
        let mut probe = Fake::new(&revision, 300);
        let mut identity = build_identity(&captured);
        mutate(&mut identity);
        let report = validate_candidate(
            &captured,
            BuildOutcome::Built(identity),
            0,
            0,
            Some(&mut probe),
        );
        assert!(!report.passed());
        let failure = report.failure().unwrap();
        assert_eq!(failure.stage, ValidationStage::Compile);
        assert_eq!(failure.kind, FailureKind::Environment);
        assert!(probe.inspected.is_empty() && probe.rendered.is_empty());
    }
}

#[test]
fn frames_must_carry_the_identity_of_the_candidate_timeline() {
    let f = fixture();
    let mut probe = Fake::new("x", 300);
    probe.frame_revision = Some("another-revision".into());
    let (report, _c) = run(&f, &mut probe, |_| {});
    assert!(!report.passed());
    let failure = report.failure().unwrap();
    assert_eq!(failure.stage, ValidationStage::Render);
    assert_eq!(failure.kind, FailureKind::Environment);
    assert!(report.errors().iter().any(|d| d.key == "render_identity"));
    assert!(report.frames().is_empty());
}

#[test]
fn a_report_is_never_an_acceptance_proof_when_read_back() {
    // The report can be written for display, but there is no way to read one back as
    // an acceptance proof: `ValidationReport` and `CapturedCandidate` are not
    // `Deserialize`.
    trait Not {
        const DESERIALIZE: bool = false;
    }
    impl<T> Not for Probe<T> {}
    struct Probe<T>(std::marker::PhantomData<T>);
    impl<T: serde::de::DeserializeOwned> Probe<T> {
        const DESERIALIZE: bool = true;
    }
    const { assert!(!<Probe<ValidationReport>>::DESERIALIZE) };
    const { assert!(!<Probe<CapturedCandidate>>::DESERIALIZE) };
    // Sanity: the detector does see ordinary Deserialize types.
    const { assert!(<Probe<Coverage>>::DESERIALIZE) };
}
