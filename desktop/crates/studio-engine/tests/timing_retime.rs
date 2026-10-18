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

#[test]
#[ignore = "requires SDK_ACTIVE or SDK_BUNDLE, FFMPEG_DIR and ODIFF_BIN; real SDK pixel parity"]
fn edited_sdk_export_matches_preview_and_preencode_at_three_times() {
    use fframes_studio_protocol::export::{ExportCodec, ExportSettings};
    use std::process::Command;
    use studio_engine::services::{
        export_service::{ExportControl, ExportProgress, ExportRequest, export_video},
        worker_project::{launch_preview_worker, shared_build_service, worker_binary},
    };

    let temp = tempfile::tempdir().unwrap();
    let (sdk, manifest) = sdk(&temp);
    let root = temp.path().join("video");
    studio_project::create(
        &root,
        "Export pixel parity",
        sdk_pin(&manifest),
        &manifest.fframes_version,
        &manifest.runtime_version,
    )
    .unwrap();
    let worker_path = root.join("src/bin/studio_worker.rs");
    let worker_source = fs::read_to_string(&worker_path).unwrap();
    let marker = "    // One-shot export:";
    assert!(worker_source.contains(marker));
    let worker_source = worker_source.replacen(marker,
        "    if args.iter().any(|arg| arg == \"--qualify-preencode\") {\n        return qualify_preencode(&video, &options, &args);\n    }\n    // One-shot export:", 1);
    fs::write(
        &worker_path,
        format!(
            "{worker_source}\n{}",
            include_str!("fixtures/timing_export_reference.rs")
        ),
    )
    .unwrap();
    let paths = AppPaths::new(temp.path().join("history")).unwrap();
    let mut controller = Controller::open(&root, &paths).unwrap();
    let preview = PreviewCoordinator::new(controller.processes.sub_manager());
    preview.set_timing(timing(&mut controller));
    preview.build(spec(&mut controller, &sdk, &manifest, &paths));
    let ready = wait_ready(&preview, &mut controller);
    assert!(preview.commit(ready.identity().clone(), ready.seek_serial));
    controller.learn_timeline(ready.tag(), &ready.timeline);
    controller
        .timeline_command(TimelineCommand::Move { clip: 2, to: 0 })
        .unwrap();
    controller
        .timeline_command(TimelineCommand::Trim {
            clip: 2,
            edge: studio_engine::timing::TrimEdge::End,
            frames: ready.timeline.fps,
        })
        .unwrap();
    preview.set_timing(timing(&mut controller));
    let ready = wait_retimed(&preview).unwrap();
    let build = ready.audio_source.as_ref().unwrap().build().clone();
    let out = std::env::var_os("EXPORT_PARITY_EVIDENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("evidence"));
    fs::create_dir_all(&out).unwrap();
    let document = controller.timing_for_export().unwrap().unwrap();
    let timing_path = out.join("timeline.json");
    fs::write(&timing_path, document).unwrap();
    let positions = [
        0,
        ready.timeline.total_frames / 2,
        ready.timeline.total_frames - 1,
    ];
    let mut command = Command::new(worker_binary(&build));
    command
        .current_dir(build.manifest.parent().unwrap())
        .args(["--qualify-preencode", "--timing-file"])
        .arg(&timing_path)
        .arg("--frames")
        .arg(positions.map(|n| n.to_string()).join(","))
        .arg("--output")
        .arg(&out);
    build
        .environment
        .build_child_environment()
        .apply_to_command(&mut command);
    let reference = command.output().unwrap();
    assert!(
        reference.status.success(),
        "{}",
        String::from_utf8_lossy(&reference.stderr)
    );

    let destination = out.join("edited.mp4");
    let mut completed = false;
    export_video(
        ExportRequest {
            source: controller.freeze_export_source().unwrap(),
            sdk: sdk.clone(),
            builds: paths.builds(),
            destination: destination.clone(),
            settings: ExportSettings {
                scale: 0.5,
                codec: ExportCodec::H264,
                ..Default::default()
            },
            timing_json: controller.timing_for_export().unwrap(),
        },
        shared_build_service(),
        controller.processes.sub_manager(),
        Arc::new(ExportControl::default()),
        |event| {
            if matches!(event, ExportProgress::Complete { .. }) {
                completed = true;
            }
            if let ExportProgress::Failed(error) = event {
                panic!("{error}");
            }
        },
    )
    .unwrap();
    assert!(completed);
    let manager = controller.processes.sub_manager();
    let mut worker = launch_preview_worker(build, ready.identity().clone(), &manager).unwrap();
    worker.negotiate().unwrap();
    let applied = worker
        .set_timing(
            controller.timing_for_export().unwrap(),
            ready.timeline.timing_generation + 1,
        )
        .unwrap();
    assert_eq!(applied.total_frames, ready.timeline.total_frames);
    let odiff = PathBuf::from(std::env::var_os("ODIFF_BIN").expect("ODIFF_BIN"));
    let ffmpeg = PathBuf::from(std::env::var_os("FFMPEG_DIR").expect("FFMPEG_DIR"))
        .join("bin")
        .join(format!("ffmpeg{}", std::env::consts::EXE_SUFFIX));
    for position in positions {
        let frame = worker.frame(position, position as u64 + 1, 0.5).unwrap();
        assert_eq!(
            fs::read(out.join(format!("preencode-{position}.rgba"))).unwrap(),
            frame.pixels,
            "preview and pre-encode CPU renderer differ at {position}"
        );
        let expected = out.join(format!("preview-{position}.png"));
        image::save_buffer(
            &expected,
            &frame.pixels,
            frame.response.header.width,
            frame.response.header.height,
            image::ColorType::Rgba8,
        )
        .unwrap();
        let decoded = out.join(format!("decoded-{position}.png"));
        let decode = Command::new(&ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(&destination)
            .args([
                "-vf",
                &format!("select=eq(n\\,{position})"),
                "-frames:v",
                "1",
            ])
            .arg(&decoded)
            .output()
            .unwrap();
        assert!(
            decode.status.success(),
            "{}",
            String::from_utf8_lossy(&decode.stderr)
        );
        let compare = |actual: &std::path::Path| {
            Command::new(&odiff)
                .arg(&expected)
                .arg(actual)
                .arg(out.join(format!("diff-{position}.png")))
                .args(["-t", "0.4"])
                .output()
                .unwrap()
        };
        let comparison = compare(&decoded);
        assert!(
            comparison.status.success(),
            "odiff at {position}: {} {}",
            String::from_utf8_lossy(&comparison.stdout),
            String::from_utf8_lossy(&comparison.stderr)
        );
        if position == 0 {
            let wrong = out.join("negative-control.png");
            let mut rgba = image::RgbaImage::from_raw(
                frame.response.header.width,
                frame.response.header.height,
                frame.pixels.clone(),
            )
            .unwrap();
            for pixel in rgba.pixels_mut() {
                *pixel = image::Rgba([255, 0, 255, 255]);
            }
            rgba.save(&wrong).unwrap();
            assert!(
                !compare(&wrong).status.success(),
                "odiff must reject a different image"
            );
        }
    }
    fs::write(out.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "positions": positions, "codec": "h264", "preencode_pixel_identity": "pass", "decoded_odiff_threshold": 0.4,
        "decoded_odiff": "pass", "negative_control": "refused", "sdk_digest": manifest.digest(),
    })).unwrap()).unwrap();
    worker.shutdown();
    manager.shutdown(Duration::ZERO);
    preview.close();
}
