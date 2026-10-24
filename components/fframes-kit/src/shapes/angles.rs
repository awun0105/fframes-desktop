//! Manim's angle marks: `Angle`, `RightAngle`, `Elbow`, and `TangentLine` for a path.

use std::f64::consts::TAU;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Shape as _, Vec2};

use super::{
    Paint, SMALL_BUFF, TOLERANCE,
    basic::arc,
    labeled::{Anims, ShapeLabel, draw_labeled, place},
    pt, shape,
};
use crate::{
    animate::{Emphasis, EnterAnim, ExitAnim},
    geometry::Shape,
    paths,
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// Manim's default angle radius, 0.4 units on a 1080 px tall canvas.
const ANGLE_RADIUS: f32 = 54.0;

/// A line given by two points.
pub type Segment = ((f32, f32), (f32, f32));

/// The angle of `v` counterclockwise on screen from the right, in `-π..=π` (Manim's
/// `angle_of_vector` with `y` up).
fn screen_angle(v: Vec2) -> f64 {
    (-v.y).atan2(v.x)
}

/// Where two lines cross, `None` when they are parallel.
pub(super) fn crossing(a: (Point, Point), b: (Point, Point)) -> Option<Point> {
    let (da, db) = (a.1 - a.0, b.1 - b.0);
    let det = da.cross(db);
    if det.abs() < 1e-9 {
        return None;
    }
    let t = (b.0 - a.0).cross(db) / det;
    Some(a.0 + da * t)
}

fn points(s: Segment) -> (Point, Point) {
    (pt(s.0.0, s.0.1), pt(s.1.0, s.1.1))
}

/// The mark of an angle: where the lines cross, where the mark meets each line, its start and
/// turn (counterclockwise on screen, negative for clockwise).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Mark {
    corner: Point,
    anchors: (Point, Point),
    start: f64,
    turn: f64,
}

/// Manim's `Angle` geometry between `line1` and `line2`.
fn mark(
    line1: Segment,
    line2: Segment,
    radius: f32,
    quadrant: (f32, f32),
    other: bool,
) -> Option<Mark> {
    let (a, b) = (points(line1), points(line2));
    let corner = crossing(a, b)?;
    let unit = |l: (Point, Point), k: f32| {
        let d = l.1 - l.0;
        (d.hypot() > 1e-9).then(|| d.normalize() * f64::from(k.signum()) * f64::from(radius))
    };
    let (u1, u2) = (unit(a, quadrant.0)?, unit(b, quadrant.1)?);
    let (a1, a2) = (screen_angle(u1), screen_angle(u2));
    let turn = match (other, a2 > a1) {
        (false, true) | (true, false) => a2 - a1,
        (false, false) => TAU - (a1 - a2),
        (true, true) => -TAU + (a2 - a1),
    };
    Some(Mark {
        corner,
        anchors: (corner + u1, corner + u2),
        start: a1,
        turn,
    })
}

impl Mark {
    /// The arc, or the corner of a square when `elbow` is set.
    fn path(&self, radius: f32, elbow: bool) -> BezPath {
        let (p1, p2) = self.anchors;
        let mut path = BezPath::new();
        if elbow {
            path.move_to(p1);
            path.line_to(p1 + (p2 - self.corner));
            path.line_to(p2);
        } else {
            path.move_to(p1);
            arc(self.corner, f64::from(radius), self.start, self.turn)
                .to_cubic_beziers(TOLERANCE, |a, b, c| path.curve_to(a, b, c));
        }
        path
    }

    /// The unit direction from the corner to the middle of the mark.
    fn middle(&self, path: &BezPath) -> Vec2 {
        let mid = paths::get_point_at_length(path, paths::get_length(path) / 2.0);
        mid.map(|m| m - self.corner)
            .filter(|d| d.hypot() > 1e-9)
            .map_or(Vec2::new(1.0, 0.0), Vec2::normalize)
    }
}

/// `Angle` and `RightAngle`: the same props with another default for `elbow`.
macro_rules! angle {
    ($(#[$meta:meta])* $name:ident, elbow: $elbow:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        #[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
        pub struct $name<'a> {
            pub line1: Segment,
            pub line2: Segment,
            pub radius: f32,
            /// Which half of each line the mark sits on: `1` along its direction, `-1` against.
            pub quadrant: (f32, f32),
            /// Marks the other angle, clockwise from the first line to the second.
            pub other_angle: bool,
            /// A square corner instead of an arc.
            pub elbow: bool,
            /// A dot inside the mark, `dot_distance` of the way out from the corner.
            pub dot: bool,
            pub dot_radius: f32,
            pub dot_distance: f32,
            pub dot_color: Color,
            /// A label beside the mark, `label_buff` beyond its radius.
            pub label: ShapeLabel<'a>,
            pub label_buff: f32,
            pub label_style: TextStyle,
            pub label_color: Color,
            pub stroke: Option<Color>,
            pub stroke_width: f32,
            pub opacity: f32,
            pub enter: Option<EnterAnim>,
            pub exit: Option<ExitAnim>,
            pub emphasis: Option<Emphasis>,
        }

        impl Themed for $name<'_> {
            fn themed(t: &Theme) -> Self {
                Self {
                    line1: ((0.0, 0.0), (200.0, 0.0)),
                    line2: ((0.0, 0.0), (0.0, -200.0)),
                    radius: ANGLE_RADIUS,
                    quadrant: (1.0, 1.0),
                    other_angle: false,
                    elbow: $elbow,
                    dot: false,
                    dot_radius: 5.0,
                    dot_distance: 0.55,
                    dot_color: t.colors.title,
                    label: ShapeLabel::None,
                    label_buff: 3.0 * SMALL_BUFF,
                    label_style: t.typography.body,
                    label_color: t.colors.title,
                    stroke: Some(t.colors.accent),
                    stroke_width: t.stroke.regular,
                    opacity: 1.0,
                    enter: None,
                    exit: None,
                    emphasis: None,
                }
            }
        }

        impl Default for $name<'_> {
            fn default() -> Self {
                Self::themed(&Theme::default())
            }
        }

        impl $name<'_> {
            fn mark(&self) -> Option<Mark> {
                mark(self.line1, self.line2, self.radius, self.quadrant, self.other_angle)
            }

            /// The angle the mark spans in radians, always positive (Manim `get_value`).
            pub fn value(&self) -> f32 {
                self.mark().map_or(0.0, |m| m.turn.abs() as f32)
            }

            /// The mark alone: an arc, or a square corner.
            pub fn path(&self) -> BezPath {
                self.mark()
                    .map(|m| m.path(self.radius, self.elbow))
                    .unwrap_or_default()
            }
        }

        impl<'a> Component<'a> for $name<'a> {
            fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
                let Some(m) = self.mark() else {
                    return Svgr::empty();
                };
                let anims = Anims {
                    enter: self.enter.as_ref(),
                    exit: self.exit.as_ref(),
                    emphasis: self.emphasis.as_ref(),
                };
                let path = m.path(self.radius, self.elbow);
                let out = m.middle(&path);
                let r = f64::from(self.radius);
                let mut parts = vec![Shape {
                    path,
                    fill: None,
                    stroke: self.stroke,
                    stroke_width: self.stroke_width,
                    dash: None,
                    opacity: self.opacity,
                }];
                if self.dot {
                    // an elbow's middle is its corner, further out than an arc's
                    let reach = if self.elbow { r * std::f64::consts::SQRT_2 } else { r };
                    let at = m.corner + out * reach * f64::from(self.dot_distance);
                    parts.push(Shape {
                        path: kurbo::Circle::new(at, f64::from(self.dot_radius)).to_path(TOLERANCE),
                        fill: Some(self.dot_color),
                        stroke: None,
                        stroke_width: 0.0,
                        dash: None,
                        opacity: self.opacity,
                    });
                }
                let at = m.corner + out * (r + f64::from(self.label_buff));
                let label = place(ui, self.label, self.label_style, self.label_color, at, anims);
                draw_labeled(ui, parts, label, false, anims)
            }
        }
    };
}

angle! {
    /// The mark of the angle between two lines (Manim `Angle`): an arc of `radius` about
    /// where they cross, counterclockwise on screen from the first line to the second.
    Angle, elbow: false
}

angle! {
    /// The square mark of a right angle between two lines (Manim `RightAngle`).
    RightAngle, elbow: true
}

shape! {
    /// Two lines that meet at a right angle, an L of `width` with its corner up and to the
    /// right of `(x, y)`, turned by `angle` about it (Manim `Elbow`).
    Elbow(t) {
        x: f32 = 0.0,
        y: f32 = 0.0,
        width: f32 = 27.0,
        angle: f32 = 0.0,
    }
    paint: Paint::outline(t);
    path(s) {
        let w = f64::from(s.width);
        let at = kurbo::Affine::translate((f64::from(s.x), f64::from(s.y)))
            * kurbo::Affine::rotate(-f64::from(s.angle));
        let mut path = BezPath::new();
        path.move_to((0.0, -w));
        path.line_to((w, -w));
        path.line_to((w, 0.0));
        at * path
    }
}

shape! {
    /// A line of `length` touching `path` at `alpha` of its length (Manim `TangentLine`). On
    /// axes, `kits::math::Tangent` draws the tangent of a function.
    TangentLine(t) {
        path: BezPath = BezPath::new(),
        alpha: f32 = 0.5,
        length: f32 = 135.0,
    }
    paint: Paint::outline(t);
    path(s) {
        let at = paths::get_length(&s.path) * f64::from(s.alpha.clamp(0.0, 1.0));
        let mut line = BezPath::new();
        let (Some(p), Some(d)) = (
            paths::get_point_at_length(&s.path, at),
            paths::get_tangent_at_length(&s.path, at),
        ) else {
            return line;
        };
        let half = d * f64::from(s.length) / 2.0;
        line.move_to(p - half);
        line.line_to(p + half);
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::assert_bbox;
    use std::f32::consts::{FRAC_PI_2, PI};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn angle_arcs_in_all_four_quadrants() {
        // the first line to the right, the second up the screen: Manim's x and y axes
        let r = f64::from(ANGLE_RADIUS);
        let at = |quadrant: (f32, f32), other_angle: bool| Angle {
            quadrant,
            other_angle,
            ..Angle::default()
        };
        // counterclockwise from the first line to the second, as Manim
        let a = at((1.0, 1.0), false);
        assert!(close(a.value(), FRAC_PI_2));
        assert_bbox(&a.path(), 0.0, -r, r, 0.0);
        let b = at((-1.0, 1.0), true);
        assert!(close(b.value(), FRAC_PI_2));
        assert_bbox(&b.path(), -r, -r, 0.0, 0.0);
        let c = at((-1.0, -1.0), false);
        assert!(close(c.value(), FRAC_PI_2));
        assert_bbox(&c.path(), -r, 0.0, 0.0, r);
        let d = at((1.0, -1.0), true);
        assert!(close(d.value(), FRAC_PI_2));
        assert_bbox(&d.path(), 0.0, 0.0, r, r);
        // the other way round the same quadrant is the reflex angle
        assert!(close(at((-1.0, 1.0), false).value(), 1.5 * PI));
        assert!(close(at((1.0, 1.0), true).value(), 1.5 * PI));
    }

    #[test]
    fn right_angle_is_a_square_corner() {
        let r = f64::from(ANGLE_RADIUS);
        let right = RightAngle::default();
        assert_bbox(&right.path(), 0.0, -r, r, 0.0);
        assert_eq!(right.path().elements().len(), 3);
    }

    #[test]
    fn elbow_and_tangent_line() {
        assert_bbox(&Elbow::default(), 0.0, -27.0, 27.0, 0.0);
        let circle = kurbo::Circle::new((0.0, 0.0), 100.0).to_path(0.01);
        // a quarter of the way round from the right, clockwise on screen: the bottom
        let t = TangentLine {
            path: circle,
            alpha: 0.25,
            ..TangentLine::default()
        };
        assert_bbox(&t, -67.5, 100.0, 67.5, 100.0);
    }
}
