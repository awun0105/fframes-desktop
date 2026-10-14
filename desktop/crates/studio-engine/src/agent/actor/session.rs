//! Provider failure and stop, provider handoff, session restoration and new chats.

use super::*;

impl Actor {
    // ---- provider failure and stop ----------------------------------------------------------------------

    pub(super) fn on_provider_failure(&mut self, failure: AgentFailure) {
        if failure.phase == studio_harness::driver::Phase::Session
            || failure.kind == studio_harness::driver::FailureKind::RestoreUnsupported
        {
            if let Ok(Some(mut manifest)) =
                super::session_store::SessionStore::load(&self.shared.paths, &self.shared.project)
            {
                manifest.state = super::session_store::SessionManifestState::Unsafe;
                let _ = super::session_store::SessionStore::save(
                    &self.shared.paths,
                    &self.shared.project,
                    &manifest,
                );
                let redacted_id = manifest.redacted_id();
                self.session_restore = Some(SessionRestoreView {
                    provider_id: manifest.provider_id,
                    redacted_session_id: redacted_id,
                    is_resumable: false,
                    last_source_revision: manifest.last_source_revision,
                    notice: Some(
                        "Previous session restoration failed; fresh session available.".to_string(),
                    ),
                });
            }
            self.pending_resume_session = None;
            self.notice("Session restoration failed; next task will start with a fresh session.");
            self.touch();
        }
        let error = present::provider_error(&failure, true);
        self.fail_with(error);
    }

    /// Ends the task `Failed`: the adapter is reaped, the draft retained, no writer left.
    pub(super) fn fail_with(&mut self, error: StructuredError) {
        if let Some(active) = &mut self.active {
            active.failed = true;
        }
        self.finish_streaming();
        self.finalize(
            Some((TaskState::Failed, error.title.clone())),
            TaskPhase::Failed,
            OutcomeKind::Failed,
            Some(error),
        );
    }

    pub(super) fn stop(&mut self) {
        let cleared = self.queue.len();
        if cleared > 0 {
            self.shared.registry.lock().pending -= cleared;
            self.queue.clear();
            self.notice(format!("Stop cancelled {cleared} queued brief(s)."));
        }
        if let Some(undo) = &self.undo {
            if undo.gate.cancel() {
                undo.scopes.0.shutdown(Duration::ZERO);
                undo.scopes.1.shutdown(Duration::ZERO);
                self.notice("Stopping the Undo; nothing was changed.");
            } else {
                self.notice("The Undo is being published and cannot be stopped; it will finish and can be undone again.");
            }
            return;
        }
        let Some(active) = &mut self.active else {
            if cleared == 0 {
                self.notice("Nothing is running.");
            }
            return;
        };
        active.stop_requested = true;
        let phase = active.phase;
        if let Some(task) = &mut self.task {
            task.stop_requested = true;
        }
        match phase {
            TaskPhase::Starting
            | TaskPhase::Editing
            | TaskPhase::Repairing
            | TaskPhase::WaitingPermission
            | TaskPhase::WaitingClarification => {
                self.finish_streaming();
                self.finalize(
                    Some((TaskState::Cancelled, "stopped by user".into())),
                    TaskPhase::Cancelled,
                    OutcomeKind::Cancelled,
                    None,
                );
            }
            TaskPhase::Quiescing | TaskPhase::Capturing | TaskPhase::Validating => {
                active.gate.cancel();
                if let Some((compiler, worker)) = &active.pipeline_scopes {
                    compiler.shutdown(Duration::ZERO);
                    worker.shutdown(Duration::ZERO);
                }
                self.notice("Stopping; the working copy is kept.");
            }
            TaskPhase::AwaitingEvidence => {
                active.evidence_cancel.store(true, Ordering::Release);
                self.finalize(
                    Some((
                        TaskState::Cancelled,
                        "stopped while capturing candidate evidence".into(),
                    )),
                    TaskPhase::Cancelled,
                    OutcomeKind::Cancelled,
                    None,
                );
            }
            TaskPhase::Promoting => {
                if active.gate.cancel() {
                    self.notice("Stopping before the edit is published.");
                } else {
                    self.notice(
                        "The edit is being published; that step is short and cannot be cancelled. Once it ends, check the result and use Undo if you do not want it.",
                    );
                }
            }
            TaskPhase::AwaitingReview => {
                active.stop_requested = false;
                if let Some(task) = &mut self.task {
                    task.stop_requested = false;
                }
                self.notice("Nothing is running; the candidate waits for your review. Use Discard to drop it.");
            }
            _ => {}
        }
    }

    // ---- provider handoff & restoration --------------------------------------------------------------------

    pub(super) fn initiate_switch(&mut self, target_provider: String) {
        if self.switch_pending.is_some() || self.switch_target.is_some() {
            self.warn("A provider switch is already pending. Choose continuation or cancel.");
            return;
        }

        self.switch_target = Some(target_provider);

        // Retain queued briefs across the switch: they must not be silently executed
        // on the new provider without explicit confirmation.
        self.retained_queue.extend(self.queue.drain(..));

        if self.undo.is_some() {
            self.notice("Waiting for in-flight Undo to complete before switching providers.");
            return;
        }

        if let Some(active) = &mut self.active {
            active.stop_requested = true;
            if let Some(task) = &mut self.task {
                task.stop_requested = true;
            }

            match active.phase {
                TaskPhase::Promoting => {
                    if !active.gate.cancel() {
                        self.notice("The edit is being published; waiting for publication before switching providers.");
                        return;
                    }
                    self.notice("Stopping publication before switching providers.");
                }
                TaskPhase::AwaitingReview => {
                    self.warn("A candidate is awaiting review. Apply or Discard the candidate before switching providers.");
                    self.switch_target = None;
                    self.queue.extend(self.retained_queue.drain(..));
                    self.touch();
                }
                TaskPhase::Quiescing | TaskPhase::Capturing | TaskPhase::Validating => {
                    active.gate.cancel();
                    if let Some((compiler, worker)) = &active.pipeline_scopes {
                        compiler.shutdown(Duration::ZERO);
                        worker.shutdown(Duration::ZERO);
                    }
                    self.notice("Stopping validation; waiting for background pipeline before switching providers.");
                }
                TaskPhase::AwaitingEvidence => {
                    active.evidence_cancel.store(true, Ordering::Release);
                    self.finalize(
                        Some((
                            TaskState::Cancelled,
                            "stopped while capturing candidate evidence".into(),
                        )),
                        TaskPhase::Cancelled,
                        OutcomeKind::Cancelled,
                        None,
                    );
                }
                TaskPhase::Starting
                | TaskPhase::Editing
                | TaskPhase::Repairing
                | TaskPhase::WaitingPermission
                | TaskPhase::WaitingClarification => {
                    active.gate.cancel();
                    if let Some(driver) = &active.driver {
                        let _ = driver.cancel_prompt();
                    }
                    if let Some((compiler, worker)) = &active.pipeline_scopes {
                        compiler.shutdown(Duration::ZERO);
                        worker.shutdown(Duration::ZERO);
                    }
                    self.finish_streaming();
                    if let Some(active) = &self.active
                        && let Some(tools) = &self.tools
                    {
                        tools.end_task(&active.ctx.identity);
                    }
                    self.finalize(
                        Some((TaskState::Cancelled, "switched provider".into())),
                        TaskPhase::Cancelled,
                        OutcomeKind::Cancelled,
                        None,
                    );
                }
                _ => {}
            }
        } else {
            // Idle: execute switch teardown directly
            self.finish_switch_teardown();
        }
    }

    pub(super) fn finish_switch_teardown(&mut self) {
        let Some(target_provider) = self.switch_target.clone() else {
            return;
        };

        // 1. Wait for in-flight Undo to finish
        if self.undo.is_some() {
            return;
        }

        // 2. Check draft state from engine:
        let draft_state = self.shared.controller.lock().agent_draft_store().snapshot();
        match draft_state.ok().flatten().map(|s| s.state) {
            Some(DraftState::UnsafeWriter { reason, .. }) => {
                // Writer escaped or was not observed gone: block handoff, retain draft!
                self.switch_target = None;
                self.notice(format!(
                    "Provider switch blocked: writer termination could not be verified ({reason}). Working copy retained."
                ));
                self.touch();
                return;
            }
            Some(DraftState::Retained { .. }) | None => {}
            Some(DraftState::Active { .. }) => {
                // Writer is still marked active; cannot capture yet.
                return;
            }
            _ => {}
        }

        // 3. Drain tool snapshots using WriterGate before capture
        let _drain_guard = if let Some(tools) = &self.tools {
            match tools.gate().try_acquire() {
                Some(guard) => Some(guard),
                None => {
                    // Tool snapshot is actively executing: wait for it to drain before capturing!
                    self.notice(
                        "Waiting for in-flight tool snapshot to drain before capturing draft.",
                    );
                    return;
                }
            }
        } else {
            None
        };

        // We can now finalize the switch teardown
        self.switch_target = None;
        // 4. Capture outgoing draft immutably to checkpoints
        let draft_revision = match self.shared.controller.lock().capture_outgoing_draft() {
            Ok(rev) => Some(rev.to_string()),
            Err(_) => None,
        };

        // 5. Capture source and accepted revision IDs
        let (source_revision, accepted_revision) = {
            let ctrl = self.shared.controller.lock();
            let source = ctrl.current_source_revision().to_string();
            let accepted = ctrl.state().accepted().to_string();
            (source, accepted)
        };

        let outgoing_brief = self.task.as_ref().map(|t| t.brief.clone());
        let handoff_context = self.task.as_ref().map_or_else(
            || {
                super::session_store::BoundedHandoffContext::new(
                    source_revision.clone(),
                    draft_revision.clone(),
                    "No active outgoing task".into(),
                    None,
                    None,
                    None,
                    Vec::new(),
                )
            },
            |task| {
                let mut diagnostics = task
                    .validation
                    .as_ref()
                    .into_iter()
                    .flat_map(|card| {
                        card.errors
                            .iter()
                            .map(|diagnostic| diagnostic.message.clone())
                            .chain(card.warnings.iter().cloned())
                    })
                    .collect::<Vec<_>>();
                if let Some(error) = &task.error {
                    diagnostics.push(error.detail.clone());
                }
                if let Some(conflict) = &task.conflict {
                    diagnostics.push(conflict.reason.clone());
                }
                let outcome = Some(format!(
                    "{}{}",
                    task.phase.label(),
                    task.reason
                        .as_deref()
                        .map(|reason| format!(": {reason}"))
                        .unwrap_or_default()
                ));
                let style = task
                    .scope
                    .style_snapshot
                    .as_ref()
                    .and_then(|style| serde_json::to_string(style).ok());
                super::session_store::BoundedHandoffContext::new(
                    source_revision.clone(),
                    draft_revision.clone(),
                    task.brief.clone(),
                    outcome,
                    Some(task.scope.clone()),
                    style,
                    diagnostics,
                )
            },
        );
        let view = SwitchPendingView {
            target_provider: target_provider.clone(),
            draft_revision,
            source_revision,
            accepted_revision,
            outgoing_task_brief: outgoing_brief,
            retained_queue_count: self.retained_queue.len(),
            handoff_context,
        };

        self.notice(format!(
            "Provider switch ready for {target_provider}. Choose whether to continue draft or restart from accepted."
        ));
        self.switch_pending = Some(view);
        self.touch();
    }

    pub(super) fn choose_handoff_continuation(&mut self, choice: HandoffChoice) {
        let Some(pending) = self.switch_pending.take() else {
            self.warn("No provider switch is pending.");
            return;
        };
        let selected_revision = match choice {
            HandoffChoice::ContinueDraft => pending
                .draft_revision
                .as_deref()
                .unwrap_or(&pending.accepted_revision),
            HandoffChoice::RestartFromAccepted => &pending.accepted_revision,
        };
        let draft_choice = match handoff_draft_choice(&pending, selected_revision) {
            Ok(choice) => choice,
            Err(error) => {
                self.warn(format!("Provider switch blocked: {error}."));
                self.switch_pending = Some(pending);
                self.touch();
                return;
            }
        };

        // Switch adapter settings to target provider
        let target_prov = pending.target_provider.clone();
        let mut registry = match crate::providers::host::load_registry(&self.shared.paths) {
            Ok(Some(reg)) => reg,
            Ok(None) => crate::providers::provider_profiles::ProviderRegistry::default(),
            Err(e) => {
                self.warn(format!(
                    "Provider switch blocked: could not load registry ({e})."
                ));
                self.switch_pending = Some(pending);
                self.touch();
                return;
            }
        };

        let Some(profile) = registry
            .profiles
            .iter()
            .find(|p| p.id == target_prov || p.label == target_prov)
            .cloned()
        else {
            self.warn(format!(
                "Provider switch to {} blocked: profile not found in registry.",
                target_prov
            ));
            self.switch_pending = Some(pending);
            self.touch();
            return;
        };

        let (settings, resolution) = profile
            .adapter
            .resolve(&self.shared.paths, &self.shared.ownership_policy);
        if !resolution.ownership.is_qualified() {
            self.warn(format!(
                "Provider switch to {} blocked: adapter is not qualified.",
                target_prov
            ));
            self.switch_pending = Some(pending);
            self.touch();
            return;
        }
        if let Err(error) = registry.select_profile(&profile.id) {
            self.warn(format!(
                "Provider switch blocked: could not select profile ({error})."
            ));
            self.switch_pending = Some(pending);
            self.touch();
            return;
        }
        if let Err(error) = crate::providers::host::save_registry(&self.shared.paths, &registry) {
            self.warn(format!(
                "Provider switch blocked: could not save the selected profile ({error})."
            ));
            self.switch_pending = Some(pending);
            self.touch();
            return;
        }
        self.shared.settings.lock().adapter = Some(settings);
        self.settings_changed();

        self.next_draft_choice = Some(draft_choice);
        self.next_handoff_context = Some(pending.handoff_context.clone());
        match choice {
            HandoffChoice::ContinueDraft => {
                self.notice(format!(
                    "Continuing from the selected outgoing revision ({}); current source and accepted history will be rechecked before launch.",
                    selected_revision
                ));
            }
            HandoffChoice::RestartFromAccepted => {
                self.notice("Restarting from the exact accepted source revision; current source and accepted history will be rechecked before launch.");
            }
        }

        if !self.retained_queue.is_empty() {
            self.notice(format!(
                "Provider switch complete. {} queued brief(s) from previous provider retained; choose Transfer or Clear.",
                self.retained_queue.len()
            ));
        }

        self.touch();
        self.start_next();
    }

    pub(super) fn cancel_switch(&mut self) {
        if self.switch_pending.is_some() || self.switch_target.is_some() {
            self.switch_pending = None;
            self.switch_target = None;
            self.queue.extend(self.retained_queue.drain(..));
            self.notice("Provider switch cancelled; remaining on current provider.");
            self.touch();
            self.start_next();
        }
    }

    pub(super) fn transfer_retained_queue(&mut self) {
        let count = self.retained_queue.len();
        if count == 0 {
            self.notice("No retained queued briefs to transfer.");
            return;
        }
        self.queue.extend(self.retained_queue.drain(..));
        self.notice(format!(
            "Transferred {count} queued brief(s) to new provider."
        ));
        self.touch();
        self.start_next();
    }

    pub(super) fn clear_retained_queue(&mut self) {
        let count = self.retained_queue.len();
        if count == 0 {
            self.notice("No retained queued briefs to clear.");
            return;
        }
        self.retained_queue.clear();
        {
            let mut r = self.shared.registry.lock();
            r.pending = r.pending.saturating_sub(count);
        }
        self.notice(format!("Cleared {count} retained queued brief(s)."));
        self.touch();
    }

    pub(super) fn on_session_ready(&mut self, _sanitized_id: String, restored: bool) {
        let launch_digest = self
            .active
            .as_ref()
            .and_then(|active| session_launch_digest(active, &self.shared.paths).ok());
        let Some(launch_digest) = launch_digest else {
            self.warn("Session restoration disabled: exact adapter launch identity could not be verified.");
            self.session_restore = Some(SessionRestoreView {
                provider_id: self
                    .active
                    .as_ref()
                    .map(|active| active.adapter.provider.clone())
                    .unwrap_or_default(),
                redacted_session_id: "unavailable".to_string(),
                is_resumable: false,
                last_source_revision: self
                    .active
                    .as_ref()
                    .map(|active| active.ctx.source_base.revision().to_string())
                    .unwrap_or_default(),
                notice: Some("Exact adapter launch identity could not be verified".to_string()),
            });
            self.touch();
            return;
        };
        let wire_id = self
            .active
            .as_ref()
            .and_then(|a| a.driver.as_ref().and_then(|d| d.wire_session_id()));
        if let Some(wire_id) = wire_id {
            if let Some(active) = &self.active {
                let accepted_rev = self.shared.controller.lock().state().accepted().to_string();
                let now_ts = now_unix().to_string();
                let manifest = SessionManifest {
                    schema_version: session_store::SESSION_MANIFEST_SCHEMA_VERSION,
                    provider_id: active.adapter.provider.clone(),
                    wire_session_id: Some(wire_id),
                    project_id: self.shared.project.to_string(),
                    draft_path: active.ctx.draft.clone(),
                    launch_digest: launch_digest.clone(),
                    last_source_revision: active.ctx.source_base.revision().to_string(),
                    last_accepted_revision: accepted_rev,
                    context_schema: session_store::CONTEXT_SCHEMA_VERSION.to_string(),
                    capability_snapshot: self.capabilities.clone(),
                    transcript_boundary: self.store.sequence_boundary(),
                    state: SessionManifestState::Ready,
                    created_at: now_ts.clone(),
                    updated_at: now_ts,
                };
                match SessionStore::save(&self.shared.paths, &self.shared.project, &manifest) {
                    Ok(()) => {
                        let redacted_id = manifest.redacted_id();
                        self.session_restore = Some(SessionRestoreView {
                            provider_id: manifest.provider_id,
                            redacted_session_id: redacted_id,
                            is_resumable: true,
                            last_source_revision: manifest.last_source_revision,
                            notice: None,
                        });
                    }
                    Err(e) => {
                        self.warn(format!(
                            "Session manifest could not be saved ({e}); session restoration is disabled."
                        ));
                        let redacted_id = manifest.redacted_id();
                        self.session_restore = Some(SessionRestoreView {
                            provider_id: manifest.provider_id,
                            redacted_session_id: redacted_id,
                            is_resumable: false,
                            last_source_revision: manifest.last_source_revision,
                            notice: Some(format!("Save failure: {e}")),
                        });
                    }
                }
            }
        } else {
            self.notice("Session restoration disabled: provider session ID required credential redaction and cannot be safely resumed.");
            self.session_restore = Some(SessionRestoreView {
                provider_id: self
                    .active
                    .as_ref()
                    .map(|a| a.adapter.provider.clone())
                    .unwrap_or_default(),
                redacted_session_id: "none".to_string(),
                is_resumable: false,
                last_source_revision: self
                    .active
                    .as_ref()
                    .map(|a| a.ctx.source_base.revision().to_string())
                    .unwrap_or_default(),
                notice: Some("Session restoration disabled (redacted ID)".to_string()),
            });
        }
        if restored {
            self.notice(
                "Session restored from previous provider state; fresh task capabilities issued.",
            );
        }
        self.touch();
    }

    pub(super) fn restore_session(&mut self) {
        match SessionStore::load(&self.shared.paths, &self.shared.project) {
            Ok(Some(manifest)) if manifest.is_resumable() => {
                let (project_id, draft_path, source_revision, accepted_revision, draft_state) = {
                    let controller = self.shared.controller.lock();
                    (
                        self.shared.project.to_string(),
                        controller.agent_draft_store().path().to_path_buf(),
                        controller.current_source_revision().to_string(),
                        controller.state().accepted().to_string(),
                        controller.agent_draft_state(),
                    )
                };
                if manifest.project_id != project_id {
                    self.notice("Saved session belongs to a different project; a fresh session will be used.");
                    self.touch();
                    return;
                }
                if manifest.draft_path != draft_path {
                    self.notice(
                        "Saved session draft location changed; a fresh session will be used.",
                    );
                    self.touch();
                    return;
                }
                match draft_state {
                    Ok(None) | Ok(Some(DraftState::Accepted { .. })) => {}
                    Ok(Some(DraftState::UnsafeWriter { .. })) => {
                        self.notice("Saved session cannot be resumed while writer termination is unverified; preserve the draft and resolve recovery first.");
                        self.touch();
                        return;
                    }
                    Ok(Some(_)) => {
                        self.notice("Saved session has a retained draft that cannot be safely matched to native memory; a fresh session will be used.");
                        self.touch();
                        return;
                    }
                    Err(error) => {
                        self.notice(format!("Saved session cannot be resumed because draft safety could not be verified ({error}); a fresh session will be used."));
                        self.touch();
                        return;
                    }
                }
                if manifest.last_source_revision != source_revision
                    || manifest.last_accepted_revision != accepted_revision
                {
                    self.notice("Saved session source or accepted revision changed; a fresh session will be used.");
                    self.touch();
                    return;
                }
                let Some(wire_id) = manifest.wire_session_id.clone() else {
                    self.notice(
                        "Saved session has no resumable session ID; a fresh session will be used.",
                    );
                    return;
                };
                self.pending_resume_session = Some(PendingResumeSession {
                    provider_id: manifest.provider_id.clone(),
                    launch_digest: manifest.launch_digest.clone(),
                    wire_session_id: wire_id,
                    draft_path: manifest.draft_path.clone(),
                    last_source_revision: manifest.last_source_revision.clone(),
                    last_accepted_revision: manifest.last_accepted_revision.clone(),
                });
                self.notice(format!(
                    "Restoring session for provider {} (session {})",
                    manifest.provider_id,
                    manifest.redacted_id()
                ));
                self.touch();
            }
            Ok(Some(manifest)) => {
                self.notice(
                    "Saved session cannot be safely resumed; a fresh session will be used.",
                );
                let redacted_id = manifest.redacted_id();
                self.session_restore = Some(SessionRestoreView {
                    provider_id: manifest.provider_id,
                    redacted_session_id: redacted_id,
                    is_resumable: false,
                    last_source_revision: manifest.last_source_revision,
                    notice: Some("Session cannot be resumed".to_string()),
                });
                self.touch();
            }
            _ => {
                self.notice(
                    "No saved session manifest found to restore; fresh session will be used.",
                );
            }
        }
    }

    pub(super) fn dismiss_restoration(&mut self) {
        if let Err(error) = SessionStore::clear(&self.shared.paths, &self.shared.project) {
            self.warn(format!(
                "The saved session could not be dismissed and stays on offer: {error}"
            ));
            self.touch();
            return;
        }
        self.session_restore = None;
        self.pending_resume_session = None;
        self.notice("Saved session dismissed.");
        self.touch();
    }

    /// Why a New chat cannot start now; `None` when the conversation can be reset.
    pub(super) fn new_chat_blocker(&self) -> Option<&'static str> {
        if self
            .active
            .as_ref()
            .is_some_and(|a| a.phase == TaskPhase::AwaitingReview)
        {
            Some("Apply or Discard the candidate before starting a new chat.")
        } else if self.active.is_some() || self.undo.is_some() {
            Some("Stop the running task or wait for it to finish before starting a new chat.")
        } else if self.switch_target.is_some() || self.switch_pending.is_some() {
            Some("Finish or cancel the provider switch before starting a new chat.")
        } else if !self.queue.is_empty() || !self.retained_queue.is_empty() {
            Some("Cancel or clear the queued briefs before starting a new chat.")
        } else if !self.recovery.unresolved_conflicts.is_empty()
            || self.task.as_ref().is_some_and(|t| t.conflict.is_some())
        {
            Some("Resolve the conflict before starting a new chat.")
        } else {
            None
        }
    }

    pub(super) fn new_chat(&mut self) {
        if let Some(reason) = self.new_chat_blocker() {
            self.warn(reason);
            self.touch();
            return;
        }
        // The saved session goes first: if it stayed, the archived chat would be offered
        // again on the next launch.
        if let Err(error) = SessionStore::clear(&self.shared.paths, &self.shared.project) {
            self.warn(format!("A new chat could not be started: {error}"));
            self.touch();
            return;
        }
        self.session_restore = None;
        self.pending_resume_session = None;
        if let Err(error) = self.store.start_new(now_unix()) {
            self.warn(format!("A new chat could not be started: {error}"));
            self.touch();
            return;
        }
        self.stream = None;
        self.tool_rows.clear();
        self.tool_order.clear();
        self.permission_rows.clear();
        self.task = None;
        self.chat += 1;
        self.touch();
    }
}
