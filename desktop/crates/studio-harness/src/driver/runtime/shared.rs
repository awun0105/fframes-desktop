//! State shared between the host handle, the connection task and the watchdog.

use super::*;

impl Shared {
    pub(in crate::driver) fn phase(&self) -> Phase {
        self.core.lock().phase
    }

    pub(super) fn set_phase(&self, phase: Phase) {
        self.core.lock().phase = phase;
    }

    /// Redacted, length-bounded copy of any wire string that is exported to hosts.
    pub(super) fn clean(&self, raw: &str) -> String {
        truncate_label(&self.redactor.redact(raw))
    }

    // ---- stderr (incrementally sanitized) ------------------------------------------

    pub(in crate::driver) fn stderr_push(&self, bytes: &[u8]) {
        let max = self.limits.max_stderr_bytes;
        let mut state = self.stderr.lock();
        state.carry.extend_from_slice(bytes);
        let mut text = String::new();
        loop {
            match std::str::from_utf8(&state.carry) {
                Ok(valid) => {
                    text.push_str(valid);
                    state.carry.clear();
                    break;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    text.push_str(
                        std::str::from_utf8(&state.carry[..valid]).expect("validated prefix"),
                    );
                    match error.error_len() {
                        Some(bad) => {
                            text.push('\u{fffd}');
                            state.carry.drain(..valid + bad);
                        }
                        None => {
                            // An incomplete multibyte sequence waits for its tail.
                            state.carry.drain(..valid);
                            break;
                        }
                    }
                }
            }
        }
        let released = state.redactor.push(&text);
        state.append(&released, max);
    }

    /// End of stderr (EOF or driver close): releases everything still held.
    pub(in crate::driver) fn stderr_finish(&self) {
        let max = self.limits.max_stderr_bytes;
        let mut state = self.stderr.lock();
        let carry = std::mem::take(&mut state.carry);
        let mut released = state.redactor.push(&String::from_utf8_lossy(&carry));
        released.push_str(&state.redactor.finish());
        state.append(&released, max);
    }

    pub(super) fn stderr_text(&self, last: usize) -> String {
        let state = self.stderr.lock();
        let skip = state.tail.len().saturating_sub(last);
        let bytes: Vec<u8> = state.tail.iter().skip(skip).copied().collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    // ---- events and failures -------------------------------------------------------

    /// Queues an event; false when it could not be queued (overflow or already closed).
    pub(super) fn emit(&self, kind: AgentEventKind) -> bool {
        // Remember short-lived scope members for escape attribution (throttled; skipped
        // when the tracker is busy, the next event records them).
        if let Some(child) = self.child.try_lock() {
            child.note_members();
        }
        match self.queue.push(kind) {
            PushOutcome::Queued => true,
            PushOutcome::Closed => false,
            PushOutcome::Overflow => {
                // The queue already holds the visible overflow failure event.
                self.record_failure(AgentFailure::new(
                    FailureKind::EventQueueOverflow,
                    self.phase(),
                    "event queue overflowed; consumer too slow",
                ));
                self.request_kill();
                false
            }
        }
    }

    /// Records the first failure only; returns whether this call recorded it.
    pub(super) fn record_failure(&self, failure: AgentFailure) -> bool {
        let mut core = self.core.lock();
        if core.failure.is_some() {
            return false;
        }
        core.failure = Some(failure);
        self.core_changed.notify_all();
        true
    }

    pub(super) fn sanitize(&self, mut failure: AgentFailure) -> AgentFailure {
        failure.message = truncate_label(&self.redactor.redact(&failure.message));
        failure.detail = failure
            .detail
            .map(|d| self.redactor.redact(&d))
            .or_else(|| {
                let text = self.stderr_text(FAILURE_DETAIL_BYTES);
                (!text.trim().is_empty()).then_some(text)
            });
        failure
    }

    /// Latches a terminal failure, emits it once and wakes the reaper. Never blocks on
    /// process cleanup, so it is safe from handle methods and SDK callbacks alike.
    pub(in crate::driver) fn fail(&self, failure: AgentFailure) {
        let failure = self.sanitize(failure);
        if self.record_failure(failure.clone()) {
            // The event goes in before the kill so consumers see cause before effect.
            let _ = self.queue.push(AgentEventKind::Failure(failure));
        }
        self.request_kill();
    }

    pub(super) fn request_kill(&self) {
        self.watchdog.request_kill();
    }

    pub(super) fn note_escapes(&self, pids: Vec<u32>) {
        if pids.is_empty() {
            return;
        }
        let mut core = self.core.lock();
        for pid in pids {
            if !core.escaped.contains(&pid) {
                core.escaped.push(pid);
            }
        }
    }

    /// Records the result of one escape scan of the owned tree.
    pub(super) fn note_scan(&self, scan: EscapeScan) {
        match scan {
            EscapeScan::Clean => {}
            EscapeScan::Escaped(pids) => self.note_escapes(pids),
            EscapeScan::Unverifiable(reason) => {
                self.note_unverified(&format!("escape scan failed: {reason}"));
            }
        }
    }

    /// Keeps the first reason containment could not be verified; ownership is demoted.
    pub(super) fn note_unverified(&self, reason: &str) {
        let reason = self.clean(reason);
        let mut core = self.core.lock();
        if core.containment_unverified.is_none() {
            core.containment_unverified = Some(reason);
        }
    }

    /// Forced stop of the owned tree. Runs on the reaper thread or in `stop()`, never on
    /// a handle caller's thread. Escapes are recorded first: once the parent dies an
    /// escaped helper is reparented and can no longer be attributed.
    pub(super) fn kill_now(&self) {
        let scan = self.child.lock().escaped_descendants();
        self.note_scan(scan);
        self.close_permissions();
        self.forced_kill.store(true, Ordering::SeqCst);
        let _ = self.child.lock().kill_forcefully();
    }

    pub(super) fn is_stopping(&self) -> bool {
        self.core.lock().stopping
    }

    // ---- permissions ---------------------------------------------------------------

    pub(super) fn push_permission_closed(&self, ids: &[PermissionId]) {
        for id in ids {
            let _ = self.queue.push(AgentEventKind::PermissionClosed {
                id: *id,
                resolution: PermissionResolution::Cancelled,
            });
        }
    }

    /// Closes every open permission request as cancelled; returns how many were open.
    pub(super) fn close_permissions(&self) -> u32 {
        let closed = self.core.lock().drain_permissions();
        self.push_permission_closed(&closed);
        closed.len() as u32
    }

    pub(super) fn on_permission_request(
        &self,
        request: RequestPermissionRequest,
        responder: Responder<RequestPermissionResponse>,
    ) -> Result<(), RpcError> {
        self.flush_before_event();
        let wire_id = responder.id().to_string();
        let sink = Box::new(AcpResponder(responder));
        if self.core.lock().session.as_deref() != Some(&*request.session_id.0) {
            let _ = sink.respond(WireAnswer::Cancelled);
            return Err(self.protocol_violation("permission request for another session"));
        }
        let tool_call_id = self.clean(&request.tool_call.tool_call_id.0);
        let title = self.clean(
            request
                .tool_call
                .fields
                .title
                .as_deref()
                .unwrap_or("Tool permission"),
        );
        let mut taken = HashMap::new();
        let mut options = Vec::new();
        let mut choices = Vec::new();
        for option in &request.options {
            let raw = option.option_id.0.to_string();
            let exported = self.export_id(&raw, &mut taken);
            choices.push(PermissionChoice {
                option_id: exported.clone(),
                name: self.clean(&option.name),
                kind: enum_label(&option.kind),
            });
            options.push((exported, raw));
        }
        match self.admit_pending(wire_id, options, sink, false) {
            Err(violation) => Err(self.protocol_violation(violation)),
            Ok(None) => Ok(()),
            Ok(Some((id, turn))) => {
                self.emit(AgentEventKind::PermissionRequested(PermissionPrompt {
                    id,
                    turn,
                    tool_call_id,
                    title,
                    options: choices,
                }));
                Ok(())
            }
        }
    }

    /// Admits one permission or question request for the active turn. Returns its id and
    /// turn, `None` when no active, uncancelled turn could ever answer it (it is answered
    /// as cancelled), or the protocol violation (also answered as cancelled; the caller
    /// fails the driver). The caller emits the request event.
    pub(in crate::driver) fn admit_pending(
        &self,
        wire_id: String,
        options: Vec<(String, String)>,
        sink: Box<dyn PermissionSink>,
        question: bool,
    ) -> Result<Option<(PermissionId, u64)>, &'static str> {
        // One critical section: cancel latch, duplicate check and insertion cannot
        // interleave with Stop, a grant, or turn completion.
        let mut core = self.core.lock();
        let active = match (&core.turn, core.stopping) {
            (Some(turn), false) if !turn.cancel_requested => Some(turn.id),
            _ => None,
        };
        let refused = if active.is_none() {
            None
        } else if core.permissions.wire_ids.contains(&wire_id) {
            Some("duplicate in-flight permission request id")
        } else if core.permissions.pending.len() >= MAX_PENDING_PERMISSIONS {
            Some("too many pending permission requests")
        } else {
            let id = core.permissions.next;
            core.permissions.next += 1;
            core.permissions.wire_ids.insert(wire_id.clone());
            core.permissions.pending.insert(
                id,
                PendingPermission {
                    sink,
                    options,
                    wire_id,
                    question,
                },
            );
            drop(core);
            // An admitted request for the active turn is agent activity.
            self.note_turn_activity();
            return Ok(active.map(|turn| (PermissionId(id), turn)));
        };
        drop(core);
        let _ = sink.respond(WireAnswer::Cancelled);
        match refused {
            Some(violation) => Err(violation),
            None => {
                // No active, uncancelled turn: such a request can never be granted.
                self.late.fetch_add(1, Ordering::SeqCst);
                Ok(None)
            }
        }
    }

    /// Exported (redacted, collision-free) copy of a wire id.
    pub(super) fn export_id(&self, raw: &str, taken: &mut HashMap<String, String>) -> String {
        let base = {
            let cleaned = self.clean(raw);
            if cleaned.is_empty() {
                "id".to_owned()
            } else {
                cleaned
            }
        };
        let mut candidate = base.clone();
        let mut n = 1;
        while let Some(existing) = taken.get(&candidate) {
            if existing == raw {
                return candidate;
            }
            n += 1;
            candidate = format!("{base}~{n}");
        }
        taken.insert(candidate.clone(), raw.to_owned());
        candidate
    }

    pub(in crate::driver) fn reply_permission(
        &self,
        id: PermissionId,
        reply: PermissionReply,
    ) -> Result<(), DriverError> {
        self.answer_pending(id, |pending| match &reply {
            PermissionReply::Select(_) if pending.question => Err(DriverError::InvalidOption(
                "a question is answered with answers, not an option".to_owned(),
            )),
            PermissionReply::Select(option) => {
                let Some((exported, raw)) = pending
                    .options
                    .iter()
                    .find(|(exported, _)| exported == option)
                else {
                    return Err(DriverError::InvalidOption(
                        "unknown permission option".to_owned(),
                    ));
                };
                Ok((
                    WireAnswer::Selected(raw.clone()),
                    PermissionResolution::Selected {
                        option_id: exported.clone(),
                    },
                ))
            }
            PermissionReply::Cancel => Ok((WireAnswer::Cancelled, PermissionResolution::Cancelled)),
        })
    }

    /// Answers one pending question request with one answer per question.
    pub(in crate::driver) fn answer_question(
        &self,
        id: PermissionId,
        answers: Vec<crate::driver::QuestionAnswer>,
    ) -> Result<(), DriverError> {
        self.answer_pending(id, |pending| {
            if !pending.question {
                return Err(DriverError::InvalidOption(
                    "a permission request is answered with an option".to_owned(),
                ));
            }
            Ok((
                WireAnswer::Answers(answers),
                PermissionResolution::Selected {
                    option_id: "answered".to_owned(),
                },
            ))
        })
    }

    /// Lookup, validation, removal and the wire reply are one step under `core`, so a
    /// concurrent Stop either cancels the request first (the call then fails) or observes
    /// it already answered.
    fn answer_pending(
        &self,
        id: PermissionId,
        decide: impl FnOnce(
            &PendingPermission,
        ) -> Result<(WireAnswer, PermissionResolution), DriverError>,
    ) -> Result<(), DriverError> {
        let (resolution, failed) = {
            let mut core = self.core.lock();
            if let Some(failure) = &core.failure {
                return Err(DriverError::Failed(failure.clone()));
            }
            let Some(pending) = core.permissions.pending.get(&id.0) else {
                return Err(DriverError::UnknownPermission(id));
            };
            let (answer, resolution) = decide(pending)?;
            let pending = core
                .permissions
                .pending
                .remove(&id.0)
                .expect("checked above");
            core.permissions.wire_ids.remove(&pending.wire_id);
            (resolution, pending.sink.respond(answer).err())
        };
        if let Some(error) = failed {
            self.fail(match error {
                SinkError::Rpc(error) => self.rpc_failure(Phase::Prompt, error),
                SinkError::Failed(failure) => failure,
            });
            return Err(DriverError::Closed);
        }
        self.emit(AgentEventKind::PermissionClosed { id, resolution });
        Ok(())
    }

    /// The adapter withdrew a pending request itself (Claude's `control_cancel_request`):
    /// it is closed as cancelled without an answer. Unknown wire ids are ignored.
    pub(in crate::driver) fn withdraw_pending(&self, wire_id: &str) {
        let id = {
            let mut core = self.core.lock();
            let Some(id) = core
                .permissions
                .pending
                .iter()
                .find(|(_, pending)| pending.wire_id == wire_id)
                .map(|(id, _)| *id)
            else {
                return;
            };
            core.permissions.pending.remove(&id);
            core.permissions.wire_ids.remove(wire_id);
            PermissionId(id)
        };
        self.push_permission_closed(&[id]);
    }

    // ---- session updates -----------------------------------------------------------

    pub(super) fn protocol_violation(&self, message: &str) -> RpcError {
        self.fail(AgentFailure::new(
            FailureKind::ProtocolViolation,
            self.phase(),
            message,
        ));
        RpcError::internal_error().data(message.to_owned())
    }

    pub(super) fn late(&self) -> Result<(), RpcError> {
        self.late.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// The adapter is still working on the turn: its prompt deadline starts over.
    pub(super) fn note_turn_activity(&self) {
        self.watchdog
            .extend(Phase::Prompt, self.limits.prompt_timeout);
    }

    pub(super) fn on_update(&self, notification: SessionNotification) -> Result<(), RpcError> {
        let (matches, unbound, in_turn, restoring, stopping) = {
            let core = self.core.lock();
            (
                core.session.as_deref() == Some(&*notification.session_id.0),
                core.session.is_none(),
                core.turn.is_some(),
                core.restoring,
                core.stopping,
            )
        };
        // Agents push their command list right after `session/new` answers, which can be
        // dispatched before the connection task has stored the new session id. Plans and
        // command lists are display-only, so those two are accepted until the id is known.
        let early_display = unbound
            && matches!(
                notification.update,
                SessionUpdate::Plan(_) | SessionUpdate::AvailableCommandsUpdate(_)
            );
        if !matches && !early_display {
            return Err(self.protocol_violation("session identity mismatch"));
        }
        if stopping {
            return self.late();
        }
        // Only real agent work restarts the turn's idle deadline; command lists, session
        // info, usage and option pushes can arrive forever without the turn progressing.
        let active = matches!(
            notification.update,
            SessionUpdate::AgentMessageChunk(_)
                | SessionUpdate::AgentThoughtChunk(_)
                | SessionUpdate::ToolCall(_)
                | SessionUpdate::ToolCallUpdate(_)
                | SessionUpdate::Plan(_)
        );
        if in_turn && active {
            self.note_turn_activity();
        }
        match notification.update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                self.on_message(MessageRole::Agent, &chunk.content, in_turn, restoring)
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                self.on_message(MessageRole::Thought, &chunk.content, in_turn, restoring)
            }
            SessionUpdate::UserMessageChunk(chunk) => {
                self.on_message(MessageRole::User, &chunk.content, in_turn, restoring)
            }
            SessionUpdate::ToolCall(call) => {
                if restoring {
                    return Ok(());
                }
                if !in_turn {
                    return self.late();
                }
                self.flush_before_event();
                let event = ToolEvent {
                    tool_call_id: self.clean(&call.tool_call_id.0),
                    title: Some(self.clean(&call.title)),
                    kind: Some(enum_label(&call.kind)),
                    status: Some(tool_status(call.status)),
                };
                self.emit(AgentEventKind::ToolCall(event));
                Ok(())
            }
            SessionUpdate::ToolCallUpdate(update) => {
                if restoring {
                    return Ok(());
                }
                if !in_turn {
                    return self.late();
                }
                self.flush_before_event();
                let event = ToolEvent {
                    tool_call_id: self.clean(&update.tool_call_id.0),
                    title: update.fields.title.as_deref().map(|t| self.clean(t)),
                    kind: update.fields.kind.as_ref().map(enum_label),
                    status: update.fields.status.map(tool_status),
                };
                self.emit(AgentEventKind::ToolUpdate(event));
                Ok(())
            }
            SessionUpdate::ConfigOptionUpdate(update) => {
                self.apply_config(&update.config_options);
                let snapshot = self.options.lock().clone();
                self.emit(AgentEventKind::Options(snapshot));
                Ok(())
            }
            SessionUpdate::CurrentModeUpdate(update) => {
                let raw = update.current_mode_id.0.to_string();
                let exported = {
                    let mut ids = self.option_ids.lock();
                    match ids.modes.iter().find(|(_, r)| **r == raw) {
                        Some((exported, _)) => exported.clone(),
                        None => self.export_id(&raw, &mut ids.modes),
                    }
                };
                let snapshot = {
                    let mut options = self.options.lock();
                    options.current_mode = Some(exported);
                    options.clone()
                };
                self.emit(AgentEventKind::Options(snapshot));
                Ok(())
            }
            // The plan is displayed, never acted on. A restored session replays its latest
            // plan, which is kept (it is state, not transcript).
            SessionUpdate::Plan(plan) => {
                let entries = plan
                    .entries
                    .iter()
                    .take(MAX_PLAN_ENTRIES)
                    .map(|entry| PlanEntry {
                        text: self.clean(&entry.content),
                        status: match entry.status {
                            PlanEntryStatus::InProgress => PlanStatus::InProgress,
                            PlanEntryStatus::Completed => PlanStatus::Completed,
                            _ => PlanStatus::Pending,
                        },
                    })
                    .collect();
                self.emit_display_state(restoring, AgentEventKind::Plan(entries));
                Ok(())
            }
            // Commands are offered for the user to type; the agent interprets them.
            SessionUpdate::AvailableCommandsUpdate(update) => {
                let commands = update
                    .available_commands
                    .iter()
                    .take(MAX_COMMANDS)
                    .map(|command| CommandInfo {
                        name: self.clean(command.name.trim_start_matches('/')),
                        description: self.clean(&command.description),
                        input_hint: command.input.as_ref().and_then(|input| match input {
                            AvailableCommandInput::Unstructured(input) => {
                                Some(self.clean(&input.hint))
                            }
                            _ => None,
                        }),
                    })
                    .filter(|command| CommandInfo::is_valid_name(&command.name))
                    .collect();
                self.emit_display_state(restoring, AgentEventKind::AvailableCommands(commands));
                Ok(())
            }
            // Usage and session info carry no workflow authority.
            _ => Ok(()),
        }
    }

    /// Emits a display-only state event (plan, command list). Held text is released first
    /// so the order stays chronological, except while a session is being restored: the
    /// replayed text is dropped, and releasing would only reset the stream state.
    fn emit_display_state(&self, restoring: bool, kind: AgentEventKind) {
        if !restoring {
            self.flush_before_event();
        }
        self.emit(kind);
    }

    pub(super) fn on_message(
        &self,
        role: MessageRole,
        content: &ContentBlock,
        in_turn: bool,
        restoring: bool,
    ) -> Result<(), RpcError> {
        // Replayed history (`session/load`) is dropped: the host's stored conversation is
        // the only source of rows, so a replay can never show a message twice.
        if restoring {
            return Ok(());
        }
        if !in_turn {
            return self.late();
        }
        let text = match content {
            ContentBlock::Text(text) => text.text.as_str(),
            _ => "[non-text content omitted]",
        };
        self.push_text(role, text);
        Ok(())
    }

    /// Streams one text delta of `role` through redaction, keeping roles chronological.
    pub(in crate::driver) fn push_text(&self, role: MessageRole, text: &str) {
        // Text held for another role is released first.
        for other in ROLES {
            if other != role {
                let held = self.streams.lock().get(other).flush_at_boundary();
                self.release_text(other, held);
            }
        }
        let released = self.streams.lock().get(role).push(text);
        self.release_text(role, released);
    }

    pub(super) fn release_text(&self, role: MessageRole, text: String) {
        if text.is_empty() {
            return;
        }
        self.transcript.lock().append(role, &text);
        // Events stay within the delta size cap so 256 queued events bound memory.
        for part in split_chunks(&text, MAX_DELTA_EVENT_BYTES) {
            if !self.emit(AgentEventKind::MessageDelta {
                role,
                text: part.to_owned(),
            }) {
                break;
            }
        }
    }

    /// Releases held text before a non-text event so consumers see chronological order.
    /// A possible secret prefix at the very end stays held until more text arrives.
    pub(super) fn flush_before_event(&self) {
        for role in ROLES {
            let held = self.streams.lock().get(role).flush_at_boundary();
            self.release_text(role, held);
        }
    }

    /// Turn boundary: a message never continues into the next turn, so a held secret
    /// prefix cannot be completed any more and is released with its own turn instead of
    /// leaking into the next one.
    pub(super) fn finish_streams_at_turn_end(&self) {
        for role in ROLES {
            let rest = self.streams.lock().get(role).finish();
            self.release_text(role, rest);
        }
    }

    /// True end of every stream: nothing more can ever complete a held prefix.
    pub(super) fn finish_streams(&self) {
        self.finish_streams_at_turn_end();
    }

    // ---- options -------------------------------------------------------------------

    pub(super) fn apply_modes(&self, modes: &SessionModeState) {
        let mut taken = HashMap::new();
        let list: Vec<ModeOption> = modes
            .available_modes
            .iter()
            .map(|m| ModeOption {
                id: self.export_id(&m.id.0, &mut taken),
                name: self.clean(&m.name),
                description: m.description.as_deref().map(|d| self.clean(d)),
            })
            .collect();
        let current = self.export_id(&modes.current_mode_id.0, &mut taken);
        let mut options = self.options.lock();
        options.current_mode = Some(current);
        options.modes = list;
        self.option_ids.lock().modes = taken;
    }

    pub(super) fn apply_config(&self, config: &[SessionConfigOption]) {
        let mut ids = HashMap::new();
        let mut raws: HashMap<String, RawConfig> = HashMap::new();
        let mut converted = Vec::new();
        for option in config {
            let mut value_ids = HashMap::new();
            let kind = match &option.kind {
                SessionConfigKind::Select(select) => {
                    let mut values = Vec::new();
                    match &select.options {
                        SessionConfigSelectOptions::Ungrouped(list) => {
                            for v in list {
                                values.push(self.config_value(v, None, &mut value_ids));
                            }
                        }
                        SessionConfigSelectOptions::Grouped(groups) => {
                            for group in groups {
                                for v in &group.options {
                                    values.push(self.config_value(
                                        v,
                                        Some(&group.name),
                                        &mut value_ids,
                                    ));
                                }
                            }
                        }
                        _ => continue,
                    }
                    let current = {
                        let raw = select.current_value.0.to_string();
                        self.export_id(&raw, &mut value_ids)
                    };
                    ConfigKind::Select { current, values }
                }
                SessionConfigKind::Boolean(b) => ConfigKind::Boolean {
                    current: b.current_value,
                },
                _ => continue,
            };
            let raw_id = option.id.0.to_string();
            let exported = self.export_id(&raw_id, &mut ids);
            raws.insert(
                exported.clone(),
                RawConfig {
                    raw_id,
                    values: value_ids,
                },
            );
            converted.push(ConfigOption {
                id: exported,
                name: self.clean(&option.name),
                description: option.description.as_deref().map(|d| self.clean(d)),
                kind,
            });
        }
        self.options.lock().config = converted;
        self.option_ids.lock().config = raws;
    }

    pub(super) fn config_value(
        &self,
        value: &agent_client_protocol::schema::v1::SessionConfigSelectOption,
        group: Option<&str>,
        ids: &mut HashMap<String, String>,
    ) -> ConfigValue {
        ConfigValue {
            value: self.export_id(&value.value.0, ids),
            name: self.clean(&value.name),
            group: group.map(|g| self.clean(g)),
        }
    }

    /// Validates an exported option/value against what the adapter advertised and maps
    /// it back to the wire ids.
    pub(super) fn resolve_option(
        &self,
        id: &str,
        value: &OptionValue,
    ) -> Result<(String, OptionValue), DriverError> {
        let options = self.options.lock();
        let ids = self.option_ids.lock();
        let (Some(option), Some(raw)) = (
            options.config.iter().find(|o| o.id == id),
            ids.config.get(id),
        ) else {
            return Err(DriverError::InvalidOption(format!(
                "config option '{id}' was not advertised by the adapter"
            )));
        };
        let invalid = || {
            DriverError::InvalidOption(format!("value is not advertised for config option '{id}'"))
        };
        match (&option.kind, value) {
            (ConfigKind::Select { values, .. }, OptionValue::Value(v))
                if values.iter().any(|candidate| &candidate.value == v) =>
            {
                let wire = raw.values.get(v).ok_or_else(invalid)?;
                Ok((raw.raw_id.clone(), OptionValue::Value(wire.clone())))
            }
            (ConfigKind::Boolean { .. }, OptionValue::Boolean(b)) => {
                Ok((raw.raw_id.clone(), OptionValue::Boolean(*b)))
            }
            _ => Err(invalid()),
        }
    }

    pub(super) fn resolve_mode(&self, mode: &str) -> Result<String, DriverError> {
        if !self.options.lock().modes.iter().any(|m| m.id == mode) {
            return Err(DriverError::InvalidOption(format!(
                "mode '{mode}' was not advertised by the adapter"
            )));
        }
        self.option_ids
            .lock()
            .modes
            .get(mode)
            .cloned()
            .ok_or_else(|| DriverError::InvalidOption(format!("mode '{mode}' is unknown")))
    }

    pub(super) fn on_option_result(
        &self,
        label: &str,
        key: u64,
        result: Result<Option<Vec<SessionConfigOption>>, RpcError>,
        is_mode: bool,
    ) {
        self.watchdog.untrack(key);
        match result {
            Ok(config) => {
                if let Some(config) = config {
                    self.apply_config(&config);
                } else if is_mode {
                    self.options.lock().current_mode = Some(label.to_owned());
                }
                let snapshot = self.options.lock().clone();
                self.emit(AgentEventKind::Options(snapshot));
            }
            Err(error) => {
                let code: i32 = error.code.into();
                self.emit(AgentEventKind::OptionRejected {
                    option: truncate_label(label),
                    message: self.clean(&error.message),
                    code: i64::from(code),
                });
            }
        }
    }

    // ---- prompt turns --------------------------------------------------------------

    /// Latches cancellation of the active turn at the call boundary: from this instant
    /// the turn can never complete as uncancelled, and its permissions are closed. The
    /// turn-bound notification is written later by the connection task. Returns the turn
    /// id the first time; repeated calls coalesce to `None`.
    pub(super) fn latch_cancel(&self, for_stop: bool) -> Result<Option<u64>, DriverError> {
        let (turn_id, first, closed) = {
            let mut core = self.core.lock();
            if !for_stop {
                if let Some(failure) = &core.failure {
                    return Err(DriverError::Failed(failure.clone()));
                }
                match core.status {
                    DriverStatus::Starting => return Err(DriverError::NotReady),
                    DriverStatus::Stopping | DriverStatus::Closed => {
                        return Err(DriverError::Closed);
                    }
                    DriverStatus::Ready | DriverStatus::Prompting => {}
                }
            }
            let Some(turn) = core.turn.as_mut() else {
                return if for_stop {
                    Ok(None)
                } else {
                    Err(DriverError::NoActiveTurn)
                };
            };
            let first = !turn.cancel_requested;
            turn.cancel_requested = true;
            let turn_id = turn.id;
            if first {
                core.cancel_to_send = Some(turn_id);
            }
            (turn_id, first, core.drain_permissions())
        };
        self.push_permission_closed(&closed);
        if first {
            // Repeated stops never extend the adapter's time to acknowledge.
            self.watchdog
                .arm(Phase::CancelAck, self.limits.cancel_ack_timeout);
        }
        Ok(first.then_some(turn_id))
    }

    /// The turn whose cancel notification is due, if it is still the active turn.
    pub(super) fn take_cancel_to_send(&self) -> Option<String> {
        let mut core = self.core.lock();
        let turn = core.cancel_to_send.take()?;
        if core.turn.as_ref().map(|t| t.id) == Some(turn) {
            core.session.clone()
        } else {
            None
        }
    }

    pub(super) fn on_prompt_result(&self, turn_id: u64, result: Result<PromptResponse, RpcError>) {
        let (response, reported_incomplete) = match result {
            Ok(response) => (response.stop_reason, false),
            // The turn is over and the session usable; the adapter only reports that
            // server-side tools never got a result. Kept distinguishable from a clean end.
            Err(error)
                if self.claude_adapter.load(Ordering::SeqCst)
                    && only_server_tools_unfinished(&error) =>
            {
                (StopReason::EndTurn, true)
            }
            Err(error) => {
                self.fail(self.rpc_failure(Phase::Prompt, error));
                return;
            }
        };
        let stop_reason = match response {
            StopReason::EndTurn => StopReasonKind::EndTurn,
            StopReason::MaxTokens => StopReasonKind::MaxTokens,
            StopReason::MaxTurnRequests => StopReasonKind::MaxTurnRequests,
            StopReason::Refusal => StopReasonKind::Refusal,
            StopReason::Cancelled => StopReasonKind::Cancelled,
            _ => {
                self.fail(AgentFailure::new(
                    FailureKind::ProtocolViolation,
                    Phase::Prompt,
                    "invalid ACP stop reason",
                ));
                return;
            }
        };
        self.finish_turn(
            turn_id,
            stop_reason,
            reported_incomplete,
            TurnOrigin::Prompted,
        );
    }

    /// Ends `turn_id` with its one completion event. A turn that is not the active one is a
    /// protocol violation. `reported_incomplete` turns carry no authoritative stop reason.
    pub(in crate::driver) fn finish_turn(
        &self,
        turn_id: u64,
        stop_reason: StopReasonKind,
        reported_incomplete: bool,
        origin: TurnOrigin,
    ) {
        // One critical section decides the cancel latch, the unresolved permissions and
        // that this turn is over; a concurrent Stop lands entirely before or after it.
        let taken = {
            let mut core = self.core.lock();
            match core.turn.take() {
                Some(turn) if turn.id == turn_id => {
                    let closed = core.drain_permissions();
                    Ok((turn, closed))
                }
                other => {
                    core.turn = other;
                    Err(())
                }
            }
        };
        let Ok((turn, closed)) = taken else {
            self.fail(AgentFailure::new(
                FailureKind::ProtocolViolation,
                Phase::Prompt,
                "prompt response does not match the active turn",
            ));
            return;
        };
        self.push_permission_closed(&closed);
        // Held-back text belongs to this turn; release it before the completion event.
        self.finish_streams_at_turn_end();
        self.watchdog.disarm();
        let scan = self.child.lock().escaped_descendants();
        self.note_scan(scan);
        let outcome = PromptOutcome {
            turn: turn.id,
            stop_reason,
            // An error response carries no stop reason of its own.
            authoritative: !reported_incomplete,
            cancel_requested: turn.cancel_requested,
            unresolved_permissions: closed.len() as u32,
            reported_incomplete,
            origin,
        };
        // The completion event goes in first so a waiter that sees the outcome also sees
        // the event. If it cannot be queued a failure is already latched: no success.
        if !self.emit(AgentEventKind::PromptFinished(outcome.clone())) {
            return;
        }
        let mut core = self.core.lock();
        if core.failure.is_some() {
            // Failure dominates completion.
            return;
        }
        core.last_outcome = Some(outcome);
        if core.status == DriverStatus::Prompting {
            core.status = DriverStatus::Ready;
        }
        core.phase = Phase::Idle;
        self.core_changed.notify_all();
    }

    // ---- failures ------------------------------------------------------------------

    pub(super) fn rpc_failure(&self, phase: Phase, error: RpcError) -> AgentFailure {
        if agent_client_protocol::is_incoming_transport_closed(&error)
            || error
                .message
                .to_ascii_lowercase()
                .contains("transport closed")
        {
            return self.process_exit_failure();
        }
        let code: i32 = error.code.into();
        let kind = if code == i32::from(agent_client_protocol::ErrorCode::AuthRequired) {
            FailureKind::AuthRequired
        } else if code == i32::from(agent_client_protocol::ErrorCode::ParseError) {
            // The SDK could not parse the adapter's response as ACP v1.
            FailureKind::ProtocolViolation
        } else {
            match phase {
                Phase::Authenticate => FailureKind::AuthRejected,
                Phase::Session => FailureKind::SessionRejected,
                Phase::Prompt | Phase::CancelAck => FailureKind::PromptRejected,
                _ => FailureKind::ProtocolViolation,
            }
        };
        let mut failure = AgentFailure::new(kind, phase, error.message);
        failure.code = Some(i64::from(code));
        failure
    }

    pub(super) fn exit_code(&self, within: Duration) -> Option<i32> {
        let deadline = Instant::now() + within;
        loop {
            if let Ok(Some(status)) = self.child.lock().try_wait() {
                return status.code();
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub(super) fn process_exit_failure(&self) -> AgentFailure {
        let phase = self.phase();
        let code = self.exit_code(Duration::from_millis(1000));
        let (kind, message) = if phase == Phase::Initialize && code == Some(127) {
            (
                FailureKind::MissingRuntime,
                "adapter runtime not found: the launcher exited with status 127".to_owned(),
            )
        } else {
            (
                FailureKind::ProcessExited,
                match code {
                    Some(code) => format!("adapter process exited with status {code}"),
                    None => "adapter process exited".to_owned(),
                },
            )
        };
        let mut failure = AgentFailure::new(kind, phase, message);
        failure.exit_code = code;
        failure
    }

    // ---- lifecycle -----------------------------------------------------------------

    pub(super) fn set_ready(&self) {
        let mut core = self.core.lock();
        if core.status == DriverStatus::Starting {
            core.status = DriverStatus::Ready;
        }
        core.phase = Phase::Idle;
        self.core_changed.notify_all();
    }

    /// Verified teardown of the owned tree; safe to call repeatedly.
    pub(super) fn teardown(&self) -> TerminationReport {
        let mut child = self.child.lock();
        self.note_scan(child.escaped_descendants());
        let mut report = child
            .terminate_verified(self.limits.shutdown_grace)
            .unwrap_or_else(|error: ProcessError| {
                // Termination that could not be verified is containment that cannot be.
                self.note_unverified(&format!("termination could not be verified: {error}"));
                TerminationReport {
                    forced: true,
                    direct_child_exited: false,
                    group_empty: false,
                    remaining: Vec::new(),
                }
            });
        if self.forced_kill.load(Ordering::SeqCst) {
            report.forced = true;
        }
        report
    }

    /// Builds the final facts from one consistent snapshot of the core (a single lock
    /// acquisition) and applies the sticky ownership demotion.
    pub(super) fn snapshot_outcome(
        &self,
        termination: TerminationReport,
        exit_code: Option<i32>,
    ) -> DriverOutcome {
        let (escaped, containment_unverified, last_prompt, failure) = {
            let core = self.core.lock();
            (
                core.escaped.clone(),
                core.containment_unverified.clone(),
                core.last_outcome.clone(),
                core.failure.clone(),
            )
        };
        let ownership = if escaped.is_empty() && containment_unverified.is_none() {
            self.ownership.clone()
        } else {
            WriterOwnership::Detached
        };
        DriverOutcome {
            task: self.queue.task().to_owned(),
            session: self.queue.session(),
            termination,
            ownership,
            escaped_pids: escaped,
            containment_unverified,
            last_prompt,
            failure,
            late_events_rejected: self.late.load(Ordering::SeqCst),
            exit_code,
        }
    }

    /// Publishes the final outcome exactly once and queues the terminal event.
    pub(super) fn publish_closed(&self, outcome: DriverOutcome) -> DriverOutcome {
        let published = {
            let mut core = self.core.lock();
            if let Some(existing) = &core.outcome {
                return existing.clone();
            }
            core.status = DriverStatus::Closed;
            core.turn = None;
            core.outcome = Some(outcome.clone());
            self.core_changed.notify_all();
            outcome
        };
        let _ = self.queue.push(AgentEventKind::Closed(ClosedInfo {
            late_events_rejected: published.late_events_rejected,
            clean: published.failure.is_none(),
        }));
        published
    }

    pub(super) fn finish(&self) {
        self.watchdog.stop();
        self.ingress.close();
        self.close_permissions();
        self.finish_streams();
        let termination = self.teardown();
        self.stderr_finish();
        let exit_code = self.exit_code(Duration::from_millis(200));
        let outcome = self.snapshot_outcome(termination, exit_code);
        self.publish_closed(outcome);
    }
}

pub(super) fn enum_label<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "other".to_owned())
}

pub(super) fn tool_status(status: ToolCallStatus) -> ToolStatus {
    match status {
        ToolCallStatus::Pending => ToolStatus::Pending,
        ToolCallStatus::InProgress => ToolStatus::InProgress,
        ToolCallStatus::Completed => ToolStatus::Completed,
        _ => ToolStatus::Failed,
    }
}
