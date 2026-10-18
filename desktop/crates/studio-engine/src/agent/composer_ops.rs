//! The composer operations of [`AgentWorkflow`]: attachments, steering, send now / send
//! at the next boundary, and answering questions item by item.
//!
//! Everything the composer needs in order to restore the user's text on a refusal is an
//! `Err` returned here, decided from the state the last snapshot showed (the registry
//! keeps the same value); the actor checks again when the message arrives.

use super::{
    AgentWorkflow, AttachmentRequest, ComposerView, Msg, PermissionRef, PreparedAttachment,
    QuestionAnswer, StagedAttachment, SteerSupport, TaskScope, WorkflowError, attachments,
};
use crate::agent_task::validate_brief;

/// Attachment checks that may run at once.
const MAX_PREPARING_AT_ONCE: usize = 8;

/// Longest free-text answer to one question.
const MAX_ANSWER_TEXT_BYTES: usize = 4096;

fn refused(message: &str) -> WorkflowError {
    WorkflowError::Refused(message.to_owned())
}

/// What the agent can take, checked before anything is sent.
fn check_attachments(
    composer: &ComposerView,
    attachments: &[PreparedAttachment],
) -> Result<(), WorkflowError> {
    let images = attachments.iter().filter(|a| a.is_image()).count();
    let files = attachments.len() - images;
    if images > attachments::MAX_ATTACHED_IMAGES {
        return Err(WorkflowError::Refused(format!(
            "At most {} images fit one message.",
            attachments::MAX_ATTACHED_IMAGES
        )));
    }
    if files > attachments::MAX_ATTACHED_FILES {
        return Err(WorkflowError::Refused(format!(
            "At most {} files fit one message.",
            attachments::MAX_ATTACHED_FILES
        )));
    }
    if images > 0 && !composer.images {
        return Err(refused(
            "This agent has not advertised image prompts (check the agent in setup).",
        ));
    }
    if files > 0 && !composer.files {
        return Err(refused("This agent cannot take file attachments."));
    }
    Ok(())
}

impl AgentWorkflow {
    fn composer_now(&self) -> ComposerView {
        self.shared.registry.lock().composer.clone()
    }

    /// Checks one attachment and turns it into something the send operations accept:
    /// the deny list, the project boundary, link handling and image decoding all happen
    /// here. It reads the file system and decodes images, so it BLOCKS the caller for that
    /// long: call it from a background thread (or use [`Self::prepare_attachment_with`]),
    /// never from a UI event or render path.
    pub fn prepare_attachment(
        &self,
        request: AttachmentRequest,
    ) -> Result<PreparedAttachment, WorkflowError> {
        self.open_check()?;
        let composer = self.composer_now();
        match &request {
            AttachmentRequest::Image { .. } if !composer.images => {
                return Err(refused(
                    "This agent has not advertised image prompts (check the agent in setup).",
                ));
            }
            AttachmentRequest::File { .. } | AttachmentRequest::Staged { .. }
                if !composer.files =>
            {
                return Err(refused("This agent cannot take file attachments."));
            }
            _ => {}
        }
        let root = self.shared.controller.lock().project.root.clone();
        attachments::prepare(&root, &self.shared.staged, request)
    }

    /// First step of attaching a file from outside the project: checks it and returns the
    /// canonical path and file identity to show the user, with a one-shot token. Only
    /// [`AttachmentRequest::Staged`] with that token, used once before it expires and
    /// while the file is still the same file, attaches it. Blocks like
    /// [`Self::prepare_attachment`].
    pub fn stage_attachment(
        &self,
        path: &std::path::Path,
    ) -> Result<StagedAttachment, WorkflowError> {
        self.open_check()?;
        if !self.composer_now().files {
            return Err(refused("This agent cannot take file attachments."));
        }
        let root = self.shared.controller.lock().project.root.clone();
        attachments::stage(&root, &self.shared.staged, path)
    }

    /// [`Self::prepare_attachment`] on its own thread; `done` receives the result there.
    pub fn prepare_attachment_with(
        self: &std::sync::Arc<Self>,
        request: AttachmentRequest,
        done: impl FnOnce(Result<PreparedAttachment, WorkflowError>) + Send + 'static,
    ) {
        self.run_attachment_job(done, move |workflow| workflow.prepare_attachment(request));
    }

    /// [`Self::stage_attachment`] on its own thread; `done` receives the result there.
    pub fn stage_attachment_with(
        self: &std::sync::Arc<Self>,
        path: std::path::PathBuf,
        done: impl FnOnce(Result<StagedAttachment, WorkflowError>) + Send + 'static,
    ) {
        self.run_attachment_job(done, move |workflow| workflow.stage_attachment(&path));
    }

    /// Runs one attachment check on its own thread. Only a few run at once (the heavy
    /// part, decoding an image, is one at a time on top of that); beyond that the caller
    /// hears at once that it should retry.
    fn run_attachment_job<T: Send + 'static>(
        self: &std::sync::Arc<Self>,
        done: impl FnOnce(Result<T, WorkflowError>) + Send + 'static,
        job: impl FnOnce(&Self) -> Result<T, WorkflowError> + Send + 'static,
    ) {
        use std::sync::atomic::Ordering;
        if self.shared.preparing.fetch_add(1, Ordering::SeqCst) >= MAX_PREPARING_AT_ONCE {
            self.shared.preparing.fetch_sub(1, Ordering::SeqCst);
            done(Err(refused(
                "Too many attachments are being checked; try again in a moment.",
            )));
            return;
        }
        let workflow = self.clone();
        let slot = std::sync::Arc::new(parking_lot::Mutex::new(Some(done)));
        let inner = slot.clone();
        let spawned = std::thread::Builder::new()
            .name("studio-workflow-attachment".into())
            .spawn(move || {
                let result = job(&workflow);
                workflow.shared.preparing.fetch_sub(1, Ordering::SeqCst);
                if let Some(done) = inner.lock().take() {
                    done(result);
                }
            });
        if let Err(error) = spawned
            && let Some(done) = slot.lock().take()
        {
            self.shared.preparing.fetch_sub(1, Ordering::SeqCst);
            done(Err(WorkflowError::Refused(error.to_string())));
        }
    }

    /// Submits a brief with attachments (see [`Self::submit_scoped`] for `scope`). The
    /// attachments travel with the first prompt of the task; the conversation shows
    /// their labels only.
    pub fn submit_with_attachments(
        &self,
        brief: &str,
        scope: Option<TaskScope>,
        attachments: Vec<PreparedAttachment>,
    ) -> Result<u64, WorkflowError> {
        self.open_check()?;
        check_attachments(&self.composer_now(), &attachments)?;
        if let Some(scope) = &scope {
            scope
                .validate()
                .map_err(|error| WorkflowError::InvalidScope(error.to_string()))?;
            if scope.project_id != String::from(self.shared.project.clone()) {
                return Err(WorkflowError::InvalidScope(
                    "scope belongs to another project".into(),
                ));
            }
        }
        self.submit_with_scope(brief, scope, attachments)
    }

    /// Delivers `text` into the running turn. Only an agent that takes a message mid-turn
    /// (`SteerSupport::Native`) accepts it; the others use [`Self::send_at_boundary`] or
    /// [`Self::send_now`].
    pub fn steer(&self, text: &str) -> Result<(), WorkflowError> {
        self.open_check()?;
        let text = validate_brief(text).map_err(|e| WorkflowError::InvalidBrief(e.to_string()))?;
        let composer = self.composer_now();
        if composer.steer != SteerSupport::Native {
            return Err(refused("This agent cannot take a message during a turn."));
        }
        if !composer.turn_live {
            return Err(refused("The agent is not running a turn right now."));
        }
        self.send(Msg::Steer(text))
    }

    fn check_followup(
        &self,
        attachments: &[PreparedAttachment],
    ) -> Result<ComposerView, WorkflowError> {
        let composer = self.composer_now();
        if composer.steer == SteerSupport::None || !composer.turn_live {
            return Err(refused(
                "The agent is not running a turn that can be redirected.",
            ));
        }
        check_attachments(&composer, attachments)?;
        Ok(composer)
    }

    /// Sends `text` when the running turn reaches a step boundary: at once when no tool
    /// call is open, otherwise when the last open one finishes. The turn is cancelled and
    /// the agent prompted again in the same session. One message can wait at a time.
    pub fn send_at_boundary(
        &self,
        text: &str,
        attachments: Vec<PreparedAttachment>,
    ) -> Result<(), WorkflowError> {
        self.open_check()?;
        let text = validate_brief(text).map_err(|e| WorkflowError::InvalidBrief(e.to_string()))?;
        let composer = self.check_followup(&attachments)?;
        if composer.held.is_some() {
            return Err(refused("A message is already waiting for the next step."));
        }
        self.send(Msg::SendAtBoundary { text, attachments })
    }

    /// Cancels the running turn and sends `text` as a new prompt of the same session.
    pub fn send_now(
        &self,
        text: &str,
        attachments: Vec<PreparedAttachment>,
    ) -> Result<(), WorkflowError> {
        self.open_check()?;
        let text = validate_brief(text).map_err(|e| WorkflowError::InvalidBrief(e.to_string()))?;
        self.check_followup(&attachments)?;
        self.send(Msg::SendNow { text, attachments })
    }

    /// Drops the message waiting for the next step boundary.
    pub fn cancel_held(&self) -> Result<(), WorkflowError> {
        self.send(Msg::CancelHeld)
    }

    /// Answers an agent question: one [`QuestionAnswer`] per item, in item order, each
    /// naming offered options or carrying free text. Nothing is sent until every item
    /// has an answer; a refusal leaves the question open.
    pub fn answer_questions(
        &self,
        reference: &PermissionRef,
        answers: Vec<QuestionAnswer>,
    ) -> Result<(), WorkflowError> {
        {
            let mut registry = self.shared.registry.lock();
            if !registry.open.contains_key(reference) {
                return Err(if registry.answered.contains(reference) {
                    WorkflowError::DuplicatePermission
                } else {
                    WorkflowError::StalePermission
                });
            }
            let Some(question) = registry
                .composer
                .questions
                .iter()
                .find(|question| &question.reference == reference)
            else {
                return Err(refused(
                    "That request is not a question; answer it with reply_permission.",
                ));
            };
            if answers.len() != question.items.len() {
                return Err(WorkflowError::Refused(format!(
                    "Answer all {} questions before sending.",
                    question.items.len()
                )));
            }
            for (item, answer) in question.items.iter().zip(&answers) {
                let has_text = answer
                    .text
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty());
                if answer.selected.is_empty() && !has_text {
                    return Err(WorkflowError::Refused(format!(
                        "\"{}\" needs an answer.",
                        item.question
                    )));
                }
                if let Some(label) = answer
                    .selected
                    .iter()
                    .find(|label| !item.options.iter().any(|option| &option.label == *label))
                {
                    return Err(WorkflowError::UnknownChoice(label.clone()));
                }
                if !item.multi_select && answer.selected.len() > 1 {
                    return Err(WorkflowError::Refused(format!(
                        "\"{}\" takes one choice.",
                        item.question
                    )));
                }
                if answer
                    .text
                    .as_deref()
                    .is_some_and(|text| text.len() > MAX_ANSWER_TEXT_BYTES)
                {
                    return Err(WorkflowError::Refused(format!(
                        "An answer is longer than {MAX_ANSWER_TEXT_BYTES} bytes."
                    )));
                }
            }
            registry.open.remove(reference);
            registry
                .composer
                .questions
                .retain(|q| &q.reference != reference);
            registry.answered.push_back(reference.clone());
            while registry.answered.len() > 64 {
                registry.answered.pop_front();
            }
        }
        self.send(Msg::AnswerQuestions {
            reference: reference.clone(),
            answers,
        })
    }
}
