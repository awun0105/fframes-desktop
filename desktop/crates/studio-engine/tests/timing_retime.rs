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
#[ignore = "requires a timeline-ready SDK_BUNDLE; explicitly run for real worker qualification"]
fn real_worker_plays_timeline_edits_without_a_rebuild() {
    let bundle = PathBuf::from(std::env::var_os("SDK_BUNDLE").expect("SDK_BUNDLE"));
    let m = CompatibilityManifest::from_json_str(
        &fs::read_to_string(bundle.join("compatibility.json")).unwrap(),
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let artifacts: Vec<_> = m
        .artifacts
        .iter()
        .map(|a| (a.clone(), bundle.join(a.url.trim_start_matches("file://"))))
        .collect();
    let sdk = SdkInstaller::new(temp.path().join("sdk"))
        .install_from_local_artifacts(&m, &artifacts)
        .unwrap();
    let root = temp.path().join("video");
    studio_project::create(
        &root,
        "Retime",
        sdk_pin(&m),
        &m.fframes_version,
        m.studio_runtime_version(),
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

    // A document the worker rejects leaves the displayed layout as it was.
    p.set_timing(PreviewTiming {
        json: Some(
            r#"{"version":1,"clips":[{"key":"intro","trim_start_frames":100000,"trim_end_frames":0}]}"#
                .into(),
        ),
        generation: edited.generation + 100,
    });
    assert!(wait_retimed(&p).is_err());
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
