use kurbo::{Affine, Point};

/// Manim's coordinate system on the canvas: units with `y` up around an origin. Every
/// explain component measures in the units of `ui.space` and draws in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Space {
    /// Pixel position of the point `(0, 0)`.
    pub origin: (f32, f32),
    /// Pixels per unit.
    pub unit: f32,
}

impl Space {
    /// Manim's frame on a 1920x1080 canvas: 8 units tall (135 px each), 14.2 wide, the
    /// origin in the middle. Every `Ui` starts with it; a [`Canvas`](super::Canvas) sets
    /// another.
    pub const FULL_HD: Space = Space {
        origin: (960.0, 540.0),
        unit: 135.0,
    };

    /// The space of a `width` x `height` pixel box at `(x, y)` that is `units_tall` units
    /// high, with the origin in its middle.
    pub fn frame(x: f32, y: f32, width: f32, height: f32, units_tall: f32) -> Space {
        Space {
            origin: (x + width / 2.0, y + height / 2.0),
            unit: if units_tall > 0.0 {
                height / units_tall
            } else {
                0.0
            },
        }
    }

    /// The pixel of the point `(x, y)` in units.
    pub fn to_px(self, x: f64, y: f64) -> Point {
        let unit = f64::from(self.unit);
        Point::new(
            f64::from(self.origin.0) + x * unit,
            f64::from(self.origin.1) - y * unit,
        )
    }

    /// The point in units at a pixel, the inverse of [`Space::to_px`]. `(0, 0)` for a space
    /// whose unit is 0.
    pub fn from_px(self, p: Point) -> (f64, f64) {
        let unit = f64::from(self.unit);
        if unit == 0.0 {
            return (0.0, 0.0);
        }
        (
            (p.x - f64::from(self.origin.0)) / unit,
            (f64::from(self.origin.1) - p.y) / unit,
        )
    }

    /// The map from units to pixels, `y` flipped.
    pub fn affine(self) -> Affine {
        let unit = f64::from(self.unit);
        Affine::new([
            unit,
            0.0,
            0.0,
            -unit,
            f64::from(self.origin.0),
            f64::from(self.origin.1),
        ])
    }

    /// A length in units as pixels.
    pub fn px(self, units: f64) -> f64 {
        units * f64::from(self.unit)
    }
}

impl Default for Space {
    fn default() -> Self {
        Self::FULL_HD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_round_trip_through_pixels() {
        let s = Space::FULL_HD;
        assert_eq!(s.to_px(0.0, 0.0), Point::new(960.0, 540.0));
        assert_eq!(s.to_px(1.0, 1.0), Point::new(1095.0, 405.0));
        assert_eq!(s.to_px(0.0, 4.0).y, 0.0);
        for (x, y) in [(0.3, -2.5), (-7.1, 4.0), (12.0, 0.001)] {
            let (bx, by) = s.from_px(s.to_px(x, y));
            assert!((bx - x).abs() < 1e-9 && (by - y).abs() < 1e-9);
        }
        assert_eq!(s.affine() * Point::new(1.0, 1.0), s.to_px(1.0, 1.0));
        let f = Space::frame(100.0, 0.0, 800.0, 400.0, 8.0);
        assert_eq!((f.origin, f.unit), ((500.0, 200.0), 50.0));
        assert_eq!(
            Space::frame(0.0, 0.0, 10.0, 10.0, 0.0).from_px(Point::ZERO),
            (0.0, 0.0)
        );
    }
}
