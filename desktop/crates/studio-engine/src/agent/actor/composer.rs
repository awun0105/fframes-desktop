//! The composer's side of the actor: follow-up delivery (steer, send now, send at the
//! next step boundary), attachments on prompts, open questions and the published
//! composer view.
//!
//! A redirect (send now, or a boundary message once no tool is open) cancels the live
//! turn and prompts again in the same session. The cancelled turn ends with its own
//! outcome, and that end is the start of the follow-up, never a failed edit, so
//! `composer_turn_finished` consumes it before the task logic sees it.

use super::prompt::prompt_failure;
use super::*;
use crate::agent::attachments::MAX_ATTACHED_IMAGES;
use crate::agent::{ComposerView, OpenQuestion, SteerSupport};
use studio_harness::driver::{
    CommandInfo, PermissionReply, PlanEntry, PromptFile, SteeringMode, ToolStatus,
};

/// Room left for the text the evidence section adds after the image budget is measured.
const EVIDENCE_TEXT_SLACK: usize = 2048;

/// A message to deliver as a new prompt of the live session.
pub(super) struct Followup {
    text: String,
    files: Vec<PromptFile>,
    images: Vec<PromptImage>,
    /// What the conversation shows: the text plus the attachment labels.
    shown: String,
    /// The driver it was prepared for; a message for a retired driver is dropped.
    epoch: u64,
}

#[derive(Default)]
pub(super) struct ComposerRun {
    /// Tool calls the agent started and has not finished in the live turn.
    pub open_tools: HashSet<String>,
    pub commands: Vec<CommandInfo>,
    pub plan: Vec<PlanEntry>,
    pub questions: Vec<OpenQuestion>,
    /// A message waiting for the turn to reach a step boundary.
    held: Option<Followup>,
    /// A message to send as soon as the cancelled turn has ended.
    redirect: Option<Followup>,
    /// Steers the driver took for sending whose answer has not arrived yet, oldest first.
    pending_steers: VecDeque<String>,
    /// Text the agent did not take (see `ComposerView::unsent`).
    unsent: Option<String>,
}

impl ComposerRun {
    /// Forgets everything that belonged to the driver being retired. `true` when a message
    /// the user had sent was still waiting and is dropped with it.
    /// The plan stays: it is the task's, and the next task clears it.
    pub(super) fn retire(&mut self) -> bool {
        let dropped = self.held.is_some() || self.redirect.is_some();
        let plan = std::mem::take(&mut self.plan);
        *self = Self {
            plan,
            ..Self::default()
        };
        dropped
    }
}

fn attached_text(text: &str, labels: &[&str]) -> String {
    if labels.is_empty() {
        text.to_owned()
    } else {
        format!("{text}\n\nAttached: {}", labels.join(", "))
    }
}

impl Actor {
    /// Retires what belonged to the driver being replaced or closed and says so when a
    /// message of the user's was still waiting for it.
    pub(super) fn retire_composer(&mut self) {
        if self.composer.retire() {
            self.warn("A message waiting to be sent was dropped because the agent session ended.");
        }
    }

    /// Checks `attachments` against the live agent and turns them into what the driver
    /// takes. The reason of a refusal is meant to be shown.
    fn checked_attachments(
        &self,
        text: &str,
        attachments: Vec<PreparedAttachment>,
    ) -> Result<(Vec<PromptFile>, Vec<PromptImage>), String> {
        if attachments.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let Some(driver) = self
            .active
            .as_ref()
            .and_then(|active| active.driver.as_deref())
        else {
            return Err("There is no live agent session to attach to.".into());
        };
        let caps = driver.capabilities();
        let draft = self
            .active
            .as_ref()
            .map(|active| active.ctx.draft.clone())
            .unwrap_or_default();
        let (files, images) = PreparedAttachment::split(attachments, &draft)?;
        if !files.is_empty() && !caps.files {
            return Err("This agent cannot take file attachments.".into());
        }
        if files.iter().any(PromptFile::is_embedded)
            && !self
                .capabilities
                .as_ref()
                .is_some_and(|c| c.prompt_embedded_context)
        {
            return Err(
                "This agent did not negotiate file content from outside the project.".into(),
            );
        }
        if images.is_empty() {
            return Ok((files, Vec::new()));
        }
        if !caps.images || !self.capabilities.as_ref().is_some_and(|c| c.prompt_image) {
            return Err("This agent did not advertise image prompts.".into());
        }
        if images.len() > MAX_ATTACHED_IMAGES {
            return Err(format!(
                "At most {MAX_ATTACHED_IMAGES} images fit one message."
            ));
        }
        let evidence = self
            .task
            .as_ref()
            .and_then(|task| task.before.as_ref())
            .map(|before| present::before_evidence_prompt(before, None))
            .unwrap_or_default();
        let budget = driver
            .prompt_image_budget(&format!("{text}{evidence}"), images.len())
            .saturating_sub(EVIDENCE_TEXT_SLACK);
        let total: usize = images.iter().map(Vec::len).sum();
        if total > budget {
            return Err(format!(
                "The images are too large for one message to this agent ({} KiB of {} KiB).",
                total.div_ceil(1024),
                budget / 1024
            ));
        }
        Ok((files, images.into_iter().map(PromptImage::png).collect()))
    }

    /// What the user attached to the brief, ready for the first prompt. A refusal is
    /// shown and the text still goes out, so nothing is dropped without saying so.
    pub(super) fn take_first_attachments(
        &mut self,
        text: &str,
    ) -> (Vec<PromptFile>, Vec<PromptImage>) {
        let attachments = match &mut self.active {
            Some(active) => std::mem::take(&mut active.first_attachments),
            None => return Default::default(),
        };
        if attachments.is_empty() {
            return Default::default();
        }
        let labels: Vec<String> = attachments.iter().map(|a| a.label().to_owned()).collect();
        match self.checked_attachments(text, attachments) {
            Ok(checked) => {
                // The conversation names only what is actually sent with the brief.
                let row = self.active.as_ref().and_then(|active| active.brief_row);
                if let Some(row) = row {
                    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                    self.update_row(row, |kind| {
                        if let RowKind::User { text, .. } = kind {
                            *text = present::bounded(
                                &attached_text(text, &label_refs),
                                MAX_ROW_TEXT_BYTES,
                            );
                        }
                    });
                }
                checked
            }
            Err(reason) => {
                self.warn(format!("The attachments were not sent: {reason}"));
                Default::default()
            }
        }
    }

    // ---- tool tracking --------------------------------------------------------------------------------

    /// Follows which tool calls are open: a held message waits for the last to close.
    pub(super) fn composer_track_tool(&mut self, event: &ToolEvent) {
        match event.status {
            Some(ToolStatus::Completed | ToolStatus::Failed) => {
                self.composer.open_tools.remove(&event.tool_call_id);
            }
            Some(ToolStatus::Pending | ToolStatus::InProgress) => {
                self.composer.open_tools.insert(event.tool_call_id.clone());
            }
            None => {
                if !self.tool_rows.contains_key(&event.tool_call_id) {
                    self.composer.open_tools.insert(event.tool_call_id.clone());
                }
            }
        }
        // A redirect already cancelling the turn owns the next prompt: a held message
        // never takes its place.
        if self.composer.open_tools.is_empty()
            && self.composer.redirect.is_none()
            && let Some(held) = self.composer.held.take()
        {
            self.start_redirect(held);
        }
    }

    // ---- follow-ups -----------------------------------------------------------------------------------

    /// Whether the live turn can take a follow-up now.
    fn turn_accepts_followup(&self) -> bool {
        self.composer.redirect.is_none()
            && self.active.as_ref().is_some_and(|active| {
                active.driver.is_some()
                    && active.turn.is_some()
                    && !active.stop_requested
                    && !active.failed
                    && matches!(active.phase, TaskPhase::Editing | TaskPhase::Repairing)
            })
    }

    fn prepare_followup(
        &mut self,
        text: String,
        attachments: Vec<PreparedAttachment>,
    ) -> Option<Followup> {
        if !self.turn_accepts_followup() {
            self.warn("The agent is not working on a turn that can take a message right now.");
            return None;
        }
        let labels: Vec<String> = attachments.iter().map(|a| a.label().to_owned()).collect();
        let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let shown = attached_text(&text, &label_refs);
        match self.checked_attachments(&text, attachments) {
            Ok((files, images)) => Some(Followup {
                text,
                files,
                images,
                shown,
                epoch: self.driver_epoch,
            }),
            Err(reason) => {
                self.warn(format!("The message was not sent: {reason}"));
                None
            }
        }
    }

    /// Delivers text into the running turn. Text only: the wire has no attachments there.
    pub(super) fn composer_steer(&mut self, text: String) {
        if !self.turn_accepts_followup() {
            self.warn("The agent is not working on a turn that can take a message right now.");
            return;
        }
        let Some(driver) = self
            .active
            .as_ref()
            .and_then(|active| active.driver.as_deref())
        else {
            return;
        };
        if driver.capabilities().steering != SteeringMode::StepBoundary {
            self.warn("This agent cannot take a message during a turn.");
            return;
        }
        match driver.steer(&text) {
            // The adapter has not answered yet: the message shows as sent only once it
            // accepts it (`on_steer_result`).
            Ok(()) => {
                self.composer.unsent = None;
                self.composer.pending_steers.push_back(text);
            }
            Err(error) => {
                self.composer.unsent = Some(text);
                self.warn(format!("The message was not delivered: {error}"));
                self.touch();
            }
        }
    }

    /// The agent's answer to a message handed over with `composer_steer`.
    pub(super) fn on_steer_result(&mut self, text: String, accepted: bool, message: String) {
        let Some(at) = self
            .composer
            .pending_steers
            .iter()
            .position(|pending| *pending == text)
        else {
            return;
        };
        self.composer.pending_steers.remove(at);
        if accepted {
            self.finish_streaming();
            self.push_row(RowKind::User {
                text: present::bounded(&text, MAX_ROW_TEXT_BYTES),
                source: UserSource::Followup,
            });
            return;
        }
        let reason = if message.is_empty() {
            String::new()
        } else {
            format!(": {message}")
        };
        self.warn(format!(
            "The agent did not take the message into the running turn and it was not sent{reason}"
        ));
        self.composer.unsent = Some(text);
        self.touch();
    }

    pub(super) fn composer_send_now(&mut self, text: String, attachments: Vec<PreparedAttachment>) {
        let Some(followup) = self.prepare_followup(text, attachments) else {
            return;
        };
        // Send now replaces a message waiting for a step boundary; it is not queued
        // behind or over it silently.
        if self.composer.held.take().is_some() {
            self.warn(
                "The message waiting for the next step was dropped because you sent another now.",
            );
        }
        self.start_redirect(followup);
    }

    pub(super) fn composer_send_at_boundary(
        &mut self,
        text: String,
        attachments: Vec<PreparedAttachment>,
    ) {
        if self.composer.held.is_some() {
            self.warn("A message is already waiting for the next step; cancel it first.");
            return;
        }
        let Some(followup) = self.prepare_followup(text, attachments) else {
            return;
        };
        if self.composer.open_tools.is_empty() {
            self.start_redirect(followup);
        } else {
            self.composer.held = Some(followup);
            self.touch();
        }
    }

    pub(super) fn composer_cancel_held(&mut self) {
        if self.composer.held.take().is_some() {
            self.touch();
        }
    }

    /// Cancels the live turn; its end sends `followup`.
    fn start_redirect(&mut self, followup: Followup) {
        let cancelled = match self
            .active
            .as_ref()
            .and_then(|active| active.driver.as_deref())
        {
            Some(driver) => driver.cancel_turn(),
            None => return,
        };
        match cancelled {
            Ok(()) => {
                self.composer.redirect = Some(followup);
                self.touch();
            }
            Err(error) => self.warn(format!("The message was not sent: {error}")),
        }
    }

    /// Called as a turn ends, before the task logic. `true` means the end started the
    /// follow-up turn and the task goes on.
    pub(super) fn composer_turn_finished(&mut self, outcome: &PromptOutcome) -> bool {
        let redirect = self.composer.redirect.take();
        let held = self.composer.held.take();
        self.composer.open_tools.clear();
        if !self.composer.pending_steers.is_empty() {
            let unconfirmed: Vec<String> = self.composer.pending_steers.drain(..).collect();
            self.warn(
                "A message sent during the turn was not confirmed by the agent before the turn ended; it may not have reached it.",
            );
            self.composer.unsent = Some(unconfirmed.join("\n\n"));
            self.touch();
        }
        let clean = outcome.eligible_for_quiescence();
        let (followup, dropped_held) = match (redirect, held) {
            (Some(redirect), held) => (Some(redirect), held.is_some()),
            (None, Some(held)) if clean => (Some(held), false),
            (None, held) => (None, held.is_some()),
        };
        if dropped_held {
            self.warn(
                "A held message was not sent because the turn ended without a step boundary.",
            );
        }
        let Some(followup) = followup else {
            return false;
        };
        let stopped = self
            .active
            .as_ref()
            .is_none_or(|active| active.stop_requested || active.failed || active.driver.is_none());
        if stopped || followup.epoch != self.driver_epoch {
            self.warn("The message was not sent because the agent session ended.");
            return false;
        }
        let Followup {
            text,
            files,
            images,
            shown,
            ..
        } = followup;
        match self.try_send_prompt(&text, files, images) {
            Ok(()) => {
                self.composer.unsent = None;
                self.push_row(RowKind::User {
                    text: present::bounded(&shown, MAX_ROW_TEXT_BYTES),
                    source: UserSource::Followup,
                });
                true
            }
            Err(error) => {
                let failure = prompt_failure(error);
                self.warn(format!("The message was not sent: {}", failure.message));
                false
            }
        }
    }

    // ---- questions ------------------------------------------------------------------------------------

    pub(super) fn composer_answer_questions(
        &mut self,
        reference: PermissionRef,
        answers: Vec<QuestionAnswer>,
    ) {
        let Some(active) = &self.active else {
            self.warn("That question is no longer current.");
            return;
        };
        let current = active.writer.map_or(0, WriterGeneration::value);
        if active.ctx.identity.task != reference.task || current != reference.writer {
            self.warn("That question belongs to an earlier agent session; it was ignored.");
            return;
        }
        let Some(driver) = &active.driver else {
            return;
        };
        let id = PermissionId(reference.request);
        let failed = match driver.answer_question(id, answers) {
            Ok(()) => return,
            Err(error) => error,
        };
        // The question already left the open list when the answers were accepted for
        // sending, so nobody can answer it again: close it as cancelled rather than
        // leave the agent waiting for it.
        let closed = driver.reply_permission(id, PermissionReply::Cancel);
        self.warn(match closed {
            Ok(()) => {
                format!("The answers were not delivered ({failed}); the question was cancelled.")
            }
            Err(_) => format!("The answers were not delivered: {failed}"),
        });
    }

    // ---- publication ----------------------------------------------------------------------------------

    /// The composer view for the next snapshot; the registry keeps the same value for the
    /// synchronous checks of the composer operations.
    pub(super) fn publish_composer(&self) -> ComposerView {
        let live = self
            .active
            .as_ref()
            .filter(|active| active.driver.is_some());
        let caps = live
            .and_then(|active| active.driver.as_deref())
            .map(|driver| driver.capabilities());
        let turn_live = self.turn_accepts_followup();
        let steer = match caps {
            Some(caps) if turn_live => {
                if caps.steering == SteeringMode::StepBoundary {
                    SteerSupport::Native
                } else if caps.stops_turn_in_place {
                    SteerSupport::AtBoundary
                } else {
                    SteerSupport::None
                }
            }
            _ => SteerSupport::None,
        };
        let advertised_images = self.capabilities.as_ref().is_some_and(|c| c.prompt_image);
        let (images, files) = match caps {
            Some(caps) => (caps.images && advertised_images, caps.files),
            None => {
                let probed = match &self.adapter.readiness {
                    AdapterReadiness::Checked { report, .. } => report.supports_images(),
                    _ => false,
                };
                let wire = self
                    .shared
                    .settings
                    .lock()
                    .adapter
                    .as_ref()
                    .map(|adapter| adapter.wire);
                (
                    probed || advertised_images,
                    wire == Some(studio_harness::AgentWire::Acp),
                )
            }
        };
        let held = self
            .composer
            .held
            .as_ref()
            .filter(|held| held.epoch == self.driver_epoch && live.is_some())
            .map(|held| self.scrub_str(&held.shown));
        let mut registry = self.shared.registry.lock();
        let questions = self
            .composer
            .questions
            .iter()
            .filter(|question| registry.open.contains_key(&question.reference))
            .map(|question| {
                let mut question = question.clone();
                question.title = self.scrub_str(&question.title);
                for item in &mut question.items {
                    item.header = self.scrub_str(&item.header);
                    item.question = self.scrub_str(&item.question);
                    for option in &mut item.options {
                        option.description = self.scrub_str(&option.description);
                    }
                }
                question
            })
            .collect();
        let view = ComposerView {
            commands: self
                .composer
                .commands
                .iter()
                .map(|command| CommandInfo {
                    description: self.scrub_str(&command.description),
                    ..command.clone()
                })
                .collect(),
            plan: self
                .composer
                .plan
                .iter()
                .map(|entry| PlanEntry {
                    text: self.scrub_str(&entry.text),
                    ..entry.clone()
                })
                .collect(),
            steer,
            images,
            files,
            embedded_files: self
                .capabilities
                .as_ref()
                .is_some_and(|c| c.prompt_embedded_context)
                && live.is_some(),
            unsent: self
                .composer
                .unsent
                .as_deref()
                .map(|text| self.scrub_str(text)),
            turn_live,
            held,
            questions,
        };
        registry.composer = view.clone();
        view
    }
}
