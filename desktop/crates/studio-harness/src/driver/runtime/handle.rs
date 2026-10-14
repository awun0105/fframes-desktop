//! The host handle that owns one adapter process and session.

use super::*;

/// Owner of one ACP v1 adapter process and session. Dropping stops it.
pub struct AcpDriver {
    pub(super) shared: Arc<Shared>,
    pub(super) commands: Mutex<Option<mpsc::Sender<Command>>>,
    pub(super) main: Option<JoinHandle<()>>,
    pub(super) watchdog: Option<JoinHandle<()>>,
    pub(super) io: Vec<JoinHandle<()>>,
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

    pub(super) fn send(&self, command: Command) -> Result<(), DriverError> {
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

    pub(super) fn require_usable(&self) -> Result<(), DriverError> {
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
    pub(super) fn admit_request(&self) -> Result<u64, DriverError> {
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

    pub(super) fn stop(&mut self) -> DriverOutcome {
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

    pub(super) fn wait_closed(&self, timeout: Duration) -> bool {
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
pub(super) fn join_when_finished(handle: JoinHandle<()>) {
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

pub(super) fn spawn_failure(
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
