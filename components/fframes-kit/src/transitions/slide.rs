use fframes::{Svgr, Transform};

use crate::{
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Direction, Presentation, Side};

/// The entering item slides in from `direction` and pushes the exiting one out (Remotion's
/// `slide()`). `size` is the area that slides, in view-box units: the canvas, 1920 by 1080
/// unless the video's view box has another size (portrait, 4K).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slide {
    pub direction: Direction,
    pub size: (f32, f32),
}

impl Default for Slide {
    fn default() -> Self {
        Self {
            direction: Direction::FromLeft,
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for Slide {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl Slide {
    /// The offset of `side` at `progress`.
    pub fn offset(&self, progress: f32, side: Side) -> (f32, f32) {
        let (tx, ty) = self.direction.travel();
        let shift = match side {
            Side::Entering => progress - 1.0,
            Side::Exiting => progress,
        };
        (tx * self.size.0 * shift, ty * self.size.1 * shift)
    }
}

impl Presentation for Slide {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        _id: &str,
    ) -> Svgr<'a> {
        let (x, y) = self.offset(progress, side);
        let transform = Transform::translate(x, y);
        fframes::svgr!(<g transform={transform}>{content}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entering_pushes_the_exiting_item_out() {
        let s = Slide::default();
        assert_eq!(s.offset(0.0, Side::Entering), (-1920.0, 0.0));
        assert_eq!(s.offset(0.0, Side::Exiting), (0.0, 0.0));
        // the two stay edge to edge
        let (a, b) = (s.offset(0.3, Side::Entering), s.offset(0.3, Side::Exiting));
        assert!((b.0 - a.0 - 1920.0).abs() < 1e-3);
        let up = Slide {
            direction: Direction::FromBottom,
            ..s
        };
        assert_eq!(up.offset(0.0, Side::Entering), (0.0, 1080.0));
    }
}
