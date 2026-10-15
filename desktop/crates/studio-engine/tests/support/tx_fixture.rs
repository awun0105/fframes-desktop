//! Shared fixtures for the edit-transaction and task-recovery integration tests.
//! Include with `#[path = "support/tx_fixture.rs"] mod tx_fixture;`.
#![allow(dead_code)]

use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use studio_bootstrap::WriterOwnership;
use studio_engine::{
    AgentTaskContext, Boundary, CaptureTicket, Controller, EngineError, Fault, QuiescenceEvidence,
    TaskState, TurnCompletion, WriterObservation,
    app_paths::AppPaths,
    build_materialization::sdk_pin,
    candidate_validation::{CapturedCandidate, ValidationReport},
};
use studio_project::checkpoint::Checkpoints;
use studio_sdk::CompatibilityManifest;

#[path = "passing_probe.rs"]
pub mod passing_probe;

pub struct Fx {
    pub temp: tempfile::TempDir,
    pub root: PathBuf,
    pub paths: AppPaths,
}

impl Fx {
    pub fn history(&self, controller: &Controller) -> PathBuf {
        self.paths.project(&controller.project.manifest.project_id)
    }
    pub fn checkpoints(&self, controller: &Controller) -> Checkpoints {
        Checkpoints::new(&self.history(controller)).unwrap()
    }
    pub fn task_journal(&self, controller: &Controller) -> PathBuf {
        self.history(controller).join("tasks.jsonl")
    }
}

/// A project with a Git repository, an executable script and a few ordinary files.
pub fn fixture() -> Fx {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("video");
    let paths = AppPaths::new(temp.path().join("history")).unwrap();
    let compatibility = CompatibilityManifest::default_linux_x64();
    studio_project::create(
        &root,
        "Video",
        sdk_pin(&compatibility),
        &compatibility.fframes_version,
        "0.1.0",
    )
    .unwrap();
    fs::write(root.join("notes.txt"), "keep me\n").unwrap();
    fs::write(root.join("old.txt"), "to be deleted\n").unwrap();
    fs::write(root.join("run.sh"), "#!/bin/sh\necho hi\n").unwrap();
    set_mode(&root.join("run.sh"), 0o644);
    fs::write(root.join("private.txt"), "secret\n").unwrap();
    set_mode(&root.join("private.txt"), 0o600);
    let git = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(&root)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap();
        assert!(status.status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    // Keep background Git maintenance from racing tests that snapshot `.git` byte-for-byte.
    git(&["config", "maintenance.auto", "false"]);
    git(&["config", "gc.auto", "0"]);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "baseline"]);
    Fx { temp, root, paths }
}

pub fn qualified() -> WriterOwnership {
    WriterOwnership::ProcessGroupContained {
        qualification: "test-adapter".into(),
    }
}

pub fn evidence(controller: &Controller, context: &AgentTaskContext) -> QuiescenceEvidence {
    QuiescenceEvidence {
        identity: context.identity.clone(),
        provider_session: None,
        writer: controller
            .agent_task()
            .unwrap()
            .writer_generation()
            .expect("a writer was started"),
        completion: TurnCompletion::EndTurn,
        cancel_requested: false,
        unresolved_requests: 0,
        observed: WriterObservation {
            ownership: qualified(),
            escaped_pids: vec![],
        },
    }
}

pub fn start(controller: &mut Controller, brief: &str) -> AgentTaskContext {
    let context = controller.begin_agent_task(brief).unwrap();
    controller
        .agent_writer_started(&context.identity, qualified())
        .unwrap();
    controller
        .agent_task_transition(&context.identity, TaskState::Editing)
        .unwrap();
    controller
        .agent_task_transition(&context.identity, TaskState::Quiescing)
        .unwrap();
    context
}

pub fn ticket(controller: &mut Controller, context: &AgentTaskContext) -> CaptureTicket {
    let evidence = evidence(controller, context);
    controller
        .agent_complete_quiescence(&context.identity, &evidence)
        .unwrap()
}

/// Starts a task, lets `edits` change the draft, quiesces, captures and validates it
/// with a genuinely passing report. The task is left `Validating`.
pub fn validated_in(
    controller: &mut Controller,
    brief: &str,
    edits: impl FnOnce(&Path),
) -> (AgentTaskContext, CapturedCandidate, ValidationReport) {
    let context = start(controller, brief);
    edits(&context.draft);
    let t = ticket(controller, &context);
    let captured = controller.agent_capture_candidate(t).unwrap();
    let report = passing_probe::passing_report(&captured);
    (context, captured, report)
}

pub fn validated(
    _f: &Fx,
    controller: &mut Controller,
    brief: &str,
    edits: impl FnOnce(&Path),
) -> (AgentTaskContext, CapturedCandidate, ValidationReport) {
    validated_in(controller, brief, edits)
}

/// Brings a task all the way to `CandidateReady` (manual review path).
pub fn ready(
    f: &Fx,
    controller: &mut Controller,
    brief: &str,
    edits: impl FnOnce(&Path),
) -> (AgentTaskContext, CapturedCandidate, ValidationReport) {
    let (context, captured, report) = validated(f, controller, brief, edits);
    controller
        .agent_apply_validation(&context.identity, &report)
        .unwrap();
    assert_eq!(
        controller.agent_task().unwrap().state(),
        TaskState::CandidateReady
    );
    (context, captured, report)
}

/// The standard multi-file edit: replace, create (also in a new directory), delete and
/// an executable-bit change.
pub fn multi_file_edit(draft: &Path) {
    fs::write(draft.join("notes.txt"), "edited by the agent\n").unwrap();
    fs::create_dir_all(draft.join("media/new")).unwrap();
    fs::write(draft.join("media/new/clip.txt"), "created\n").unwrap();
    fs::write(draft.join("added.txt"), "added\n").unwrap();
    fs::remove_file(draft.join("old.txt")).unwrap();
    set_mode(&draft.join("run.sh"), 0o755);
}

// ---- observing the filesystem ---------------------------------------------------------

pub fn symlink_dir(target: &Path, link: &Path) {
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let result = std::os::windows::fs::symlink_dir(target, link);
    result.unwrap_or_else(|e| {
        panic!("creating a directory symlink failed ({e}); on Windows this needs SeCreateSymbolicLinkPrivilege (Administrator or Developer Mode)")
    });
}

pub fn symlink_file(target: &Path, link: &Path) {
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let result = std::os::windows::fs::symlink_file(target, link);
    result.unwrap_or_else(|e| {
        panic!("creating a file symlink failed ({e}); on Windows this needs SeCreateSymbolicLinkPrivilege (Administrator or Developer Mode)")
    });
}

/// A directory junction (`mklink /J`), which needs no privilege.
#[cfg(windows)]
pub fn junction(target: &Path, link: &Path) {
    let status = Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(link)
        .arg(target)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "mklink /J failed");
}

/// The modes the fixture's files read back with: on Windows a file is never executable
/// and only read-only is kept, so `run.sh` stays `0o644` after the executable-bit edit and
/// `private.txt` (writable) reads `0o644`.
#[cfg(unix)]
pub const EXEC_MODE: u32 = 0o755;
#[cfg(windows)]
pub const EXEC_MODE: u32 = 0o644;
#[cfg(unix)]
pub const PRIVATE_MODE: u32 = 0o600;
#[cfg(windows)]
pub const PRIVATE_MODE: u32 = 0o644;

/// Sets the permission bits on Unix. Windows has one permission bit, read-only, which
/// follows the write bits (as `studio_project::publish` creates files).
pub fn set_mode(path: &Path, mode: u32) {
    let mut permissions = fs::metadata(path).unwrap().permissions();
    #[cfg(unix)]
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, mode);
    #[cfg(windows)]
    permissions.set_readonly(mode & 0o222 == 0);
    fs::set_permissions(path, permissions).unwrap();
}

/// Permission bits as `studio_project::publish` reports them: `st_mode & 0o7777` on
/// Unix, synthesised from the read-only attribute on Windows.
pub fn mode_of(meta: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        std::os::unix::fs::MetadataExt::mode(meta) & 0o7777
    }
    #[cfg(windows)]
    {
        if meta.is_dir() {
            0o755
        } else if meta.permissions().readonly() {
            0o444
        } else {
            0o644
        }
    }
}

pub fn mode_at(path: &Path) -> u32 {
    mode_of(&fs::symlink_metadata(path).unwrap())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: char,
    pub mode: u32,
    pub sha: String,
}

/// Every entry below `root` (including `.git` and any transaction file), by relative
/// path, with kind, permission bits and content hash. Symlinks are recorded, not
/// followed.
pub fn snapshot(root: &Path) -> BTreeMap<String, Node> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Node>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let meta = fs::symlink_metadata(&path).unwrap();
            let mode = mode_of(&meta);
            if meta.is_dir() {
                out.insert(
                    relative,
                    Node {
                        kind: 'd',
                        mode,
                        sha: String::new(),
                    },
                );
                walk(root, &path, out);
            } else if meta.file_type().is_symlink() {
                out.insert(
                    relative,
                    Node {
                        kind: 'l',
                        mode,
                        sha: fs::read_link(&path).unwrap().to_string_lossy().into_owned(),
                    },
                );
            } else if meta.is_file() {
                out.insert(
                    relative,
                    Node {
                        kind: 'f',
                        mode,
                        sha: format!("{:x}", Sha256::digest(fs::read(&path).unwrap())),
                    },
                );
            } else {
                out.insert(
                    relative,
                    Node {
                        kind: 'o',
                        mode,
                        sha: String::new(),
                    },
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// Names of leftover transaction files.
pub fn internal_files(root: &Path) -> Vec<String> {
    snapshot(root)
        .into_keys()
        .filter(|p| p.rsplit('/').next().unwrap().starts_with(".fframes-tx-"))
        .collect()
}

/// The snapshot without internal transaction files and without empty directories the
/// source inventory does not see.
pub fn without_internal(snapshot: &BTreeMap<String, Node>) -> BTreeMap<String, Node> {
    snapshot
        .iter()
        .filter(|(p, _)| !p.rsplit('/').next().unwrap().starts_with(".fframes-tx-"))
        .map(|(p, n)| (p.clone(), n.clone()))
        .collect()
}

/// Entries of `after` that differ from `before` (added, removed or changed).
pub fn differences(before: &BTreeMap<String, Node>, after: &BTreeMap<String, Node>) -> Vec<String> {
    let mut out = Vec::new();
    for (path, node) in before {
        if after.get(path) != Some(node) {
            out.push(path.clone());
        }
    }
    for path in after.keys() {
        if !before.contains_key(path) {
            out.push(path.clone());
        }
    }
    out.sort();
    out
}

/// Paths that must never change in any transaction test.
pub fn outside_task(
    before: &BTreeMap<String, Node>,
    after: &BTreeMap<String, Node>,
    touched: &[&str],
) -> Vec<String> {
    differences(before, after)
        .into_iter()
        .filter(|p| !touched.contains(&p.as_str()))
        .filter(|p| !p.rsplit('/').next().unwrap().starts_with(".fframes-tx-"))
        .collect()
}

/// Every byte sequence found anywhere below `root`: lets a test prove that a variant
/// survived somewhere, whatever its name.
pub fn all_contents(root: &Path) -> Vec<Vec<u8>> {
    snapshot(root)
        .into_iter()
        .filter(|(_, n)| n.kind == 'f')
        .map(|(p, _)| fs::read(root.join(p)).unwrap())
        .collect()
}

pub fn survives(root: &Path, bytes: &[u8]) -> bool {
    all_contents(root).iter().any(|c| c == bytes)
}

// ---- hooks -----------------------------------------------------------------------------

/// Callback run at every recorded boundary.
pub type TapeAction = Arc<dyn Fn(usize, &Boundary) + Send + Sync>;

/// Records every boundary, optionally failing or crashing at the `n`-th (0-based).
#[derive(Clone, Default)]
pub struct Tape {
    pub seen: Arc<Mutex<Vec<Boundary>>>,
    pub crash_at: Option<usize>,
    pub io_at: Option<(usize, std::io::ErrorKind)>,
    pub action: Option<TapeAction>,
}

impl Tape {
    pub fn recording() -> Self {
        Self::default()
    }
    pub fn crashing_at(n: usize) -> Self {
        Self {
            crash_at: Some(n),
            ..Self::default()
        }
    }
    pub fn failing_at(n: usize, kind: std::io::ErrorKind) -> Self {
        Self {
            io_at: Some((n, kind)),
            ..Self::default()
        }
    }
    /// Runs `action` the first time a boundary matching `when` is reached.
    pub fn on(
        when: impl Fn(&Boundary) -> bool + Send + Sync + 'static,
        action: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
        Self {
            action: Some(Arc::new(move |_, b| {
                if when(b) && !fired.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    action();
                }
            })),
            ..Self::default()
        }
    }
    pub fn hooks(&self) -> Arc<dyn studio_engine::TransactionHooks> {
        Arc::new(self.clone())
    }
    pub fn boundaries(&self) -> Vec<Boundary> {
        self.seen.lock().clone()
    }
}

impl studio_engine::TransactionHooks for Tape {
    fn at(&self, boundary: &Boundary) -> Result<(), Fault> {
        let n = {
            let mut seen = self.seen.lock();
            seen.push(boundary.clone());
            seen.len() - 1
        };
        if let Some(action) = &self.action {
            action(n, boundary);
        }
        if self.crash_at == Some(n) {
            return Err(Fault::Crash);
        }
        if let Some((at, kind)) = self.io_at
            && at == n
        {
            return Err(Fault::Io(kind));
        }
        Ok(())
    }
}

/// Opens a controller, tolerating only the project lock being held for a moment by a
/// process another test thread is spawning: a fork/exec child briefly shares every
/// descriptor of the parent, including a just-dropped controller's lock, until it
/// execs. A genuinely held lock still fails after the grace period.
pub fn open_with_hooks_retrying(
    root: &Path,
    paths: &AppPaths,
    hooks: Arc<dyn studio_engine::TransactionHooks>,
) -> Result<Controller, EngineError> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match Controller::open_with_hooks(root, paths, hooks.clone()) {
            Err(EngineError::Diagnostic(message))
                if message.contains("already open in another controller")
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            other => return other,
        }
    }
}

pub fn open_retrying(root: &Path, paths: &AppPaths) -> Result<Controller, EngineError> {
    open_with_hooks_retrying(root, paths, Arc::new(studio_engine::NoHooks))
}

pub fn is_crash(error: &EngineError) -> bool {
    matches!(
        error,
        EngineError::Promotion(studio_engine::PromotionError::Crashed)
    )
}

/// The committed task revisions according to the durable task journal.
pub fn journal_commits(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|text| text.lines().filter(|l| l.contains("\"Commit\"")).count())
        .unwrap_or(0)
}

/// The identity `studio_project::publish` sees for a file (std's Windows file index is
/// not the same value).
pub fn file_id(path: &Path) -> studio_project::publish::FileId {
    let dir = studio_project::publish::Dir::open_root(path.parent().unwrap()).unwrap();
    let name = path.file_name().unwrap().to_str().unwrap();
    dir.stat(name).unwrap().unwrap().id
}
