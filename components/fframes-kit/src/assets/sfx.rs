//! Sound effects on the beats of a video (Remotion's `@remotion/sfx`). The kit's pack is
//! the folder `sfx/` of the crate: short CC0 files, each with its source in `sfx/LICENSE.md`.
//! Copy the files a video uses into its media folder (the pack's `LICENSE.md` is not a media
//! file, so `include_media_dir!` cannot read the pack itself).
//!
//! The kit cannot change `fn audio`, so the helpers return [`AudioTrack`]s to add to the
//! video's or the scene's [`AudioMap`](fframes::AudioMap). Each hit plays at
//! [`DEFAULT_GAIN_DB`], at a rate varied by up to [`RATE_VARIATION`] (seeded by its index,
//! so a render sounds the same every time) so repeats do not sound mechanical. Change either
//! on the returned tracks.

use std::ops::Range;

use fframes::{
    AudioTimestamp::{Eof, Second},
    AudioTrack,
};

use super::cursor::CursorPath;
use crate::{
    motion::{Beat, hash},
    transitions::{SeriesLength, audio_fades},
};

/// The gain of a hit. The pack's files peak at -3 dBFS.
pub const DEFAULT_GAIN_DB: f32 = -6.0;

/// The largest change of rate (and pitch) of a hit, as a share: 0.04 plays between 0.96 and
/// 1.04.
pub const RATE_VARIATION: f32 = 0.04;

/// A sound of the pack, or a file of your own. Each names its file in the media folder, so
/// a file of your own under the same name replaces the kit's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sfx<'a> {
    /// `whoosh.wav`: a stick swung through the air, for transitions.
    Whoosh,
    /// `click.wav`: a mouse click.
    Click,
    /// `ding.wav`: a bell.
    Ding,
    /// `pop.wav`: a short pop, for things that appear.
    Pop,
    /// `swoosh-down.wav`: a falling sweep, for things that close or leave.
    SwooshDown,
    /// `shutter.wav`: a camera shutter.
    Shutter,
    /// `switch.wav`: a toggle switch.
    Switch,
    /// A file of the media folder.
    File(&'a str),
}

impl<'a> Sfx<'a> {
    /// The sounds of the pack.
    pub const PACK: [Sfx<'static>; 7] = [
        Sfx::Whoosh,
        Sfx::Click,
        Sfx::Ding,
        Sfx::Pop,
        Sfx::SwooshDown,
        Sfx::Shutter,
        Sfx::Switch,
    ];

    /// The file the sound plays.
    pub fn file(self) -> &'a str {
        match self {
            Sfx::Whoosh => "whoosh.wav",
            Sfx::Click => "click.wav",
            Sfx::Ding => "ding.wav",
            Sfx::Pop => "pop.wav",
            Sfx::SwooshDown => "swoosh-down.wav",
            Sfx::Shutter => "shutter.wav",
            Sfx::Switch => "switch.wav",
            Sfx::File(file) => file,
        }
    }
}

/// The rate of the hit with index `i`.
fn rate(i: usize) -> f32 {
    1.0 + RATE_VARIATION * (hash(i as u32) * 2.0 - 1.0)
}

/// One hit of `sfx` at each of `times` (seconds of the video, or of the scene in a scene's
/// `fn audio`).
pub fn at<'a>(times: &[f32], sfx: Sfx<'a>) -> Vec<AudioTrack<'a>> {
    times
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            AudioTrack::new(sfx.file(), Second(t.max(0.0))..Eof)
                .gain_db(DEFAULT_GAIN_DB)
                .rate(rate(i))
        })
        .collect()
}

/// When each transition of a series with these items starts (where its two sequences start
/// to overlap), in seconds from the series start at `fps`. Skipped transitions are left
/// out, shortened ones start where the series starts them. See
/// [`TransitionSeries::lengths`](crate::transitions::TransitionSeries::lengths).
pub fn transition_times(series: &[SeriesLength], fps: usize) -> Vec<f32> {
    audio_fades(series, fps)
        .iter()
        .filter(|fade| fade.fade_in > 0.0)
        .map(|fade| fade.start)
        .collect()
}

/// One hit of `sfx` at the start of each transition of a series (see [`transition_times`]),
/// for a series that starts at 0; shift the times and use [`at`] for one that starts later.
pub fn on_transitions<'a>(
    series: &[SeriesLength],
    fps: usize,
    sfx: Sfx<'a>,
) -> Vec<AudioTrack<'a>> {
    at(&transition_times(series, fps), sfx)
}

/// One hit of `sfx` on every whole beat of `beats` (the end excluded): `0.0..8.0` is beats
/// 0 to 7.
pub fn on_beats<'a>(beat: &Beat, beats: Range<f32>, sfx: Sfx<'a>) -> Vec<AudioTrack<'a>> {
    let first = beats.start.ceil() as i64;
    // the last whole beat before the end
    let last = (beats.end.ceil() as i64) - 1;
    let times: Vec<f32> = (first..=last).map(|b| beat.time(b as f32)).collect();
    at(&times, sfx)
}

/// A [`Sfx::Click`] on each click of a cursor path.
pub fn on_clicks(path: &CursorPath) -> Vec<AudioTrack<'static>> {
    let times: Vec<f32> = path.clicks().collect();
    at(&times, Sfx::Click)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{anim::RateFunc, assets::cursor::CursorPoint, transitions::Timing};

    fn linear(duration: f32) -> SeriesLength {
        SeriesLength::Transition(Timing::Linear {
            duration,
            easing: RateFunc::Linear,
        })
    }

    fn start(track: &AudioTrack<'_>) -> f32 {
        match track.range.start {
            Second(s) => s,
            _ => f32::NAN,
        }
    }

    #[test]
    fn on_transitions_hits_each_overlap_start() {
        let series = [
            SeriesLength::Sequence(2.0),
            linear(0.5),
            SeriesLength::Sequence(2.0),
            linear(1.0),
            SeriesLength::Sequence(3.0),
        ];
        let tracks = on_transitions(&series, 30, Sfx::Whoosh);
        assert_eq!(tracks.len(), 2);
        // 2.0 - 0.5, then 1.5 + 2.0 - 1.0
        assert_eq!(start(&tracks[0]), 1.5);
        assert_eq!(start(&tracks[1]), 2.5);
        assert!(tracks.iter().all(|t| t.file == "whoosh.wav"));
        // a transition at the edge is skipped
        let edge = [linear(0.5), SeriesLength::Sequence(1.0)];
        assert!(on_transitions(&edge, 30, Sfx::Whoosh).is_empty());
    }

    #[test]
    fn on_beats_matches_the_beat_grid() {
        let grid = Beat::from_bpm(120.0, 1.0);
        let tracks = on_beats(&grid, 0.5..4.0, Sfx::Click);
        let times: Vec<f32> = tracks.iter().map(start).collect();
        assert_eq!(times, vec![grid.time(1.0), grid.time(2.0), grid.time(3.0)]);
        assert_eq!(times, vec![1.5, 2.0, 2.5]);
    }

    #[test]
    fn hits_vary_in_rate_and_files_can_be_replaced() {
        let tracks = at(&[0.0, 1.0, 2.0], Sfx::File("mine.wav"));
        let rates: Vec<f32> = tracks.iter().map(|t| t.mix.rate).collect();
        assert!(rates.iter().all(|r| (r - 1.0).abs() <= RATE_VARIATION));
        assert_ne!(rates[0], rates[1]);
        assert_eq!(at(&[0.0], Sfx::Pop)[0].mix.rate, rates[0]);
        assert_eq!(tracks[0].file, "mine.wav");
        assert_eq!(tracks[0].mix.gain_db, DEFAULT_GAIN_DB);
        assert_eq!(Sfx::SwooshDown.file(), "swoosh-down.wav");
    }

    #[test]
    fn on_clicks_follows_the_cursor() {
        let path = CursorPath {
            points: vec![
                CursorPoint::new(0.0, 0.0, 0.0),
                CursorPoint::new(1.2, 10.0, 0.0).click(),
                CursorPoint::new(2.0, 20.0, 0.0).click(),
            ],
            ..CursorPath::default()
        };
        let times: Vec<f32> = on_clicks(&path).iter().map(start).collect();
        assert_eq!(times, vec![1.2, 2.0]);
    }
}
