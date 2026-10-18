//! Restart and recovery of interrupted tasks, publications and Undos.

use super::*;

#[test]
fn an_interrupted_task_is_recovered_on_open_with_its_draft_locked_until_confirmed() {
    let mut w = World::new();
    w.submit(&json!({"text": "partial", "write": {"scratch.txt": "partial"}, "hang": true}));
    w.wait_phase(TaskPhase::Editing);
    w.poll("the partial write", || {
        w.ctl()
            .lock()
            .agent_draft_store()
            .path()
            .join("scratch.txt")
            .exists()
    });
    let task = w.snap().task.clone().unwrap();
    w.wf().stop().unwrap();
    w.wait_task(None);
    // The app died mid-task: the marker still says Active.
    let project = w.ctl().lock().project.manifest.project_id.clone();
    let marker = w.paths.agent_draft_state(&project);
    let mut value: Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
    value["state"] = json!({"state": "active", "task": task.id.0});
    fs::write(&marker, serde_json::to_vec(&value).unwrap()).unwrap();
    w.restart();
    let s = w.snap();
    let notice = s.recovery.draft.as_ref().expect("recovery draft notice");
    assert!(notice.interrupted_by_restart && notice.can_acknowledge);
    assert!(matches!(notice.state, DraftState::UnsafeWriter { .. }));
    assert!(s.rows.iter().any(
        |r| matches!(&r.kind, RowKind::Outcome(o) if o.kind == OutcomeKind::Interrupted)
            && r.task.as_ref() == Some(&task.id)
    ));
    // The conversation of the earlier session is still there (persisted rows).
    assert!(s.rows.iter().any(|r| matches!(
        &r.kind,
        RowKind::User {
            source: UserSource::Brief,
            ..
        }
    )));
    // The retained draft is not refreshed while it may still be written.
    w.submit(&good("blocked"));
    w.wait("the draft lock", |s| has_error(s, "draft_unsafe"));
    w.wf().acknowledge_writer_gone().unwrap();
    w.wait("unlocked", |s| {
        matches!(
            s.recovery.draft.as_ref().map(|d| &d.state),
            Some(DraftState::Retained { .. })
        )
    });
    w.submit(&good("recovered"));
    let next = w.wait_task(None);
    assert_eq!(next.phase, TaskPhase::Accepted, "{:?}", next.error);
    // Opening twice does not duplicate the interrupted outcome.
    let interrupted = w
        .snap()
        .rows
        .iter()
        .filter(|r| matches!(&r.kind, RowKind::Outcome(o) if o.kind == OutcomeKind::Interrupted))
        .count();
    assert_eq!(interrupted, 1);
}

struct CrashAtPublish {
    armed: AtomicBool,
}

impl TransactionHooks for CrashAtPublish {
    fn at(&self, boundary: &Boundary) -> Result<(), Fault> {
        if self.armed.load(Ordering::SeqCst) && matches!(boundary, Boundary::AfterPublish(_)) {
            return Err(Fault::Crash);
        }
        Ok(())
    }
}

#[test]
fn an_interrupted_publication_is_rolled_back_by_recovery_and_work_continues() {
    let hooks = Arc::new(CrashAtPublish {
        armed: AtomicBool::new(true),
    });
    let mut w = World::with(Options {
        hooks: Some(hooks.clone()),
        ..Options::default()
    });
    let original = w.source("src/lib.rs");
    w.submit(&good("half published"));
    let task = w.wait_task(None);
    assert_ne!(
        task.phase,
        TaskPhase::Accepted,
        "the process 'died' mid-publication"
    );
    assert!(
        matches!(
            task.error.as_ref().map(|e| e.code.as_str()),
            Some("promotion_failed")
        ),
        "{:?}",
        task.error
    );
    assert_eq!(w.snap().history.len(), 0);
    hooks.armed.store(false, Ordering::SeqCst);
    w.restart();
    let s = w.snap();
    assert_eq!(
        s.recovery.rolled_back.len(),
        1,
        "recovery rolled the transaction back"
    );
    assert_eq!(
        w.source("src/lib.rs"),
        original,
        "the half-published file was restored"
    );
    assert!(s.history.is_empty());
    w.submit(&good("after recovery"));
    let next = w.wait_task(None);
    assert_eq!(next.phase, TaskPhase::Accepted, "{:?}", next.error);
}

#[test]
fn an_interrupted_undo_is_rolled_back_by_recovery_and_stays_available() {
    let hooks = Arc::new(CrashAtPublish {
        armed: AtomicBool::new(false),
    });
    let mut w = World::with(Options {
        hooks: Some(hooks.clone()),
        ..Options::default()
    });
    w.submit(&good("kept"));
    w.wait_task(None);
    assert_eq!(w.snap().history.len(), 1);
    hooks.armed.store(true, Ordering::SeqCst);
    w.wf().undo(None).unwrap();
    w.wait("the failed Undo", |s| has_error(s, "promotion_failed"));
    hooks.armed.store(false, Ordering::SeqCst);
    w.restart();
    let s = w.snap();
    assert_eq!(s.recovery.rolled_back.len(), 1);
    assert_eq!(
        w.source("src/lib.rs"),
        "// kept\n",
        "the accepted edit survived the interrupted Undo"
    );
    assert_eq!(s.history.len(), 1);
    assert!(matches!(s.undo, UndoView::Available { .. }));
}

struct BlockAtPublish {
    reached: AtomicBool,
    release: AtomicBool,
}

impl TransactionHooks for BlockAtPublish {
    fn at(&self, boundary: &Boundary) -> Result<(), Fault> {
        if matches!(boundary, Boundary::BeforePublish(_)) && !self.release.load(Ordering::SeqCst) {
            self.reached.store(true, Ordering::SeqCst);
            while !self.release.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        Ok(())
    }
}

#[test]
fn stop_during_publication_cannot_cancel_the_commit_and_reports_it() {
    let hooks = Arc::new(BlockAtPublish {
        reached: AtomicBool::new(false),
        release: AtomicBool::new(false),
    });
    let w = World::with(Options {
        hooks: Some(hooks.clone()),
        ..Options::default()
    });
    w.submit(&good("boundary"));
    let deadline = Instant::now() + WAIT;
    while !hooks.reached.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline, "publication never started");
        std::thread::sleep(Duration::from_millis(5));
    }
    let task = w.wait_phase(TaskPhase::Promoting);
    assert_eq!(task.engine_state, TaskState::Promoting);
    w.wf().stop().unwrap();
    w.wait("the explanation", |s| {
        s.rows.iter().any(|r| {
            matches!(&r.kind, RowKind::Notice { text, .. } if text.starts_with("The edit is being published"))
        })
    });
    assert_eq!(w.snap().task.as_ref().unwrap().phase, TaskPhase::Promoting);
    hooks.release.store(true, Ordering::SeqCst);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(
        w.source("src/lib.rs"),
        "// boundary\n",
        "the commit was never cancelled"
    );
    let s = w.snap();
    assert!(s.rows.iter().any(
        |r| matches!(&r.kind, RowKind::Notice { level: NoticeLevel::Warning, text }
        if text.starts_with("Stop arrived while the edit was being published"))
    ));
    assert_eq!(s.history.len(), 1);
    assert!(
        matches!(s.undo, UndoView::Available { .. }),
        "Undo is the recovery"
    );
    w.assert_clean();
}

#[test]
fn a_blocked_publication_gate_retains_the_candidate_and_apply_can_be_retried() {
    let w = World::new();
    w.ctl()
        .lock()
        .override_apply_gate(Some(studio_engine::ApplyGate::Blocked(
            "test filesystem".into(),
        )));
    let original = w.source("src/lib.rs");
    w.submit(&good("blocked gate"));
    let task = w.wait_phase(TaskPhase::AwaitingReview);
    let review = task.review.as_ref().unwrap();
    assert!(
        review
            .apply_blocked
            .as_deref()
            .is_some_and(|r| r.contains("test filesystem")),
        "{review:?}"
    );
    assert!(
        has_error(&w.snap(), "apply_blocked"),
        "{:?}",
        w.snap().task.as_ref().and_then(|t| t.error.clone())
    );
    assert_eq!(w.source("src/lib.rs"), original);
    assert_eq!(w.engine_state(), Some(TaskState::CandidateReady));
    // Export still works while Apply is blocked.
    let export = w._temp.path().join("blocked-export");
    w.wf().export_candidate(&export).unwrap();
    w.wait("the export", |s| {
        s.rows.iter().any(|r| matches!(&r.kind, RowKind::Notice { text, .. } if text.starts_with("Exported the candidate")))
    });
    w.ctl().lock().override_apply_gate(None);
    w.wf().apply().unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(w.source("src/lib.rs"), "// blocked gate\n");
    w.assert_clean();
}
