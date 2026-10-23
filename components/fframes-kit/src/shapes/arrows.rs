//! Manim's arrows: an outlined shaft with a filled triangular tip, in one path.

use kurbo::{BezPath, Point, Shape as _, Vec2};

use super::{
    Paint, TOLERANCE,
    basic::{arc, polar},
    pt, shape,
};

/// Appends a closed tip whose point is `point` and whose base is centered on `base`.
fn tip(path: &mut BezPath, base: Point, point: Point, width: f64) {
    let dir = point - base;
    let Some(dir) = (dir.hypot() > 1e-9).then(|| dir.normalize()) else {
        return;
    };
    let side = Vec2::new(-dir.y, dir.x) * (width / 2.0);
    path.move_to(base + side);
    path.line_to(point);
    path.line_to(base - side);
    path.close_path();
}

/// A shaft from `from` to `to` with tips at the ends that are set. The shaft stops at the
/// tips so its round cap does not show through them.
fn straight(from: Point, to: Point, tip_length: f64, tip_width: f64, start: bool) -> BezPath {
    let mut path = BezPath::new();
    let d = to - from;
    let length = d.hypot();
    if length < 1e-9 {
        path.move_to(from);
        return path;
    }
    let dir = d / length;
    let tip_length = tip_length.min(length / if start { 2.0 } else { 1.0 });
    let head = to - dir * tip_length;
    let tail = if start { from + dir * tip_length } else { from };
    path.move_to(tail);
    path.line_to(head);
    tip(&mut path, head, to, tip_width);
    if start {
        tip(&mut path, tail, from, tip_width);
    }
    path
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
    }
    paint: Paint::arrow(t);
    path(s) {
        straight(
            pt(s.x1, s.y1),
            pt(s.x2, s.y2),
            f64::from(s.tip_length),
            f64::from(s.tip_width),
            false,
        )
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
    }
    paint: Paint::arrow(t);
    path(s) {
        straight(
            pt(s.x, s.y),
            pt(s.x + s.dx, s.y + s.dy),
            f64::from(s.tip_length),
            f64::from(s.tip_width),
            false,
        )
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
    }
    paint: Paint::arrow(t);
    path(s) {
        straight(
            pt(s.x1, s.y1),
            pt(s.x2, s.y2),
            f64::from(s.tip_length),
            f64::from(s.tip_width),
            true,
        )
    }
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
    }
    paint: Paint::arrow(t);
    path(s) {
        let (from, to) = (pt(s.x1, s.y1), pt(s.x2, s.y2));
        let (tip_length, tip_width) = (f64::from(s.tip_length), f64::from(s.tip_width));
        let theta = f64::from(s.angle);
        let chord = to - from;
        if theta.abs() < 1e-4 || chord.hypot() < 1e-9 {
            return straight(from, to, tip_length, tip_width, false);
        }
        let radius = chord.hypot() / (2.0 * (theta / 2.0).sin().abs());
        // a counterclockwise turn has the center on the left of the chord on screen
        let left = Vec2::new(chord.y, -chord.x).normalize();
        let center = from.midpoint(to) + left * (chord.hypot() / 2.0 / (theta / 2.0).tan());
        let start = (-(from.y - center.y)).atan2(from.x - center.x);
        let tip_turn = (tip_length / radius).min(theta.abs() / 2.0) * theta.signum();
        let mut path = arc(center, radius, start, theta - tip_turn).to_path(TOLERANCE);
        tip(&mut path, polar(center, radius, start + theta - tip_turn), to, tip_width);
        path
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
}
