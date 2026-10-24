use std::f64::consts::{FRAC_PI_2, TAU};

use fframes::Svgr;
use kurbo::{Arc, BezPath, Point, Shape as _, Vec2};

use crate::{
    geometry::path_data,
    motion::clamp01,
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{
    Presentation, Side,
    presentation::{clipped, reach},
};

/// The exiting item is wiped away by a hand that sweeps clockwise from twelve o'clock round
/// the middle, uncovering the entering item (Remotion's `clockWipe()`). `size` is the area in
/// view-box units, as for [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockWipe {
    pub size: (f32, f32),
}

impl Default for ClockWipe {
    fn default() -> Self {
        Self {
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for ClockWipe {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl ClockWipe {
    /// The uncovered wedge at `progress`, from the middle to past the corners.
    pub fn wedge(&self, progress: f32) -> BezPath {
        let (w, h) = (f64::from(self.size.0), f64::from(self.size.1));
        let center = Point::new(w / 2.0, h / 2.0);
        let r = f64::from(reach((self.size.0 / 2.0, self.size.1 / 2.0), self.size)) + 1.0;
        let sweep = f64::from(clamp01(progress)) * TAU;
        let mut path = BezPath::new();
        path.move_to(center);
        // y points down, so a positive sweep from -90° turns clockwise from twelve o'clock
        path.line_to(center + Vec2::new(0.0, -r));
        let arc = Arc::new(center, (r, r), -FRAC_PI_2, sweep, 0.0);
        arc.path_elements(0.1).skip(1).for_each(|el| path.push(el));
        path.close_path();
        path
    }
}

impl Presentation for ClockWipe {
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
        if progress <= 0.0 {
            return Svgr::empty();
        }
        let d = path_data(&self.wedge(progress));
        clipped(content, fframes::svgr!(<path d={d} />), id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wedge_sweeps_clockwise_from_twelve() {
        let clock = ClockWipe::default();
        // a quarter: from twelve to three o'clock, the top right of the frame
        let quarter = clock.wedge(0.25);
        assert!(quarter.contains(Point::new(1400.0, 200.0)));
        assert!(!quarter.contains(Point::new(500.0, 200.0)));
        assert!(!quarter.contains(Point::new(1400.0, 900.0)));
        // three quarters leave only the top left
        let most = clock.wedge(0.75);
        assert!(most.contains(Point::new(500.0, 900.0)));
        assert!(!most.contains(Point::new(500.0, 200.0)));
        // the whole turn covers the corners
        let full = clock.wedge(0.999);
        for corner in [(1.0, 1.0), (1919.0, 1.0), (1.0, 1079.0), (1919.0, 1079.0)] {
            assert!(full.contains(Point::new(corner.0, corner.1)));
        }
    }
}
