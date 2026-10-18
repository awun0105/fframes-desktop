//! The six project tools for labelled draft and captured revisions.

use super::*;

#[test]
fn six_tools_answer_for_labelled_immutable_draft_revisions() {
    let mut w = world();
    let context = w.start();
    worker_config(
        &context.draft,
        json!({"frames": 60, "scenes": [[0, 30], [30, 60]], "tracks": [[0.0, 1.0]], "pixel": 33,
               "inspect": [{"frame": 3, "severity": "warning", "key": "shader_unsupported", "message": "falls back"}]}),
    );
    let t = tools(&w);
    let binding = draft_binding(&t, &context);

    // build_status observes: no capture, no compile.
    let status = call(&t, &binding, "build_status", json!({})).unwrap();
    assert_eq!(w.compiler.started.load(Ordering::SeqCst), 0);
    assert_eq!(status["revision"]["label"], "task_base");
    assert_eq!(status["revision"]["validated"], false);
    assert_eq!(status["result"]["service"]["compiles_started"], 0);

    let ctx_reply = call(&t, &binding, "project_context", json!({})).unwrap();
    let first_revision = ctx_reply["revision"]["id"].as_str().unwrap().to_owned();
    assert_eq!(ctx_reply["revision"]["label"], "draft_snapshot");
    assert_eq!(ctx_reply["revision"]["validated"], false);
    assert_ne!(first_revision, context.source_base.revision().as_str());
    let files = ctx_reply["result"]["files"].as_array().unwrap();
    assert!(files.iter().any(|f| f["path"] == "src/lib.rs"));
    assert_eq!(
        ctx_reply["result"]["task"]["base_revision"],
        context.source_base.revision().as_str()
    );
    assert_eq!(
        w.compiler.started.load(Ordering::SeqCst),
        0,
        "project_context never compiles"
    );

    let timeline = call(&t, &binding, "timeline", json!({})).unwrap();
    assert_eq!(timeline["result"]["total_frames"], 60);
    assert_eq!(timeline["result"]["scenes"].as_array().unwrap().len(), 2);
    assert_eq!(timeline["revision"]["id"], first_revision);
    assert_eq!(w.compiler.started.load(Ordering::SeqCst), 1);

    let frame = call(
        &t,
        &binding,
        "render_frame",
        json!({"frame": 4, "scale": 1.0}),
    )
    .unwrap();
    let artifact = &frame["artifacts"][0];
    assert_eq!(artifact["media_type"], "image/png");
    let path = PathBuf::from(artifact["path"].as_str().unwrap());
    assert!(path.starts_with(t.backend.artifacts().root()));
    let bytes = fs::read(&path).unwrap();
    assert_eq!(artifact["bytes"], bytes.len());
    assert!(bytes.len() <= 8 * 1024 * 1024);
    assert_eq!(artifact["sha256"], format!("{:x}", sha2_hex(&bytes)));
    let decoded = image::load_from_memory(&bytes).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (8, 4));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert!(!serde_json::to_string(&frame).unwrap().contains("base64"));

    let strip = call(
        &t,
        &binding,
        "render_strip",
        json!({"start": 0, "end": 59, "count": 5, "scale": 1.0}),
    )
    .unwrap();
    assert_eq!(strip["result"]["frames"].as_array().unwrap().len(), 5);
    assert_eq!(strip["result"]["width"], 8 * 5);
    assert_eq!(strip["artifacts"].as_array().unwrap().len(), 1);

    let inspect = call(&t, &binding, "inspect", json!({"frames": [3, 10]})).unwrap();
    assert_eq!(
        inspect["result"]["diagnostics"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        inspect["result"]["diagnostics"][0]["key"],
        "shader_unsupported"
    );
    // Out-of-range frames and ranges are rejected before any worker I/O.
    assert_eq!(
        call(&t, &binding, "inspect", json!({"frames": [60]})).err(),
        Some(ToolErrorCode::InvalidParams)
    );
    assert_eq!(
        call(&t, &binding, "render_frame", json!({"frame": 60})).err(),
        Some(ToolErrorCode::InvalidParams)
    );
    assert_eq!(
        call(
            &t,
            &binding,
            "render_strip",
            json!({"start": 0, "end": 500, "count": 4})
        )
        .err(),
        Some(ToolErrorCode::InvalidParams)
    );
    assert_eq!(
        call(
            &t,
            &binding,
            "render_strip",
            json!({"start": 0, "end": 5, "count": 25})
        )
        .err(),
        Some(ToolErrorCode::InvalidParams)
    );

    // The same draft revision reused one compile and one worker.
    assert_eq!(w.compiler.started.load(Ordering::SeqCst), 1);
    assert_eq!(t.backend.worker_count(), 1);

    // An edit produces a different labelled revision (and a different build key).
    worker_config(&context.draft, json!({"frames": 90}));
    let second = call(&t, &binding, "timeline", json!({})).unwrap();
    assert_ne!(second["revision"]["id"], first_revision);
    assert_eq!(second["result"]["total_frames"], 90);
    assert_eq!(w.compiler.started.load(Ordering::SeqCst), 2);
    // An earlier snapshot of this task can still be asked about; a stranger cannot.
    let again = call(
        &t,
        &binding,
        "timeline",
        json!({"revision": first_revision}),
    )
    .unwrap();
    assert_eq!(again["result"]["total_frames"], 60);
    assert_eq!(
        call(
            &t,
            &binding,
            "timeline",
            json!({"revision": "f".repeat(64)})
        )
        .err(),
        Some(ToolErrorCode::StaleRevision)
    );
    // Tool workers are bounded; each uses its own scope, never the UI lanes.
    assert!(t.backend.worker_count() <= 2);
}

#[test]
fn cached_revision_labels_follow_the_binding_not_the_open_order() {
    for draft_first in [false, true] {
        let mut w = world();
        let context = w.start();
        let t = tools(&w);
        let draft = draft_binding(&t, &context);
        let base_revision = context.source_base.revision().clone();
        let base = ToolBinding {
            task: context.identity.clone(),
            revision: BoundRevision::Fixed(base_revision.clone()),
        };

        if draft_first {
            let draft_reply = call(&t, &draft, "project_context", json!({})).unwrap();
            assert_eq!(draft_reply["revision"]["id"], base_revision.as_str());
            assert_eq!(draft_reply["revision"]["label"], "draft_snapshot");

            let base_reply = call(&t, &base, "project_context", json!({})).unwrap();
            assert_eq!(base_reply["revision"]["id"], base_revision.as_str());
            assert_eq!(base_reply["revision"]["label"], "task_base");
        } else {
            let base_reply = call(&t, &base, "project_context", json!({})).unwrap();
            assert_eq!(base_reply["revision"]["label"], "task_base");

            let draft_reply = call(
                &t,
                &draft,
                "project_context",
                json!({"revision": base_revision.as_str()}),
            )
            .unwrap();
            assert_eq!(draft_reply["revision"]["id"], base_revision.as_str());
            assert_eq!(draft_reply["revision"]["label"], "draft_snapshot");
        }

        t.backend.close();
    }
}

#[test]
fn draft_requests_return_busy_while_the_writer_gate_is_held_and_never_compile() {
    let mut w = world();
    let context = w.start();
    let t = tools(&w);
    let binding = draft_binding(&t, &context);
    let guard = t.backend.writer_gate().try_acquire().unwrap();
    assert_eq!(
        call(&t, &binding, "timeline", json!({})).err(),
        Some(ToolErrorCode::Busy)
    );
    assert_eq!(
        call(&t, &binding, "project_context", json!({})).err(),
        Some(ToolErrorCode::Busy)
    );
    assert_eq!(w.compiler.started.load(Ordering::SeqCst), 0);
    drop(guard);
    assert!(call(&t, &binding, "project_context", json!({})).is_ok());
}

#[test]
fn fixed_bindings_answer_for_the_captured_candidate_and_share_the_validation_build() {
    let mut w = world();
    let context = w.start();
    worker_config(&context.draft, json!({"frames": 45}));
    let captured = w.capture(&context);
    let run = w.run(&captured, 0);
    assert!(run.report.passed());
    drop(run);
    assert_eq!(w.compiler.started.load(Ordering::SeqCst), 1);
    let t = tools(&w);
    t.backend.register_task(
        &context.identity,
        context.draft.clone(),
        context.source_base.revision().clone(),
    );
    let binding = ToolBinding {
        task: context.identity.clone(),
        revision: BoundRevision::Fixed(captured.candidate().revision().clone()),
    };
    let timeline = call(&t, &binding, "timeline", json!({})).unwrap();
    assert_eq!(timeline["revision"]["label"], "candidate");
    assert_eq!(
        timeline["revision"]["id"],
        captured.candidate().revision().as_str()
    );
    assert_eq!(timeline["result"]["total_frames"], 45);
    assert_eq!(
        w.compiler.started.load(Ordering::SeqCst),
        1,
        "tools joined the validation build"
    );
    // An assertion naming another revision is rejected, as is an unknown fixed revision.
    assert_eq!(
        call(
            &t,
            &binding,
            "timeline",
            json!({"revision": "a".repeat(64)})
        )
        .err(),
        Some(ToolErrorCode::StaleRevision)
    );
    let missing = ToolBinding {
        task: context.identity.clone(),
        revision: BoundRevision::Fixed("e".repeat(64).try_into().unwrap()),
    };
    assert_eq!(
        call(&t, &missing, "timeline", json!({})).err(),
        Some(ToolErrorCode::Unavailable)
    );
    let environment = studio_engine::services::build_service::CompileEnvironment::resolve(
        &w.sdk,
        &manifest(),
        &w.paths.builds(),
    )
    .unwrap();
    let key = BuildKey::worker(&studio_project::open(&context.draft).unwrap(), &environment);
    assert!(matches!(
        w.service.status(&key),
        studio_engine::services::build_service::BuildState::Ready { .. }
    ));
}

#[test]
fn captured_snapshot_history_is_bounded_and_evicted_identities_are_stale() {
    use studio_engine::agent_tools::{
        MAX_TEXT_REPLY_BYTES,
        backend::{MAX_KNOWN_REVISIONS, STATUS_KNOWN_REVISIONS},
    };
    let mut w = world();
    let context = w.start();
    let t = tools(&w);
    let binding = draft_binding(&t, &context);
    let captured = MAX_KNOWN_REVISIONS + 8;
    let mut ids = Vec::new();
    for n in 0..captured {
        worker_config(&context.draft, json!({"frames": 10 + n}));
        let reply = call(&t, &binding, "project_context", json!({})).unwrap();
        let id = reply["revision"]["id"].as_str().unwrap().to_owned();
        assert!(!ids.contains(&id), "every edit is a distinct snapshot");
        ids.push(id);
    }

    let status = call(&t, &binding, "build_status", json!({})).unwrap();
    let listed = status["result"]["known_revisions"].as_array().unwrap();
    assert_eq!(listed.len(), STATUS_KNOWN_REVISIONS);
    assert_eq!(listed.last().unwrap(), ids.last().unwrap().as_str());
    assert_eq!(status["result"]["known_revisions_total"], captured);
    assert_eq!(status["result"]["known_revisions_truncated"], true);
    assert!(serde_json::to_vec(&status).unwrap().len() < MAX_TEXT_REPLY_BYTES);

    // The oldest identities were evicted: the existing stale error, never a restore.
    let evicted = captured - MAX_KNOWN_REVISIONS;
    for id in &ids[..evicted] {
        assert_eq!(
            call(&t, &binding, "project_context", json!({"revision": id})).err(),
            Some(ToolErrorCode::StaleRevision)
        );
    }
    // The oldest retained and the newest identities are still valid.
    for id in [&ids[evicted], ids.last().unwrap()] {
        let again = call(&t, &binding, "project_context", json!({"revision": id})).unwrap();
        assert_eq!(again["revision"]["id"], id.as_str());
    }
}

#[test]
fn tool_access_is_bound_to_the_task_and_project_and_closes_cleanly() {
    let mut w = world();
    let context = w.start();
    worker_config(&context.draft, json!({"frames": 30}));
    let t = tools(&w);
    let binding = draft_binding(&t, &context);
    call(&t, &binding, "render_frame", json!({"frame": 1})).unwrap();
    assert_eq!(t.backend.worker_count(), 1);
    assert!(!t.backend.artifacts().is_empty());
    // Another generation of "the same" task is stale; an unknown task is stale.
    let mut stale = binding.clone();
    stale.task.generation += 1;
    assert_eq!(
        call(&t, &stale, "timeline", json!({})).err(),
        Some(ToolErrorCode::StaleTask)
    );
    // Cross-project: an explicit assertion and a foreign identity are both refused.
    assert_eq!(
        call(&t, &binding, "timeline", json!({"project": "someone-else"})).err(),
        Some(ToolErrorCode::CrossProject)
    );
    let mut foreign = binding.clone();
    foreign.task.project = "foreign-project".to_owned().try_into().unwrap();
    assert_eq!(
        call(&t, &foreign, "timeline", json!({})).err(),
        Some(ToolErrorCode::CrossProject)
    );
    // Unknown methods and fields never reach the backend.
    assert_eq!(
        call(&t, &binding, "shell", json!({})).err(),
        Some(ToolErrorCode::MethodNotFound)
    );
    assert_eq!(
        call(&t, &binding, "timeline", json!({"path": "../x"})).err(),
        Some(ToolErrorCode::InvalidParams)
    );
    // Task end releases the workers, restored trees and artifacts it owned.
    assert!(studio_engine::agent_tools::TaskLiveness::is_live(
        &*t.backend,
        &context.identity
    ));
    t.backend.unregister_task(&context.identity.task);
    assert_eq!(t.backend.worker_count(), 0);
    assert!(t.backend.artifacts().is_empty());
    assert_eq!(
        call(&t, &binding, "timeline", json!({})).err(),
        Some(ToolErrorCode::StaleTask)
    );
    assert!(!studio_engine::agent_tools::TaskLiveness::is_live(
        &*t.backend,
        &context.identity
    ));
    let deadline = Instant::now() + Duration::from_secs(5);
    while w.controller.processes.active_count() > 0 {
        assert!(Instant::now() < deadline, "tool worker survived task end");
        std::thread::sleep(Duration::from_millis(10));
    }
    t.backend.close();
    assert!(t.artifacts.exists());
}

#[test]
fn at_most_two_tool_workers_live_and_close_releases_all_of_them() {
    let mut w = world();
    let context = w.start();
    let t = tools(&w);
    let binding = draft_binding(&t, &context);
    for frames in [30, 31, 32, 33] {
        worker_config(&context.draft, json!({"frames": frames}));
        let reply = call(&t, &binding, "timeline", json!({})).unwrap();
        assert_eq!(reply["result"]["total_frames"], frames);
        assert!(t.backend.worker_count() <= 2, "worker bound");
    }
    assert_eq!(t.backend.worker_count(), 2);
    t.backend.close();
    assert_eq!(t.backend.worker_count(), 0);
    assert_eq!(
        call(&t, &binding, "timeline", json!({})).err(),
        Some(ToolErrorCode::StaleTask)
    );
}

#[test]
fn artifact_store_enforces_ids_expiry_size_and_symlink_containment() {
    let temp = tempfile::tempdir().unwrap();
    let store =
        ArtifactStore::with_limit(&temp.path().join("art"), Duration::from_millis(400), 4096)
            .unwrap();
    let task = studio_engine::AgentTaskId::new();
    let png = vec![0u8; 100];
    let artifact = store.put_png(&task, &png, 1, 1).unwrap();
    assert!(artifact.id.starts_with("art-") && artifact.id.len() == 36);
    assert_eq!(
        store.read_png_for_task(&task, &artifact.id, 100).unwrap(),
        png
    );
    assert_eq!(
        store
            .read_png_for_task(&studio_engine::AgentTaskId::new(), &artifact.id, 100)
            .err()
            .map(|error| error.code),
        Some(ToolErrorCode::Unauthorized)
    );
    assert_eq!(
        store
            .read_png_for_task(&task, &artifact.id, 99)
            .err()
            .map(|error| error.code),
        Some(ToolErrorCode::TooLarge)
    );
    assert_eq!(
        store.resolve(&artifact.id).unwrap(),
        PathBuf::from(&artifact.path)
    );
    for bad in [
        "../x",
        "art-../../etc/passwd",
        "art-xyz",
        "",
        "ART-0123456789abcdef0123456789abcdef",
    ] {
        assert_eq!(
            store.resolve(bad).err().map(|e| e.code),
            Some(ToolErrorCode::InvalidParams),
            "{bad}"
        );
    }
    assert_eq!(
        store
            .resolve(&format!("art-{}", "0".repeat(32)))
            .err()
            .map(|e| e.code),
        Some(ToolErrorCode::NotFound)
    );
    // Oversized images are refused; the total store size is bounded by eviction.
    assert_eq!(
        store
            .put_png(&task, &vec![0u8; 8 * 1024 * 1024 + 1], 1, 1)
            .err()
            .map(|e| e.code),
        Some(ToolErrorCode::TooLarge)
    );
    for _ in 0..60 {
        store.put_png(&task, &[1u8; 100], 1, 1).unwrap();
    }
    assert!(store.len() <= 40, "the store stays within its byte limit");
    // A symlink swapped in for an artifact file is refused.
    #[cfg(unix)]
    {
        let victim = store.put_png(&task, &png, 1, 1).unwrap();
        let outside = temp.path().join("outside.png");
        fs::write(&outside, b"secret").unwrap();
        fs::remove_file(&victim.path).unwrap();
        std::os::unix::fs::symlink(&outside, &victim.path).unwrap();
        assert_eq!(
            store.resolve(&victim.id).err().map(|e| e.code),
            Some(ToolErrorCode::Unauthorized)
        );
    }
    // Expiry removes the file and the id.
    let short = store.put_png(&task, &png, 1, 1).unwrap();
    std::thread::sleep(Duration::from_millis(450));
    assert_eq!(
        store.resolve(&short.id).err().map(|e| e.code),
        Some(ToolErrorCode::NotFound)
    );
    assert!(!Path::new(&short.path).exists());
    // Task removal drops everything the task owned.
    let owned = store.put_png(&task, &png, 1, 1).unwrap();
    store.remove_task(&task);
    assert!(!Path::new(&owned.path).exists());
    assert!(store.is_empty());
}

#[test]
fn mcp_session_config_passes_only_the_capability_path() {
    let temp = tempfile::tempdir().unwrap();
    let capability = temp.path().join("cap-test.json");
    let secret = "s".repeat(64);
    fs::write(&capability, format!("{{\"secret\":\"{secret}\"}}")).unwrap();
    let grant = studio_engine::agent_tools::broker::ToolGrant {
        capability_file: capability.clone(),
        expires_at: std::time::SystemTime::now(),
    };
    let command = temp.path().join("studio-mcp");
    let server = studio_engine::agent_tools::mcp_server_for(&grant, &command).unwrap();
    assert_eq!(server.command, command);
    assert_eq!(server.args, ["--capability", capability.to_str().unwrap()]);
    assert!(server.env.is_empty());
    assert!(!format!("{server:?}").contains(&secret));
    assert!(!server.args.iter().any(|a| a.contains(&secret)));
    // A relative command is refused before any adapter sees it.
    assert!(studio_engine::agent_tools::mcp_server_for(&grant, Path::new("studio-mcp")).is_err());
}
