use fframes::{AudioTrack, Frame};

use super::{
    Timing,
    series::{Slot, Spec, layout},
};

/// How far the transitions of a [`TransitionSeries`](super::TransitionSeries) sequence have
/// got, from [`Ui::transition_progress`](crate::ui::Ui::transition_progress) (Remotion's
/// `useTransitionProgress`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionProgress {
    /// 0 to 1 while the sequence comes in, 1 otherwise.
    pub entering: f32,
    /// 0 to 1 while the sequence goes away, 0 otherwise.
    pub exiting: f32,
    /// The `Ui` renders a sequence of a series.
    pub in_series: bool,
}

impl Default for TransitionProgress {
    fn default() -> Self {
        Self {
            entering: 1.0,
            exiting: 0.0,
            in_series: false,
        }
    }
}

/// What a `Ui` knows about the transitions around it: the progress, and the length in
/// seconds of each transition that is running.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct TransitionState {
    pub progress: TransitionProgress,
    pub entering_seconds: Option<f32>,
    pub exiting_seconds: Option<f32>,
}

/// The length of one item of a [`TransitionSeries`](super::TransitionSeries), for
/// [`audio_fades`]: plain data, so it can be built in `fn audio`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeriesLength {
    /// A sequence of this many seconds.
    Sequence(f32),
    /// A transition with this timing.
    Transition(Timing),
}

/// Where a sequence of a series plays and how long its audio fades, in seconds from the
/// start of the series (Remotion's audio fade on transitions). The kit cannot change
/// `fn audio`, so place the scene's audio there with these times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioFade {
    /// The sequence starts.
    pub start: f32,
    /// The sequence ends.
    pub end: f32,
    /// The length of the transition it comes in with, 0 without one.
    pub fade_in: f32,
    /// The length of the transition it goes away with, 0 without one.
    pub fade_out: f32,
}

impl AudioFade {
    /// `track` with these fades. Its range is the caller's: usually
    /// `Second(series_start + fade.start)..Second(series_start + fade.end)`.
    pub fn apply<'a>(&self, track: AudioTrack<'a>) -> AudioTrack<'a> {
        track.fade_in(self.fade_in).fade_out(self.fade_out)
    }
}

/// The audio fades of every sequence of a series whose items have these lengths, at `fps`.
/// Transitions are placed, shortened and skipped as the series places them.
///
/// ```
/// use fframes::{AudioTimestamp::Second, AudioTrack};
/// use fframes_kit::prelude::*;
///
/// let fades = audio_fades(
///     &[
///         SeriesLength::Sequence(3.0),
///         SeriesLength::Transition(Timing::default()),
///         SeriesLength::Sequence(3.0),
///     ],
///     30,
/// );
/// let second = fades[1].apply(AudioTrack::new("two.wav", Second(fades[1].start)..Second(fades[1].end)));
/// assert_eq!(second.mix.fade_in, Timing::default().duration());
/// ```
pub fn audio_fades(items: &[SeriesLength], fps: usize) -> Vec<AudioFade> {
    let at = Frame::new(0, 0, fps.max(1));
    let specs: Vec<Spec> = items
        .iter()
        .map(|item| match item {
            SeriesLength::Sequence(s) => Spec::Sequence(at.second_to_frame(s.max(0.0))),
            SeriesLength::Transition(t) => {
                Spec::Transition(at.second_to_frame(t.duration().max(0.0)))
            }
        })
        .collect();
    let (slots, _) = layout(&specs);
    let window = |i: Option<usize>| match i.and_then(|i| slots.get(i)) {
        Some(Slot::Transition(Some(w))) => at.frame_to_second(w.len),
        _ => 0.0,
    };
    slots
        .iter()
        .enumerate()
        .filter_map(|(i, slot)| match *slot {
            Slot::Sequence { start, len } => Some(AudioFade {
                start: at.frame_to_second(start),
                end: at.frame_to_second(start + len),
                fade_in: window(i.checked_sub(1)),
                fade_out: window(Some(i + 1)),
            }),
            Slot::Transition(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::RateFunc;

    fn linear(duration: f32) -> SeriesLength {
        SeriesLength::Transition(Timing::Linear {
            duration,
            easing: RateFunc::Linear,
        })
    }

    #[test]
    fn fades_follow_the_transitions_around_each_sequence() {
        let fades = audio_fades(
            &[
                SeriesLength::Sequence(2.0),
                linear(0.5),
                SeriesLength::Sequence(2.0),
                linear(1.0),
                SeriesLength::Sequence(3.0),
            ],
            30,
        );
        assert_eq!(fades.len(), 3);
        assert_eq!((fades[0].start, fades[0].end), (0.0, 2.0));
        assert_eq!((fades[0].fade_in, fades[0].fade_out), (0.0, 0.5));
        assert_eq!((fades[1].start, fades[1].end), (1.5, 3.5));
        assert_eq!((fades[1].fade_in, fades[1].fade_out), (0.5, 1.0));
        assert_eq!(
            (fades[2].start, fades[2].fade_in, fades[2].fade_out),
            (2.5, 1.0, 0.0)
        );
    }

    #[test]
    fn skipped_and_shortened_transitions_fade_like_the_series_draws_them() {
        let fades = audio_fades(
            &[
                linear(0.5),
                SeriesLength::Sequence(1.0),
                linear(2.0),
                SeriesLength::Sequence(3.0),
            ],
            30,
        );
        assert_eq!(fades[0].fade_in, 0.0);
        // shortened to the one second sequence before it
        assert_eq!(fades[0].fade_out, 1.0);
        assert_eq!(fades[1].start, 0.0);
    }
}
