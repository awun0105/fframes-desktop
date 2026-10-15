//! Split, trim and move on the laid out clips (Cap's `editor_edits.rs` maths, rewritten
//! against the timing document).
use serde::Serialize;

use super::doc::{CodeScene, TIMING_DOC_VERSION, TimingClip, TimingDoc};

/// One clip as it plays: a code scene, or the part of it the document keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedClip {
    pub key: String,
    /// The scene's own length, before trims.
    pub scene_frames: usize,
    pub trim_start_frames: usize,
    pub trim_end_frames: usize,
}

impl ResolvedClip {
    /// Frames the clip plays (always at least one).
    pub fn frames(&self) -> usize {
        self.scene_frames - self.trim_start_frames - self.trim_end_frames
    }
}

/// Which edge of a clip a trim moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimEdge {
    Start,
    End,
}

/// The clips of `doc` over `scenes`, laid out as the framework lays them out: the
/// document's clips of known scenes in its order, then the scenes it leaves out in code
/// order. Clips of scenes the code no longer has come back separately; edits keep them.
pub fn resolve(
    doc: &TimingDoc,
    scenes: &[CodeScene],
) -> Result<(Vec<ResolvedClip>, Vec<TimingClip>), String> {
    let mut clips = Vec::with_capacity(doc.clips.len().max(scenes.len()));
    let mut orphans = Vec::new();
    let mut mentioned = vec![false; scenes.len()];
    for clip in &doc.clips {
        let Some(index) = scenes.iter().position(|scene| scene.key == clip.key) else {
            orphans.push(clip.clone());
            continue;
        };
        let scene_frames = scenes[index].frames;
        if clip
            .trim_start_frames
            .checked_add(clip.trim_end_frames)
            .is_none_or(|trimmed| trimmed >= scene_frames)
        {
            return Err(format!(
                "clip {:?} trims {}+{} frames of a {scene_frames} frame scene",
                clip.key, clip.trim_start_frames, clip.trim_end_frames
            ));
        }
        mentioned[index] = true;
        clips.push(ResolvedClip {
            key: clip.key.clone(),
            scene_frames,
            trim_start_frames: clip.trim_start_frames,
            trim_end_frames: clip.trim_end_frames,
        });
    }
    for (scene, _) in scenes.iter().zip(mentioned).filter(|(_, seen)| !seen) {
        clips.push(ResolvedClip {
            key: scene.key.clone(),
            scene_frames: scene.frames,
            trim_start_frames: 0,
            trim_end_frames: 0,
        });
    }
    Ok((clips, orphans))
}

/// The document that plays `clips`, followed by the clips of missing scenes.
pub fn to_doc(clips: &[ResolvedClip], orphans: &[TimingClip]) -> TimingDoc {
    TimingDoc {
        version: TIMING_DOC_VERSION,
        clips: clips
            .iter()
            .map(|clip| TimingClip {
                key: clip.key.clone(),
                trim_start_frames: clip.trim_start_frames,
                trim_end_frames: clip.trim_end_frames,
            })
            .chain(orphans.iter().cloned())
            .collect(),
    }
}

fn clip_at(clips: &[ResolvedClip], clip: usize) -> Result<&ResolvedClip, String> {
    clips
        .get(clip)
        .ok_or_else(|| format!("there is no clip {clip}; the timeline has {}", clips.len()))
}

/// Splits clip `clip` `at` frames after its first played frame into two clips of the
/// same scene that play the same frames together.
pub fn split(clips: &mut Vec<ResolvedClip>, clip: usize, at: usize) -> Result<(), String> {
    let original = clip_at(clips, clip)?.clone();
    if at == 0 || at >= original.frames() {
        return Err(format!(
            "a split must fall inside the clip (1..{} frames)",
            original.frames()
        ));
    }
    let tail = ResolvedClip {
        trim_start_frames: original.trim_start_frames + at,
        ..original.clone()
    };
    clips[clip].trim_end_frames = original.trim_end_frames + original.frames() - at;
    clips.insert(clip + 1, tail);
    Ok(())
}

/// Moves one edge of clip `clip` to cut `frames` frames of its scene, kept so the clip
/// plays at least one frame. Out-of-range values are clamped, as a drag past the end of
/// the scene stops there.
pub fn trim(
    clips: &mut [ResolvedClip],
    clip: usize,
    edge: TrimEdge,
    frames: usize,
) -> Result<(), String> {
    clip_at(clips, clip)?;
    let clip = &mut clips[clip];
    match edge {
        TrimEdge::Start => {
            clip.trim_start_frames = frames.min(clip.scene_frames - clip.trim_end_frames - 1)
        }
        TrimEdge::End => {
            clip.trim_end_frames = frames.min(clip.scene_frames - clip.trim_start_frames - 1)
        }
    }
    Ok(())
}

/// Moves clip `from` so that it plays at position `to` (indices of the result).
pub fn move_clip(clips: &mut Vec<ResolvedClip>, from: usize, to: usize) -> Result<(), String> {
    clip_at(clips, from)?;
    if to >= clips.len() {
        return Err(format!(
            "there is no position {to}; the timeline has {} clips",
            clips.len()
        ));
    }
    let clip = clips.remove(from);
    clips.insert(to, clip);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenes() -> Vec<CodeScene> {
        [("intro", 60), ("story", 90), ("outro", 30)]
            .into_iter()
            .map(|(key, frames)| CodeScene {
                key: key.into(),
                frames,
            })
            .collect()
    }

    fn layout(clips: &[ResolvedClip]) -> Vec<(&str, usize, usize)> {
        clips
            .iter()
            .map(|c| (c.key.as_str(), c.trim_start_frames, c.trim_end_frames))
            .collect()
    }

    #[test]
    fn an_empty_document_plays_the_code_order() {
        let (clips, orphans) = resolve(&TimingDoc::default(), &scenes()).unwrap();
        assert_eq!(
            layout(&clips),
            [("intro", 0, 0), ("story", 0, 0), ("outro", 0, 0)]
        );
        assert!(orphans.is_empty());
    }

    #[test]
    fn missing_scenes_are_appended_and_unknown_clips_kept() {
        let doc = TimingDoc::from_json(
            r#"{"version":1,"clips":[
                {"key":"outro","trim_start_frames":0,"trim_end_frames":5},
                {"key":"gone","trim_start_frames":0,"trim_end_frames":0}]}"#,
        )
        .unwrap();
        let (clips, orphans) = resolve(&doc, &scenes()).unwrap();
        assert_eq!(
            layout(&clips),
            [("outro", 0, 5), ("intro", 0, 0), ("story", 0, 0)]
        );
        assert_eq!(orphans[0].key, "gone");
        let round = to_doc(&clips, &orphans);
        assert_eq!(round.clips.len(), 4);
        assert_eq!(round.clips[3].key, "gone");
        let too_long = TimingDoc::from_json(
            r#"{"version":1,"clips":[{"key":"outro","trim_start_frames":20,"trim_end_frames":10}]}"#,
        )
        .unwrap();
        assert!(resolve(&too_long, &scenes()).is_err());
    }

    #[test]
    fn split_keeps_every_frame_and_the_scene_numbers() {
        let (mut clips, _) = resolve(&TimingDoc::default(), &scenes()).unwrap();
        trim(&mut clips, 1, TrimEdge::Start, 10).unwrap();
        split(&mut clips, 1, 30).unwrap();
        assert_eq!(
            layout(&clips),
            [
                ("intro", 0, 0),
                ("story", 10, 50),
                ("story", 40, 0),
                ("outro", 0, 0)
            ]
        );
        assert_eq!(clips[1].frames() + clips[2].frames(), 80);
        assert!(split(&mut clips, 1, 0).is_err());
        assert!(
            split(&mut clips, 1, 30).is_err(),
            "the head is 30 frames now"
        );
        assert!(split(&mut clips, 9, 1).is_err());
    }

    #[test]
    fn trims_clamp_to_one_frame_and_moves_reorder() {
        let (mut clips, _) = resolve(&TimingDoc::default(), &scenes()).unwrap();
        trim(&mut clips, 2, TrimEdge::End, 10).unwrap();
        trim(&mut clips, 2, TrimEdge::Start, 500).unwrap();
        assert_eq!(layout(&clips)[2], ("outro", 19, 10));
        assert_eq!(clips[2].frames(), 1);
        move_clip(&mut clips, 2, 0).unwrap();
        assert_eq!(clips[0].key, "outro");
        assert!(move_clip(&mut clips, 0, 3).is_err());
        assert!(trim(&mut clips, 3, TrimEdge::Start, 1).is_err());
    }
}
