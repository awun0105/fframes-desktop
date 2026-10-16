//! The Claude Code stream-json driver against the real, signed-in CLI.
//!
//! Ignored by default: it needs Claude Code on `PATH` (or `FFRAMES_CLAUDE`) and makes
//! model calls. Run with `cargo test -p studio-harness --test claude_real_cli -- --ignored`.

use std::{
    path::Path,
    time::{Duration, Instant},
};
use studio_bootstrap::ProcessTreeManager;
use studio_harness::{
    AdapterConfig,
    discovery::{AdapterLaunch, ExecutableSearch},
    driver::{
        AgentDriver, AgentEventKind, AgentWire, DriverConfig, DriverLimits, DriverMode,
        McpStdioSupport, OwnershipSource, PermissionReply, PromptOutcome, StopReasonKind,
        start_driver,
    },
};

const TURN: Duration = Duration::from_secs(180);
const EDIT: &str = "Use the Edit tool to replace the word hello with goodbye in hello.txt. \
                    Do not use any other tool and do not ask me anything.";

fn start(root: &Path) -> (Box<dyn AgentDriver>, ProcessTreeManager) {
    let config = AdapterConfig {
        executable: std::env::var("FFRAMES_CLAUDE").unwrap_or_else(|_| "claude".into()),
        args: vec!["--model".into(), "haiku".into()],
        auth_env_names: vec![],
    };
    let search = ExecutableSearch {
        managed_dirs: vec![],
        gui_path: std::env::var_os("PATH"),
        node: None,
    };
    let launch = AdapterLaunch::resolve_with_env(&config, &search, |_| None)
        .expect("Claude Code is on PATH or in FFRAMES_CLAUDE");
    let manager = ProcessTreeManager::new();
    let driver = start_driver(
        AgentWire::ClaudeStreamJson,
        DriverConfig {
            provider: "claude".into(),
            task: "real-cli-check".into(),
            cwd: root.to_owned(),
            launch,
            limits: DriverLimits::default(),
            resume_session: None,
            ownership: OwnershipSource::Verified,
            mode: DriverMode::Full,
            mcp_servers: vec![],
            mcp_stdio: McpStdioSupport::Baseline,
        },
        &manager,
    )
    .expect("Claude Code starts");
    driver.wait_ready(TURN).expect("the session becomes ready");
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

#[test]
#[ignore = "needs the real, signed-in Claude Code CLI and makes model calls"]
fn deny_blocks_the_edit_allow_applies_it_and_interrupt_keeps_the_process() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("hello.txt");
    std::fs::write(&file, "hello\n").unwrap();
    let (driver, manager) = start(root.path());

    driver.prompt(EDIT, Vec::new()).unwrap();
    let (outcome, asked) = run_turn(&*driver, "deny", || {});
    eprintln!("deny: {:?}, {asked} request(s)", outcome.stop_reason);
    assert!(asked > 0, "the edit was never asked for");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello\n");

    driver.prompt(EDIT, Vec::new()).unwrap();
    let (outcome, asked) = run_turn(&*driver, "allow", || {});
    eprintln!("allow: {:?}, {asked} request(s)", outcome.stop_reason);
    assert!(outcome.eligible_for_quiescence(), "{outcome:?}");
    assert!(std::fs::read_to_string(&file).unwrap().contains("goodbye"));

    let processes = manager.active_count();
    driver
        .prompt(
            "Write the numbers from 1 to 400 in words, one per line.",
            Vec::new(),
        )
        .unwrap();
    let (outcome, _) = run_turn(&*driver, "deny", || driver.cancel_turn().unwrap());
    eprintln!("interrupt: {:?}", outcome.stop_reason);
    assert_eq!(outcome.stop_reason, StopReasonKind::Cancelled);
    assert_eq!(
        manager.active_count(),
        processes,
        "the CLI keeps running after an interrupt"
    );

    driver
        .prompt("Reply with the single word ok.", Vec::new())
        .unwrap();
    let (outcome, _) = run_turn(&*driver, "deny", || {});
    eprintln!("after interrupt: {:?}", outcome.stop_reason);
    assert!(outcome.eligible_for_quiescence(), "{outcome:?}");

    let ended = driver.shutdown();
    assert!(ended.failure.is_none(), "{:?}", ended.failure);
    assert!(ended.termination.group_empty);
}
