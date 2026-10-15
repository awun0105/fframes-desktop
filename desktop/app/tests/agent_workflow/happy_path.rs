//! The happy path: a brief becomes a validated, applied candidate; tools and advertised options.

use super::*;

#[test]
fn a_brief_becomes_a_validated_candidate_is_auto_applied_and_accepted() {
    let w = World::new();
    let original = w.source("src/lib.rs");
    w.submit(&good("first"));
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(task.engine_state, TaskState::Accepted);
    assert_eq!(w.engine_state(), Some(TaskState::Accepted));
    assert_ne!(w.source("src/lib.rs"), original);
    assert_eq!(w.source("src/lib.rs"), "// first\n");
    assert_eq!(task.review_policy, studio_engine::ReviewPolicy::AutoApply);
    assert_eq!(
        task.repair.used, 0,
        "validation failed once: {:?} / {:?}",
        task.repair, task.validation
    );
    assert_eq!(task.turns, 1);
    assert_eq!(
        task.writer.as_ref().unwrap().ownership,
        "process-group-contained"
    );
    assert!(task.writer.as_ref().unwrap().provider_session.is_some());
    assert!(matches!(
        task.draft_state,
        Some(DraftState::Accepted { .. })
    ));

    let s = w.snap();
    // Ordered rows: strictly increasing ids, the brief first.
    let ids: Vec<u64> = s.rows.iter().map(|r| r.id.0).collect();
    assert!(ids.windows(2).all(|p| p[0] < p[1]), "{ids:?}");
    assert!(matches!(
        &s.rows[0].kind,
        RowKind::User { source: UserSource::Brief, text } if text.starts_with("SCRIPT")
    ));
    // Streamed text coalesced; tool deltas updated the SAME two cards in place.
    let agent_text: String = s
        .rows
        .iter()
        .filter_map(|r| match &r.kind {
            RowKind::Agent {
                text, streaming, ..
            } => {
                assert!(!streaming, "the stream closed with the turn");
                Some(text.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(agent_text, "Working on it. done");
    let tools: Vec<&ToolCard> = s
        .rows
        .iter()
        .filter_map(|r| match &r.kind {
            RowKind::Tool(card) => Some(card),
            _ => None,
        })
        .collect();
    assert_eq!(tools.len(), 2, "tool updates must not append rows");
    assert!(tools.iter().all(|t| {
        t.status == Some(studio_harness::driver::ToolStatus::Completed) && t.updates == 2
    }));
    // Changed-file summary and validation coverage/diagnostics.
    let changes = task.changes.as_ref().expect("changed files");
    assert_eq!(changes.total, 2);
    assert!(changes.entries.iter().any(|e| e.path == "src/lib.rs"));
    let validation = task.validation.as_ref().expect("validation card");
    assert!(validation.passed, "{}", validation.summary);
    assert!(validation.coverage.as_ref().unwrap().rendered_frames > 0);
    assert!(validation.audio.as_ref().unwrap().placement_verified);
    assert!(
        s.rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Validation(c) if c.passed))
    );
    assert!(
        s.rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Outcome(o) if o.kind == OutcomeKind::Accepted))
    );
    // Advertised options and capabilities (limits to show); nothing else settable.
    assert_eq!(
        s.options
            .modes
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        ["code", "plan"]
    );
    assert!(s.capabilities.is_some());
    // History, Undo and the awaiting-preview state (no preview sink configured).
    assert_eq!(s.history.len(), 1);
    assert!(matches!(s.undo, UndoView::Available { .. }));
    let handoff = s.handoff.as_ref().unwrap();
    assert_eq!(handoff.kind, HandoffKind::Apply);
    assert_eq!(
        handoff.state,
        HandoffState::AwaitingPreview { reason: None }
    );
    w.assert_clean();
}

#[test]
fn project_tools_are_offered_over_mcp_only_when_policy_allows_and_die_with_the_task() {
    let w = World::new();
    w.submit(&json!({"hang": true}));
    w.wait_phase(TaskPhase::Editing);
    w.wait_prompts(1);
    let params: Value =
        serde_json::from_str(&fs::read_to_string(w.agent.join("session-params.json")).unwrap())
            .unwrap();
    let servers = params["mcpServers"].as_array().unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0]["name"], "fframes-studio");
    let args = servers[0]["args"].as_array().unwrap();
    assert_eq!(args[0], "--capability");
    let capability = PathBuf::from(args[1].as_str().unwrap());
    assert!(
        capability.is_file(),
        "the capability exists while the task runs"
    );
    assert!(
        servers[0]["env"].as_array().unwrap().is_empty(),
        "no secret in the environment"
    );
    let s = w.snap();
    assert!(s.mcp.policy_enabled && s.mcp.binary_available && s.mcp.active);
    assert_eq!(s.resources.broker_grants, 1);
    w.wf().stop().unwrap();
    w.wait_task(None);
    assert!(!capability.exists(), "the capability died with the task");
    w.assert_clean();

    // Host policy disables MCP: no MCP server, but the CLI route has its own capability
    // (exercised end to end in `the_command_line_route_has_a_task_capability...`).
    let w = World::with(Options {
        mcp: McpStdioSupport::Unsupported,
        ..Options::default()
    });
    w.submit(&json!({"hang": true}));
    w.wait_phase(TaskPhase::Editing);
    w.wait_prompts(1);
    let params: Value =
        serde_json::from_str(&fs::read_to_string(w.agent.join("session-params.json")).unwrap())
            .unwrap();
    assert!(params["mcpServers"].as_array().unwrap().is_empty());
    let s = w.snap();
    assert!(!s.mcp.policy_enabled && !s.mcp.active && s.mcp.cli_active);
    assert_eq!(s.resources.broker_grants, 1);
    w.wf().stop().unwrap();
    w.wait_task(None);
}

#[test]
fn advertised_options_can_be_changed_and_unadvertised_ones_are_refused() {
    let w = World::new();
    w.submit(&json!({"hang": true}));
    w.wait("advertised options", |s| !s.options.modes.is_empty());
    w.wait_phase(TaskPhase::Editing);
    w.wf().set_mode("plan").unwrap();
    assert_eq!(
        w.wf().set_mode("bogus").unwrap_err(),
        WorkflowError::UnknownOption("bogus".into())
    );
    w.wf()
        .set_config_option("model", OptionValue::Value("deep".into()))
        .unwrap();
    assert_eq!(
        w.wf()
            .set_config_option("nope", OptionValue::Boolean(true))
            .unwrap_err(),
        WorkflowError::UnknownOption("nope".into())
    );
    let deadline = Instant::now() + WAIT;
    while !(w.agent.join("set-mode.json").exists() && w.agent.join("set-config.json").exists()) {
        assert!(
            Instant::now() < deadline,
            "the agent never saw the option changes"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // Settings cannot change under a running task.
    assert!(matches!(
        w.wf()
            .set_review_policy(studio_engine::ReviewPolicy::ManualReview),
        Ok(())
    ));
    assert!(matches!(
        w.wf().set_adapter(None),
        Err(WorkflowError::Busy(_))
    ));
    w.wf().stop().unwrap();
    w.wait_task(None);
}
