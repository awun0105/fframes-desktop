//! Voiceover timing (manim-voiceover): [`VoiceoverScript`] places recorded blocks on the
//! timeline and times their `<bookmark mark="A"/>` tags from word timings. The kit never
//! records or transcribes; the timings come from a transcript or a subtitle file.

mod bookmarks;
mod script;

pub use bookmarks::{Mark, bookmark_ms, strip_bookmarks};
pub use script::{BlockTime, Bookmark, VoiceoverBlock, VoiceoverScript};
