//! Against the real SDK (ignored unless `SDK_BUNDLE` or `SDK_ACTIVE` is set): edits, Apply, the hand-off, Undo and reopening.

use super::*;

/// The SDK the real tests compile against: `SDK_BUNDLE` (an assembled bundle, installed
/// into a temporary SDK home) or `SDK_ACTIVE` (an installed SDK directory).
fn real_sdk(temp: &tempfile::TempDir) -> (PathBuf, studio_sdk::CompatibilityManifest) {
    let path = if let Some(bundle) = std::env::var_os("SDK_BUNDLE") {
        let bundle = PathBuf::from(bundle);
        let manifest = studio_sdk::CompatibilityManifest::from_json_str(
            &fs::read_to_string(bundle.join("compatibility.json")).unwrap(),
        )
        .unwrap();
        let artifacts: Vec<_> = manifest
            .artifacts
            .iter()
            .map(|a| (a.clone(), bundle.join(a.url.trim_start_matches("file://"))))
            .collect();
        let home = std::env::var_os("CANDIDATE_SDK_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| temp.path().join("sdk"));
        studio_sdk::SdkInstaller::new(home)
            .install_from_local_artifacts(&manifest, &artifacts)
            .unwrap()
    } else {
        PathBuf::from(std::env::var_os("SDK_ACTIVE").expect("SDK_ACTIVE or SDK_BUNDLE"))
    };
    let manifest = studio_sdk::CompatibilityManifest::from_json_str(
        &fs::read_to_string(path.join("compatibility.json")).unwrap(),
    )
    .unwrap();
    (path, manifest)
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The cue with another tone: the candidate's audio differs from the base's.
fn retoned(cue: &[u8], hz: f64) -> Vec<u8> {
    let mut wav = cue.to_vec();
    for (i, chunk) in wav[44..].as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let t = i as f64 / 48_000.;
        let left = ((t * hz * std::f64::consts::TAU).sin() * 3000.) as i16;
        chunk[..2].copy_from_slice(&left.to_le_bytes());
    }
    wav
}

/// Real Cargo compiles, the real preview worker, real Apply/Undo through the project
/// folder, and the scripted peer as the agent editing the working copy. Executed
/// explicitly; a skipped run is not evidence:
///
///     SDK_BUNDLE=/tmp/studio-m2-sdk-stage3 cargo test --locked -p fframes-studio \
///         --test agent_workflow -- --ignored --nocapture
#[test]
#[ignore = "requires SDK_BUNDLE or SDK_ACTIVE; real compiler + preview worker; scripted agent"]
fn real_sdk_workflow_edits_applies_hands_off_undoes_and_closes_clean() {
    let sdk_temp = tempfile::tempdir().unwrap();
    let (sdk, manifest) = real_sdk(&sdk_temp);
    let owner = ProcessTreeManager::new();
    let sink = Arc::new(Sink {
        coordinator: Mutex::new(Some(Arc::new(PreviewCoordinator::new(owner.sub_manager())))),
        fail: AtomicBool::new(false),
        adopted: Mutex::new(Vec::new()),
    });
    let w = World::with(Options {
        real: Some((sdk, manifest)),
        handoff: Some(sink.clone()),
        ..Options::default()
    });
    let original_lib = w.source("src/lib.rs");
    let original_cue = fs::read(w.root.join("media/cue.wav")).unwrap();
    let original_revision = w.ctl().lock().state().source().as_str().to_owned();
    let new_cue = retoned(&original_cue, 880.);
    w.submit_plan(&[json!({
        "text": "Retoned the cue.",
        "write": {"src/lib.rs": format!("{original_lib}\n// edited by the scripted agent\n")},
        "write_b64": {"media/cue.wav": base64(&new_cue)},
    })]);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let validation = task.validation.as_ref().expect("validation");
    assert!(validation.passed, "{}", validation.summary);
    assert!(validation.coverage.as_ref().unwrap().rendered_frames > 0);
    assert!(
        validation
            .audio
            .as_ref()
            .is_some_and(|a| !a.silent && a.placement_verified)
    );
    let changes = task.changes.as_ref().unwrap();
    assert!(changes.entries.iter().any(|e| e.path == "media/cue.wav"));
    assert!(changes.entries.iter().any(|e| e.path == "src/lib.rs"));
    assert_eq!(fs::read(w.root.join("media/cue.wav")).unwrap(), new_cue);
    let published = w.ctl().lock().state().source().as_str().to_owned();
    assert_ne!(published, original_revision);
    let coordinator = sink.coordinator.lock().clone().unwrap();
    let ready = wait_ready(&coordinator);
    assert_eq!(
        ready.identity().source_revision,
        published,
        "the handed-off preview is the published revision"
    );
    assert!(ready.audio_source.is_some());
    assert_eq!(
        w.snap().handoff.as_ref().unwrap().state,
        HandoffState::Adopted
    );
    drop(ready);

    // Undo returns to the original bytes through the same validation + handoff.
    w.wf().undo(None).unwrap();
    w.wait("the Undo", |s| s.history.len() == 2);
    assert_eq!(w.source("src/lib.rs"), original_lib);
    assert_eq!(
        fs::read(w.root.join("media/cue.wav")).unwrap(),
        original_cue
    );
    assert_eq!(w.ctl().lock().state().source().as_str(), original_revision);
    let ready = wait_ready(&coordinator);
    assert_eq!(ready.identity().source_revision, original_revision);
    drop(ready);
    drop(coordinator);
    let owned = sink.coordinator.lock().take().unwrap();
    owned.close();
    drop(owned);
    w.wf().close();
    w.assert_clean();
}

/// A real generated starter title is selected from managed-worker frame metadata, frozen in
/// the task, edited by the workflow, validated and applied, then undone and reopened. The ACP
/// peer is intentionally scripted: this is M5 development evidence, never provider evidence.
#[test]
#[ignore = "requires SDK_BUNDLE; real managed compiler/worker and scripted development agent"]
fn selected_title_prompt_apply_undo_and_reopen_round_trips_identity() {
    let sdk_temp = tempfile::tempdir().unwrap();
    let (sdk, manifest) = real_sdk(&sdk_temp);
    let mut world = World::with(Options {
        real: Some((sdk, manifest)),
        starter_project: true,
        ..Options::default()
    });

    let (scope, initial_preview, initial_title) = measured_starter_title_scope(&world);
    world
        .wf()
        .set_displayed_preview_identity(Some(initial_preview.clone()))
        .unwrap();
    let original_source = world.source("src/lib.rs");
    assert_eq!(original_source.matches("Your video starts here").count(), 1);
    let candidate_source =
        original_source.replace("Your video starts here", "A selected title change");
    scope.validate().unwrap();
    world.submit_scoped_plan(
        "Change only the selected title to ‘A selected title change’.",
        scope.clone(),
        &[json!({
            "text": "Updated the selected starter title.",
            "write": {"src/lib.rs": candidate_source},
        })],
    );

    let task = world.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(task.scope.canvas_selection, scope.canvas_selection);
    assert!(
        task.scope
            .prompt_context()
            .contains("Explicit source anchor")
    );
    assert!(task.scope.prompt_context().contains("starter-title"));
    let validation = task.validation.as_ref().expect("real candidate validation");
    assert!(validation.passed, "{}", validation.summary);
    assert!(validation.coverage.as_ref().unwrap().rendered_frames > 0);
    assert_eq!(world.source("src/lib.rs"), candidate_source);
    let applied_revision = world.ctl().lock().state().source().as_str().to_owned();
    assert_ne!(applied_revision, initial_preview.source_revision);

    let prompts = world.evidence("prompts.jsonl");
    assert_eq!(prompts.len(), 1);
    let prompt = prompts[0]["text"].as_str().unwrap();
    assert!(
        prompt.contains(
            "Requested scope: Element scene=intro component=starter-title object=headline"
        ),
        "{prompt}"
    );
    assert!(prompt.contains("scene=intro"), "{prompt}");
    assert!(prompt.contains("component=starter-title"), "{prompt}");
    assert!(
        prompt.contains("marker=Some(\"studio-title-source-anchor\")"),
        "{prompt}"
    );

    world.wf().undo(None).unwrap();
    world.wait("selected title undo", |snapshot| {
        snapshot.history.len() == 2
    });
    assert_eq!(world.source("src/lib.rs"), original_source);
    assert_eq!(
        world.ctl().lock().state().source().as_str(),
        initial_preview.source_revision
    );
    world.restart();
    assert_eq!(world.source("src/lib.rs"), original_source);
    assert_eq!(
        world.ctl().lock().project.inventory.revision.as_str(),
        initial_preview.source_revision
    );
    let (reopened_scope, reopened_preview, reopened_title) = measured_starter_title_scope(&world);
    assert_eq!(
        reopened_preview.source_revision,
        initial_preview.source_revision
    );
    assert_eq!(reopened_title.identity, initial_title.identity);
    assert_eq!(reopened_title.source_anchor, initial_title.source_anchor);
    assert_eq!(reopened_title.style_tokens, initial_title.style_tokens);
    assert_eq!(
        reopened_scope.canvas_selection.unwrap().editor_index_digest,
        scope.canvas_selection.unwrap().editor_index_digest
    );
    assert_ne!(applied_revision, initial_preview.source_revision);
    world.assert_clean();
}

fn measured_starter_title_scope(
    world: &World,
) -> (
    TaskScope,
    PreviewIdentity,
    fframes_studio_protocol::EditorObjectGeometry,
) {
    let project = world.ctl().lock().project.clone();
    let preview = PreviewIdentity {
        project_id: String::from(project.manifest.project_id.clone()),
        open_session: "m5-selected-title-session".into(),
        source_revision: project.inventory.revision.as_str().into(),
        worker_generation: 1,
    };
    let manager = ProcessTreeManager::new();
    let build = studio_engine::services::worker_project::compile_portable_worker(
        &project,
        &world.sdk,
        world.manifest.clone(),
        &world._temp.path().join("builds"),
        &manager,
    )
    .expect("compile the generated starter using the managed SDK");
    let mut worker = studio_engine::services::worker_project::launch_preview_worker(
        build,
        preview.clone(),
        &manager,
    )
    .expect("launch the generated starter preview worker");
    let timeline = worker.timeline().expect("managed starter timeline");
    let frame = worker.frame(0, 17, 1.0).expect("managed title frame");
    let metadata = frame
        .response
        .editor_metadata
        .expect("frame-coupled starter title metadata");
    assert_eq!(metadata.status, EditorFrameStatus::Supported);
    assert_eq!((metadata.frame_index, metadata.seek_serial), (0, 17));
    let title = metadata
        .objects
        .into_iter()
        .find(|object| {
            object.identity.scene_instance_key == "intro"
                && object.identity.component_key == "starter-title"
                && object.identity.object_key == "headline"
                && object.identity.repeat_key == "primary"
        })
        .expect("the registered starter title is present in the actual managed frame");
    let left = title.bounds.x.floor() as u32;
    let top = title.bounds.y.floor() as u32;
    let right = (title.bounds.x + title.bounds.width).ceil() as u32;
    let bottom = (title.bounds.y + title.bounds.height).ceil() as u32;
    let mut scope = TaskScope::from_timeline(&timeline, &TimelineSelection::default()).unwrap();
    scope.canvas_selection = Some(CanvasTaskSelection {
        preview: preview.clone(),
        frame_index: metadata.frame_index,
        seek_serial: metadata.seek_serial,
        editor_index_digest: Some(metadata.editor_index_digest),
        frame_geometry_digest: Some(metadata.frame_geometry_digest),
        video_width: metadata.video_width,
        video_height: metadata.video_height,
        selection: CanvasTaskSelectionKind::Element {
            identity: title.identity.clone(),
            bounds: VideoPixelRect {
                x: left,
                y: top,
                width: right - left,
                height: bottom - top,
            },
            support: EditorGeometrySupport::ApproximateBounds,
            source_anchor: title.source_anchor.clone(),
            style_tokens: title.style_tokens.clone(),
        },
    });
    scope.validate().unwrap();
    drop(worker);
    manager.terminate_all(Duration::from_millis(300));
    assert_eq!(manager.active_count(), 0);
    (scope, preview, title)
}
