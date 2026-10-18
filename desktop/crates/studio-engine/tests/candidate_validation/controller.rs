//! Capture and validation through the controller and the task's end states.

use super::*;

#[test]
fn the_controller_captures_only_with_a_current_ticket_and_routes_validation_reports() {
    let f = fixture();
    let mut controller = Controller::open(&f.root, &f.paths).unwrap();
    let context = start(&mut controller);
    let id = context.identity.clone();
    fs::write(context.draft.join("src/lib.rs"), "// first").unwrap();
    let first = ticket(&mut controller, &context);
    // A newer quiescence attempt retires the first ticket: it can no longer capture.
    let second = ticket(&mut controller, &context);
    assert!(controller.agent_capture_candidate(first).is_err());
    assert_eq!(
        controller.agent_task().unwrap().state(),
        TaskState::Quiescing
    );
    assert!(controller.agent_task().unwrap().candidate().is_none());

    let captured = controller.agent_capture_candidate(second).unwrap();
    assert_eq!(
        controller.agent_task().unwrap().state(),
        TaskState::Validating
    );
    assert_eq!(
        controller.agent_task().unwrap().candidate(),
        Some(captured.candidate())
    );

    let revision = captured.candidate().revision().as_str().to_owned();
    let validate = |probe: &mut Fake, repair_used| {
        probe.revision = revision.clone();
        probe.timeline.envelope = envelope(&revision);
        if let Ok(audio) = &mut probe.audio {
            audio.descriptor.envelope = probe.timeline.envelope.clone();
        }
        validate_candidate(
            &captured,
            BuildOutcome::Built(build_identity(&captured)),
            0,
            repair_used,
            Some(probe),
        )
    };

    // A report for another candidate (or task) never moves this task.
    let (foreign_controller, _foreign_ctx, foreign) = captured_with(&fixture(), |_| {});
    let foreign_report = validate_candidate(
        &foreign,
        BuildOutcome::Built(build_identity(&foreign)),
        0,
        0,
        None,
    );
    assert!(
        controller
            .agent_apply_validation(&id, &foreign_report)
            .is_err()
    );
    drop(foreign_controller);
    assert_eq!(
        controller.agent_task().unwrap().state(),
        TaskState::Validating
    );

    // A source failure spends the single automatic repair.
    let mut bad = Fake::new("x", 60);
    bad.diagnostics = vec![PreviewDiagnostic {
        frame: 1,
        severity: DiagnosticSeverity::Error,
        key: "missing_asset".into(),
        message: "media/absent.jpg".into(),
    }];
    let failed = validate(&mut bad, 0);
    match controller.agent_apply_validation(&id, &failed).unwrap() {
        NextStep::Repair { attempt: 1, .. } => {}
        other => panic!("{other:?}"),
    }
    assert_eq!(
        controller.agent_task().unwrap().state(),
        TaskState::RepairNeeded
    );
}

#[test]
fn captured_candidate_retains_the_scope_frozen_on_the_task_context() {
    let f = fixture();
    let mut controller = Controller::open(&f.root, &f.paths).unwrap();
    let project = studio_project::open(&f.root).unwrap();
    let inventory = studio_project::revision::SourceInventory::scan(&f.root).unwrap();
    let scope = TaskScope::whole_project(
        String::from(project.manifest.project_id.clone()),
        inventory.revision.as_str(),
    );
    let context = controller
        .begin_agent_task_scoped("edit", scope.clone())
        .unwrap();
    controller
        .agent_writer_started(&context.identity, qualified())
        .unwrap();
    controller
        .agent_task_transition(&context.identity, TaskState::Editing)
        .unwrap();
    controller
        .agent_task_transition(&context.identity, TaskState::Quiescing)
        .unwrap();
    let ticket = ticket(&mut controller, &context);
    let captured = controller.agent_capture_candidate(ticket).unwrap();
    assert_eq!(captured.scope(), &scope);
}

#[test]
fn accepted_environment_and_exhausted_reports_end_in_the_right_states() {
    // Pass: the task becomes CandidateReady and the report is the evidence.
    let f = fixture();
    let (mut controller, context, captured) = captured_with(&f, |_| {});
    let revision = captured.candidate().revision().as_str().to_owned();
    let mut probe = Fake::new(&revision, 60);
    let report = validate_candidate(
        &captured,
        BuildOutcome::Built(build_identity(&captured)),
        0,
        0,
        Some(&mut probe),
    );
    assert!(report.passed());
    assert_eq!(
        controller
            .agent_apply_validation(&context.identity, &report)
            .unwrap(),
        NextStep::Accept
    );
    assert_eq!(
        controller.agent_task().unwrap().state(),
        TaskState::CandidateReady
    );
    // The report cannot be applied twice.
    assert!(
        controller
            .agent_apply_validation(&context.identity, &report)
            .is_err()
    );
    drop(controller);

    // Environment failure: terminal, draft retained, repair budget untouched.
    let f = fixture();
    let (mut controller, context, captured) = captured_with(&f, |_| {});
    let report = validate_candidate(
        &captured,
        BuildOutcome::Failed {
            kind: FailureKind::Environment,
            stage: ValidationStage::Worker,
            output: "incompatible preview bridge".into(),
            build: None,
        },
        0,
        0,
        None,
    );
    assert!(matches!(
        controller
            .agent_apply_validation(&context.identity, &report)
            .unwrap(),
        NextStep::Retain { .. }
    ));
    let task = controller.agent_task().unwrap();
    assert_eq!(task.state(), TaskState::Failed);
    assert_eq!(task.repair().used(), 0);
    assert!(matches!(
        controller.agent_draft_state().unwrap().unwrap(),
        studio_engine::DraftState::Retained { .. }
    ));
    drop(controller);

    // Source failure with the single repair already spent is terminal.
    let f = fixture();
    let (mut controller, context, captured) = captured_with(&f, |_| {});
    let source_failure = |used| {
        validate_candidate(
            &captured,
            BuildOutcome::Failed {
                kind: FailureKind::Source,
                stage: ValidationStage::Compile,
                output: "error[E0425]".into(),
                build: None,
            },
            0,
            used,
            None,
        )
    };
    assert!(matches!(
        controller
            .agent_apply_validation(&context.identity, &source_failure(0))
            .unwrap(),
        NextStep::Repair { attempt: 1, .. }
    ));
    assert_eq!(controller.agent_begin_repair(&context.identity).unwrap(), 1);
    let task = controller.agent_task().unwrap();
    assert_eq!(task.repair().remaining(), 0);
    // Second candidate in the repair epoch.
    controller
        .agent_writer_started(&context.identity, qualified())
        .unwrap();
    controller
        .agent_task_transition(&context.identity, TaskState::Quiescing)
        .unwrap();
    let t = ticket(&mut controller, &context);
    let second = controller.agent_capture_candidate(t).unwrap();
    let second_report = validate_candidate(
        &second,
        BuildOutcome::Failed {
            kind: FailureKind::Source,
            stage: ValidationStage::Compile,
            output: "error[E0425]".into(),
            build: None,
        },
        0,
        1,
        None,
    );
    assert!(matches!(
        controller
            .agent_apply_validation(&context.identity, &second_report)
            .unwrap(),
        NextStep::Retain { .. }
    ));
    assert_eq!(controller.agent_task().unwrap().state(), TaskState::Failed);
    assert_eq!(controller.agent_task().unwrap().repair().used(), 1);
}
