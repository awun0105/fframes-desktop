//! The Setup tab: providers, the adapter description and writer containment.

use super::*;

impl ConversationPanel {
    pub(super) fn setup_tab(
        &mut self,
        snapshot: Arc<WorkflowSnapshot>,
        controls: &Controls,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let configured = snapshot.adapter.provider.is_some();
        let g = guidance(&snapshot.adapter.readiness, configured);
        let mut column = div()
            .id("agent-setup-tab")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(12.));
        column = column.child(
            self.section(&g.headline)
                .children(g.steps.iter().map(|s| self.line(s.clone()))),
        );
        if let Some(error) = &self.settings_error {
            column = column.child(Notice::new(
                "agent-settings-error",
                UiTone::Danger,
                error.clone(),
            ));
        }
        let switch_pending = snapshot.switch_pending.is_some();
        let selection_busy = snapshot
            .task
            .as_ref()
            .is_some_and(|task| !task.phase.is_terminal())
            || matches!(&snapshot.undo, UndoView::InProgress { .. });
        column = column.child(self.providers_section(switch_pending, selection_busy, cx));
        column = column.child(self.detect_section(
            &snapshot.adapter.readiness,
            controls.settings_editable,
            cx,
        ));

        if let Some(choice) = &controls.switch_choice {
            let mut handoff = self
                .section("Stopped-writer handoff")
                .child(self.line(choice.note.clone()))
                .child(self.line(format!(
                    "Outgoing task: {}",
                    snapshot
                        .task
                        .as_ref()
                        .map(|task| task.brief.as_str())
                        .unwrap_or("no active task")
                )))
                .child(self.line(format!(
                    "Draft revision: {} · current source: {} · accepted: {}",
                    choice.draft_revision.as_deref().unwrap_or("unavailable"),
                    choice.source_revision,
                    choice.accepted_revision
                )));
            if choice.retained_queue_count > 0 {
                handoff = handoff.child(self.line(format!(
                    "{} queued brief(s) remain with the outgoing provider until you explicitly transfer or clear them.",
                    choice.retained_queue_count
                )));
            }
            handoff = handoff.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(self.action_button(
                        "agent-handoff-continue-draft",
                        "Continue captured draft",
                        true,
                        Tone::Primary,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                this.command(
                                    workflow.choose_handoff_continuation(
                                        studio_engine::agent::model::HandoffChoice::ContinueDraft,
                                    ),
                                    cx,
                                );
                            }
                        },
                    ))
                    .child(self.action_button(
                        "agent-handoff-restart-accepted",
                        "Restart from accepted",
                        true,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                this.command(
                                    workflow.choose_handoff_continuation(
                                        studio_engine::agent::model::HandoffChoice::RestartFromAccepted,
                                    ),
                                    cx,
                                );
                            }
                        },
                    ))
                    .child(self.action_button(
                        "agent-handoff-cancel",
                        "Cancel switch",
                        true,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                this.command(workflow.cancel_switch(), cx);
                            }
                        },
                    )),
            );
            column = column.child(handoff);
        }
        if controls.retained_queue.is_some() {
            let mut retained = self.section("Queued work held for the previous provider");
            for brief in &snapshot.retained_queue {
                retained = retained.child(self.line(format!("• {}", brief.summary)));
            }
            retained = retained.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(self.action_button(
                        "agent-transfer-retained-queue",
                        "Explicitly transfer queued briefs",
                        !selection_busy && !switch_pending,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                this.command(workflow.transfer_retained_queue(), cx);
                            }
                        },
                    ))
                    .child(self.action_button(
                        "agent-clear-retained-queue",
                        "Clear held briefs",
                        !selection_busy && !switch_pending,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                this.command(workflow.clear_retained_queue(), cx);
                            }
                        },
                    )),
            );
            column = column.child(retained);
        }
        if let Some(restore) = &controls.session_restore {
            let mut session = self
                .section("Previous provider session")
                .child(self.line(format!(
                    "{} · session {} · {}",
                    restore.provider_id,
                    restore.redacted_session_id,
                    restore
                        .notice
                        .as_deref()
                        .unwrap_or("Native conversation memory stays with its provider.")
                )));
            if restore.is_resumable {
                session = session.child(self.action_button(
                    "agent-restore-session",
                    "Use restored session for next task",
                    !selection_busy && !switch_pending,
                    Tone::Normal,
                    cx,
                    |this, _, cx| {
                        if let Some(workflow) = this.workflow().cloned() {
                            this.command(workflow.restore_session(), cx);
                        }
                    },
                ));
            }
            session = session.child(self.action_button(
                "agent-dismiss-session",
                "Dismiss saved session",
                !selection_busy && !switch_pending,
                Tone::Normal,
                cx,
                |this, _, cx| {
                    if let Some(workflow) = this.workflow().cloned() {
                        this.command(workflow.dismiss_restoration(), cx);
                    }
                },
            ));
            column = column.child(session);
        }
        let field = |this: &Self,
                     label: &'static str,
                     input: &Entity<TextInputState>,
                     cx: &mut Context<Self>| {
            // The handler reads THIS field's composition state: an active IME composition
            // keeps Enter/Tab/Escape (candidate selection and cancellation) in the field.
            let field = input.clone();
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(self.line(label))
                .child(
                    div()
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, window, cx| {
                                this.key_in_field(&field, event, false, window, cx);
                            },
                        ))
                        .child(TextInput::plain(
                            &this.theme,
                            SharedString::from(format!("setup-{label}")),
                            input,
                        )),
                )
        };
        let editable = controls.settings_editable;
        let mut form = self.section("Adapter");
        form = form
            .child(field(self, "Label", &self.setup.provider.clone(), cx))
            .child(field(
                self,
                "Executable",
                &self.setup.executable.clone(),
                cx,
            ))
            .child(field(self, "Arguments", &self.setup.args.clone(), cx))
            .child(field(
                self,
                "Sign-in variable names (values are never stored)",
                &self.setup.env_names.clone(),
                cx,
            ));
        form = form
            .child(self.line(
                "Writer containment is not a setting: it is derived from the qualification ledger installed in the app data folder, and only for this exact adapter and platform.",
            ))
            .child(self.line(
                self.containment.as_ref().map_or_else(
                    || "Writer containment: no adapter configured.".to_owned(),
                    Resolution::summary,
                ),
            ))
            .child({
                let entity = cx.entity();
                let label = match self.mcp_choice {
                    McpChoice::Baseline => "Project tools: MCP and command line",
                    McpChoice::Unsupported => "Project tools: command line only",
                };
                field_row(&self.theme,
                    label,
                    Toggle::plain(&self.theme, "agent-mcp-toggle", self.mcp_choice == McpChoice::Baseline)
                        .label(label)
                        .disabled(&self.theme, !editable)
                        .on_change(move |_, _, app| {
                            entity.update(app, |this, cx| {
                                this.mcp_choice = match this.mcp_choice {
                                    McpChoice::Baseline => McpChoice::Unsupported,
                                    McpChoice::Unsupported => McpChoice::Baseline,
                                };
                                cx.notify();
                            });
                        }),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(self.action_button(
                        "agent-save-adapter",
                        "Save",
                        editable && !self.saves.is_pending(),
                        Tone::Primary,
                        cx,
                        |this, _, cx| this.save_adapter(cx),
                    ))
                    .child(self.action_button(
                        "agent-check-adapter",
                        "Check adapter",
                        controls.check_adapter,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(w) = this.workflow().cloned() {
                                let r = w.check_adapter();
                                this.command(r, cx);
                            }
                        },
                    )),
            );
        if !editable {
            form = form.child(
                self.line("Settings can change after the running task finishes or is stopped."),
            );
        }
        column = column.child(form);

        let mut status = self.section("Status");
        status = status
            .child(self.line(format!(
                "Adapter: {}",
                snapshot
                    .adapter
                    .executable
                    .clone()
                    .unwrap_or_else(|| "not configured".into())
            )))
            .child(self.line(format!(
                "Writer process model: {}",
                snapshot
                    .adapter
                    .writer_ownership
                    .clone()
                    .unwrap_or_else(|| "n/a".into())
            )));
        if snapshot
            .adapter
            .writer_ownership
            .as_deref()
            .is_some_and(|label| label != "process-group-contained")
        {
            status = status.child(self.line(
                "This adapter is not qualified for this exact launch and platform, so its candidates cannot be captured or applied until a passed writer-containment qualification of it is installed.",
            ));
        }
        if let AdapterReadiness::Checked { report, .. } = &snapshot.adapter.readiness
            && let Some(info) = &report.initialized
        {
            status = status.child(self.line(format!(
                "Adapter reports {} {} · ACP v{}",
                info.agent_name, info.agent_version, info.protocol_version
            )));
        }
        if let Some(caps) = &snapshot.capabilities {
            let mut unsupported = Vec::new();
            if !caps.prompt_image {
                unsupported.push("images");
            }
            if !caps.prompt_audio {
                unsupported.push("audio");
            }
            if !caps.load_session && !caps.resume_session {
                unsupported.push("resuming a session");
            }
            if !unsupported.is_empty() {
                status = status.child(self.line(format!(
                    "Not supported by this agent (prompts are text only): {}",
                    unsupported.join(", ")
                )));
            }
        }
        let mcp = &snapshot.mcp;
        status = status.child(self.line(format!(
            "Project tools: MCP {} · command line {}{}",
            if mcp.policy_enabled && mcp.binary_available {
                "offered"
            } else {
                "not offered"
            },
            if mcp.cli_active {
                "offered"
            } else {
                "when a task runs"
            },
            mcp.note
                .as_ref()
                .map(|n| format!(" · {n}"))
                .unwrap_or_default()
        )));
        column = column.child(status);
        column.into_any_element()
    }
}
