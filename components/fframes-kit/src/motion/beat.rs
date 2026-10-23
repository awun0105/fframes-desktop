//! A musical beat grid, for videos cut to a soundtrack.

use super::{clamp01, spring};

/// The beat grid of a soundtrack: beat 0 is the first downbeat, at `downbeat` seconds, and
/// beats are `period` seconds apart (negative beats are before it). Animations written in
/// beats stay on the music when the tempo changes.
///
/// The period is stored, not the BPM, so a measured period (`0.454_522` s) is used as is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beat {
    /// Seconds per beat.
    pub period: f32,
    /// Second of the video where beat 0 is.
    pub downbeat: f32,
    /// `(stiffness, damping)` of [`snap`](Self::snap): fast, a small overshoot.
    pub snap_spring: (f32, f32),
    /// `(stiffness, damping)` of [`soft`](Self::soft): slower, a visible overshoot.
    pub soft_spring: (f32, f32),
}

impl Beat {
    /// A grid of `period` seconds per beat starting at `downbeat` seconds.
    pub const fn new(period: f32, downbeat: f32) -> Self {
        Self {
            period,
            downbeat,
            snap_spring: (320.0, 22.0),
            soft_spring: (170.0, 15.0),
        }
    }

    /// A grid of `bpm` beats per minute starting at `downbeat` seconds.
    pub fn from_bpm(bpm: f32, downbeat: f32) -> Self {
        Self::new(60.0 / bpm, downbeat)
    }

    /// The second `beat` is on.
    pub fn time(&self, beat: f32) -> f32 {
        self.downbeat + beat * self.period
    }

    /// The frame `beat` starts on, rounded so boundaries placed on beats never drift.
    pub fn frame(&self, beat: f32, fps: usize) -> usize {
        (self.time(beat) * fps as f32).round().max(0.0) as usize
    }

    /// The beat at `seconds` (negative before beat 0).
    pub fn beat_at(&self, seconds: f32) -> f32 {
        (seconds - self.downbeat) / self.period
    }

    /// Frames from beat `start` to beat `end`; `None` is frame 0 for `start` and
    /// `total_frames` for `end`.
    pub fn span_frames(
        &self,
        start: Option<f32>,
        end: Option<f32>,
        fps: usize,
        total_frames: usize,
    ) -> usize {
        let a = start.map_or(0, |b| self.frame(b, fps));
        let b = end.map_or(total_frames, |b| self.frame(b, fps));
        b.saturating_sub(a)
    }

    /// The snappy spring, `beats` after its release.
    pub fn snap(&self, beats: f32) -> f32 {
        let (k, d) = self.snap_spring;
        spring(beats * self.period, k, d)
    }

    /// The soft spring, `beats` after its release.
    pub fn soft(&self, beats: f32) -> f32 {
        let (k, d) = self.soft_spring;
        spring(beats * self.period, k, d)
    }

    /// A pulse that fires on every beat: 1 on the beat, decaying by `sharpness` until the
    /// next. `beat` is a position on the grid, such as `beat_at(seconds)`.
    pub fn pulse(beat: f32, sharpness: f32) -> f32 {
        let f = beat - beat.floor();
        clamp01((-f * sharpness).exp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRID: Beat = Beat::new(0.454_522, 2.148_51);

    #[test]
    fn beats_map_to_seconds_and_frames() {
        assert_eq!(GRID.time(0.0), 2.148_51);
        assert_eq!(GRID.time(4.0), 2.148_51 + 4.0 * 0.454_522);
        assert_eq!(GRID.frame(0.0, 60), 129);
        assert!((GRID.beat_at(GRID.time(7.5)) - 7.5).abs() < 1e-4);
        assert_eq!(
            GRID.span_frames(Some(0.0), Some(8.0), 60, 0),
            GRID.frame(8.0, 60) - 129
        );
        assert_eq!(GRID.span_frames(None, None, 60, 600), 600);
        assert!((Beat::from_bpm(120.0, 0.0).period - 0.5).abs() < 1e-6);
    }

    #[test]
    fn springs_and_pulse() {
        assert_eq!(GRID.snap(0.0), 0.0);
        assert!((GRID.snap(4.0) - 1.0).abs() < 1e-3);
        assert!(GRID.soft(0.5) < GRID.snap(0.5));
        assert_eq!(Beat::pulse(3.0, 8.0), 1.0);
        assert!(Beat::pulse(3.5, 8.0) < 0.02);
    }
}
