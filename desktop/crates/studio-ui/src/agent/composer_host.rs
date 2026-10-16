//! The input side of the agent panel: the composer, the plan list and the session list,
//! and everything that moves between them and the workflow.
//!
//! The components own what the user is typing and nothing else. This module pushes the
//! snapshot's composer state into them ([`ConversationPanel::sync_composer`]), turns their
//! events into workflow calls and puts a refused message back, so a refusal never costs
//! the user their text or chips. Attachments are prepared in [`super::attachment_flow`],
//! agent questions answered in [`super::question_card`], sessions listed and opened in
//! [`super::history_host`].

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use gpui::{App, Focusable, KeyDownEvent};
use studio_engine::agent::{
    CommandInfo, OpenQuestion, PreparedAttachment, StagedAttachment, SteerSupport, WorkflowError,
};

use super::composer::{
    AttachKind, AttachmentId, Composer, ComposerEvent, SlashCommand, SteerMode, Submission,
};
use super::history::{HistoryEvent, HistoryList};
use super::question_card::ItemDraft;
use super::todo_panel::{TodoItem, TodoPanel};
use super::*;
use crate::probe::BoundsSink;

/// Why image chips are off while the agent has not said it takes images.
pub(super) const IMAGES_OFF: &str =
    "This agent has not advertised image prompts (check the agent in setup).";
/// Why file chips are off while the agent takes no files.
pub(super) const FILES_OFF: &str = "This agent cannot take file attachments.";

/// An outside file the engine checked and the user has not confirmed yet.
pub(super) struct Staged {
    pub staged: StagedAttachment,
    /// When the engine issued the token; it expires, so an old one is staged again.
    pub at: Instant,
}

/// The entities and the bookkeeping the composer side of the panel keeps.
pub(super) struct ComposerHost {
    pub composer: Entity<Composer>,
    pub todo: Entity<TodoPanel>,
    pub history: Entity<HistoryList>,
    /// What the engine accepted for each chip. A send takes copies, so a refused send
    /// still has them when the chips come back.
    pub prepared: HashMap<AttachmentId, PreparedAttachment>,
    /// Counts conversations; an attachment answer from an older one is dropped.
    pub generation: u64,
    /// Attachment checks in flight.
    pub preparing: usize,
    pub stage: VecDeque<Staged>,
    /// The last `unsent` the engine showed, so only a change acts.
    pub last_unsent: Option<String>,
    /// A message the agent refused while the composer held something new; the user
    /// decides what to do with it.
    pub refused: Option<String>,
    pub last_commands: Vec<CommandInfo>,
    /// The engine's last reason for refusing an attachment.
    pub last_refusal: Option<String>,
    pub questions: HashMap<studio_engine::agent::PermissionRef, Vec<ItemDraft>>,
    /// Counts session list requests; only the newest answer is shown.
    pub sessions_serial: u64,
}

impl ComposerHost {
    pub(super) fn new(
        window: &mut Window,
        theme: Theme,
        sink: BoundsSink,
        cx: &mut Context<ConversationPanel>,
    ) -> Self {
        let composer = cx.new(|cx| {
            let mut composer = Composer::new("Describe the change you want", theme, cx);
            composer.set_bounds_sink(sink.clone());
            composer
        });
        cx.subscribe(&composer, |panel, _, event: &ComposerEvent, cx| {
            panel.on_composer_event(event.clone(), cx)
        })
        .detach();
        let history = cx.new(|cx| {
            let mut history = HistoryList::new(window, cx);
            history.set_bounds_sink(sink);
            history
        });
        cx.subscribe_in(
            &history,
            window,
            |panel, _, event: &HistoryEvent, window, cx| {
                panel.on_history_event(event.clone(), window, cx)
            },
        )
        .detach();
        Self {
            composer,
            todo: cx.new(|_| TodoPanel::new()),
            history,
            prepared: HashMap::new(),
            generation: 0,
            preparing: 0,
            stage: VecDeque::new(),
            last_unsent: None,
            refused: None,
            last_commands: Vec::new(),
            last_refusal: None,
            questions: HashMap::new(),
            sessions_serial: 0,
        }
    }
}

impl ConversationPanel {
    pub(super) fn prompt_is_empty(&self, cx: &App) -> bool {
        self.host.composer.read(cx).text(cx).trim().is_empty()
    }

    /// Moves the keyboard focus into the composer.
    pub(super) fn focus_composer(&self, window: &mut Window, cx: &mut App) {
        let handle = self.host.composer.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
    }

    /// Whether the composer or a question's text field holds the keyboard focus.
    pub(super) fn composer_has_focus(&self, window: &Window, cx: &App) -> bool {
        self.host
            .composer
            .read(cx)
            .focus_handle(cx)
            .is_focused(window)
            || self
                .host
                .questions
                .values()
                .flatten()
                .any(|draft| draft.text.focus_handle(cx).is_focused(window))
    }

    // ---- state from the snapshot -------------------------------------------------------------

    /// Pushes the snapshot's composer state into the components. Cheap and idempotent: each
    /// setter does nothing when the value did not change.
    pub(super) fn sync_composer(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        let view = &snapshot.composer;
        if self.host.last_commands != view.commands {
            self.host.last_commands = view.commands.clone();
            let commands = view
                .commands
                .iter()
                .map(|c| SlashCommand::new(c.name.clone(), c.description.clone()))
                .collect();
            self.host
                .composer
                .update(cx, |composer, cx| composer.set_commands(commands, cx));
        }
        let mode = match view.steer {
            SteerSupport::None => SteerMode::None,
            SteerSupport::AtBoundary => SteerMode::AtBoundary,
            SteerSupport::Native => SteerMode::Native,
        };
        let off = |off: bool, why: &'static str| off.then(|| SharedString::from(why));
        self.host.composer.update(cx, |composer, cx| {
            composer.set_running(view.turn_live, cx);
            composer.set_steer_mode(mode, cx);
            composer.set_attach_disabled(AttachKind::Image, off(!view.images, IMAGES_OFF), cx);
            composer.set_attach_disabled(AttachKind::File, off(!view.files, FILES_OFF), cx);
        });
        let items: Vec<TodoItem> = view.plan.iter().cloned().map(TodoItem::from).collect();
        let chat = snapshot.chat.to_string();
        self.host.todo.update(cx, |todo, cx| {
            todo.set_plan(chat, items, cx);
            todo.set_live(view.turn_live, cx);
        });
        if self.host.last_unsent != view.unsent {
            self.host.last_unsent = view.unsent.clone();
            if let Some(text) = view.unsent.clone() {
                if self.prompt_is_empty(cx) {
                    self.host
                        .composer
                        .update(cx, |composer, cx| composer.set_text(text, cx));
                } else {
                    // The user has typed something new since: never overwrite it.
                    self.host.refused = Some(text);
                }
            }
        }
    }

    /// A different conversation, or none: nothing prepared for the old one may travel.
    pub(super) fn reset_composer_state(&mut self, cx: &mut Context<Self>) {
        let host = &mut self.host;
        host.generation += 1;
        host.preparing = 0;
        host.prepared.clear();
        host.stage.clear();
        host.last_unsent = None;
        host.refused = None;
        host.last_refusal = None;
        host.questions.clear();
        host.composer
            .update(cx, |composer, cx| composer.clear_attachments(cx));
    }

    /// The composer side of [`Self::detach`].
    pub(super) fn clear_composer(&mut self, cx: &mut Context<Self>) {
        self.reset_composer_state(cx);
        self.host.last_commands.clear();
        self.host
            .composer
            .update(cx, |composer, cx| composer.set_commands(Vec::new(), cx));
        self.host
            .todo
            .update(cx, |todo, cx| todo.set_plan("", Vec::new(), cx));
    }

    // ---- sending -----------------------------------------------------------------------------

    pub(super) fn on_composer_event(&mut self, event: ComposerEvent, cx: &mut Context<Self>) {
        match event {
            ComposerEvent::Submit(sub) => self.send_submission(sub, cx),
            ComposerEvent::SendAtBoundary(sub) => self.send_follow_up(sub, false, cx),
            ComposerEvent::SendNow(sub) => self.send_follow_up(sub, true, cx),
            ComposerEvent::Steer(sub) => self.steer_with(sub, cx),
            ComposerEvent::Stop => self.stop(cx),
            ComposerEvent::AttachRequested(request) => self.attach_requested(request, cx),
            ComposerEvent::AttachmentRemoved(id) => {
                self.host.prepared.remove(&id);
            }
            ComposerEvent::AttachBlocked { reason, .. } => {
                self.host.last_refusal = Some(reason.to_string());
                self.say(reason.to_string(), cx);
            }
        }
    }

    /// The Enter gesture from outside the composer (the shell's submit action). An empty
    /// draft says why nothing was sent.
    pub fn submit(&mut self, cx: &mut Context<Self>) {
        let (Some(_), Some(snapshot)) = (&self.attached, &self.snapshot) else {
            return;
        };
        if !self.host.composer.read(cx).has_draft(cx) {
            let controls = derive_controls(snapshot, true, self.sdk_ready);
            if let Some(reason) = controls.submit.blocked {
                self.say(reason, cx);
            }
            return;
        }
        self.host
            .composer
            .update(cx, |composer, cx| composer.send_primary(cx));
    }

    /// Puts a refused submission back and says why.
    fn refuse(&mut self, sub: Submission, reason: impl Into<String>, cx: &mut Context<Self>) {
        self.host
            .composer
            .update(cx, |composer, cx| composer.restore(sub, cx));
        self.say(reason, cx);
    }

    /// Copies of what the engine accepted for each chip of `sub`.
    fn prepared_for(&self, sub: &Submission) -> Vec<PreparedAttachment> {
        sub.attachments
            .iter()
            .filter_map(|chip| self.host.prepared.get(&chip.id).cloned())
            .collect()
    }

    /// The send went through: the chips' prepared copies are no longer needed.
    fn sent(&mut self, sub: &Submission, cx: &mut Context<Self>) {
        for chip in &sub.attachments {
            self.host.prepared.remove(&chip.id);
        }
        self.notice = None;
        cx.notify();
    }

    fn send_submission(&mut self, sub: Submission, cx: &mut Context<Self>) {
        let (Some(workflow), Some(snapshot)) = (self.workflow().cloned(), self.snapshot.clone())
        else {
            return self.refuse(sub, "No project is open.", cx);
        };
        let controls = derive_controls(&snapshot, sub.text.trim().is_empty(), self.sdk_ready);
        if let Some(reason) = controls.submit.blocked {
            return self.refuse(sub, reason, cx);
        }
        if let Some(error) = self.scope_error.clone() {
            let reason = format!("The selected timeline scope is unavailable: {error}");
            return self.refuse(sub, reason, cx);
        }
        let attachments = self.prepared_for(&sub);
        let scope = self.task_scope.clone();
        let sent = match &controls.submit.kind {
            SubmitKind::Clarify(_) if !attachments.is_empty() => Err(WorkflowError::Refused(
                "Attachments cannot go with the answer to the agent's question.".into(),
            )),
            SubmitKind::Clarify(task) => workflow.reply_clarification(task, &sub.text),
            SubmitKind::Brief | SubmitKind::Queue if attachments.is_empty() => match scope {
                Some(scope) => workflow.submit_scoped(&sub.text, scope).map(|_| ()),
                None => workflow.submit(&sub.text).map(|_| ()),
            },
            SubmitKind::Brief | SubmitKind::Queue => workflow
                .submit_with_attachments(&sub.text, scope, attachments)
                .map(|_| ()),
        };
        match sent {
            Ok(()) => self.sent(&sub, cx),
            Err(error) => self.refuse(sub, error.to_string(), cx),
        }
    }

    fn steer_with(&mut self, sub: Submission, cx: &mut Context<Self>) {
        let Some(workflow) = self.workflow().cloned() else {
            return self.refuse(sub, "No project is open.", cx);
        };
        if !sub.attachments.is_empty() {
            let reason = "Steering takes text only; use Send now or Send at boundary to include attachments.";
            return self.refuse(sub, reason, cx);
        }
        match workflow.steer(&sub.text) {
            Ok(()) => self.sent(&sub, cx),
            Err(error) => self.refuse(sub, error.to_string(), cx),
        }
    }

    /// Send at the next boundary, or `now`: cancel the turn first.
    fn send_follow_up(&mut self, sub: Submission, now: bool, cx: &mut Context<Self>) {
        let Some(workflow) = self.workflow().cloned() else {
            return self.refuse(sub, "No project is open.", cx);
        };
        let attachments = self.prepared_for(&sub);
        let sent = if now {
            workflow.send_now(&sub.text, attachments)
        } else {
            workflow.send_at_boundary(&sub.text, attachments)
        };
        match sent {
            Ok(()) => self.sent(&sub, cx),
            Err(error) => self.refuse(sub, error.to_string(), cx),
        }
    }

    // ---- what the panel shows above the composer ---------------------------------------------

    /// The bottom of the chat tab: notes and cards the composer's state asks for, the plan
    /// and the composer itself.
    pub(super) fn composer_dock(
        &mut self,
        snapshot: &WorkflowSnapshot,
        controls: &Controls,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let mut dock = div()
            .flex()
            .flex_col()
            .flex_shrink(1.)
            .min_h_0()
            .gap(px(6.))
            .px(px(12.))
            .pt(px(8.))
            .pb(px(12.));
        if let Some(restore) = controls
            .session_restore
            .as_ref()
            .filter(|r| !r.is_resumable)
        {
            dock = dock.child(
                Notice::new("agent-readonly-session", UiTone::Info, "Read-only conversation")
                    .body(restore.notice.clone().unwrap_or_else(|| {
                        "The provider cannot resume this session; its stored messages are shown as they were."
                            .to_owned()
                    })),
            );
        }
        for (index, question) in snapshot.composer.questions.iter().enumerate() {
            // In a window too short for the whole card it takes the leftover height and
            // scrolls, instead of pushing the composer out of view.
            dock = dock.child(
                div()
                    .id(("question-scroll", index))
                    .flex_shrink(1.)
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(self.question_card(index, question, cx)),
            );
        }
        if let Some(held) = snapshot.composer.held.clone() {
            dock = dock.child(
                Notice::new("agent-held", UiTone::Info, "Waiting for the next step")
                    .body(held)
                    .action(self.action_button(
                        "agent-held-cancel",
                        "Cancel message",
                        true,
                        Tone::Normal,
                        cx,
                        |this, _, cx| {
                            if let Some(workflow) = this.workflow().cloned() {
                                let result = workflow.cancel_held();
                                this.command(result, cx);
                            }
                        },
                    )),
            );
        }
        if let Some(text) = self.host.refused.clone() {
            dock = dock.child(
                Notice::new(
                    "agent-refused",
                    UiTone::Warning,
                    "The agent did not take this message",
                )
                .body(text)
                .action(self.action_button(
                    "agent-refused-restore",
                    "Put back",
                    true,
                    Tone::Normal,
                    cx,
                    |this, _, cx| this.restore_refused(cx),
                ))
                .action(self.action_button(
                    "agent-refused-dismiss",
                    "Dismiss",
                    true,
                    Tone::Normal,
                    cx,
                    |this, _, cx| {
                        this.host.refused = None;
                        cx.notify();
                    },
                )),
            );
        }
        if let Some(card) = self.stage_card(cx) {
            dock = dock.child(card);
        }
        if self.host.todo.read(cx).is_visible() {
            // Like a question card: scrolls in a short window rather than crowding out the
            // task banner and the composer.
            dock = dock.child(
                div()
                    .id("agent-todo-scroll")
                    .flex_shrink(1.)
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(self.host.todo.clone()),
            );
        }
        // The composer's own Stop covers a live turn; this one covers every other state a
        // task can be in (waiting, reviewing, queued briefs).
        if !snapshot.composer.turn_live
            && let Some(note) = controls.stop.clone()
        {
            dock = dock.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(self.stop_button(cx))
                    .child(div().flex_1().min_w_0().child(self.line(note))),
            );
        }
        dock = dock.child(
            div()
                .id("agent-prompt")
                .on_key_down(cx.listener(|_, event: &KeyDownEvent, _, cx| {
                    let m = &event.keystroke.modifiers;
                    if event.keystroke.key == "escape" && !m.shift && !m.alt && !m.control {
                        cx.emit(PanelEvent::Leave);
                        cx.stop_propagation();
                    }
                }))
                .child(self.host.composer.clone()),
        );
        if let Some(reason) = controls.submit.blocked.clone()
            && !self.host.composer.read(cx).has_draft(cx)
        {
            dock = dock.child(self.line(reason));
        }
        dock
    }

    /// Puts the refused text back into the composer after whatever is there.
    fn restore_refused(&mut self, cx: &mut Context<Self>) {
        let Some(refused) = self.host.refused.take() else {
            return;
        };
        let current = self.host.composer.read(cx).text(cx);
        let text = if current.trim().is_empty() {
            refused
        } else {
            format!("{current}\n\n{refused}")
        };
        self.host
            .composer
            .update(cx, |composer, cx| composer.set_text(text, cx));
        cx.notify();
    }

    /// Redacted observation of the composer side: counts, states and agent-supplied names,
    /// never what the user typed.
    pub(super) fn composer_telemetry(&self, cx: &App) -> serde_json::Value {
        use serde_json::json;
        let composer = self.host.composer.read(cx);
        let todo = self.host.todo.read(cx);
        let history = self.host.history.read(cx);
        let view = self.snapshot.as_ref().map(|s| &s.composer);
        let chip_kinds: Vec<&str> = composer
            .attachments()
            .iter()
            .map(|chip| match chip.draft.kind {
                super::composer::AttachmentKind::File => "file",
                super::composer::AttachmentKind::Image => "image",
            })
            .collect();
        json!({
            "text_chars": composer.text(cx).chars().count(),
            "chips": chip_kinds,
            "commands": self.host.last_commands.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            "slash_rows": composer.slash_rows(),
            "images_off": composer.attach_disabled(AttachKind::Image),
            "files_off": composer.attach_disabled(AttachKind::File),
            "turn_live": view.map(|v| v.turn_live),
            "steer": view.map(|v| format!("{:?}", v.steer)),
            "held": view.is_some_and(|v| v.held.is_some()),
            "unsent": view.is_some_and(|v| v.unsent.is_some()),
            "refused_shown": self.host.refused.is_some(),
            "preparing": self.host.preparing,
            "last_refusal": self.host.last_refusal,
            "stage": self.host.stage.front().map(|s| json!({
                "path": s.staged.path,
                "bytes": s.staged.size,
                "waiting": self.host.stage.len(),
            })),
            "questions": view.map(|v| v.questions.iter().map(|q| self.question_telemetry(q, cx)).collect::<Vec<_>>()),
            "todo": {
                "visible": todo.is_visible(),
                "label": todo.summary().label(),
                "items": todo.items().iter().map(|i| json!({"text": i.text, "status": format!("{:?}", i.status)})).collect::<Vec<_>>(),
            },
            "history": {
                "shown": history.shown().iter().map(|e| json!({
                    "id": e.id, "current": e.current, "title_chars": e.title.chars().count(),
                })).collect::<Vec<_>>(),
                "query_chars": history.query(cx).chars().count(),
            },
        })
    }

    fn question_telemetry(&self, question: &OpenQuestion, cx: &App) -> serde_json::Value {
        let drafts = self.host.questions.get(&question.reference);
        serde_json::json!({
            "items": question.items.len(),
            "selected": drafts.map(|d| d.iter().map(|i| i.selected.len()).collect::<Vec<_>>()),
            "typed": drafts.map(|d| d.iter().map(|i| !i.text.read(cx).text().trim().is_empty()).collect::<Vec<_>>()),
        })
    }
}
