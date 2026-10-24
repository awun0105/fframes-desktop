use std::f64::consts::PI;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle as KCircle, Point, Shape as _, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::draw_animated,
    explain::{Range, ramp, sigmoid_length},
    geometry::Shape,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{arrow, electro::grid_arrow, line};

/// A current-carrying wire, Manim-physics' `Wire`, in units of `ui.space`. `current` is in
/// any unit; only the ratios matter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Wire {
    /// A long straight wire through `(x, y)` at right angles to the page, its current out of
    /// the page when positive.
    Through { x: f64, y: f64, current: f64 },
    /// A straight piece of wire in the page, its current flowing from `from` to `to`.
    Segment {
        from: (f64, f64),
        to: (f64, f64),
        current: f64,
    },
}

/// The magnetic field of `wires` at the point `(x, y)` of the page, `(B_x, B_y, B_z)` with
/// `z` out of the page, by the Biot–Savart law without `μ₀`. A wire through the page gives
/// `I / (2π r)` around it, counterclockwise for a current out of the page; a segment in the
/// page gives `I / (4π d) (sin θ₂ - sin θ₁)` out of or into the page, `d` the distance from
/// its line. On a wire the field is taken as 0.
pub fn magnetic_field(wires: &[Wire], x: f64, y: f64) -> (f64, f64, f64) {
    let mut b = (0.0, 0.0, 0.0);
    for wire in wires {
        match *wire {
            Wire::Through {
                x: wx,
                y: wy,
                current,
            } => {
                let (dx, dy) = (x - wx, y - wy);
                let r2 = dx * dx + dy * dy;
                if r2 > 1e-18 {
                    let s = current / (2.0 * PI * r2);
                    b.0 -= s * dy;
                    b.1 += s * dx;
                }
            }
            Wire::Segment { from, to, current } => {
                let along = Vec2::new(to.0 - from.0, to.1 - from.1);
                let length = along.hypot();
                if length < 1e-12 {
                    continue;
                }
                let u = along / length;
                let p = Vec2::new(x - from.0, y - from.1);
                // signed: positive to the left of the current, where the field comes out
                let d = u.cross(p);
                if d.abs() < 1e-9 {
                    continue;
                }
                let s1 = -p.dot(u);
                let s2 = length + s1;
                let sin = |s: f64| s / s.hypot(d);
                b.2 += current / (4.0 * PI * d) * (sin(s2) - sin(s1));
            }
        }
    }
    b
}

/// Manim-physics' `MagneticField` in the page: on every grid point of `x_range` by
/// `y_range` (units of `ui.space`) the field of `wires`. Where it lies mostly in the page it
/// is an arrow `length(|B|)` units long, otherwise a circle `length(|B|)` units across with a
/// dot (out of the page) or a cross (into it). Both are coloured by `|B|` from `colors[0]` at
/// `min_magnitude` to the last colour at `max_magnitude`. Grid points closer than
/// `clearance` units to a wire are left out. With `show_wires` a wire through the page is a
/// disc with a dot or a cross, a segment is a thick line with an arrow along its current.
/// `wires` is a plain prop, so the field follows wires computed from `ui.seconds()`.
/// Everything fades with `enter` and `exit`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
#[allow(unpredictable_function_pointer_comparisons)] // equal props, not equal functions
pub struct MagneticField {
    pub wires: Vec<Wire>,
    pub x_range: Range,
    pub y_range: Range,
    pub length: fn(f64) -> f64,
    pub colors: Vec<Color>,
    pub min_magnitude: f64,
    pub max_magnitude: f64,
    pub clearance: f64,
    pub show_wires: bool,
    pub wire_color: Color,
    pub wire_radius: f32,
    pub stroke_width: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for MagneticField {
    fn themed(t: &Theme) -> Self {
        Self {
            wires: Vec::new(),
            x_range: Range::new(-7.0, 7.0, 0.5),
            y_range: Range::new(-4.0, 4.0, 0.5),
            length: sigmoid_length,
            colors: vec![t.colors.muted, t.colors.accent, t.colors.highlight],
            min_magnitude: 0.0,
            max_magnitude: 0.5,
            clearance: 0.3,
            show_wires: true,
            wire_color: t.colors.title,
            wire_radius: 22.0,
            stroke_width: t.stroke.thin,
            enter: None,
            exit: None,
        }
    }
}

impl Default for MagneticField {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Distance from `p` to the segment `a`–`b`.
fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.hypot2().max(1e-18)).clamp(0.0, 1.0);
    (p - (a + ab * t)).hypot()
}

/// A circle of `radius` pixels around `at` with a dot in it (`out`) or a cross.
fn out_or_in(at: Point, radius: f64, out: bool, color: Color, width: f32) -> Vec<Shape> {
    let ring = Shape {
        path: KCircle::new(at, radius).to_path(0.1),
        fill: None,
        stroke: Some(color),
        stroke_width: width,
        dash: None,
        opacity: 1.0,
    };
    let mark = if out {
        Shape {
            path: KCircle::new(at, (radius * 0.28).max(1.5)).to_path(0.1),
            fill: Some(color),
            stroke: None,
            ..ring.clone()
        }
    } else {
        let r = radius * 0.6;
        let mut path = BezPath::new();
        path.move_to(at + Vec2::new(-r, -r));
        path.line_to(at + Vec2::new(r, r));
        path.move_to(at + Vec2::new(-r, r));
        path.line_to(at + Vec2::new(r, -r));
        Shape {
            path,
            ..ring.clone()
        }
    };
    vec![ring, mark]
}

impl MagneticField {
    /// The field at `(x, y)`, see [`magnetic_field`].
    pub fn at(&self, x: f64, y: f64) -> (f64, f64, f64) {
        magnetic_field(&self.wires, x, y)
    }

    fn near_a_wire(&self, x: f64, y: f64) -> bool {
        let p = Vec2::new(x, y);
        self.wires.iter().any(|w| match *w {
            Wire::Through { x: wx, y: wy, .. } => (x - wx).hypot(y - wy) < self.clearance,
            Wire::Segment { from, to, .. } => {
                to_segment(p, Vec2::new(from.0, from.1), Vec2::new(to.0, to.1)) < self.clearance
            }
        })
    }
}

impl<'a> Component<'a> for MagneticField {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let mut shapes = Vec::new();
        let ys = self.y_range.ticks();
        for x in self.x_range.ticks() {
            for &y in &ys {
                if self.near_a_wire(x, y) {
                    continue;
                }
                let (bx, by, bz) = self.at(x, y);
                let norm = (bx * bx + by * by + bz * bz).sqrt();
                if !(norm.is_finite() && norm > 0.0) {
                    continue;
                }
                let color = ramp(&self.colors, self.min_magnitude, self.max_magnitude, norm);
                let length = (self.length)(norm);
                if bx.hypot(by) >= bz.abs() {
                    shapes.extend(grid_arrow(
                        space,
                        (x, y),
                        (bx, by),
                        length,
                        color,
                        self.stroke_width,
                    ));
                } else {
                    let radius = length * f64::from(space.unit) / 2.0;
                    let at = space.to_px(x, y);
                    shapes.extend(out_or_in(at, radius, bz > 0.0, color, self.stroke_width));
                }
            }
        }
        if self.show_wires {
            let width = self.stroke_width * 3.0;
            for wire in &self.wires {
                match *wire {
                    Wire::Through { x, y, current } => {
                        let at = space.to_px(x, y);
                        let r = f64::from(self.wire_radius);
                        shapes.push(Shape {
                            path: KCircle::new(at, r).to_path(0.1),
                            fill: Some(ui.theme.colors.background),
                            stroke: None,
                            stroke_width: 0.0,
                            dash: None,
                            opacity: 1.0,
                        });
                        shapes.extend(out_or_in(at, r, current >= 0.0, self.wire_color, width));
                    }
                    Wire::Segment { from, to, current } => {
                        let (a, b) = (space.to_px(from.0, from.1), space.to_px(to.0, to.1));
                        shapes.push(line(ui, a, b, self.wire_color, width));
                        let (a, b) = if current >= 0.0 { (a, b) } else { (b, a) };
                        let mid = a.midpoint(b);
                        let d = b - a;
                        if d.hypot() > 1.0 {
                            let half = d / d.hypot() * 18.0;
                            shapes.push(arrow(ui, mid - half, mid + half, self.wire_color, width));
                        }
                    }
                }
            }
        }
        let field = draw_animated(ui, &shapes, None, None, None);
        fade(ui, field, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_straight_wire_falls_off_as_one_over_r() {
        let wires = [Wire::Through {
            x: 0.5,
            y: -0.5,
            current: 2.0,
        }];
        for r in [0.5, 1.0, 2.0, 4.0] {
            let (bx, by, bz) = magnetic_field(&wires, 0.5 + r, -0.5);
            assert!(bz.abs() < 1e-12);
            assert!(bx.abs() < 1e-12);
            // counterclockwise for a current out of the page: up on the right
            assert!((by - 2.0 / (2.0 * PI * r)).abs() < 1e-12, "{r}");
        }
    }

    #[test]
    fn a_long_segment_approaches_the_infinite_wire() {
        let wires = [Wire::Segment {
            from: (-1000.0, 0.0),
            to: (1000.0, 0.0),
            current: 1.0,
        }];
        for d in [0.5, 1.0, 2.0] {
            let (_, _, above) = magnetic_field(&wires, 0.0, d);
            let (_, _, below) = magnetic_field(&wires, 0.0, -d);
            assert!((above - 1.0 / (2.0 * PI * d)).abs() < 1e-6, "{d}");
            assert!((below + above).abs() < 1e-12);
        }
    }

    #[test]
    fn a_short_segment_gives_its_biot_savart_angle() {
        // from the perpendicular bisector the segment spans ±45°: I/(4πd) · 2 sin 45°
        let wires = [Wire::Segment {
            from: (-1.0, 0.0),
            to: (1.0, 0.0),
            current: 1.0,
        }];
        let (_, _, bz) = magnetic_field(&wires, 0.0, 1.0);
        assert!((bz - 2.0f64.sqrt() / (4.0 * PI)).abs() < 1e-12);
        // on the wire's own line the field is taken as 0
        assert_eq!(magnetic_field(&wires, 3.0, 0.0), (0.0, 0.0, 0.0));
    }
}
