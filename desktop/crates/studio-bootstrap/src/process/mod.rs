use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("process spawn failed for '{command}': {source}")]
    SpawnFailed {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("process '{command}' (pid {pid}) failed assignment to job object")]
    JobAssignmentFailed { command: String, pid: u32 },
    #[error("process '{command}' (pid {pid}) timed out during graceful termination")]
    TerminationTimeout { command: String, pid: u32 },
    #[error(
        "process '{command}' (pid {pid}) left running members in its task process group after termination"
    )]
    GroupSurvivors { command: String, pid: u32 },
    #[error("process error: {0}")]
    Custom(String),
}

/// App-local environment configuration that never mutates global PATH
/// and starts strictly from an explicit allowlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildEnvironment {
    vars: BTreeMap<String, String>,
}

impl Default for ChildEnvironment {
    fn default() -> Self {
        Self::default_allowlist()
    }
}

impl ChildEnvironment {
    /// Constructs a clean environment with only allowlisted variables from current process.
    pub fn default_allowlist() -> Self {
        let mut vars = BTreeMap::new();

        #[cfg(unix)]
        const ALLOWED_KEYS: &[&str] = &[
            "HOME",
            "USER",
            "LOGNAME",
            "SHELL",
            "LANG",
            "LC_ALL",
            "LC_CTYPE",
            "TMPDIR",
            "PATH",
            "TERM",
            "XDG_RUNTIME_DIR",
            "XDG_DATA_DIRS",
            "XDG_CONFIG_DIRS",
            "LIBCLANG_PATH",
            "DEVELOPER_DIR",
            "SDKROOT",
            "SSL_CERT_FILE",
            "SSL_CERT_DIR",
            // Package installs behind a proxy or a TLS-inspecting gateway; tools read
            // either spelling of the proxy variables.
            "HTTPS_PROXY",
            "https_proxy",
            "HTTP_PROXY",
            "http_proxy",
            "NO_PROXY",
            "no_proxy",
            "NODE_EXTRA_CA_CERTS",
        ];

        #[cfg(windows)]
        const ALLOWED_KEYS: &[&str] = &[
            "SystemRoot",
            "SystemDrive",
            "WINDIR",
            // rustc locates the MSVC linker through the Visual Studio setup instances under
            // %ProgramData% (and vswhere under Program Files) when no developer prompt is set.
            "ProgramData",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "APPDATA",
            "LOCALAPPDATA",
            "USERPROFILE",
            "COMSPEC",
            "PATHEXT",
            "TEMP",
            "TMP",
            "PATH",
            "USERNAME",
            "INCLUDE",
            "LIB",
            "LIBPATH",
            "VCToolsInstallDir",
            "VSINSTALLDIR",
            "WindowsSdkDir",
            "WindowsSDKVersion",
            "UniversalCRTSdkDir",
            "UCRTVersion",
            "LIBCLANG_PATH",
            // Package installs behind a proxy or a TLS-inspecting gateway. Names are
            // case-insensitive here, so one spelling also picks up `https_proxy` and kin.
            "HTTPS_PROXY",
            "HTTP_PROXY",
            "NO_PROXY",
            "NODE_EXTRA_CA_CERTS",
        ];

        for &key in ALLOWED_KEYS {
            if let Ok(val) = std::env::var(key) {
                vars.insert(key.to_string(), val);
            }
        }

        Self { vars }
    }

    /// Empty environment without any host variables.
    pub fn empty() -> Self {
        Self {
            vars: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, val: impl Into<String>) -> &mut Self {
        self.vars.insert(key.into(), val.into());
        self
    }

    pub fn remove(&mut self, key: &str) -> &mut Self {
        self.vars.remove(key);
        self
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(|s| s.as_str())
    }

    /// Every variable in deterministic (name) order, unredacted: for fingerprinting the
    /// exact environment a child will receive, never for logging.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.vars.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Prepend a path directory to PATH within this child environment,
    /// without modifying the host process PATH.
    pub fn prepend_path(&mut self, path: impl AsRef<Path>) -> &mut Self {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let path_key = if cfg!(windows) { "Path" } else { "PATH" };

        // Windows variable names are case-insensitive: fold every spelling (the host
        // allowlist reads `PATH`) into the one key written here, preferring that key's
        // value, so a second prepend extends the first instead of the host value.
        let current_path = if cfg!(windows) {
            let spellings: Vec<String> = self
                .vars
                .keys()
                .filter(|key| key.eq_ignore_ascii_case("PATH"))
                .cloned()
                .collect();
            let mut current = None;
            for key in spellings {
                let value = self.vars.remove(&key);
                if key == path_key || current.is_none() {
                    current = value.or(current);
                }
            }
            current.unwrap_or_default()
        } else {
            self.vars.get("PATH").cloned().unwrap_or_default()
        };
        let new_path = if current_path.is_empty() {
            path_str
        } else {
            #[cfg(unix)]
            {
                format!("{path_str}:{current_path}")
            }
            #[cfg(windows)]
            {
                format!("{path_str};{current_path}")
            }
        };

        self.vars.insert(path_key.to_string(), new_path);
        self
    }

    /// Apply the environment strictly to `Command` after clearing ambient environment.
    pub fn apply_to_command(&self, cmd: &mut Command) {
        cmd.env_clear();
        for (k, v) in &self.vars {
            cmd.env(k, v);
        }
    }

    /// Return a redacted copy of variables safe for logging/diagnostics.
    pub fn export_redacted(&self) -> BTreeMap<String, String> {
        // `PROXY`: a proxy URL may carry `user:password@`.
        let sensitive_fragments = [
            "TOKEN",
            "SECRET",
            "KEY",
            "PASS",
            "AUTH",
            "CREDENTIAL",
            "PROXY",
        ];
        self.vars
            .iter()
            .map(|(k, v)| {
                let is_sensitive = sensitive_fragments
                    .iter()
                    .any(|frag| k.to_ascii_uppercase().contains(frag));
                if is_sensitive {
                    (k.clone(), "[REDACTED]".to_string())
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect()
    }
}

pub struct SpawnOptions {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub current_dir: Option<PathBuf>,
    pub env: ChildEnvironment,
    pub stdin: Stdio,
    pub stdout: Stdio,
    pub stderr: Stdio,
    /// Test only: lets a member start a process outside the job, so escape
    /// detection can be exercised against a real escapee.
    #[cfg(all(test, windows))]
    breakaway_ok: bool,
}

impl SpawnOptions {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            env: ChildEnvironment::default_allowlist(),
            stdin: Stdio::null(),
            stdout: Stdio::piped(),
            stderr: Stdio::piped(),
            #[cfg(all(test, windows))]
            breakaway_ok: false,
        }
    }

    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.args.push(arg.as_ref().to_string_lossy().to_string());
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for a in args {
            self.arg(a);
        }
        self
    }

    pub fn current_dir(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
        self.current_dir = Some(dir.into());
        self
    }

    pub fn stdin(&mut self, stdio: Stdio) -> &mut Self {
        self.stdin = stdio;
        self
    }

    pub fn stdout(&mut self, stdio: Stdio) -> &mut Self {
        self.stdout = stdio;
        self
    }

    pub fn stderr(&mut self, stdio: Stdio) -> &mut Self {
        self.stderr = stdio;
        self
    }
}

/// Members of a task process group (job object on Windows).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupMembership {
    /// No live (non-zombie) member remains.
    Empty,
    /// Live members whose pids could be enumerated.
    Members(Vec<u32>),
    /// Live members exist but could not be enumerated on this platform.
    Present,
}

/// What one escape scan of a task tree established; see
/// [`TrackedChild::escaped_descendants`] for what each platform can see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscapeScan {
    /// Nothing outside the task scope was attributed to it. Not a proof of containment:
    /// only descendants the platform can trace are checked.
    Clean,
    /// Descendants found outside the task process group (job object on Windows).
    Escaped(Vec<u32>),
    /// The scan, or the member history it relies on, failed: escapes cannot be ruled out.
    Unverifiable(String),
}

/// Verified outcome of terminating a task tree. `group_empty` covers the task
/// process group only; it cannot see helpers that escaped it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminationReport {
    /// SIGKILL (or job termination) was required after the graceful drain.
    pub forced: bool,
    pub direct_child_exited: bool,
    pub group_empty: bool,
    /// Live group members after termination when enumerable.
    pub remaining: Vec<u32>,
}
impl TerminationReport {
    /// Direct child gone and no member left in the owned group.
    pub fn verified(&self) -> bool {
        self.direct_child_exited && self.group_empty
    }
}

/// Outcome of terminating every tree owned by a [`ProcessTreeManager`] scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeTermination {
    pub children: Vec<TerminationReport>,
}
impl ScopeTermination {
    /// Every owned tree was terminated and its group verified empty.
    pub fn verified(&self) -> bool {
        self.children.iter().all(TerminationReport::verified)
    }

    /// One report for the whole scope: forced if any tree needed it, exited/empty only
    /// if every tree was. A scope with no owned trees is trivially clean.
    pub fn merged(&self) -> TerminationReport {
        TerminationReport {
            forced: self.children.iter().any(|c| c.forced),
            direct_child_exited: self.children.iter().all(|c| c.direct_child_exited),
            group_empty: self.children.iter().all(|c| c.group_empty),
            remaining: self
                .children
                .iter()
                .flat_map(|c| c.remaining.iter().copied())
                .collect(),
        }
    }
}

/// Non-destructive snapshot of every tree a scope owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeObservation {
    /// Number of owned trees that are still alive (leader running or group non-empty).
    pub live_children: usize,
    /// Combined state of the owned trees; clean iff `live_children == 0`.
    pub termination: TerminationReport,
    /// Descendants that already left their task process group (job object on Windows).
    /// An empty list never proves containment: see
    /// [`TrackedChild::escaped_descendants`] for what each platform can see.
    pub escaped: Vec<u32>,
    /// Why escapes could not be ruled out for some owned tree (first reason seen).
    pub containment_unverified: Option<String>,
}
impl ScopeObservation {
    pub fn is_clean(&self) -> bool {
        self.live_children == 0
            && self.termination.verified()
            && self.escaped.is_empty()
            && self.containment_unverified.is_none()
    }
}

/// Whether an adapter's writers are known to stay inside the owned process group.
/// Only an explicit qualification of a specific adapter produces
/// [`WriterOwnership::ProcessGroupContained`]; escaped, background or unmodelled
/// writers are never claimed detectable, so everything else blocks candidate capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriterOwnership {
    /// Qualified: this adapter's writer descendants remain in the task process group.
    ProcessGroupContained { qualification: String },
    /// An escape from the task process group was observed.
    Detached,
    /// Not qualified; a surviving writer cannot be ruled out.
    Unknown,
}
impl WriterOwnership {
    pub fn is_qualified(&self) -> bool {
        matches!(self, Self::ProcessGroupContained { .. })
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::ProcessGroupContained { .. } => "process-group-contained",
            Self::Detached => "detached",
            Self::Unknown => "unknown",
        }
    }
}

const GROUP_VERIFY_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix::*;
#[cfg(any(windows, test))]
mod windows;
#[cfg(any(windows, test))]
use windows::*;

pub struct TrackedChild {
    pub command: String,
    pub pid: u32,
    #[cfg(unix)]
    pgid: i32,
    /// Kernel start time of the leader; distinguishes our group from a recycled id.
    #[cfg(target_os = "linux")]
    leader_start: Option<u64>,
    /// Kernel start time of the leader; distinguishes our group from a recycled id.
    #[cfg(target_os = "macos")]
    leader_start: (u64, u64),
    child: Child,
    /// The job and every process seen in it at a scan: children of members that already
    /// exited are still attributed to the task.
    #[cfg(windows)]
    scope: Arc<JobScope>,
    /// Background member recording while the tree may still be alive.
    #[cfg(windows)]
    sampler: Option<MemberSampler>,
    /// When `note_members` last recorded.
    #[cfg(windows)]
    members_noted: Mutex<Option<Instant>>,
    /// Set once the group was verified empty and the leader reaped. After this the
    /// group/job id may belong to someone else: it is never probed or signalled again.
    terminal: Option<TerminationReport>,
    os_calls: u32,
}
unsafe impl Send for TrackedChild {}
unsafe impl Sync for TrackedChild {}

impl TrackedChild {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    /// Number of signals and group probes issued so far. Diagnostic: a tracker whose
    /// group is already verified empty must never increase it.
    pub fn os_calls(&self) -> u32 {
        self.os_calls
    }

    /// Whether the group was verified empty and this tracker is permanently inert.
    pub fn is_terminated(&self) -> bool {
        self.terminal.is_some()
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, ProcessError> {
        let status = self.child.try_wait()?;
        if status.is_some() && self.terminal.is_none() {
            // The leader is reaped; if nothing else is left, latch before the id can recycle.
            self.latch_if_finished(false);
        }
        Ok(status)
    }

    pub fn wait(&mut self) -> Result<ExitStatus, ProcessError> {
        let status = self.child.wait()?;
        if self.terminal.is_none() {
            self.latch_if_finished(false);
        }
        Ok(status)
    }

    pub fn is_alive(&mut self) -> bool {
        if self.terminal.is_some() {
            return false;
        }
        if matches!(self.child.try_wait(), Ok(None)) {
            return true;
        }
        self.group_membership() != GroupMembership::Empty
    }

    /// Gracefully terminate the child process and its entire process tree/group.
    /// Sends SIGTERM (or closes job on Windows), allows up to `drain_timeout` for exit,
    /// then forces SIGKILL if any processes in the tree remain. Fails when the group
    /// still has live members after forced termination; direct-child exit alone is
    /// never reported as success.
    pub fn terminate_gracefully(&mut self, drain_timeout: Duration) -> Result<(), ProcessError> {
        let report = self.terminate_verified(drain_timeout)?;
        self.require_empty(&report)
    }

    /// Same termination as [`Self::terminate_gracefully`] but returns the verified
    /// outcome so callers can record whether forced cleanup was needed and which
    /// group members remain. The verification covers only the task process group
    /// (job object on Windows): a helper that escaped it (for example through
    /// `setsid`) is invisible here. Every wait is bounded; an unverifiable group is
    /// reported as not empty rather than assumed gone. A tracker whose group was
    /// already verified empty returns the recorded report without any OS call.
    pub fn terminate_verified(
        &mut self,
        drain_timeout: Duration,
    ) -> Result<TerminationReport, ProcessError> {
        if let Some(report) = &self.terminal {
            return Ok(report.clone());
        }
        if self.group_membership() == GroupMembership::Empty && self.reap_leader() {
            return Ok(self.latch_report(false));
        }
        #[cfg(unix)]
        {
            self.signal_group(libc::SIGTERM);
            let start = Instant::now();
            while start.elapsed() < drain_timeout {
                if self.group_membership() == GroupMembership::Empty && self.reap_leader() {
                    return Ok(self.latch_report(false));
                }
                std::thread::sleep(POLL_INTERVAL.min(drain_timeout));
            }
            self.signal_group(libc::SIGKILL);
        }
        #[cfg(windows)]
        {
            let _ = drain_timeout;
            self.terminate_job();
        }
        self.await_group_empty();
        Ok(self.report(true))
    }

    /// Forcefully kill the entire process tree immediately and verify the group is empty.
    pub fn kill_forcefully(&mut self) -> Result<(), ProcessError> {
        if self.terminal.is_some() {
            return Ok(());
        }
        #[cfg(unix)]
        self.signal_group(libc::SIGKILL);
        #[cfg(windows)]
        self.terminate_job();
        self.await_group_empty();
        let report = self.report(true);
        self.require_empty(&report)
    }

    /// Current membership of the task process group (job object on Windows).
    pub fn group_membership(&mut self) -> GroupMembership {
        if self.terminal.is_some() {
            return GroupMembership::Empty;
        }
        #[cfg(unix)]
        {
            self.os_calls += 1;
            #[cfg(target_os = "linux")]
            {
                let pgid = self.pgid;
                let start = self.leader_start;
                membership_with(
                    pgid,
                    start,
                    &mut || probe_group(pgid),
                    &mut scan_proc,
                    &|| std::thread::sleep(Duration::from_millis(3)),
                )
            }
            #[cfg(target_os = "macos")]
            {
                let pgid = self.pgid;
                let start = self.leader_start;
                mac_membership_with(
                    pgid,
                    Some(start),
                    &mut || probe_group(pgid),
                    &mut || scan_process_group(pgid),
                    &|| std::thread::sleep(Duration::from_millis(3)),
                )
            }
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            {
                match probe_group(self.pgid) {
                    Probe::Gone => GroupMembership::Empty,
                    _ => GroupMembership::Present,
                }
            }
        }
        #[cfg(windows)]
        {
            self.os_calls += 1;
            self.scope.record();
            if job_has_active_processes(self.scope.job) {
                GroupMembership::Present
            } else {
                GroupMembership::Empty
            }
        }
    }

    /// Descendants of the direct child that are no longer in the task process group (job
    /// object on Windows). Fails closed: an answer that cannot be established is
    /// `Unverifiable`, never `Clean`, and even `Clean` never proves containment.
    ///
    /// - Linux: one `/proc` scan; an incomplete scan is `Unverifiable`. An escape such as
    ///   `setsid` is seen only while the escaping parent is still alive: a double-forked
    ///   helper is reparented to init.
    /// - Windows: a process outside the job is attributed through every job member
    ///   recorded so far (a background sampler records them while the tree lives), so its
    ///   parent may have exited. A failed snapshot is `Unverifiable`, and so is any scan
    ///   once the member record has a gap (a member that could not be recorded, a failed
    ///   job query, or the member cap), unless escapes were found anyway: `Escaped` wins.
    ///   Work a pre-existing service runs on the task's behalf (COM, Task Scheduler, WMI)
    ///   is not a descendant and is never seen.
    /// - Other platforms: escapes are not scanned, so always `Clean`; containment there
    ///   rests on qualification evidence alone, as before.
    ///
    /// A tracker whose tree was verified terminated answers without any OS call: `Clean`,
    /// or `Unverifiable` when its member record has a gap. Scan before terminating.
    pub fn escaped_descendants(&self) -> EscapeScan {
        if self.terminal.is_some() {
            #[cfg(windows)]
            if let Some(reason) = self.scope.history.lock().gap.clone() {
                return EscapeScan::Unverifiable(reason);
            }
            return EscapeScan::Clean;
        }
        #[cfg(target_os = "linux")]
        {
            let scan = scan_proc();
            if !scan.complete {
                return EscapeScan::Unverifiable(
                    "the process table could not be read completely".into(),
                );
            }
            let escaped = escaped_in_scan(&scan, self.pid, self.pgid, self.leader_start);
            if escaped.is_empty() {
                EscapeScan::Clean
            } else {
                EscapeScan::Escaped(escaped)
            }
        }
        #[cfg(windows)]
        {
            self.scope.record();
            let Some(snapshot) = windows_process_snapshot() else {
                return EscapeScan::Unverifiable("the process snapshot could not be taken".into());
            };
            let (members, gap) = {
                let history = self.scope.history.lock();
                (history.ancestry(), history.gap.clone())
            };
            let job = self.scope.job;
            let escaped = escaped_from_members(&snapshot, &members, &mut |pid| {
                let ppid = snapshot.iter().find(|p| p.pid == pid).map_or(0, |p| p.ppid);
                probe_candidate(pid, ppid, job)
            });
            escape_verdict(escaped, gap.as_deref())
        }
        // No escape scan exists here. The scan only ever demotes; containment is proven by
        // qualification evidence, so the absence of a scan is not a failed scan.
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            EscapeScan::Clean
        }
    }

    /// Distinct processes besides the direct child seen in the task scope so far, from the
    /// job's member history (Windows). `None` where membership is not recorded, or when
    /// the record has a gap (a member could not be recorded, a job query failed, or the
    /// member cap was reached).
    pub fn observed_descendants(&self) -> Option<usize> {
        #[cfg(windows)]
        {
            if self.terminal.is_none() {
                self.scope.record();
            }
            self.scope.history.lock().observed(self.pid)
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    /// Records the scope's current members, at most every 100 ms; cheap enough to call on
    /// every agent event. A background sampler records them on the same period anyway;
    /// this adds a recording right at an event. A no-op where membership is not recorded.
    pub fn note_members(&self) {
        #[cfg(windows)]
        {
            if self.terminal.is_some() {
                return;
            }
            {
                let mut noted = self.members_noted.lock();
                if noted.is_some_and(|at| at.elapsed() < MEMBER_NOTE_INTERVAL) {
                    return;
                }
                *noted = Some(Instant::now());
            }
            self.scope.record();
        }
    }

    /// Stops and joins the background member sampler; the member record stays readable.
    #[cfg(windows)]
    fn stop_sampler(&mut self) {
        if let Some((stop, thread)) = self.sampler.take() {
            drop(stop);
            // A panic in the sampler leaves nothing to clean up; the record only stops
            // growing.
            let _ = thread.join();
        }
    }

    /// Makes the tracker inert: the group/job id may now belong to someone else.
    fn set_terminal(&mut self, report: TerminationReport) {
        self.terminal = Some(report);
        #[cfg(windows)]
        self.stop_sampler();
    }

    #[cfg(unix)]
    fn signal_group(&mut self, signal: i32) {
        if self.terminal.is_some() {
            return;
        }
        self.os_calls += 1;
        // SAFETY: signalling a process group this tracker created and still owns:
        // `terminal` is unset, so the group was not yet verified empty.
        unsafe {
            libc::kill(-self.pgid, signal);
        }
    }

    #[cfg(windows)]
    fn terminate_job(&mut self) {
        if self.terminal.is_some() {
            return;
        }
        self.os_calls += 1;
        // SAFETY: terminates the job object owned by this tracker.
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.scope.job, 1);
        }
    }

    /// Reaps the direct child within a bounded time; false if it did not exit.
    fn reap_leader(&mut self) -> bool {
        let deadline = Instant::now() + GROUP_VERIFY_TIMEOUT;
        loop {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn await_group_empty(&mut self) {
        let deadline = Instant::now() + GROUP_VERIFY_TIMEOUT;
        loop {
            if self.group_membership() == GroupMembership::Empty {
                self.reap_leader();
                return;
            }
            if Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    fn report(&mut self, forced: bool) -> TerminationReport {
        let membership = self.group_membership();
        let report = TerminationReport {
            forced,
            direct_child_exited: matches!(self.child.try_wait(), Ok(Some(_))),
            group_empty: membership == GroupMembership::Empty,
            remaining: match membership {
                GroupMembership::Members(pids) => pids,
                _ => Vec::new(),
            },
        };
        if report.verified() {
            self.set_terminal(report.clone());
        }
        report
    }

    fn latch_report(&mut self, forced: bool) -> TerminationReport {
        let report = TerminationReport {
            forced,
            direct_child_exited: true,
            group_empty: true,
            remaining: Vec::new(),
        };
        self.set_terminal(report.clone());
        report
    }

    /// Latches terminal emptiness when the leader is reaped and the group is empty.
    fn latch_if_finished(&mut self, forced: bool) {
        if matches!(self.child.try_wait(), Ok(Some(_)))
            && self.group_membership() == GroupMembership::Empty
        {
            self.latch_report(forced);
        }
    }

    fn require_empty(&self, report: &TerminationReport) -> Result<(), ProcessError> {
        if report.group_empty {
            Ok(())
        } else {
            Err(ProcessError::GroupSurvivors {
                command: self.command.clone(),
                pid: self.pid,
            })
        }
    }
}

#[cfg(unix)]
impl Drop for TrackedChild {
    fn drop(&mut self) {
        if self.terminal.is_some() {
            return;
        }
        if self.group_membership() != GroupMembership::Empty {
            self.signal_group(libc::SIGKILL);
            self.await_group_empty();
        }
    }
}

#[cfg(windows)]
impl Drop for TrackedChild {
    fn drop(&mut self) {
        // The sampler holds the job scope: join it first so this tracker drops the last
        // reference and closing the job (kill on close) happens here.
        self.stop_sampler();
    }
}

/// Spawns a child process with process group ownership and app-local environment isolation.
pub fn spawn_tracked(opts: SpawnOptions) -> Result<TrackedChild, ProcessError> {
    let cmd_str = opts.program.to_string_lossy().to_string();
    let mut command = Command::new(&opts.program);
    command.args(&opts.args);

    if let Some(dir) = &opts.current_dir {
        command.current_dir(dir);
    }

    opts.env.apply_to_command(&mut command);
    command.stdin(opts.stdin);
    command.stdout(opts.stdout);
    command.stderr(opts.stderr);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Make the child the group leader of a new process group (setpgid(0, 0))
        command.process_group(0);

        let child = command
            .spawn()
            .map_err(|source| ProcessError::SpawnFailed {
                command: cmd_str.clone(),
                source,
            })?;

        let pid = child.id();
        let pgid = pid as i32;
        #[cfg(target_os = "linux")]
        let leader_start = std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|stat| parse_proc_stat(pid, &stat))
            .map(|entry| entry.start);
        #[cfg(target_os = "macos")]
        let leader_start = match mac_process_start_time(pid as libc::pid_t) {
            Some(start) => start,
            None => {
                // Keep the unreaped leader's pid reserved while cleaning up this just-created
                // group; without its start time, later group-id reuse cannot be distinguished.
                // SAFETY: this process group was created for this child and its leader is not
                // reaped until after the signal, so the numeric group id cannot be recycled.
                unsafe {
                    libc::kill(-pgid, libc::SIGKILL);
                }
                let mut child = child;
                let _ = child.wait();
                return Err(ProcessError::Custom(
                    "could not identify the spawned process before tracking its group".into(),
                ));
            }
        };

        Ok(TrackedChild {
            command: cmd_str,
            pid,
            pgid,
            #[cfg(target_os = "linux")]
            leader_start,
            #[cfg(target_os = "macos")]
            leader_start,
            child,
            terminal: None,
            os_calls: 0,
        })
    }

    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
        use windows_sys::Win32::System::JobObjects::*;
        use windows_sys::Win32::System::Threading::*;

        // Create a Job Object configured to kill all processes on close
        unsafe {
            let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
            if job.is_null() {
                return Err(ProcessError::Custom(
                    "failed to create Windows JobObject".into(),
                ));
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            // A crashing member dies at once instead of handing the crash to Windows Error
            // Reporting, whose WerFault would start outside the job as an apparent escapee.
            info.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
            #[cfg(test)]
            if opts.breakaway_ok {
                info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_BREAKAWAY_OK;
            }
            let res = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if res == 0 {
                CloseHandle(job);
                return Err(ProcessError::Custom(
                    "failed to configure JobObject limit".into(),
                ));
            }

            // Spawn suspended so it cannot execute or break away before assignment, and
            // without a console window: the studio is a GUI process, so every console child
            // (cargo, the worker, .cmd adapters) would otherwise open its own.
            command.creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW);
            let child = command.spawn().map_err(|source| {
                CloseHandle(job);
                ProcessError::SpawnFailed {
                    command: cmd_str.clone(),
                    source,
                }
            })?;

            let pid = child.id();
            let proc_handle = child.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
            let assign_res = AssignProcessToJobObject(job, proc_handle);
            if assign_res == 0 {
                let _ = TerminateProcess(proc_handle, 1);
                CloseHandle(job);
                return Err(ProcessError::JobAssignmentFailed {
                    command: cmd_str,
                    pid,
                });
            }

            // Query Toolhelp to find the primary thread of the child process
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                let _ = TerminateProcess(proc_handle, 1);
                CloseHandle(job);
                return Err(ProcessError::Custom(
                    "failed to take thread snapshot for primary thread resolution".into(),
                ));
            }

            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
            let mut target_tid = 0;
            if Thread32First(snapshot, &mut entry) != 0 {
                loop {
                    if entry.th32OwnerProcessID == pid {
                        target_tid = entry.th32ThreadID;
                        break;
                    }
                    if Thread32Next(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);

            if target_tid == 0 {
                let _ = TerminateProcess(proc_handle, 1);
                CloseHandle(job);
                return Err(ProcessError::Custom(
                    "could not locate primary thread for child process".into(),
                ));
            }

            let thread_handle = OpenThread(THREAD_SUSPEND_RESUME, 0, target_tid);
            if thread_handle.is_null() {
                let _ = TerminateProcess(proc_handle, 1);
                CloseHandle(job);
                return Err(ProcessError::Custom(
                    "failed to open primary thread for child process".into(),
                ));
            }

            let resume_res = ResumeThread(thread_handle);
            CloseHandle(thread_handle);
            if resume_res == u32::MAX {
                let _ = TerminateProcess(proc_handle, 1);
                CloseHandle(job);
                return Err(ProcessError::Custom(
                    "ResumeThread failed on child primary thread".into(),
                ));
            }

            let scope = Arc::new(JobScope {
                job,
                history: Mutex::new(MemberHistory::new()),
            });
            // The leader is the first member; the sampler records the rest while it lives.
            scope.record();
            let sampler = start_member_sampler(&scope);
            Ok(TrackedChild {
                command: cmd_str,
                pid,
                child,
                scope,
                sampler,
                members_noted: Mutex::new(None),
                terminal: None,
                os_calls: 0,
            })
        }
    }
}

type ChildList = Arc<Mutex<Vec<Arc<Mutex<TrackedChild>>>>>;

/// Global/app process tree manager to ensure all spawned children are cleaned up.
#[derive(Clone, Default)]
pub struct ProcessTreeManager {
    children: ChildList,
    stopped: Arc<AtomicBool>,
    ancestors: Vec<Arc<AtomicBool>>,
    parents: Vec<ChildList>,
    lifecycle: Arc<Mutex<()>>,
}

impl ProcessTreeManager {
    pub fn new() -> Self {
        Self {
            children: Arc::new(Mutex::new(Vec::new())),
            stopped: Arc::new(AtomicBool::new(false)),
            ancestors: Vec::new(),
            parents: Vec::new(),
            lifecycle: Arc::new(Mutex::new(())),
        }
    }

    /// A terminal child scope cancels its descendants, not siblings. The root retains
    /// every process for app-wide shutdown, including nested scopes.
    pub fn sub_manager(&self) -> Self {
        let mut ancestors = self.ancestors.clone();
        ancestors.push(self.stopped.clone());
        let mut parents = self.parents.clone();
        parents.push(self.children.clone());
        Self {
            children: Arc::new(Mutex::new(Vec::new())),
            stopped: Arc::new(AtomicBool::new(false)),
            ancestors,
            parents,
            lifecycle: Arc::clone(&self.lifecycle),
        }
    }

    pub fn spawn(&self, opts: SpawnOptions) -> Result<Arc<Mutex<TrackedChild>>, ProcessError> {
        // Serialize spawn/publication with terminal shutdown so no child can be
        // created after the shutdown sweep, including by a background installer.
        let _lifecycle = self.lifecycle.lock();
        let mut list = self.children.lock();
        if self.is_shutdown() {
            return Err(ProcessError::Custom(
                "Process owner is shutting down".into(),
            ));
        }
        let tracked = spawn_tracked(opts)?;
        let arc_child = Arc::new(Mutex::new(tracked));
        // Prune exited children whose groups/jobs are also dead
        list.retain(|c| {
            if let Some(mut lock) = c.try_lock() {
                lock.is_alive()
            } else {
                true
            }
        });
        list.push(Arc::clone(&arc_child));
        for parent in &self.parents {
            let mut parent_list = parent.lock();
            parent_list.retain(|c| {
                if let Some(mut lock) = c.try_lock() {
                    lock.is_alive()
                } else {
                    true
                }
            });
            parent_list.push(Arc::clone(&arc_child));
        }
        Ok(arc_child)
    }

    pub fn terminate_all(&self, drain_timeout: Duration) {
        let _lifecycle = self.lifecycle.lock();
        self.terminate_all_locked(drain_timeout);
    }

    fn terminate_all_locked(&self, drain_timeout: Duration) -> Vec<TerminationReport> {
        let list = {
            let mut l = self.children.lock();
            std::mem::take(&mut *l)
        };
        let mut survivors = Vec::new();
        let mut terminated = Vec::new();
        let mut reports = Vec::new();
        for child_arc in list {
            let mut child = child_arc.lock();
            match child.terminate_verified(drain_timeout) {
                Ok(report) => reports.push(report),
                Err(_) => reports.push(TerminationReport {
                    forced: true,
                    direct_child_exited: false,
                    group_empty: false,
                    remaining: Vec::new(),
                }),
            }
            if child.is_alive() {
                survivors.push(Arc::clone(&child_arc));
            } else {
                terminated.push(Arc::clone(&child_arc));
            }
        }
        for parent in &self.parents {
            parent
                .lock()
                .retain(|child| !terminated.iter().any(|done| Arc::ptr_eq(child, done)));
        }
        // Preserve ownership if cleanup fails; active_count must not claim success.
        self.children.lock().extend(survivors);
        reports
    }

    /// Terminal cancellation shared by all clones; unlike terminate_all, rejects future spawns.
    pub fn shutdown(&self, drain_timeout: Duration) {
        let _ = self.shutdown_verified(drain_timeout);
    }

    /// Seals the scope against any future spawn (including from clones held by background
    /// threads), then terminates and verifies every owned tree. The returned report lists
    /// one verified outcome per owned child; an unverifiable child is reported not empty.
    pub fn shutdown_verified(&self, drain_timeout: Duration) -> ScopeTermination {
        let _lifecycle = self.lifecycle.lock();
        self.stopped.store(true, Ordering::Release);
        ScopeTermination {
            children: self.terminate_all_locked(drain_timeout),
        }
    }

    /// Seals the scope against future spawns without terminating anything.
    pub fn seal(&self) {
        let _lifecycle = self.lifecycle.lock();
        self.stopped.store(true, Ordering::Release);
    }

    /// Observes every owned tree without signalling or reaping it. Escapes are read
    /// while their parents are still alive, so call this before any termination.
    /// Serialized with spawn and shutdown: with the scope sealed the answer cannot go
    /// stale through a new spawn.
    pub fn observe(&self) -> ScopeObservation {
        let _lifecycle = self.lifecycle.lock();
        let list: Vec<_> = self.children.lock().iter().cloned().collect();
        let mut observation = ScopeObservation {
            live_children: 0,
            termination: TerminationReport {
                forced: false,
                direct_child_exited: true,
                group_empty: true,
                remaining: Vec::new(),
            },
            escaped: Vec::new(),
            containment_unverified: None,
        };
        for child_arc in list {
            let mut child = child_arc.lock();
            if child.is_terminated() {
                continue;
            }
            match child.escaped_descendants() {
                EscapeScan::Clean => {}
                EscapeScan::Escaped(pids) => observation.escaped.extend(pids),
                EscapeScan::Unverifiable(reason) => {
                    observation.containment_unverified.get_or_insert(reason);
                }
            }
            let exited = matches!(child.child_mut().try_wait(), Ok(Some(_)));
            let membership = child.group_membership();
            if exited && membership == GroupMembership::Empty {
                continue;
            }
            observation.live_children += 1;
            observation.termination.direct_child_exited &= exited;
            match membership {
                GroupMembership::Empty => {}
                GroupMembership::Members(pids) => {
                    observation.termination.group_empty = false;
                    observation.termination.remaining.extend(pids);
                }
                GroupMembership::Present => observation.termination.group_empty = false,
            }
        }
        observation
    }

    pub fn is_shutdown(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
            || self
                .ancestors
                .iter()
                .any(|flag| flag.load(Ordering::Acquire))
    }

    pub fn active_count(&self) -> usize {
        let mut list = self.children.lock();
        list.retain(|c| {
            if let Some(mut lock) = c.try_lock() {
                lock.is_alive()
            } else {
                true
            }
        });
        list.len()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod job_escape_tests;
