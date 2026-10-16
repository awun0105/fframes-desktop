//! The driver trait the engine talks to, whatever wire the agent speaks.
//!
//! Each adapter declares its [`AgentWire`]; [`start_driver`] picks the implementation.
//! Every driver reports through the same [`AgentEvent`] queue and ends each prompted turn
//! with exactly one [`AgentEventKind::PromptFinished`] carrying the turn id its
//! [`AgentDriver::prompt`] returned. Turn ids come from the driver's own counter, so a
//! driver can also end a turn the agent started by itself.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use studio_bootstrap::{ProcessTreeManager, WriterOwnership};

use super::{
    AcpDriver, AgentEvent, AgentFailure, AgentOptions, ClaudeDriver, CodexDriver, DriverConfig,
    DriverError, DriverOutcome, OptionValue, PermissionId, PermissionReply, PromptFile,
    PromptImage, PromptOutcome, SessionInfo,
};

/// The protocol an adapter speaks on its stdio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentWire {
    /// Agent Client Protocol v1.
    #[default]
    Acp,
    /// Claude Code's `--print` stream-json control protocol.
    ClaudeStreamJson,
    /// `codex app-server` JSON-RPC.
    CodexAppServer,
}

impl AgentWire {
    pub const ALL: [Self; 3] = [Self::Acp, Self::ClaudeStreamJson, Self::CodexAppServer];

    pub fn is_acp(&self) -> bool {
        *self == Self::Acp
    }

    /// Stable identifier, also the serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Acp => "acp",
            Self::ClaudeStreamJson => "claude_stream_json",
            Self::CodexAppServer => "codex_app_server",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Acp => "Agent Client Protocol",
            Self::ClaudeStreamJson => "Claude Code (stream-json)",
            Self::CodexAppServer => "Codex (app-server)",
        }
    }
}

/// When a message sent during a running turn reaches the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SteeringMode {
    /// The driver cannot steer; a message waits for the turn to end.
    None,
    /// At the agent's next step inside the running turn.
    StepBoundary,
    /// Only between turns.
    TurnBoundary,
}

/// What a driver can do, fixed for its lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverCapabilities {
    /// Every turn, prompted or started by the agent, ends with a turn-end frame from the
    /// agent's own wire. Candidate capture still waits for writer quiescence.
    pub deterministic_turn_end: bool,
    /// [`AgentDriver::cancel_turn`] ends only the turn; the agent process keeps running.
    pub stops_turn_in_place: bool,
    pub steering: SteeringMode,
    /// The wire can carry prompt images. ACP additionally needs the agent to negotiate
    /// image prompts (`AgentCapabilityInfo::prompt_image`).
    pub images: bool,
    /// The wire can point the agent at files ([`AgentDriver::prompt_with_files`]).
    pub files: bool,
}

/// One question in a [`QuestionPrompt`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionItem {
    /// Short label, possibly empty.
    pub header: String,
    pub question: String,
    pub options: Vec<QuestionOption>,
    pub multi_select: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionOption {
    pub label: String,
    pub description: String,
}

/// A request that asks the user for input rather than for permission. It shares the
/// permission id space: [`AgentDriver::reply_permission`] with
/// [`PermissionReply::Cancel`] declines it, and it closes with
/// [`super::AgentEventKind::PermissionClosed`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionPrompt {
    pub id: PermissionId,
    pub turn: u64,
    pub title: String,
    pub questions: Vec<QuestionItem>,
}

/// The answer to one [`QuestionItem`], in question order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct QuestionAnswer {
    /// Labels of the chosen options.
    pub selected: Vec<String>,
    /// Free text, when the user typed an answer of their own.
    pub text: Option<String>,
}

/// One agent session behind any wire. Every method is synchronous and never blocks on
/// the agent unless its name says so (`wait_*`).
pub trait AgentDriver: Send {
    fn wire(&self) -> AgentWire;
    fn capabilities(&self) -> DriverCapabilities;
    /// The writer ownership established at launch.
    fn ownership(&self) -> WriterOwnership;
    /// The exact session id to resume with, when it carries no secret.
    fn wire_session_id(&self) -> Option<String>;
    fn options(&self) -> AgentOptions;
    fn failure(&self) -> Option<AgentFailure>;
    /// Blocks until the session is ready, the driver fails, or `timeout` passes.
    fn wait_ready(&self, timeout: Duration) -> Result<SessionInfo, AgentFailure>;
    /// Blocks until `turn` ends, the driver fails, or `timeout` passes.
    fn wait_turn(&self, turn: u64, timeout: Duration) -> Result<PromptOutcome, AgentFailure>;
    fn drain_events(&self, max: usize) -> Vec<AgentEvent>;
    /// Largest total of raw PNG bytes `count` images can carry next to `text`.
    fn prompt_image_budget(&self, text: &str, count: usize) -> usize;
    /// Starts a prompted turn; returns its turn id.
    fn prompt(&self, text: &str, images: Vec<PromptImage>) -> Result<u64, DriverError>;
    /// Starts a prompted turn that also points the agent at `files`. A wire that cannot
    /// carry files (`DriverCapabilities::files`) refuses a non-empty list instead of
    /// dropping it.
    fn prompt_with_files(
        &self,
        text: &str,
        files: Vec<PromptFile>,
        images: Vec<PromptImage>,
    ) -> Result<u64, DriverError> {
        if !files.is_empty() {
            return Err(DriverError::Unsupported(
                "this agent cannot take file attachments",
            ));
        }
        self.prompt(text, images)
    }
    /// Delivers `text` to the running turn at the driver's steering boundary. Never waits
    /// for the agent: `Ok` means the message was handed to the wire, and the agent's
    /// answer arrives as exactly one `AgentEventKind::SteerResult`.
    fn steer(&self, text: &str) -> Result<(), DriverError>;
    /// Stops the running turn; the turn still ends with its `PromptFinished`.
    fn cancel_turn(&self) -> Result<(), DriverError>;
    fn reply_permission(&self, id: PermissionId, reply: PermissionReply)
    -> Result<(), DriverError>;
    /// Answers an open [`QuestionPrompt`], one answer per question.
    fn answer_question(
        &self,
        id: PermissionId,
        answers: Vec<QuestionAnswer>,
    ) -> Result<(), DriverError>;
    fn set_mode(&self, mode_id: &str) -> Result<(), DriverError>;
    fn set_config_option(&self, id: &str, value: OptionValue) -> Result<(), DriverError>;
    /// Stops the session and its process tree and reports what the driver saw.
    fn shutdown(self: Box<Self>) -> DriverOutcome;
}

/// Starts a driver for `wire`. The session handshake runs in the background; observe it
/// with [`AgentDriver::wait_ready`] and the event queue.
pub fn start_driver(
    wire: AgentWire,
    config: DriverConfig,
    processes: &ProcessTreeManager,
) -> Result<Box<dyn AgentDriver>, AgentFailure> {
    match wire {
        AgentWire::Acp => Ok(Box::new(AcpDriver::start(config, processes)?)),
        AgentWire::ClaudeStreamJson => Ok(Box::new(ClaudeDriver::start(config, processes)?)),
        AgentWire::CodexAppServer => Ok(Box::new(CodexDriver::start(config, processes)?)),
    }
}

impl AgentDriver for AcpDriver {
    fn wire(&self) -> AgentWire {
        AgentWire::Acp
    }

    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities {
            // ACP adapters may hold a turn open for background work or end it early.
            deterministic_turn_end: false,
            // Cancel keeps the adapter running for the next prompt.
            stops_turn_in_place: true,
            // A message reaches a running turn only through the `_session/steering`
            // extension, when the adapter advertised it.
            steering: if self.steering_negotiated() {
                SteeringMode::StepBoundary
            } else {
                SteeringMode::None
            },
            images: true,
            // Resource links are baseline ACP content.
            files: true,
        }
    }

    fn ownership(&self) -> WriterOwnership {
        AcpDriver::ownership(self)
    }

    fn wire_session_id(&self) -> Option<String> {
        AcpDriver::wire_session_id(self)
    }

    fn options(&self) -> AgentOptions {
        AcpDriver::options(self)
    }

    fn failure(&self) -> Option<AgentFailure> {
        AcpDriver::failure(self)
    }

    fn wait_ready(&self, timeout: Duration) -> Result<SessionInfo, AgentFailure> {
        AcpDriver::wait_ready(self, timeout)
    }

    fn wait_turn(&self, turn: u64, timeout: Duration) -> Result<PromptOutcome, AgentFailure> {
        AcpDriver::wait_turn(self, turn, timeout)
    }

    fn drain_events(&self, max: usize) -> Vec<AgentEvent> {
        AcpDriver::drain_events(self, max)
    }

    fn prompt_image_budget(&self, text: &str, count: usize) -> usize {
        AcpDriver::prompt_image_budget(self, text, count)
    }

    fn prompt(&self, text: &str, images: Vec<PromptImage>) -> Result<u64, DriverError> {
        AcpDriver::prompt_with_images(self, text, images)
    }

    fn prompt_with_files(
        &self,
        text: &str,
        files: Vec<PromptFile>,
        images: Vec<PromptImage>,
    ) -> Result<u64, DriverError> {
        AcpDriver::prompt_with_files(self, text, files, images)
    }

    fn steer(&self, text: &str) -> Result<(), DriverError> {
        AcpDriver::steer_turn(self, text)
    }

    fn cancel_turn(&self) -> Result<(), DriverError> {
        AcpDriver::cancel_prompt(self)
    }

    fn reply_permission(
        &self,
        id: PermissionId,
        reply: PermissionReply,
    ) -> Result<(), DriverError> {
        AcpDriver::reply_permission(self, id, reply)
    }

    fn answer_question(
        &self,
        id: PermissionId,
        _answers: Vec<QuestionAnswer>,
    ) -> Result<(), DriverError> {
        // ACP v1 never asks a question (see `AgentEventKind::Question`).
        Err(DriverError::UnknownPermission(id))
    }

    fn set_mode(&self, mode_id: &str) -> Result<(), DriverError> {
        AcpDriver::set_mode(self, mode_id)
    }

    fn set_config_option(&self, id: &str, value: OptionValue) -> Result<(), DriverError> {
        AcpDriver::set_config_option(self, id, value)
    }

    fn shutdown(self: Box<Self>) -> DriverOutcome {
        AcpDriver::shutdown(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_serializes_as_its_stable_name_and_defaults_to_acp() {
        for wire in AgentWire::ALL {
            let json = serde_json::to_string(&wire).unwrap();
            assert_eq!(json, format!("\"{}\"", wire.as_str()));
            assert_eq!(serde_json::from_str::<AgentWire>(&json).unwrap(), wire);
        }
        assert_eq!(AgentWire::default(), AgentWire::Acp);
    }
}
