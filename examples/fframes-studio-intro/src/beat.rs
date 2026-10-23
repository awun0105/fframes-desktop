//! The musical clock. The soundtrack is an edit of a 132 BPM track whose
//! cuts all land on phrase boundaries, so the whole video sits on one beat
//! grid: every scene starts on a downbeat and animations are written in beats.

use fframes::Frame;
use fframes_kit::motion::Beat;
pub use fframes_kit::motion::{clamp01, cubic_in_out, expo_in, expo_out, lerp, quart_out, spring};

pub const FPS: usize = 60;
/// One beat of the soundtrack, in seconds (132.007 BPM, measured).
pub const BEAT: f32 = 0.454_522;
/// First downbeat of the soundtrack (bar 1, beat 1).
pub const DOWNBEAT0: f32 = 2.148_51;
/// The video stops a moment after the last bar and the music fades out under it.
pub const TOTAL_SECONDS: f32 = 108.8;

/// The 16:9 cut plays one 32-beat phrase of the music twice (see `audio_map_long`),
/// so it runs longer than the portrait and short cuts.
pub const LONG_EXTRA_BEATS: f32 = 32.0;
/// Video beat where the repeated phrase starts in the 16:9 cut.
pub const LONG_REPEAT_AT: f32 = 144.0;
/// Last beat of the 16:9 cut, where it stops.
pub const LONG_END: f32 = (TOTAL_SECONDS - DOWNBEAT0) / BEAT + LONG_EXTRA_BEATS;
/// Length of the 16:9 cut.
pub const LONG_TOTAL_SECONDS: f32 = TOTAL_SECONDS + LONG_EXTRA_BEATS * BEAT;

/// The soundtrack's beat grid.
pub const GRID: Beat = Beat::new(BEAT, DOWNBEAT0);

pub fn beat_time(beat: f32) -> f32 {
    GRID.time(beat)
}

/// The frame a beat starts on, rounded so scene boundaries never drift.
pub fn beat_frame(beat: f32) -> usize {
    GRID.frame(beat, FPS)
}

/// Seconds since the start of the video, whichever scene the frame belongs to.
pub fn gsec(frame: &Frame) -> f32 {
    frame.global_index as f32 / FPS as f32
}

/// Position on the beat grid of the whole video (negative before bar 1).
pub fn gbeat(frame: &Frame) -> f32 {
    GRID.beat_at(gsec(frame))
}

/// Number of frames between two beats; `None` stands for the start or end of
/// the video.
pub fn span_frames(start: Option<f32>, end: Option<f32>) -> usize {
    GRID.span_frames(
        start,
        end,
        FPS,
        (TOTAL_SECONDS * FPS as f32).round() as usize,
    )
}

/// Where `x` is between `a` and `b`, clamped to 0..1. Unlike `motion::progress`, a
/// reversed window (`b < a`) ramps down instead of stepping.
pub fn prog(x: f32, a: f32, b: f32) -> f32 {
    clamp01((x - a) / (b - a))
}

/// A snappy UI spring, `beats` since it was released.
pub fn snap(beats: f32) -> f32 {
    GRID.snap(beats)
}

/// A softer spring with a visible overshoot.
pub fn soft(beats: f32) -> f32 {
    GRID.soft(beats)
}

/// Decaying pulse that fires on every beat: 1 on the beat, ~0 half a beat later.
pub fn pulse(beat: f32, sharpness: f32) -> f32 {
    Beat::pulse(beat, sharpness)
}

/// Deterministic hash noise, 0..1.
pub fn hash(n: f32) -> f32 {
    let x = (n * 127.1 + 311.7).sin() * 43_758.547;
    x - x.floor()
}
