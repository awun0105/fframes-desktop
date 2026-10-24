use fframes::{
    Svgr,
    diagnostics::{Diagnostic, report},
};

use crate::ui::{Children, Component, Ui};

use super::{
    Presentation, Side, Timing,
    progress::{AudioFade, SeriesLength, TransitionProgress, TransitionState, audio_fades},
};

/// One item of a [`TransitionSeries`].
pub enum TransitionItem<'a> {
    /// Children for `seconds` seconds.
    Sequence {
        seconds: f32,
        children: Children<'a>,
    },
    /// A transition between the sequences before and after it.
    Transition {
        timing: Timing,
        presentation: Box<dyn Presentation + 'a>,
    },
}

/// Sequences one after another with transitions between them (Remotion's
/// `<TransitionSeries>`). A transition overlaps its two sequences by its duration, so the
/// series is shorter than its sequences by the sum of the transitions. A transition longer
/// than one of its sequences is shortened to it; a transition that is first, last, or next
/// to another transition is skipped. `inspect` warns about both.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct TransitionSeries<'a> {
    /// Makes the ids of the transitions' clip paths unique when two series share a frame.
    pub key: Option<&'static str>,
    pub items: Vec<TransitionItem<'a>>,
}

impl<'a> TransitionSeries<'a> {
    /// A sequence item.
    pub fn sequence(seconds: f32, children: Children<'a>) -> TransitionItem<'a> {
        TransitionItem::Sequence { seconds, children }
    }

    /// The lengths of the items, for [`audio_fades`].
    pub fn lengths(&self) -> Vec<SeriesLength> {
        self.items
            .iter()
            .map(|item| match item {
                TransitionItem::Sequence { seconds, .. } => SeriesLength::Sequence(*seconds),
                TransitionItem::Transition { timing, .. } => SeriesLength::Transition(*timing),
            })
            .collect()
    }

    /// Where each sequence plays and how long its audio fades at `fps`, see [`audio_fades`].
    pub fn audio_fades(&self, fps: usize) -> Vec<AudioFade> {
        audio_fades(&self.lengths(), fps)
    }

    /// A transition item.
    pub fn transition(timing: Timing, presentation: impl Presentation + 'a) -> TransitionItem<'a> {
        TransitionItem::Transition {
            timing,
            presentation: Box::new(presentation),
        }
    }
}

/// The length in frames of each item, as the layout sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Spec {
    Sequence(usize),
    Transition(usize),
}

/// Where an item landed, in frames from the series start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Slot {
    Sequence {
        start: usize,
        len: usize,
    },
    /// `None` for a transition without a sequence on both sides.
    Transition(Option<Window>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Window {
    start: usize,
    pub(super) len: usize,
    /// The transition was longer than a neighbour and got shortened.
    clamped: bool,
}

/// Places the items and returns them with the series' length in frames.
pub(super) fn layout(specs: &[Spec]) -> (Vec<Slot>, usize) {
    let sequence_len = |i: usize| match specs.get(i) {
        Some(Spec::Sequence(len)) => Some(*len),
        _ => None,
    };
    // the overlap each transition takes, before anything is placed
    let overlaps: Vec<Option<(usize, bool)>> = specs
        .iter()
        .enumerate()
        .map(|(i, spec)| match *spec {
            Spec::Transition(len) => {
                let before = i.checked_sub(1).and_then(sequence_len)?;
                let after = sequence_len(i + 1)?;
                let shortest = before.min(after);
                Some((len.min(shortest), len > shortest))
            }
            Spec::Sequence(_) => None,
        })
        .collect();
    let mut slots = Vec::with_capacity(specs.len());
    let mut end = 0;
    let mut total = 0;
    for (i, spec) in specs.iter().enumerate() {
        match *spec {
            Spec::Sequence(len) => {
                let overlap = i
                    .checked_sub(1)
                    .and_then(|j| overlaps[j])
                    .map_or(0, |(d, _)| d);
                let start = end - overlap.min(end);
                slots.push(Slot::Sequence { start, len });
                end = start + len;
                total = total.max(end);
            }
            Spec::Transition(_) => {
                slots.push(Slot::Transition(overlaps[i].map(|(len, clamped)| Window {
                    start: end - len,
                    len,
                    clamped,
                })));
            }
        }
    }
    (slots, total)
}

/// The progress and the length in seconds of the transition placed at `w` at frame `f`, when
/// `f` is inside it.
fn running(w: &Window, timing: &Timing, f: usize, fps: f32) -> Option<(f32, f32)> {
    (f >= w.start && f < w.start + w.len).then(|| {
        let length = w.len as f32 / fps;
        (timing.progress((f - w.start) as f32 / fps, length), length)
    })
}

/// What a sequence sees of the transitions around it, each `(progress, seconds)` while it
/// runs.
fn sequence_state(entering: Option<(f32, f32)>, exiting: Option<(f32, f32)>) -> TransitionState {
    TransitionState {
        progress: TransitionProgress {
            entering: entering.map_or(1.0, |e| e.0),
            exiting: exiting.map_or(0.0, |e| e.0),
            in_series: true,
        },
        entering_seconds: entering.map(|e| e.1),
        exiting_seconds: exiting.map(|e| e.1),
    }
}

impl<'a> Component<'a> for TransitionSeries<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let fps = ui.frame.fps as f32;
        let specs: Vec<Spec> = self
            .items
            .iter()
            .map(|item| match item {
                TransitionItem::Sequence { seconds, .. } => Spec::Sequence(ui.frames(*seconds)),
                TransitionItem::Transition { timing, .. } => {
                    Spec::Transition(ui.frames(timing.duration()))
                }
            })
            .collect();
        let (slots, _) = layout(&specs);
        let f = ui.frame.index;
        for (i, slot) in slots.iter().enumerate() {
            match slot {
                Slot::Transition(None) => report(Diagnostic::RendererWarning {
                    message: format!(
                        "TransitionSeries: transition {i} needs a sequence on both sides, it is skipped"
                    ),
                }),
                Slot::Transition(Some(w)) if w.clamped => report(Diagnostic::RendererWarning {
                    message: format!(
                        "TransitionSeries: transition {i} is longer than a sequence next to it, it is shortened to {} frames",
                        w.len
                    ),
                }),
                _ => {}
            }
        }
        // keyed ids still carry the index: two transitions around a short sequence overlap
        let id = |j: usize| match self.key {
            Some(key) => ui.id(Some(&format!("{key}-{j}")), j, "transition"),
            None => ui.id(None, j, "transition"),
        };
        // progress and length in seconds of the transition in slot `j`, when the frame is
        // inside it
        let progress = |j: usize| -> Option<(f32, f32, &dyn Presentation, String)> {
            let (
                Slot::Transition(Some(w)),
                TransitionItem::Transition {
                    timing,
                    presentation,
                },
            ) = (slots.get(j)?, self.items.get(j)?)
            else {
                return None;
            };
            let (p, length) = running(w, timing, f, fps)?;
            Some((p, length, presentation.as_ref(), id(j)))
        };
        let mut drawn = Vec::new();
        for (i, (slot, item)) in slots.iter().zip(&self.items).enumerate() {
            let (Slot::Sequence { start, len }, TransitionItem::Sequence { children, .. }) =
                (slot, item)
            else {
                continue;
            };
            if f < *start || f >= start + len {
                continue;
            }
            let exiting = progress(i + 1);
            let entering = i.checked_sub(1).and_then(progress);
            let mut child = ui.shifted(*start, Some(*len));
            child.transition = sequence_state(
                entering.as_ref().map(|e| (e.0, e.1)),
                exiting.as_ref().map(|e| (e.0, e.1)),
            );
            let mut content = children(&mut child);
            if let Some((p, _, presentation, id)) = exiting {
                content = presentation.apply(p, Side::Exiting, content, &child, &id);
            }
            if let Some((p, _, presentation, id)) = entering {
                content = presentation.apply(p, Side::Entering, content, &child, &id);
            }
            drawn.push(content);
        }
        drawn.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use Spec::{Sequence as S, Transition as T};

    #[test]
    fn transitions_overlap_their_neighbours() {
        let (slots, total) = layout(&[S(60), T(15), S(60), T(30), S(90)]);
        assert_eq!(total, 60 + 60 + 90 - 15 - 30);
        assert_eq!(slots[0], Slot::Sequence { start: 0, len: 60 });
        assert_eq!(
            slots[1],
            Slot::Transition(Some(Window {
                start: 45,
                len: 15,
                clamped: false
            }))
        );
        // the second sequence starts with the transition, so both show during it
        assert_eq!(slots[2], Slot::Sequence { start: 45, len: 60 });
        assert_eq!(slots[4], Slot::Sequence { start: 75, len: 90 });
    }

    #[test]
    fn misplaced_transitions_are_skipped() {
        let (slots, total) = layout(&[T(10), S(30), T(10), T(10), S(30), T(5)]);
        assert_eq!(total, 60);
        assert!(
            slots
                .iter()
                .all(|s| !matches!(s, Slot::Transition(Some(_))))
        );
    }

    #[test]
    fn long_transitions_are_shortened_to_the_shorter_neighbour() {
        let (slots, total) = layout(&[S(20), T(40), S(60)]);
        assert_eq!(
            slots[1],
            Slot::Transition(Some(Window {
                start: 0,
                len: 20,
                clamped: true
            }))
        );
        assert_eq!(total, 60);
    }

    #[test]
    fn sequences_see_the_progress_of_their_transitions() {
        let timing = Timing::Linear {
            duration: 0.5,
            easing: crate::anim::RateFunc::Linear,
        };
        let (slots, _) = layout(&[S(30), T(15), S(30)]);
        let Slot::Transition(Some(w)) = slots[1] else {
            panic!("placed");
        };
        let fps = 30.0;
        let at = |f: usize| {
            let run = running(&w, &timing, f, fps);
            (sequence_state(None, run), sequence_state(run, None))
        };
        // before the transition: the first sequence has not started to exit
        let (before, _) = at(14);
        assert_eq!(before.progress.exiting, 0.0);
        assert_eq!(before.exiting_seconds, None);
        // its first frame
        let (exiting, entering) = at(15);
        assert_eq!(exiting.progress.exiting, 0.0);
        assert_eq!(entering.progress.entering, 0.0);
        assert_eq!(entering.entering_seconds, Some(0.5));
        assert_eq!(entering.progress.exiting, 0.0);
        assert!(entering.progress.in_series);
        // half way
        let (exiting, entering) = at(15 + 15 / 2);
        assert!((exiting.progress.exiting - 7.0 / 15.0).abs() < 1e-6);
        assert!((entering.progress.entering - 7.0 / 15.0).abs() < 1e-6);
        // after it: entered for good
        let (_, entering) = at(30);
        assert_eq!(entering.progress.entering, 1.0);
        assert_eq!(entering.entering_seconds, None);
        // outside a series
        let outside = TransitionProgress::default();
        assert_eq!(
            (outside.entering, outside.exiting, outside.in_series),
            (1.0, 0.0, false)
        );
    }

    #[test]
    fn progress_is_zero_at_the_start_and_one_at_the_end() {
        let timing = Timing::Linear {
            duration: 0.5,
            easing: crate::anim::RateFunc::Linear,
        };
        let (fps, len) = (30.0, 15);
        let at = |f: usize| timing.progress(f as f32 / fps, len as f32 / fps);
        assert_eq!(at(0), 0.0);
        assert!(at(len - 1) < 1.0);
        assert_eq!(at(len), 1.0);
    }
}
