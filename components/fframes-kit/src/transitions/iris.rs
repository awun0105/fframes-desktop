use fframes::Svgr;

use crate::{
    motion::clamp01,
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{
    Presentation, Side,
    presentation::{clipped, reach},
};

/// The entering item is uncovered by a circle that opens from the middle, like a camera iris
/// (Remotion's `iris()`); the exiting item stays in place under it. `size` is the area in
/// view-box units, as for [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Iris {
    pub size: (f32, f32),
}

impl Default for Iris {
    fn default() -> Self {
        Self {
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for Iris {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl Iris {
    /// The radius of the opening at `progress`: 0 at the start, the distance to the corners
    /// at the end.
    pub fn radius(&self, progress: f32) -> f32 {
        let (w, h) = self.size;
        clamp01(progress) * reach((w / 2.0, h / 2.0), self.size)
    }
}

impl Presentation for Iris {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a> {
        if side == Side::Exiting || progress >= 1.0 {
            return content;
        }
        let (cx, cy) = (self.size.0 / 2.0, self.size.1 / 2.0);
        let r = self.radius(progress);
        if r <= 0.0 {
            return Svgr::empty();
        }
        clipped(
            content,
            fframes::svgr!(<circle cx={cx} cy={cy} r={r} />),
            id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_opening_reaches_the_corners() {
        let iris = Iris::default();
        assert_eq!(iris.radius(0.0), 0.0);
        let corner = 960.0_f32.hypot(540.0);
        assert!((iris.radius(1.0) - corner).abs() < 1e-3);
        assert!((iris.radius(0.5) - corner / 2.0).abs() < 1e-3);
        // a spring's overshoot stays at the corners
        assert_eq!(iris.radius(1.2), iris.radius(1.0));
    }
}
