//! The timing document (`timeline.json`) as the studio edits it.
//!
//! The format is `fframes::TimingDoc`. The engine does not link the framework, so it
//! mirrors the type and the checks that need no video; the worker checks the trims
//! against the scene lengths when the document is applied.
use fframes_studio_protocol::{MAX_TIMING_DOC_BYTES, PreviewTimelineResponse};
use serde::{Deserialize, Serialize};

/// The only document version the framework reads.
pub const TIMING_DOC_VERSION: u32 = 1;
/// Upper bound on clips in one document (`fframes::MAX_TIMING_CLIPS`).
pub const MAX_TIMING_CLIPS: usize = 4096;
/// Upper bound on the bytes of one clip key (`fframes::MAX_TIMING_KEY_BYTES`).
pub const MAX_TIMING_KEY_BYTES: usize = 512;

/// Play order, trims and splits of the video's code scenes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimingDoc {
    pub version: u32,
    pub clips: Vec<TimingClip>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimingClip {
    pub key: String,
    pub trim_start_frames: usize,
    pub trim_end_frames: usize,
}

impl Default for TimingDoc {
    /// No clips: the scenes play in code order.
    fn default() -> Self {
        Self {
            version: TIMING_DOC_VERSION,
            clips: Vec::new(),
        }
    }
}

impl TimingDoc {
    /// Parses and validates a serialized document; any error rejects all of it.
    pub fn from_json(json: &str) -> Result<Self, String> {
        if json.len() > MAX_TIMING_DOC_BYTES {
            return Err(format!(
                "the timing document of {} bytes exceeds the limit of {MAX_TIMING_DOC_BYTES}",
                json.len()
            ));
        }
        let doc: Self = serde_json::from_str(json)
            .map_err(|error| format!("the timing document is malformed: {error}"))?;
        doc.validate()?;
        Ok(doc)
    }

    /// Everything that does not depend on the video's scenes.
    pub fn validate(&self) -> Result<(), String> {
        if self.version != TIMING_DOC_VERSION {
            return Err(format!(
                "timing version {} is not supported (expected {TIMING_DOC_VERSION})",
                self.version
            ));
        }
        if self.clips.len() > MAX_TIMING_CLIPS {
            return Err(format!(
                "{} clips exceed the limit of {MAX_TIMING_CLIPS}",
                self.clips.len()
            ));
        }
        if let Some(clip) = self.clips.iter().find(|clip| {
            clip.key.is_empty()
                || clip.key.len() > MAX_TIMING_KEY_BYTES
                || clip.key.chars().any(char::is_control)
        }) {
            return Err(format!("invalid clip key {:?}", clip.key));
        }
        Ok(())
    }

    /// The bytes written to `timeline.json`: pretty JSON with a final newline, so the
    /// file reads well in a diff.
    pub fn to_json(&self) -> String {
        let mut json =
            serde_json::to_string_pretty(self).expect("a timing document always serializes");
        json.push('\n');
        json
    }
}

/// One scene of the video as the code defines it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeScene {
    /// The scene's key in timing documents.
    pub key: String,
    /// The scene's own length in frames, before trims.
    pub frames: usize,
}

/// Why a timeline cannot be edited from a worker's report.
pub const TIMING_UNSUPPORTED: &str = "this preview's worker does not report the timing of its scenes, so its timeline cannot be edited";

/// The video's scenes in code order, read from a worker's timeline whatever document it
/// was laid out with: every scene appears at least once (the worker appends the scenes
/// a document leaves out).
pub fn code_scenes(timeline: &PreviewTimelineResponse) -> Result<Vec<CodeScene>, String> {
    let mut scenes: Vec<(usize, CodeScene)> = Vec::new();
    for scene in &timeline.scenes {
        let Some(clip) = &scene.clip else {
            return Err(TIMING_UNSUPPORTED.into());
        };
        let (Some(index), Some(frames)) = (clip.scene_index, clip.scene_frames) else {
            return Err(TIMING_UNSUPPORTED.into());
        };
        match scenes.iter().find(|(known, _)| *known == index) {
            Some((_, known)) if known.key != clip.timing_key || known.frames != frames => {
                return Err(format!(
                    "the preview reports scene {index} twice with different keys or lengths"
                ));
            }
            Some(_) => {}
            None => scenes.push((
                index,
                CodeScene {
                    key: clip.timing_key.clone(),
                    frames,
                },
            )),
        }
    }
    scenes.sort_by_key(|(index, _)| *index);
    if scenes.iter().enumerate().any(|(n, (index, _))| n != *index) {
        return Err("the preview timeline does not list every scene of the video".into());
    }
    Ok(scenes.into_iter().map(|(_, scene)| scene).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_parse_like_the_framework() {
        let doc = TimingDoc::from_json(
            r#"{"version":1,"clips":[{"key":"intro","trim_start_frames":3,"trim_end_frames":0}]}"#,
        )
        .unwrap();
        assert_eq!(doc.clips[0].trim_start_frames, 3);
        assert_eq!(TimingDoc::from_json(&doc.to_json()).unwrap(), doc);
        for bad in [
            r#"{"version":2,"clips":[]}"#,
            r#"{"version":1,"clips":[],"extra":1}"#,
            r#"{"version":1,"clips":[{"key":"","trim_start_frames":0,"trim_end_frames":0}]}"#,
            r#"{"version":1,"clips":[{"key":"a","trim_start_frames":-1,"trim_end_frames":0}]}"#,
            "not json",
        ] {
            assert!(TimingDoc::from_json(bad).is_err(), "{bad}");
        }
        let large = " ".repeat(MAX_TIMING_DOC_BYTES + 1);
        assert!(
            TimingDoc::from_json(&large)
                .unwrap_err()
                .contains("exceeds")
        );
    }
}
