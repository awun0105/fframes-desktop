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
        McpStdioSupport, OwnershipSource, PermissionReply, PromptOutcome, SteeringMode,
        StopReasonKind, TurnOrigin, start_driver,
    },
};

mod real_cli;
use real_cli::{WORD, run_until_idle};

const TURN: Duration = Duration::from_secs(180);
const EDIT: &str = "Use the Edit tool to replace the word hello with goodbye in hello.txt. \
                    Do not use any other tool and do not ask me anything.";

fn start(root: &Path, resume: Option<String>) -> (Box<dyn AgentDriver>, ProcessTreeManager) {
    let resumed = resume.is_some();
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
            resume_session: resume,
            ownership: OwnershipSource::Verified,
            mode: DriverMode::Full,
            mcp_servers: vec![],
            mcp_stdio: McpStdioSupport::Baseline,
        },
        &manager,
    )
    .expect("Claude Code starts");
    let info = driver.wait_ready(TURN).expect("the session becomes ready");
    assert_eq!(
        info.restored, resumed,
        "a resumed session reports itself restored"
    );
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
    let (driver, manager) = start(root.path(), None);

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

#[test]
#[ignore = "needs the real, signed-in Claude Code CLI and makes model calls"]
fn a_steer_reaches_the_running_turn_and_a_resumed_session_remembers_it() {
    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), None);
    assert_eq!(driver.capabilities().steering, SteeringMode::StepBoundary);

    driver
        .prompt(
            "Write the numbers from 1 to 300 in words, one per line. Do not use any tool.",
            Vec::new(),
        )
        .unwrap();
    let mut steered = None;
    let record = run_until_idle(&*driver, 2 * TURN, "deny", || {
        steered = Some(driver.steer(&format!(
            "Stop the list now. Reply with exactly the code word {WORD} and nothing else."
        )));
    });
    let stops: Vec<_> = record.outcomes.iter().map(|o| o.stop_reason).collect();
    eprintln!(
        "steer: {steered:?}, turns {stops:?}, {} chars",
        record.text.len()
    );
    steered
        .expect("text streamed")
        .expect("the steer is accepted");
    assert!(
        record.text.contains(WORD),
        "the steer never reached the model"
    );
    assert!(
        record
            .outcomes
            .iter()
            .all(PromptOutcome::eligible_for_quiescence)
    );

    let session = driver.wire_session_id().expect("the session has an id");
    assert!(driver.shutdown().termination.group_empty);

    let (driver, _manager) = start(root.path(), Some(session.clone()));
    assert_eq!(driver.wire_session_id(), Some(session));
    driver
        .prompt(
            "What code word did you reply with earlier in this conversation? \
             Reply with only the code word.",
            Vec::new(),
        )
        .unwrap();
    let record = run_until_idle(&*driver, TURN, "deny", || {});
    eprintln!("resumed: {:?}", record.text.trim());
    assert!(
        record.text.contains(WORD),
        "the resumed session lost the conversation"
    );
    let ended = driver.shutdown();
    assert!(ended.failure.is_none(), "{:?}", ended.failure);
    assert!(ended.termination.group_empty);
}

#[test]
#[ignore = "needs the real, signed-in Claude Code CLI and makes model calls"]
fn a_background_task_ends_in_a_turn_of_the_agents_own_before_the_agent_is_idle() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("bg.txt");
    let (driver, _manager) = start(root.path(), None);

    driver
        .prompt(
            "Use the Bash tool with run_in_background set to true to run exactly: \
             sleep 20 && echo done > bg.txt\n\
             Then reply with the single word started and end your turn without waiting. \
             When the command finishes, reply with the single word finished.",
            Vec::new(),
        )
        .unwrap();
    let begun = Instant::now();
    let record = run_until_idle(&*driver, 2 * TURN, "allow", || {});
    let turns: Vec<_> = record
        .outcomes
        .iter()
        .map(|o| (o.origin, o.stop_reason))
        .collect();
    eprintln!(
        "background: turns {turns:?}, agent turns started {:?}, tasks at the first end {}, idle after {:?}",
        record.started,
        record.tasks_at_first_end,
        begun.elapsed()
    );
    assert!(record.asked > 0, "the command was never asked for");
    assert_eq!(record.outcomes[0].origin, TurnOrigin::Prompted);
    assert!(
        record.tasks_at_first_end > 0,
        "the running task is reported before the first turn ends"
    );
    assert!(record.started.contains(&TurnOrigin::Autonomous));
    assert_eq!(
        record.outcomes.last().unwrap().origin,
        TurnOrigin::Autonomous
    );
    assert!(
        record
            .outcomes
            .iter()
            .all(PromptOutcome::eligible_for_quiescence)
    );
    // Idle is when capture may start; the background command's write is already there.
    assert!(marker.exists(), "idle before the background task finished");

    let ended = driver.shutdown();
    assert!(ended.failure.is_none(), "{:?}", ended.failure);
    assert!(ended.termination.group_empty);
}

#[test]
#[ignore = "needs the real, signed-in Claude Code CLI and makes model calls"]
fn a_question_with_two_items_gets_both_answers_back() {
    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), None);

    driver
        .prompt(
            "Use the AskUserQuestion tool once, with two questions in the same call. \
             Question one: Which color? with the options Red and Green. \
             Question two: Which fruit? with the options Apple and Mango. \
             After I answer, reply with exactly my two answers separated by a comma.",
            Vec::new(),
        )
        .unwrap();
    let record = run_until_idle(&*driver, TURN, "deny", || {});
    eprintln!(
        "question: {} prompt(s), items {:?}, chose {:?}, reply {:?}",
        record.questions.len(),
        record
            .questions
            .iter()
            .map(|q| q.questions.len())
            .collect::<Vec<_>>(),
        record.chosen,
        record.text.trim()
    );
    assert_eq!(record.questions.len(), 1, "one question request");
    assert_eq!(
        record.questions[0].questions.len(),
        2,
        "both items reach the user"
    );
    assert_eq!(record.chosen.len(), 2);
    for answer in &record.chosen {
        assert!(
            record.text.contains(answer.as_str()),
            "{answer} did not reach Claude"
        );
    }
    let ended = driver.shutdown();
    assert!(ended.failure.is_none(), "{:?}", ended.failure);
    assert!(ended.termination.group_empty);
}
