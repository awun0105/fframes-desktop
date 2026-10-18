//! Install authorization of a staged preview of just-published bytes: the explicit
//! candidate-to-published relationship, never the task base.

#[path = "support/tx_fixture.rs"]
mod tx_fixture;

use fframes_studio_protocol::*;
use std::fs;
use studio_engine::*;
use tx_fixture::*;

fn envelope(t: &OperationTag, request_id: u64) -> PreviewEnvelope {
    PreviewEnvelope {
        contract_version: PREVIEW_CONTRACT_VERSION,
        identity: preview_identity(t),
        request_id,
    }
}
fn frame(t: &OperationTag, position: usize, serial: u64) -> PreviewFrame {
    PreviewFrame {
        response: ScaledFrameResponse {
            envelope: envelope(t, 4),
            frame_index: position,
            seek_serial: serial,
            scale: 1.,
            header: FrameHeader::new_straight_rgba(
                t.base_source.as_str(),
                t.generation,
                4,
                position,
                2,
                2,
            )
            .unwrap(),
            render_duration_micros: 10,
            record: BinaryRecordHeader {
                kind: BinaryRecordKind::FrameRgba8,
                identity: preview_identity(t),
                request_id: 4,
                offset: 0,
                payload_len: 16,
            },
            editor_metadata: None,
        },
        pixels: vec![37; 16],
    }
}
fn ready(t: &OperationTag, position: usize, serial: u64, total: usize) -> ReadyPreview {
    let timeline = PreviewTimelineResponse {
        timing_generation: 0,
        envelope: envelope(t, 2),
        fps: 30,
        width: 2,
        height: 2,
        total_frames: total,
        duration_seconds: total as f32 / 30.,
        scenes: vec![],
        audio_tracks: vec![],
    };
    let samples = total as u64 * 1600;
    let start = (position as u64 * 1600).min(samples);
    ReadyPreview::new(
        t.clone(),
        timeline,
        InspectResponse {
            envelope: envelope(t, 3),
            diagnostics: vec![],
            truncated: false,
        },
        if total == 0 {
            None
        } else {
            Some(frame(t, position.min(total - 1), serial))
        },
        PreparedAudioDescriptor {
            envelope: envelope(t, 5),
            artifact_id: "mix".into(),
            sample_rate: 48000,
            channels: 2,
            sample_count: samples,
            byte_count: samples * 8,
            sha256: "0".repeat(64),
            silent: true,
        },
        start,
        vec![0; ((samples - start) * 8).min(256) as usize],
        position,
        serial,
    )
    .unwrap()
}

fn applied() -> (Fx, Controller, Promotion, PreviewState, OperationTag) {
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    // The preview currently displayed was built from the task base.
    let base = c.begin_job(JobKind::Build).unwrap();
    c.complete(&base, JobResult::Built(base.base_source.clone()))
        .unwrap();
    let mut displayed = PreviewState::default();
    displayed.begin(base.clone());
    displayed
        .install(&ready(&base, 0, 0, 60), c.state())
        .unwrap();
    let (_context, captured, report) = validated(&f, &mut c, "edit", multi_file_edit);
    let CompletionOutcome::Applied(promotion) =
        c.complete_validated_task(&captured, &report).unwrap()
    else {
        panic!("auto apply")
    };
    (f, c, *promotion, displayed, base)
}

#[test]
fn a_staged_candidate_installs_only_under_its_promotion_authorization() {
    let (_f, c, promotion, mut p, base_tag) = applied();
    let auth = promotion.authorization.clone().unwrap();
    let serial = p.seek(12, 1.).unwrap();
    let staged = ready(auth.tag(), 12, serial, 60);
    // Not begun: refused, and the old preview keeps playing.
    assert!(p.install(&staged, c.state()).is_err());
    assert_eq!(p.displayed(), Some(&preview_identity(&base_tag)));
    p.begin_promotion(&auth);
    assert_eq!(p.status, PreviewStatus::Preparing);
    p.install(&staged, c.state()).unwrap();
    assert_eq!(p.displayed(), Some(&preview_identity(auth.tag())));
    assert_eq!(
        preview_identity(auth.tag()).source_revision,
        promotion.record.published.as_str()
    );
    assert_ne!(
        preview_identity(auth.tag()).source_revision,
        base_tag.base_source.as_str()
    );
}

#[test]
fn a_preview_tagged_with_the_task_base_or_a_stale_playhead_never_installs() {
    let (_f, c, promotion, mut p, base_tag) = applied();
    let auth = promotion.authorization.clone().unwrap();
    let serial = p.seek(12, 1.).unwrap();
    // A candidate forged with the accepted-base identity.
    let forged = OperationTag {
        base_source: promotion.record.task_base.clone(),
        ..auth.tag().clone()
    };
    p.begin_promotion(&auth);
    assert!(
        p.install(&ready(&forged, 12, serial, 60), c.state())
            .is_err()
    );
    assert!(
        p.install(&ready(&base_tag, 12, serial, 60), c.state())
            .is_err()
    );
    // Another generation / operation of the same published bytes.
    let other = OperationTag {
        generation: auth.tag().generation + 1,
        ..auth.tag().clone()
    };
    assert!(
        p.install(&ready(&other, 12, serial, 60), c.state())
            .is_err()
    );
    // A seek after staging: the staged first frame is obsolete (latest intent wins).
    let newer = p.seek(30, 1.).unwrap();
    assert!(newer > serial);
    assert!(
        p.install(&ready(auth.tag(), 12, serial, 60), c.state())
            .is_err()
    );
    assert!(
        p.install(&ready(auth.tag(), 30, newer, 60), c.state())
            .is_ok()
    );
    assert_eq!(p.position(), 30);
}

#[test]
fn the_adopted_preview_of_a_promotion_enables_timeline_editing() {
    let (_f, mut c, promotion, _p, base_tag) = applied();
    let auth = promotion.authorization.clone().unwrap();
    c.learn_timeline(&base_tag, &ready(&base_tag, 0, 0, 60).timeline);
    assert!(matches!(
        c.timeline_snapshot().editing,
        TimingEditing::Disabled(_)
    ));
    c.learn_timeline(auth.tag(), &ready(auth.tag(), 0, 0, 60).timeline);
    assert_eq!(c.timeline_snapshot().editing, TimingEditing::Enabled);
}

/// The timeline of `t` with scenes `intro` (30 frames), `main` (60) and `outro` (30).
fn three_scenes(t: &OperationTag) -> PreviewTimelineResponse {
    let mut start = 0;
    let scenes = [("intro", 30), ("main", 60), ("outro", 30)]
        .into_iter()
        .enumerate()
        .map(|(index, (key, frames))| {
            let scene = PreviewSceneInfo {
                instance_id: format!("scene-{index}"),
                editor_instance_key: Some(key.into()),
                index,
                name: key.into(),
                full_name: key.into(),
                start_frame: start,
                end_frame: start + frames,
                start_seconds: start as f32 / 30.,
                end_seconds: (start + frames) as f32 / 30.,
                clip: Some(PreviewClipTiming {
                    timing_key: key.into(),
                    clip_index: 0,
                    trim_start_frames: 0,
                    trim_end_frames: 0,
                    scene_index: Some(index),
                    scene_frames: Some(frames),
                }),
            };
            start += frames;
            scene
        })
        .collect();
    PreviewTimelineResponse {
        scenes,
        ..ready(t, 0, 0, 120).timeline
    }
}

const PUBLISHED_TIMING: &str = "{\"version\":1,\"clips\":[{\"key\":\"outro\",\"trim_start_frames\":0,\"trim_end_frames\":0},{\"key\":\"intro\",\"trim_start_frames\":0,\"trim_end_frames\":0}]}\n";

#[test]
fn a_published_timeline_json_is_what_the_session_exports_and_edits() {
    use studio_engine::timing::{TimelineCommand, TimingDoc, TrimEdge};
    let f = fixture();
    let mut c = open_retrying(&f.root, &f.paths).unwrap();
    let base = c.begin_job(JobKind::Build).unwrap();
    c.complete(&base, JobResult::Built(base.base_source.clone()))
        .unwrap();
    c.learn_timeline(&base, &three_scenes(&base));
    let (_context, captured, report) = validated(&f, &mut c, "reorder", |draft| {
        fs::write(draft.join("timeline.json"), PUBLISHED_TIMING).unwrap()
    });
    let CompletionOutcome::Applied(promotion) =
        c.complete_validated_task(&captured, &report).unwrap()
    else {
        panic!("auto apply")
    };
    c.reconcile().unwrap();
    let published = TimingDoc::from_json(PUBLISHED_TIMING).unwrap();
    let exported = c.timing_for_export().unwrap().expect("published clips");
    assert_eq!(TimingDoc::from_json(&exported).unwrap(), published);
    let notice = c.timeline_snapshot().notice.expect("history cleared");
    assert!(notice.starts_with("An agent edit published"), "{notice}");

    let auth = promotion.authorization.clone().unwrap();
    c.learn_timeline(auth.tag(), &three_scenes(auth.tag()));
    c.timeline_command(TimelineCommand::Trim {
        clip: 0,
        edge: TrimEdge::End,
        frames: 5,
    })
    .unwrap();
    let live =
        TimingDoc::from_json(&fs::read_to_string(f.root.join("timeline.json")).unwrap()).unwrap();
    assert_eq!(live.clips[0].key, "outro");
    assert_eq!(live.clips[0].trim_end_frames, 5);
}

#[test]
fn export_refuses_a_published_edit_whose_code_changed_since() {
    let (f, mut c, _promotion, _p, _base) = applied();
    fs::write(
        f.root.join("src/lib.rs"),
        "// changed after the publication\n",
    )
    .unwrap();
    c.reconcile().unwrap();
    let build = c.begin_job(JobKind::Build).unwrap();
    c.complete(&build, JobResult::Built(build.base_source.clone()))
        .unwrap();
    let error = c.freeze_export_source().err().expect("refused").to_string();
    assert!(
        error.contains("is not the current project source"),
        "{error}"
    );
}

#[test]
fn any_later_source_change_revokes_the_authorization() {
    let (f, mut c, promotion, mut p, _base) = applied();
    let auth = promotion.authorization.clone().unwrap();
    p.begin_promotion(&auth);
    let staged = ready(auth.tag(), 0, p.serial(), 60);
    assert!(p.can_install(&staged, c.state()).is_ok());
    fs::write(f.root.join("notes.txt"), "edited after the Apply\n").unwrap();
    c.reconcile().unwrap();
    assert!(!auth.is_current(c.state()));
    assert!(p.can_install(&staged, c.state()).is_err());
    // A reopened session cannot use the old authorization either.
    drop(c);
    let c = open_retrying(&f.root, &f.paths).unwrap();
    assert!(c.state().promotion().is_none());
    assert!(p.can_install(&staged, c.state()).is_err());
}
