//! Agent task identities, lifecycle, stable draft, writer lease and quiescence gate.
//!
//! A task is separate from the Build/Checkpoint jobs of [`crate::ProjectState`]: it
//! never advances the accepted checkpoint. Its source base is captured from the
//! current (possibly dirty) project bytes, independent of any older checkpoint, and
//! the artifact it produces is a differently typed [`CandidateRevision`].

use crate::OpenSession;
use crate::TaskScope;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use studio_bootstrap::{ScopeObservation, TerminationReport, WriterOwnership};
use studio_project::{
    ProjectId, SourceRevision,
    checkpoint::Checkpoints,
    lifecycle::{atomic_write, sync_directory},
    revision::SourceFile,
};

/// Automatic repair attempts allowed per task (provisional engineering default).
pub const MAX_AUTOMATIC_REPAIRS: u32 = 1;
const MAX_BRIEF_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentTaskId(pub String);

impl AgentTaskId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for AgentTaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything that makes a task result attributable to exactly one task of one open
/// session. A result carrying another session or generation is stale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskIdentity {
    pub task: AgentTaskId,
    pub project: ProjectId,
    pub session: OpenSession,
    /// Monotonic per open session; every new task gets the next value.
    pub generation: u64,
}

/// The source bytes the stable draft was materialized from. Captured from the current
/// project, never reconciled from an older checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSourceBase(SourceRevision);

impl TaskSourceBase {
    pub(crate) fn new(revision: SourceRevision) -> Self {
        Self(revision)
    }
    pub fn revision(&self) -> &SourceRevision {
        &self.0
    }
}

/// Immutable bytes proposed by a task. Deliberately a different type from
/// [`TaskSourceBase`] and from the M1 saved checkpoint: relabelling one as the
/// other is a compile error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateRevision(SourceRevision);

impl CandidateRevision {
    /// `revision` must name an immutable capture of the quiesced draft.
    pub fn new(revision: SourceRevision) -> Self {
        Self(revision)
    }
    pub fn revision(&self) -> &SourceRevision {
        &self.0
    }
}

/// Whole-project context captured when a task starts. No scoped M4 packet and no M5
/// retrieval: only the brief, the current source and what already exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTaskContext {
    pub identity: TaskIdentity,
    pub source_base: TaskSourceBase,
    /// Immutable whole-project/scene/range scope frozen at submission.
    pub scope: TaskScope,
    /// The M1 saved checkpoint at task start; informational, never the draft base.
    pub prior_checkpoint: SourceRevision,
    pub assets: Vec<SourceFile>,
    pub instructions: Vec<SourceFile>,
    pub brief: String,
    /// The stable draft directory; the agent's working directory for every task.
    pub draft: PathBuf,
    /// Where the previous retained draft was archived before this refresh, if any.
    pub archived_previous: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskState {
    ContextReady,
    Editing,
    Waiting,
    Quiescing,
    Validating,
    RepairNeeded,
    CandidateReady,
    Promoting,
    Accepted,
    Conflict,
    Failed,
    Cancelled,
    Interrupted,
}

impl TaskState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Accepted | Self::Conflict | Self::Failed | Self::Cancelled | Self::Interrupted
        )
    }

    /// The complete transition table; everything not listed is illegal.
    pub fn can_transition_to(self, next: Self) -> bool {
        use TaskState::*;
        matches!(
            (self, next),
            (ContextReady, Editing | Failed | Cancelled | Interrupted)
                | (
                    Editing,
                    Waiting | Quiescing | Failed | Cancelled | Interrupted
                )
                | (Waiting, Editing | Failed | Cancelled | Interrupted)
                | (Quiescing, Validating | Failed | Cancelled | Interrupted)
                | (
                    Validating,
                    CandidateReady | RepairNeeded | Failed | Cancelled | Interrupted
                )
                | (RepairNeeded, Editing | Failed | Cancelled | Interrupted)
                | (CandidateReady, Promoting | Failed | Cancelled | Interrupted)
                | (Promoting, Accepted | Conflict | Failed | Interrupted)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairBudget {
    max: u32,
    used: u32,
}

impl RepairBudget {
    pub fn automatic() -> Self {
        Self {
            max: MAX_AUTOMATIC_REPAIRS,
            used: 0,
        }
    }
    pub fn used(&self) -> u32 {
        self.used
    }
    pub fn remaining(&self) -> u32 {
        self.max - self.used
    }
    fn consume(&mut self) -> Result<u32, TaskError> {
        if self.used >= self.max {
            return Err(TaskError::RepairExhausted);
        }
        self.used += 1;
        Ok(self.used)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairDecision {
    /// The task is `RepairNeeded`; `begin_repair` starts attempt `attempt`.
    Repair { attempt: u32 },
    /// No automatic repair is left; the task was failed.
    Exhausted,
}

/// How the last prompt turn ended, as reported by the ACP driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnCompletion {
    EndTurn,
    MaxTokens,
    MaxTurnRequests,
    Refusal,
    Cancelled,
    /// No authoritative completion: stopped, timed out or failed mid-turn.
    None,
}

impl TurnCompletion {
    /// Maps an ACP stop reason string; unknown reasons are not completions.
    pub fn from_stop_reason(reason: &str) -> Self {
        match reason {
            "end_turn" => Self::EndTurn,
            "max_tokens" => Self::MaxTokens,
            "max_turn_requests" => Self::MaxTurnRequests,
            "refusal" => Self::Refusal,
            "cancelled" => Self::Cancelled,
            _ => Self::None,
        }
    }
}

/// Identifies one provider writer epoch of a task. Every writer start and every repair
/// begins a new generation, so evidence or tickets gathered for an earlier writer can
/// never be applied to a later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WriterGeneration(u64);

impl WriterGeneration {
    pub fn value(self) -> u64 {
        self.0
    }
}

/// What the ACP driver observed of the writer (its `DriverOutcome`). Strictly a
/// demotion channel: it can lower the recorded ownership (an escape was seen, the
/// adapter is not qualified) but never raise it, and it never supplies termination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriterObservation {
    /// Ownership as the driver last reported it (already demoted on an observed escape).
    pub ownership: WriterOwnership,
    /// Descendants the driver saw leave the task process group.
    pub escaped_pids: Vec<u32>,
}

/// Facts required before candidate capture. Protocol completion, source scans and
/// quiet periods never substitute for any of them. The evidence is bound to the
/// current task, provider session and writer generation; termination is *not* part of
/// it because the controller derives that from the task's own process scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuiescenceEvidence {
    /// The task this evidence was gathered for.
    pub identity: TaskIdentity,
    /// The provider session of the completing turn (`None` if none was recorded).
    pub provider_session: Option<String>,
    /// The writer generation returned by `agent_writer_started`.
    pub writer: WriterGeneration,
    pub completion: TurnCompletion,
    /// The user/app sent a cancel during the completing turn.
    pub cancel_requested: bool,
    /// Permission requests that were still open when the turn ended.
    pub unresolved_requests: u32,
    /// The driver's view of the writer; can only demote the recorded ownership.
    pub observed: WriterObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QuiescenceBlock {
    #[error("the evidence belongs to another task, session or generation")]
    ForeignEvidence,
    #[error("the evidence names writer generation {found}, the current writer is {current}")]
    StaleWriter { found: u64, current: u64 },
    #[error("the evidence names a different provider session than the task's")]
    ProviderSessionMismatch,
    #[error("no writer was started in this task epoch")]
    NoWriter,
    #[error("no authoritative turn completion was received")]
    NoAuthoritativeCompletion,
    #[error("the turn ended with {0:?}, not end_turn")]
    NotEndTurn(TurnCompletion),
    #[error("the turn was cancelled by the client")]
    CancelRequested,
    #[error("{0} permission request(s) were still open at completion")]
    UnresolvedRequests(u32),
    #[error("the adapter process was not verified terminated")]
    WriterNotTerminated,
    #[error("process group members survived termination: {0:?}")]
    GroupSurvivors(Vec<u32>),
    #[error(
        "adapter writer ownership is {0}; only a process-group-contained adapter can be captured"
    )]
    UnqualifiedOwnership(&'static str),
}

/// Authoritative `end_turn` + verified writer termination + qualified ownership.
/// `ownership` and `termination` are the manager's recorded, sticky values, never
/// caller assertions.
pub fn evaluate_quiescence(
    evidence: &QuiescenceEvidence,
    ownership: &WriterOwnership,
    termination: &TerminationReport,
) -> Result<(), QuiescenceBlock> {
    match evidence.completion {
        TurnCompletion::None => return Err(QuiescenceBlock::NoAuthoritativeCompletion),
        TurnCompletion::EndTurn => {}
        other => return Err(QuiescenceBlock::NotEndTurn(other)),
    }
    if evidence.cancel_requested {
        return Err(QuiescenceBlock::CancelRequested);
    }
    if evidence.unresolved_requests > 0 {
        return Err(QuiescenceBlock::UnresolvedRequests(
            evidence.unresolved_requests,
        ));
    }
    if !termination.group_empty {
        return Err(if termination.remaining.is_empty() {
            QuiescenceBlock::WriterNotTerminated
        } else {
            QuiescenceBlock::GroupSurvivors(termination.remaining.clone())
        });
    }
    if !termination.direct_child_exited {
        return Err(QuiescenceBlock::WriterNotTerminated);
    }
    if !ownership.is_qualified() {
        return Err(QuiescenceBlock::UnqualifiedOwnership(ownership.label()));
    }
    Ok(())
}

/// Proof that quiescence was established for one writer generation of one task;
/// required to capture a candidate. Only [`AgentTaskManager::complete_quiescence`] can
/// create one. Deliberately not `Clone`: it is single-use and bound to the stored gate
/// state (writer generation + nonce), so it cannot be replayed after a repair, a new
/// writer, a failed quiescence or a newer ticket.
#[derive(Debug)]
pub struct CaptureTicket {
    identity: TaskIdentity,
    base: TaskSourceBase,
    draft: PathBuf,
    generation: WriterGeneration,
    nonce: u64,
}

impl CaptureTicket {
    pub fn identity(&self) -> &TaskIdentity {
        &self.identity
    }
    pub fn source_base(&self) -> &TaskSourceBase {
        &self.base
    }
    /// The quiesced draft; no owned writer remains.
    pub fn draft(&self) -> &Path {
        &self.draft
    }
    pub fn writer(&self) -> WriterGeneration {
        self.generation
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TaskError {
    #[error("agent task identity is stale (task, session or generation changed)")]
    StaleIdentity,
    #[error("no agent task is active")]
    NoActiveTask,
    #[error("invalid agent task transition {from:?} -> {to:?}")]
    InvalidTransition { from: TaskState, to: TaskState },
    #[error("transition to {0:?} requires its dedicated operation")]
    GatedTransition(TaskState),
    #[error("operation requires task state {expected:?} but the task is {actual:?}")]
    WrongState {
        expected: TaskState,
        actual: TaskState,
    },
    #[error("agent task {} already holds this project's writer lease", .0.0)]
    LeaseHeld(AgentTaskId),
    #[error("the automatic repair budget is exhausted")]
    RepairExhausted,
    #[error("agent task identity counter exhausted")]
    CounterExhausted,
    #[error("{0}")]
    InvalidBrief(&'static str),
    #[error(
        "the agent draft is in use by a live provider session; stop it before starting another task"
    )]
    DraftBusy,
    #[error("the agent draft is retained: {0}")]
    DraftUnsafe(String),
    #[error("candidate capture is blocked: {0}")]
    QuiescenceBlocked(QuiescenceBlock),
    #[error(
        "the capture ticket is not valid for the current writer (replayed, superseded or issued before a writer start, repair or failed quiescence)"
    )]
    StaleTicket,
    #[error("no writer was started in this task epoch")]
    NoWriter,
    #[error("the validation report is not for this task's recorded candidate")]
    ReportMismatch,
    #[error("agent draft storage: {0}; check disk space and permissions")]
    Storage(String),
}

fn storage(error: impl std::fmt::Display) -> TaskError {
    TaskError::Storage(error.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WriterRecord {
    generation: WriterGeneration,
    /// Sticky: only ever lowered by observations, never raised.
    ownership: WriterOwnership,
    /// Derived from the task's own process scope by the controller, never supplied by
    /// a caller.
    termination: Option<TerminationReport>,
}

/// How much a writer ownership model can be trusted; lower is worse.
fn trust_rank(ownership: &WriterOwnership) -> u8 {
    match ownership {
        WriterOwnership::Detached => 0,
        WriterOwnership::Unknown => 1,
        WriterOwnership::ProcessGroupContained { .. }
        | WriterOwnership::JobObjectContained { .. } => 2,
    }
}

/// The less trusted of two ownership models; ties keep `current`.
fn demote(current: &WriterOwnership, observed: &WriterOwnership) -> WriterOwnership {
    if trust_rank(observed) < trust_rank(current) {
        observed.clone()
    } else {
        current.clone()
    }
}

/// Facts the controller derived from a task's own process scope: the combined
/// termination of its trees, the escapes seen before they were killed, why escapes
/// could not be ruled out (if they could not), and whether any tree was alive when the
/// scope was torn down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScopeFacts {
    pub(crate) termination: TerminationReport,
    pub(crate) escaped: Vec<u32>,
    pub(crate) containment_unverified: Option<String>,
    pub(crate) had_live: bool,
}

impl ScopeFacts {
    /// No scope could be observed: nothing is verified, so the gate stays shut.
    pub(crate) fn unobserved() -> Self {
        Self {
            termination: TerminationReport {
                forced: false,
                direct_child_exited: false,
                group_empty: false,
                remaining: Vec::new(),
            },
            escaped: Vec::new(),
            containment_unverified: None,
            had_live: true,
        }
    }

    /// An escape was seen or could not be ruled out: either way the writer is treated
    /// as detached (fail closed).
    fn detaches_writer(&self) -> bool {
        !self.escaped.is_empty() || self.containment_unverified.is_some()
    }
}

impl From<ScopeObservation> for ScopeFacts {
    fn from(observation: ScopeObservation) -> Self {
        Self {
            had_live: observation.live_children > 0,
            termination: observation.termination,
            escaped: observation.escaped,
            containment_unverified: observation.containment_unverified,
        }
    }
}

/// One agent task. Mutated only through [`AgentTaskManager`], which checks identity.
#[derive(Debug, Clone)]
pub struct AgentTask {
    context: AgentTaskContext,
    state: TaskState,
    repair: RepairBudget,
    provider_session: Option<String>,
    writer: Option<WriterRecord>,
    /// Current writer epoch; bumped by every writer start and repair.
    epoch: u64,
    /// The single outstanding capture ticket nonce, if one was issued in this epoch.
    issued_ticket: Option<u64>,
    next_nonce: u64,
    candidate: Option<CandidateRevision>,
    /// SHA-256 of the captured checkpoint manifest of `candidate`.
    candidate_manifest: Option<String>,
    source_invalidated: bool,
    reason: Option<String>,
}

impl AgentTask {
    fn new(context: AgentTaskContext) -> Self {
        Self {
            context,
            state: TaskState::ContextReady,
            repair: RepairBudget::automatic(),
            provider_session: None,
            writer: None,
            epoch: 0,
            issued_ticket: None,
            next_nonce: 0,
            candidate: None,
            candidate_manifest: None,
            source_invalidated: false,
            reason: None,
        }
    }
    pub fn identity(&self) -> &TaskIdentity {
        &self.context.identity
    }
    pub fn context(&self) -> &AgentTaskContext {
        &self.context
    }
    pub fn state(&self) -> TaskState {
        self.state
    }
    pub fn repair(&self) -> &RepairBudget {
        &self.repair
    }
    pub fn provider_session(&self) -> Option<&str> {
        self.provider_session.as_deref()
    }
    pub fn candidate(&self) -> Option<&CandidateRevision> {
        self.candidate.as_ref()
    }
    /// Manifest hash recorded with the candidate at capture.
    pub fn candidate_manifest(&self) -> Option<&str> {
        self.candidate_manifest.as_deref()
    }
    pub fn writer_ownership(&self) -> Option<&WriterOwnership> {
        self.writer.as_ref().map(|w| &w.ownership)
    }
    /// The current writer generation, if a writer was started in this epoch.
    pub fn writer_generation(&self) -> Option<WriterGeneration> {
        self.writer.as_ref().map(|w| w.generation)
    }
    /// The project source became unreadable/invalid while the task ran. Sticky: an
    /// edit back to the base bytes never revives the task.
    pub fn source_invalidated(&self) -> bool {
        self.source_invalidated
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    /// No owned writer can still be modifying the draft: either none ever started,
    /// or its termination was verified under a qualified ownership model.
    pub fn writer_safe(&self) -> bool {
        match &self.writer {
            None => true,
            Some(w) => {
                w.ownership.is_qualified() && w.termination.as_ref().is_some_and(|t| t.verified())
            }
        }
    }
    pub(crate) fn unsafe_reason(&self) -> String {
        match &self.writer {
            None => "no writer started".to_owned(),
            Some(w) if !w.ownership.is_qualified() => format!(
                "adapter writer ownership is {}; a detached or unknown writer may still modify the draft",
                w.ownership.label()
            ),
            Some(_) => {
                "adapter termination was not verified; a writer may still be running".to_owned()
            }
        }
    }

    /// Starts a new writer epoch: every outstanding ticket and all earlier evidence die.
    fn bump_epoch(&mut self) -> Result<WriterGeneration, TaskError> {
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or(TaskError::CounterExhausted)?;
        self.issued_ticket = None;
        Ok(WriterGeneration(self.epoch))
    }

    /// Lowers the recorded ownership with a driver observation; escapes force `Detached`.
    fn demote_with(&mut self, observed: &WriterObservation) {
        if let Some(writer) = self.writer.as_mut() {
            writer.ownership = demote(&writer.ownership, &observed.ownership);
            if !observed.escaped_pids.is_empty() {
                writer.ownership = WriterOwnership::Detached;
            }
        }
    }

    /// Applies termination/escape facts derived from the task's own process scope. An
    /// escape that could not be ruled out demotes exactly like an observed one.
    fn apply_scope_facts(&mut self, facts: ScopeFacts) {
        let detached = facts.detaches_writer();
        match self.writer.as_mut() {
            Some(writer) => {
                if detached {
                    writer.ownership = WriterOwnership::Detached;
                }
                writer.termination = Some(facts.termination);
            }
            // Processes lived in the task scope but no writer was ever registered for
            // them: their ownership model is unproven.
            None if facts.had_live || detached || !facts.termination.verified() => {
                let ownership = if detached {
                    WriterOwnership::Detached
                } else {
                    WriterOwnership::Unknown
                };
                self.writer = Some(WriterRecord {
                    generation: WriterGeneration(self.epoch),
                    ownership,
                    termination: Some(facts.termination),
                });
            }
            None => {}
        }
    }
}

/// What the controller must record about the draft when a task ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedTask {
    pub identity: TaskIdentity,
    pub state: TaskState,
    pub writer_safe: bool,
    pub reason: String,
    pub unsafe_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WriterLease {
    task: AgentTaskId,
}

/// Serialized owner of the single active task and the project's one writer lease.
/// Lives inside the per-project controller, so every call is already exclusive.
#[derive(Debug)]
pub struct AgentTaskManager {
    project: ProjectId,
    session: OpenSession,
    next_generation: u64,
    active: Option<AgentTask>,
    lease: Option<WriterLease>,
    last_finished: Option<AgentTask>,
}

/// Reserved identity for a task that is still being prepared.
#[derive(Debug)]
pub(crate) struct TaskReservation {
    pub(crate) identity: TaskIdentity,
}

impl AgentTaskManager {
    pub fn new(project: ProjectId, session: OpenSession) -> Self {
        Self {
            project,
            session,
            next_generation: 0,
            active: None,
            lease: None,
            last_finished: None,
        }
    }

    /// Starts allocation at `next` (for counter exhaustion tests).
    pub fn with_next_generation(mut self, next: u64) -> Self {
        self.next_generation = next;
        self
    }

    pub fn lease_holder(&self) -> Option<&AgentTaskId> {
        self.lease.as_ref().map(|l| &l.task)
    }

    pub fn active(&self) -> Option<&AgentTask> {
        self.active.as_ref()
    }

    /// The active task, or the most recently finished one.
    pub fn current(&self) -> Option<&AgentTask> {
        self.active.as_ref().or(self.last_finished.as_ref())
    }

    pub(crate) fn reserve(&mut self) -> Result<TaskReservation, TaskError> {
        if let Some(lease) = &self.lease {
            return Err(TaskError::LeaseHeld(lease.task.clone()));
        }
        let generation = self.next_generation;
        self.next_generation = generation
            .checked_add(1)
            .ok_or(TaskError::CounterExhausted)?;
        Ok(TaskReservation {
            identity: TaskIdentity {
                task: AgentTaskId::new(),
                project: self.project.clone(),
                session: self.session.clone(),
                generation,
            },
        })
    }

    /// Allocates a fresh generation for an identity that owns no writer lease (Undo).
    pub(crate) fn allocate_generation(&mut self) -> Result<u64, TaskError> {
        let generation = self.next_generation;
        self.next_generation = generation
            .checked_add(1)
            .ok_or(TaskError::CounterExhausted)?;
        Ok(generation)
    }

    /// Installs a prepared task and takes the writer lease.
    pub(crate) fn begin(&mut self, context: AgentTaskContext) -> Result<(), TaskError> {
        if let Some(lease) = &self.lease {
            return Err(TaskError::LeaseHeld(lease.task.clone()));
        }
        self.lease = Some(WriterLease {
            task: context.identity.task.clone(),
        });
        self.last_finished = None;
        self.active = Some(AgentTask::new(context));
        Ok(())
    }

    fn check(&self, identity: &TaskIdentity) -> Result<&AgentTask, TaskError> {
        let task = self.active.as_ref().ok_or(TaskError::NoActiveTask)?;
        if task.identity() != identity || identity.session != self.session {
            return Err(TaskError::StaleIdentity);
        }
        Ok(task)
    }

    fn check_mut(&mut self, identity: &TaskIdentity) -> Result<&mut AgentTask, TaskError> {
        self.check(identity)?;
        Ok(self.active.as_mut().expect("checked"))
    }

    /// Non-terminal transitions that need no extra proof. Candidate registration,
    /// repair, and terminal states use their dedicated operations.
    pub fn transition(
        &mut self,
        identity: &TaskIdentity,
        next: TaskState,
    ) -> Result<(), TaskError> {
        let task = self.check_mut(identity)?;
        if next.is_terminal() {
            return Err(TaskError::GatedTransition(next));
        }
        if !task.state.can_transition_to(next) {
            return Err(TaskError::InvalidTransition {
                from: task.state,
                to: next,
            });
        }
        if matches!(next, TaskState::Validating | TaskState::RepairNeeded)
            || next == TaskState::CandidateReady
            || (task.state == TaskState::RepairNeeded && next == TaskState::Editing)
        {
            return Err(TaskError::GatedTransition(next));
        }
        task.state = next;
        Ok(())
    }

    /// Validates the identity against the active task without touching anything.
    pub(crate) fn validate(&self, identity: &TaskIdentity) -> Result<&AgentTask, TaskError> {
        self.check(identity)
    }

    /// Records that a provider writer was started for this task and how its writers
    /// are owned, beginning a new writer generation. All earlier evidence and every
    /// outstanding capture ticket die. Ownership is sticky within the task: a writer
    /// already recorded as less trusted keeps its demotion.
    pub fn writer_started(
        &mut self,
        identity: &TaskIdentity,
        ownership: WriterOwnership,
    ) -> Result<WriterGeneration, TaskError> {
        let task = self.check_mut(identity)?;
        let generation = task.bump_epoch()?;
        let ownership = match &task.writer {
            Some(previous) => demote(&previous.ownership, &ownership),
            None => ownership,
        };
        task.writer = Some(WriterRecord {
            generation,
            ownership,
            termination: None,
        });
        Ok(generation)
    }

    /// Applies the driver's view of the writer (its `DriverOutcome`). It can only lower
    /// the recorded ownership; it never proves termination.
    pub fn observe_writer(
        &mut self,
        identity: &TaskIdentity,
        observed: &WriterObservation,
    ) -> Result<(), TaskError> {
        let task = self.check_mut(identity)?;
        if task.writer.is_none() {
            return Err(TaskError::NoWriter);
        }
        task.demote_with(observed);
        Ok(())
    }

    pub fn record_provider_session(
        &mut self,
        identity: &TaskIdentity,
        session: &str,
    ) -> Result<(), TaskError> {
        self.check_mut(identity)?.provider_session = Some(session.to_owned());
        Ok(())
    }

    /// Records termination and escapes the controller derived from this task's own
    /// process scope at teardown. Escapes demote the ownership for good.
    pub(crate) fn record_scope_facts(
        &mut self,
        identity: &TaskIdentity,
        facts: ScopeFacts,
    ) -> Result<(), TaskError> {
        self.check_mut(identity)?.apply_scope_facts(facts);
        Ok(())
    }

    /// Evaluates the quiescence gate. `observe` seals the task's process scope against
    /// further spawns and observes it; it runs only after the identity, state, writer
    /// and evidence-binding checks pass, so foreign or stale evidence never reaches or
    /// affects a scope. The recorded ownership is sticky (evidence can only demote it)
    /// and termination comes from `observe`, never from the caller. Every attempt
    /// retires the outstanding ticket; only a pass yields a new one.
    pub(crate) fn complete_quiescence(
        &mut self,
        identity: &TaskIdentity,
        evidence: &QuiescenceEvidence,
        observe: impl FnOnce() -> ScopeFacts,
    ) -> Result<CaptureTicket, TaskError> {
        let task = self.check_mut(identity)?;
        if task.state != TaskState::Quiescing {
            return Err(TaskError::WrongState {
                expected: TaskState::Quiescing,
                actual: task.state,
            });
        }
        task.issued_ticket = None;
        let block = TaskError::QuiescenceBlocked;
        if evidence.identity != *identity {
            return Err(block(QuiescenceBlock::ForeignEvidence));
        }
        let Some(writer) = task.writer.as_ref() else {
            return Err(block(QuiescenceBlock::NoWriter));
        };
        if evidence.writer != writer.generation || writer.generation.0 != task.epoch {
            return Err(block(QuiescenceBlock::StaleWriter {
                found: evidence.writer.0,
                current: writer.generation.0,
            }));
        }
        if evidence.provider_session != task.provider_session {
            return Err(block(QuiescenceBlock::ProviderSessionMismatch));
        }
        task.demote_with(&evidence.observed);
        task.apply_scope_facts(observe());
        let writer = task.writer.as_ref().expect("writer checked above");
        let termination = writer
            .termination
            .as_ref()
            .expect("scope facts always record termination for a registered writer");
        evaluate_quiescence(evidence, &writer.ownership, termination).map_err(block)?;
        let nonce = task.next_nonce;
        task.next_nonce = nonce.checked_add(1).ok_or(TaskError::CounterExhausted)?;
        task.issued_ticket = Some(nonce);
        Ok(CaptureTicket {
            identity: task.context.identity.clone(),
            base: task.context.source_base.clone(),
            draft: task.context.draft.clone(),
            generation: WriterGeneration(task.epoch),
            nonce,
        })
    }

    /// Registers the immutable candidate captured under `ticket` and starts validation.
    /// The ticket is single-use and must match the stored writer generation and nonce.
    pub fn record_candidate(
        &mut self,
        ticket: CaptureTicket,
        candidate: CandidateRevision,
        manifest_sha256: String,
    ) -> Result<(), TaskError> {
        self.check_ticket(&ticket)?;
        let task = self.check_mut(&ticket.identity)?;
        task.issued_ticket = None;
        task.candidate = Some(candidate);
        task.candidate_manifest = Some(manifest_sha256);
        task.state = TaskState::Validating;
        Ok(())
    }

    /// Whether `ticket` is still the single outstanding ticket of the current writer
    /// epoch of its task. Nothing changes.
    pub(crate) fn check_ticket(&self, ticket: &CaptureTicket) -> Result<(), TaskError> {
        let task = self.check(&ticket.identity)?;
        if task.state != TaskState::Quiescing {
            return Err(TaskError::WrongState {
                expected: TaskState::Quiescing,
                actual: task.state,
            });
        }
        if ticket.generation.0 != task.epoch
            || task.issued_ticket != Some(ticket.nonce)
            || !task.writer_safe()
        {
            return Err(TaskError::StaleTicket);
        }
        Ok(())
    }

    /// A passing validation report for the recorded candidate: `Validating` ->
    /// `CandidateReady`.
    pub(crate) fn candidate_validated(
        &mut self,
        identity: &TaskIdentity,
        candidate: &CandidateRevision,
    ) -> Result<(), TaskError> {
        let task = self.check_mut(identity)?;
        if task.state != TaskState::Validating {
            return Err(TaskError::WrongState {
                expected: TaskState::Validating,
                actual: task.state,
            });
        }
        if task.candidate.as_ref() != Some(candidate) {
            return Err(TaskError::ReportMismatch);
        }
        task.state = TaskState::CandidateReady;
        Ok(())
    }

    /// Validation failed: spend the repair budget or fail the task.
    pub fn validation_failed(
        &mut self,
        identity: &TaskIdentity,
        reason: &str,
    ) -> Result<RepairDecision, TaskError> {
        let task = self.check_mut(identity)?;
        if task.state != TaskState::Validating {
            return Err(TaskError::WrongState {
                expected: TaskState::Validating,
                actual: task.state,
            });
        }
        task.candidate = None;
        task.candidate_manifest = None;
        task.issued_ticket = None;
        if task.repair.remaining() == 0 {
            self.finish(identity, TaskState::Failed, reason)?;
            return Ok(RepairDecision::Exhausted);
        }
        let task = self.check_mut(identity)?;
        task.state = TaskState::RepairNeeded;
        task.reason = Some(reason.to_owned());
        Ok(RepairDecision::Repair {
            attempt: task.repair.used() + 1,
        })
    }

    /// Starts the automatic repair attempt as a new writer epoch: the previous writer
    /// record, evidence and every ticket are discarded, so a new writer needs fresh
    /// quiescence evidence. `ready` runs after validation and before any mutation (the
    /// controller seals and verifies the previous process scope there); if it fails the
    /// task stays `RepairNeeded`, untouched.
    pub(crate) fn begin_repair(
        &mut self,
        identity: &TaskIdentity,
        ready: impl FnOnce() -> Result<(), TaskError>,
    ) -> Result<u32, TaskError> {
        let task = self.check_mut(identity)?;
        if task.state != TaskState::RepairNeeded {
            return Err(TaskError::WrongState {
                expected: TaskState::RepairNeeded,
                actual: task.state,
            });
        }
        if task.repair.remaining() == 0 {
            return Err(TaskError::RepairExhausted);
        }
        if task.epoch == u64::MAX {
            return Err(TaskError::CounterExhausted);
        }
        ready()?;
        let attempt = task.repair.consume()?;
        task.bump_epoch()?;
        task.state = TaskState::Editing;
        task.writer = None;
        Ok(attempt)
    }

    /// Validates that `finish(identity, terminal, ..)` would succeed, without changing
    /// anything. The controller calls this before touching any process scope.
    pub(crate) fn check_finish(
        &self,
        identity: &TaskIdentity,
        terminal: TaskState,
    ) -> Result<(), TaskError> {
        let task = self.check(identity)?;
        if !terminal.is_terminal() || !task.state.can_transition_to(terminal) {
            return Err(TaskError::InvalidTransition {
                from: task.state,
                to: terminal,
            });
        }
        Ok(())
    }

    /// Settles the task in a terminal state and releases the writer lease.
    pub fn finish(
        &mut self,
        identity: &TaskIdentity,
        terminal: TaskState,
        reason: &str,
    ) -> Result<FinishedTask, TaskError> {
        self.check_finish(identity, terminal)?;
        let task = self.check_mut(identity)?;
        task.state = terminal;
        task.reason = Some(reason.to_owned());
        task.issued_ticket = None;
        let task = self.active.take().expect("checked");
        self.lease = None;
        let finished = FinishedTask {
            identity: task.context.identity.clone(),
            state: terminal,
            writer_safe: task.writer_safe(),
            reason: reason.to_owned(),
            unsafe_reason: (!task.writer_safe()).then(|| task.unsafe_reason()),
        };
        self.last_finished = Some(task);
        Ok(finished)
    }

    /// The project source became invalid: the active task is interrupted and can never
    /// resume, even if the source later returns to its base bytes.
    pub(crate) fn invalidate_source(&mut self, reason: &str) -> Option<FinishedTask> {
        let task = self.active.as_mut()?;
        task.source_invalidated = true;
        let identity = task.context.identity.clone();
        self.finish(&identity, TaskState::Interrupted, reason).ok()
    }
}

mod draft;

pub use draft::*;

/// Reads whole-project brief limits shared by every entry point.
pub fn validate_brief(brief: &str) -> Result<String, TaskError> {
    let trimmed = brief.trim();
    if trimmed.is_empty() {
        return Err(TaskError::InvalidBrief("the task brief is empty"));
    }
    if trimmed.len() > MAX_BRIEF_BYTES {
        return Err(TaskError::InvalidBrief(
            "the task brief exceeds 64 KiB; shorten it",
        ));
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests;
