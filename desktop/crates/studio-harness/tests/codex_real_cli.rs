//! The Codex app-server driver against the real, signed-in CLI.
//!
//! Ignored by default: it needs Codex on `PATH` (or `FFRAMES_CODEX`) and makes model
//! calls. Run with `cargo test -p studio-harness --test codex_real_cli -- --ignored`.
//! It lists processes read-only to see which servers Codex started; it prints executable
//! names, never command lines.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::{Duration, Instant},
};
use studio_bootstrap::ProcessTreeManager;
use studio_harness::{
    AdapterConfig,
    discovery::{AdapterLaunch, ExecutableSearch},
    driver::{
        AgentDriver, AgentEventKind, AgentWire, DriverConfig, DriverLimits, DriverMode,
        McpStdioServer, McpStdioSupport, OwnershipSource, PermissionReply, PromptOutcome,
        StopReasonKind, start_driver,
    },
};

const TURN: Duration = Duration::from_secs(240);
const WRITE: &str = "Create a file named ran.txt in the current directory containing the \
                     single word ran. Do nothing else and do not ask me anything.";
const LONG: &str = "Run exactly this shell command and wait for it to finish: \
                    ping -n 120 127.0.0.1";
const PING: &str = "PING.EXE";
/// In the studio stub's command line, so the listing can tell it from other servers.
const STUB_MARKER: &str = "studio-mcp-stub-3c9a";

fn start(root: &Path) -> (Box<dyn AgentDriver>, ProcessTreeManager) {
    let config = AdapterConfig {
        executable: std::env::var("FFRAMES_CODEX").unwrap_or_else(|_| "codex".into()),
        args: vec!["--effort".into(), "low".into()],
        auth_env_names: vec![],
    };
    let search = ExecutableSearch {
        managed_dirs: vec![],
        gui_path: std::env::var_os("PATH"),
        node: None,
    };
    let launch = AdapterLaunch::resolve_with_env(&config, &search, |_| None)
        .expect("Codex is on PATH or in FFRAMES_CODEX");
    // A stand-in for the studio server: it records that it ran and idles.
    let stub = format!(
        "import pathlib, sys, time; pathlib.Path(sys.argv[1], 'studio-started').write_text('{STUB_MARKER}'); time.sleep(600)"
    );
    let studio = McpStdioServer::new(
        "studio",
        studio_test_support::python(),
        vec!["-c".into(), stub, root.to_string_lossy().into()],
        vec![],
    )
    .unwrap();
    let manager = ProcessTreeManager::new();
    let driver = start_driver(
        AgentWire::CodexAppServer,
        DriverConfig {
            provider: "codex".into(),
            task: "real-cli-check".into(),
            cwd: root.to_owned(),
            launch,
            limits: DriverLimits::default(),
            resume_session: None,
            ownership: OwnershipSource::Verified,
            mode: DriverMode::Full,
            mcp_servers: vec![studio],
            mcp_stdio: McpStdioSupport::Baseline,
        },
        &manager,
    )
    .expect("Codex starts");
    driver.wait_ready(TURN).expect("the thread becomes ready");
    (driver, manager)
}

/// Pumps one turn: every permission request gets `choice`, `on_text` sees the first text.
fn run_turn(
    driver: &dyn AgentDriver,
    choice: &str,
    mut on_text: impl FnMut(),
) -> (PromptOutcome, u32) {
    let deadline = Instant::now() + TURN;
    let mut asked = 0;
    let mut texted = false;
    while Instant::now() < deadline {
        for event in driver.drain_events(64) {
            match event.kind {
                AgentEventKind::PermissionRequested(prompt) => {
                    asked += 1;
                    eprintln!("asked: {}", prompt.title);
                    driver
                        .reply_permission(prompt.id, PermissionReply::Select(choice.into()))
                        .unwrap();
                }
                AgentEventKind::MessageDelta { .. } if !texted => {
                    texted = true;
                    on_text();
                }
                AgentEventKind::PromptFinished(outcome) => return (outcome, asked),
                AgentEventKind::Failure(failure) => panic!("the session failed: {failure:?}"),
                AgentEventKind::Closed(closed) => panic!("the session closed: {closed:?}"),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the turn never ended");
}

struct Process {
    parent: u32,
    name: String,
    stub: bool,
}

fn processes() -> HashMap<u32, Process> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,CommandLine | ConvertTo-Json -Compress",
        ])
        .output()
        .expect("the process listing runs");
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    rows.into_iter()
        .map(|row| {
            let id = row["ProcessId"].as_u64().unwrap() as u32;
            let process = Process {
                parent: row["ParentProcessId"].as_u64().unwrap_or(0) as u32,
                name: row["Name"].as_str().unwrap_or("").to_owned(),
                stub: row["CommandLine"]
                    .as_str()
                    .is_some_and(|line| line.contains(STUB_MARKER)),
            };
            (id, process)
        })
        .collect()
}

/// Every process below this test process.
fn descendants() -> HashMap<u32, Process> {
    let mut all = processes();
    let mut below = HashSet::from([std::process::id()]);
    loop {
        let more: Vec<u32> = all
            .iter()
            .filter(|(id, p)| below.contains(&p.parent) && !below.contains(id))
            .map(|(id, _)| *id)
            .collect();
        if more.is_empty() {
            break;
        }
        below.extend(more);
    }
    below.remove(&std::process::id());
    all.retain(|id, p| below.contains(id) && p.name != "powershell.exe");
    all
}

#[test]
#[ignore = "needs the real, signed-in Codex CLI and makes model calls"]
fn decline_blocks_accept_writes_in_the_draft_interrupt_keeps_the_server_and_stop_ends_commands() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("ran.txt");
    let (driver, manager) = start(root.path());

    driver.prompt(WRITE, Vec::new()).unwrap();
    let (outcome, asked) = run_turn(&*driver, "decline", || {});
    eprintln!("decline: {:?}, {asked} request(s)", outcome.stop_reason);
    assert!(asked > 0, "the write was never asked for");
    assert!(!file.exists(), "a declined write ran");

    driver.prompt(WRITE, Vec::new()).unwrap();
    let (outcome, asked) = run_turn(&*driver, "accept", || {});
    eprintln!("accept: {:?}, {asked} request(s)", outcome.stop_reason);
    assert!(outcome.eligible_for_quiescence(), "{outcome:?}");
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .to_lowercase()
            .contains("ran")
    );

    assert!(
        root.path().join("studio-started").exists(),
        "the studio server ran"
    );
    let running = descendants();
    let mut names: Vec<&str> = running.values().map(|p| p.name.as_str()).collect();
    names.sort_unstable();
    eprintln!("processes below the test: {names:?}");
    let others: Vec<&str> = running
        .values()
        .filter(|p| !p.stub && !p.name.starts_with("codex") && p.name != "conhost.exe")
        .map(|p| p.name.as_str())
        .collect();
    assert!(
        others.is_empty(),
        "servers other than studio ran: {others:?}"
    );

    let processes = manager.active_count();
    driver
        .prompt(
            "Write the numbers from 1 to 400 in words, one per line.",
            Vec::new(),
        )
        .unwrap();
    let (outcome, _) = run_turn(&*driver, "decline", || driver.cancel_turn().unwrap());
    eprintln!("interrupt: {:?}", outcome.stop_reason);
    assert_eq!(outcome.stop_reason, StopReasonKind::Cancelled);
    assert_eq!(
        manager.active_count(),
        processes,
        "the app server keeps running after an interrupt"
    );

    driver
        .prompt("Reply with the single word ok.", Vec::new())
        .unwrap();
    let (outcome, _) = run_turn(&*driver, "decline", || {});
    eprintln!("after interrupt: {:?}", outcome.stop_reason);
    assert!(outcome.eligible_for_quiescence(), "{outcome:?}");

    // Codex's interrupt leaves a running command running: it must be reported as
    // background work, and session stop must still end it.
    let pings_before = count_named(PING);
    driver.prompt(LONG, Vec::new()).unwrap();
    let deadline = Instant::now() + TURN;
    let mut cancelled = false;
    let mut background = Vec::new();
    let mut seen = HashSet::new();
    let outcome = 'turn: loop {
        assert!(
            Instant::now() < deadline,
            "the long command turn never ended"
        );
        for event in driver.drain_events(64) {
            match event.kind {
                AgentEventKind::PermissionRequested(prompt) => {
                    eprintln!("asked: {}", prompt.title);
                    driver
                        .reply_permission(prompt.id, PermissionReply::Select("accept".into()))
                        .unwrap();
                }
                AgentEventKind::Background { tasks, .. } => background.push(tasks),
                AgentEventKind::PromptFinished(outcome) => break 'turn outcome,
                AgentEventKind::Failure(failure) => panic!("the session failed: {failure:?}"),
                _ => {}
            }
        }
        if !cancelled && count_named(PING) > pings_before {
            seen.extend(descendants().into_keys());
            driver.cancel_turn().unwrap();
            cancelled = true;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    eprintln!(
        "long command: {:?}, cancelled while running: {cancelled}, background {background:?}",
        outcome.stop_reason
    );
    assert!(cancelled, "the command never ran");
    assert_eq!(outcome.stop_reason, StopReasonKind::Cancelled);
    std::thread::sleep(Duration::from_secs(2));
    for event in driver.drain_events(64) {
        if let AgentEventKind::Background { tasks, .. } = event.kind {
            background.push(tasks);
        }
    }
    let pings_now = count_named(PING);
    eprintln!(
        "after the interrupt: background {background:?}, ping processes {pings_now} (before {pings_before})"
    );

    seen.extend(descendants().into_keys());
    let ended = driver.shutdown();
    assert!(ended.failure.is_none(), "{:?}", ended.failure);
    assert!(ended.termination.group_empty);
    let left: Vec<String> = processes_left(&seen);
    assert!(left.is_empty(), "left running after stop: {left:?}");
    assert_eq!(
        count_named(PING),
        pings_before,
        "the command outlived the session"
    );
}

fn count_named(name: &str) -> usize {
    processes()
        .values()
        .filter(|p| p.name.eq_ignore_ascii_case(name))
        .count()
}

fn processes_left(seen: &HashSet<u32>) -> Vec<String> {
    processes()
        .into_iter()
        .filter(|(id, _)| seen.contains(id))
        .map(|(_, p)| p.name)
        .collect()
}
