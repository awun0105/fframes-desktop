use super::*;
use studio_bootstrap::GroupMembership;

fn manager() -> AgentTaskManager {
    AgentTaskManager::new(
        ProjectId::try_from("project-1".to_owned()).unwrap(),
        OpenSession::new(),
    )
}

fn revision(c: char) -> SourceRevision {
    SourceRevision::try_from(c.to_string().repeat(64)).unwrap()
}

fn context(m: &mut AgentTaskManager) -> AgentTaskContext {
    let reservation = m.reserve().unwrap();
    AgentTaskContext {
        identity: reservation.identity,
        source_base: TaskSourceBase::new(revision('a')),
        scope: TaskScope::whole_project("project-1", revision('a').as_str()),
        prior_checkpoint: revision('b'),
        assets: vec![],
        instructions: vec![],
        brief: "make it blue".into(),
        draft: PathBuf::from("/draft"),
        archived_previous: None,
    }
}

fn qualified() -> WriterOwnership {
    WriterOwnership::ProcessGroupContained {
        qualification: "test".into(),
    }
}

fn clean_termination() -> TerminationReport {
    TerminationReport {
        forced: false,
        direct_child_exited: true,
        group_empty: true,
        remaining: vec![],
    }
}

fn clean_facts() -> ScopeFacts {
    ScopeFacts {
        termination: clean_termination(),
        escaped: vec![],
        containment_unverified: None,
        had_live: false,
    }
}

fn evidence_with(id: &TaskIdentity, writer: WriterGeneration) -> QuiescenceEvidence {
    QuiescenceEvidence {
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
    }
}

/// Clean end-turn evidence bound to the task's current writer generation.
fn evidence_for(m: &AgentTaskManager, id: &TaskIdentity) -> QuiescenceEvidence {
    evidence_with(id, m.active().unwrap().writer_generation().unwrap())
}

fn quiesce(m: &mut AgentTaskManager, id: &TaskIdentity) -> Result<CaptureTicket, TaskError> {
    let evidence = evidence_for(m, id);
    m.complete_quiescence(id, &evidence, clean_facts)
}

fn started(m: &mut AgentTaskManager) -> TaskIdentity {
    let context = context(m);
    let identity = context.identity.clone();
    m.begin(context).unwrap();
    m.writer_started(&identity, qualified()).unwrap();
    identity
}

#[test]
fn transition_table_matches_the_documented_lifecycle() {
    use TaskState::*;
    let all = [
        ContextReady,
        Editing,
        Waiting,
        Quiescing,
        Validating,
        RepairNeeded,
        CandidateReady,
        Promoting,
        Accepted,
        Conflict,
        Failed,
        Cancelled,
        Interrupted,
    ];
    for terminal in all.iter().filter(|s| s.is_terminal()) {
        for next in all {
            assert!(!terminal.can_transition_to(next), "{terminal:?} is final");
        }
    }
    for from in all.iter().filter(|s| !s.is_terminal()) {
        assert!(
            from.can_transition_to(Interrupted),
            "{from:?} can be interrupted"
        );
    }
    assert!(ContextReady.can_transition_to(Editing));
    assert!(Editing.can_transition_to(Quiescing));
    assert!(Quiescing.can_transition_to(Validating));
    assert!(Validating.can_transition_to(RepairNeeded));
    assert!(RepairNeeded.can_transition_to(Editing));
    assert!(CandidateReady.can_transition_to(Promoting));
    assert!(Promoting.can_transition_to(Accepted));
    assert!(Promoting.can_transition_to(Conflict));
    // No shortcuts around validation, quiescence or promotion.
    assert!(!Editing.can_transition_to(Validating));
    assert!(!Editing.can_transition_to(CandidateReady));
    assert!(!Quiescing.can_transition_to(CandidateReady));
    assert!(!Validating.can_transition_to(Promoting));
    assert!(!CandidateReady.can_transition_to(Accepted));
    assert!(!ContextReady.can_transition_to(Quiescing));
    assert!(!Waiting.can_transition_to(Quiescing));
    assert!(!Promoting.can_transition_to(Cancelled));
}

#[test]
fn quiescence_requires_end_turn_verified_termination_and_qualified_ownership() {
    let id = TaskIdentity {
        task: AgentTaskId::new(),
        project: ProjectId::try_from("project-1".to_owned()).unwrap(),
        session: OpenSession::new(),
        generation: 0,
    };
    let clean = evidence_with(&id, WriterGeneration(1));
    let eval = |e: &QuiescenceEvidence, o: &WriterOwnership, t: &TerminationReport| {
        evaluate_quiescence(e, o, t)
    };
    assert_eq!(eval(&clean, &qualified(), &clean_termination()), Ok(()));
    let mut e = clean.clone();
    e.completion = TurnCompletion::None;
    assert_eq!(
        eval(&e, &qualified(), &clean_termination()),
        Err(QuiescenceBlock::NoAuthoritativeCompletion)
    );
    for completion in [
        TurnCompletion::Refusal,
        TurnCompletion::Cancelled,
        TurnCompletion::MaxTokens,
        TurnCompletion::MaxTurnRequests,
    ] {
        let mut e = clean.clone();
        e.completion = completion;
        assert_eq!(
            eval(&e, &qualified(), &clean_termination()),
            Err(QuiescenceBlock::NotEndTurn(completion))
        );
    }
    let mut e = clean.clone();
    e.cancel_requested = true;
    assert_eq!(
        eval(&e, &qualified(), &clean_termination()),
        Err(QuiescenceBlock::CancelRequested)
    );
    let mut e = clean.clone();
    e.unresolved_requests = 2;
    assert_eq!(
        eval(&e, &qualified(), &clean_termination()),
        Err(QuiescenceBlock::UnresolvedRequests(2))
    );
    let mut t = clean_termination();
    t.group_empty = false;
    t.remaining = vec![7];
    assert_eq!(
        eval(&clean, &qualified(), &t),
        Err(QuiescenceBlock::GroupSurvivors(vec![7]))
    );
    let mut t = clean_termination();
    t.direct_child_exited = false;
    assert_eq!(
        eval(&clean, &qualified(), &t),
        Err(QuiescenceBlock::WriterNotTerminated)
    );
    for ownership in [WriterOwnership::Unknown, WriterOwnership::Detached] {
        assert_eq!(
            eval(&clean, &ownership, &clean_termination()),
            Err(QuiescenceBlock::UnqualifiedOwnership(ownership.label()))
        );
    }
    let _ = GroupMembership::Empty;
}

#[test]
fn stop_reasons_map_and_unknown_reasons_are_not_completions() {
    assert_eq!(
        TurnCompletion::from_stop_reason("end_turn"),
        TurnCompletion::EndTurn
    );
    assert_eq!(
        TurnCompletion::from_stop_reason("refusal"),
        TurnCompletion::Refusal
    );
    assert_eq!(
        TurnCompletion::from_stop_reason("bogus"),
        TurnCompletion::None
    );
}

#[test]
fn one_writer_lease_per_project_is_released_when_the_task_ends() {
    let mut m = manager();
    let id = started(&mut m);
    assert_eq!(m.lease_holder(), Some(&id.task));
    assert!(matches!(m.reserve(), Err(TaskError::LeaseHeld(t)) if t == id.task));
    m.finish(&id, TaskState::Cancelled, "user stop").unwrap();
    assert_eq!(m.lease_holder(), None);
    assert!(m.active().is_none());
    assert_eq!(m.current().unwrap().state(), TaskState::Cancelled);
    m.reserve().expect("lease is free again");
}

#[test]
fn stale_identities_are_rejected_across_generations_and_sessions() {
    let mut m = manager();
    let first = started(&mut m);
    m.finish(&first, TaskState::Cancelled, "done").unwrap();
    let second = started(&mut m);
    assert_eq!(second.generation, first.generation + 1);
    assert_eq!(
        m.transition(&first, TaskState::Editing),
        Err(TaskError::StaleIdentity)
    );
    let mut other_session = second.clone();
    other_session.session = OpenSession::new();
    assert_eq!(
        m.transition(&other_session, TaskState::Editing),
        Err(TaskError::StaleIdentity)
    );
    let mut other_generation = second.clone();
    other_generation.generation += 1;
    assert_eq!(
        m.transition(&other_generation, TaskState::Editing),
        Err(TaskError::StaleIdentity)
    );
    assert_eq!(m.transition(&second, TaskState::Editing), Ok(()));
}

#[test]
fn identity_counter_exhaustion_is_an_error_not_a_wraparound() {
    let mut m = manager().with_next_generation(u64::MAX - 1);
    let last = m.reserve().unwrap();
    assert_eq!(last.identity.generation, u64::MAX - 1);
    assert_eq!(m.reserve().err(), Some(TaskError::CounterExhausted));
    assert_eq!(
        m.reserve().err(),
        Some(TaskError::CounterExhausted),
        "stays exhausted"
    );
}

#[test]
fn dedicated_operations_guard_validation_repair_and_terminal_states() {
    let mut m = manager();
    let id = started(&mut m);
    m.transition(&id, TaskState::Editing).unwrap();
    assert_eq!(
        m.transition(&id, TaskState::Validating),
        Err(TaskError::InvalidTransition {
            from: TaskState::Editing,
            to: TaskState::Validating
        })
    );
    assert_eq!(
        m.transition(&id, TaskState::Failed),
        Err(TaskError::GatedTransition(TaskState::Failed))
    );
    assert_eq!(
        m.transition(&id, TaskState::CandidateReady),
        Err(TaskError::InvalidTransition {
            from: TaskState::Editing,
            to: TaskState::CandidateReady
        })
    );
    assert_eq!(
        m.begin_repair(&id, || Ok(())),
        Err(TaskError::WrongState {
            expected: TaskState::RepairNeeded,
            actual: TaskState::Editing
        })
    );
    m.transition(&id, TaskState::Waiting).unwrap();
    m.transition(&id, TaskState::Editing).unwrap();
    m.transition(&id, TaskState::Quiescing).unwrap();
    assert_eq!(
        m.transition(&id, TaskState::Validating),
        Err(TaskError::GatedTransition(TaskState::Validating))
    );
    assert!(matches!(
        m.finish(&id, TaskState::Accepted, "no"),
        Err(TaskError::InvalidTransition { .. })
    ));
}

#[test]
fn capture_requires_the_gate_and_a_blocked_gate_leaves_the_task_quiescing() {
    let mut m = manager();
    let id = started(&mut m);
    assert!(matches!(
        quiesce(&mut m, &id),
        Err(TaskError::WrongState {
            expected: TaskState::Quiescing,
            ..
        })
    ));
    m.transition(&id, TaskState::Editing).unwrap();
    m.transition(&id, TaskState::Quiescing).unwrap();
    // The scope still shows a live member: whatever the evidence claims, no ticket.
    let evidence = evidence_for(&m, &id);
    let live = ScopeFacts {
        termination: TerminationReport {
            forced: false,
            direct_child_exited: false,
            group_empty: false,
            remaining: vec![4242],
        },
        escaped: vec![],
        containment_unverified: None,
        had_live: true,
    };
    assert_eq!(
        m.complete_quiescence(&id, &evidence, || live).err(),
        Some(TaskError::QuiescenceBlocked(
            QuiescenceBlock::GroupSurvivors(vec![4242])
        ))
    );
    assert_eq!(m.active().unwrap().state(), TaskState::Quiescing);
    assert!(
        !m.active().unwrap().writer_safe(),
        "a live member keeps the draft unsafe"
    );
    let ticket = quiesce(&mut m, &id).unwrap();
    assert_eq!(ticket.source_base().revision(), &revision('a'));
    m.record_candidate(ticket, CandidateRevision::new(revision('c')), "m".into())
        .unwrap();
    assert_eq!(m.active().unwrap().state(), TaskState::Validating);
    assert_eq!(
        m.active().unwrap().candidate().unwrap().revision(),
        &revision('c')
    );
    assert!(m.active().unwrap().writer_safe());
}

#[test]
fn repair_budget_allows_exactly_one_automatic_repair_then_fails() {
    let mut m = manager();
    let id = started(&mut m);
    let capture = |m: &mut AgentTaskManager| {
        m.transition(&id, TaskState::Quiescing).unwrap();
        let ticket = quiesce(m, &id).unwrap();
        m.record_candidate(ticket, CandidateRevision::new(revision('c')), "m".into())
            .unwrap();
    };
    m.transition(&id, TaskState::Editing).unwrap();
    capture(&mut m);
    assert_eq!(
        m.validation_failed(&id, "compile error").unwrap(),
        RepairDecision::Repair { attempt: 1 }
    );
    assert_eq!(m.active().unwrap().state(), TaskState::RepairNeeded);
    assert_eq!(
        m.active().unwrap().repair().remaining(),
        1,
        "spent only when the repair starts"
    );
    assert_eq!(m.begin_repair(&id, || Ok(())).unwrap(), 1);
    m.writer_started(&id, qualified()).unwrap();
    assert_eq!(m.active().unwrap().repair().remaining(), 0);
    assert!(
        m.active().unwrap().candidate().is_none(),
        "a failed candidate is dropped"
    );
    capture(&mut m);
    assert_eq!(
        m.validation_failed(&id, "still broken").unwrap(),
        RepairDecision::Exhausted
    );
    assert!(m.active().is_none());
    assert_eq!(m.current().unwrap().state(), TaskState::Failed);
    assert_eq!(m.lease_holder(), None);
}

#[test]
fn repair_requires_fresh_quiescence_evidence() {
    let mut m = manager();
    let id = started(&mut m);
    m.transition(&id, TaskState::Editing).unwrap();
    m.transition(&id, TaskState::Quiescing).unwrap();
    let ticket = quiesce(&mut m, &id).unwrap();
    m.record_candidate(ticket, CandidateRevision::new(revision('c')), "m".into())
        .unwrap();
    m.validation_failed(&id, "x").unwrap();
    m.begin_repair(&id, || Ok(())).unwrap();
    assert!(
        m.active().unwrap().writer_ownership().is_none(),
        "old evidence is discarded"
    );
    m.writer_started(&id, WriterOwnership::Unknown).unwrap();
    assert!(
        !m.active().unwrap().writer_safe(),
        "a new writer is unproven until it is reaped"
    );
}

/// Builds a second, independent ticket value for the current gate state so replay of
/// a copy can be exercised (the public type is deliberately not `Clone`).
fn duplicate(ticket: &CaptureTicket) -> CaptureTicket {
    CaptureTicket {
        identity: ticket.identity.clone(),
        base: ticket.base.clone(),
        draft: ticket.draft.clone(),
        generation: ticket.generation,
        nonce: ticket.nonce,
    }
}

#[test]
fn a_copied_ticket_cannot_authorize_a_candidate_from_a_repair_writer() {
    let mut m = manager();
    let id = started(&mut m);
    m.transition(&id, TaskState::Editing).unwrap();
    m.transition(&id, TaskState::Quiescing).unwrap();
    let first = quiesce(&mut m, &id).unwrap();
    let replay = duplicate(&first);
    m.record_candidate(first, CandidateRevision::new(revision('c')), "m".into())
        .unwrap();
    m.validation_failed(&id, "x").unwrap();
    m.begin_repair(&id, || Ok(())).unwrap();
    // An unverified repair writer is now modifying the draft, then the task is put
    // back into Quiescing without any fresh quiescence evaluation.
    m.writer_started(&id, WriterOwnership::Unknown).unwrap();
    m.transition(&id, TaskState::Quiescing).unwrap();
    assert_eq!(
        m.record_candidate(replay, CandidateRevision::new(revision('d')), "m".into()),
        Err(TaskError::StaleTicket)
    );
    assert_eq!(m.active().unwrap().state(), TaskState::Quiescing);
    assert!(m.active().unwrap().candidate().is_none());
}

#[test]
fn tickets_are_single_use_and_superseded_by_every_new_gate_event() {
    let mut m = manager();
    let id = started(&mut m);
    m.transition(&id, TaskState::Editing).unwrap();
    m.transition(&id, TaskState::Quiescing).unwrap();
    let candidate = || CandidateRevision::new(revision('c'));

    // A newer ticket supersedes every ticket issued before it.
    let older = quiesce(&mut m, &id).unwrap();
    let newer = quiesce(&mut m, &id).unwrap();
    assert_eq!(
        m.record_candidate(older, candidate(), "m".into()),
        Err(TaskError::StaleTicket)
    );

    // A failed quiescence attempt retires the outstanding ticket.
    let mut blocked = evidence_for(&m, &id);
    blocked.cancel_requested = true;
    assert!(m.complete_quiescence(&id, &blocked, clean_facts).is_err());
    assert_eq!(
        m.record_candidate(newer, candidate(), "m".into()),
        Err(TaskError::StaleTicket)
    );

    // Starting a writer retires the outstanding ticket as well.
    let fresh = quiesce(&mut m, &id).unwrap();
    m.writer_started(&id, qualified()).unwrap();
    assert_eq!(
        m.record_candidate(fresh, candidate(), "m".into()),
        Err(TaskError::StaleTicket)
    );

    // A ticket works exactly once.
    let ticket = quiesce(&mut m, &id).unwrap();
    let again = duplicate(&ticket);
    m.record_candidate(ticket, candidate(), "m".into()).unwrap();
    assert!(m.record_candidate(again, candidate(), "m".into()).is_err());
}

#[test]
fn capture_tickets_are_not_clone() {
    // Fails to compile (ambiguous impl) if `CaptureTicket` ever becomes `Clone`.
    trait AmbiguousIfClone<A> {
        fn check() {}
    }
    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    impl<T: Clone> AmbiguousIfClone<u8> for T {}
    <CaptureTicket as AmbiguousIfClone<_>>::check();
}

#[test]
fn evidence_for_another_task_generation_or_session_never_reaches_the_gate() {
    let mut m = manager();
    let id = started(&mut m);
    m.transition(&id, TaskState::Editing).unwrap();
    m.transition(&id, TaskState::Quiescing).unwrap();
    let observed = std::cell::Cell::new(false);
    let observe = || {
        observed.set(true);
        clean_facts()
    };

    let mut foreign = evidence_for(&m, &id);
    foreign.identity.task = AgentTaskId::new();
    assert_eq!(
        m.complete_quiescence(&id, &foreign, observe).err(),
        Some(TaskError::QuiescenceBlocked(
            QuiescenceBlock::ForeignEvidence
        ))
    );
    let mut other_generation = evidence_for(&m, &id);
    other_generation.identity.generation += 1;
    assert!(matches!(
        m.complete_quiescence(&id, &other_generation, observe).err(),
        Some(TaskError::QuiescenceBlocked(
            QuiescenceBlock::ForeignEvidence
        ))
    ));

    let mut stale_writer = evidence_for(&m, &id);
    let old = stale_writer.writer;
    m.writer_started(&id, qualified()).unwrap();
    stale_writer.writer = old;
    assert!(matches!(
        m.complete_quiescence(&id, &stale_writer, observe).err(),
        Some(TaskError::QuiescenceBlocked(
            QuiescenceBlock::StaleWriter { .. }
        ))
    ));

    let mut other_session = evidence_for(&m, &id);
    other_session.provider_session = Some("session-from-elsewhere".into());
    assert_eq!(
        m.complete_quiescence(&id, &other_session, observe).err(),
        Some(TaskError::QuiescenceBlocked(
            QuiescenceBlock::ProviderSessionMismatch
        ))
    );
    assert!(
        !observed.get(),
        "the scope is never consulted for bad evidence"
    );
}

#[test]
fn quiescence_requires_a_writer_record_in_the_current_epoch() {
    let mut m = manager();
    let context = context(&mut m);
    let id = context.identity.clone();
    m.begin(context).unwrap();
    m.transition(&id, TaskState::Editing).unwrap();
    m.transition(&id, TaskState::Quiescing).unwrap();
    // No writer was started: nothing the caller supplies can stand in for one.
    let evidence = evidence_with(&id, WriterGeneration(0));
    assert_eq!(
        m.complete_quiescence(&id, &evidence, clean_facts).err(),
        Some(TaskError::QuiescenceBlocked(QuiescenceBlock::NoWriter))
    );
    assert_eq!(m.active().unwrap().state(), TaskState::Quiescing);
}

#[test]
fn ownership_demotion_is_sticky_and_evidence_can_never_upgrade_it() {
    let mut m = manager();
    let id = started(&mut m);
    m.transition(&id, TaskState::Editing).unwrap();
    m.transition(&id, TaskState::Quiescing).unwrap();
    m.observe_writer(
        &id,
        &WriterObservation {
            ownership: WriterOwnership::Detached,
            escaped_pids: vec![],
        },
    )
    .unwrap();
    assert_eq!(
        m.active().unwrap().writer_ownership(),
        Some(&WriterOwnership::Detached)
    );
    // Evidence (and a fresh writer start) that claim qualification change nothing.
    let mut claims_qualified = evidence_for(&m, &id);
    claims_qualified.observed.ownership = qualified();
    assert_eq!(
        m.complete_quiescence(&id, &claims_qualified, clean_facts)
            .err(),
        Some(TaskError::QuiescenceBlocked(
            QuiescenceBlock::UnqualifiedOwnership("detached")
        ))
    );
    m.writer_started(&id, qualified()).unwrap();
    assert_eq!(
        m.active().unwrap().writer_ownership(),
        Some(&WriterOwnership::Detached),
        "a later writer start cannot launder the demotion"
    );
    m.observe_writer(
        &id,
        &WriterObservation {
            ownership: qualified(),
            escaped_pids: vec![],
        },
    )
    .unwrap();
    assert_eq!(
        m.active().unwrap().writer_ownership(),
        Some(&WriterOwnership::Detached)
    );
}

#[test]
fn driver_observed_escapes_and_scope_escapes_demote_a_qualified_writer() {
    let mut m = manager();
    let id = started(&mut m);
    m.transition(&id, TaskState::Editing).unwrap();
    m.observe_writer(
        &id,
        &WriterObservation {
            ownership: qualified(),
            escaped_pids: vec![4242],
        },
    )
    .unwrap();
    assert_eq!(
        m.active().unwrap().writer_ownership(),
        Some(&WriterOwnership::Detached)
    );

    let mut m = manager();
    let id = started(&mut m);
    m.record_scope_facts(
        &id,
        ScopeFacts {
            escaped: vec![7],
            ..clean_facts()
        },
    )
    .unwrap();
    let task = m.active().unwrap();
    assert_eq!(task.writer_ownership(), Some(&WriterOwnership::Detached));
    assert!(
        !task.writer_safe(),
        "a clean count never outvotes an escape"
    );
}

#[test]
fn unverifiable_containment_demotes_like_an_escape() {
    let unverified = || ScopeFacts {
        containment_unverified: Some("the escape scan failed".into()),
        ..clean_facts()
    };
    // A registered, qualified writer is detached and never gets a ticket.
    let mut m = manager();
    let id = started(&mut m);
    m.transition(&id, TaskState::Editing).unwrap();
    m.transition(&id, TaskState::Quiescing).unwrap();
    let evidence = evidence_for(&m, &id);
    assert_eq!(
        m.complete_quiescence(&id, &evidence, unverified).err(),
        Some(TaskError::QuiescenceBlocked(
            QuiescenceBlock::UnqualifiedOwnership("detached")
        ))
    );
    let task = m.active().unwrap();
    assert_eq!(task.writer_ownership(), Some(&WriterOwnership::Detached));
    assert!(!task.writer_safe());

    // A scope with no registered writer records a detached one: the draft is unsafe.
    let mut m = manager();
    let context = context(&mut m);
    let id = context.identity.clone();
    m.begin(context).unwrap();
    m.record_scope_facts(&id, unverified()).unwrap();
    let task = m.active().unwrap();
    assert_eq!(task.writer_ownership(), Some(&WriterOwnership::Detached));
    assert!(!task.writer_safe());
}

#[test]
fn source_invalidation_interrupts_for_good() {
    let mut m = manager();
    let id = started(&mut m);
    m.transition(&id, TaskState::Editing).unwrap();
    let finished = m.invalidate_source("source became invalid").unwrap();
    assert_eq!(finished.state, TaskState::Interrupted);
    assert!(m.current().unwrap().source_invalidated());
    assert_eq!(
        m.transition(&id, TaskState::Quiescing),
        Err(TaskError::NoActiveTask)
    );
    assert_eq!(m.lease_holder(), None);
    assert!(m.invalidate_source("again").is_none());
}

#[test]
fn brief_is_trimmed_and_bounded() {
    assert_eq!(validate_brief("  hi \n").unwrap(), "hi");
    assert!(validate_brief("   ").is_err());
    assert!(validate_brief(&"x".repeat(MAX_BRIEF_BYTES + 1)).is_err());
}

#[test]
fn candidate_and_base_are_distinct_types() {
    // Compile-time distinctness: neither converts into the other.
    fn takes_base(_: &TaskSourceBase) {}
    fn takes_candidate(_: &CandidateRevision) {}
    takes_base(&TaskSourceBase::new(revision('a')));
    takes_candidate(&CandidateRevision::new(revision('a')));
}
