//! Regressions: escaped writers, queued successors, engine refusals, workers that fail to start or panic, and stale cards.

use super::*;

#[cfg(unix)]
fn escaped_pids(w: &World) -> Vec<i32> {
    fs::read_to_string(w.agent.join("escape.pid"))
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect()
}

#[cfg(unix)]
fn kill_escaped(w: &World) {
    for pid in escaped_pids(w) {
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }
}

/// Every row of the conversation, whether resident or paged back from the log.
fn every_row(w: &World) -> std::collections::BTreeMap<u64, Arc<Row>> {
    let mut all = std::collections::BTreeMap::new();
    let mut before = RowId(u64::MAX);
    loop {
        let page = w.wf().history_page(before, 100).unwrap();
        let Some(oldest) = page.first().map(|r| r.id) else {
            break;
        };
        before = oldest;
        for row in page {
            all.insert(row.id.0, row);
        }
    }
    for row in &w.snap().rows {
        all.insert(row.id.0, row.clone());
    }
    all
}

/// A successor is refused while the draft may still be written, until the user confirms.
#[cfg(unix)]
fn assert_successor_refused_then_unlocked(w: &World, previous: &AgentTaskId) {
    assert!(
        matches!(w.draft_state(), Some(DraftState::UnsafeWriter { .. })),
        "{:?}",
        w.draft_state()
    );
    w.wf().submit(&World::brief(&good("blocked"))).unwrap();
    w.wait("the draft lock", |s| has_error(s, "draft_unsafe"));
    assert_eq!(
        w.snap().task.as_ref().unwrap().id,
        *previous,
        "no new task was created"
    );
    kill_escaped(w);
    w.wf().acknowledge_writer_gone().unwrap();
    w.wait("the unlocked draft", |s| {
        matches!(
            s.recovery.draft.as_ref().map(|d| &d.state),
            Some(DraftState::Retained { .. })
        )
    });
    w.submit(&good("after unlock"));
    let next = w.wait_task(Some(previous));
    assert_eq!(next.phase, TaskPhase::Accepted, "{:?}", next.error);
}

#[cfg(unix)]
#[test]
fn an_escaped_writer_locks_the_draft_when_the_user_stops_the_task() {
    let w = World::new();
    w.submit(&json!({"escape": true, "hang": true}));
    w.wait_phase(TaskPhase::Editing);
    w.poll("the escaped helper", || w.agent.join("escape.pid").exists());
    w.wf().stop().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Cancelled);
    assert!(
        matches!(task.draft_state, Some(DraftState::UnsafeWriter { .. })),
        "{:?}",
        task.draft_state
    );
    assert_successor_refused_then_unlocked(&w, &task.id);
}

#[cfg(unix)]
#[test]
fn an_escaped_writer_locks_the_draft_when_the_provider_fails() {
    let w = World::new();
    w.submit(&json!({"escape": true, "linger": true, "malformed": true}));
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Failed);
    assert!(
        task.error
            .as_ref()
            .is_some_and(|e| e.code.starts_with("provider_")),
        "{:?}",
        task.error
    );
    assert_successor_refused_then_unlocked(&w, &task.id);
}

#[cfg(unix)]
#[test]
fn an_escaped_writer_locks_the_draft_when_the_project_closes() {
    let mut w = World::new();
    w.submit(&json!({"escape": true, "hang": true}));
    w.wait_phase(TaskPhase::Editing);
    w.poll("the escaped helper", || w.agent.join("escape.pid").exists());
    w.wf().close();
    assert!(
        matches!(w.draft_state(), Some(DraftState::UnsafeWriter { .. })),
        "{:?}",
        w.draft_state()
    );
    // A reopened project refuses a successor until the user confirms.
    w.restart();
    w.wf().submit(&World::brief(&good("blocked"))).unwrap();
    w.wait("the draft lock", |s| has_error(s, "draft_unsafe"));
    kill_escaped(&w);
    w.wf().acknowledge_writer_gone().unwrap();
    w.wait("the unlocked draft", |s| {
        matches!(
            s.recovery.draft.as_ref().map(|d| &d.state),
            Some(DraftState::Retained { .. })
        )
    });
    w.submit(&good("after unlock"));
    let next = w.wait_task(None);
    assert_eq!(next.phase, TaskPhase::Accepted, "{:?}", next.error);
}

#[cfg(unix)]
#[test]
fn an_escaped_writer_locks_the_draft_when_stop_cancels_the_quiescence() {
    let w = World::new();
    // The adapter ignores SIGTERM and stdin EOF: the reap inside the pipeline takes its
    // whole grace period, which is when Stop arrives.
    let mut script = good("quiescing");
    script["escape"] = json!(true);
    script["linger"] = json!(true);
    w.submit(&script);
    w.wait_phase(TaskPhase::Quiescing);
    w.wf().stop().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Cancelled, "{:?}", task.error);
    assert!(
        matches!(task.draft_state, Some(DraftState::UnsafeWriter { .. })),
        "{:?}",
        task.draft_state
    );
    assert_successor_refused_then_unlocked(&w, &task.id);
}

#[test]
fn a_crashing_first_task_never_fails_its_queued_successors() {
    let w = World::new();
    w.set_plan(&[
        json!({"text": ["before the crash ", "and more"], "tool": 3, "crash": 7}),
        good("second"),
        good("third"),
    ]);
    w.wf().submit("plan brief one").unwrap();
    w.wf().submit("plan brief two").unwrap();
    w.wf().submit("plan brief three").unwrap();
    let first = w.wait_task(None);
    assert_eq!(first.phase, TaskPhase::Failed);
    let second = w.wait_task(Some(&first.id));
    assert_eq!(second.phase, TaskPhase::Accepted, "{:?}", second.error);
    let third = w.wait_task(Some(&second.id));
    assert_eq!(third.phase, TaskPhase::Accepted, "{:?}", third.error);
    assert_eq!(w.source("src/lib.rs"), "// third\n");
    let errors = w
        .snap()
        .rows
        .iter()
        .filter(|r| matches!(r.kind, RowKind::Error(_)))
        .count();
    assert_eq!(errors, 1, "only the crash is an error");
    // The crashed task's own text stays attached to it, and nothing of it leaks forward.
    for row in &w.snap().rows {
        if let RowKind::Agent { text, .. } = &row.kind
            && text.contains("before the crash")
        {
            assert_eq!(row.task.as_ref(), Some(&first.id));
        }
    }
    w.assert_clean();
}

#[test]
fn an_engine_refusal_before_the_session_is_registered_fails_boundedly_and_close_completes() {
    let w = World::new();
    fs::write(w.agent.join("hold-session"), "held").unwrap();
    w.submit(&good("never runs"));
    w.poll("the engine task", || {
        w.ctl()
            .lock()
            .agent_task()
            .is_some_and(|t| t.state() == TaskState::ContextReady)
    });
    // Another owner of the shared controller ends the task while the agent is still
    // answering `session/new`.
    let identity = w.ctl().lock().agent_task().unwrap().identity().clone();
    w.ctl()
        .lock()
        .finish_agent_task(&identity, TaskState::Interrupted, "ended by another owner")
        .unwrap();
    w.release("release-session");
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Failed);
    assert!(task.error.is_some());
    let workflow = w.workflow.clone().unwrap();
    let (done, closed) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        workflow.close();
        let _ = done.send(());
    });
    closed
        .recv_timeout(Duration::from_secs(30))
        .expect("close must not wedge behind a controller guard");
    w.assert_clean();
}

#[test]
fn a_conversation_log_that_cannot_be_written_stops_the_task_and_keeps_memory_bounded() {
    use studio_engine::agent::log::RowLimits;
    let limits = RowLimits {
        max_rows: 8,
        max_bytes: 64 * 1024,
        ..RowLimits::default()
    };
    let w = World::with(Options {
        row_limits: Some(limits),
        ..Options::default()
    });
    // The log breaks after the workflow opened: a directory where the file belongs.
    let log = session_store::SessionStore::current_log_path(&w.paths, &w.snap().project).unwrap();
    let _ = fs::remove_file(&log);
    fs::create_dir(&log).unwrap();
    w.submit(&json!({"flood_tools": 3000, "hang": true}));
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Failed);
    assert_eq!(
        task.error.as_ref().map(|e| e.code.as_str()),
        Some("conversation_unsaved"),
        "{:?}",
        task.error
    );
    let s = w.snap();
    assert!(s.rows.len() <= limits.max_rows, "{} rows", s.rows.len());
    assert!(s.resources.resident_bytes <= limits.max_bytes);
    w.wf().close();
    w.assert_clean();
}

#[test]
fn stop_is_dispatched_while_the_controller_is_held_and_a_queued_refresh_waits_behind_it() {
    let w = World::new();
    let mut script = good("held");
    script["wait_file_late"] = json!("go");
    w.submit(&script);
    w.wait_phase(TaskPhase::Editing);
    // Whoever holds the controller for a long time (a slow capture or publication).
    let controller = w.ctl();
    let guard = controller.lock();
    w.release("go");
    w.wait_phase(TaskPhase::Quiescing);
    w.wf().refresh().unwrap();
    w.wf().stop().unwrap();
    // The actor dispatched Stop although a Refresh was queued ahead of it and the
    // controller is still held: neither command blocked it.
    w.wait("Stop dispatched under the barrier", |s| {
        s.task.as_ref().is_some_and(|t| t.stop_requested)
    });
    drop(guard);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Cancelled, "{:?}", task.error);
    w.assert_clean();
}

#[test]
fn stop_while_a_promotion_waits_for_the_controller_cancels_it_and_nothing_is_published() {
    let w = World::new();
    w.wf()
        .set_review_policy(studio_engine::ReviewPolicy::ManualReview)
        .unwrap();
    w.submit(&good("never published"));
    w.wait_phase(TaskPhase::AwaitingReview);
    let original = w.source("src/lib.rs");
    let controller = w.ctl();
    let guard = controller.lock();
    w.wf().apply().unwrap();
    w.wait_phase(TaskPhase::Promoting);
    w.wf().stop().unwrap();
    drop(guard);
    let task = w.wait_task(None);
    assert_eq!(
        task.phase,
        TaskPhase::Cancelled,
        "Stop wins while the controller lock is still awaited: {:?}",
        task.error
    );
    assert_eq!(w.source("src/lib.rs"), original, "nothing was published");
    assert_eq!(w.snap().history.len(), 0);
    w.assert_clean();
}

#[test]
fn a_configured_credential_never_reaches_a_persisted_row_card_or_repair_prompt() {
    let secret = "zQ9-plain-credential-0042";
    let w = World::with(Options {
        secret: Some(secret.into()),
        ..Options::default()
    });
    w.set_plan(&[
        json!({"text": "oops", "write": {"src/lib.rs": format!("// BROKEN {secret}\n"), FAKE_CONFIG: GOOD_CONFIG}}),
        good("fixed"),
    ]);
    w.wf()
        .submit(&format!("plan brief that mentions {secret} verbatim"))
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert!(task.repair.used == 1, "the compile failure was repaired");
    let rows = every_row(&w);
    let all = format!("{rows:?}");
    assert!(
        !all.contains(secret),
        "a persisted row carries the credential"
    );
    assert!(
        rows.values().any(|r| matches!(&r.kind, RowKind::User { source: UserSource::Brief, text } if text.contains("[REDACTED]"))),
        "the brief row shows the redaction"
    );
    assert!(
        rows.values().any(|r| matches!(
            &r.kind,
            RowKind::User {
                source: UserSource::Repair,
                ..
            }
        )),
        "the repair prompt is a row"
    );
    let s = w.snap();
    assert!(!format!("{:?}", s.task).contains(secret));
    let log = session_store::SessionStore::current_log_path(&w.paths, &s.project).unwrap();
    assert!(
        !fs::read_to_string(log).unwrap().contains(secret),
        "the conversation log carries the credential"
    );
    w.assert_clean();
}

fn fault_once(job: &'static str, fault: JobFault) -> JobFaults {
    let fired = Arc::new(AtomicBool::new(false));
    Arc::new(move |name| {
        if name == job && !fired.swap(true, Ordering::SeqCst) {
            fault
        } else {
            JobFault::None
        }
    })
}

#[test]
fn a_pipeline_worker_that_cannot_start_or_panics_settles_the_task_and_the_queue_continues() {
    for (fault, code) in [
        (JobFault::Refuse, "thread_failed"),
        (JobFault::Panic, "worker_failed"),
    ] {
        let w = World::with(Options {
            job_faults: Some(fault_once("pipeline", fault)),
            ..Options::default()
        });
        w.submit(&good("first"));
        w.submit(&good("second"));
        let first = w.wait_task(None);
        assert_eq!(first.phase, TaskPhase::Failed, "{fault:?}");
        assert_eq!(first.error.as_ref().map(|e| e.code.as_str()), Some(code));
        let second = w.wait_task(Some(&first.id));
        assert_eq!(second.phase, TaskPhase::Accepted, "{:?}", second.error);
        assert_eq!(w.source("src/lib.rs"), "// second\n");
        w.assert_clean();
    }
}

#[test]
fn a_promotion_worker_that_cannot_start_leaves_the_candidate_applicable() {
    let w = World::with(Options {
        job_faults: Some(fault_once("promote", JobFault::Refuse)),
        ..Options::default()
    });
    w.wf()
        .set_review_policy(studio_engine::ReviewPolicy::ManualReview)
        .unwrap();
    w.submit(&good("applied later"));
    w.wait_phase(TaskPhase::AwaitingReview);
    w.wf().apply().unwrap();
    w.wait("the refused promotion", |s| {
        has_error(s, "thread_failed")
            && s.task.as_ref().is_some_and(|t| {
                t.phase == TaskPhase::AwaitingReview
                    && t.review.as_ref().is_some_and(|r| r.apply_blocked.is_some())
            })
    });
    assert_eq!(w.engine_state(), Some(TaskState::CandidateReady));
    w.wf().apply().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(w.source("src/lib.rs"), "// applied later\n");
    w.assert_clean();
}

#[test]
fn a_promotion_worker_that_panics_fails_the_task_and_publishes_nothing() {
    let w = World::with(Options {
        job_faults: Some(fault_once("promote", JobFault::Panic)),
        ..Options::default()
    });
    w.wf()
        .set_review_policy(studio_engine::ReviewPolicy::ManualReview)
        .unwrap();
    let original = w.source("src/lib.rs");
    w.submit(&good("never published"));
    w.wait_phase(TaskPhase::AwaitingReview);
    w.wf().apply().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Failed, "{:?}", task.error);
    assert_eq!(
        task.error.as_ref().map(|e| e.code.as_str()),
        Some("worker_failed")
    );
    assert_eq!(w.source("src/lib.rs"), original);
    assert_eq!(w.engine_state(), Some(TaskState::Failed));
    w.assert_clean();
}

#[test]
fn an_undo_worker_that_cannot_start_or_panics_never_leaves_the_undo_in_progress() {
    for (fault, code) in [
        (JobFault::Refuse, "thread_failed"),
        (JobFault::Panic, "worker_failed"),
    ] {
        let w = World::with(Options {
            job_faults: Some(fault_once("undo", fault)),
            ..Options::default()
        });
        let original = w.source("src/lib.rs");
        w.submit(&good("to undo"));
        assert_eq!(w.wait_task(None).phase, TaskPhase::Accepted);
        w.wf().undo(None).unwrap();
        w.wait("the failed undo", |s| {
            has_error(s, code) && !matches!(s.undo, UndoView::InProgress { .. })
        });
        assert_eq!(w.source("src/lib.rs"), "// to undo\n", "{fault:?}");
        // The Undo is still available and now runs.
        w.wf().undo(None).unwrap();
        w.wait("the undo", |s| s.history.len() == 2);
        assert_eq!(w.source("src/lib.rs"), original);
        w.assert_clean();
    }
}

#[test]
fn a_check_sent_right_after_an_adapter_change_probes_the_new_adapter() {
    // Select saves the adapter and fires one check at once: the check must see the new
    // settings, never the previous adapter.
    let w = World::new();
    let mut changed = w.adapter_settings();
    changed.adapter.executable = "definitely-not-an-adapter-xyz".into();
    w.wf().set_adapter(Some(changed)).unwrap();
    w.wf().check_adapter().unwrap();
    let s = w.wait("the probe result", |s| {
        matches!(s.adapter.readiness, AdapterReadiness::Checked { .. })
    });
    let AdapterReadiness::Checked { report, .. } = &s.adapter.readiness else {
        unreachable!()
    };
    assert!(
        matches!(
            report.status,
            studio_harness::AdapterStatus::MissingExecutable { .. }
        ),
        "{:?}",
        report.status
    );
}

#[test]
fn a_check_still_running_when_the_adapter_changes_is_redone_for_the_new_adapter() {
    // The scripted peer's check is still running when the adapter is replaced: its result
    // describes the old adapter and must never be shown for the new one.
    let w = World::new();
    w.wf().check_adapter().unwrap();
    w.wait("the check of the scripted peer", |s| {
        s.adapter.readiness == AdapterReadiness::Checking
    });
    let mut changed = w.adapter_settings();
    changed.adapter.executable = "definitely-not-an-adapter-xyz".into();
    w.wf().set_adapter(Some(changed)).unwrap();
    let s = w.wait("the probe result", |s| {
        matches!(s.adapter.readiness, AdapterReadiness::Checked { .. })
    });
    let AdapterReadiness::Checked { report, .. } = &s.adapter.readiness else {
        unreachable!()
    };
    assert!(
        matches!(
            report.status,
            studio_harness::AdapterStatus::MissingExecutable { .. }
        ),
        "{:?}",
        report.status
    );
}

#[test]
fn a_probe_worker_that_cannot_start_reports_it_and_readiness_is_not_stuck_checking() {
    let w = World::with(Options {
        job_faults: Some(fault_once("probe", JobFault::Refuse)),
        ..Options::default()
    });
    w.wf().check_adapter().unwrap();
    w.wait("the refused probe", |s| has_error(s, "thread_failed"));
    assert_eq!(w.snap().adapter.readiness, AdapterReadiness::Unchecked);
    w.wf().check_adapter().unwrap();
    w.wait("the probe", |s| {
        matches!(s.adapter.readiness, AdapterReadiness::Checked { .. })
    });
}

#[test]
fn settings_are_refused_for_a_just_submitted_task_and_for_an_undo_before_any_snapshot_shows_them() {
    let w = World::new();
    w.wf()
        .submit(&World::brief(&json!({"hang": true})))
        .unwrap();
    assert!(matches!(
        w.wf().set_build(None),
        Err(WorkflowError::Busy(_))
    ));
    assert!(matches!(
        w.wf().set_tools(None),
        Err(WorkflowError::Busy(_))
    ));
    assert!(matches!(
        w.wf().set_adapter(None),
        Err(WorkflowError::Busy(_))
    ));
    w.wait_phase(TaskPhase::Editing);
    w.wf().stop().unwrap();
    let first = w.wait_task(None);
    w.wf().set_adapter(Some(w.adapter_settings())).unwrap();
    w.submit(&good("one edit"));
    assert_eq!(w.wait_task(Some(&first.id)).phase, TaskPhase::Accepted);
    w.wf().undo(None).unwrap();
    assert!(
        matches!(w.wf().set_adapter(None), Err(WorkflowError::Busy(_))),
        "an Undo is busy from the moment it is requested"
    );
    w.wait("the undo", |s| s.history.len() == 2);
    w.wf().set_adapter(Some(w.adapter_settings())).unwrap();
}

#[test]
fn an_open_permission_stays_answerable_after_its_card_leaves_the_transcript_window() {
    use studio_engine::agent::log::RowLimits;
    let w = World::with(Options {
        row_limits: Some(RowLimits {
            max_rows: 5,
            ..RowLimits::default()
        }),
        ..Options::default()
    });
    let mut script = good("evicted request");
    script["permission"] = json!({"title": "Edit title"});
    w.submit(&script);
    let s = w.wait_phase_snapshot(TaskPhase::WaitingPermission);
    let card = s.open_permissions()[0].clone();
    // Enough transcript traffic (one notice per queued brief) to push the card out.
    for _ in 0..8 {
        w.wf()
            .submit(&World::brief(&json!({"hang": true})))
            .unwrap();
    }
    let s = w.wait("the card to leave the window", |s| {
        s.queue.len() == 8 && s.open_permissions().is_empty()
    });
    assert_eq!(s.resources.open_permissions, 1, "the request is still open");
    assert_eq!(
        w.wf()
            .reply_permission(&card.reference, PermissionAnswer::Select("nope".into()))
            .unwrap_err(),
        WorkflowError::UnknownChoice("nope".into()),
        "a bad choice is refused without consuming the request"
    );
    w.wf()
        .reply_permission(&card.reference, PermissionAnswer::Select("allow".into()))
        .unwrap();
    assert_eq!(
        w.wf()
            .reply_permission(&card.reference, PermissionAnswer::Select("allow".into()))
            .unwrap_err(),
        WorkflowError::DuplicatePermission
    );
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    w.wf().stop().unwrap();
    w.wait("the queue to be cleared", |s| s.queue.is_empty());
    let rows = every_row(&w);
    let persisted = rows
        .values()
        .find_map(|r| match &r.kind {
            RowKind::Permission(c) if c.reference == card.reference => Some(c.state.clone()),
            _ => None,
        })
        .expect("the card is in the log");
    assert_eq!(
        persisted,
        PermissionState::Selected {
            option_id: "allow".into()
        },
        "the closure reaches the persisted card, not a second one"
    );
    assert_eq!(
        rows.values()
            .filter(|r| matches!(&r.kind, RowKind::Permission(c) if c.reference == card.reference))
            .count(),
        1
    );
    assert_eq!(w.snap().resources.open_permissions, 0);
}

/// Reports the handoff outcome from inside `handoff()`, before the job's result message.
struct EagerSink {
    workflow: Mutex<std::sync::Weak<AgentWorkflow>>,
    display: bool,
}

impl PreviewHandoff for EagerSink {
    fn handoff(&self, handoff: PromotionHandoff) -> Result<(), String> {
        let published = handoff.promotion.record.published.as_str().to_owned();
        let workflow = self
            .workflow
            .lock()
            .upgrade()
            .ok_or("the workflow is gone")?;
        if self.display {
            workflow.preview_displayed(&published).unwrap();
        } else {
            workflow
                .report_handoff(&published, Err("failed while handing off".into()))
                .unwrap();
        }
        Ok(())
    }
}

#[test]
fn an_acknowledgement_sent_from_inside_the_handoff_is_not_lost() {
    for display in [false, true] {
        let sink = Arc::new(EagerSink {
            workflow: Mutex::new(std::sync::Weak::new()),
            display,
        });
        let w = World::with(Options {
            handoff: Some(sink.clone()),
            ..Options::default()
        });
        *sink.workflow.lock() = Arc::downgrade(w.workflow.as_ref().unwrap());
        w.submit(&good("eager"));
        let task = w.wait_task(None);
        assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
        let state = w.snap().handoff.as_ref().unwrap().state.clone();
        if display {
            assert_eq!(state, HandoffState::Displayed);
        } else {
            assert_eq!(
                state,
                HandoffState::AwaitingPreview {
                    reason: Some("failed while handing off".into())
                },
                "the early failure is not overwritten by the sink's Ok"
            );
        }
    }
}

#[test]
fn a_follow_up_task_keeps_the_pending_preview_of_the_accepted_revision() {
    let sink = Arc::new(Sink {
        coordinator: Mutex::new(None),
        fail: AtomicBool::new(true),
        adopted: Mutex::new(Vec::new()),
    });
    let w = World::with(Options {
        handoff: Some(sink),
        ..Options::default()
    });
    w.submit(&good("accepted first"));
    let first = w.wait_task(None);
    assert_eq!(first.phase, TaskPhase::Accepted, "{:?}", first.error);
    let published = w.ctl().lock().state().source().as_str().to_owned();
    let awaiting = HandoffState::AwaitingPreview {
        reason: Some("preview worker refused".into()),
    };
    assert_eq!(w.snap().handoff.as_ref().unwrap().state, awaiting);
    // A follow-up that waits and is stopped never touches the accepted revision's state.
    w.submit(&json!({"hang": true}));
    w.wait_phase(TaskPhase::Editing);
    assert_eq!(w.snap().handoff.as_ref().unwrap().state, awaiting);
    // The delayed preview recovers while the follow-up runs.
    w.wf().report_handoff(&published, Ok(())).unwrap();
    w.wait("the late adoption", |s| {
        s.handoff
            .as_ref()
            .is_some_and(|h| h.state == HandoffState::Adopted)
    });
    w.wf().stop().unwrap();
    w.wait_task(Some(&first.id));
    assert_eq!(
        w.snap().handoff.as_ref().unwrap().state,
        HandoffState::Adopted
    );
}

#[test]
fn a_tool_call_updated_after_its_card_left_the_window_is_one_persisted_card() {
    use studio_engine::agent::log::RowLimits;
    use studio_harness::driver::ToolStatus;
    let w = World::with(Options {
        row_limits: Some(RowLimits {
            max_rows: 5,
            ..RowLimits::default()
        }),
        ..Options::default()
    });
    let mut script = good("interleaved tools");
    script["tool"] = Value::Array(
        (0..12)
            .map(|i| json!({"id": format!("call-{i}"), "title": format!("Tool {i}")}))
            .collect(),
    );
    script["tool_interleave"] = json!(true);
    w.submit(&script);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let rows = every_row(&w);
    let cards: Vec<&ToolCard> = rows
        .values()
        .filter_map(|r| match &r.kind {
            RowKind::Tool(card) if card.call_id.starts_with("call-") => Some(card),
            _ => None,
        })
        .collect();
    assert_eq!(cards.len(), 12, "one persisted card per tool call");
    assert!(
        cards
            .iter()
            .all(|c| c.status == Some(ToolStatus::Completed) && c.updates >= 2),
        "every card carries its final state"
    );
}

fn run_studio_tools(capability: &std::path::Path, tool: &str) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_studio-tools"))
        .arg("--capability")
        .arg(capability)
        .arg(tool)
        .output()
        .unwrap()
}

#[test]
fn the_command_line_route_has_a_task_capability_when_mcp_is_disabled_or_its_helper_is_missing() {
    for (options, why) in [
        (
            Options {
                mcp: McpStdioSupport::Unsupported,
                ..Options::default()
            },
            "disabled by host policy",
        ),
        (
            Options {
                mcp_helper: false,
                ..Options::default()
            },
            "helper was not found",
        ),
    ] {
        let w = World::with(options);
        w.submit(&json!({"hang": true}));
        w.wait_phase(TaskPhase::Editing);
        w.wait_prompts(1);
        let params: Value =
            serde_json::from_str(&fs::read_to_string(w.agent.join("session-params.json")).unwrap())
                .unwrap();
        assert!(params["mcpServers"].as_array().unwrap().is_empty());
        let s = w.snap();
        assert!(!s.mcp.active && s.mcp.cli_active);
        assert!(
            s.mcp.note.as_deref().is_some_and(|n| n.contains(why)),
            "{:?}",
            s.mcp.note
        );
        assert_eq!(s.resources.broker_grants, 1);
        let capability = s.mcp.capability_file.clone().unwrap();
        assert!(capability.is_file());
        let out = run_studio_tools(&capability, "project_context");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let reply: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(reply.is_object(), "{reply}");
        let prompt = w.evidence("prompts.jsonl")[0]["text"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(prompt.contains("studio-tools") && prompt.contains(&*capability.to_string_lossy()));
        w.wf().stop().unwrap();
        w.wait_task(None);
        assert!(!capability.exists(), "the capability died with the task");
        w.assert_clean();
    }
}

#[test]
fn without_any_tool_route_no_capability_exists_and_the_snapshot_says_so() {
    let w = World::with(Options {
        mcp_helper: false,
        cli_helper: false,
        ..Options::default()
    });
    w.submit(&json!({"hang": true}));
    w.wait_phase(TaskPhase::Editing);
    let s = w.snap();
    assert!(!s.mcp.active && !s.mcp.cli_active && s.mcp.capability_file.is_none());
    assert_eq!(s.resources.broker_grants, 0);
    assert!(
        s.mcp
            .note
            .as_deref()
            .is_some_and(|n| n.contains("no project tools")),
        "{:?}",
        s.mcp.note
    );
    w.wf().stop().unwrap();
    w.wait_task(None);
    w.assert_clean();
}

#[cfg(unix)]
#[test]
fn a_history_page_never_blocks_the_caller_or_the_actor_when_the_log_read_blocks() {
    use std::os::unix::ffi::OsStrExt;
    let w = World::new();
    // A FIFO where the log belongs: opening it for reading blocks until a writer comes.
    let log = session_store::SessionStore::current_log_path(&w.paths, &w.snap().project).unwrap();
    let _ = fs::remove_file(&log);
    let path = std::ffi::CString::new(log.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let (tx, rx) = std::sync::mpsc::channel();
    let started = Instant::now();
    w.wf()
        .history_page_with(RowId(u64::MAX), 10, move |page| {
            let _ = tx.send(page);
        })
        .unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "the caller was not blocked by the read"
    );
    let before = w.snap().revision;
    w.wf()
        .set_review_policy(studio_engine::ReviewPolicy::ManualReview)
        .unwrap();
    let after = w.wf().wait_changed(before, Duration::from_secs(10));
    assert!(
        after.revision > before,
        "the actor kept publishing while the read was blocked"
    );
    assert!(
        rx.try_recv().is_err(),
        "the page is still waiting on the disk"
    );
    // A writer arrives and leaves: the read completes.
    drop(fs::OpenOptions::new().write(true).open(&log).unwrap());
    let page = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("the page completes once the disk answers");
    assert!(page.is_ok(), "{page:?}");
}

#[test]
fn open_detached_delivers_the_workflow_on_its_own_thread_and_the_replay_never_runs_on_the_caller() {
    let w = World::new();
    w.wf().close();
    let (tx, rx) = std::sync::mpsc::channel();
    let caller = std::thread::current().id();
    AgentWorkflow::open_detached(w.config(), w.ctl(), move |opened| {
        let _ = tx.send((std::thread::current().id(), opened));
    });
    let (thread, opened) = rx.recv_timeout(Duration::from_secs(30)).unwrap();
    assert_ne!(thread, caller, "the open ran on its own thread");
    let opened = opened.expect("the project opens");
    assert_eq!(opened.snapshot().project, w.snap().project);
    opened.close_detached();
}
