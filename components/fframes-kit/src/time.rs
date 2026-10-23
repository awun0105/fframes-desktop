//! Time containers, as in Remotion: [`Sequence`], [`Series`], [`Loop`], [`Freeze`], and
//! the full-canvas group [`Fill`].
//!
//! A component has no start time of its own. Its parent places it with a container, and
//! inside the container `ui.seconds()` starts at 0.

use fframes::{Color, Svgr};

use crate::ui::{Children, Component, Ui};

/// The local index inside a window that starts at frame `start` and lasts `length` frames
/// (`None`: no end), or `None` outside it.
fn window(index: usize, start: usize, length: Option<usize>) -> Option<usize> {
    let local = index.checked_sub(start)?;
    match length {
        Some(length) if local >= length => None,
        _ => Some(local),
    }
}

/// Shows its children from `from` seconds on, for `duration` seconds or until the parent
/// ends. Inside, time starts at 0 and `ui.duration` is the sequence's length.
pub struct Sequence<'a> {
    /// Start in the parent's local seconds.
    pub from: f32,
    /// Length in seconds; `None` runs until the parent ends.
    pub duration: Option<f32>,
    pub children: Children<'a>,
}

impl<'a> Component<'a> for Sequence<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let start = ui.frames(self.from);
        let length = match self.duration {
            Some(seconds) => Some(ui.frames(seconds)),
            None => ui.duration.map(|d| d.saturating_sub(start)),
        };
        if window(ui.frame.index, start, length).is_none() {
            return Svgr::empty();
        }
        let mut child = ui.shifted(start, length);
        (self.children)(&mut child)
    }
}

/// Children one after the other, each for its own number of seconds. Each item starts
/// `overlap` seconds before the previous one ends, so both show during the overlap.
pub struct Series<'a> {
    /// `(seconds, children)` in play order.
    pub items: Vec<(f32, Children<'a>)>,
    /// Seconds each item overlaps the one before it; 0 plays them back to back.
    pub overlap: f32,
}

/// Start and length in frames of each `Series` item.
fn series_windows(fps: usize, lengths: &[f32], overlap: f32) -> Vec<(usize, usize)> {
    let frames = |s: f32| (s.max(0.0) * fps as f32) as usize;
    let mut start = 0.0_f32;
    lengths
        .iter()
        .map(|&seconds| {
            let window = (frames(start), frames(seconds));
            start = (start + seconds - overlap).max(0.0);
            window
        })
        .collect()
}

impl<'a> Component<'a> for Series<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let lengths: Vec<f32> = self.items.iter().map(|(s, _)| *s).collect();
        let windows = series_windows(ui.frame.fps, &lengths, self.overlap);
        self.items
            .iter()
            .zip(windows)
            .filter(|(_, (start, length))| window(ui.frame.index, *start, Some(*length)).is_some())
            .map(|((_, children), (start, length))| children(&mut ui.shifted(start, Some(length))))
            .collect()
    }
}

/// Repeats its children every `duration` seconds, `times` times or forever (`None`).
pub struct Loop<'a> {
    pub duration: f32,
    pub times: Option<usize>,
    pub children: Children<'a>,
}

/// The start frame of the current iteration, or `None` after the last one.
fn loop_start(index: usize, period: usize, times: Option<usize>) -> Option<usize> {
    if period == 0 {
        return None;
    }
    let iteration = index / period;
    match times {
        Some(times) if iteration >= times => None,
        _ => Some(iteration * period),
    }
}

impl<'a> Component<'a> for Loop<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let period = ui.frames(self.duration);
        let Some(start) = loop_start(ui.frame.index, period, self.times) else {
            return Svgr::empty();
        };
        let mut child = ui.shifted(start, Some(period));
        (self.children)(&mut child)
    }
}

/// Shows its children frozen at `at` seconds of their local time.
pub struct Freeze<'a> {
    pub at: f32,
    pub children: Children<'a>,
}

impl<'a> Component<'a> for Freeze<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let mut child = ui.shifted(0, ui.duration);
        child.frame.index = ui.frames(self.at);
        (self.children)(&mut child)
    }
}

/// A group covering the whole canvas, optionally on a background colour (Remotion's
/// `AbsoluteFill`).
pub struct Fill<'a> {
    pub background: Option<Color>,
    pub children: Children<'a>,
}

impl<'a> Component<'a> for Fill<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let content = (self.children)(ui);
        let Some(fill) = self.background else {
            return fframes::svgr!(<g>{content}</g>);
        };
        fframes::svgr!(
            <g>
                <rect x="0" y="0" width="100%" height="100%" fill={fill} />
                {content}
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_bounds() {
        // before the start, at the start, last frame, after the end
        assert_eq!(window(9, 10, Some(5)), None);
        assert_eq!(window(10, 10, Some(5)), Some(0));
        assert_eq!(window(14, 10, Some(5)), Some(4));
        assert_eq!(window(15, 10, Some(5)), None);
        assert_eq!(window(1000, 10, None), Some(990));
        assert_eq!(window(3, 0, Some(0)), None);
    }

    #[test]
    fn nested_windows_add_up() {
        // a sequence from frame 30 holding one from frame 15 starts at global frame 45
        let outer = window(50, 30, Some(60)).unwrap();
        assert_eq!(window(outer, 15, Some(30)), Some(5));
        assert_eq!(
            window(44, 30, Some(60)).and_then(|o| window(o, 15, None)),
            None
        );
    }

    #[test]
    fn series_places_items_back_to_back_or_overlapping() {
        assert_eq!(
            series_windows(30, &[1.0, 2.0, 0.5], 0.0),
            vec![(0, 30), (30, 60), (90, 15)]
        );
        assert_eq!(
            series_windows(30, &[1.0, 2.0], 0.5),
            vec![(0, 30), (15, 60)]
        );
    }

    #[test]
    fn loop_wraps_and_stops() {
        assert_eq!(loop_start(0, 30, None), Some(0));
        assert_eq!(loop_start(29, 30, None), Some(0));
        assert_eq!(loop_start(31, 30, None), Some(30));
        assert_eq!(loop_start(59, 30, Some(2)), Some(30));
        assert_eq!(loop_start(60, 30, Some(2)), None);
        assert_eq!(loop_start(5, 0, None), None);
    }
}
