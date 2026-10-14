//! Permissions, cancel and stop, deadlines and blocked IO, bounded events.

use super::*;

// ---- permissions, cancel, stop, late events ----------------------------------------------------

#[test]
fn permission_replies_are_correlated_and_duplicates_or_late_replies_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "permission");
    ready(&driver);
    let turn = driver.prompt("edit").unwrap();
    let events = collect_until(&driver, |e| {
        matches!(e.kind, AgentEventKind::PermissionRequested(_))
    });
    let AgentEventKind::PermissionRequested(prompt) = &events.last().unwrap().kind else {
        unreachable!()
    };
    assert_eq!(prompt.turn, turn);
    assert_eq!(prompt.title, "Edit title");
    assert_eq!(prompt.options.len(), 2);
    let id = prompt.id;

    assert!(matches!(
        driver.reply_permission(id, PermissionReply::Select("nope".into())),
        Err(DriverError::InvalidOption(_))
    ));
    driver
        .reply_permission(id, PermissionReply::Select("allow".into()))
        .unwrap();
    assert!(matches!(
        driver.reply_permission(id, PermissionReply::Select("allow".into())),
        Err(DriverError::UnknownPermission(_))
    ));
    assert!(matches!(
        driver.reply_permission(
            studio_harness::driver::PermissionId(999),
            PermissionReply::Cancel
        ),
        Err(DriverError::UnknownPermission(_))
    ));
    let outcome = driver.wait_turn(turn, WAIT).unwrap();
    assert!(outcome.eligible_for_quiescence());
    assert_eq!(
        read_json(root.path(), "permission-perm-1.json")["optionId"],
        "allow"
    );
    assert!(
        std::fs::read_to_string(root.path().join("main.rs"))
            .unwrap()
            .contains("Permitted edit")
    );
    drop(driver.shutdown());
}

#[test]
fn cancelling_a_turn_closes_permissions_and_is_never_eligible() {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "permission");
    ready(&driver);
    let turn = driver.prompt("edit").unwrap();
    let events = collect_until(&driver, |e| {
        matches!(e.kind, AgentEventKind::PermissionRequested(_))
    });
    let AgentEventKind::PermissionRequested(prompt) = &events.last().unwrap().kind else {
        unreachable!()
    };
    let id = prompt.id;
    driver.cancel_prompt().unwrap();
    let outcome = driver.wait_turn(turn, WAIT).unwrap();
    assert_eq!(outcome.stop_reason, StopReasonKind::Cancelled);
    assert!(outcome.cancel_requested && !outcome.eligible_for_quiescence());
    let closed = collect_until(&driver, |e| {
        matches!(e.kind, AgentEventKind::PromptFinished(_))
    });
    assert!(closed.iter().any(|e| matches!(
        &e.kind,
        AgentEventKind::PermissionClosed { id: closed_id, resolution: PermissionResolution::Cancelled }
            if *closed_id == id
    )));
    assert!(matches!(
        driver.reply_permission(id, PermissionReply::Select("allow".into())),
        Err(DriverError::UnknownPermission(_))
    ));
    assert_eq!(
        read_json(root.path(), "permission-perm-1.json")["outcome"],
        "cancelled"
    );
    assert!(!root.path().join("main.rs").exists());
    assert!(matches!(
        driver.cancel_prompt(),
        Err(DriverError::NoActiveTurn)
    ));
    let outcome = driver.shutdown();
    assert!(outcome.termination.verified());
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn stop_closes_pending_permissions_reaps_the_tree_and_rejects_later_replies() {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "permission-two");
    ready(&driver);
    driver.prompt("edit").unwrap();
    let mut seen = 0;
    collect_until(&driver, |e| {
        if matches!(e.kind, AgentEventKind::PermissionRequested(_)) {
            seen += 1;
        }
        seen == 2
    });
    let start_stop = Instant::now();
    let outcome = driver.shutdown();
    assert!(start_stop.elapsed() < Duration::from_secs(8));
    assert!(outcome.termination.verified());
    assert_eq!(manager.active_count(), 0);
    assert!(
        outcome.last_prompt.is_none(),
        "stopped mid-turn: no completion"
    );
    assert!(!root.path().join("main.rs").exists());
}

#[test]
fn events_after_completion_are_rejected_counted_and_never_recorded() {
    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "late");
    ready(&driver);
    let turn = driver.prompt("go").unwrap();
    let outcome = driver.wait_turn(turn, WAIT).unwrap();
    assert!(outcome.eligible_for_quiescence());
    wait_for(|| (driver.late_events_rejected() >= 3).then_some(()));
    assert!(
        driver.failure().is_none(),
        "late traffic is not a protocol failure"
    );
    wait_for(|| root.path().join("late-perm.json").exists().then_some(()));
    assert_eq!(
        read_json(root.path(), "late-perm.json")["outcome"]["outcome"],
        "cancelled"
    );
    let text: String = driver
        .transcript_page(0, 100)
        .entries
        .iter()
        .map(|e| e.text.clone())
        .collect();
    assert!(text.contains("before") && !text.contains("late text"));
    let events = driver.drain_events(256);
    assert!(!events.iter().any(|e| matches!(
        &e.kind,
        AgentEventKind::ToolCall(t) if t.tool_call_id == "late"
    )));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, AgentEventKind::PermissionRequested(_)))
    );
    let outcome = driver.shutdown();
    assert!(outcome.late_events_rejected >= 3);
}

// ---- deadlines and blocked IO ---------------------------------------------------------------------

#[test]
fn deadlines_fail_visibly_and_reap_the_owned_tree() {
    let short = Duration::from_millis(400);
    type SetLimit = fn(&mut DriverLimits, Duration);
    let cases: [(&str, SetLimit, Phase); 3] = [
        ("hang-init", |l, d| l.init_timeout = d, Phase::Initialize),
        ("hang-prompt", |l, d| l.prompt_timeout = d, Phase::Prompt),
        (
            "hang-cancel",
            |l, d| l.cancel_ack_timeout = d,
            Phase::CancelAck,
        ),
    ];
    for (mode, set, phase) in cases {
        let root = tempfile::tempdir().unwrap();
        let mut cfg = config(root.path(), mode);
        set(&mut cfg.limits, short);
        let (driver, manager) = start_with(cfg);
        let began = Instant::now();
        let failure = if mode == "hang-init" {
            driver.wait_ready(WAIT).unwrap_err()
        } else {
            ready(&driver);
            let turn = driver.prompt("go").unwrap();
            if mode == "hang-cancel" {
                std::thread::sleep(Duration::from_millis(100));
                driver.cancel_prompt().unwrap();
            }
            driver.wait_turn(turn, WAIT).unwrap_err()
        };
        assert_eq!(
            failure.kind,
            FailureKind::DeadlineExceeded,
            "{mode}: {failure:?}"
        );
        assert_eq!(failure.phase, phase, "{mode}");
        assert!(began.elapsed() < Duration::from_secs(10), "{mode}");
        let outcome = driver.shutdown();
        assert!(outcome.termination.verified(), "{mode}");
        assert_eq!(manager.active_count(), 0, "{mode}");
    }
}

#[test]
fn default_limits_match_the_stage_contract() {
    let limits = DriverLimits::default();
    assert_eq!(limits.max_message_bytes, 1024 * 1024);
    assert_eq!(limits.max_queued_events, 256);
    assert_eq!(limits.max_transcript_bytes, 4 * 1024 * 1024);
    assert_eq!(limits.max_stderr_bytes, 1024 * 1024);
    assert_eq!(limits.init_timeout, Duration::from_secs(30));
    assert_eq!(limits.prompt_timeout, Duration::from_secs(15 * 60));
    assert_eq!(limits.max_turn, Duration::from_secs(2 * 60 * 60));
    assert_eq!(limits.shutdown_grace, Duration::from_secs(2));
}

/// Runs one prompt against `mode` with the given idle and turn limits.
fn timed_turn(
    mode: &str,
    idle: Duration,
    max_turn: Duration,
) -> (
    Result<studio_harness::driver::PromptOutcome, studio_harness::driver::AgentFailure>,
    Duration,
) {
    let root = tempfile::tempdir().unwrap();
    let mut cfg = config(root.path(), mode);
    cfg.limits.prompt_timeout = idle;
    cfg.limits.max_turn = max_turn;
    let (driver, manager) = start_with(cfg);
    ready(&driver);
    let began = Instant::now();
    let turn = driver.prompt("go").unwrap();
    let result = driver.wait_turn(turn, Duration::from_secs(30));
    let elapsed = began.elapsed();
    assert!(driver.shutdown().termination.verified(), "{mode}");
    assert_eq!(manager.active_count(), 0, "{mode}");
    (result, elapsed)
}

#[test]
fn command_lists_and_session_info_do_not_keep_a_turn_alive() {
    let (result, elapsed) = timed_turn(
        "inert-forever",
        Duration::from_millis(500),
        Duration::from_secs(3600),
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.kind, FailureKind::DeadlineExceeded, "{failure:?}");
    assert_eq!(failure.phase, Phase::Prompt);
    assert!(failure.message.contains("idle timeout"), "{failure:?}");
    assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
}

#[test]
fn agent_activity_keeps_a_long_turn_alive_past_the_idle_timeout() {
    // Messages, thoughts, plans, tool calls and tool updates every 100 ms for 2 s.
    let (result, elapsed) = timed_turn(
        "active-slow",
        Duration::from_secs(1),
        Duration::from_secs(3600),
    );
    let outcome = result.unwrap();
    assert!(outcome.eligible_for_quiescence(), "{outcome:?}");
    assert!(elapsed >= Duration::from_secs(2), "{elapsed:?}");
}

#[test]
fn a_turn_that_never_stops_working_hits_the_turn_cap() {
    let (result, elapsed) = timed_turn(
        "active-forever",
        Duration::from_secs(1),
        Duration::from_secs(2),
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.kind, FailureKind::DeadlineExceeded, "{failure:?}");
    assert_eq!(failure.phase, Phase::Prompt);
    assert!(failure.message.contains("turn limit"), "{failure:?}");
    assert!(elapsed >= Duration::from_secs(2), "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
}

#[test]
fn a_non_reading_adapter_cannot_block_callers_and_stop_spam_coalesces_into_a_deadline_failure() {
    let root = tempfile::tempdir().unwrap();
    let mut cfg = config(root.path(), "stall-stdin");
    cfg.limits.cancel_ack_timeout = Duration::from_millis(600);
    let (driver, manager) = start_with(cfg);
    ready(&driver);
    driver.prompt(&"x".repeat(200 * 1024)).unwrap();
    let began = Instant::now();
    // The adapter has stopped reading. Stop is latched and coalesced at the call
    // boundary, so no number of calls can block or grow a queue.
    for _ in 0..5000 {
        let _ = driver.cancel_prompt();
    }
    assert!(
        began.elapsed() < Duration::from_secs(2),
        "{:?}",
        began.elapsed()
    );
    let failure = wait_for(|| driver.failure());
    assert_eq!(failure.kind, FailureKind::DeadlineExceeded, "{failure:?}");
    assert_eq!(failure.phase, Phase::CancelAck);
    assert!(began.elapsed() < Duration::from_secs(10));
    let outcome = driver.shutdown();
    assert!(outcome.termination.verified());
    assert_eq!(manager.active_count(), 0);
}

// ---- bounded events and transcripts -----------------------------------------------------------------

#[test]
fn critical_event_overflow_fails_visibly_instead_of_dropping_events() {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "flood-tools");
    ready(&driver);
    let turn = driver.prompt("flood").unwrap();
    // Nobody drains the queue: the 256-event bound must trip.
    let failure = driver.wait_turn(turn, WAIT).unwrap_err();
    assert_eq!(failure.kind, FailureKind::EventQueueOverflow, "{failure:?}");
    assert!(driver.queued_events() <= 256 + 4);
    let events = driver.drain_events(1000);
    assert!(events.iter().any(|e| matches!(
        &e.kind,
        AgentEventKind::Failure(f) if f.kind == FailureKind::EventQueueOverflow
    )));
    let outcome = driver.shutdown();
    assert!(outcome.termination.verified());
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn message_deltas_batch_and_the_transcript_stays_bounded() {
    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "flood-deltas");
    ready(&driver);
    let turn = driver.prompt("stream").unwrap();
    let outcome = driver.wait_turn(turn, Duration::from_secs(60)).unwrap();
    assert!(outcome.eligible_for_quiescence());
    assert!(
        driver.queued_events() <= 256,
        "deltas coalesce into few events"
    );
    assert!(driver.failure().is_none());
    let mut text = String::new();
    let mut cursor = Some(0);
    while let Some(from) = cursor {
        let page = driver.transcript_page(from, 64);
        text.extend(page.entries.iter().map(|e| e.text.as_str()));
        cursor = page.next;
    }
    assert!(text.starts_with("delta-0 delta-1 "));
    assert!(text.trim_end().ends_with("delta-19999"));
    assert!(driver.transcript_resident_bytes() <= 4 * 1024 * 1024 + 16 * 1024);
    drop(driver.shutdown());
}
