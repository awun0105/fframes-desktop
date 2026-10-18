//! Negotiation, prompts and completion, framing failures, discovery and auth status.

use super::*;

// ---- negotiation, prompt and authoritative completion -------------------------------

#[test]
fn v1_is_negotiated_explicitly_and_a_turn_completes_authoritatively() {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "write");
    let info = driver.wait_ready(WAIT).unwrap();
    assert_eq!(info.initialized.protocol_version, 1);
    assert!(
        !info.initialized.authenticated,
        "initialize never proves auth"
    );
    assert_eq!(info.session_id.as_deref(), Some("test-session"));
    assert!(!info.restored);
    assert_eq!(info.initialized.agent_name, "protocol-peer");

    let init = read_json(root.path(), "initialize.json");
    assert_eq!(init["protocolVersion"], 1);
    let capabilities = &init["clientCapabilities"];
    assert_ne!(capabilities["fs"]["readTextFile"], true);
    assert_ne!(capabilities["fs"]["writeTextFile"], true);
    assert_ne!(capabilities["terminal"], true);
    assert_eq!(init["clientInfo"]["name"], "fframes-studio");

    let turn = driver.prompt("Edit the title").unwrap();
    let outcome = driver.wait_turn(turn, WAIT).unwrap();
    assert_eq!(outcome.stop_reason, StopReasonKind::EndTurn);
    assert!(outcome.authoritative && outcome.eligible_for_quiescence());
    assert!(
        std::fs::read_to_string(root.path().join("main.rs"))
            .unwrap()
            .contains("agent edit")
    );
    assert_eq!(
        read_json(root.path(), "prompt.json")["sessionId"],
        "test-session"
    );

    let events = driver.drain_events(256);
    let kinds: Vec<&str> = events
        .iter()
        .map(|e| match &e.kind {
            AgentEventKind::Initialized(_) => "initialized",
            AgentEventKind::SessionReady { .. } => "session",
            AgentEventKind::MessageDelta { .. } => "delta",
            AgentEventKind::ToolCall(_) => "tool",
            AgentEventKind::ToolUpdate(_) => "tool-update",
            AgentEventKind::PromptFinished(_) => "finished",
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "initialized",
            "session",
            "delta",
            "tool",
            "tool-update",
            "finished"
        ]
    );
    assert!(events.windows(2).all(|w| w[0].sequence < w[1].sequence));
    assert!(
        events
            .iter()
            .all(|e| e.provider == "fixture" && e.task == "task-1")
    );
    assert_eq!(
        events[0].session, None,
        "session is unknown before creation"
    );
    assert!(
        events[2..]
            .iter()
            .all(|e| e.session.as_deref() == Some("test-session"))
    );
    assert!(matches!(
        &events[2].kind,
        AgentEventKind::MessageDelta { role: MessageRole::Agent, text } if text == "Hello world"
    ));
    let page = driver.transcript_page(0, 10);
    assert_eq!(page.entries[0].text, "Hello world");

    let outcome = driver.shutdown();
    assert!(outcome.termination.verified());
    assert_eq!(outcome.last_prompt.as_ref().map(|p| p.turn), Some(turn));
    assert!(outcome.failure.is_none());
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn turns_reuse_one_session_and_reject_overlap() {
    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "hang-prompt");
    ready(&driver);
    assert!(matches!(
        driver.prompt("  "),
        Err(DriverError::InvalidPrompt(_))
    ));
    driver.prompt("first").unwrap();
    assert!(matches!(
        driver.prompt("overlap"),
        Err(DriverError::TurnInProgress)
    ));
    drop(driver.shutdown());

    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "late");
    ready(&driver);
    let first = driver.prompt("one").unwrap();
    driver.wait_turn(first, WAIT).unwrap();
    let second = driver.prompt("two").unwrap();
    assert_eq!(second, first + 1);
    driver.wait_turn(second, WAIT).unwrap();
    assert_eq!(driver.session_id().as_deref(), Some("test-session"));
    drop(driver.shutdown());
}

#[test]
fn refusal_and_exhausted_limits_are_never_quiescence_eligible() {
    for (mode, expected) in [
        ("refuse", StopReasonKind::Refusal),
        ("max-tokens", StopReasonKind::MaxTokens),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (driver, _manager) = start(root.path(), mode);
        ready(&driver);
        let turn = driver.prompt("go").unwrap();
        let outcome = driver.wait_turn(turn, WAIT).unwrap();
        assert_eq!(outcome.stop_reason, expected);
        assert!(!outcome.eligible_for_quiescence());
        drop(driver.shutdown());
    }
}

#[test]
fn unsupported_version_and_unknown_stop_reason_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let (driver, manager) = start(root.path(), "version2");
    let failure = failure_of(&driver);
    assert_eq!(failure.kind, FailureKind::UnsupportedVersion);
    assert_eq!(failure.phase, Phase::Initialize);
    assert!(failure.message.contains("version 2"), "{failure:?}");
    let outcome = driver.shutdown();
    assert!(outcome.termination.verified());
    assert_eq!(manager.active_count(), 0);

    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "unknown-stop");
    ready(&driver);
    let turn = driver.prompt("go").unwrap();
    let failure = driver.wait_turn(turn, WAIT).unwrap_err();
    assert_eq!(failure.kind, FailureKind::ProtocolViolation, "{failure:?}");
    assert!(
        driver.last_prompt().is_none(),
        "invalid stop reason never recorded"
    );
    drop(driver.shutdown());
}

// ---- framing failures ------------------------------------------------------------------

#[test]
fn malformed_truncated_and_oversized_messages_fail_the_driver_visibly() {
    for (mode, kind) in [
        ("garbage-init", FailureKind::MalformedMessage),
        ("truncated", FailureKind::TruncatedMessage),
        ("oversized", FailureKind::OversizedMessage),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (driver, manager) = start(root.path(), mode);
        let failure = failure_of(&driver);
        assert_eq!(failure.kind, kind, "{mode}: {failure:?}");
        assert_eq!(failure.phase, Phase::Initialize);
        let outcome = driver.shutdown();
        assert!(outcome.termination.verified(), "{mode}");
        assert_eq!(outcome.failure.map(|f| f.kind), Some(kind));
        assert_eq!(manager.active_count(), 0, "{mode}");
    }
}

#[test]
fn mid_turn_protocol_failures_stop_the_tree_and_surface_one_failure_event() {
    for (mode, kind) in [
        ("malformed-mid", FailureKind::MalformedMessage),
        ("truncated-mid", FailureKind::TruncatedMessage),
        ("oversized-mid", FailureKind::OversizedMessage),
        ("wrong-session", FailureKind::ProtocolViolation),
        ("crash-mid", FailureKind::ProcessExited),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (driver, manager) = start(root.path(), mode);
        ready(&driver);
        let turn = driver.prompt("go").unwrap();
        let failure = driver.wait_turn(turn, WAIT).unwrap_err();
        assert_eq!(failure.kind, kind, "{mode}: {failure:?}");
        assert_eq!(failure.phase, Phase::Prompt, "{mode}");
        if mode == "crash-mid" {
            assert_eq!(failure.exit_code, Some(7));
        }
        assert!(matches!(
            driver.prompt("again"),
            Err(DriverError::Failed(_)) | Err(DriverError::Closed)
        ));
        let outcome = driver.shutdown();
        assert!(outcome.termination.verified(), "{mode}");
        assert_eq!(manager.active_count(), 0, "{mode}");
    }
}

// ---- discovery and auth status -----------------------------------------------------------

fn probe(root: &Path, mode: &str, verify: bool, method: Option<&str>) -> AdapterStatus {
    let config = AdapterConfig {
        executable: python(),
        args: vec![peer_script(), mode.into(), root.to_string_lossy().into()],
        auth_env_names: vec![],
    };
    let mut options = ProbeOptions::new(root.to_owned());
    options.verify_session = verify;
    options.auth_method = method.map(str::to_owned);
    options.timeout = Duration::from_secs(10);
    let manager = ProcessTreeManager::new();
    let report = probe_adapter(&config, &search(), &options, &manager);
    assert_eq!(manager.active_count(), 0, "probe leaves no process behind");
    report.status
}

#[test]
fn discovery_reports_distinct_executable_runtime_protocol_and_auth_states() {
    let root = tempfile::tempdir().unwrap();
    assert!(matches!(
        probe(root.path(), "ok", true, None),
        AdapterStatus::Ready { ref agent, .. } if agent == "protocol-peer"
    ));
    assert!(matches!(
        probe(root.path(), "ok", false, None),
        AdapterStatus::AuthUnknown { .. }
    ));
    assert_eq!(
        probe(root.path(), "auth-required", true, None),
        AdapterStatus::AuthRequired {
            methods: vec!["key".into()]
        }
    );
    assert!(matches!(
        probe(root.path(), "auth-rejected", true, Some("key")),
        AdapterStatus::AuthRejected { .. }
    ));
    assert!(matches!(
        probe(root.path(), "auth-ok", true, Some("key")),
        AdapterStatus::Ready { .. }
    ));
    assert!(matches!(
        probe(root.path(), "version2", true, None),
        AdapterStatus::ProtocolMismatch { .. }
    ));
    assert!(matches!(
        probe(root.path(), "garbage-init", true, None),
        AdapterStatus::ProtocolMismatch { .. }
    ));
    assert!(matches!(
        probe(root.path(), "exit-early", true, None),
        AdapterStatus::Failed { ref failure }
            if failure.kind == FailureKind::ProcessExited && failure.exit_code == Some(3)
    ));
    // An unadvertised authentication method is rejected, never silently skipped.
    assert!(matches!(
        probe(root.path(), "ok", true, Some("key")),
        AdapterStatus::AuthRejected { .. }
    ));
}

#[test]
fn discovery_distinguishes_a_missing_executable_from_a_missing_runtime() {
    let root = tempfile::tempdir().unwrap();
    let config = AdapterConfig {
        executable: "no-such-adapter-binary".into(),
        args: vec![],
        auth_env_names: vec![],
    };
    let options = ProbeOptions::new(root.path().to_owned());
    let manager = ProcessTreeManager::new();
    let report = probe_adapter(&config, &search(), &options, &manager);
    assert!(matches!(
        report.status,
        AdapterStatus::MissingExecutable { .. }
    ));
    assert!(report.executable.is_none());
    assert!(matches!(
        AdapterLaunch::resolve(&config, &search()),
        Err(ResolveError::NotFound { .. })
    ));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = root.path().join("adapter-needs-runtime");
        std::fs::write(&script, "#!/usr/bin/env definitely-missing-runtime-xyz\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let config = AdapterConfig {
            executable: script.to_string_lossy().into(),
            args: vec![],
            auth_env_names: vec![],
        };
        // A concurrent fork in a sibling test can briefly hold the freshly written
        // script open for writing (ETXTBSY); retry only that known test-side race.
        let mut report = probe_adapter(&config, &search(), &options, &manager);
        for _ in 0..40 {
            let busy = matches!(&report.status, AdapterStatus::Failed { failure }
                if failure.message.contains("Text file busy"));
            if !busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
            report = probe_adapter(&config, &search(), &options, &manager);
        }
        assert!(
            matches!(report.status, AdapterStatus::MissingRuntime { .. }),
            "{:?}",
            report.status
        );
        assert_eq!(report.executable.as_deref(), Some(script.as_path()));
    }
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn authenticate_flow_requires_an_advertised_method_and_proves_auth_only_via_a_session() {
    let root = tempfile::tempdir().unwrap();
    let mut cfg = config(root.path(), "auth-ok");
    cfg.launch = cfg.launch.with_auth_method("key");
    let (driver, _manager) = start_with(cfg);
    let info = driver.wait_ready(WAIT).unwrap();
    assert_eq!(info.initialized.auth_methods, vec!["key".to_owned()]);
    assert!(
        read_json(root.path(), "calls.json")
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m == "authenticate")
    );
    drop(driver.shutdown());

    let root = tempfile::tempdir().unwrap();
    let (driver, _manager) = start(root.path(), "auth-required");
    let failure = failure_of(&driver);
    assert_eq!(failure.kind, FailureKind::AuthRequired);
    assert_eq!(failure.code, Some(-32000));
    assert_eq!(failure.phase, Phase::Session);
    drop(driver.shutdown());
}
