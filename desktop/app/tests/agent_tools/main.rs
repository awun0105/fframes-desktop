//! Tool contract, broker security/limits, MCP protocol and CLI/MCP subprocess parity.
//! A deterministic in-test backend and liveness oracle stand in for the app.
//! Unix and Windows run the same suite; the few file-mode, symlink and socket-path cases
//! are unix-only and have their Windows counterparts next to them.

use parking_lot::{Condvar, Mutex};
use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::{
    fs::{PermissionsExt, symlink},
    net::{UnixListener, UnixStream},
};
use std::{
    collections::HashSet,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
#[cfg(windows)]
use studio_engine::agent_tools::transport;
use studio_engine::agent_tools::{
    ArtifactRef, BoundRevision, FrameSelection, MAX_IMAGE_ARTIFACT_BYTES, MAX_QUEUED_CALLS,
    MAX_REQUEST_BYTES, MAX_STRIP_FRAMES, MAX_TEXT_REPLY_BYTES, RevisionInfo, RevisionLabel,
    TOOL_NAMES, TOOL_WORKERS, TaskLiveness, ToolBackend, ToolBinding, ToolCall, ToolDispatcher,
    ToolError, ToolErrorCode, ToolReply, ToolRequest,
    backend::{MAX_KNOWN_REVISIONS, RevisionHistory, STATUS_KNOWN_REVISIONS},
    broker::{BrokerConfig, BrokerError, ToolBroker},
    client::{BrokerClient, CAPABILITY_ENV, CallFailure, ClientError},
    mcp::{
        INVALID_PARAMS, INVALID_REQUEST, MAX_MCP_LINE_BYTES, MCP_PROTOCOL_VERSION,
        MCP_QUALIFICATION, METHOD_NOT_FOUND, McpServer, NOT_INITIALIZED, PARSE_ERROR, ToolCaller,
    },
    tool_descriptions,
};
use studio_engine::{AgentTaskId, OpenSession, TaskIdentity};

use studio_project::{ProjectId, SourceRevision};

const REV_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const REV_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const WAIT: Duration = Duration::from_secs(20);

// ---------------------------------------------------------------- fixtures

type Behavior = dyn Fn(&ToolBinding, &ToolRequest, &(dyn Fn() -> bool + Sync)) -> Result<ToolReply, ToolError>
    + Send
    + Sync;

fn revision_info() -> RevisionInfo {
    RevisionInfo {
        id: REV_A.into(),
        label: RevisionLabel::DraftSnapshot,
        validated: false,
    }
}

fn plain_reply(request: &ToolRequest) -> ToolReply {
    ToolReply {
        revision: revision_info(),
        result: json!({"tool": request.call.name()}),
        artifacts: vec![],
    }
}

fn artifact(bytes: u64) -> ArtifactRef {
    ArtifactRef {
        id: "art-0123456789abcdef".into(),
        media_type: "image/png".into(),
        path: "/tmp/fframes-artifacts/art-0123456789abcdef.png".into(),
        bytes,
        sha256: "c".repeat(64),
        width: 640,
        height: 360,
        expires_at_unix: 1_900_000_000,
    }
}

fn standard(
    _binding: &ToolBinding,
    request: &ToolRequest,
    _cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<ToolReply, ToolError> {
    match request.call {
        ToolCall::RenderFrame { frame: 999, .. } => Err(ToolError::new(
            ToolErrorCode::NotFound,
            "frame 999 does not exist",
        )),
        ToolCall::RenderFrame { .. } | ToolCall::RenderStrip { .. } => Ok(ToolReply {
            artifacts: vec![artifact(2048)],
            ..plain_reply(request)
        }),
        _ => Ok(plain_reply(request)),
    }
}

struct Gate {
    state: Mutex<(usize, bool)>,
    changed: Condvar,
}

impl Gate {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new((0, false)),
            changed: Condvar::new(),
        })
    }

    /// Announce arrival, then block until opened or cancelled. True when opened.
    fn pass(&self, cancelled: &(dyn Fn() -> bool + Sync)) -> bool {
        let mut state = self.state.lock();
        state.0 += 1;
        self.changed.notify_all();
        while !state.1 {
            if cancelled() {
                return false;
            }
            self.changed.wait_for(&mut state, Duration::from_millis(10));
        }
        true
    }

    fn wait_entered(&self, count: usize) {
        let deadline = Instant::now() + WAIT;
        let mut state = self.state.lock();
        while state.0 < count {
            assert!(
                !self.changed.wait_until(&mut state, deadline).timed_out() || state.0 >= count,
                "gate was not entered in time"
            );
        }
    }

    fn open(&self) {
        self.state.lock().1 = true;
        self.changed.notify_all();
    }
}

struct FakeBackend {
    calls: Mutex<Vec<String>>,
    running: AtomicUsize,
    max_running: AtomicUsize,
    behavior: Mutex<Arc<Behavior>>,
}

impl FakeBackend {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::new(Vec::new()),
            running: AtomicUsize::new(0),
            max_running: AtomicUsize::new(0),
            behavior: Mutex::new(Arc::new(standard)),
        })
    }

    fn set(
        &self,
        behavior: impl Fn(
            &ToolBinding,
            &ToolRequest,
            &(dyn Fn() -> bool + Sync),
        ) -> Result<ToolReply, ToolError>
        + Send
        + Sync
        + 'static,
    ) {
        *self.behavior.lock() = Arc::new(behavior);
    }

    fn call_count(&self) -> usize {
        self.calls.lock().len()
    }
}

struct RunningGuard<'a>(&'a AtomicUsize);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl ToolBackend for FakeBackend {
    fn execute(
        &self,
        binding: &ToolBinding,
        request: &ToolRequest,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<ToolReply, ToolError> {
        self.calls.lock().push(request.call.name().into());
        let now = self.running.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_running.fetch_max(now, Ordering::SeqCst);
        let _guard = RunningGuard(&self.running);
        let behavior = self.behavior.lock().clone();
        behavior(binding, request, cancelled)
    }
}

#[derive(Default)]
struct FakeLiveness {
    dead: Mutex<HashSet<AgentTaskId>>,
}

impl FakeLiveness {
    fn kill(&self, task: &TaskIdentity) {
        self.dead.lock().insert(task.task.clone());
    }
}

impl TaskLiveness for FakeLiveness {
    fn is_live(&self, task: &TaskIdentity) -> bool {
        !self.dead.lock().contains(&task.task)
    }
}

fn task() -> TaskIdentity {
    TaskIdentity {
        task: AgentTaskId::new(),
        project: ProjectId::try_from("proj-1".to_string()).unwrap(),
        session: OpenSession::new(),
        generation: 1,
    }
}

fn draft(task: &TaskIdentity) -> ToolBinding {
    ToolBinding {
        task: task.clone(),
        revision: BoundRevision::Draft,
    }
}

fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(5));
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    runtime: PathBuf,
    backend: Arc<FakeBackend>,
    liveness: Arc<FakeLiveness>,
    broker: ToolBroker,
}

#[cfg(unix)]
fn short_tempdir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("fft")
        .tempdir_in("/tmp")
        .unwrap()
}

/// Pipe names do not depend on the runtime directory's path.
#[cfg(windows)]
fn short_tempdir() -> tempfile::TempDir {
    tempfile::Builder::new().prefix("fft").tempdir().unwrap()
}

#[cfg(unix)]
type RawStream = UnixStream;
#[cfg(windows)]
type RawStream = transport::Stream;

fn raw_connect(endpoint: &Path) -> std::io::Result<RawStream> {
    #[cfg(unix)]
    return UnixStream::connect(endpoint);
    #[cfg(windows)]
    return transport::connect(endpoint, Duration::from_secs(5));
}

/// Whether the broker still serves its endpoint. A pipe is looked up in the pipe
/// directory: opening its path would connect to it.
fn endpoint_exists(endpoint: &Path) -> bool {
    #[cfg(unix)]
    return endpoint.exists();
    #[cfg(windows)]
    return fs::read_dir(r"\\.\pipe\")
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| Some(entry.file_name().as_os_str()) == endpoint.file_name());
}

fn start_in(
    runtime: &Path,
    backend: &Arc<FakeBackend>,
    liveness: &Arc<FakeLiveness>,
) -> Result<ToolBroker, BrokerError> {
    ToolBroker::start(BrokerConfig {
        runtime_dir: runtime.to_path_buf(),
        dispatcher: ToolDispatcher::new(backend.clone()),
        liveness: liveness.clone(),
    })
}

fn harness() -> Harness {
    let dir = short_tempdir();
    let runtime = dir.path().join("rt");
    let backend = FakeBackend::new();
    let liveness = Arc::new(FakeLiveness::default());
    let broker = start_in(&runtime, &backend, &liveness).unwrap();
    Harness {
        _dir: dir,
        runtime,
        backend,
        liveness,
        broker,
    }
}

impl Harness {
    fn grant(&self, task: &TaskIdentity) -> Cap {
        let grant = self
            .broker
            .grant(draft(task), Duration::from_secs(600))
            .unwrap();
        Cap::read(&grant.capability_file)
    }
}

struct Cap {
    file: PathBuf,
    socket: PathBuf,
    capability: String,
    secret: String,
}

impl Cap {
    fn read(file: &Path) -> Self {
        let value: Value = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
        Self {
            file: file.to_path_buf(),
            socket: PathBuf::from(value["socket"].as_str().unwrap()),
            capability: value["capability"].as_str().unwrap().into(),
            secret: value["secret"].as_str().unwrap().into(),
        }
    }

    fn wire(&self) -> Wire {
        let mut wire = Wire::connect(&self.socket);
        let reply = wire.hello(&self.capability, &self.secret);
        assert_eq!(reply, json!({"ok": true}), "hello must succeed");
        wire
    }
}

struct Wire {
    stream: RawStream,
    reader: BufReader<RawStream>,
}

impl Wire {
    fn connect(socket: &Path) -> Self {
        let stream = raw_connect(socket).unwrap();
        stream.set_read_timeout(Some(WAIT)).unwrap();
        Self {
            reader: BufReader::new(stream.try_clone().unwrap()),
            stream,
        }
    }

    fn send_raw(&mut self, text: &str) {
        self.stream.write_all(text.as_bytes()).unwrap();
    }

    fn send(&mut self, value: &Value) {
        self.send_raw(&format!("{value}\n"));
    }

    /// `None` on end of stream or a reset connection.
    fn recv(&mut self) -> Option<Value> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(
                serde_json::from_str(&line)
                    .unwrap_or_else(|error| panic!("non-JSON broker line {line:?}: {error}")),
            ),
        }
    }

    fn hello(&mut self, capability: &str, secret: &str) -> Value {
        self.send(&json!({"hello": {"capability": capability, "secret": secret}}));
        self.recv().expect("hello answer")
    }

    fn request(&mut self, id: u64, method: &str, params: Value) {
        self.send(&json!({"id": id, "method": method, "params": params}));
    }

    fn call(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.request(id, method, params);
        self.recv().expect("reply")
    }
}

fn error_code(reply: &Value) -> &str {
    reply["error"]["code"].as_str().unwrap_or("<no error>")
}

fn never() -> bool {
    false
}

#[cfg(unix)]
fn private_copy(dir: &Path, name: &str, bytes: &[u8], mode: u32) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
    path
}

/// A mode without group or world bits makes the file private to the current user; any
/// other mode leaves the temp directory's inherited permissions, which grant others.
#[cfg(windows)]
fn private_copy(dir: &Path, name: &str, bytes: &[u8], mode: u32) -> PathBuf {
    let path = dir.join(name);
    if mode & 0o077 == 0 {
        transport::create_private_file(&path, bytes).unwrap();
    } else {
        fs::write(&path, bytes).unwrap();
    }
    path
}

fn rpc(server: &mut McpServer, request: Value) -> Value {
    let line = server
        .handle_line(&request.to_string())
        .expect("a response");
    assert!(!line.contains('\n'), "one response per line");
    serde_json::from_str(&line).unwrap()
}

fn initialize(server: &mut McpServer) -> Value {
    rpc(
        server,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion": MCP_PROTOCOL_VERSION, "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"}}}),
    )
}

mod broker;
mod client;
mod contract;
mod mcp;
mod slow_peers;
mod subprocess;
