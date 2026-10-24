use fframes::Svgr;

use crate::{
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Presentation, Side};

/// A hard cut in the middle of the transition (Remotion's `none()`). The series still
/// overlaps the two items by the transition's length, so a scene can run its own animation
/// through [`Ui::transition_progress`](crate::ui::Ui::transition_progress); the exiting item
/// shows until half of the progress, the entering item from then on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoTransition;

impl Themed for NoTransition {
    fn themed(_: &Theme) -> Self {
        Self
    }
}

impl NoTransition {
    /// Whether `side` shows at `progress`.
    pub fn shows(progress: f32, side: Side) -> bool {
        match side {
            Side::Exiting => progress < 0.5,
            Side::Entering => progress >= 0.5,
        }
    }
}

impl Presentation for NoTransition {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        _id: &str,
    ) -> Svgr<'a> {
        if Self::shows(progress, side) {
            content
        } else {
            Svgr::empty()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_in_the_middle() {
        for p in [0.0, 0.2, 0.49] {
            assert!(NoTransition::shows(p, Side::Exiting));
            assert!(!NoTransition::shows(p, Side::Entering));
        }
        for p in [0.5, 0.8, 1.0] {
            assert!(!NoTransition::shows(p, Side::Exiting));
            assert!(NoTransition::shows(p, Side::Entering));
        }
    }
}
