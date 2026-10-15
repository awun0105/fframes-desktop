//! The clip track: the timing document's clips when the engine resolved them, else the
//! compiled report's first scene lane. Cap's track model over `TimelineSnapshot`.
use std::collections::HashMap;

use studio_engine::TimelineModel;
use studio_engine::timing::TimelineSnapshot;

/// One segment of the scene rows, in video frames.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipSegment {
    pub start: usize,
    pub end: usize,
    /// The scene row: 0 is the clip track, higher rows hold scenes that overlap it.
    pub lane: usize,
    pub label: String,
    /// The compiled scene that plays this clip, used for selection and the task scope.
    /// `None` while the displayed report has not caught up with the document.
    pub scene: Option<usize>,
}

/// The scene rows: the engine's clips in play order, then any compiled scene no clip
/// plays. A clip maps to the report scene with the same timing key and occurrence and
/// takes that scene's frames and row, so overlapping scenes stay where they play and a
/// click never maps through pixel positions across two layouts. A clip the displayed
/// report lacks follows the previous segment on the clip track. Without clips the rows
/// are the report's scenes.
pub fn clip_segments(
    model: &TimelineModel,
    snapshot: Option<&TimelineSnapshot>,
) -> Vec<ClipSegment> {
    let report = model.report();
    let from_scene = |index: usize| {
        let scene = &report.scenes[index];
        ClipSegment {
            start: scene.start_frame,
            end: scene.end_frame,
            lane: model.scene_lane(index),
            label: format!("{} #{}", scene.name, scene.index),
            scene: Some(index),
        }
    };
    let mut claimed = vec![false; report.scenes.len()];
    let mut occurrences = HashMap::<&str, usize>::new();
    let mut previous_end = 0_usize;
    let mut segments: Vec<_> = snapshot
        .map_or(&[][..], |snapshot| &snapshot.clips[..])
        .iter()
        .map(|clip| {
            let occurrence = occurrences.entry(clip.key.as_str()).or_default();
            let scene = report.scenes.iter().position(|scene| {
                scene.clip.as_ref().is_some_and(|timing| {
                    timing.timing_key == clip.key && timing.clip_index == *occurrence
                })
            });
            *occurrence += 1;
            let segment = match scene {
                Some(index) => {
                    claimed[index] = true;
                    from_scene(index)
                }
                None => ClipSegment {
                    start: previous_end,
                    end: previous_end.saturating_add(clip.frames()),
                    lane: 0,
                    label: clip.key.clone(),
                    scene: None,
                },
            };
            previous_end = segment.end;
            segment
        })
        .collect();
    segments.extend(
        (0..report.scenes.len())
            .filter(|index| {
                let scene = &report.scenes[*index];
                !claimed[*index] && scene.start_frame < scene.end_frame
            })
            .map(from_scene),
    );
    segments
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use fframes_studio_protocol::*;
    use studio_engine::TimingEditing;
    use studio_engine::timing::ResolvedClip;

    pub(crate) fn report(fps: usize, scenes: &[(&str, usize, usize)]) -> PreviewTimelineResponse {
        let total_frames = scenes.iter().map(|(_, _, end)| *end).max().unwrap_or(0);
        PreviewTimelineResponse {
            timing_generation: 0,
            envelope: PreviewEnvelope {
                contract_version: PREVIEW_CONTRACT_VERSION,
                identity: PreviewIdentity {
                    project_id: "project".into(),
                    open_session: "session".into(),
                    source_revision: "a".repeat(64),
                    worker_generation: 1,
                },
                request_id: 1,
            },
            fps,
            width: 1920,
            height: 1080,
            total_frames,
            duration_seconds: total_frames as f32 / fps as f32,
            scenes: scenes
                .iter()
                .enumerate()
                .map(|(index, (key, start, end))| PreviewSceneInfo {
                    instance_id: format!("scene-{index}"),
                    editor_instance_key: None,
                    index,
                    name: (*key).into(),
                    full_name: (*key).into(),
                    start_frame: *start,
                    end_frame: *end,
                    start_seconds: *start as f32 / fps as f32,
                    end_seconds: *end as f32 / fps as f32,
                    clip: Some(PreviewClipTiming {
                        timing_key: (*key).into(),
                        clip_index: scenes[..index].iter().filter(|(k, ..)| k == key).count(),
                        trim_start_frames: 0,
                        trim_end_frames: 0,
                        scene_index: Some(index),
                        scene_frames: Some(end - start),
                    }),
                })
                .collect(),
            audio_tracks: Vec::new(),
        }
    }

    pub(crate) fn snapshot(clips: &[(&str, usize, usize, usize)]) -> TimelineSnapshot {
        TimelineSnapshot {
            generation: 1,
            timing_json: None,
            clips: clips
                .iter()
                .map(|(key, scene_frames, trim_start, trim_end)| ResolvedClip {
                    key: (*key).into(),
                    scene_frames: *scene_frames,
                    trim_start_frames: *trim_start,
                    trim_end_frames: *trim_end,
                })
                .collect(),
            can_undo: false,
            can_redo: false,
            editing: TimingEditing::Enabled,
            notice: None,
        }
    }

    fn spans(segments: &[ClipSegment]) -> Vec<(usize, usize, Option<usize>)> {
        segments.iter().map(|s| (s.start, s.end, s.scene)).collect()
    }

    #[test]
    fn overlapping_scenes_keep_their_frames_and_row() {
        // `Overlap`: the middle scene starts 15 frames before the first ends.
        let overlapped = TimelineModel::new(report(
            30,
            &[("intro", 0, 60), ("middle", 45, 135), ("outro", 105, 165)],
        ))
        .unwrap();
        let clips = snapshot(&[
            ("intro", 60, 0, 0),
            ("middle", 90, 0, 0),
            ("outro", 60, 0, 0),
        ]);
        for snapshot in [None, Some(&clips)] {
            let segments = clip_segments(&overlapped, snapshot);
            assert_eq!(
                segments
                    .iter()
                    .map(|s| (s.start, s.end, s.lane))
                    .collect::<Vec<_>>(),
                [(0, 60, 0), (45, 135, 1), (105, 165, 0)]
            );
            assert_eq!(segments.iter().map(|s| s.end).max(), Some(165));
        }
    }

    #[test]
    fn without_clips_the_rows_are_the_reports_scenes() {
        // The overlay [20, 40) packs into row 1, off the clip track.
        let model = TimelineModel::new(report(
            30,
            &[("intro", 0, 30), ("badge", 20, 40), ("main", 30, 90)],
        ))
        .unwrap();
        let segments = clip_segments(&model, None);
        assert_eq!(
            spans(&segments),
            [(0, 30, Some(0)), (20, 40, Some(1)), (30, 90, Some(2))]
        );
        assert_eq!(
            segments.iter().map(|s| s.lane).collect::<Vec<_>>(),
            [0, 1, 0]
        );
        assert_eq!(segments[2].label, "main #2");
        let empty = snapshot(&[]);
        assert_eq!(clip_segments(&model, Some(&empty)), segments);
    }

    #[test]
    fn snapshot_clips_lay_out_end_to_end_and_match_the_retimed_report() {
        // timeline.json: outro, intro trimmed by 5 at its start, then intro split again.
        let clips = snapshot(&[
            ("outro", 30, 0, 0),
            ("intro", 30, 5, 10),
            ("intro", 30, 20, 0),
        ]);
        let retimed = TimelineModel::new(report(
            30,
            &[("outro", 0, 30), ("intro", 30, 45), ("intro", 45, 55)],
        ))
        .unwrap();
        let segments = clip_segments(&retimed, Some(&clips));
        assert_eq!(
            spans(&segments),
            [(0, 30, Some(0)), (30, 45, Some(1)), (45, 55, Some(2))]
        );
        for segment in &segments {
            let scene = &retimed.report().scenes[segment.scene.unwrap()];
            assert_eq!(
                (segment.start, segment.end),
                (scene.start_frame, scene.end_frame)
            );
        }
    }

    #[test]
    fn clips_the_displayed_report_lacks_are_not_selectable() {
        // The report still plays the code order with one intro; the document splits it.
        let code_order =
            TimelineModel::new(report(30, &[("intro", 0, 30), ("outro", 30, 60)])).unwrap();
        let clips = snapshot(&[
            ("intro", 30, 0, 15),
            ("intro", 30, 15, 0),
            ("outro", 30, 0, 0),
        ]);
        let segments = clip_segments(&code_order, Some(&clips));
        // Mapped clips keep the report's frames until the report catches up; the clip
        // the report lacks follows the previous segment.
        assert_eq!(
            spans(&segments),
            [(0, 30, Some(0)), (30, 45, None), (30, 60, Some(1))]
        );
        assert_eq!(segments[1].label, "intro");
    }
}
