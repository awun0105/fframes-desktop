use std::f32::consts::PI;

use fframes::Svgr;
use kurbo::Affine;

use crate::{
    geometry::matrix,
    motion::clamp01,
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Direction, Presentation, Side};

/// The exiting item turns half a turn like a card and the entering item is on its back
/// (Remotion's `flip()`). The turn is drawn flat, as a scale about the middle of `size`, with
/// no perspective: `direction` only picks the axis, a vertical one for `FromLeft` and
/// `FromRight` and a horizontal one for `FromTop` and `FromBottom`. `size` is the area in
/// view-box units, as for [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flip {
    pub direction: Direction,
    pub size: (f32, f32),
}

impl Default for Flip {
    fn default() -> Self {
        Self {
            direction: Direction::FromLeft,
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for Flip {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl Flip {
    /// The width of `side` along the turning axis as a share of its own, at `progress`: the
    /// exiting item narrows to 0 at the middle of the transition, then the entering item
    /// widens from 0. `None` while `side` faces away.
    pub fn squash(&self, progress: f32, side: Side) -> Option<f32> {
        let p = clamp01(progress);
        let facing = match side {
            Side::Exiting => p < 0.5,
            Side::Entering => p >= 0.5,
        };
        facing.then(|| (p * PI).cos().abs())
    }
}

impl Presentation for Flip {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        _id: &str,
    ) -> Svgr<'a> {
        let Some(s) = self.squash(progress, side) else {
            return Svgr::empty();
        };
        if s >= 1.0 {
            return content;
        }
        let (cx, cy) = (f64::from(self.size.0) / 2.0, f64::from(self.size.1) / 2.0);
        let s = f64::from(s);
        let (sx, sy) = match self.direction {
            Direction::FromLeft | Direction::FromRight => (s, 1.0),
            Direction::FromTop | Direction::FromBottom => (1.0, s),
        };
        let turn = Affine::translate((cx, cy))
            * Affine::scale_non_uniform(sx, sy)
            * Affine::translate((-cx, -cy));
        let transform = matrix(turn);
        fframes::svgr!(<g transform={transform}>{content}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_side_faces_the_viewer_at_a_time() {
        let flip = Flip::default();
        assert_eq!(flip.squash(0.0, Side::Exiting), Some(1.0));
        assert_eq!(flip.squash(0.0, Side::Entering), None);
        assert!(flip.squash(0.49, Side::Exiting).unwrap() < 0.05);
        assert_eq!(flip.squash(0.5, Side::Exiting), None);
        assert!(flip.squash(0.5, Side::Entering).unwrap() < 1e-6);
        assert!((flip.squash(1.0, Side::Entering).unwrap() - 1.0).abs() < 1e-6);
        // a spring's overshoot ends flat on the entering side
        assert_eq!(
            flip.squash(1.1, Side::Entering),
            flip.squash(1.0, Side::Entering)
        );
    }
}
