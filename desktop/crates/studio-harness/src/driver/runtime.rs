//! Production ACP v1 driver on the official SDK.
//!
//! One adapter process is owned per driver (inside a caller-supplied
//! [`ProcessTreeManager`] scope). The SDK connection runs on a dedicated owned
//! thread with `futures::executor`, never on a UI or audio thread. Hosts interact
//! through a synchronous handle and a bounded event queue.
//!
//! Concurrency contract: turn state, cancellation and permission requests all live under
//! the single `core` mutex, so "Stop", "grant" and "turn finished" are totally ordered.
//! Teardown after a failure never runs on a handle caller's thread: failures only latch
//! state and wake the reaper thread. Lock order: `core` is never held while taking
//! `child`; `child` may be held while taking `core`.

use super::{
    McpStdioServer,
    events::*,
    redact::{Redactor, StreamRedactor},
    transcript::{Transcript, TranscriptPage},
    transport::{
        self, INGRESS_MAX_BYTES, INGRESS_MAX_FRAMES, Ingress, LineSink, LineStream, fits_wire,
    },
};
use crate::{AgentSupervisor, SupervisorError, discovery::AdapterLaunch};
use agent_client_protocol::{
    Agent, Client, ConnectionTo, Dispatch, Error as RpcError, Handled, JsonRpcMessage, Lines,
    RawJsonRpcMessage, Responder,
    schema::{
        ProtocolVersion,
        v1::{
            AuthenticateRequest, CancelNotification, ClientCapabilities, ContentBlock, EnvVariable,
            ImageContent, Implementation, InitializeRequest, InitializeResponse,
            LoadSessionRequest, McpServer, McpServerStdio, NewSessionRequest, PromptRequest,
            PromptResponse, RequestId, RequestPermissionOutcome, RequestPermissionRequest,
            RequestPermissionResponse, ResumeSessionRequest, SelectedPermissionOutcome,
            SessionConfigKind, SessionConfigOption, SessionConfigOptionValue,
            SessionConfigSelectOptions, SessionModeState, SessionNotification, SessionUpdate,
            SetSessionConfigOptionRequest, SetSessionModeRequest, StopReason, TextContent,
            ToolCallStatus,
        },
    },
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures::{StreamExt, channel::mpsc, future::Either};
use parking_lot::{Condvar, Mutex};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use studio_bootstrap::{
    EscapeScan, ProcessError, ProcessTreeManager, TerminationReport, TrackedChild, WriterOwnership,
};

const CLIENT_NAME: &str = "fframes-studio";
/// `agentInfo.name` of the Claude ACP adapter (its npm package name), current and former.
const CLAUDE_ADAPTER_NAMES: [&str; 2] = [
    "@agentclientprotocol/claude-agent-acp",
    "@zed-industries/claude-code-acp",
];
const MAX_PROMPT_BYTES: usize = 256 * 1024;
const MAX_PROMPT_IMAGE_COUNT: usize = 3;
const MAX_PENDING_PERMISSIONS: usize = 16;
const FAILURE_DETAIL_BYTES: usize = 2048;
/// Host commands accepted but not yet executed by the connection task.
const COMMAND_QUEUE: usize = 8;
/// Mode/config requests that may await an adapter reply at once.
pub const MAX_OUTSTANDING_REQUESTS: usize = 4;

/// Engineering bounds; defaults are the Stage 1 contract values.
#[derive(Debug, Clone)]
pub struct DriverLimits {
    pub max_message_bytes: usize,
    pub max_queued_events: usize,
    pub max_transcript_bytes: usize,
    pub max_stderr_bytes: usize,
    pub init_timeout: Duration,
    /// Idle time allowed during a turn. Only real agent activity restarts it: message and
    /// thought chunks, tool calls and their updates, plan updates and permission requests.
    /// Command lists, session info and option pushes do not.
    pub prompt_timeout: Duration,
    /// Hard cap on one turn from the moment it is sent, however active the adapter is.
    pub max_turn: Duration,
    /// Graceful drain before the owned tree is force-killed.
    pub shutdown_grace: Duration,
    /// Time an adapter has to answer a cancelled turn before it is killed.
    pub cancel_ack_timeout: Duration,
    /// Time an adapter has to answer a mode/config request.
    pub request_timeout: Duration,
}

impl Default for DriverLimits {
    fn default() -> Self {
        Self {
            max_message_bytes: 1024 * 1024,
            max_queued_events: 256,
            max_transcript_bytes: 4 * 1024 * 1024,
            max_stderr_bytes: 1024 * 1024,
            init_timeout: Duration::from_secs(30),
            prompt_timeout: Duration::from_secs(15 * 60),
            max_turn: Duration::from_secs(2 * 60 * 60),
            shutdown_grace: Duration::from_secs(2),
            cancel_ack_timeout: Duration::from_secs(2),
            request_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverMode {
    /// Initialize, authenticate (if configured), create/restore a session.
    Full,
    /// Stop after `initialize`; used by discovery when proving a session is unwanted.
    InitializeOnly,
}

/// Whether stdio MCP servers are sent in `session/new` / `session/load` /
/// `session/resume`. ACP v1 has no stdio capability flag (stdio is the mandatory
/// baseline transport; `McpCapabilities` only advertises `http`/`sse`, and a `stdio`
/// flag exists only in v2, which is never enabled), so the gate is host policy on top
/// of the already enforced v1 negotiation. HTTP and SSE servers are never sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum McpStdioSupport {
    #[default]
    Baseline,
    /// Always send an empty server list.
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct DriverConfig {
    pub provider: String,
    pub task: String,
    /// Absolute session working directory (the stable draft).
    pub cwd: PathBuf,
    pub launch: AdapterLaunch,
    pub limits: DriverLimits,
    /// Restore this session instead of creating one; fails unless negotiated.
    pub resume_session: Option<String>,
    /// Qualified writer model for this adapter; anything but a qualification result
    /// must be `Unknown`.
    pub writer_ownership: WriterOwnership,
    pub mode: DriverMode,
    /// Task-specific stdio MCP servers; env values are added to the redaction set.
    pub mcp_servers: Vec<McpStdioServer>,
    pub mcp_stdio: McpStdioSupport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverStatus {
    Starting,
    Ready,
    Prompting,
    Stopping,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub session_id: Option<String>,
    pub restored: bool,
    pub initialized: InitializedInfo,
}

/// Final facts about a closed driver; the inputs to the quiescence gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverOutcome {
    /// The `DriverConfig::task` binding this outcome belongs to.
    pub task: String,
    /// Sanitized provider session id, if a session was established.
    pub session: Option<String>,
    /// Verified termination of the owned task process group.
    pub termination: TerminationReport,
    /// Configured ownership, demoted to `Detached` if an escape was observed or
    /// containment could not be verified.
    pub ownership: WriterOwnership,
    pub escaped_pids: Vec<u32>,
    /// Why containment of the task's processes could not be verified (an escape scan or
    /// the verified termination failed), if it could not.
    pub containment_unverified: Option<String>,
    /// The last finished turn, if any. A latched failure always dominates it: consumers
    /// must refuse capture authority when `failure` is set.
    pub last_prompt: Option<PromptOutcome>,
    pub failure: Option<AgentFailure>,
    pub late_events_rejected: u64,
    pub exit_code: Option<i32>,
}

#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("the agent session is closed")]
    Closed,
    #[error("{0}")]
    Failed(AgentFailure),
    #[error("the agent session is not ready")]
    NotReady,
    #[error("a prompt turn is already in progress")]
    TurnInProgress,
    #[error("no prompt turn is in progress")]
    NoActiveTurn,
    #[error("permission request {0:?} is not pending (duplicate, late or closed)")]
    UnknownPermission(PermissionId),
    #[error("{0}")]
    InvalidOption(String),
    #[error("{0}")]
    InvalidPrompt(&'static str),
    /// A bounded admission limit is full; retry after the adapter answers.
    #[error("{0}")]
    Busy(&'static str),
}

#[derive(Debug)]
enum Command {
    Prompt {
        turn: u64,
        content: Vec<ContentBlock>,
    },
    /// Nudges the loop to look at latched state such as a pending cancel.
    Wake,
    SetMode {
        raw: String,
        label: String,
        key: u64,
    },
    SetConfig {
        raw_id: String,
        label: String,
        value: OptionValue,
        key: u64,
    },
    Shutdown,
}

/// A bounded PNG attachment for an ACP prompt. The image is never added to the driver's
/// text transcript or persisted conversation log.
pub struct PromptImage {
    data: Vec<u8>,
}

impl PromptImage {
    pub fn png(data: Vec<u8>) -> Self {
        Self { data }
    }
}

#[derive(Debug, Clone)]
struct Turn {
    id: u64,
    /// Latched by the host call itself, before any command is queued.
    cancel_requested: bool,
}

struct PendingPermission {
    responder: Responder<RequestPermissionResponse>,
    /// `(exported, raw)` option ids; only exported ids are ever shown to hosts.
    options: Vec<(String, String)>,
    wire_id: String,
}

#[derive(Default)]
struct Permissions {
    next: u64,
    pending: HashMap<u64, PendingPermission>,
    /// JSON-RPC ids of in-flight permission requests; duplicates are protocol violations.
    wire_ids: HashSet<String>,
}

struct Core {
    status: DriverStatus,
    phase: Phase,
    session: Option<String>,
    restored: bool,
    restoring: bool,
    initialized: Option<InitializedInfo>,
    turn: Option<Turn>,
    next_turn: u64,
    last_outcome: Option<PromptOutcome>,
    failure: Option<AgentFailure>,
    stopping: bool,
    escaped: Vec<u32>,
    /// First reason containment could not be verified.
    containment_unverified: Option<String>,
    outcome: Option<DriverOutcome>,
    permissions: Permissions,
    /// Turn whose `session/cancel` notification still has to be written.
    cancel_to_send: Option<u64>,
}

/// Runs SDK calls with SDK diagnostics muted. The SDK traces whole wire messages (a
/// permission reply carries the raw option id) from whichever thread sends them, so every
/// call into it that is not already on the muted connection thread goes through here.
fn quiet<R>(f: impl FnOnce() -> R) -> R {
    tracing::subscriber::with_default(tracing::subscriber::NoSubscriber::default(), f)
}

impl Core {
    /// Answers every open permission request as cancelled, in one ordered step with
    /// whatever latched the caller's state change. Returns the closed ids.
    fn drain_permissions(&mut self) -> Vec<PermissionId> {
        let mut drained: Vec<(u64, PendingPermission)> = self.permissions.pending.drain().collect();
        drained.sort_by_key(|(id, _)| *id);
        self.permissions.wire_ids.clear();
        drained
            .into_iter()
            .map(|(id, pending)| {
                let _ = quiet(|| {
                    pending.responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    ))
                });
                PermissionId(id)
            })
            .collect()
    }
}

struct Streams {
    agent: StreamRedactor,
    thought: StreamRedactor,
    user: StreamRedactor,
}

impl Streams {
    fn get(&mut self, role: MessageRole) -> &mut StreamRedactor {
        match role {
            MessageRole::Agent => &mut self.agent,
            MessageRole::Thought => &mut self.thought,
            MessageRole::User => &mut self.user,
        }
    }
}

const ROLES: [MessageRole; 3] = [MessageRole::Agent, MessageRole::Thought, MessageRole::User];

/// Incrementally sanitized stderr: only redacted text is ever retained or exposed.
struct StderrState {
    redactor: StreamRedactor,
    carry: Vec<u8>,
    tail: VecDeque<u8>,
}

impl StderrState {
    fn append(&mut self, released: &str, max: usize) {
        self.tail.extend(released.as_bytes());
        if self.tail.len() > max {
            let excess = self.tail.len() - max;
            self.tail.drain(..excess);
        }
    }
}

/// Exported-id -> wire-id tables; wire ids may carry secrets and never leave this module.
#[derive(Default)]
struct OptionIds {
    modes: HashMap<String, String>,
    config: HashMap<String, RawConfig>,
}

struct RawConfig {
    raw_id: String,
    values: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Deadline {
    at: Instant,
    phase: Phase,
    /// Hard limit `at` can never be extended past (the turn cap).
    cap: Option<Instant>,
}

struct WatchState {
    phase_deadline: Option<Deadline>,
    requests: HashMap<u64, Instant>,
    next_request: u64,
    kill: bool,
    stop: bool,
}

enum WatchAction {
    Kill,
    Phase(Phase),
    /// The prompt turn reached `DriverLimits::max_turn`.
    TurnCap,
    RequestTimedOut,
}

fn earliest(a: Option<Instant>, b: Option<Instant>) -> Option<Instant> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// Deadline ledger and reaper wake-up shared with the watchdog thread.
struct Watchdog {
    state: Mutex<WatchState>,
    changed: Condvar,
}

impl Watchdog {
    fn new() -> Self {
        Self {
            state: Mutex::new(WatchState {
                phase_deadline: None,
                requests: HashMap::new(),
                next_request: 0,
                kill: false,
                stop: false,
            }),
            changed: Condvar::new(),
        }
    }
    fn arm(&self, phase: Phase, after: Duration) {
        self.state.lock().phase_deadline = Instant::now().checked_add(after).map(|at| Deadline {
            at,
            phase,
            cap: None,
        });
        self.changed.notify_all();
    }
    /// Arms the prompt idle deadline under a hard cap of `max_turn` from now.
    fn arm_turn(&self, idle: Duration, max_turn: Duration) {
        let now = Instant::now();
        let cap = now.checked_add(max_turn);
        self.state.lock().phase_deadline =
            earliest(now.checked_add(idle), cap).map(|at| Deadline {
                at,
                phase: Phase::Prompt,
                cap,
            });
        self.changed.notify_all();
    }
    fn disarm(&self) {
        self.state.lock().phase_deadline = None;
        self.changed.notify_all();
    }
    /// Pushes an armed `phase` deadline to `after` from now, never past its cap; any
    /// other deadline is kept.
    fn extend(&self, phase: Phase, after: Duration) {
        let mut state = self.state.lock();
        if let Some(deadline) = &mut state.phase_deadline
            && deadline.phase == phase
            && let Some(at) = earliest(Instant::now().checked_add(after), deadline.cap)
        {
            deadline.at = at;
            self.changed.notify_all();
        }
    }
    fn stop(&self) {
        self.state.lock().stop = true;
        self.changed.notify_all();
    }
    fn request_kill(&self) {
        self.state.lock().kill = true;
        self.changed.notify_all();
    }
    /// Admits one outstanding request with a reply deadline, or refuses at `max`.
    fn track_request(&self, after: Duration, max: usize) -> Option<u64> {
        let mut state = self.state.lock();
        if state.requests.len() >= max {
            return None;
        }
        let key = state.next_request;
        state.next_request += 1;
        state.requests.insert(key, Instant::now() + after);
        self.changed.notify_all();
        Some(key)
    }
    fn untrack(&self, key: u64) {
        self.state.lock().requests.remove(&key);
        self.changed.notify_all();
    }
    fn outstanding(&self) -> usize {
        self.state.lock().requests.len()
    }
    fn next_action(&self) -> Option<WatchAction> {
        let mut state = self.state.lock();
        loop {
            if state.stop {
                return None;
            }
            if state.kill {
                state.kill = false;
                return Some(WatchAction::Kill);
            }
            let request = state
                .requests
                .iter()
                .min_by_key(|(_, at)| **at)
                .map(|(k, at)| (*k, *at));
            let phase = state.phase_deadline;
            let next = earliest(phase.map(|d| d.at), request.map(|(_, at)| at));
            let Some(at) = next else {
                self.changed.wait(&mut state);
                continue;
            };
            if Instant::now() >= at {
                if let Some(deadline) = phase
                    && deadline.at <= at
                {
                    state.phase_deadline = None;
                    return Some(if deadline.cap.is_some_and(|cap| deadline.at >= cap) {
                        WatchAction::TurnCap
                    } else {
                        WatchAction::Phase(deadline.phase)
                    });
                }
                if let Some((key, _)) = request {
                    state.requests.remove(&key);
                }
                return Some(WatchAction::RequestTimedOut);
            }
            let _ = self.changed.wait_until(&mut state, at);
        }
    }
}

pub(super) struct Shared {
    pub(super) limits: DriverLimits,
    ownership: WriterOwnership,
    redactor: Arc<Redactor>,
    queue: Arc<EventQueue>,
    child: Arc<Mutex<TrackedChild>>,
    core: Mutex<Core>,
    core_changed: Condvar,
    transcript: Mutex<Transcript>,
    stderr: Mutex<StderrState>,
    options: Mutex<AgentOptions>,
    option_ids: Mutex<OptionIds>,
    streams: Mutex<Streams>,
    ingress: Arc<Ingress>,
    late: AtomicU64,
    declined: AtomicU64,
    watchdog: Watchdog,
    forced_kill: AtomicBool,
    /// The adapter identified itself as the Claude ACP adapter at initialize.
    claude_adapter: AtomicBool,
}

/// Splits `text` into pieces of at most `max` bytes on char boundaries.
fn split_chunks(text: &str, max: usize) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let mut end = max.min(rest.len());
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        let (head, tail) = rest.split_at(end);
        rest = tail;
        Some(head)
    })
}

impl Shared {
    pub(super) fn phase(&self) -> Phase {
        self.core.lock().phase
    }

    fn set_phase(&self, phase: Phase) {
        self.core.lock().phase = phase;
    }

    /// Redacted, length-bounded copy of any wire string that is exported to hosts.
    fn clean(&self, raw: &str) -> String {
        truncate_label(&self.redactor.redact(raw))
    }

    // ---- stderr (incrementally sanitized) ------------------------------------------

    pub(super) fn stderr_push(&self, bytes: &[u8]) {
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
    pub(super) fn stderr_finish(&self) {
        let max = self.limits.max_stderr_bytes;
        let mut state = self.stderr.lock();
        let carry = std::mem::take(&mut state.carry);
        let mut released = state.redactor.push(&String::from_utf8_lossy(&carry));
        released.push_str(&state.redactor.finish());
        state.append(&released, max);
    }

    fn stderr_text(&self, last: usize) -> String {
        let state = self.stderr.lock();
        let skip = state.tail.len().saturating_sub(last);
        let bytes: Vec<u8> = state.tail.iter().skip(skip).copied().collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    // ---- events and failures -------------------------------------------------------

    /// Queues an event; false when it could not be queued (overflow or already closed).
    fn emit(&self, kind: AgentEventKind) -> bool {
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
    fn record_failure(&self, failure: AgentFailure) -> bool {
        let mut core = self.core.lock();
        if core.failure.is_some() {
            return false;
        }
        core.failure = Some(failure);
        self.core_changed.notify_all();
        true
    }

    fn sanitize(&self, mut failure: AgentFailure) -> AgentFailure {
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
    pub(super) fn fail(&self, failure: AgentFailure) {
        let failure = self.sanitize(failure);
        if self.record_failure(failure.clone()) {
            // The event goes in before the kill so consumers see cause before effect.
            let _ = self.queue.push(AgentEventKind::Failure(failure));
        }
        self.request_kill();
    }

    fn request_kill(&self) {
        self.watchdog.request_kill();
    }

    fn note_escapes(&self, pids: Vec<u32>) {
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
    fn note_scan(&self, scan: EscapeScan) {
        match scan {
            EscapeScan::Clean => {}
            EscapeScan::Escaped(pids) => self.note_escapes(pids),
            EscapeScan::Unverifiable(reason) => {
                self.note_unverified(&format!("escape scan failed: {reason}"));
            }
        }
    }

    /// Keeps the first reason containment could not be verified; ownership is demoted.
    fn note_unverified(&self, reason: &str) {
        let reason = self.clean(reason);
        let mut core = self.core.lock();
        if core.containment_unverified.is_none() {
            core.containment_unverified = Some(reason);
        }
    }

    /// Forced stop of the owned tree. Runs on the reaper thread or in `stop()`, never on
    /// a handle caller's thread. Escapes are recorded first: once the parent dies an
    /// escaped helper is reparented and can no longer be attributed.
    fn kill_now(&self) {
        let scan = self.child.lock().escaped_descendants();
        self.note_scan(scan);
        self.close_permissions();
        self.forced_kill.store(true, Ordering::SeqCst);
        let _ = self.child.lock().kill_forcefully();
    }

    fn is_stopping(&self) -> bool {
        self.core.lock().stopping
    }

    // ---- permissions ---------------------------------------------------------------

    fn push_permission_closed(&self, ids: &[PermissionId]) {
        for id in ids {
            let _ = self.queue.push(AgentEventKind::PermissionClosed {
                id: *id,
                resolution: PermissionResolution::Cancelled,
            });
        }
    }

    /// Closes every open permission request as cancelled; returns how many were open.
    fn close_permissions(&self) -> u32 {
        let closed = self.core.lock().drain_permissions();
        self.push_permission_closed(&closed);
        closed.len() as u32
    }

    fn on_permission_request(
        &self,
        request: RequestPermissionRequest,
        responder: Responder<RequestPermissionResponse>,
    ) -> Result<(), RpcError> {
        let cancel = |responder: Responder<RequestPermissionResponse>| {
            let _ = quiet(|| {
                responder.respond(RequestPermissionResponse::new(
                    RequestPermissionOutcome::Cancelled,
                ))
            });
        };
        self.flush_before_event();
        let wire_id = responder.id().to_string();
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
        // One critical section: session check, cancel latch, duplicate check and
        // insertion cannot interleave with Stop, a grant, or turn completion.
        let mut core = self.core.lock();
        if core.session.as_deref() != Some(&*request.session_id.0) {
            drop(core);
            cancel(responder);
            return Err(self.protocol_violation("permission request for another session"));
        }
        let active = match (&core.turn, core.stopping) {
            (Some(turn), false) if !turn.cancel_requested => Some(turn.id),
            _ => None,
        };
        let Some(turn) = active else {
            // No active, uncancelled turn: such a request can never be granted.
            drop(core);
            self.late.fetch_add(1, Ordering::SeqCst);
            cancel(responder);
            return Ok(());
        };
        if core.permissions.wire_ids.contains(&wire_id) {
            drop(core);
            cancel(responder);
            return Err(self.protocol_violation("duplicate in-flight permission request id"));
        }
        if core.permissions.pending.len() >= MAX_PENDING_PERMISSIONS {
            drop(core);
            cancel(responder);
            return Err(self.protocol_violation("too many pending permission requests"));
        }
        let id = core.permissions.next;
        core.permissions.next += 1;
        core.permissions.wire_ids.insert(wire_id.clone());
        core.permissions.pending.insert(
            id,
            PendingPermission {
                responder,
                options,
                wire_id,
            },
        );
        drop(core);
        // An admitted request for the active turn is agent activity.
        self.note_turn_activity();
        self.emit(AgentEventKind::PermissionRequested(PermissionPrompt {
            id: PermissionId(id),
            turn,
            tool_call_id,
            title,
            options: choices,
        }));
        Ok(())
    }

    /// Exported (redacted, collision-free) copy of a wire id.
    fn export_id(&self, raw: &str, taken: &mut HashMap<String, String>) -> String {
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

    fn reply_permission(
        &self,
        id: PermissionId,
        reply: PermissionReply,
    ) -> Result<(), DriverError> {
        // Lookup, validation, removal and the wire reply are one step under `core`, so a
        // concurrent Stop either cancels this request first (this call then fails) or
        // observes it already answered.
        let (resolution, failed) = {
            let mut core = self.core.lock();
            if let Some(failure) = &core.failure {
                return Err(DriverError::Failed(failure.clone()));
            }
            let Some(pending) = core.permissions.pending.get(&id.0) else {
                return Err(DriverError::UnknownPermission(id));
            };
            let (outcome, resolution) = match &reply {
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
                    (
                        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                            raw.clone(),
                        )),
                        PermissionResolution::Selected {
                            option_id: exported.clone(),
                        },
                    )
                }
                PermissionReply::Cancel => (
                    RequestPermissionOutcome::Cancelled,
                    PermissionResolution::Cancelled,
                ),
            };
            let pending = core
                .permissions
                .pending
                .remove(&id.0)
                .expect("checked above");
            core.permissions.wire_ids.remove(&pending.wire_id);
            let failed = quiet(|| {
                pending
                    .responder
                    .respond(RequestPermissionResponse::new(outcome))
            })
            .err();
            (resolution, failed)
        };
        if let Some(error) = failed {
            self.fail(self.rpc_failure(Phase::Prompt, error));
            return Err(DriverError::Closed);
        }
        self.emit(AgentEventKind::PermissionClosed { id, resolution });
        Ok(())
    }

    // ---- session updates -----------------------------------------------------------

    fn protocol_violation(&self, message: &str) -> RpcError {
        self.fail(AgentFailure::new(
            FailureKind::ProtocolViolation,
            self.phase(),
            message,
        ));
        RpcError::internal_error().data(message.to_owned())
    }

    fn late(&self) -> Result<(), RpcError> {
        self.late.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// The adapter is still working on the turn: its prompt deadline starts over.
    fn note_turn_activity(&self) {
        self.watchdog
            .extend(Phase::Prompt, self.limits.prompt_timeout);
    }

    fn on_update(&self, notification: SessionNotification) -> Result<(), RpcError> {
        let (matches, in_turn, restoring, stopping) = {
            let core = self.core.lock();
            (
                core.session.as_deref() == Some(&*notification.session_id.0),
                core.turn.is_some(),
                core.restoring,
                core.stopping,
            )
        };
        if !matches {
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
            // Plans, command lists, usage and session info carry no workflow authority.
            _ => Ok(()),
        }
    }

    fn on_message(
        &self,
        role: MessageRole,
        content: &ContentBlock,
        in_turn: bool,
        restoring: bool,
    ) -> Result<(), RpcError> {
        if !in_turn && !restoring {
            return self.late();
        }
        let text = match content {
            ContentBlock::Text(text) => text.text.clone(),
            _ => "[non-text content omitted]".to_owned(),
        };
        // Keep roles chronological: text held for another role is released first.
        for other in ROLES {
            if other != role {
                let held = self.streams.lock().get(other).flush_at_boundary();
                self.release_text(other, held, restoring);
            }
        }
        let released = self.streams.lock().get(role).push(&text);
        self.release_text(role, released, restoring);
        Ok(())
    }

    fn release_text(&self, role: MessageRole, text: String, restoring: bool) {
        if text.is_empty() {
            return;
        }
        self.transcript.lock().append(role, &text);
        // Replayed history is available through transcript pages, not live events.
        if restoring {
            return;
        }
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
    fn flush_before_event(&self) {
        for role in ROLES {
            let held = self.streams.lock().get(role).flush_at_boundary();
            self.release_text(role, held, false);
        }
    }

    /// Turn/restore boundary: a message never continues into the next turn or the replay
    /// into the first live turn, so a held secret prefix cannot be completed any more and
    /// is released with its own turn instead of leaking into the next one.
    fn finish_streams_at_turn_end(&self, restoring: bool) {
        for role in ROLES {
            let rest = self.streams.lock().get(role).finish();
            self.release_text(role, rest, restoring);
        }
    }

    /// True end of every stream: nothing more can ever complete a held prefix.
    fn finish_streams(&self) {
        self.finish_streams_at_turn_end(false);
    }

    // ---- options -------------------------------------------------------------------

    fn apply_modes(&self, modes: &SessionModeState) {
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

    fn apply_config(&self, config: &[SessionConfigOption]) {
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

    fn config_value(
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
    fn resolve_option(
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

    fn resolve_mode(&self, mode: &str) -> Result<String, DriverError> {
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

    fn on_option_result(
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
    fn latch_cancel(&self, for_stop: bool) -> Result<Option<u64>, DriverError> {
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
    fn take_cancel_to_send(&self) -> Option<String> {
        let mut core = self.core.lock();
        let turn = core.cancel_to_send.take()?;
        if core.turn.as_ref().map(|t| t.id) == Some(turn) {
            core.session.clone()
        } else {
            None
        }
    }

    fn on_prompt_result(&self, turn_id: u64, result: Result<PromptResponse, RpcError>) {
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
        self.finish_streams_at_turn_end(false);
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

    fn rpc_failure(&self, phase: Phase, error: RpcError) -> AgentFailure {
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

    fn exit_code(&self, within: Duration) -> Option<i32> {
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

    fn process_exit_failure(&self) -> AgentFailure {
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

    fn set_ready(&self) {
        let mut core = self.core.lock();
        if core.status == DriverStatus::Starting {
            core.status = DriverStatus::Ready;
        }
        core.phase = Phase::Idle;
        self.core_changed.notify_all();
    }

    /// Verified teardown of the owned tree; safe to call repeatedly.
    fn teardown(&self) -> TerminationReport {
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
    fn snapshot_outcome(
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
    fn publish_closed(&self, outcome: DriverOutcome) -> DriverOutcome {
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

    fn finish(&self) {
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

fn enum_label<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "other".to_owned())
}

fn tool_status(status: ToolCallStatus) -> ToolStatus {
    match status {
        ToolCallStatus::Pending => ToolStatus::Pending,
        ToolCallStatus::InProgress => ToolStatus::InProgress,
        ToolCallStatus::Completed => ToolStatus::Completed,
        _ => ToolStatus::Failed,
    }
}

// ---- connection task --------------------------------------------------------------------

struct DriveParams {
    cwd: PathBuf,
    mode: DriverMode,
    resume: Option<String>,
    auth_method: Option<String>,
    mcp_servers: Vec<McpServer>,
    mcp_stdio: bool,
}

enum Plan {
    New,
    Resume(String),
    Load(String),
}

/// Length of the standard base64 encoding of `bytes` bytes (with padding).
fn base64_len(bytes: usize) -> Option<usize> {
    bytes.div_ceil(3).checked_mul(4)
}

/// Counts serialized bytes without buffering them.
struct ByteCount(usize);

impl std::io::Write for ByteCount {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 += buf.len();
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn json_len(value: &impl serde::Serialize) -> Option<usize> {
    let mut count = ByteCount(0);
    serde_json::to_writer(&mut count, value).ok()?;
    Some(count.0)
}

/// Bytes of the line (without its newline) the SDK writes for `request`: the JSON-RPC
/// envelope it builds around the serialized params. The SDK's request ids are always
/// UUID strings, so a placeholder of the same length gives the exact size. `usize::MAX`
/// when it cannot be serialized, which never fits.
fn prompt_line_bytes(request: &PromptRequest) -> usize {
    const SDK_REQUEST_ID: &str = "00000000-0000-0000-0000-000000000000";
    let envelope = RawJsonRpcMessage::request(
        request.method().to_owned(),
        serde_json::json!({}),
        RequestId::Str(SDK_REQUEST_ID.to_owned()),
    )
    .ok()
    .and_then(|message| json_len(&message));
    // The empty `{}` params in the envelope are replaced by the real ones.
    match (envelope, json_len(request)) {
        (Some(envelope), Some(params)) => (envelope - 2).saturating_add(params),
        _ => usize::MAX,
    }
}

fn wire_server(server: &McpStdioServer) -> McpServer {
    McpServer::Stdio(
        McpServerStdio::new(server.name.clone(), server.command.clone())
            .args(server.args.clone())
            .env(
                server
                    .env
                    .iter()
                    .map(|(name, value)| EnvVariable::new(name.clone(), value.clone()))
                    .collect(),
            ),
    )
}

/// Whether a prompt error is the Claude adapter reporting a turn that ended with only
/// server-side tool calls (`srvtoolu_` ids: tools the model API runs itself, such as the
/// advisor) left without a result. The client could not have supplied one, and the adapter
/// has already closed those calls as failed. The adapter tags this error with
/// `data.errorKind = "incomplete_tool_call"` and lists every unfinished id after the
/// message prefix; any other id, a missing tag or another code is a real failure.
fn only_server_tools_unfinished(error: &RpcError) -> bool {
    const MARKER: &str = "Claude ended the turn without returning results for tool calls: ";
    const SERVER_TOOL: &str = "srvtoolu_";
    if i32::from(error.code) != i32::from(agent_client_protocol::ErrorCode::InternalError) {
        return false;
    }
    let tagged = error
        .data
        .as_ref()
        .and_then(|data| data.get("errorKind"))
        .and_then(serde_json::Value::as_str)
        == Some("incomplete_tool_call");
    if !tagged {
        return false;
    }
    let Some((_, ids)) = error.message.split_once(MARKER) else {
        return false;
    };
    let mut ids = ids
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|id| !id.is_empty())
        .peekable();
    ids.peek().is_some()
        && ids.all(|id| id.len() > SERVER_TOOL.len() && id.starts_with(SERVER_TOOL))
}

fn initialized_info(init: &InitializeResponse, mcp_stdio: bool) -> InitializedInfo {
    let caps = &init.agent_capabilities;
    InitializedInfo {
        agent_name: init
            .agent_info
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), |i| i.name.clone()),
        agent_version: init
            .agent_info
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), |i| i.version.clone()),
        protocol_version: 1,
        capabilities: AgentCapabilityInfo {
            load_session: caps.load_session,
            resume_session: caps.session_capabilities.resume.is_some(),
            prompt_image: caps.prompt_capabilities.image,
            prompt_audio: caps.prompt_capabilities.audio,
            prompt_embedded_context: caps.prompt_capabilities.embedded_context,
            mcp_http: caps.mcp_capabilities.http,
            mcp_sse: caps.mcp_capabilities.sse,
            mcp_stdio,
        },
        auth_methods: init
            .auth_methods
            .iter()
            .map(|m| m.id().0.to_string())
            .collect(),
        authenticated: false,
    }
}

type SessionReply = Result<
    (
        Option<String>,
        Option<SessionModeState>,
        Option<Vec<SessionConfigOption>>,
    ),
    RpcError,
>;

async fn drive(
    shared: Arc<Shared>,
    cx: ConnectionTo<Agent>,
    mut commands: mpsc::Receiver<Command>,
    params: DriveParams,
) -> Result<(), AgentFailure> {
    shared.set_phase(Phase::Initialize);
    let init = cx
        .send_request(
            InitializeRequest::new(ProtocolVersion::V1)
                // Optional client services (fs, terminal) are declined by omission.
                .client_capabilities(ClientCapabilities::new())
                .client_info(Implementation::new(CLIENT_NAME, env!("CARGO_PKG_VERSION"))),
        )
        .block_task()
        .await
        .map_err(|e| shared.rpc_failure(Phase::Initialize, e))?;
    if init.protocol_version != ProtocolVersion::V1 {
        return Err(AgentFailure::new(
            FailureKind::UnsupportedVersion,
            Phase::Initialize,
            format!(
                "adapter negotiated ACP protocol version {}; only version 1 is supported",
                init.protocol_version
            ),
        ));
    }
    let raw_info = initialized_info(&init, params.mcp_stdio);
    shared.claude_adapter.store(
        CLAUDE_ADAPTER_NAMES.contains(&raw_info.agent_name.as_str()),
        Ordering::SeqCst,
    );
    // Wire metadata is adapter-controlled text: only a sanitized copy is exported.
    let mut info = raw_info.clone();
    info.agent_name = shared.clean(&raw_info.agent_name);
    info.agent_version = shared.clean(&raw_info.agent_version);
    info.auth_methods = raw_info
        .auth_methods
        .iter()
        .map(|m| shared.clean(m))
        .collect();
    shared.core.lock().initialized = Some(info.clone());
    shared.emit(AgentEventKind::Initialized(info.clone()));
    if params.mode == DriverMode::InitializeOnly {
        shared.watchdog.disarm();
        shared.set_ready();
        return command_loop(&shared, &cx, &mut commands).await;
    }
    if let Some(method) = &params.auth_method {
        shared.set_phase(Phase::Authenticate);
        if !raw_info.auth_methods.contains(method) {
            return Err(AgentFailure::new(
                FailureKind::AuthRejected,
                Phase::Authenticate,
                format!(
                    "authentication method '{}' is not advertised by the adapter",
                    shared.clean(method)
                ),
            ));
        }
        cx.send_request(AuthenticateRequest::new(method.clone()))
            .block_task()
            .await
            .map_err(|e| shared.rpc_failure(Phase::Authenticate, e))?;
    }
    shared.set_phase(Phase::Session);
    let plan = match &params.resume {
        None => Plan::New,
        Some(id) if info.capabilities.resume_session => Plan::Resume(id.clone()),
        Some(id) if info.capabilities.load_session => Plan::Load(id.clone()),
        Some(_) => {
            return Err(AgentFailure::new(
                FailureKind::RestoreUnsupported,
                Phase::Session,
                "adapter did not negotiate session restoration; start a new session instead",
            ));
        }
    };
    let (done_tx, done_rx) = futures::channel::oneshot::channel::<SessionReply>();
    let restored = !matches!(plan, Plan::New);
    match plan {
        Plan::New => {
            cx.send_request(
                NewSessionRequest::new(params.cwd.clone()).mcp_servers(params.mcp_servers.clone()),
            )
            .on_receiving_result(move |result| {
                let _ =
                    done_tx
                        .send(result.map(|r| {
                            (Some(r.session_id.0.to_string()), r.modes, r.config_options)
                        }));
                async { Ok(()) }
            })
            .map_err(|e| shared.rpc_failure(Phase::Session, e))?;
        }
        Plan::Resume(id) => {
            shared.core.lock().session = Some(id.clone());
            cx.send_request(
                ResumeSessionRequest::new(id, params.cwd.clone())
                    .mcp_servers(params.mcp_servers.clone()),
            )
            .on_receiving_result(move |result| {
                let _ = done_tx.send(result.map(|r| (None, r.modes, r.config_options)));
                async { Ok(()) }
            })
            .map_err(|e| shared.rpc_failure(Phase::Session, e))?;
        }
        Plan::Load(id) => {
            {
                let mut core = shared.core.lock();
                core.session = Some(id.clone());
                core.restoring = true;
            }
            cx.send_request(
                LoadSessionRequest::new(id, params.cwd.clone())
                    .mcp_servers(params.mcp_servers.clone()),
            )
            .on_receiving_result(move |result| {
                let _ = done_tx.send(result.map(|r| (None, r.modes, r.config_options)));
                async { Ok(()) }
            })
            .map_err(|e| shared.rpc_failure(Phase::Session, e))?;
        }
    }
    let reply = done_rx.await.map_err(|_| {
        AgentFailure::new(
            FailureKind::SessionRejected,
            Phase::Session,
            "session request was dropped before a response",
        )
    })?;
    let (new_id, modes, config) = reply.map_err(|e| shared.rpc_failure(Phase::Session, e))?;
    shared.finish_streams_at_turn_end(true);
    {
        let mut core = shared.core.lock();
        if let Some(id) = new_id {
            core.session = Some(id);
        }
        core.restoring = false;
        core.restored = restored;
    }
    let session_id = shared
        .core
        .lock()
        .session
        .clone()
        .expect("session set by plan");
    // Hosts only ever see the sanitized id; the wire id stays inside `core`.
    let exported_session = shared.clean(&session_id);
    shared.queue.set_session(&exported_session);
    if let Some(modes) = &modes {
        shared.apply_modes(modes);
    }
    if let Some(config) = &config {
        shared.apply_config(config);
    }
    shared.emit(AgentEventKind::SessionReady {
        session_id: exported_session,
        restored,
    });
    let snapshot = shared.options.lock().clone();
    if !snapshot.modes.is_empty() || !snapshot.config.is_empty() {
        shared.emit(AgentEventKind::Options(snapshot));
    }
    shared.watchdog.disarm();
    shared.set_ready();
    command_loop(&shared, &cx, &mut commands).await
}

async fn command_loop(
    shared: &Arc<Shared>,
    cx: &ConnectionTo<Agent>,
    commands: &mut mpsc::Receiver<Command>,
) -> Result<(), AgentFailure> {
    let internal = |shared: &Shared, error: RpcError| shared.rpc_failure(shared.phase(), error);
    loop {
        // A latched Stop is written here, bound to the turn that was stopped.
        if let Some(session) = shared.take_cancel_to_send() {
            cx.send_notification(CancelNotification::new(session))
                .map_err(|e| internal(shared, e))?;
        }
        let incoming = std::pin::pin!(cx.incoming_closed());
        let next = std::pin::pin!(commands.next());
        match futures::future::select(next, incoming).await {
            Either::Left((None, _)) => return Ok(()),
            Either::Right(_) => {
                return if shared.is_stopping() {
                    Ok(())
                } else {
                    Err(shared.process_exit_failure())
                };
            }
            Either::Left((Some(command), _)) => match command {
                Command::Wake => {}
                Command::Prompt { turn, content } => {
                    let session = shared.core.lock().session.clone().ok_or_else(|| {
                        AgentFailure::new(FailureKind::Internal, Phase::Idle, "no session")
                    })?;
                    let sh = shared.clone();
                    cx.send_request(PromptRequest::new(session, content))
                        // Runs in dispatch order, so updates after the response are late.
                        .on_receiving_result(move |result| {
                            sh.on_prompt_result(turn, result);
                            async { Ok(()) }
                        })
                        .map_err(|e| internal(shared, e))?;
                }
                Command::SetMode { raw, label, key } => {
                    let session = shared.core.lock().session.clone().unwrap_or_default();
                    let sh = shared.clone();
                    cx.send_request(SetSessionModeRequest::new(session, raw))
                        .on_receiving_result(move |result| {
                            sh.on_option_result(&label, key, result.map(|_| None), true);
                            async { Ok(()) }
                        })
                        .map_err(|e| internal(shared, e))?;
                }
                Command::SetConfig {
                    raw_id,
                    label,
                    value,
                    key,
                } => {
                    let session = shared.core.lock().session.clone().unwrap_or_default();
                    let sh = shared.clone();
                    let wire = match value {
                        OptionValue::Value(v) => SessionConfigOptionValue::value_id(v),
                        OptionValue::Boolean(b) => SessionConfigOptionValue::boolean(b),
                    };
                    cx.send_request(SetSessionConfigOptionRequest::new(session, raw_id, wire))
                        .on_receiving_result(move |result| {
                            sh.on_option_result(
                                &label,
                                key,
                                result.map(|r| Some(r.config_options)),
                                false,
                            );
                            async { Ok(()) }
                        })
                        .map_err(|e| internal(shared, e))?;
                }
                Command::Shutdown => {
                    // Stop latches cancellation itself; deliver the notification best effort.
                    let _ = shared.latch_cancel(true);
                    if let Some(session) = shared.take_cancel_to_send() {
                        let _ = cx.send_notification(CancelNotification::new(session));
                    }
                    shared.close_permissions();
                    return Ok(());
                }
            },
        }
    }
}

fn run_connection(
    shared: Arc<Shared>,
    sink: LineSink,
    stream: LineStream,
    commands: mpsc::Receiver<Command>,
    params: DriveParams,
) {
    let notify_shared = shared.clone();
    let permission_shared = shared.clone();
    let decline_shared = shared.clone();
    let ingress_shared = shared.clone();
    let drive_shared = shared.clone();
    // SDK diagnostics trace whole wire lines (including outbound MCP environment values
    // and inbound text) before any application redaction. Nothing the SDK logs from this
    // connection may reach a host subscriber, so the connection thread runs under a
    // scoped null subscriber.
    let result =
        tracing::subscriber::with_default(tracing::subscriber::NoSubscriber::default(), || {
            futures::executor::block_on(
                Client
                    .builder()
                    .name(CLIENT_NAME)
                    // First in the chain, so it observes every dispatched message in arrival
                    // order and returns the ingress credit its frame was admitted with.
                    .on_receive_dispatch(
                        async move |dispatch: Dispatch, _cx| {
                            ingress_shared.ingress.release_one();
                            Ok(Handled::No {
                                message: dispatch,
                                retry: false,
                            })
                        },
                        agent_client_protocol::on_receive_dispatch!(),
                    )
                    .on_receive_notification(
                        async move |notification: SessionNotification, _cx| {
                            notify_shared.on_update(notification)
                        },
                        agent_client_protocol::on_receive_notification!(),
                    )
                    .on_receive_request(
                        async move |request: RequestPermissionRequest, responder, _cx| {
                            permission_shared.on_permission_request(request, responder)
                        },
                        agent_client_protocol::on_receive_request!(),
                    )
                    // Optional client services (fs, terminal, ...) are not advertised. The
                    // SDK would leave such a request unanswered, hanging the adapter, so
                    // every remaining request is declined explicitly.
                    .on_receive_dispatch(
                        async move |dispatch: Dispatch, _cx| match dispatch {
                            Dispatch::Request(message, responder) => {
                                decline_shared.declined.fetch_add(1, Ordering::SeqCst);
                                responder
                                    .respond_with_error(
                                        RpcError::method_not_found()
                                            .data(message.method().to_owned()),
                                    )
                                    .map(|()| Handled::Yes)
                            }
                            other => Ok(Handled::No {
                                message: other,
                                retry: false,
                            }),
                        },
                        agent_client_protocol::on_receive_dispatch!(),
                    )
                    .connect_with(
                        Lines::new(sink, stream),
                        async move |cx: ConnectionTo<Agent>| match drive(
                            drive_shared.clone(),
                            cx,
                            commands,
                            params,
                        )
                        .await
                        {
                            Ok(()) => Ok(()),
                            Err(failure) => {
                                let message = failure.message.clone();
                                drive_shared.fail(failure);
                                Err(RpcError::internal_error().data(message))
                            }
                        },
                    ),
            )
        });
    if let Err(error) = result {
        let recorded = shared.core.lock().failure.is_some();
        if !recorded && !shared.is_stopping() {
            let failure = if agent_client_protocol::is_incoming_transport_closed(&error) {
                shared.process_exit_failure()
            } else {
                shared.rpc_failure(shared.phase(), error)
            };
            shared.fail(failure);
        }
    } else if !shared.is_stopping() && shared.core.lock().failure.is_none() {
        // The command channel closed without a stop request: the owner is gone.
        shared.core.lock().stopping = true;
    }
    shared.finish();
}

/// Watchdog and reaper thread: expires deadlines and performs failure-triggered kills so
/// that no handle method ever waits on process cleanup.
fn run_watchdog(shared: Arc<Shared>) {
    while let Some(action) = shared.watchdog.next_action() {
        match action {
            WatchAction::Kill => shared.kill_now(),
            WatchAction::Phase(Phase::Prompt) => shared.fail(AgentFailure::new(
                FailureKind::DeadlineExceeded,
                Phase::Prompt,
                format!(
                    "the agent showed no activity for {:?} during the turn (idle timeout)",
                    shared.limits.prompt_timeout
                ),
            )),
            WatchAction::TurnCap => shared.fail(AgentFailure::new(
                FailureKind::DeadlineExceeded,
                Phase::Prompt,
                format!(
                    "the turn ran longer than the {:?} turn limit",
                    shared.limits.max_turn
                ),
            )),
            WatchAction::Phase(expired) => shared.fail(AgentFailure::new(
                FailureKind::DeadlineExceeded,
                expired,
                format!("{expired:?} deadline exceeded"),
            )),
            WatchAction::RequestTimedOut => shared.fail(AgentFailure::new(
                FailureKind::DeadlineExceeded,
                Phase::Idle,
                format!(
                    "the adapter did not answer a mode/config request within {:?}",
                    shared.limits.request_timeout
                ),
            )),
        }
    }
}

// ---- host handle ----------------------------------------------------------------------------

/// Owner of one ACP v1 adapter process and session. Dropping stops it.
pub struct AcpDriver {
    shared: Arc<Shared>,
    commands: Mutex<Option<mpsc::Sender<Command>>>,
    main: Option<JoinHandle<()>>,
    watchdog: Option<JoinHandle<()>>,
    io: Vec<JoinHandle<()>>,
}

impl AcpDriver {
    /// Spawns the adapter and starts the session handshake in the background.
    /// Observe progress with [`Self::wait_ready`] and the event queue.
    pub fn start(
        config: DriverConfig,
        processes: &ProcessTreeManager,
    ) -> Result<Self, AgentFailure> {
        let DriverConfig {
            provider,
            task,
            cwd,
            launch,
            limits,
            resume_session,
            writer_ownership,
            mode,
            mcp_servers,
            mcp_stdio,
        } = config;
        for server in &mcp_servers {
            server.validate().map_err(|error| {
                AgentFailure::new(
                    FailureKind::SpawnFailed,
                    Phase::Spawn,
                    format!("invalid MCP server configuration: {error}"),
                )
            })?;
        }
        let redactor = Arc::new(Redactor::new(
            launch.secrets.iter().cloned().chain(
                mcp_servers
                    .iter()
                    .flat_map(|s| s.secrets().map(str::to_owned)),
            ),
        ));
        if !cwd.is_absolute() || !cwd.is_dir() {
            return Err(AgentFailure::new(
                FailureKind::SpawnFailed,
                Phase::Spawn,
                "session working directory must be an existing absolute directory",
            ));
        }
        let child = AgentSupervisor::new(processes.clone())
            .spawn_adapter(
                &launch.executable,
                &launch.args,
                &cwd,
                Some(launch.env.clone()),
            )
            .map_err(|error| spawn_failure(&launch, error, &redactor))?;
        let (stdin, stdout, stderr) = {
            let mut child = child.lock();
            let process = child.child_mut();
            (
                process.stdin.take(),
                process.stdout.take(),
                process.stderr.take(),
            )
        };
        let (Some(stdin), Some(stdout), Some(stderr)) = (stdin, stdout, stderr) else {
            let _ = child.lock().kill_forcefully();
            return Err(AgentFailure::new(
                FailureKind::SpawnFailed,
                Phase::Spawn,
                "adapter pipes were not available",
            ));
        };
        let queue = EventQueue::new(provider, task, limits.max_queued_events);
        let ingress = Ingress::new(INGRESS_MAX_FRAMES, INGRESS_MAX_BYTES);
        let shared = Arc::new(Shared {
            ownership: writer_ownership,
            streams: Mutex::new(Streams {
                agent: StreamRedactor::new(redactor.clone()),
                thought: StreamRedactor::new(redactor.clone()),
                user: StreamRedactor::new(redactor.clone()),
            }),
            stderr: Mutex::new(StderrState {
                redactor: StreamRedactor::new(redactor.clone()),
                carry: Vec::new(),
                tail: VecDeque::new(),
            }),
            redactor,
            queue,
            child,
            core: Mutex::new(Core {
                status: DriverStatus::Starting,
                phase: Phase::Initialize,
                session: None,
                restored: false,
                restoring: false,
                initialized: None,
                turn: None,
                next_turn: 1,
                last_outcome: None,
                failure: None,
                stopping: false,
                escaped: Vec::new(),
                containment_unverified: None,
                outcome: None,
                permissions: Permissions::default(),
                cancel_to_send: None,
            }),
            core_changed: Condvar::new(),
            transcript: Mutex::new(Transcript::new(limits.max_transcript_bytes)),
            options: Mutex::new(AgentOptions::default()),
            option_ids: Mutex::new(OptionIds::default()),
            ingress: ingress.clone(),
            late: AtomicU64::new(0),
            declined: AtomicU64::new(0),
            watchdog: Watchdog::new(),
            forced_kill: AtomicBool::new(false),
            claude_adapter: AtomicBool::new(false),
            limits,
        });
        shared
            .watchdog
            .arm(Phase::Initialize, shared.limits.init_timeout);
        let (line_tx, line_rx) = mpsc::channel(32);
        let mut io = vec![
            transport::spawn_reader(shared.clone(), stdout, line_tx, ingress),
            transport::spawn_stderr(shared.clone(), stderr),
        ];
        let (sink, writer) = transport::spawn_writer(shared.clone(), stdin);
        io.push(writer);
        let (command_tx, command_rx) = mpsc::channel(COMMAND_QUEUE);
        let params = DriveParams {
            cwd,
            mode,
            resume: resume_session,
            auth_method: launch.auth_method.clone(),
            mcp_servers: match mcp_stdio {
                McpStdioSupport::Baseline => mcp_servers.iter().map(wire_server).collect(),
                McpStdioSupport::Unsupported => Vec::new(),
            },
            mcp_stdio: mcp_stdio == McpStdioSupport::Baseline,
        };
        let watchdog = {
            let shared = shared.clone();
            std::thread::Builder::new()
                .name("acp-watchdog".into())
                .spawn(move || run_watchdog(shared))
                .expect("spawn acp watchdog")
        };
        let main = {
            let shared = shared.clone();
            std::thread::Builder::new()
                .name("acp-connection".into())
                .spawn(move || run_connection(shared, sink, Box::pin(line_rx), command_rx, params))
                .expect("spawn acp connection")
        };
        Ok(Self {
            shared,
            commands: Mutex::new(Some(command_tx)),
            main: Some(main),
            watchdog: Some(watchdog),
            io,
        })
    }

    pub fn provider(&self) -> &str {
        self.shared.queue.provider()
    }

    pub fn task(&self) -> &str {
        self.shared.queue.task()
    }

    /// Sanitized provider session id (secrets and token patterns redacted).
    pub fn session_id(&self) -> Option<String> {
        let raw = self.shared.core.lock().session.clone()?;
        Some(self.shared.clean(&raw))
    }

    /// The exact wire session id for restoration, but only when it carries no secret;
    /// an id that required redaction is never exposed.
    pub fn wire_session_id(&self) -> Option<String> {
        let raw = self.shared.core.lock().session.clone()?;
        (self.shared.redactor.redact(&raw) == raw).then_some(raw)
    }

    pub fn status(&self) -> DriverStatus {
        self.shared.core.lock().status
    }

    pub fn initialized(&self) -> Option<InitializedInfo> {
        self.shared.core.lock().initialized.clone()
    }

    pub fn failure(&self) -> Option<AgentFailure> {
        self.shared.core.lock().failure.clone()
    }

    pub fn options(&self) -> AgentOptions {
        self.shared.options.lock().clone()
    }

    pub fn late_events_rejected(&self) -> u64 {
        self.shared.late.load(Ordering::SeqCst)
    }

    /// Requests for client services that were not advertised and were declined.
    pub fn declined_requests(&self) -> u64 {
        self.shared.declined.load(Ordering::SeqCst)
    }

    /// Adapter frames admitted but not yet dispatched by the SDK connection.
    pub fn ingress_in_flight(&self) -> usize {
        self.shared.ingress.in_flight()
    }

    /// Mode/config requests currently awaiting an adapter reply.
    pub fn outstanding_requests(&self) -> usize {
        self.shared.watchdog.outstanding()
    }

    pub fn last_prompt(&self) -> Option<PromptOutcome> {
        self.shared.core.lock().last_outcome.clone()
    }

    /// Redacted stderr tail. Only text that already passed redaction is retained, so
    /// polling between fragments of a secret never exposes a prefix of it.
    pub fn stderr_tail(&self) -> String {
        self.shared.stderr_text(self.shared.limits.max_stderr_bytes)
    }

    pub fn transcript_page(&self, from: u64, limit: usize) -> TranscriptPage {
        self.shared.transcript.lock().page(from, limit)
    }

    pub fn transcript_resident_bytes(&self) -> usize {
        self.shared.transcript.lock().resident_bytes()
    }

    pub fn next_event(&self, timeout: Duration) -> Option<AgentEvent> {
        self.shared.queue.next(timeout)
    }

    pub fn try_next_event(&self) -> Option<AgentEvent> {
        self.shared.queue.try_next()
    }

    pub fn drain_events(&self, max: usize) -> Vec<AgentEvent> {
        self.shared.queue.drain(max)
    }

    pub fn queued_events(&self) -> usize {
        self.shared.queue.len()
    }

    /// Blocks until the session (or, in `InitializeOnly` mode, initialization) is ready,
    /// the driver fails, or `timeout` passes.
    pub fn wait_ready(&self, timeout: Duration) -> Result<SessionInfo, AgentFailure> {
        let deadline = Instant::now() + timeout;
        let mut core = self.shared.core.lock();
        loop {
            if let Some(failure) = &core.failure {
                return Err(failure.clone());
            }
            if core.status == DriverStatus::Closed {
                return Err(AgentFailure::new(
                    FailureKind::ProcessExited,
                    core.phase,
                    "driver closed before it became ready",
                ));
            }
            if core.status != DriverStatus::Starting
                && let Some(initialized) = &core.initialized
            {
                let initialized = initialized.clone();
                let restored = core.restored;
                let session = core.session.clone();
                drop(core);
                return Ok(SessionInfo {
                    session_id: session.map(|s| self.shared.clean(&s)),
                    restored,
                    initialized,
                });
            }
            if self
                .shared
                .core_changed
                .wait_until(&mut core, deadline)
                .timed_out()
            {
                return Err(AgentFailure::new(
                    FailureKind::DeadlineExceeded,
                    core.phase,
                    "timed out waiting for the adapter to become ready",
                ));
            }
        }
    }

    /// Blocks until `turn` finishes, the driver fails, or `timeout` passes. A latched
    /// failure always wins over a recorded completion.
    pub fn wait_turn(&self, turn: u64, timeout: Duration) -> Result<PromptOutcome, AgentFailure> {
        let deadline = Instant::now() + timeout;
        let mut core = self.shared.core.lock();
        loop {
            if let Some(failure) = &core.failure {
                return Err(failure.clone());
            }
            if let Some(outcome) = &core.last_outcome
                && outcome.turn == turn
            {
                return Ok(outcome.clone());
            }
            if core.status == DriverStatus::Closed {
                return Err(AgentFailure::new(
                    FailureKind::ProcessExited,
                    core.phase,
                    "driver closed before the turn finished",
                ));
            }
            if self
                .shared
                .core_changed
                .wait_until(&mut core, deadline)
                .timed_out()
            {
                return Err(AgentFailure::new(
                    FailureKind::DeadlineExceeded,
                    core.phase,
                    "timed out waiting for the turn to finish",
                ));
            }
        }
    }

    fn send(&self, command: Command) -> Result<(), DriverError> {
        let mut guard = self.commands.lock();
        let sender = guard.as_mut().ok_or(DriverError::Closed)?;
        sender.try_send(command).map_err(|error| {
            if error.is_full() {
                DriverError::Busy("the adapter command queue is full")
            } else {
                DriverError::Closed
            }
        })
    }

    fn require_usable(&self) -> Result<(), DriverError> {
        let core = self.shared.core.lock();
        if let Some(failure) = &core.failure {
            return Err(DriverError::Failed(failure.clone()));
        }
        match core.status {
            DriverStatus::Starting => Err(DriverError::NotReady),
            DriverStatus::Stopping | DriverStatus::Closed => Err(DriverError::Closed),
            DriverStatus::Ready | DriverStatus::Prompting => Ok(()),
        }
    }

    /// Starts one text prompt turn; returns its turn id.
    pub fn prompt(&self, text: &str) -> Result<u64, DriverError> {
        self.prompt_with_images(text, Vec::new())
    }

    /// Largest total of raw PNG bytes that `count` images can carry next to `text` in one
    /// prompt, however the bytes are split between the images. The bound is the wire
    /// message limit (`DriverLimits::max_message_bytes`) after base64 and JSON framing.
    /// 0 when no image fits, `count` is 0 or above the image limit, or there is no session.
    pub fn prompt_image_budget(&self, text: &str, count: usize) -> usize {
        if count == 0 || count > MAX_PROMPT_IMAGE_COUNT {
            return 0;
        }
        let Some(session) = self.shared.core.lock().session.clone() else {
            return 0;
        };
        let mut content = vec![ContentBlock::Text(TextContent::new(text))];
        content.extend((0..count).map(|_| ContentBlock::Image(ImageContent::new("", "image/png"))));
        let framing = prompt_line_bytes(&PromptRequest::new(session, content));
        // Base64 room left on the line; each image rounds up to whole 4-byte groups, so
        // 4 * ceil(n / 3) <= (4n + 8) / 3 and the total n must satisfy 4n <= 3 * room - 8c.
        let max_line = self.shared.limits.max_message_bytes.saturating_sub(1);
        let room = max_line.saturating_sub(framing);
        room.saturating_mul(3).saturating_sub(8 * count) / 4
    }

    /// Starts a text prompt with bounded PNG evidence. Image content is rejected unless
    /// the adapter negotiated ACP's image prompt capability, and the whole request must
    /// fit one wire message (see [`Self::prompt_image_budget`]).
    pub fn prompt_with_images(
        &self,
        text: &str,
        images: Vec<PromptImage>,
    ) -> Result<u64, DriverError> {
        if text.trim().is_empty() {
            return Err(DriverError::InvalidPrompt("prompt is empty"));
        }
        if text.len() > MAX_PROMPT_BYTES {
            return Err(DriverError::InvalidPrompt("prompt exceeds the size limit"));
        }
        if images.len() > MAX_PROMPT_IMAGE_COUNT {
            return Err(DriverError::InvalidPrompt("too many prompt images"));
        }
        if images.iter().any(|image| image.data.is_empty()) {
            return Err(DriverError::InvalidPrompt("prompt image is empty"));
        }
        if images
            .iter()
            .any(|image| !image.data.starts_with(b"\x89PNG\r\n\x1a\n"))
        {
            return Err(DriverError::InvalidPrompt("prompt image is not a PNG"));
        }
        if !images.is_empty()
            && !self
                .initialized()
                .is_some_and(|info| info.capabilities.prompt_image)
        {
            return Err(DriverError::InvalidPrompt(
                "the adapter did not negotiate image prompts",
            ));
        }
        // The whole request must fit one wire message, or the transport would fail the
        // driver when it is written. Images are measured encoded before anything is
        // encoded, then the exact line is measured; a rejection changes no state.
        let max = self.shared.limits.max_message_bytes;
        let encoded = images.iter().try_fold(0usize, |total, image| {
            total.checked_add(base64_len(image.data.len())?)
        });
        if !encoded.is_some_and(|bytes| fits_wire(bytes, max)) {
            return Err(DriverError::InvalidPrompt(
                "prompt images exceed the message size limit",
            ));
        }
        let mut content = Vec::with_capacity(images.len() + 1);
        content.push(ContentBlock::Text(TextContent::new(text)));
        content.extend(images.into_iter().map(|image| {
            ContentBlock::Image(ImageContent::new(STANDARD.encode(image.data), "image/png"))
        }));
        // Without a session the status check below refuses the prompt anyway.
        let session = self.shared.core.lock().session.clone();
        if let Some(session) = session {
            let request = PromptRequest::new(session, content);
            if !fits_wire(prompt_line_bytes(&request), max) {
                return Err(DriverError::InvalidPrompt(
                    "prompt exceeds the message size limit",
                ));
            }
            content = request.prompt;
        }
        let turn = {
            let mut core = self.shared.core.lock();
            if let Some(failure) = &core.failure {
                return Err(DriverError::Failed(failure.clone()));
            }
            match core.status {
                DriverStatus::Ready if core.session.is_some() => {}
                DriverStatus::Prompting => return Err(DriverError::TurnInProgress),
                DriverStatus::Starting => return Err(DriverError::NotReady),
                _ => return Err(DriverError::Closed),
            }
            let id = core.next_turn;
            core.next_turn = id.checked_add(1).ok_or_else(|| {
                DriverError::Failed(AgentFailure::new(
                    FailureKind::CounterExhausted,
                    Phase::Prompt,
                    "turn counter exhausted",
                ))
            })?;
            core.turn = Some(Turn {
                id,
                cancel_requested: false,
            });
            core.status = DriverStatus::Prompting;
            core.phase = Phase::Prompt;
            id
        };
        self.shared.watchdog.arm_turn(
            self.shared.limits.prompt_timeout,
            self.shared.limits.max_turn,
        );
        if let Err(error) = self.send(Command::Prompt { turn, content }) {
            // Never leave a phantom turn that nothing will ever answer.
            let mut core = self.shared.core.lock();
            if core.turn.as_ref().map(|t| t.id) == Some(turn) {
                core.turn = None;
                if core.status == DriverStatus::Prompting {
                    core.status = DriverStatus::Ready;
                }
            }
            drop(core);
            self.shared.watchdog.disarm();
            return Err(error);
        }
        Ok(turn)
    }

    /// Stops the active turn. The cancellation is latched before this call returns: the
    /// turn can no longer complete as uncancelled and its permissions are closed. The
    /// adapter keeps running and must answer with an authoritative `cancelled`. Repeated
    /// calls coalesce into one turn-bound notification.
    pub fn cancel_prompt(&self) -> Result<(), DriverError> {
        if self.shared.latch_cancel(false)?.is_some() {
            // The notification itself rides the latched state; a full queue only delays
            // the wake-up because queued commands wake the loop anyway.
            let _ = self.send(Command::Wake);
        }
        Ok(())
    }

    /// Writer-containment qualification only: kills the adapter's own process (not its
    /// tree) through the handle it was started with, as a provider crash would. The
    /// driver then observes the closed transport like any crash.
    pub fn crash_adapter_for_qualification(&self) -> std::io::Result<()> {
        self.shared.child.lock().child_mut().kill()
    }

    /// Processes the adapter started in its task scope so far (`None` where the platform
    /// does not record them or the record is incomplete).
    pub fn observed_descendants(&self) -> Option<usize> {
        self.shared.child.lock().observed_descendants()
    }

    /// Answers one pending permission request. Duplicate, late or unknown ids fail.
    pub fn reply_permission(
        &self,
        id: PermissionId,
        reply: PermissionReply,
    ) -> Result<(), DriverError> {
        self.shared.reply_permission(id, reply)
    }

    pub fn set_mode(&self, mode_id: &str) -> Result<(), DriverError> {
        self.require_usable()?;
        let raw = self.shared.resolve_mode(mode_id)?;
        let key = self.admit_request()?;
        self.send(Command::SetMode {
            raw,
            label: mode_id.to_owned(),
            key,
        })
        .inspect_err(|_| self.shared.watchdog.untrack(key))
    }

    pub fn set_config_option(&self, id: &str, value: OptionValue) -> Result<(), DriverError> {
        self.require_usable()?;
        let (raw_id, value) = self.shared.resolve_option(id, &value)?;
        let key = self.admit_request()?;
        self.send(Command::SetConfig {
            raw_id,
            label: id.to_owned(),
            value,
            key,
        })
        .inspect_err(|_| self.shared.watchdog.untrack(key))
    }

    /// Admits one adapter request with a reply deadline, or refuses when the bound of
    /// outstanding requests is reached.
    fn admit_request(&self) -> Result<u64, DriverError> {
        self.shared
            .watchdog
            .track_request(self.shared.limits.request_timeout, MAX_OUTSTANDING_REQUESTS)
            .ok_or(DriverError::Busy(
                "too many mode/config requests await an adapter reply",
            ))
    }

    /// Stops the adapter: closes requests as cancelled, drains for the graceful period,
    /// force-kills the owned tree and verifies the group is empty.
    pub fn shutdown(mut self) -> DriverOutcome {
        self.stop()
    }

    fn stop(&mut self) -> DriverOutcome {
        {
            let mut core = self.shared.core.lock();
            if core.status != DriverStatus::Closed {
                core.stopping = true;
                core.status = DriverStatus::Stopping;
            }
        }
        // Latch the stop before anything else: permissions close and the turn is flagged.
        let _ = self.shared.latch_cancel(true);
        let _ = self.send(Command::Shutdown);
        // Dropping the sender also ends the command loop if the queue was full.
        self.commands.lock().take();
        let grace = self.shared.limits.shutdown_grace;
        if !self.wait_closed(grace + Duration::from_secs(1)) {
            // The connection did not unwind by itself (for example it is blocked on a pipe
            // an escaped helper still holds open): force the owned tree down here.
            self.shared.kill_now();
            self.wait_closed(grace + Duration::from_secs(1));
        }
        let existing = self.shared.core.lock().outcome.clone();
        let outcome = match existing {
            Some(outcome) => outcome,
            None => {
                // Wedged connection: verify the tree here, from one consistent snapshot,
                // with the same sticky ownership demotion as the normal path.
                let termination = self.shared.teardown();
                self.shared.stderr_finish();
                let outcome = self.shared.snapshot_outcome(termination, None);
                self.shared.publish_closed(outcome)
            }
        };
        self.shared.watchdog.stop();
        self.shared.ingress.close();
        if let Some(main) = self.main.take() {
            join_when_finished(main);
        }
        if let Some(watchdog) = self.watchdog.take() {
            join_when_finished(watchdog);
        }
        for handle in self.io.drain(..) {
            // A pipe held open by an escaped helper must not wedge shutdown.
            join_when_finished(handle);
        }
        outcome
    }

    fn wait_closed(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut core = self.shared.core.lock();
        while core.status != DriverStatus::Closed {
            if self
                .shared
                .core_changed
                .wait_until(&mut core, deadline)
                .timed_out()
            {
                return core.status == DriverStatus::Closed;
            }
        }
        true
    }
}

/// Joins a thread that is finishing; a still-blocked thread is detached after a
/// short wait because it only holds shared state, never a lock.
fn join_when_finished(handle: JoinHandle<()>) {
    let deadline = Instant::now() + Duration::from_millis(500);
    while !handle.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    if handle.is_finished() {
        let _ = handle.join();
    }
}

impl Drop for AcpDriver {
    fn drop(&mut self) {
        if self.main.is_some() {
            let _ = self.stop();
        }
    }
}

fn spawn_failure(
    launch: &AdapterLaunch,
    error: SupervisorError,
    redactor: &Redactor,
) -> AgentFailure {
    let (kind, message) = match &error {
        SupervisorError::Process(ProcessError::SpawnFailed { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            if launch.executable.exists() {
                (
                    FailureKind::MissingRuntime,
                    format!(
                        "adapter {} exists but its runtime could not be started",
                        launch.executable.display()
                    ),
                )
            } else {
                (
                    FailureKind::SpawnFailed,
                    format!("adapter {} was not found", launch.executable.display()),
                )
            }
        }
        other => (FailureKind::SpawnFailed, other.to_string()),
    };
    AgentFailure::new(kind, Phase::Spawn, redactor.redact(&message))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn internal(message: &str) -> RpcError {
        RpcError::new(
            i32::from(agent_client_protocol::ErrorCode::InternalError),
            message,
        )
        .data(serde_json::json!({ "errorKind": "incomplete_tool_call" }))
    }

    #[test]
    fn turn_activity_extends_only_the_armed_prompt_deadline() {
        let deadline = |watchdog: &Watchdog| watchdog.state.lock().phase_deadline;
        let watchdog = Watchdog::new();
        // Nothing armed: activity arms nothing.
        watchdog.extend(Phase::Prompt, Duration::from_secs(60));
        assert_eq!(deadline(&watchdog), None);

        watchdog.arm_turn(Duration::from_millis(10), Duration::from_secs(3600));
        let first = deadline(&watchdog).unwrap();
        watchdog.extend(Phase::Prompt, Duration::from_secs(60));
        let extended = deadline(&watchdog).unwrap();
        assert_eq!(extended.phase, Phase::Prompt);
        assert!(extended.at >= first.at + Duration::from_secs(59));

        // Another phase's deadline is never pushed back by turn activity.
        watchdog.arm(Phase::CancelAck, Duration::from_millis(10));
        let before = deadline(&watchdog);
        watchdog.extend(Phase::Prompt, Duration::from_secs(60));
        assert_eq!(deadline(&watchdog), before);
    }

    #[test]
    fn activity_never_extends_a_turn_past_its_cap() {
        let watchdog = Watchdog::new();
        watchdog.arm_turn(Duration::from_millis(10), Duration::from_millis(50));
        let armed = watchdog.state.lock().phase_deadline.unwrap();
        let cap = armed.cap.unwrap();
        assert!(armed.at < cap);
        watchdog.extend(Phase::Prompt, Duration::from_secs(60));
        assert_eq!(watchdog.state.lock().phase_deadline.unwrap().at, cap);
        assert!(matches!(watchdog.next_action(), Some(WatchAction::TurnCap)));

        // The idle deadline expiring first is an idle timeout, not the cap.
        watchdog.arm_turn(Duration::from_millis(10), Duration::from_secs(60));
        assert!(matches!(
            watchdog.next_action(),
            Some(WatchAction::Phase(Phase::Prompt))
        ));
        // A cap too large to represent leaves only the idle deadline.
        watchdog.arm_turn(Duration::from_millis(10), Duration::MAX);
        assert_eq!(watchdog.state.lock().phase_deadline.unwrap().cap, None);
    }

    #[test]
    fn prompt_lines_are_measured_exactly_as_the_sdk_writes_them() {
        let request = PromptRequest::new(
            "sess-1",
            vec![
                ContentBlock::Text(TextContent::new("quote \" and \u{1} and é")),
                ContentBlock::Image(ImageContent::new("AAAA", "image/png")),
            ],
        );
        let untyped = request.to_untyped_message().unwrap();
        let line = serde_json::to_string(
            &RawJsonRpcMessage::request(
                untyped.method,
                untyped.params,
                RequestId::Str("123e4567-e89b-12d3-a456-426614174000".to_owned()),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(prompt_line_bytes(&request), line.len(), "{line}");
        assert_eq!(base64_len(0), Some(0));
        assert_eq!(base64_len(1), Some(4));
        assert_eq!(base64_len(3), Some(4));
        assert_eq!(base64_len(4), Some(8));
        assert_eq!(base64_len(usize::MAX), None);
    }

    #[test]
    fn only_unfinished_server_tools_end_the_turn_normally() {
        let prefix =
            "Internal error: Claude ended the turn without returning results for tool calls: ";
        assert!(only_server_tools_unfinished(&internal(&format!(
            "{prefix}srvtoolu_018rBjyoATEpEkALdkGDKJtB"
        ))));
        assert!(only_server_tools_unfinished(&internal(&format!(
            "{prefix}srvtoolu_a, srvtoolu_b"
        ))));
        // Every id counts, wherever the list wraps.
        assert!(only_server_tools_unfinished(&internal(&format!(
            "{prefix}srvtoolu_a,\nsrvtoolu_b"
        ))));
        assert!(!only_server_tools_unfinished(&internal(&format!(
            "{prefix}srvtoolu_a\ntoolu_b"
        ))));
        // A client tool that never finished is a real failure.
        assert!(!only_server_tools_unfinished(&internal(&format!(
            "{prefix}srvtoolu_a, toolu_b"
        ))));
        assert!(!only_server_tools_unfinished(&internal(&format!(
            "{prefix}srvtoolu_"
        ))));
        assert!(!only_server_tools_unfinished(&internal(prefix)));
        assert!(!only_server_tools_unfinished(&internal(
            "Internal error: boom"
        )));
        // The same text without the adapter's structured tag is not this report.
        let untagged = || {
            RpcError::new(
                i32::from(agent_client_protocol::ErrorCode::InternalError),
                format!("{prefix}srvtoolu_a"),
            )
        };
        assert!(!only_server_tools_unfinished(&untagged()));
        assert!(!only_server_tools_unfinished(
            &untagged().data(serde_json::json!({ "errorKind": "no_result" }))
        ));
        // Same text under another error code is not this adapter rejection.
        assert!(!only_server_tools_unfinished(
            &RpcError::new(
                i32::from(agent_client_protocol::ErrorCode::InvalidParams),
                format!("{prefix}srvtoolu_a"),
            )
            .data(serde_json::json!({ "errorKind": "incomplete_tool_call" }))
        ));
    }
}
