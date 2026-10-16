//! Undo, review policy and recovery, finishing a task and closing the actor.

use super::*;

impl Actor {
    // ---- Undo / policy / recovery ------------------------------------------------------------------------------

    pub(super) fn start_undo(&mut self, target: Option<String>) {
        if self.active.is_some() || self.undo.is_some() {
            self.warn("Undo needs the agent to be idle.");
            return;
        }
        let Some(env) = self.env() else {
            self.error_row(present::simple_error(
                "sdk_unavailable",
                "No compatible SDK is installed",
                "Undo validates the restored source against the SDK; none is selected.",
                Some("Install or select the SDK in setup."),
                false,
            ));
            return;
        };
        let gate = Gate::new();
        let root = self.shared.root_scope.clone();
        let scopes = (root.sub_manager(), root.sub_manager());
        self.undo = Some(UndoRun {
            gate: gate.clone(),
            scopes: scopes.clone(),
        });
        self.sync_registry(0);
        self.undo_view = UndoView::InProgress {
            phase: "Preparing the Undo".into(),
        };
        self.touch();
        self.set_inflight(Some(gate.clone()));
        let playhead = self
            .shared
            .playhead
            .load(std::sync::atomic::Ordering::Relaxed);
        let sink = self.shared.handoff.clone();
        let tx = env.tx.clone();
        let refused = self.admit(
            JobKind::Undo,
            (env, target, scopes, gate, sink),
            move |(env, target, scopes, gate, sink)| {
                let end = run_undo(
                    env,
                    target,
                    RunScopes {
                        compiler: scopes.0,
                        worker: scopes.1,
                    },
                    gate,
                    playhead,
                    sink,
                );
                let _ = tx.send(Msg::Undone(end));
            },
        );
        if let Err((message, (_, _, scopes, _, _))) = refused {
            // Nothing started, so nothing can be waiting for a result.
            scopes.0.shutdown(Duration::ZERO);
            scopes.1.shutdown(Duration::ZERO);
            self.set_inflight(None);
            self.undo = None;
            self.error_row(present::simple_error(
                "thread_failed",
                "A background worker could not be started",
                &message,
                Some("Close other applications to free system resources, then try again; nothing was changed."),
                false,
            ));
            self.undo_settled();
        }
    }

    pub(super) fn on_undone(&mut self, end: UndoEnd) {
        self.set_inflight(None);
        self.undo = None;
        match end {
            UndoEnd::Committed {
                promotion,
                handoff,
                scopes,
            } => {
                scopes.compiler.shutdown(Duration::ZERO);
                self.record_promotion(&promotion, &handoff, HandoffKind::Undo);
            }
            UndoEnd::Cancelled => self.notice("Undo was stopped; nothing was changed."),
            UndoEnd::ValidationFailed(card) => {
                let summary = card.summary.clone();
                self.push_row(RowKind::Validation(card));
                self.error_row(present::simple_error(
                    "undo_validation_failed",
                    "The Undo did not validate",
                    &summary,
                    Some("Nothing was changed."),
                    false,
                ));
            }
            UndoEnd::Failed(error) => self.error_row(present::engine_error(&error, false)),
        }
        self.undo_settled();
    }

    /// An Undo ended in any way: the registry, the views and the queue move on.
    pub(super) fn undo_settled(&mut self) {
        self.sync_registry(0);
        self.refresh_idle();
        if self.switch_target.is_some() {
            self.finish_switch_teardown();
        }
        self.start_next();
    }

    pub(super) fn set_policy(&mut self, policy: ReviewPolicy) {
        let result = self.shared.controller.lock().set_review_policy(policy);
        match result {
            Ok(()) => {
                self.policy = policy;
                if self.active.is_some() {
                    self.notice("The review setting changed; it applies to the next task, not the one running.");
                }
                self.touch();
            }
            Err(error) => self.error_row(present::engine_error(&error, false)),
        }
    }

    pub(super) fn acknowledge_writer_gone(&mut self) {
        if self.active.is_some() {
            self.warn("Wait for the running task to end first.");
            return;
        }
        let result = self
            .shared
            .controller
            .lock()
            .acknowledge_agent_writer_gone(&WriterGoneEvidence::UserConfirmed);
        match result {
            Ok(()) => {
                self.interrupted_task = None;
                self.notice("The working copy was unlocked; it is archived before the next task refreshes it.");
            }
            Err(error) => self.error_row(present::engine_error(&error, true)),
        }
        self.refresh_idle();
    }

    pub(super) fn resolve_conflict(&mut self, transaction: &str, note: &str) {
        let result = self
            .shared
            .controller
            .lock()
            .resolve_conflict(transaction, note);
        match result {
            Ok(()) => self.notice("The conflict was marked resolved; every variant stays on disk."),
            Err(error) => self.error_row(present::engine_error(&error, false)),
        }
        self.refresh_idle();
    }

    // ---- finishing -------------------------------------------------------------------------------------------------

    /// Ends the active task: reaps the adapter and every owned consumer, records the
    /// terminal state in the engine (when `finish` names one; `None` means the engine
    /// already ended it), and publishes the outcome.
    pub(super) fn finalize(
        &mut self,
        finish: Option<(TaskState, String)>,
        phase: TaskPhase,
        kind: OutcomeKind,
        error: Option<StructuredError>,
    ) {
        let Some(mut active) = self.active.take() else {
            return;
        };
        self.driver_epoch += 1;
        let identity = active.ctx.identity.clone();
        // Closing a driver closes its open permissions as cancelled.
        if let Some(driver) = active.driver.take() {
            let _ = driver.cancel_prompt();
            if let Some(turn) = active.turn {
                // Give the adapter the chance to acknowledge the cancel (its permission
                // requests are already closed as cancelled); the driver kills the tree
                // itself if it does not answer within its cancel-ack deadline.
                let _ = driver.wait_turn(turn, Duration::from_secs(5));
            }
            let leftovers = driver.drain_events(256);
            for event in leftovers {
                if let AgentEventKind::MessageDelta { role, text } = event.kind {
                    self.append_message(role, &text);
                }
            }
            // What the driver saw of the writer (an escaped descendant, an unproven
            // ownership) reaches the engine before the task ends: it cannot be recovered
            // once the adapter process is gone, and the draft is classified from it.
            let outcome = driver.shutdown();
            if let Err(error) = observe_writer(&self.shared.controller, &identity, &outcome) {
                self.warn(format!(
                    "The agent's final process observation could not be recorded ({error}); the working copy stays locked until you confirm that nothing writes to it."
                ));
            }
        }
        if let Some((compiler, worker)) = active.pipeline_scopes.take() {
            compiler.shutdown(Duration::ZERO);
            worker.shutdown(Duration::ZERO);
        }
        if let Some(retained) = active.retained.take() {
            retained.scopes.compiler.shutdown(Duration::ZERO);
            retained.scopes.worker.shutdown(Duration::ZERO);
            drop(retained);
        }
        let mut engine_error = None;
        if let Some((state, reason)) = &finish {
            let result = self
                .shared
                .controller
                .lock()
                .finish_agent_task(&identity, *state, reason);
            if let Err(error) = result {
                let ended = self
                    .shared
                    .controller
                    .lock()
                    .agent_task()
                    .is_some_and(|t| t.state().is_terminal());
                if !ended {
                    engine_error = Some(present::engine_error(&error, true));
                }
            }
        }
        if let Ok(Some(mut manifest)) = SessionStore::load(&self.shared.paths, &self.shared.project)
            && manifest.provider_id == active.adapter.provider
        {
            if kind == OutcomeKind::Accepted {
                let controller = self.shared.controller.lock();
                manifest.last_source_revision = controller.current_source_revision().to_string();
                manifest.last_accepted_revision = controller.state().accepted().to_string();
                manifest.state = SessionManifestState::Ready;
            } else {
                // A retained working copy may contain edits the manifest cannot identify
                // as an immutable revision. Never resume it into a refreshed stable cwd.
                manifest.state = SessionManifestState::Unsafe;
            }
            manifest.transcript_boundary = self.store.sequence_boundary();
            manifest.updated_at = now_unix().to_string();
            match SessionStore::save(&self.shared.paths, &self.shared.project, &manifest) {
                Ok(()) => {
                    let restore_failure_notice = self
                        .session_restore
                        .as_ref()
                        .and_then(|view| view.notice.clone())
                        .filter(|notice| notice.contains("restoration failed"));
                    self.session_restore = Some(SessionRestoreView {
                        provider_id: manifest.provider_id.clone(),
                        redacted_session_id: manifest.redacted_id(),
                        is_resumable: manifest.is_resumable(),
                        last_source_revision: manifest.last_source_revision.clone(),
                        notice: if manifest.is_resumable() {
                            None
                        } else {
                            restore_failure_notice.or_else(|| {
                                Some("Previous session ended with a retained draft; a fresh session is required.".to_string())
                            })
                        },
                    });
                }
                Err(error) => self.warn(format!(
                    "Session manifest could not be updated ({error}); restoration is disabled."
                )),
            }
        }
        active.evidence_cancel.store(true, Ordering::Release);
        if let Some(tools) = &self.tools {
            tools.end_task(&identity);
        }
        // The task's own app-owned artifacts are gone with it; the view keeps their
        // ids and hashes as a record, marked released.
        if let Some(task) = &mut self.task {
            for view in [&mut task.before, &mut task.after].into_iter().flatten() {
                if matches!(view.state, EvidenceState::Ready | EvidenceState::Pending) {
                    view.state = EvidenceState::Released;
                }
            }
        }
        self.shared.live.set(None);
        // Every card still open is stale now.
        {
            let mut registry = self.shared.registry.lock();
            registry.open.retain(|r, _| r.task != identity.task);
            registry.clarification = None;
        }
        // Cards still open, wherever they live (the transcript window is not the index).
        let open_cards: Vec<RowId> = self.permission_rows.drain().map(|(_, row)| row).collect();
        for id in open_cards {
            self.update_row(id, |kind| {
                if let RowKind::Permission(card) = kind
                    && card.state == PermissionState::Open
                {
                    card.state = PermissionState::Cancelled;
                }
            });
        }
        self.set_inflight(None);
        self.finish_streaming();
        let draft_state = self
            .shared
            .controller
            .lock()
            .agent_draft_state()
            .ok()
            .flatten();
        let retained_draft = !matches!(draft_state, Some(DraftState::Accepted { .. }));
        // The task row stays visible until the next task replaces it. Rows pushed below
        // belong to it, so keep the id attached while pushing.
        let reason = finish
            .as_ref()
            .map(|(_, r)| r.clone())
            .or_else(|| error.as_ref().map(|e| e.title.clone()));
        if let Some(task) = &mut self.task {
            task.phase = phase;
            task.engine_state = engine_state_of(phase);
            task.reason = reason.clone();
            task.ended_unix = Some(now_unix());
            task.draft_state = draft_state;
            task.review = None;
            task.repair.in_progress = false;
            if error.is_some() {
                task.error = error.clone();
            }
        }
        if let Some(error) = &error {
            self.push_row(RowKind::Error(error.clone()));
        }
        if let Some(error) = engine_error {
            self.push_row(RowKind::Error(error));
        }
        let summary = match kind {
            OutcomeKind::Accepted => "The edit was applied.".to_owned(),
            OutcomeKind::Cancelled => "Stopped. The working copy was kept.".to_owned(),
            OutcomeKind::Discarded => {
                "The candidate was discarded; the working copy was kept.".to_owned()
            }
            OutcomeKind::Interrupted => {
                "The task was interrupted; the working copy was kept.".to_owned()
            }
            OutcomeKind::Conflict => {
                "The project changed while the task ran; the candidate and working copy were kept."
                    .to_owned()
            }
            _ => "The task ended without applying an edit; the working copy was kept.".to_owned(),
        };
        if kind != OutcomeKind::Accepted {
            self.push_row(RowKind::Outcome(OutcomeCard {
                kind,
                summary,
                published: None,
                draft_retained: retained_draft,
            }));
        }
        self.refresh_idle();
        self.touch();
        self.sync_registry(0);
        self.publish(true);
        if self.switch_target.is_some() {
            self.finish_switch_teardown();
        }
        self.start_next();
    }

    // ---- close ---------------------------------------------------------------------------------------------------------

    pub(super) fn shutdown_all(&mut self, done: Option<Sender<()>>) {
        self.closing = true;
        self.deferred.clear();
        let cleared = self.queue.len() + self.retained_queue.len();
        if cleared > 0 {
            let mut r = self.shared.registry.lock();
            r.pending = r.pending.saturating_sub(cleared);
            drop(r);
            self.queue.clear();
            self.retained_queue.clear();
        }
        // Cancel what can be cancelled, then let every job finish: a publication that has
        // begun is short and completes (its result is settled below).
        if let Some(undo) = &self.undo {
            undo.gate.cancel();
            undo.scopes.0.shutdown(Duration::ZERO);
            undo.scopes.1.shutdown(Duration::ZERO);
        }
        if let Some(active) = &mut self.active {
            active.gate.cancel();
            active.evidence_cancel.store(true, Ordering::Release);
            if let Some((compiler, worker)) = &active.pipeline_scopes {
                compiler.shutdown(Duration::ZERO);
                worker.shutdown(Duration::ZERO);
            }
        }
        for job in std::mem::take(&mut self.jobs) {
            let _ = job.join();
        }
        while let Ok(message) = self.rx.try_recv() {
            match message {
                Msg::Pipeline(end) => self.on_pipeline(end),
                Msg::Promote(end) => self.on_promote(end),
                Msg::Undone(end) => self.on_undone(end),
                Msg::Rejected(end) => self.on_rejected(end),
                Msg::Committed { published, kind } => self.on_committed(&published, kind),
                Msg::JobAborted { kind, message } => self.on_job_aborted(kind, &message),
                Msg::Evidence { task, kind, view } => self.on_evidence(task, kind, view),
                Msg::Probe { .. } => self.probing = false,
                Msg::Close(other) => {
                    let _ = other.send(());
                }
                _ => {}
            }
        }
        if self.active.is_some() {
            self.finalize(
                Some((TaskState::Interrupted, "project closed".into())),
                TaskPhase::Interrupted,
                OutcomeKind::Interrupted,
                None,
            );
        }
        if let Some(tools) = self.tools.take() {
            tools.shutdown();
        }
        if let Some(build) = self.shared.settings.lock().build.clone() {
            build
                .service
                .close_project(String::from(self.shared.project.clone()).as_str());
        }
        self.shared.live.set(None);
        {
            let mut registry = self.shared.registry.lock();
            registry.open.clear();
            registry.clarification = None;
        }
        self.dirty = true;
        self.publish(true);
        if let Some(done) = done {
            let _ = done.send(());
        }
    }
}
