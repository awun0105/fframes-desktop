//! What the composer, the todo panel and the question cards need from the workflow.
//!
//! One immutable [`ComposerView`] rides on every snapshot. The actor builds it from the
//! live driver (capabilities are read from `AgentDriver::capabilities` and nowhere
//! else) and the same value is kept in the registry, so the synchronous checks of the
//! composer operations see exactly what the UI last saw.

use super::model::PermissionRef;
use studio_harness::driver::{CommandInfo, PlanEntry, QuestionItem};

/// How a message typed while the agent works can reach it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SteerSupport {
    /// No live turn can take a message (no agent yet, or the turn is already ending).
    #[default]
    None,
    /// The agent takes no message mid-turn; "send at the next boundary" and "send now"
    /// cancel the turn and prompt again in the same session.
    AtBoundary,
    /// The agent takes a message into the running turn (`AgentWorkflow::steer`).
    Native,
}

/// One question of an agent, with every item it asks. Answer it with
/// `AgentWorkflow::answer_questions`, one [`super::QuestionAnswer`] per item.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenQuestion {
    pub reference: PermissionRef,
    pub title: String,
    pub items: Vec<QuestionItem>,
}

/// Composer-facing state. Everything is display data except the capability flags, which
/// the operations enforce again (the UI is never trusted).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ComposerView {
    /// Slash commands the agent offers (insertion only: the agent interprets them).
    pub commands: Vec<CommandInfo>,
    /// The agent's latest plan, one entry per todo row.
    pub plan: Vec<PlanEntry>,
    /// How a message sent while the agent works is delivered.
    pub steer: SteerSupport,
    /// Images can be attached: the agent advertised image prompts (known after a
    /// readiness check or once a session is live).
    pub images: bool,
    /// Project files can be attached (as links into the agent's working copy).
    pub files: bool,
    /// Files from outside the project can be attached: the agent takes embedded content
    /// (the adapter negotiated embedded context). Known once a session is live.
    pub embedded_files: bool,
    /// Text the agent did not take after the call that sent it had returned `Ok` (a steer
    /// the adapter refused). Show it in the composer again; it is replaced by the next
    /// such failure and cleared by the next message sent.
    pub unsent: Option<String>,
    /// A turn is running and can take a follow-up message.
    pub turn_live: bool,
    /// A message waits for the next step boundary (`send_at_boundary`).
    pub held: Option<String>,
    /// Open agent questions, all items with their options.
    pub questions: Vec<OpenQuestion>,
}
