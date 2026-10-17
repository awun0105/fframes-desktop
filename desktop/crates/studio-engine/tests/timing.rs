//! Timeline edits of an open project: split, trim and move, gestures, undo and redo, the
//! write of `timeline.json` and how it meets outside writes, crashes and agent tasks.
use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use fframes_studio_protocol::{
    PREVIEW_CONTRACT_VERSION, PreviewClipTiming, PreviewEnvelope, PreviewSceneInfo,
    PreviewTimelineResponse,
};
use studio_bootstrap::WriterOwnership;
use studio_engine::{
    Boundary, Controller, Fault, JobKind, JobResult, JobState, QuiescenceEvidence, TaskState,
    TimingEditing, TurnCompletion, WriterObservation,
    app_paths::AppPaths,
    build_materialization::sdk_pin,
    preview_identity,
    timing::{TimelineCommand, TimelineSnapshot, TimingDoc, TrimEdge, file::WriteStep},
};
use studio_project::revision::{FileKind, TIMING_FILE};
use studio_sdk::CompatibilityManifest;

struct Fixture {
    _temp: tempfile::TempDir,
    root: std::path::PathBuf,
    paths: AppPaths,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("video");
    let paths = AppPaths::new(temp.path().join("history")).unwrap();
    create(&root);
    Fixture {
        _temp: temp,
        root,
        paths,
    }
}

fn create(root: &std::path::Path) {
    let compatibility = CompatibilityManifest::default_linux_x64();
    studio_project::create(
        root,
        "Video",
        sdk_pin(&compatibility),
        &compatibility.fframes_version,
        &compatibility.runtime_version,
    )
    .unwrap();
}

/// Scenes `intro` (30 frames), `main` (60) and `outro` (30), in code order.
const SCENES: [(&str, usize); 3] = [("intro", 30), ("main", 60), ("outro", 30)];

/// A finished build of the current source whose preview reported [`SCENES`].
fn built(controller: &mut Controller) {
    let tag = controller.begin_job(JobKind::Build).unwrap();
    controller
        .complete(&tag, JobResult::Built(tag.base_source.clone()))
        .unwrap();
    let identity = preview_identity(&tag);
    let mut start = 0;
    let scenes = SCENES
        .iter()
        .enumerate()
        .map(|(index, (key, frames))| {
            let scene = PreviewSceneInfo {
                instance_id: format!("scene-{index}"),
                editor_instance_key: Some((*key).into()),
                index,
                name: (*key).into(),
                full_name: (*key).into(),
                start_frame: start,
                end_frame: start + frames,
                start_seconds: start as f32 / 30.,
                end_seconds: (start + frames) as f32 / 30.,
                clip: Some(PreviewClipTiming {
                    timing_key: (*key).into(),
                    clip_index: 0,
                    trim_start_frames: 0,
                    trim_end_frames: 0,
                    scene_index: Some(index),
                    scene_frames: Some(*frames),
                }),
            };
            start += frames;
            scene
        })
        .collect();
    let timeline = PreviewTimelineResponse {
        envelope: PreviewEnvelope {
            contract_version: PREVIEW_CONTRACT_VERSION,
            identity,
            request_id: 1,
        },
        fps: 30,
        width: 1920,
        height: 1080,
        total_frames: start,
        duration_seconds: start as f32 / 30.,
        scenes,
        audio_tracks: vec![],
        timing_generation: 0,
    };
    controller.learn_timeline(&tag, &timeline);
}

fn opened(f: &Fixture) -> Controller {
    let mut controller = Controller::open(&f.root, &f.paths).unwrap();
    built(&mut controller);
    assert_eq!(
        controller.timeline_snapshot().editing,
        TimingEditing::Enabled
    );
    controller
}

/// The clips as `(key, played frames)`.
fn clips(snapshot: &TimelineSnapshot) -> Vec<(String, usize)> {
    snapshot
        .clips
        .iter()
        .map(|clip| (clip.key.clone(), clip.frames()))
        .collect()
}

fn live(f: &Fixture) -> TimingDoc {
    TimingDoc::from_json(&fs::read_to_string(f.root.join(TIMING_FILE)).unwrap()).unwrap()
}

fn outside_doc(trim: usize) -> String {
    format!(
        "{{\"version\":1,\"clips\":[{{\"key\":\"outro\",\"trim_start_frames\":{trim},\"trim_end_frames\":0}}]}}\n"
    )
}

fn diagnostic(result: Result<(), studio_engine::EngineError>) -> String {
    result.unwrap_err().to_string()
}

#[test]
fn timing_edits_split_trim_and_move_and_write_each_one() {
    let f = fixture();
    let mut c = opened(&f);
    let snapshot = c.timeline_snapshot();
    assert_eq!(
        clips(&snapshot),
        [
            ("intro".into(), 30),
            ("main".into(), 60),
            ("outro".into(), 30)
        ]
    );
    assert_eq!(
        snapshot.timing_json, None,
        "no document plays the code order"
    );

    c.timeline_command(TimelineCommand::Split { clip: 1, at: 20 })
        .unwrap();
    assert_eq!(
        clips(&c.timeline_snapshot()),
        [
            ("intro".into(), 30),
            ("main".into(), 20),
            ("main".into(), 40),
            ("outro".into(), 30)
        ]
    );
    assert_eq!(live(&f).clips.len(), 4);

    c.timeline_command(TimelineCommand::Trim {
        clip: 0,
        edge: TrimEdge::Start,
        frames: 10,
    })
    .unwrap();
    assert_eq!(live(&f).clips[0].trim_start_frames, 10);
    assert_eq!(clips(&c.timeline_snapshot())[0], ("intro".into(), 20));

    c.timeline_command(TimelineCommand::Move { clip: 3, to: 0 })
        .unwrap();
    let keys: Vec<_> = live(&f).clips.into_iter().map(|clip| clip.key).collect();
    assert_eq!(keys, ["outro", "intro", "main", "main"]);

    let snapshot = c.timeline_snapshot();
    assert!(snapshot.can_undo && !snapshot.can_redo);
    assert_eq!(
        snapshot.timing_json.as_deref(),
        Some(
            fs::read_to_string(f.root.join(TIMING_FILE))
                .unwrap()
                .as_str()
        )
    );
    c.timeline_command(TimelineCommand::Undo).unwrap();
    assert_eq!(live(&f).clips[0].key, "intro");
    c.timeline_command(TimelineCommand::Redo).unwrap();
    assert_eq!(live(&f).clips[0].key, "outro");
}

#[test]
fn timing_edit_after_a_build_still_exports_and_plays_the_edited_document() {
    let f = fixture();
    let mut c = opened(&f);
    c.freeze_export_source().unwrap();
    assert_eq!(
        c.timing_for_export(),
        Ok(None),
        "no clips export the code order"
    );

    c.timeline_command(TimelineCommand::Trim {
        clip: 2,
        edge: TrimEdge::End,
        frames: 5,
    })
    .unwrap();
    assert_ne!(c.state().source(), c.state().accepted());
    let frozen = c
        .freeze_export_source()
        .expect("a timing edit keeps the built code exportable");
    assert_eq!(frozen.revision(), c.state().accepted());
    assert_eq!(frozen.live_source_warning(), None);
    let exported = TimingDoc::from_json(&c.timing_for_export().unwrap().unwrap()).unwrap();
    assert_eq!(exported, live(&f));
    assert_eq!(exported.clips[2].trim_end_frames, 5);
}

#[test]
fn timing_export_refuses_built_code_that_was_never_saved_or_an_invalid_document() {
    let f = fixture();
    let mut c = opened(&f);
    fs::write(f.root.join("src/lib.rs"), "// unsaved code\n").unwrap();
    built(&mut c);
    let error = c.freeze_export_source().err().expect("refused").to_string();
    assert!(error.contains("unsaved, unvalidated source"), "{error}");

    let f = fixture();
    fs::write(f.root.join(TIMING_FILE), "{ not json").unwrap();
    let c = Controller::open(&f.root, &f.paths).unwrap();
    let error = c.timing_for_export().unwrap_err();
    assert!(error.contains(TIMING_FILE), "{error}");
}

#[test]
fn timing_gesture_of_sixty_steps_is_one_undo_step_one_write_and_no_build() {
    let f = fixture();
    let mut c = opened(&f);
    let writes = Arc::new(AtomicUsize::new(0));
    let counted = writes.clone();
    c.set_transaction_hooks(Arc::new(move |boundary: &Boundary| {
        if matches!(boundary, Boundary::TimingWrite(WriteStep::Staged)) {
            counted.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }));
    let job = c.state().job().clone();
    let generation = c.state().generation();
    let build = c.state().build_source().clone();
    let source = c.state().source().clone();
    let first = c.timeline_snapshot().generation;

    c.timeline_command(TimelineCommand::BeginGesture).unwrap();
    for step in 0..60 {
        c.timeline_command(TimelineCommand::Trim {
            clip: 2,
            edge: TrimEdge::End,
            frames: step % 20 + 1,
        })
        .unwrap();
    }
    let dragging = c.timeline_snapshot();
    assert!(dragging.generation > first, "every step previews live");
    assert!(!dragging.can_undo, "nothing to undo while dragging");
    assert!(
        !f.root.join(TIMING_FILE).exists(),
        "nothing written mid-drag"
    );
    c.timeline_command(TimelineCommand::EndGesture).unwrap();

    assert_eq!(writes.load(Ordering::SeqCst), 1);
    assert_eq!(live(&f).clips[2].trim_end_frames, 20);
    // The source changed by the timing document alone: the build stands.
    assert_ne!(c.state().source(), &source);
    assert_eq!(c.state().build_source(), &build);
    assert_eq!(c.state().built(), Some(&build));
    assert_eq!(c.state().generation(), generation);
    assert_eq!(c.state().job(), &job, "no build job was queued");
    assert!(matches!(c.state().job(), JobState::Succeeded(_)));
    // Staged and displaced copies stay out of the source.
    let inventory = studio_project::open(&f.root).unwrap().inventory;
    assert!(
        inventory
            .files
            .iter()
            .all(|file| !file.path.as_str().starts_with(".fframes")),
        "{:?}",
        inventory.files
    );
    assert_eq!(
        inventory.timing_file().map(|file| file.path.as_str()),
        Some(TIMING_FILE)
    );

    c.timeline_command(TimelineCommand::Undo).unwrap();
    assert_eq!(live(&f), TimingDoc::default());
    assert!(!c.timeline_snapshot().can_undo, "the drag was one step");
    assert_eq!(writes.load(Ordering::SeqCst), 2);
}

#[test]
fn timing_crash_between_displace_and_publish_is_recovered_on_open() {
    let f = fixture();
    let mut c = opened(&f);
    c.timeline_command(TimelineCommand::Move { clip: 2, to: 0 })
        .unwrap();
    c.set_transaction_hooks(Arc::new(|boundary: &Boundary| {
        if matches!(boundary, Boundary::TimingWrite(WriteStep::Displaced)) {
            Err(Fault::Crash)
        } else {
            Ok(())
        }
    }));
    assert!(
        c.timeline_command(TimelineCommand::Split { clip: 1, at: 5 })
            .is_err()
    );
    assert!(
        !f.root.join(TIMING_FILE).exists(),
        "the crash left no live file"
    );
    drop(c);

    let mut reopened = Controller::open(&f.root, &f.paths).unwrap();
    let doc = live(&f);
    assert_eq!(doc.clips.len(), 4, "the interrupted write was completed");
    assert_eq!(doc.clips[0].key, "outro");
    assert!(
        reopened
            .recovery_notice
            .as_deref()
            .is_some_and(|notice| notice.contains("interrupted save")),
        "{:?}",
        reopened.recovery_notice
    );
    built(&mut reopened);
    assert_eq!(clips(&reopened.timeline_snapshot()).len(), 4);
}

#[test]
fn timing_outside_write_during_a_drag_is_kept() {
    let f = fixture();
    let mut c = opened(&f);
    c.timeline_command(TimelineCommand::BeginGesture).unwrap();
    c.timeline_command(TimelineCommand::Trim {
        clip: 0,
        edge: TrimEdge::Start,
        frames: 4,
    })
    .unwrap();
    fs::write(f.root.join(TIMING_FILE), outside_doc(7)).unwrap();
    let error = diagnostic(c.timeline_command(TimelineCommand::EndGesture));
    assert!(error.contains("changed outside"), "{error}");
    assert_eq!(
        fs::read_to_string(f.root.join(TIMING_FILE)).unwrap(),
        outside_doc(7)
    );
    let snapshot = c.timeline_snapshot();
    assert_eq!(
        snapshot
            .timing_json
            .map(|json| TimingDoc::from_json(&json).unwrap()),
        Some(TimingDoc::from_json(&outside_doc(7)).unwrap()),
        "the session plays the outside version"
    );
    assert!(snapshot.notice.is_some() && !snapshot.can_undo);

    // Seen by a rescan in the middle of the drag instead: the drag is abandoned.
    c.timeline_command(TimelineCommand::BeginGesture).unwrap();
    fs::write(f.root.join(TIMING_FILE), outside_doc(9)).unwrap();
    c.reconcile().unwrap();
    let error = diagnostic(c.timeline_command(TimelineCommand::Trim {
        clip: 0,
        edge: TrimEdge::Start,
        frames: 2,
    }));
    assert!(error.contains("changed outside"), "{error}");
    let _ = c.timeline_command(TimelineCommand::EndGesture);
    assert_eq!(
        fs::read_to_string(f.root.join(TIMING_FILE)).unwrap(),
        outside_doc(9)
    );
}

#[test]
fn timing_undo_after_an_outside_change_is_refused() {
    let f = fixture();
    let mut c = opened(&f);
    c.timeline_command(TimelineCommand::Move { clip: 0, to: 2 })
        .unwrap();
    assert!(c.timeline_snapshot().can_undo);
    fs::write(f.root.join(TIMING_FILE), outside_doc(3)).unwrap();
    c.reconcile().unwrap();
    let snapshot = c.timeline_snapshot();
    assert!(!snapshot.can_undo);
    assert!(
        snapshot
            .notice
            .as_deref()
            .is_some_and(|notice| notice.contains("changed outside"))
    );
    let error = diagnostic(c.timeline_command(TimelineCommand::Undo));
    assert!(error.contains("changed outside"), "{error}");
    assert_eq!(
        fs::read_to_string(f.root.join(TIMING_FILE)).unwrap(),
        outside_doc(3)
    );
}

#[test]
fn timing_history_is_cleared_by_a_project_switch() {
    let f = fixture();
    let mut c = opened(&f);
    c.timeline_command(TimelineCommand::Split { clip: 0, at: 10 })
        .unwrap();
    assert!(c.timeline_snapshot().can_undo);
    c.close().unwrap();
    drop(c);

    let other = f.root.with_file_name("other");
    create(&other);
    let mut switched = Controller::open(&other, &f.paths).unwrap();
    built(&mut switched);
    let snapshot = switched.timeline_snapshot();
    assert!(!snapshot.can_undo && snapshot.timing_json.is_none());
    switched.close().unwrap();
    drop(switched);

    let mut back = opened(&f);
    let snapshot = back.timeline_snapshot();
    assert!(!snapshot.can_undo, "history does not outlive the session");
    assert_eq!(clips(&snapshot).len(), 4, "the written document stays");
}

#[test]
fn timing_edits_are_refused_while_an_agent_task_runs() {
    let f = fixture();
    let mut c = opened(&f);
    let context = c.begin_agent_task("tighten the intro").unwrap();
    let TimingEditing::Disabled(reason) = c.timeline_snapshot().editing else {
        panic!("editing is disabled during a task");
    };
    assert!(reason.contains("agent task"), "{reason}");
    for command in [
        TimelineCommand::Split { clip: 0, at: 5 },
        TimelineCommand::BeginGesture,
        TimelineCommand::Undo,
    ] {
        let error = diagnostic(c.timeline_command(command));
        assert!(error.contains("agent task"), "{error}");
    }
    assert!(!f.root.join(TIMING_FILE).exists());
    c.finish_agent_task(&context.identity, TaskState::Cancelled, "user stop")
        .unwrap();
    built(&mut c);
    c.timeline_command(TimelineCommand::Split { clip: 0, at: 5 })
        .unwrap();
}

#[test]
fn timing_only_agent_candidate_is_captured() {
    let f = fixture();
    let mut c = opened(&f);
    let context = c.begin_agent_task("play the outro first").unwrap();
    fs::write(context.draft.join(TIMING_FILE), outside_doc(0)).unwrap();
    let id = &context.identity;
    let qualified = WriterOwnership::ProcessGroupContained {
        qualification: "test-adapter".into(),
    };
    c.agent_writer_started(id, qualified.clone()).unwrap();
    c.agent_task_transition(id, TaskState::Editing).unwrap();
    c.agent_task_transition(id, TaskState::Quiescing).unwrap();
    let task = c.agent_task().unwrap();
    let evidence = QuiescenceEvidence {
        identity: id.clone(),
        provider_session: task.provider_session().map(str::to_owned),
        writer: task.writer_generation().unwrap(),
        completion: TurnCompletion::EndTurn,
        cancel_requested: false,
        unresolved_requests: 0,
        observed: WriterObservation {
            ownership: qualified,
            escaped_pids: vec![],
        },
    };
    let ticket = c.agent_complete_quiescence(id, &evidence).unwrap();
    let captured = c.agent_capture_candidate(ticket).unwrap();
    let changes = &captured.changes().entries;
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(changes[0].path.as_str(), TIMING_FILE);
    assert_eq!(changes[0].kind, FileKind::Timing);
    assert_eq!(
        studio_engine::candidate_validation::broadening(captured.changes()),
        Some(studio_engine::candidate_validation::BroadenReason::SharedSourceChanged)
    );
}
