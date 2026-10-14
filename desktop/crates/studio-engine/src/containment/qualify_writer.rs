//! `fframes-studio qualify-writer`: measures whether a real adapter's writer processes stay
//! inside the task scope (process group, job object on Windows) and records the result as
//! authentic `m6-authentic/1` evidence in the installed ledger
//! (`<data>/qualification/m6-results.json`), the only input that may make a writer
//! `ProcessGroupContained` (see [`crate::containment`]).
//!
//! Three scenarios run against the configured adapter in a throwaway folder, each with a
//! fresh adapter and scope, all within one time limit for the whole run:
//! * `edit`: the agent creates and changes a file; the file must change;
//! * `stop`: the agent runs a long shell command; Stop is sent while it runs;
//! * `provider_crash`: the same command; the adapter's own process is killed while it runs.
//!
//! A scenario counts as contained only when no escaped descendant was seen and none could
//! have gone unseen, the scope was verified empty afterwards, and (Stop and crash) a
//! subprocess actually ran when it was
//! interrupted: zero descendants means nothing was measured, which is a failure. The gate
//! passes only when every scenario was contained. What the record claims is exactly the
//! gate's criteria; the stated limits are recorded with it. Prompts, agent text and
//! credential values are never written.
use crate::app_paths::AppPaths;
use crate::containment::{
    FIXTURE_AGENT_NAMES, LEDGER_DIR, LEDGER_FILE_M6, Platform, hash_file, launch_identity,
};
use crate::providers::{
    host::{AdapterFile, adapter_search},
    provider_profiles::ProviderRegistry,
};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use studio_bootstrap::{ProcessTreeManager, WriterOwnership};
use studio_harness::{
    AcpDriver, AdapterLaunch, AgentEventKind, DriverConfig, DriverLimits, DriverMode,
    McpStdioSupport,
    driver::{PermissionChoice, PermissionReply},
};

pub const GATE: &str = "interruption_and_cleanup";
pub const SCHEMA: &str = "m6-authentic/1";
const CRITERIA: &str = "Enforces verified process group termination on user Stop or adapter crash with zero escaped descendants.";
const LIMITS: &str = "Measures the adapter's own process tree in its task scope (job object on Windows, process group on Linux). Work a pre-existing service performs on the adapter's behalf (COM servers, Task Scheduler, WMI, daemons) is not a descendant and is not observed. Bound to this exact adapter launch and platform; an adapter update needs a new run.";
const EDIT_FILE: &str = "qualify-edit.txt";
/// How long the shell command must have been running before it is interrupted.
const SETTLE: Duration = Duration::from_secs(3);
/// How long an install waits for another run to release the ledger lock.
const LOCK_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Edit,
    Stop,
    Crash,
}

impl Act {
    fn name(self) -> &'static str {
        match self {
            Self::Edit => "edit",
            Self::Stop => "stop",
            Self::Crash => "provider_crash",
        }
    }
}

/// What one scenario measured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub act: Act,
    /// Scope members besides the adapter before the prompt (its own helpers).
    pub baseline_descendants: usize,
    pub descendants_observed: usize,
    pub escaped: usize,
    /// Why escapes could not be ruled out (the driver's escape scan failed, or the scope's
    /// member record has a gap), if they could not.
    pub containment_unverified: Option<String>,
    pub group_empty_after: bool,
    /// Edit: the file changed. Stop/crash: the interruption happened while a tool ran.
    pub exercised: bool,
    pub note: String,
}

impl Scenario {
    fn failed(act: Act, note: impl Into<String>) -> Self {
        Self {
            act,
            baseline_descendants: 0,
            descendants_observed: 0,
            escaped: 0,
            containment_unverified: None,
            group_empty_after: false,
            exercised: false,
            note: note.into(),
        }
    }

    /// Why this scenario does not count as contained (every reason), if it does not.
    pub fn refusal(&self) -> Option<String> {
        let mut reasons = Vec::new();
        if self.escaped > 0 {
            reasons.push(format!(
                "{} process(es) escaped the task scope",
                self.escaped
            ));
        }
        if let Some(reason) = &self.containment_unverified {
            reasons.push(format!("containment could not be verified: {reason}"));
        }
        if !self.group_empty_after {
            reasons.push("the task scope was not verified empty afterwards".into());
        }
        if !self.exercised {
            reasons.push(if self.note.is_empty() {
                "the scenario did not run to the point it measures".into()
            } else {
                self.note.clone()
            });
        } else if self.act != Act::Edit && self.descendants_observed <= self.baseline_descendants {
            reasons
                .push("no new subprocess started during the turn, so nothing was measured".into());
        }
        (!reasons.is_empty()).then(|| reasons.join("; "))
    }
}

/// The adapter as measured.
#[derive(Debug, Clone)]
pub struct Measured {
    pub profile: String,
    pub agent_name: String,
    pub protocol_version: u16,
    pub launch_identity: String,
    pub platform: Platform,
    pub scenarios: Vec<Scenario>,
    pub owned_processes_after: usize,
    pub development: bool,
}

impl Measured {
    pub fn passed(&self) -> bool {
        !self.development
            && self.owned_processes_after == 0
            && self.scenarios.iter().all(|s| s.refusal().is_none())
            && [Act::Edit, Act::Stop, Act::Crash]
                .iter()
                .all(|act| self.scenarios.iter().any(|s| s.act == *act))
    }
}

/// The evidence record (`m6-authentic/1`).
pub fn record(measured: &Measured, measured_unix: u64) -> Value {
    json!({
        "schema": SCHEMA,
        "gate": GATE,
        "evidence_kind": if measured.development { "development" } else { "authentic" },
        "fixture_only": measured.development,
        "measured_unix": measured_unix,
        "platform": {
            "system": measured.platform.system,
            "machine": measured.platform.machine,
        },
        "adapter": {
            "agent_name": measured.agent_name,
            "launch_identity": measured.launch_identity,
            "protocol_version": measured.protocol_version,
        },
        "measurements": {
            "scenarios": measured.scenarios.iter().map(|s| json!({
                "name": s.act.name(),
                "baseline_descendants": s.baseline_descendants,
                "descendants_observed": s.descendants_observed,
                "escaped_descendants": s.escaped,
                "containment_unverified": s.containment_unverified,
                "group_empty_after": s.group_empty_after,
                "exercised": s.exercised,
                "contained": s.refusal().is_none(),
                "note": s.refusal().unwrap_or_default(),
            })).collect::<Vec<_>>(),
        },
        "cleanup": { "owned_processes_after": measured.owned_processes_after },
        "limits": LIMITS,
    })
}

/// Writes the record into `<ledger_dir>/evidence/` and upserts this provider's entry of
/// `m6-results.json`; every other provider and gate is kept. Both files are replaced
/// atomically, the record under a name no other run uses, and the ledger's
/// read-modify-write holds an exclusive lock so concurrent runs never lose an update.
/// Returns the record's path.
pub fn install(ledger_dir: &Path, measured: &Measured) -> Result<PathBuf, String> {
    let stamp = now_unix();
    let evidence_dir = ledger_dir.join("evidence");
    std::fs::create_dir_all(&evidence_dir).map_err(|e| format!("evidence folder: {e}"))?;
    let file_name = format!(
        "writer-{}-{stamp}-{}.json",
        safe_name(&measured.profile),
        unique_tag()
    );
    let record_path = evidence_dir.join(&file_name);
    let bytes = serde_json::to_vec_pretty(&record(measured, stamp)).map_err(|e| e.to_string())?;
    write_atomic(&record_path, &bytes)?;
    let digest = hash_file(&record_path).map_err(|e| format!("hash the record: {e}"))?;

    let ledger_path = ledger_dir.join(LEDGER_FILE_M6);
    let _lock = LedgerLock::acquire(&ledger_path)?;
    let mut ledger = match std::fs::read(&ledger_path) {
        Ok(bytes) => serde_json::from_slice::<Value>(&bytes)
            .ok()
            .filter(|l| l["kind"] == "m6" && l["schema_version"] == 1)
            .ok_or("the installed M6 ledger is not a valid M6 ledger; move it aside first")?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({
            "kind": "m6",
            "schema_version": 1,
            "environment": {
                "scope": "Installed on this computer by fframes-studio qualify-writer; only measured gates are recorded.",
            },
            "providers": {},
        }),
        Err(e) => return Err(format!("read the M6 ledger: {e}")),
    };
    let passed = measured.passed();
    let notes = if passed {
        "Measured by fframes-studio qualify-writer: every scenario contained.".to_owned()
    } else {
        let reasons: Vec<String> = measured
            .scenarios
            .iter()
            .filter_map(|s| s.refusal().map(|r| format!("{}: {r}", s.act.name())))
            .collect();
        format!("Not contained: {}", reasons.join("; "))
    };
    let providers = ledger["providers"]
        .as_object_mut()
        .ok_or("the M6 ledger has no providers object")?;
    let entry = providers
        .entry(measured.profile.clone())
        .or_insert_with(|| json!({ "id": measured.profile, "gates": {} }));
    entry["launch_identity"] = json!(measured.launch_identity);
    entry["status"] = json!(if passed { "pass" } else { "fail" });
    if !entry["gates"].is_object() {
        entry["gates"] = json!({});
    }
    entry["gates"][GATE] = json!({
        "kind": if measured.development { "development" } else { "authentic" },
        "status": if passed { "pass" } else { "fail" },
        "criteria": CRITERIA,
        "notes": notes,
        "evidence": [{ "path": format!("evidence/{file_name}"), "sha256": digest }],
    });
    ledger["timestamp_unix"] = json!(stamp);
    let bytes = serde_json::to_vec_pretty(&ledger).map_err(|e| e.to_string())?;
    write_atomic(&ledger_path, &bytes)?;
    Ok(record_path)
}

/// An exclusive lock file next to the ledger, removed when dropped. A lock left behind by
/// a run that was killed is named in the error so it can be removed by hand.
struct LedgerLock(PathBuf);

impl LedgerLock {
    fn acquire(ledger_path: &Path) -> Result<Self, String> {
        let path = ledger_path.with_extension("json.lock");
        let deadline = Instant::now() + LOCK_WAIT;
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => return Ok(Self(path)),
                Err(e) if e.kind() == ErrorKind::AlreadyExists && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    return Err(format!(
                        "another qualify-writer run holds the ledger lock {}; remove it if no run is active",
                        path.display()
                    ));
                }
                Err(e) => return Err(format!("lock the M6 ledger: {e}")),
            }
        }
    }
}

impl Drop for LedgerLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// A name part no other run or call uses: process id, time and a per-process counter.
fn unique_tag() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!(
        "{}-{nanos}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let tmp = path.with_extension(format!("{}.tmp", unique_tag()));
    let result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)
    })();
    result.map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("write {}: {e}", path.display())
    })
}

fn safe_name(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

// ---- the live run ---------------------------------------------------------------------------

pub struct Options {
    /// Registry profile id; the selected profile when `None`.
    pub profile: Option<String>,
    /// Time limit for the whole run (all three scenarios together).
    pub timeout: Duration,
    /// Development run against a fixture: the bundle goes to this folder, never the
    /// installed ledger, and is marked `fixture_only`. Refused inside the installed
    /// ledger folder (see [`development_dir`]).
    pub development_out: Option<PathBuf>,
}

/// Runs the scenarios and installs the result. `Ok` carries the measurement and where
/// the record was written, whether or not the gate passed.
pub fn run(paths: &AppPaths, options: &Options) -> Result<(Measured, PathBuf), String> {
    let deadline = Instant::now()
        .checked_add(options.timeout)
        .ok_or("the time limit is too large")?;
    let development_out = options
        .development_out
        .as_deref()
        .map(|out| development_dir(paths, out))
        .transpose()?;
    let registry = crate::providers::host::load_registry(paths)?
        .ok_or("no provider is configured; select one in Setup first")?;
    let (profile, file) = pick_profile(&registry, options.profile.as_deref())?;
    let identity = launch_identity(&file, paths)?;
    let launch = AdapterLaunch::resolve(
        &studio_harness::AdapterConfig {
            executable: file.executable.clone(),
            args: file.args.clone(),
            auth_env_names: file.auth_env_names.clone(),
        },
        &adapter_search(paths),
    )
    .map_err(|e| format!("the adapter cannot be started: {e}"))?;
    let launch = match &file.auth_method {
        Some(method) => launch.with_auth_method(method.clone()),
        None => launch,
    };

    let scratch = std::env::temp_dir().join(format!("fframes-qualify-writer-{}", unique_tag()));
    std::fs::create_dir_all(&scratch).map_err(|e| format!("scratch folder: {e}"))?;
    std::fs::write(
        scratch.join("README.txt"),
        "Scratch folder for a writer-containment run.\n",
    )
    .map_err(|e| format!("scratch folder: {e}"))?;
    let scratch = scratch
        .canonicalize()
        .map_err(|e| format!("scratch folder: {e}"))?;
    let ctx = Ctx {
        provider: profile.clone(),
        prompt,
        launch,
        cwd: scratch.clone(),
        deadline,
    };

    let mut agent: Option<(String, u16)> = None;
    let mut scenarios = Vec::new();
    let mut owned_after = 0;
    for act in [Act::Edit, Act::Stop, Act::Crash] {
        eprintln!("qualify-writer: running the {} scenario…", act.name());
        let (scenario, info, left) = run_scenario(act, &ctx);
        if let Some(info) = info {
            if development_out.is_none()
                && FIXTURE_AGENT_NAMES.contains(&info.0.to_ascii_lowercase().as_str())
            {
                let _ = std::fs::remove_dir_all(&scratch);
                return Err(format!(
                    "{} is a scripted fixture; it can only be run with --development",
                    info.0
                ));
            }
            agent.get_or_insert(info);
        }
        owned_after += left;
        eprintln!(
            "qualify-writer: {} {}",
            act.name(),
            scenario
                .refusal()
                .map_or("contained".into(), |r| format!("NOT contained: {r}"))
        );
        scenarios.push(scenario);
    }
    let _ = std::fs::remove_dir_all(&scratch);
    let (agent_name, protocol_version) =
        agent.ok_or("the adapter never completed its handshake; nothing was measured")?;
    let measured = Measured {
        profile,
        agent_name,
        protocol_version,
        launch_identity: identity.digest(),
        platform: Platform::current(),
        scenarios,
        owned_processes_after: owned_after,
        development: development_out.is_some(),
    };
    let ledger_dir = development_out.unwrap_or_else(|| paths.data.join(LEDGER_DIR));
    let path = install(&ledger_dir, &measured)?;
    Ok((measured, path))
}

/// The absolute folder of a development run's `--out`. Refused when it is, or lies inside,
/// the installed ledger folder `<data>/qualification`: development evidence never goes
/// next to the authentic ledger. Nothing is created here.
pub fn development_dir(paths: &AppPaths, out: &Path) -> Result<PathBuf, String> {
    let out = resolve_for_comparison(out)?;
    let ledger = resolve_for_comparison(&paths.data.join(LEDGER_DIR))?;
    if is_within(&out, &ledger) {
        return Err(format!(
            "--out {} is inside the installed ledger folder {}; choose another folder",
            out.display(),
            ledger.display()
        ));
    }
    Ok(out)
}

/// `path` made absolute with its existing part canonicalized (links and `..` resolved);
/// the part that does not exist yet is appended as written and may not contain `..`.
fn resolve_for_comparison(path: &Path) -> Result<PathBuf, String> {
    let absolute = std::path::absolute(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut existing = absolute.as_path();
    let mut missing = Vec::new();
    while std::fs::symlink_metadata(existing).is_err() {
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                missing.push(name.to_owned());
                existing = parent;
            }
            _ => {
                return Err(format!(
                    "{}: `..` is only allowed in folders that already exist",
                    path.display()
                ));
            }
        }
    }
    let mut resolved = existing
        .canonicalize()
        .map_err(|e| format!("{}: {e}", existing.display()))?;
    resolved.extend(missing.iter().rev());
    Ok(resolved)
}

/// `path` is `dir` or lies inside it; components compare case-insensitively on Windows.
fn is_within(path: &Path, dir: &Path) -> bool {
    let same = |a: Component, b: Component| {
        if cfg!(windows) {
            a.as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&b.as_os_str().to_string_lossy())
        } else {
            a == b
        }
    };
    let mut parts = path.components();
    dir.components()
        .all(|d| parts.next().is_some_and(|p| same(p, d)))
}

fn pick_profile(
    registry: &ProviderRegistry,
    id: Option<&str>,
) -> Result<(String, AdapterFile), String> {
    let profile = match id {
        Some(id) => registry.profiles.iter().find(|p| p.id == id),
        None => registry.selected_profile(),
    }
    .ok_or("the provider profile was not found; select one in Setup or pass --profile")?;
    Ok((profile.id.clone(), profile.adapter.clone()))
}

struct Ctx {
    provider: String,
    prompt: fn(Act) -> String,
    launch: AdapterLaunch,
    cwd: PathBuf,
    /// The end of the whole run.
    deadline: Instant,
}

fn prompt(act: Act) -> String {
    let long_command = if cfg!(windows) {
        "ping -n 120 127.0.0.1"
    } else {
        "sleep 120"
    };
    match act {
        Act::Edit => format!(
            "Create a file named {EDIT_FILE} in the current folder containing exactly the text `contained`. Then change that file so it contains exactly `contained twice`. Do nothing else."
        ),
        Act::Stop | Act::Crash => format!(
            "Run this exact shell command in the current folder with your shell tool and wait for it to finish: {long_command}. Do nothing else."
        ),
    }
}

/// An allow option, preferring a one-time allow.
fn allow_choice(options: &[PermissionChoice]) -> Option<String> {
    options
        .iter()
        .find(|o| o.kind == "allow_once")
        .or_else(|| options.iter().find(|o| o.kind.starts_with("allow")))
        .map(|o| o.option_id.clone())
}

/// What a scenario saw before its adapter was shut down.
#[derive(Default)]
struct Sampled {
    baseline: usize,
    descendants: usize,
    /// The scope's member record had a gap at some sample.
    recording_gap: bool,
    exercised: bool,
    note: String,
}

/// Shuts the scenario's adapter down and completes the measurement with what the shutdown
/// verified: escapes, unverifiable containment and the scope's termination.
fn conclude(
    act: Act,
    driver: AcpDriver,
    manager: &ProcessTreeManager,
    sampled: Sampled,
) -> (Scenario, usize) {
    let outcome = driver.shutdown();
    let left = manager.active_count();
    let containment_unverified = outcome.containment_unverified.or_else(|| {
        sampled.recording_gap.then(|| {
            "the task scope's member record has a gap, so a descendant may have gone unseen"
                .to_owned()
        })
    });
    let scenario = Scenario {
        act,
        baseline_descendants: sampled.baseline,
        descendants_observed: sampled.descendants,
        escaped: outcome.escaped_pids.len(),
        containment_unverified,
        group_empty_after: outcome.termination.verified(),
        exercised: sampled.exercised,
        note: sampled.note,
    };
    (scenario, left)
}

/// One scenario with its own adapter and scope. Returns the measurement, the agent's
/// `(name, protocol version)` once the handshake finished, and the processes the scope
/// still owned afterwards.
fn run_scenario(act: Act, ctx: &Ctx) -> (Scenario, Option<(String, u16)>, usize) {
    if Instant::now() >= ctx.deadline {
        return (
            Scenario::failed(act, "the run's time limit ran out before this scenario"),
            None,
            0,
        );
    }
    let manager = ProcessTreeManager::new();
    let config = DriverConfig {
        provider: ctx.provider.clone(),
        task: format!("qualify-writer-{}", act.name()),
        cwd: ctx.cwd.clone(),
        launch: ctx.launch.clone(),
        limits: DriverLimits::default(),
        resume_session: None,
        writer_ownership: WriterOwnership::Unknown,
        mode: DriverMode::Full,
        mcp_servers: Vec::new(),
        mcp_stdio: McpStdioSupport::Unsupported,
    };
    let driver = match AcpDriver::start(config, &manager) {
        Ok(driver) => driver,
        Err(failure) => {
            let left = manager.active_count();
            return (
                Scenario::failed(
                    act,
                    format!("the adapter did not start: {}", failure.message),
                ),
                None,
                left,
            );
        }
    };
    let info = match driver.wait_ready(ctx.deadline.saturating_duration_since(Instant::now())) {
        Ok(session) => Some((
            session.initialized.agent_name.clone(),
            session.initialized.protocol_version,
        )),
        Err(failure) => {
            let note = format!("no session: {}", failure.message);
            let (scenario, left) = conclude(
                act,
                driver,
                &manager,
                Sampled {
                    note,
                    ..Default::default()
                },
            );
            return (scenario, None, left);
        }
    };

    // The adapter may keep helpers of its own: only processes started after the prompt
    // count, so the baseline is taken before it is sent.
    let Some(baseline) = driver.observed_descendants() else {
        let note = "the task scope's members are not recorded here (or the record has a gap), so nothing can be measured".to_owned();
        let (scenario, left) = conclude(
            act,
            driver,
            &manager,
            Sampled {
                note,
                ..Default::default()
            },
        );
        return (scenario, info, left);
    };
    let mut sampled = Sampled {
        baseline,
        descendants: baseline,
        ..Default::default()
    };
    if let Err(error) = driver.prompt(&(ctx.prompt)(act)) {
        sampled.note = format!("prompt refused: {error}");
        let (scenario, left) = conclude(act, driver, &manager, sampled);
        return (scenario, info, left);
    }

    // A shell (`execute`) tool when the adapter reports tool kinds, else any tool.
    let mut execute_since: Option<Instant> = None;
    let mut tool_since: Option<Instant> = None;
    let mut kinds_reported = false;
    let mut interrupted = false;
    let mut finished = false;
    let mut note = String::new();
    let sample = |sampled: &mut Sampled| match driver.observed_descendants() {
        Some(seen) => sampled.descendants = sampled.descendants.max(seen),
        None => sampled.recording_gap = true,
    };
    while Instant::now() < ctx.deadline {
        sample(&mut sampled);
        match driver
            .next_event(Duration::from_millis(200))
            .map(|e| e.kind)
        {
            Some(AgentEventKind::PermissionRequested(request)) => {
                let reply = allow_choice(&request.options)
                    .map_or(PermissionReply::Cancel, PermissionReply::Select);
                if let Err(error) = driver.reply_permission(request.id, reply) {
                    note = format!("the permission request could not be answered: {error}");
                    break;
                }
            }
            Some(AgentEventKind::ToolCall(tool) | AgentEventKind::ToolUpdate(tool)) => {
                tool_since.get_or_insert_with(Instant::now);
                if let Some(kind) = &tool.kind {
                    kinds_reported = true;
                    if kind == "execute" {
                        execute_since.get_or_insert_with(Instant::now);
                    }
                }
            }
            Some(AgentEventKind::PromptFinished(_)) => {
                finished = true;
                if act == Act::Edit || interrupted {
                    break;
                }
                note = "the turn ended before the command could be interrupted".into();
                break;
            }
            Some(AgentEventKind::Failure(failure)) => {
                if !interrupted {
                    note = format!("the adapter failed: {}", failure.message);
                }
                break;
            }
            Some(AgentEventKind::Closed(_)) => break,
            _ => {}
        }
        let running_since = if kinds_reported {
            execute_since
        } else {
            tool_since
        };
        if act != Act::Edit
            && !interrupted
            && sampled.descendants > baseline
            && running_since.is_some_and(|t| t.elapsed() >= SETTLE)
        {
            interrupted = true;
            let result = match act {
                Act::Stop => driver.cancel_prompt().map_err(|e| e.to_string()),
                _ => driver
                    .crash_adapter_for_qualification()
                    .map_err(|e| e.to_string()),
            };
            if let Err(error) = result {
                note = format!("could not interrupt: {error}");
                break;
            }
        }
    }
    sample(&mut sampled);
    let exercised = note.is_empty()
        && match act {
            Act::Edit => {
                finished
                    && std::fs::read_to_string(ctx.cwd.join(EDIT_FILE))
                        .is_ok_and(|text| text.trim() == "contained twice")
            }
            Act::Stop | Act::Crash => interrupted,
        };
    if note.is_empty() && !exercised {
        note = match act {
            Act::Edit if !finished => "the turn did not finish in time".into(),
            Act::Edit => format!("{EDIT_FILE} does not hold the requested text"),
            _ => "no shell command was seen running before the time limit".into(),
        };
    }
    sampled.exercised = exercised;
    sampled.note = note;
    let (scenario, left) = conclude(act, driver, &manager, sampled);
    (scenario, info, left)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::containment::resolve_with;

    const DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn contained(act: Act) -> Scenario {
        Scenario {
            act,
            baseline_descendants: 1,
            descendants_observed: 2,
            escaped: 0,
            containment_unverified: None,
            group_empty_after: true,
            exercised: true,
            note: String::new(),
        }
    }

    fn measured() -> Measured {
        Measured {
            profile: "claude".into(),
            agent_name: "claude-agent-acp".into(),
            protocol_version: 1,
            launch_identity: DIGEST.into(),
            platform: Platform::current(),
            scenarios: vec![
                contained(Act::Edit),
                contained(Act::Stop),
                contained(Act::Crash),
            ],
            owned_processes_after: 0,
            development: false,
        }
    }

    fn qualifies(dir: &Path) -> Result<(), String> {
        let resolution = resolve_with(dir, DIGEST, &Platform::current());
        if resolution.ownership.is_qualified() {
            Ok(())
        } else {
            Err(resolution.summary())
        }
    }

    #[test]
    fn an_installed_contained_run_qualifies_the_exact_launch() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path(), &measured()).unwrap();
        qualifies(dir.path()).unwrap();
        let other = resolve_with(dir.path(), &"2".repeat(64), &Platform::current());
        assert!(!other.ownership.is_qualified());
    }

    #[test]
    fn any_uncontained_scenario_fails_the_gate() {
        let cases: [fn(&mut Scenario); 5] = [
            |s| s.escaped = 1,
            |s| s.containment_unverified = Some("the escape scan failed".into()),
            |s| s.group_empty_after = false,
            |s| s.descendants_observed = s.baseline_descendants,
            |s| s.exercised = false,
        ];
        for (index, change) in cases.iter().enumerate() {
            let dir = tempfile::tempdir().unwrap();
            let mut m = measured();
            change(&mut m.scenarios[1]);
            install(dir.path(), &m).unwrap();
            assert!(qualifies(dir.path()).is_err(), "case {index}");
        }
    }

    #[test]
    fn missing_scenarios_development_runs_and_leftover_processes_never_qualify() {
        let mut short = measured();
        short.scenarios.pop();
        let mut development = measured();
        development.development = true;
        let mut leftovers = measured();
        leftovers.owned_processes_after = 1;
        for m in [short, development, leftovers] {
            let dir = tempfile::tempdir().unwrap();
            install(dir.path(), &m).unwrap();
            assert!(qualifies(dir.path()).is_err());
        }
    }

    #[test]
    fn concurrent_installs_keep_every_update() {
        let dir = tempfile::tempdir().unwrap();
        let profiles = ["a", "b", "c", "d"];
        std::thread::scope(|scope| {
            for profile in profiles {
                let dir = dir.path();
                scope.spawn(move || {
                    let mut m = measured();
                    m.profile = profile.into();
                    install(dir, &m).unwrap();
                });
            }
        });
        let ledger: Value =
            serde_json::from_slice(&std::fs::read(dir.path().join(LEDGER_FILE_M6)).unwrap())
                .unwrap();
        for profile in profiles {
            assert_eq!(ledger["providers"][profile]["status"], "pass", "{profile}");
        }
        let evidence = std::fs::read_dir(dir.path().join("evidence"))
            .unwrap()
            .count();
        assert_eq!(evidence, profiles.len());
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".lock") || name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn development_output_inside_the_installed_ledger_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path().join("data")).unwrap();
        let ledger = paths.data.join(LEDGER_DIR);
        let inside = [
            ledger.clone(),
            ledger.join("dev"),
            ledger.join("new").join("deeper"),
            paths.data.join("new").join("..").join(LEDGER_DIR),
        ];
        for out in &inside {
            assert!(development_dir(&paths, out).is_err(), "{}", out.display());
        }
        assert!(!ledger.exists(), "a refused folder is never created");
        std::fs::create_dir_all(ledger.join("dev")).unwrap();
        for out in &inside {
            assert!(development_dir(&paths, out).is_err(), "{}", out.display());
        }
        #[cfg(windows)]
        assert!(development_dir(&paths, &paths.data.join("QUALIFICATION").join("x")).is_err());

        let outside = temp.path().join("dev-out");
        assert_eq!(
            development_dir(&paths, &outside).unwrap(),
            temp.path().canonicalize().unwrap().join("dev-out")
        );
        assert!(development_dir(&paths, &paths.data.join("qualification-dev")).is_ok());
        assert!(!outside.exists());
    }

    #[test]
    fn a_tampered_record_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = install(dir.path(), &measured()).unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.push(b' ');
        std::fs::write(&path, bytes).unwrap();
        assert!(qualifies(dir.path()).is_err());
    }

    #[test]
    fn the_upsert_keeps_other_providers_and_gates() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(LEDGER_FILE_M6),
            serde_json::to_vec(&json!({
                "kind": "m6",
                "schema_version": 1,
                "providers": {
                    "codex": { "id": "codex", "status": "not_run", "gates": {} },
                    "claude": { "id": "claude", "gates": { "restoration_and_fallback": { "status": "not_run" } } },
                },
            }))
            .unwrap(),
        )
        .unwrap();
        install(dir.path(), &measured()).unwrap();
        let ledger: Value =
            serde_json::from_slice(&std::fs::read(dir.path().join(LEDGER_FILE_M6)).unwrap())
                .unwrap();
        assert_eq!(ledger["providers"]["codex"]["status"], "not_run");
        assert_eq!(
            ledger["providers"]["claude"]["gates"]["restoration_and_fallback"]["status"],
            "not_run"
        );
        assert_eq!(
            ledger["providers"]["claude"]["gates"][GATE]["status"],
            "pass"
        );
        qualifies(dir.path()).unwrap();
    }

    /// Development run of the live scenario code against the scripted ACP peer: a tool
    /// card, a helper subprocess and a hang stand in for the real agent's shell command.
    /// The peer is a Python script run by the resolved test interpreter. Never authentic
    /// evidence.
    #[test]
    #[cfg(windows)]
    fn the_scenarios_measure_the_scripted_peer_and_its_helper() {
        fn script(act: Act) -> String {
            match act {
                Act::Edit => {
                    r#"SCRIPT {"tool":1,"write":{"qualify-edit.txt":"contained twice"}}"#.into()
                }
                Act::Stop | Act::Crash => {
                    r#"SCRIPT {"tool":[{"kind":"execute"}],"helper":true,"hang":true}"#.into()
                }
            }
        }
        let peer_root = tempfile::tempdir().unwrap();
        let cwd = tempfile::tempdir().unwrap();
        let ctx = Ctx {
            provider: "scripted".into(),
            prompt: script,
            launch: AdapterLaunch {
                executable: studio_test_support::python().to_path_buf(),
                args: vec![
                    format!("{}/tests/support/acp-agent.py", env!("CARGO_MANIFEST_DIR")),
                    peer_root.path().to_string_lossy().into_owned(),
                ],
                env: studio_bootstrap::ChildEnvironment::default_allowlist(),
                secrets: Vec::new(),
                auth_names: Vec::new(),
                auth_method: None,
            },
            cwd: cwd.path().canonicalize().unwrap(),
            deadline: Instant::now() + Duration::from_secs(180),
        };
        for act in [Act::Edit, Act::Stop, Act::Crash] {
            let (scenario, info, left) = run_scenario(act, &ctx);
            assert_eq!(info.as_ref().map(|i| i.1), Some(1), "{act:?}: {scenario:?}");
            assert_eq!(scenario.refusal(), None, "{act:?}: {scenario:?}");
            assert_eq!(left, 0, "{act:?}");
            if act != Act::Edit {
                assert!(
                    scenario.descendants_observed > scenario.baseline_descendants,
                    "{act:?}: {scenario:?}"
                );
            }
        }
    }

    #[test]
    fn allow_once_is_preferred_and_a_reject_only_prompt_is_declined() {
        let choice = |id: &str, kind: &str| PermissionChoice {
            option_id: id.into(),
            name: id.into(),
            kind: kind.into(),
        };
        assert_eq!(
            allow_choice(&[choice("a", "allow_always"), choice("b", "allow_once")]),
            Some("b".into())
        );
        assert_eq!(allow_choice(&[choice("r", "reject_once")]), None);
    }
}
