//! A timeline edit played by a real preview worker: the displayed worker is laid out
//! with the session's timing document without a rebuild, and a later build starts with it.
use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use studio_engine::{
    Controller, JobKind, JobResult, PreviewState, ReadyPreview,
    app_paths::AppPaths,
    build_materialization::sdk_pin,
    services::preview_coordinator::{BuildSpec, PreviewCoordinator, PreviewTiming, SeekIntent},
    timing::TimelineCommand,
};
use studio_sdk::{CompatibilityManifest, SdkInstaller};

fn sdk(temp: &tempfile::TempDir) -> (PathBuf, CompatibilityManifest) {
    let sdk = if let Some(bundle) = std::env::var_os("SDK_BUNDLE") {
        let bundle = PathBuf::from(bundle);
        let manifest = CompatibilityManifest::from_json_str(
            &fs::read_to_string(bundle.join("compatibility.json")).unwrap(),
        )
        .unwrap();
        let artifacts: Vec<_> = manifest
            .artifacts
            .iter()
            .map(|a| (a.clone(), bundle.join(a.url.trim_start_matches("file://"))))
            .collect();
        SdkInstaller::new(temp.path().join("sdk"))
            .install_from_local_artifacts(&manifest, &artifacts)
            .unwrap()
    } else {
        PathBuf::from(std::env::var_os("SDK_ACTIVE").expect("SDK_BUNDLE or SDK_ACTIVE"))
    };
    let manifest = CompatibilityManifest::from_json_str(
        &fs::read_to_string(sdk.join("compatibility.json")).unwrap(),
    )
    .unwrap();
    (sdk, manifest)
}

fn spec(
    c: &mut Controller,
    sdk: &std::path::Path,
    m: &CompatibilityManifest,
    paths: &AppPaths,
) -> BuildSpec {
    let tag = c.begin_job(JobKind::Build).unwrap();
    BuildSpec {
        project: c.project.clone(),
        sdk: sdk.into(),
        compatibility: m.clone(),
        builds: paths.builds(),
        tag,
        compiler: c.operation_processes(),
        worker: c.processes.sub_manager(),
        service: studio_engine::services::worker_project::shared_build_service(),
    }
}

fn wait_ready(p: &PreviewCoordinator, c: &mut Controller) -> Arc<ReadyPreview> {
    let deadline = Instant::now() + Duration::from_secs(600);
    loop {
        let e = p.events();
        if let Some(tag) = e.compiled {
            c.complete(&tag, JobResult::Built(tag.base_source.clone()))
                .unwrap();
        }
        if let Some((_, error)) = e.error {
            panic!("candidate failed: {error}");
        }
        if let Some(r) = e.ready {
            c.reconcile().unwrap();
            return r;
        }
        assert!(Instant::now() < deadline, "preview readiness deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_retimed(p: &PreviewCoordinator) -> Result<Arc<ReadyPreview>, String> {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(retimed) = p.events().retimed {
            return retimed.map_err(|(_, error)| error);
        }
        assert!(Instant::now() < deadline, "retime deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn keys(r: &ReadyPreview) -> Vec<String> {
    r.timeline
        .scenes
        .iter()
        .map(|scene| scene.clip.as_ref().unwrap().timing_key.clone())
        .collect()
}

fn timing(c: &mut Controller) -> PreviewTiming {
    let (json, generation) = c.timing_for_preview();
    PreviewTiming { json, generation }
}

#[test]
#[ignore = "requires a timeline-ready SDK_BUNDLE or SDK_ACTIVE; real worker qualification"]
fn real_worker_plays_timeline_edits_without_a_rebuild() {
    let temp = tempfile::tempdir().unwrap();
    let (sdk, m) = sdk(&temp);
    let root = temp.path().join("video");
    studio_project::create(
        &root,
        "Retime",
        sdk_pin(&m),
        &m.fframes_version,
        &m.runtime_version,
    )
    .unwrap();
    let paths = AppPaths::new(temp.path().join("history")).unwrap();
    let mut c = Controller::open(&root, &paths).unwrap();
    let p = PreviewCoordinator::new(c.processes.sub_manager());
    let mut state = PreviewState::default();
    p.set_timing(timing(&mut c));
    let first = spec(&mut c, &sdk, &m, &paths);
    state.begin(first.tag.clone());
    p.build(first);
    let r = wait_ready(&p, &mut c);
    assert!(p.commit(r.identity().clone(), r.seek_serial));
    state.install(&r, c.state()).unwrap();
    c.learn_timeline(r.tag(), &r.timeline);
    assert_eq!(keys(&r), ["intro", "story", "outro"]);
    let job = c.state().job().clone();

    // Outro first, then the story cut by a second: one retime of the displayed worker.
    c.timeline_command(TimelineCommand::Move { clip: 2, to: 0 })
        .unwrap();
    c.timeline_command(TimelineCommand::Trim {
        clip: 2,
        edge: studio_engine::timing::TrimEdge::End,
        frames: r.timeline.fps,
    })
    .unwrap();
    let edited = timing(&mut c);
    p.set_timing(edited.clone());
    let retimed = wait_retimed(&p).unwrap();
    assert_eq!(retimed.identity(), r.identity(), "the same worker");
    assert_eq!(retimed.timeline.timing_generation, edited.generation);
    assert_eq!(keys(&retimed), ["outro", "intro", "story"]);
    assert_eq!(
        retimed.timeline.total_frames,
        r.timeline.total_frames - r.timeline.fps
    );
    assert!(retimed.audio_source.is_some());
    state.retime(&retimed).unwrap();
    assert_eq!(c.state().job(), &job, "no build ran");

    // The retimed worker serves frames of the new layout.
    let last = retimed.timeline.total_frames - 1;
    let serial = state.seek(last, state.scale()).unwrap();
    p.seek(SeekIntent {
        identity: retimed.identity().clone(),
        serial,
        position: last,
        scale: state.scale(),
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(frame) = p.events().frame
            && state.accepts_frame(&frame)
        {
            assert_eq!(frame.response.frame_index, last);
            break;
        }
        assert!(Instant::now() < deadline, "frame after retime");
        std::thread::sleep(Duration::from_millis(10));
    }

    // A document the worker rejects leaves the displayed layout as it was, and the same
    // worker keeps serving frames.
    p.set_timing(PreviewTiming {
        json: Some(
            r#"{"version":1,"clips":[{"key":"intro","trim_start_frames":100000,"trim_end_frames":0}]}"#
                .into(),
        ),
        generation: edited.generation + 100,
    });
    assert!(wait_retimed(&p).is_err());
    // Seek before sending another document or rebuilding: rejection must leave the
    // existing connection, frame pump and displayed layout usable.
    let scale = 1. / 49.;
    let serial = state.seek(1, scale).unwrap();
    p.seek(SeekIntent {
        identity: retimed.identity().clone(),
        serial,
        position: 1,
        scale,
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let events = p.events();
        assert!(events.error.is_none(), "{:?}", events.error);
        if let Some(frame) = events.frame
            && state.accepts_frame(&frame)
        {
            assert_eq!(frame.response.frame_index, 1);
            assert_eq!(frame.response.scale, scale);
            break;
        }
        assert!(Instant::now() < deadline, "frame after rejected timing");
        std::thread::sleep(Duration::from_millis(10));
    }
    p.set_timing(edited.clone());

    // A new build of the same source starts with the session's document.
    let again = spec(&mut c, &sdk, &m, &paths);
    state.begin(again.tag.clone());
    p.build(again);
    let rebuilt = wait_ready(&p, &mut c);
    assert_eq!(rebuilt.timeline.timing_generation, edited.generation);
    assert_eq!(keys(&rebuilt), ["outro", "intro", "story"]);
    p.close();
}

#[test]
#[ignore = "requires SDK_BUNDLE or SDK_ACTIVE; real frames and thumbnails of the hand-check layout"]
fn real_worker_builds_split_template_at_fractional_scales() {
    use studio_engine::services::thumbnail_cache::ThumbnailKey;

    let temp = tempfile::tempdir().unwrap();
    let (sdk, manifest) = sdk(&temp);
    let root = temp.path().join("video");
    studio_project::create(
        &root,
        "Split template",
        sdk_pin(&manifest),
        &manifest.fframes_version,
        &manifest.runtime_version,
    )
    .unwrap();
    // The six-clip layout from the hand-check session, including the 29-frame story.
    fs::write(
        root.join("timeline.json"),
        r#"{"version":1,"clips":[
        {"key":"intro","trim_start_frames":65,"trim_end_frames":0},
        {"key":"intro","trim_start_frames":0,"trim_end_frames":25},
        {"key":"intro","trim_start_frames":76,"trim_end_frames":0},
        {"key":"story","trim_start_frames":0,"trim_end_frames":91},
        {"key":"story","trim_start_frames":29,"trim_end_frames":0},
        {"key":"outro","trim_start_frames":0,"trim_end_frames":0}
    ]}"#,
    )
    .unwrap();
    let paths = AppPaths::new(temp.path().join("history")).unwrap();
    let mut controller = Controller::open(&root, &paths).unwrap();
    let preview = PreviewCoordinator::new(controller.processes.sub_manager());
    preview.set_timing(timing(&mut controller));
    let build = spec(&mut controller, &sdk, &manifest, &paths);
    let mut state = PreviewState::default();
    state.begin(build.tag.clone());
    preview.build(build);
    let ready = wait_ready(&preview, &mut controller);
    assert_eq!(ready.timeline.scenes.len(), 6);
    assert_eq!(ready.timeline.total_frames, 314);
    assert!(preview.commit(ready.identity().clone(), ready.seek_serial));
    state.install(&ready, controller.state()).unwrap();

    for (index, scene) in ready.timeline.scenes.iter().enumerate() {
        let scale = [19. / 1920., 1. / 49., 0.37, 0.58][index % 4];
        let position = scene.start_frame;
        let serial = state.seek(position, scale).unwrap();
        preview.seek(SeekIntent {
            identity: ready.identity().clone(),
            serial,
            position,
            scale,
        });
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let events = preview.events();
            assert!(events.error.is_none(), "{:?}", events.error);
            if let Some(frame) = events.frame
                && state.accepts_frame(&frame)
            {
                assert_eq!(frame.response.frame_index, position);
                assert_eq!(frame.response.scale, scale);
                break;
            }
            assert!(Instant::now() < deadline, "frame for clip {index}");
            std::thread::sleep(Duration::from_millis(10));
        }
        let key = ThumbnailKey::new(
            ready.identity().clone(),
            ready.timeline.timing_generation,
            1. / 49.,
            position,
        );
        preview.thumbnails(vec![key.clone()]);
        loop {
            let events = preview.events();
            assert!(events.error.is_none(), "{:?}", events.error);
            if let Some((actual, frame)) = events.thumbnail {
                assert_eq!(actual, key);
                assert_eq!(frame.response.frame_index, position);
                break;
            }
            assert!(Instant::now() < deadline, "thumbnail for clip {index}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    preview.close();
}
