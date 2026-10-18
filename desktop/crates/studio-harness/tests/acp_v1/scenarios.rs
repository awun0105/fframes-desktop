//! Scenario coverage, the wire budget and server-side tools.

use super::*;

// ---- scenario coverage: content, updates, errors and framing -------------------------------

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-pixels";

struct Run {
    driver: AcpDriver,
    manager: ProcessTreeManager,
    outcome: studio_harness::driver::PromptOutcome,
    root: tempfile::TempDir,
}

impl Run {
    /// Shuts the adapter down and proves nothing it owned is left running.
    fn finish(self) {
        assert_contained(&self.driver.shutdown());
        assert_eq!(self.manager.active_count(), 0);
    }
}

/// A clean run ends verified and contained, keeping the containment verified at launch.
fn assert_contained(outcome: &studio_harness::driver::DriverOutcome) {
    assert!(outcome.termination.verified(), "{:?}", outcome.termination);
    assert_eq!(
        outcome.containment_unverified, None,
        "containment could not be verified"
    );
    assert!(
        outcome.escaped_pids.is_empty(),
        "{:?}",
        outcome.escaped_pids
    );
    assert!(outcome.ownership.is_qualified(), "{:?}", outcome.ownership);
}

fn run_turn(mode: &str, prompt: &str) -> Run {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), mode);
    ready(&driver);
    let turn = driver.prompt(prompt).unwrap();
    let outcome = driver.wait_turn(turn, WAIT).unwrap();
    Run {
        driver,
        manager,
        outcome,
        root,
    }
}

fn deltas(events: &[AgentEvent]) -> Vec<(MessageRole, String)> {
    events
        .iter()
        .filter_map(|e| match &e.kind {
            AgentEventKind::MessageDelta { role, text } => Some((*role, text.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn image_prompts_are_validated_before_anything_reaches_the_adapter() {
    let invalid = |driver: &AcpDriver, images: Vec<PromptImage>| match driver
        .prompt_with_images("look", images)
    {
        Err(DriverError::InvalidPrompt(reason)) => reason,
        other => panic!("expected InvalidPrompt, got {other:?}"),
    };

    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "ok");
    ready(&driver);
    assert!(invalid(&driver, vec![PromptImage::png(PNG.to_vec())]).contains("did not negotiate"));
    assert!(matches!(
        driver.prompt("  "),
        Err(DriverError::InvalidPrompt(_))
    ));
    drop(driver.shutdown());

    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "image-ok");
    ready(&driver);
    let png = || PromptImage::png(PNG.to_vec());
    assert!(invalid(&driver, vec![png(), png(), png(), png()]).contains("too many"));
    assert!(
        invalid(
            &driver,
            vec![PromptImage::png(b"GIF89a-not-a-png".to_vec())]
        )
        .contains("not a PNG")
    );
    assert!(invalid(&driver, vec![PromptImage::png(Vec::new())]).contains("empty"));
    let mut huge = PNG.to_vec();
    huge.resize(4 * 1024 * 1024 + 1, 0);
    assert!(invalid(&driver, vec![PromptImage::png(huge)]).contains("size limit"));
    assert!(matches!(
        driver.prompt(&"x".repeat(300 * 1024)),
        Err(DriverError::InvalidPrompt(_))
    ));
    // Within the text limit, but JSON escapes each control character to 6 bytes.
    match driver.prompt(&"\u{1}".repeat(250 * 1024)) {
        Err(DriverError::InvalidPrompt(reason)) => {
            assert!(reason.contains("message size limit"), "{reason}");
        }
        other => panic!("expected InvalidPrompt, got {other:?}"),
    }
    assert_eq!(
        driver.status(),
        DriverStatus::Ready,
        "rejections never start a turn"
    );
    assert!(!root.path().join("prompt.json").exists());

    let turn = driver.prompt_with_images("look", vec![png()]).unwrap();
    driver.wait_turn(turn, WAIT).unwrap();
    assert!(root.path().join("prompt.json").exists());
    drop(driver.shutdown());
}

#[test]
fn steering_needs_the_negotiated_extension_and_a_running_turn() {
    use studio_harness::driver::{AgentDriver, SteeringMode};

    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "steer");
    ready(&driver);
    assert_eq!(
        AgentDriver::capabilities(&driver).steering,
        SteeringMode::StepBoundary
    );
    assert!(matches!(
        driver.steer_turn("now"),
        Err(DriverError::NoActiveTurn)
    ));
    let turn = driver.prompt("start").unwrap();
    wait_for(|| root.path().join("turn-open.json").exists().then_some(()));
    driver.steer_turn("turn left").unwrap();
    let outcome = driver.wait_turn(turn, WAIT).unwrap();
    assert!(outcome.eligible_for_quiescence());
    assert_eq!(read_json(root.path(), "steering.json")["text"], "turn left");
    let events = driver.drain_events(256);
    assert!(
        events.iter().any(|e| matches!(
            &e.kind,
            AgentEventKind::SteerResult { text, accepted: true, .. } if text == "turn left"
        )),
        "{events:?}"
    );
    let text: String = driver
        .transcript_page(0, 16)
        .entries
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert_eq!(text, "working steered");
    drop(driver.shutdown());

    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "ok");
    ready(&driver);
    assert_eq!(
        AgentDriver::capabilities(&driver).steering,
        SteeringMode::None
    );
    assert!(matches!(
        driver.steer_turn("now"),
        Err(DriverError::Unsupported(_))
    ));
    drop(driver.shutdown());
}

#[test]
fn a_refused_steer_is_reported_without_ending_the_turn() {
    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "steer-reject");
    ready(&driver);
    let turn = driver.prompt("start").unwrap();
    wait_for(|| root.path().join("turn-open.json").exists().then_some(()));
    driver.steer_turn("turn left").unwrap();
    let events = collect_until(&driver, |e| {
        matches!(&e.kind, AgentEventKind::SteerResult { .. })
    });
    match &events.last().unwrap().kind {
        AgentEventKind::SteerResult {
            text,
            accepted,
            message,
        } => {
            assert_eq!(text, "turn left");
            assert!(!accepted);
            assert!(message.contains("cannot steer now"), "{message}");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(driver.status(), DriverStatus::Prompting);
    driver.cancel_prompt().unwrap();
    driver.wait_turn(turn, WAIT).unwrap();
    drop(driver.shutdown());
}

#[test]
fn files_travel_as_resource_links_with_their_name_and_uri() {
    use studio_harness::driver::PromptFile;

    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "files");
    ready(&driver);
    let notes = root.path().join("my notes.txt");
    std::fs::write(&notes, "x").unwrap();
    assert!(matches!(
        driver.prompt_with_files(
            "read",
            vec![PromptFile::new("relative", PathBuf::from("a.txt"))],
            Vec::new()
        ),
        Err(DriverError::InvalidPrompt(_))
    ));
    let turn = driver
        .prompt_with_files(
            "read",
            vec![PromptFile::new("my notes.txt", notes)],
            Vec::new(),
        )
        .unwrap();
    driver.wait_turn(turn, WAIT).unwrap();
    let blocks = read_json(root.path(), "prompt-blocks.json");
    let link = blocks
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["type"] == "resource_link")
        .expect("a resource link block");
    assert_eq!(link["name"], "my notes.txt");
    let uri = link["uri"].as_str().unwrap();
    assert!(uri.starts_with("file:///"), "{uri}");
    assert!(uri.ends_with("my%20notes.txt"), "{uri}");
    drop(driver.shutdown());
}

#[test]
fn embedded_files_travel_as_resources_only_when_the_adapter_negotiated_them() {
    use studio_harness::driver::PromptFile;

    let plain = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(plain.path(), "files");
    ready(&driver);
    assert!(matches!(
        driver.prompt_with_files(
            "read",
            vec![PromptFile::embedded("notes.txt", b"hi".to_vec())],
            Vec::new()
        ),
        Err(DriverError::InvalidPrompt(_))
    ));
    drop(driver.shutdown());

    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "embedded");
    ready(&driver);
    let turn = driver
        .prompt_with_files(
            "read",
            vec![
                PromptFile::embedded("my notes.txt", b"hello".to_vec()),
                PromptFile::embedded("bin.dat", vec![0xff, 0xfe, 0x00]),
            ],
            Vec::new(),
        )
        .unwrap();
    driver.wait_turn(turn, WAIT).unwrap();
    let blocks = read_json(root.path(), "prompt-blocks.json");
    let resources: Vec<_> = blocks
        .as_array()
        .unwrap()
        .iter()
        .filter(|block| block["type"] == "resource")
        .collect();
    assert_eq!(resources.len(), 2);
    assert_eq!(resources[0]["resource"]["text"], "hello");
    assert_eq!(
        resources[0]["resource"]["uri"],
        "attachment:///my%20notes.txt"
    );
    assert_eq!(resources[1]["resource"]["blob"], "//4A");
    drop(driver.shutdown());
}

#[test]
fn thought_and_message_text_keep_their_roles_and_chronological_order() {
    let run = run_turn("thoughts", "think");
    let driver = &run.driver;
    let outcome = &run.outcome;
    assert!(outcome.eligible_for_quiescence());
    assert_eq!(
        deltas(&driver.drain_events(256)),
        [
            (MessageRole::Thought, "plan A".to_owned()),
            (MessageRole::Agent, "Answer ".to_owned()),
            (MessageRole::Thought, "more thinking".to_owned()),
            (
                MessageRole::Agent,
                "done.[non-text content omitted]".to_owned()
            ),
        ]
    );
    let page = driver.transcript_page(0, 16);
    assert_eq!(page.entries[0].role, MessageRole::Thought);
    assert_eq!(page.entries[1].text, "Answer ");
    run.finish();
}

#[test]
fn plans_and_command_lists_are_displayed_and_session_info_is_inert() {
    let run = run_turn("inert-updates", "go");
    let driver = &run.driver;
    let outcome = &run.outcome;
    assert!(outcome.eligible_for_quiescence());
    let events = driver.drain_events(256);
    assert_eq!(
        deltas(&events),
        [(MessageRole::Agent, "still fine".to_owned())]
    );
    assert!(
        events.iter().all(|e| matches!(
            e.kind,
            AgentEventKind::Initialized(_)
                | AgentEventKind::SessionReady { .. }
                | AgentEventKind::Options(_)
                | AgentEventKind::MessageDelta { .. }
                | AgentEventKind::Plan(_)
                | AgentEventKind::AvailableCommands(_)
                | AgentEventKind::PromptFinished(_)
        )),
        "usage and session info stay inert: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(&e.kind, AgentEventKind::Plan(entries) if entries[0].text == "step"))
    );
    assert!(events.iter().any(|e| matches!(
        &e.kind,
        AgentEventKind::AvailableCommands(commands) if commands[0].name == "compact"
    )));
    assert_eq!(driver.late_events_rejected(), 0);
    assert!(driver.failure().is_none());
    run.finish();
}

#[test]
fn plan_entries_and_commands_map_with_their_status_and_hint() {
    let run = run_turn("plan", "go");
    let events = run.driver.drain_events(256);
    let plan = events
        .iter()
        .find_map(|e| match &e.kind {
            AgentEventKind::Plan(entries) => Some(entries.clone()),
            _ => None,
        })
        .expect("plan event");
    let shown: Vec<_> = plan.iter().map(|e| (e.text.as_str(), e.status)).collect();
    assert_eq!(
        shown,
        [
            (
                "read the code",
                studio_harness::driver::PlanStatus::Completed
            ),
            ("edit it", studio_harness::driver::PlanStatus::InProgress),
            ("test it", studio_harness::driver::PlanStatus::Pending),
        ]
    );
    let commands = events
        .iter()
        .find_map(|e| match &e.kind {
            AgentEventKind::AvailableCommands(commands) => Some(commands.clone()),
            _ => None,
        })
        .expect("commands event");
    assert_eq!(commands[0].name, "compact");
    assert_eq!(commands[1].name, "review");
    assert_eq!(commands[1].input_hint.as_deref(), Some("what to review"));
    assert!(run.outcome.eligible_for_quiescence());
    run.finish();
}

#[test]
fn adapter_pushed_mode_and_config_changes_update_the_exported_options() {
    let run = run_turn("push-options", "go");
    let driver = &run.driver;
    let events = driver.drain_events(256);
    let snapshots: Vec<_> = events
        .iter()
        .filter_map(|e| match &e.kind {
            AgentEventKind::Options(options) => Some(options.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        snapshots.len(),
        3,
        "initial + mode push + config push: {events:?}"
    );
    let mode_name = |options: &studio_harness::driver::AgentOptions| {
        options
            .modes
            .iter()
            .find(|m| Some(&m.id) == options.current_mode.as_ref())
            .map(|m| m.name.clone())
    };
    let model_value = |options: &studio_harness::driver::AgentOptions| {
        options.config.iter().find_map(|c| match &c.kind {
            studio_harness::driver::ConfigKind::Select { current, values } if c.name == "Model" => {
                values
                    .iter()
                    .find(|v| &v.value == current)
                    .map(|v| v.name.clone())
            }
            _ => None,
        })
    };
    assert_eq!(mode_name(&snapshots[0]).as_deref(), Some("Code"));
    assert_eq!(model_value(&snapshots[0]).as_deref(), Some("Fast"));
    assert_eq!(mode_name(&snapshots[1]).as_deref(), Some("Plan"));
    assert_eq!(model_value(&snapshots[1]).as_deref(), Some("Fast"));
    assert_eq!(mode_name(&snapshots[2]).as_deref(), Some("Plan"));
    assert_eq!(model_value(&snapshots[2]).as_deref(), Some("Deep"));
    assert_eq!(driver.options(), *snapshots.last().unwrap());
    assert!(driver.failure().is_none());
    run.finish();
}

#[test]
fn a_turn_without_any_output_still_completes_exactly_once() {
    let run = run_turn("empty-turn", "quiet");
    let driver = &run.driver;
    let outcome = &run.outcome;
    assert!(outcome.authoritative && outcome.eligible_for_quiescence());
    let events = driver.drain_events(256);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.kind, AgentEventKind::PromptFinished(_)))
            .count(),
        1
    );
    assert!(deltas(&events).is_empty());
    assert!(driver.transcript_page(0, 8).entries.is_empty());
    run.finish();
}

#[test]
fn an_error_response_to_the_prompt_is_a_visible_prompt_failure() {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "prompt-error");
    ready(&driver);
    let turn = driver.prompt("go").unwrap();
    let failure = driver.wait_turn(turn, WAIT).unwrap_err();
    assert_eq!(failure.kind, FailureKind::PromptRejected, "{failure:?}");
    drop(driver.shutdown());
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn a_duplicate_prompt_response_never_settles_a_second_turn() {
    let run = run_turn("dup-finish", "go");
    let driver = &run.driver;
    let outcome = &run.outcome;
    assert!(outcome.eligible_for_quiescence());
    // The peer sends a late tool call after the duplicate response; frames are handled
    // in order, so once it is counted the duplicate has been handled too.
    wait_for(|| (driver.late_events_rejected() >= 1).then_some(()));
    let events = driver.drain_events(256);
    let finished: Vec<u64> = events
        .iter()
        .filter_map(|e| match &e.kind {
            AgentEventKind::PromptFinished(o) => Some(o.turn),
            _ => None,
        })
        .collect();
    assert_eq!(finished, [outcome.turn], "{events:?}");
    run.finish();
}

#[test]
fn an_adapter_that_dies_while_idle_surfaces_one_failure_and_refuses_new_prompts() {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "idle-crash");
    // The session may become ready before the exit is observed.
    let _ = driver.wait_ready(WAIT);
    let failure = wait_for(|| driver.failure());
    assert_eq!(failure.kind, FailureKind::ProcessExited, "{failure:?}");
    assert_eq!(failure.exit_code, Some(5));
    assert!(matches!(
        driver.prompt("anyone there"),
        Err(DriverError::Failed(_)) | Err(DriverError::Closed)
    ));
    let events = driver.drain_events(256);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.kind, AgentEventKind::Failure(_)))
            .count(),
        1,
        "{events:?}"
    );
    assert!(driver.shutdown().termination.verified());
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn a_permission_request_for_a_foreign_session_is_a_protocol_violation() {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "foreign-perm");
    ready(&driver);
    let turn = driver.prompt("go").unwrap();
    let failure = driver.wait_turn(turn, WAIT).unwrap_err();
    assert_eq!(failure.kind, FailureKind::ProtocolViolation, "{failure:?}");
    assert!(
        !driver
            .drain_events(256)
            .iter()
            .any(|e| matches!(e.kind, AgentEventKind::PermissionRequested(_)))
    );
    drop(driver.shutdown());
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn an_unknown_adapter_request_is_answered_with_an_error_and_the_turn_continues() {
    let run = run_turn("unknown-method", "go");
    let driver = &run.driver;
    let outcome = &run.outcome;
    let root = run.root.path();
    assert!(outcome.eligible_for_quiescence());
    // The peer writes the file after the turn ended; ignore a half-written one.
    let reply: serde_json::Value = wait_for(|| {
        std::fs::read(root.join("custom-ping.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    });
    assert!(reply.get("error").is_some(), "{reply}");
    assert!(driver.failure().is_none());
    run.finish();
}

#[test]
fn blank_lines_and_crlf_framing_do_not_corrupt_the_turn() {
    let run = run_turn("crlf", "go");
    let driver = &run.driver;
    let outcome = &run.outcome;
    assert!(outcome.eligible_for_quiescence());
    assert_eq!(
        deltas(&driver.drain_events(256)),
        [(MessageRole::Agent, "crlf framed".to_owned())]
    );
    run.finish();
}

#[test]
fn a_reply_ending_in_a_secret_prefix_is_released_with_its_own_turn() {
    // "s" starts the configured secret; it must neither vanish from turn 1 nor reappear
    // in front of turn 2.
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "secret-tail");
    ready(&driver);
    let first = driver.prompt("one").unwrap();
    driver.wait_turn(first, WAIT).unwrap();
    assert_eq!(
        deltas(&driver.drain_events(256)),
        [(MessageRole::Agent, "tails".to_owned())]
    );
    let second = driver.prompt("two").unwrap();
    driver.wait_turn(second, WAIT).unwrap();
    assert_eq!(
        deltas(&driver.drain_events(256)),
        [(MessageRole::Agent, "Hello".to_owned())]
    );
    assert!(driver.shutdown().termination.verified());
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn a_stale_prompt_response_never_settles_a_newer_turn() {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "stale-finish");
    ready(&driver);
    let first = driver.prompt("one").unwrap();
    driver.wait_turn(first, WAIT).unwrap();
    let second = driver.prompt("two").unwrap();
    // The peer answers turn 1 a second time, then sends a tool call of turn 2; frames are
    // handled in order, so once that event is seen the stale response has been handled.
    collect_until(
        &driver,
        |event| matches!(&event.kind, AgentEventKind::ToolCall(tool) if tool.tool_call_id == "after-stale"),
    );
    assert_eq!(
        driver.status(),
        DriverStatus::Prompting,
        "{:?}",
        driver.failure()
    );
    assert!(driver.last_prompt().is_some_and(|p| p.turn == first));
    std::fs::write(root.path().join("go"), "").unwrap();
    let outcome = driver.wait_turn(second, WAIT).unwrap();
    assert!(outcome.authoritative && outcome.eligible_for_quiescence());
    assert!(driver.failure().is_none());
    assert!(driver.shutdown().termination.verified());
    assert_eq!(manager.active_count(), 0);
}

// ---- wire budget -----------------------------------------------------------------------------

fn png_of(len: usize) -> PromptImage {
    let mut data = PNG.to_vec();
    data.resize(len, b'p');
    PromptImage::png(data)
}

fn prompt_record(root: &Path) -> (u64, u64) {
    let prompt = read_json(root, "prompt.json");
    (
        prompt["lineBytes"].as_u64().unwrap(),
        prompt["images"].as_u64().unwrap(),
    )
}

#[test]
fn a_prompt_fills_the_wire_message_exactly_and_one_more_byte_is_refused() {
    const MAX: usize = 64 * 1024;
    let root = tempfile::tempdir().unwrap();
    let mut cfg = config(root.path(), "image-ok");
    cfg.limits.max_message_bytes = MAX;
    let (driver, manager) = start_with(cfg);
    ready(&driver);
    let images = || vec![png_of(3000), png_of(3001), png_of(3002)];

    // Exactly three images are accepted; the peer reports the line it received.
    let turn = driver
        .prompt_with_images(&"a".repeat(100), images())
        .unwrap();
    driver.wait_turn(turn, WAIT).unwrap();
    let (base, count) = prompt_record(root.path());
    assert_eq!(count, 3);

    // Text moves the line one byte at a time: fill it to the limit (newline included).
    let fill = 100 + (MAX as u64 - 1 - base) as usize;
    let turn = driver
        .prompt_with_images(&"a".repeat(fill), images())
        .unwrap();
    driver.wait_turn(turn, WAIT).unwrap();
    assert_eq!(prompt_record(root.path()), (MAX as u64 - 1, 3));

    match driver.prompt_with_images(&"a".repeat(fill + 1), images()) {
        Err(DriverError::InvalidPrompt(reason)) => {
            assert!(reason.contains("message size limit"), "{reason}");
        }
        other => panic!("expected InvalidPrompt, got {other:?}"),
    }
    assert_eq!(driver.status(), DriverStatus::Ready, "no turn was started");
    assert!(driver.failure().is_none());

    // The advertised budget is always sendable, however the bytes are split.
    let budget = driver.prompt_image_budget("look", 3);
    assert!(budget > 9000 && budget < MAX, "{budget}");
    let third = budget / 3;
    let split = vec![
        png_of(third),
        png_of(third + 1),
        png_of(budget - 2 * third - 1),
    ];
    let turn = driver.prompt_with_images("look", split).unwrap();
    driver.wait_turn(turn, WAIT).unwrap();
    assert!(prompt_record(root.path()).0 < MAX as u64);
    assert_eq!(driver.prompt_image_budget("look", 4), 0);
    assert_eq!(driver.prompt_image_budget("look", 0), 0);

    // Images alone that cannot fit are refused before they are encoded.
    match driver.prompt_with_images("look", vec![png_of(MAX)]) {
        Err(DriverError::InvalidPrompt(reason)) => {
            assert!(reason.contains("message size limit"), "{reason}");
        }
        other => panic!("expected InvalidPrompt, got {other:?}"),
    }
    assert!(driver.failure().is_none());
    assert!(driver.shutdown().termination.verified());
    assert_eq!(manager.active_count(), 0);
}

// ---- server-side tools ------------------------------------------------------------------------

#[test]
fn a_turn_ended_with_only_server_tools_unfinished_is_over_but_never_quiesces() {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "srvtool-ok");
    ready(&driver);
    let turn = driver.prompt("advise").unwrap();
    let outcome = driver.wait_turn(turn, WAIT).unwrap();
    assert!(outcome.reported_incomplete, "{outcome:?}");
    assert!(
        !outcome.authoritative,
        "an error response carries no stop reason"
    );
    assert_eq!(outcome.stop_reason, StopReasonKind::EndTurn);
    assert!(!outcome.eligible_for_quiescence());
    assert!(driver.failure().is_none());
    assert_eq!(driver.status(), DriverStatus::Ready);

    // The session stays usable and the next turn is a clean one.
    let next = driver.prompt("again").unwrap();
    let outcome = driver.wait_turn(next, WAIT).unwrap();
    assert!(!outcome.reported_incomplete && outcome.eligible_for_quiescence());
    let last = driver.shutdown();
    assert!(last.failure.is_none());
    assert_contained(&last);
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn unfinished_tool_reports_that_are_not_server_tools_only_fail_the_turn() {
    // No structured tag, a client tool among the ids, or another adapter.
    for mode in ["srvtool-nodata", "srvtool-client", "srvtool-other-agent"] {
        let root = tempfile::tempdir().unwrap();
        let (driver, manager) = start(root.path(), mode);
        ready(&driver);
        let turn = driver.prompt("advise").unwrap();
        let failure = driver.wait_turn(turn, WAIT).unwrap_err();
        assert_eq!(
            failure.kind,
            FailureKind::PromptRejected,
            "{mode}: {failure:?}"
        );
        assert!(driver.last_prompt().is_none(), "{mode}");
        drop(driver.shutdown());
        assert_eq!(manager.active_count(), 0, "{mode}");
    }
}
