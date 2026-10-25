use std::borrow::Cow;

use fframes::{AudioTimestamp, AudioTrack};

use super::bookmarks::{bookmark_ms, strip_bookmarks};
use crate::{text::CaptionWord, time::Sequence, ui::Children};

/// One voiceover: a recorded `audio` file of the media and what is said in it (manim-voiceover's
/// `with self.voiceover(text=...)` block).
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceoverBlock<'a> {
    /// What is said, with `<bookmark mark="A"/>` tags where something should happen.
    pub text: Cow<'a, str>,
    pub audio: &'a str,
    /// Seconds of the container (the video or the scene the script belongs to).
    pub start: f32,
    pub duration: f32,
    /// The words of the audio, in milliseconds from the start of the file (a transcript from
    /// `from_whisper_json`, or the cues of `parse_srt`).
    pub words: Vec<CaptionWord>,
}

impl<'a> VoiceoverBlock<'a> {
    /// A block at 0 that lasts until its last word ends.
    pub fn new(text: impl Into<Cow<'a, str>>, audio: &'a str, words: Vec<CaptionWord>) -> Self {
        let duration = words.last().map_or(0.0, |w| w.end_ms as f32 / 1000.0);
        Self {
            text: text.into(),
            audio,
            start: 0.0,
            duration,
            words,
        }
    }
}

/// When a block plays, in seconds of the container.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockTime {
    pub start: f32,
    pub duration: f32,
    pub end: f32,
}

/// A bookmark of a block and its time in seconds of the container.
#[derive(Debug, Clone, PartialEq)]
pub struct Bookmark {
    pub mark: String,
    pub block: usize,
    pub seconds: f32,
}

/// The timing of a narrated video (manim-voiceover's `VoiceoverScene` and its tracker): when
/// each block plays, when each bookmark is reached, the words for [`WordCaptions`] and the
/// audio for `fn audio`. It reads timings that exist; it records and transcribes nothing.
///
/// [`WordCaptions`]: crate::overlay::WordCaptions
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceoverScript<'a> {
    pub blocks: Vec<VoiceoverBlock<'a>>,
    bookmarks: Vec<Bookmark>,
}

impl<'a> VoiceoverScript<'a> {
    /// The blocks at their own starts. A bookmark gets the start of the first word after it,
    /// counted in letters and digits so a transcript that splits words differently from the
    /// text still lines up; a bookmark after the last word gets the end of that word, and one
    /// in a block without words the block's start.
    pub fn new(blocks: Vec<VoiceoverBlock<'a>>) -> Self {
        let mut bookmarks = Vec::new();
        for (i, block) in blocks.iter().enumerate() {
            let (_, marks) = strip_bookmarks(&block.text);
            for mark in marks {
                let ms = bookmark_ms(&block.words, mark.offset).unwrap_or(0.0);
                let seconds = (ms as f32 / 1000.0).min(block.duration);
                bookmarks.push(Bookmark {
                    mark: mark.name,
                    block: i,
                    seconds: block.start + seconds,
                });
            }
        }
        Self { blocks, bookmarks }
    }

    /// The blocks one after the other from `start`, `gap` seconds apart.
    pub fn in_sequence(mut blocks: Vec<VoiceoverBlock<'a>>, start: f32, gap: f32) -> Self {
        let mut at = start;
        for block in &mut blocks {
            block.start = at;
            at += block.duration + gap;
        }
        Self::new(blocks)
    }

    /// When block `i` plays.
    pub fn block(&self, i: usize) -> Option<BlockTime> {
        self.blocks.get(i).map(|b| BlockTime {
            start: b.start,
            duration: b.duration,
            end: b.start + b.duration,
        })
    }

    /// The end of the last block.
    pub fn end(&self) -> f32 {
        self.blocks
            .iter()
            .map(|b| b.start + b.duration)
            .fold(0.0, f32::max)
    }

    /// The text of block `i` without its bookmark tags.
    pub fn plain_text(&self, i: usize) -> Option<String> {
        self.blocks.get(i).map(|b| strip_bookmarks(&b.text).0)
    }

    /// The time of the bookmark `mark`; the first one when several blocks use the name.
    pub fn bookmark(&self, mark: &str) -> Option<f32> {
        self.bookmarks
            .iter()
            .find(|b| b.mark == mark)
            .map(|b| b.seconds)
    }

    /// Every bookmark in the order of the blocks.
    pub fn bookmarks(&self) -> &[Bookmark] {
        &self.bookmarks
    }

    /// Seconds from `t` until the bookmark `mark`, 0 once it has passed (the tracker's
    /// `time_until_bookmark`, for `wait_until_bookmark`).
    pub fn time_until_bookmark(&self, mark: &str, t: f32) -> Option<f32> {
        self.bookmark(mark).map(|at| (at - t).max(0.0))
    }

    /// Seconds of block `i` left at `t`: its duration before it starts, 0 after it ends (the
    /// tracker's `get_remaining_duration`).
    pub fn remaining(&self, i: usize, t: f32) -> Option<f32> {
        self.block(i)
            .map(|b| (b.end - t).clamp(0.0, b.duration.max(0.0)))
    }

    /// The words of every block in milliseconds of the container, for `tiktok_pages` and
    /// [`WordCaptions`] (manim-voiceover's subcaptions). The first word of a block gets a
    /// leading space, so a page can break between blocks. A block without
    /// words is one caption with its text, over the whole block.
    ///
    /// [`WordCaptions`]: crate::overlay::WordCaptions
    pub fn subcaptions(&self) -> Vec<CaptionWord> {
        let mut out = Vec::new();
        for (i, block) in self.blocks.iter().enumerate() {
            let shift = f64::from(block.start) * 1000.0;
            if block.words.is_empty() {
                let end = shift + f64::from(block.duration) * 1000.0;
                let text = self.plain_text(i).unwrap_or_default();
                out.push(CaptionWord::new(format!(" {text}"), shift, end));
                continue;
            }
            for (j, word) in block.words.iter().enumerate() {
                let mut word = word.clone();
                if j == 0 && !word.text.starts_with(char::is_whitespace) {
                    word.text.insert(0, ' ');
                }
                word.start_ms += shift;
                word.end_ms += shift;
                word.timestamp_ms = word.timestamp_ms.map(|ms| ms + shift);
                out.push(word);
            }
        }
        out
    }

    /// One track per block, playing its audio from its start for its duration. Add them to
    /// the video's (or the scene's) `fn audio`.
    pub fn audio_tracks(&self) -> Vec<AudioTrack<'a>> {
        self.blocks
            .iter()
            .map(|b| {
                AudioTrack::new(
                    b.audio,
                    AudioTimestamp::Second(b.start)..AudioTimestamp::Second(b.start + b.duration),
                )
            })
            .collect()
    }

    /// A [`Sequence`] that shows `children` while block `i` plays, its time starting at 0
    /// with the block. For a [`Series`](crate::time::Series), give an item the block's
    /// `duration` from [`block`](Self::block).
    pub fn sequence(&self, i: usize, children: Children<'a>) -> Option<Sequence<'a>> {
        self.block(i).map(|b| Sequence {
            from: b.start,
            duration: Some(b.duration),
            children,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(list: &[(&str, f64, f64)]) -> Vec<CaptionWord> {
        list.iter()
            .map(|(text, start, end)| CaptionWord::new(*text, *start, *end))
            .collect()
    }

    fn script() -> VoiceoverScript<'static> {
        let first = VoiceoverBlock::new(
            "Watch the <bookmark mark=\"circle\"/>circle grow.",
            "one.wav",
            words(&[
                ("Watch", 0.0, 300.0),
                (" the", 300.0, 450.0),
                (" circle", 450.0, 900.0),
                (" grow.", 900.0, 1500.0),
            ]),
        );
        let second = VoiceoverBlock::new(
            "Then <bookmark mark=\"square\"/>a square.<bookmark mark=\"done\"/>",
            "two.wav",
            words(&[
                (" Then", 0.0, 400.0),
                (" a", 500.0, 600.0),
                (" square.", 600.0, 1200.0),
            ]),
        );
        VoiceoverScript::in_sequence(vec![first, second], 0.5, 0.25)
    }

    #[test]
    fn blocks_follow_each_other() {
        let script = script();
        assert_eq!(
            script.block(1),
            Some(BlockTime {
                start: 2.25,
                duration: 1.2,
                end: 3.45
            })
        );
        assert_eq!(script.end(), 3.45);
        assert_eq!(script.block(2), None);
        assert_eq!(script.plain_text(1).as_deref(), Some("Then a square."));
    }

    #[test]
    fn bookmarks_are_timed_from_the_words() {
        let script = script();
        assert_eq!(script.bookmark("circle"), Some(0.95));
        assert_eq!(script.bookmark("square"), Some(2.75));
        assert_eq!(script.bookmark("done"), Some(3.45));
        assert_eq!(script.bookmark("missing"), None);
        assert_eq!(script.time_until_bookmark("square", 2.0), Some(0.75));
        assert_eq!(script.time_until_bookmark("square", 3.0), Some(0.0));
    }

    #[test]
    fn the_remaining_time_counts_down() {
        let script = script();
        assert_eq!(script.remaining(0, 0.0), Some(1.5));
        assert_eq!(script.remaining(0, 1.5), Some(0.5));
        assert_eq!(script.remaining(0, 9.0), Some(0.0));
        assert_eq!(script.remaining(5, 0.0), None);
    }

    #[test]
    fn subcaptions_and_audio_are_in_container_time() {
        let script = script();
        let words = script.subcaptions();
        assert_eq!(words.len(), 7);
        assert_eq!(
            words[0].text, " Watch",
            "a block's first word starts a page"
        );
        assert_eq!((words[0].start_ms, words[0].end_ms), (500.0, 800.0));
        assert_eq!(words[4].start_ms, 2250.0);
        assert_eq!(words[4].timestamp_ms, Some(2450.0));
        let pages = crate::overlay::tiktok_pages(&words, 1200.0);
        assert_eq!(pages.len(), 2, "the second block starts a page");

        let tracks = script.audio_tracks();
        assert_eq!(tracks.len(), 2);
        let silent = VoiceoverScript::new(vec![VoiceoverBlock {
            start: 1.0,
            duration: 2.0,
            ..VoiceoverBlock::new("Said <bookmark mark=\"x\"/>here", "a.wav", Vec::new())
        }]);
        assert_eq!(silent.bookmark("x"), Some(1.0));
        assert_eq!(
            silent.subcaptions(),
            [CaptionWord::new(" Said here", 1000.0, 3000.0)]
        );
    }
}
