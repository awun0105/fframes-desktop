//! The tools over the broker: the CLI, MCP sessions and concurrency bounds.

use super::*;

#[cfg(unix)]
#[test]
fn cli_through_the_broker_reaches_the_real_backend_and_stops_after_the_task_ends() {
    use studio_engine::agent_tools::broker::{BrokerConfig, ToolBroker};
    let mut w = world();
    let context = w.start();
    worker_config(
        &context.draft,
        json!({"frames": 40, "tracks": [[0.0, 1.0]], "audio": "tone"}),
    );
    let t = tools(&w);
    let binding = draft_binding(&t, &context);
    // Unix socket paths are short: use a short runtime directory.
    let runtime = tempfile::Builder::new()
        .prefix("fft")
        .tempdir_in("/tmp")
        .unwrap();
    let broker = ToolBroker::start(BrokerConfig {
        runtime_dir: runtime.path().join("rt"),
        dispatcher: ToolDispatcher::new(t.backend.clone()),
        liveness: t.backend.clone(),
    })
    .unwrap();
    let grant = broker
        .grant(binding.clone(), Duration::from_secs(300))
        .unwrap();
    let cli = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_studio-tools"))
            .arg("--capability")
            .arg(&grant.capability_file)
            .args(args)
            .output()
            .unwrap()
    };
    let out = cli(&["timeline"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let via_cli: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(via_cli["result"]["total_frames"], 40);
    assert_eq!(via_cli["revision"]["validated"], false);
    // The facade started no compiler of its own: exactly the backend's one shared compile.
    assert_eq!(w.compiler.started.load(Ordering::SeqCst), 1);
    let frame = cli(&["render_frame", "--json", "{\"frame\":3}"]);
    assert!(frame.status.success());
    let frame: Value = serde_json::from_slice(&frame.stdout).unwrap();
    assert!(Path::new(frame["artifacts"][0]["path"].as_str().unwrap()).is_file());
    let status = cli(&["build_status"]);
    assert!(status.status.success());
    assert_eq!(
        w.compiler.started.load(Ordering::SeqCst),
        1,
        "build_status never compiles"
    );
    // The same call straight through the dispatcher answers identically.
    let direct = call(&t, &binding, "timeline", json!({})).unwrap();
    assert_eq!(direct, via_cli);
    // Ending the task (liveness) refuses the next facade call.
    t.backend.unregister_task(&context.identity.task);
    let stale = cli(&["timeline"]);
    // The capability is refused at connect time: "capability unusable" (exit 3).
    assert_eq!(stale.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&stale.stderr).contains("stale_task"));
    drop(broker);
    assert!(
        !grant.capability_file.exists(),
        "the capability file is removed with the broker"
    );
}

#[test]
fn a_draft_that_names_another_project_is_never_served() {
    let mut w = world();
    let context = w.start();
    let manifest_path = context.draft.join("studio.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    value["project_id"] = json!("another-project");
    fs::write(&manifest_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let t = tools(&w);
    let binding = draft_binding(&t, &context);
    for method in ["project_context", "timeline"] {
        assert_eq!(
            call(&t, &binding, method, json!({})).err(),
            Some(ToolErrorCode::CrossProject),
            "{method}"
        );
    }
    assert_eq!(
        w.compiler.started.load(Ordering::SeqCst),
        0,
        "nothing was built"
    );
}

#[test]
fn a_task_that_ends_mid_call_leaves_no_worker_or_artifact_behind() {
    let mut w = world();
    let context = w.start();
    worker_config(&context.draft, json!({"delay_ms": 250}));
    let t = tools(&w);
    let binding = draft_binding(&t, &context);
    let task = context.identity.task.clone();
    // The task ends while its worker is still starting.
    let during_launch = std::thread::scope(|s| {
        let running = s.spawn(|| call(&t, &binding, "render_frame", json!({"frame": 1})));
        std::thread::sleep(Duration::from_millis(300));
        t.backend.unregister_task(&task);
        running.join().unwrap()
    });
    assert_eq!(during_launch.err(), Some(ToolErrorCode::StaleTask));
    assert_eq!(t.backend.worker_count(), 0);
    assert!(t.backend.artifacts().is_empty());

    // The task ends while a frame is rendering: the image is dropped, not stored.
    let binding = draft_binding(&t, &context);
    call(&t, &binding, "timeline", json!({})).unwrap();
    assert_eq!(t.backend.worker_count(), 1);
    let during_render = std::thread::scope(|s| {
        let running = s.spawn(|| call(&t, &binding, "render_frame", json!({"frame": 2})));
        std::thread::sleep(Duration::from_millis(100));
        t.backend.unregister_task(&task);
        running.join().unwrap()
    });
    assert_eq!(during_render.err(), Some(ToolErrorCode::StaleTask));
    assert_eq!(t.backend.worker_count(), 0);
    assert!(t.backend.artifacts().is_empty());
}

#[test]
fn concurrent_calls_on_distinct_revisions_never_exceed_the_worker_bound() {
    use studio_engine::candidate_validation::capture_draft_revision;
    let mut w = world();
    let context = w.start();
    worker_config(&context.draft, json!({"delay_ms": 60}));
    let store = w.checkpoints();
    let revisions: Vec<_> = (0..4)
        .map(|i| {
            fs::write(
                context.draft.join("src/lib.rs"),
                format!("// revision {i}\n"),
            )
            .unwrap();
            capture_draft_revision(&store, &context.draft).unwrap()
        })
        .collect();
    let t = tools(&w);
    t.backend.register_task(
        &context.identity,
        context.draft.clone(),
        context.source_base.revision().clone(),
    );
    let done = std::sync::atomic::AtomicBool::new(false);
    let (results, high_water) = std::thread::scope(|s| {
        let monitor = s.spawn(|| {
            let mut high = 0;
            while !done.load(Ordering::SeqCst) {
                high = high.max(t.backend.worker_count());
                std::thread::sleep(Duration::from_millis(1));
            }
            high
        });
        let callers: Vec<_> = revisions
            .iter()
            .map(|revision| {
                let binding = ToolBinding {
                    task: context.identity.clone(),
                    revision: BoundRevision::Fixed(revision.clone()),
                };
                let t = &t;
                s.spawn(move || call(t, &binding, "timeline", json!({})))
            })
            .collect();
        let results: Vec<_> = callers.into_iter().map(|c| c.join().unwrap()).collect();
        done.store(true, Ordering::SeqCst);
        (results, monitor.join().unwrap())
    });
    assert!(high_water <= 2, "worker bound exceeded: {high_water}");
    assert!(results.iter().any(Result::is_ok), "{results:?}");
    for result in &results {
        assert!(
            matches!(result, Ok(_) | Err(ToolErrorCode::Busy)),
            "{result:?}"
        );
    }
    assert!(t.backend.worker_count() <= 2);
}

#[cfg(unix)]
#[test]
fn cli_and_mcp_agree_on_revision_artifact_hashes_and_errors_over_the_real_backend() {
    use studio_engine::agent_tools::broker::{BrokerConfig, ToolBroker};
    let mut w = world();
    let context = w.start();
    worker_config(
        &context.draft,
        json!({"frames": 40, "tracks": [[0.0, 1.0]], "audio": "tone", "pixel": 21}),
    );
    let t = tools(&w);
    let binding = draft_binding(&t, &context);
    let runtime = tempfile::Builder::new()
        .prefix("fft")
        .tempdir_in("/tmp")
        .unwrap();
    let broker = ToolBroker::start(BrokerConfig {
        runtime_dir: runtime.path().join("rt"),
        dispatcher: ToolDispatcher::new(t.backend.clone()),
        liveness: t.backend.clone(),
    })
    .unwrap();
    let grant = broker.grant(binding, Duration::from_secs(300)).unwrap();
    let cli = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_studio-tools"))
            .arg("--capability")
            .arg(&grant.capability_file)
            .args(args)
            .output()
            .unwrap()
    };
    let call_mcp = |id: u64, name: &str, arguments: Value| json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}});
    let replies = mcp_session(
        &grant.capability_file,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}),
            call_mcp(2, "timeline", json!({})),
            call_mcp(3, "render_frame", json!({"frame": 5})),
            call_mcp(4, "render_frame", json!({"frame": 9999})),
            call_mcp(5, "inspect", json!({"count": 4, "start": 0, "end": 39})),
            call_mcp(6, "project_context", json!({})),
        ],
    );
    assert_eq!(replies.len(), 6);

    // timeline: identical result, revision and label.
    let out = cli(&["timeline"]);
    assert!(out.status.success());
    let cli_timeline: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(replies[1]["result"]["structuredContent"], cli_timeline);

    // render_frame: same revision, same pixels (hash), distinct app-owned artifact files.
    let out = cli(&["render_frame", "--json", "{\"frame\":5}"]);
    assert!(out.status.success());
    let cli_frame: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mcp_frame = &replies[2]["result"]["structuredContent"];
    assert_eq!(mcp_frame["revision"], cli_frame["revision"]);
    assert_eq!(mcp_frame["result"], cli_frame["result"]);
    assert_eq!(
        mcp_frame["artifacts"][0]["sha256"],
        cli_frame["artifacts"][0]["sha256"]
    );
    assert!(Path::new(mcp_frame["artifacts"][0]["path"].as_str().unwrap()).is_file());

    // The same out-of-range request fails with the same tool error code on both.
    let out = cli(&["render_frame", "--json", "{\"frame\":9999}"]);
    assert_eq!(out.status.code(), Some(1));
    let cli_error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(replies[3]["result"]["isError"], true);
    let mcp_error: Value =
        serde_json::from_str(replies[3]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(mcp_error, cli_error["error"]);
    assert_eq!(mcp_error["code"], "invalid_params");

    // inspect and project_context agree too.
    let out = cli(&["inspect", "--json", "{\"count\":4,\"start\":0,\"end\":39}"]);
    let cli_inspect: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(replies[4]["result"]["structuredContent"], cli_inspect);
    let out = cli(&["project_context"]);
    let cli_context: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(replies[5]["result"]["structuredContent"], cli_context);
    // One compile served every facade call.
    assert_eq!(w.compiler.started.load(Ordering::SeqCst), 1);
    drop(broker);
}
