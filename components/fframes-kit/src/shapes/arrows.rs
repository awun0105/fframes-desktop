//! Manim's arrows: an outlined shaft with tips (a filled triangle by default) at its ends.

use kurbo::{BezPath, Point, Shape as _, Vec2};

use super::{Paint, TOLERANCE, basic::ChordArc, pt, shape};
use crate::geometry::Shape;

/// The tip at an end of an arrow (Manim's tip classes, `ArrowTriangleFilledTip` by default).
/// A tip is `tip_length` long from its point back and `tip_width` wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum ArrowTip {
    /// `ArrowTriangleTip`: a triangle outline.
    Triangle,
    /// `ArrowTriangleFilledTip`.
    #[default]
    TriangleFilled,
    /// `ArrowCircleTip`: a circle outline, `tip_length` across.
    Circle,
    /// `ArrowCircleFilledTip`.
    CircleFilled,
    /// `ArrowSquareTip`: a square outline with sides of `tip_length`.
    Square,
    /// `ArrowSquareFilledTip`.
    SquareFilled,
    /// `StealthTip`: a filled arrowhead with a notch in its back.
    Stealth,
}

/// How far into a stealth tip its notch reaches, as a share of the tip's length.
const STEALTH_NOTCH: f64 = 0.6;

impl ArrowTip {
    /// Whether the tip is filled, or only outlined.
    pub fn filled(self) -> bool {
        matches!(
            self,
            Self::TriangleFilled | Self::CircleFilled | Self::SquareFilled | Self::Stealth
        )
    }

    /// Where the shaft stops behind the point, as a share of the tip's length.
    fn back(self) -> f64 {
        match self {
            Self::Stealth => STEALTH_NOTCH,
            _ => 1.0,
        }
    }

    /// Appends the tip, a closed subpath, with its point at `point` and pointing along `dir`
    /// (a unit vector).
    fn append(self, path: &mut BezPath, point: Point, dir: Vec2, length: f64, width: f64) {
        let side = Vec2::new(-dir.y, dir.x);
        let base = point - dir * length;
        match self {
            Self::Triangle | Self::TriangleFilled => {
                path.move_to(base + side * (width / 2.0));
                path.line_to(point);
                path.line_to(base - side * (width / 2.0));
                path.close_path();
            }
            Self::Circle | Self::CircleFilled => {
                let circle = kurbo::Circle::new(point.midpoint(base), length / 2.0);
                path.extend(circle.path_elements(TOLERANCE));
            }
            Self::Square | Self::SquareFilled => {
                let half = side * (length / 2.0);
                path.move_to(point + half);
                path.line_to(point - half);
                path.line_to(base - half);
                path.line_to(base + half);
                path.close_path();
            }
            Self::Stealth => {
                path.move_to(point);
                path.line_to(base + side * (width / 2.0));
                path.line_to(point - dir * (length * STEALTH_NOTCH));
                path.line_to(base - side * (width / 2.0));
                path.close_path();
            }
        }
    }
}

/// A shaft from `from` to `to` with a filled triangle at the end, and at the start when
/// `start` is set.
pub(crate) fn straight(
    from: Point,
    to: Point,
    tip_length: f64,
    tip_width: f64,
    start: bool,
) -> BezPath {
    let start = start.then_some(ArrowTip::TriangleFilled);
    tipped(
        from,
        to,
        tip_length,
        tip_width,
        start,
        Some(ArrowTip::TriangleFilled),
    )
}

/// A shaft from `from` to `to` with the tips that are set. The shaft stops at the tips so
/// its round cap does not show through them. Subpaths: the shaft, the end tip, the start tip.
pub(crate) fn tipped(
    from: Point,
    to: Point,
    tip_length: f64,
    tip_width: f64,
    tip_start: Option<ArrowTip>,
    tip_end: Option<ArrowTip>,
) -> BezPath {
    let mut path = BezPath::new();
    let d = to - from;
    let length = d.hypot();
    if length < 1e-9 {
        path.move_to(from);
        return path;
    }
    let dir = d / length;
    let tips = usize::from(tip_start.is_some()) + usize::from(tip_end.is_some());
    let tip_length = tip_length.min(length / tips.max(1) as f64);
    let back = |tip: Option<ArrowTip>| tip.map_or(0.0, |t| t.back() * tip_length);
    let head = to - dir * back(tip_end);
    let tail = from + dir * back(tip_start);
    path.move_to(tail);
    path.line_to(head);
    if let Some(t) = tip_end {
        t.append(&mut path, to, dir, tip_length, tip_width);
    }
    if let Some(t) = tip_start {
        t.append(&mut path, from, -dir, tip_length, tip_width);
    }
    path
}

/// The parts an arrow is drawn in: its shaft, an outline only, then each tip, filled or not.
/// A straight arrow whose tips are all filled stays one path; one path would fill the inside
/// of a curved shaft or an outline tip.
pub(crate) fn arrow_parts(
    shape: Shape,
    tip_start: Option<ArrowTip>,
    tip_end: Option<ArrowTip>,
    straight_shaft: bool,
) -> Vec<Shape> {
    let tips: Vec<ArrowTip> = [tip_end, tip_start].into_iter().flatten().collect();
    if straight_shaft && tips.iter().all(|t| t.filled()) {
        return vec![shape];
    }
    let subpaths = crate::paths::get_subpaths(&shape.path);
    if subpaths.len() != tips.len() + 1 {
        return vec![shape];
    }
    subpaths
        .into_iter()
        .enumerate()
        .map(|(i, path)| {
            let filled = i > 0 && tips[i - 1].filled();
            Shape {
                path,
                fill: if filled { shape.fill } else { None },
                ..shape.clone()
            }
        })
        .collect()
}

shape! {
    /// An arrow from `(x1, y1)` to `(x2, y2)` (Manim `Arrow`).
    Arrow(t) {
        x1: f32 = 0.0,
        y1: f32 = 0.0,
        x2: f32 = 200.0,
        y2: f32 = 0.0,
        tip_length: f32 = 24.0,
        tip_width: f32 = 20.0,
        tip_start: Option<ArrowTip> = None,
        tip_end: Option<ArrowTip> = Some(ArrowTip::TriangleFilled),
    }
    paint: Paint::arrow(t);
    path(s) {
        tipped(
            pt(s.x1, s.y1),
            pt(s.x2, s.y2),
            f64::from(s.tip_length),
            f64::from(s.tip_width),
            s.tip_start,
            s.tip_end,
        )
    }
    parts(s) {
        arrow_parts(s.shape(), s.tip_start, s.tip_end, true)
    }
}

shape! {
    /// An arrow from `(x, y)` by `(dx, dy)` (Manim `Vector`, which starts at the origin).
    Vector(t) {
        x: f32 = 0.0,
        y: f32 = 0.0,
        dx: f32 = 200.0,
        dy: f32 = 0.0,
        tip_length: f32 = 24.0,
        tip_width: f32 = 20.0,
        tip_start: Option<ArrowTip> = None,
        tip_end: Option<ArrowTip> = Some(ArrowTip::TriangleFilled),
    }
    paint: Paint::arrow(t);
    path(s) {
        tipped(
            pt(s.x, s.y),
            pt(s.x + s.dx, s.y + s.dy),
            f64::from(s.tip_length),
            f64::from(s.tip_width),
            s.tip_start,
            s.tip_end,
        )
    }
    parts(s) {
        arrow_parts(s.shape(), s.tip_start, s.tip_end, true)
    }
}

shape! {
    /// An arrow with a tip at both ends (Manim `DoubleArrow`).
    DoubleArrow(t) {
        x1: f32 = 0.0,
        y1: f32 = 0.0,
        x2: f32 = 200.0,
        y2: f32 = 0.0,
        tip_length: f32 = 24.0,
        tip_width: f32 = 20.0,
        tip_start: Option<ArrowTip> = Some(ArrowTip::TriangleFilled),
        tip_end: Option<ArrowTip> = Some(ArrowTip::TriangleFilled),
    }
    paint: Paint::arrow(t);
    path(s) {
        tipped(
            pt(s.x1, s.y1),
            pt(s.x2, s.y2),
            f64::from(s.tip_length),
            f64::from(s.tip_width),
            s.tip_start,
            s.tip_end,
        )
    }
    parts(s) {
        arrow_parts(s.shape(), s.tip_start, s.tip_end, true)
    }
}

/// An arrow along the arc from `from` to `to` that turns by `angle`, with the tips that are
/// set. The arc stops at the tips; each tip points along the chord from where the arc stops.
pub(crate) fn curved(
    from: Point,
    to: Point,
    angle: f64,
    (tip_length, tip_width): (f64, f64),
    tip_start: Option<ArrowTip>,
    tip_end: Option<ArrowTip>,
) -> BezPath {
    let Some(a) = ChordArc::new(from, to, angle) else {
        return tipped(from, to, tip_length, tip_width, tip_start, tip_end);
    };
    let ends = 1.0 + f64::from(u8::from(tip_start.is_some()) + u8::from(tip_end.is_some()));
    let turn_of = |tip: Option<ArrowTip>| {
        tip.map_or(0.0, |t| {
            (t.back() * tip_length / a.radius).min(angle.abs() / ends) * angle.signum()
        })
    };
    let (first, last) = (turn_of(tip_start), turn_of(tip_end));
    let mut path = a.path(first, angle - last - first);
    let mut put = |tip: Option<ArrowTip>, back: Point, point: Point| {
        if let Some(t) = tip {
            let d = point - back;
            if d.hypot() > 1e-9 {
                t.append(
                    &mut path,
                    point,
                    d.normalize(),
                    d.hypot() / t.back(),
                    tip_width,
                );
            }
        }
    };
    put(tip_end, a.point(angle - last), to);
    put(tip_start, a.point(first), from);
    path
}

shape! {
    /// An arrow along a circular arc that turns by `angle` from `(x1, y1)` to `(x2, y2)`,
    /// counterclockwise on screen for a positive angle (Manim `CurvedArrow`).
    CurvedArrow(t) {
        x1: f32 = 0.0,
        y1: f32 = 0.0,
        x2: f32 = 200.0,
        y2: f32 = 0.0,
        angle: f32 = std::f32::consts::FRAC_PI_2,
        tip_length: f32 = 24.0,
        tip_width: f32 = 20.0,
        tip_start: Option<ArrowTip> = None,
        tip_end: Option<ArrowTip> = Some(ArrowTip::TriangleFilled),
    }
    paint: Paint::arrow(t);
    path(s) {
        curved(
            pt(s.x1, s.y1),
            pt(s.x2, s.y2),
            f64::from(s.angle),
            (f64::from(s.tip_length), f64::from(s.tip_width)),
            s.tip_start,
            s.tip_end,
        )
    }
    parts(s) {
        arrow_parts(s.shape(), s.tip_start, s.tip_end, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::Geometry, shapes::assert_bbox};

    #[test]
    fn arrow_tip_ends_at_the_point() {
        assert_bbox(&Arrow::default(), 0.0, -10.0, 200.0, 10.0);
        assert_bbox(
            &Vector {
                dy: 100.0,
                dx: 0.0,
                ..Vector::default()
            },
            -10.0,
            0.0,
            10.0,
            100.0,
        );
        assert_bbox(&DoubleArrow::default(), 0.0, -10.0, 200.0, 10.0);
        // shaft, then the tip: two subpaths
        assert_eq!(
            crate::paths::get_subpaths(&Arrow::default().path()).len(),
            2
        );
    }

    #[test]
    fn curved_arrow_turns_like_manim() {
        // counterclockwise from left to right passes below, as Manim's arc does; a quarter
        // turn over a 200 px chord sags 100 * (sqrt 2 - 1) ≈ 41 px
        let a = CurvedArrow::default();
        let b = a.bbox();
        assert!(b.y1 > 38.0 && b.y1 < 44.0 && b.y0 > -12.0, "{b:?}");
        assert!(b.x0.abs() < 1.0 && (b.x1 - 200.0).abs() < 12.0, "{b:?}");
        let up = CurvedArrow {
            angle: -std::f32::consts::FRAC_PI_2,
            ..a
        }
        .bbox();
        assert!(up.y0 < -38.0, "{up:?}");
    }

    #[test]
    fn tips_end_at_the_point_and_outline_tips_stay_unfilled() {
        for tip in [
            ArrowTip::Triangle,
            ArrowTip::TriangleFilled,
            ArrowTip::Circle,
            ArrowTip::CircleFilled,
            ArrowTip::Square,
            ArrowTip::SquareFilled,
            ArrowTip::Stealth,
        ] {
            let a = Arrow {
                tip_end: Some(tip),
                tip_start: Some(tip),
                ..Arrow::default()
            };
            let b = a.bbox();
            assert!(
                b.x0.abs() < 0.5 && (b.x1 - 200.0).abs() < 0.5,
                "{tip:?}: {b:?}"
            );
            let parts = arrow_parts(a.shape(), a.tip_start, a.tip_end, true);
            let fills: Vec<bool> = parts.iter().map(|p| p.fill.is_some()).collect();
            if tip.filled() {
                assert_eq!(parts.len(), 1, "{tip:?}");
            } else {
                assert_eq!(fills, [false, false, false], "{tip:?}");
            }
        }
        // the shaft of a stealth tip reaches into its notch
        let stealth = Arrow {
            tip_end: Some(ArrowTip::Stealth),
            ..Arrow::default()
        };
        let shaft = crate::paths::get_subpaths(&stealth.path())[0].bounding_box();
        assert!(
            (shaft.x1 - (200.0 - 24.0 * STEALTH_NOTCH)).abs() < 1e-6,
            "{shaft:?}"
        );
    }

    #[test]
    fn default_tips_keep_one_path() {
        // the filled triangle stays the default: a straight arrow is one shape
        let a = Arrow::default();
        assert_eq!(
            arrow_parts(a.shape(), a.tip_start, a.tip_end, true).len(),
            1
        );
        let c = CurvedArrow::default();
        assert_eq!(
            arrow_parts(c.shape(), c.tip_start, c.tip_end, false).len(),
            2
        );
    }
}
