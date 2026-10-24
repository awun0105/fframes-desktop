//! Small drawing helpers the diagram kits share: strokes, arrow tips, bent links between
//! round nodes, partial paths and colour blends.

use fframes::{Color, Svgr, animation::Animatable};
use kurbo::{BezPath, CubicBez, ParamCurve, ParamCurveArclen, Point, Shape as _, Vec2};

use crate::geometry::Shape;

/// Blends two colours, `p` from 0 (`a`) to 1 (`b`).
pub(crate) fn mix(a: Color, b: Color, p: f32) -> Color {
    Animatable::apply_progress(&a, &b, p)
}

pub(crate) fn stroke(path: BezPath, color: Color, width: f32, opacity: f32) -> Shape {
    Shape {
        path,
        fill: None,
        stroke: Some(color),
        stroke_width: width,
        dash: None,
        opacity,
    }
}

/// A filled arrow tip pointing along `dir` (a unit vector) with its point at `at`.
pub(crate) fn tip(at: Point, dir: Vec2, length: f64, color: Color, opacity: f32) -> Shape {
    let side = Vec2::new(-dir.y, dir.x) * (length * 0.45);
    let base = at - dir * length;
    let mut path = BezPath::new();
    path.move_to(at);
    path.line_to(base + side);
    path.line_to(base - side);
    path.close_path();
    Shape {
        path,
        fill: Some(color),
        stroke: None,
        stroke_width: 0.0,
        dash: None,
        opacity,
    }
}

/// The left of the way from `a` to `b` on screen (up when moving right), as a unit vector.
pub(crate) fn left_of(a: Point, b: Point) -> Vec2 {
    let d = b - a;
    let length = d.hypot();
    if length < 1e-9 {
        return Vec2::new(0.0, -1.0);
    }
    Vec2::new(d.y, -d.x) / length
}

/// A link between round nodes at `a` and `b`, leaving and entering them at their edges
/// (`ra`, `rb`) and bent `bend` pixels to the left of its way at the middle.
pub(crate) fn link(a: Point, b: Point, ra: f64, rb: f64, bend: f64) -> CubicBez {
    let control = a.midpoint(b) + left_of(a, b) * (2.0 * bend);
    let towards = |from: Point| {
        let d = control - from;
        if d.hypot() < 1e-9 {
            Vec2::ZERO
        } else {
            d.normalize()
        }
    };
    let start = a + towards(a) * ra;
    let end = b + towards(b) * rb;
    CubicBez::new(
        start,
        start + (control - start) * (2.0 / 3.0),
        end + (control - end) * (2.0 / 3.0),
        end,
    )
}

/// The bend nearest to `base` (in steps of 40 pixels) that keeps the link from `a` to `b`
/// `clearance` pixels away from every point of `obstacles`, so a long edge goes round the
/// nodes in its way instead of through them.
pub(crate) fn clear_bend(
    a: Point,
    b: Point,
    radius: f64,
    obstacles: &[Point],
    clearance: f64,
    base: f64,
) -> f64 {
    let clear = |bend: f64| {
        let curve = link(a, b, radius, radius, bend);
        (1..16).all(|i| {
            let p = curve.eval(f64::from(i) / 16.0);
            obstacles.iter().all(|o| (p - *o).hypot() >= clearance)
        })
    };
    [0.0, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0]
        .into_iter()
        .map(|k| base + k * 40.0)
        .find(|&bend| clear(bend))
        .unwrap_or(base)
}

/// Where the label of a link sits: beside its middle, on the side it bends to.
// only the computer science kit labels its links
#[cfg_attr(not(feature = "cs"), allow(dead_code))]
pub(crate) fn label_point(curve: &CubicBez, offset: f64) -> Point {
    let middle = curve.eval(0.5);
    let side = left_of(curve.p0, curve.p3);
    let bulge = middle - curve.p0.midpoint(curve.p3);
    let sign = if bulge.dot(side) < -1e-6 { -1.0 } else { 1.0 };
    middle + side * (offset * sign)
}

/// The first `fraction` of the polyline `points`, by length.
pub(crate) fn partial(points: &[Point], fraction: f64) -> Vec<Point> {
    let total: f64 = points.windows(2).map(|w| (w[1] - w[0]).hypot()).sum();
    let mut left = total * fraction.clamp(0.0, 1.0);
    let mut out = vec![points[0]];
    for w in points.windows(2) {
        let length = (w[1] - w[0]).hypot();
        if left >= length {
            out.push(w[1]);
            left -= length;
        } else {
            if length > 0.0 {
                out.push(w[0] + (w[1] - w[0]) * (left / length));
            }
            break;
        }
    }
    out
}

/// `curve` drawn as an arrow (with a tip at its end when `head`).
pub(crate) fn arrow_along<'a>(
    curve: CubicBez,
    head: bool,
    color: Color,
    width: f32,
    opacity: f32,
) -> Svgr<'a> {
    let tip_length = 8.0 + f64::from(width) * 2.4;
    let length = curve.arclen(0.5);
    if !head || length <= tip_length {
        return stroke(curve.into_path(0.1), color, width, opacity).draw();
    }
    let t = 1.0 - tip_length / length;
    let shaft = curve.subsegment(0.0..t);
    let dir = (curve.p3 - shaft.p3).normalize();
    let tip = tip(curve.p3, dir, tip_length, color, opacity);
    fframes::svgr!(<g>{stroke(shaft.into_path(0.1), color, width, opacity).draw()}{tip.draw()}</g>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_paths_follow_their_length() {
        let points = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ];
        assert_eq!(
            partial(&points, 0.25),
            [Point::new(0.0, 0.0), Point::new(5.0, 0.0)]
        );
        assert_eq!(
            partial(&points, 0.75),
            [
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 5.0)
            ]
        );
        assert_eq!(partial(&points, 1.0).len(), 3);
    }
}
