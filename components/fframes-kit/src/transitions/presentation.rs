use fframes::Svgr;

use crate::ui::Ui;

/// Which of the two items of a transition a presentation draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The item that comes in; it is drawn on top.
    Entering,
    /// The item that goes away.
    Exiting,
}

/// Where a `Slide` or `Wipe` comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    FromLeft,
    FromRight,
    FromTop,
    FromBottom,
}

impl Direction {
    /// The way the entering item moves, as a unit vector (`y` down).
    pub fn travel(self) -> (f32, f32) {
        match self {
            Direction::FromLeft => (1.0, 0.0),
            Direction::FromRight => (-1.0, 0.0),
            Direction::FromTop => (0.0, 1.0),
            Direction::FromBottom => (0.0, -1.0),
        }
    }
}

/// How a transition draws its two items (Remotion's presentations). Both sides get the
/// same `progress`, 0 when the transition starts and 1 when it ends; a spring timing may
/// overshoot 1. `id` is unique to this transition in the frame, for clip paths, masks and
/// gradients.
pub trait Presentation {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a>;
}
