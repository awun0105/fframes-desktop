//! Manim's geometry mobjects. Angles are radians, counterclockwise on screen as in Manim.

use std::f64::consts::{FRAC_PI_2, TAU};

use kurbo::{BezPath, Point, RoundedRect, Shape as _, Vec2};

use super::{Paint, TOLERANCE, pt, shape};

/// The point at `angle` (counterclockwise on screen) and `radius` from `center`.
pub(crate) fn polar(center: Point, radius: f64, angle: f64) -> Point {
    center + Vec2::new(angle.cos(), -angle.sin()) * radius
}

/// An arc counterclockwise on screen from `start` by `sweep`, without the move to its start.
pub(crate) fn arc(center: Point, radius: f64, start: f64, sweep: f64) -> kurbo::Arc {
    kurbo::Arc::new(center, (radius, radius), -start, -sweep, 0.0)
}

/// A closed path through `points`.
pub(crate) fn closed(points: impl IntoIterator<Item = Point>) -> BezPath {
    let mut path = BezPath::new();
    for (i, p) in points.into_iter().enumerate() {
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
}

/// Manim's default first vertex: on the right for an even count, at the top for an odd one.
fn first_vertex(n: usize, start_angle: Option<f32>) -> f64 {
    start_angle.map_or(if n.is_multiple_of(2) { 0.0 } else { FRAC_PI_2 }, f64::from)
}

fn regular(center: Point, radius: f64, n: usize, start: f64) -> BezPath {
    let n = n.max(3);
    closed((0..n).map(|i| polar(center, radius, start + TAU * i as f64 / n as f64)))
}

shape! {
    /// A circle (Manim `Circle`, Remotion `Circle`).
    Circle(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        radius: f32 = 100.0,
    }
    paint: Paint::outline(t);
    path(s) {
        kurbo::Circle::new(pt(s.cx, s.cy), f64::from(s.radius)).to_path(TOLERANCE)
    }
}

shape! {
    /// A small filled circle that marks a point (Manim `Dot`).
    Dot(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        radius: f32 = 8.0,
    }
    paint: Paint::solid(t);
    path(s) {
        kurbo::Circle::new(pt(s.cx, s.cy), f64::from(s.radius)).to_path(TOLERANCE)
    }
}

shape! {
    /// An ellipse in a `width` by `height` box (Manim `Ellipse`, Remotion `Ellipse`).
    Ellipse(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        width: f32 = 200.0,
        height: f32 = 100.0,
    }
    paint: Paint::outline(t);
    path(s) {
        kurbo::Ellipse::new(
            pt(s.cx, s.cy),
            (f64::from(s.width) / 2.0, f64::from(s.height) / 2.0),
            0.0,
        )
        .to_path(TOLERANCE)
    }
}

shape! {
    /// Part of a circle's outline from `start_angle` by `angle` (Manim `Arc`).
    Arc(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        radius: f32 = 100.0,
        start_angle: f32 = 0.0,
        angle: f32 = std::f32::consts::FRAC_PI_2,
    }
    paint: Paint::outline(t);
    path(s) {
        let a = arc(
            pt(s.cx, s.cy),
            f64::from(s.radius),
            f64::from(s.start_angle),
            f64::from(s.angle),
        );
        a.to_path(TOLERANCE)
    }
}

shape! {
    /// A ring between two circles (Manim `Annulus`).
    Annulus(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        inner_radius: f32 = 50.0,
        outer_radius: f32 = 100.0,
    }
    paint: Paint::solid(t);
    path(s) {
        let c = pt(s.cx, s.cy);
        let mut path = kurbo::Circle::new(c, f64::from(s.outer_radius)).to_path(TOLERANCE);
        let inner = kurbo::Circle::new(c, f64::from(s.inner_radius)).to_path(TOLERANCE);
        path.extend(inner.reverse_subpaths().iter());
        path
    }
}

shape! {
    /// A slice of a disc from `start_angle` by `angle` (Manim `Sector`).
    Sector(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        radius: f32 = 100.0,
        start_angle: f32 = 0.0,
        angle: f32 = std::f32::consts::FRAC_PI_2,
    }
    paint: Paint::solid(t);
    path(s) {
        let c = pt(s.cx, s.cy);
        let (r, start) = (f64::from(s.radius), f64::from(s.start_angle));
        let mut path = BezPath::new();
        path.move_to(c);
        path.line_to(polar(c, r, start));
        arc(c, r, start, f64::from(s.angle)).to_cubic_beziers(TOLERANCE, |p1, p2, p| {
            path.curve_to(p1, p2, p);
        });
        path.close_path();
        path
    }
}

shape! {
    /// A straight line from `(x1, y1)` to `(x2, y2)` (Manim `Line`).
    Line(t) {
        x1: f32 = 0.0,
        y1: f32 = 0.0,
        x2: f32 = 200.0,
        y2: f32 = 0.0,
    }
    paint: Paint::outline(t);
    path(s) {
        let mut path = BezPath::new();
        path.move_to(pt(s.x1, s.y1));
        path.line_to(pt(s.x2, s.y2));
        path
    }
}

shape! {
    /// A dashed straight line (Manim `DashedLine`); `dash` is the dash and gap length.
    DashedLine(t) {
        x1: f32 = 0.0,
        y1: f32 = 0.0,
        x2: f32 = 200.0,
        y2: f32 = 0.0,
    }
    paint: Paint::outline(t);
    dash: (12.0, 12.0);
    path(s) {
        let mut path = BezPath::new();
        path.move_to(pt(s.x1, s.y1));
        path.line_to(pt(s.x2, s.y2));
        path
    }
}

shape! {
    /// A closed polygon through `points` (Manim `Polygon`, Remotion `Polygon`).
    Polygon(t) {
        points: Vec<(f32, f32)> = vec![(0.0, -100.0), (100.0, 100.0), (-100.0, 100.0)],
    }
    paint: Paint::outline(t);
    path(s) {
        closed(s.points.iter().map(|&(x, y)| pt(x, y)))
    }
}

shape! {
    /// A regular polygon with `n` corners on a circle of `radius` (Manim `RegularPolygon`).
    /// `start_angle` is the first corner; `None` is Manim's default (right for an even `n`,
    /// top for an odd one).
    RegularPolygon(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        radius: f32 = 100.0,
        n: usize = 6,
        start_angle: Option<f32> = None,
    }
    paint: Paint::outline(t);
    path(s) {
        regular(pt(s.cx, s.cy), f64::from(s.radius), s.n, first_vertex(s.n, s.start_angle))
    }
}

shape! {
    /// An equilateral triangle pointing up, its corners on a circle of `radius` (Manim
    /// `Triangle`, Remotion `Triangle`).
    Triangle(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        radius: f32 = 100.0,
    }
    paint: Paint::outline(t);
    path(s) {
        regular(pt(s.cx, s.cy), f64::from(s.radius), 3, FRAC_PI_2)
    }
}

shape! {
    /// A `width` by `height` rectangle (Manim `Rectangle`).
    Rectangle(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        width: f32 = 400.0,
        height: f32 = 200.0,
    }
    paint: Paint::outline(t);
    path(s) {
        let (w, h) = (f64::from(s.width) / 2.0, f64::from(s.height) / 2.0);
        let c = pt(s.cx, s.cy);
        kurbo::Rect::new(c.x - w, c.y - h, c.x + w, c.y + h).to_path(TOLERANCE)
    }
}

shape! {
    /// A square with sides of `side_length` (Manim `Square`).
    Square(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        side_length: f32 = 200.0,
    }
    paint: Paint::outline(t);
    path(s) {
        let h = f64::from(s.side_length) / 2.0;
        let c = pt(s.cx, s.cy);
        kurbo::Rect::new(c.x - h, c.y - h, c.x + h, c.y + h).to_path(TOLERANCE)
    }
}

shape! {
    /// A rectangle with round corners (Manim `RoundedRectangle`).
    RoundedRectangle(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        width: f32 = 400.0,
        height: f32 = 200.0,
        corner_radius: f32 = t.radius.md,
    }
    paint: Paint::outline(t);
    path(s) {
        let (w, h) = (f64::from(s.width) / 2.0, f64::from(s.height) / 2.0);
        let c = pt(s.cx, s.cy);
        RoundedRect::new(c.x - w, c.y - h, c.x + w, c.y + h, f64::from(s.corner_radius))
            .to_path(TOLERANCE)
    }
}

shape! {
    /// A star with `n` points (Manim `Star`, Remotion `Star`). The first point is at the top;
    /// `inner_radius` is where the edges meet between the points.
    Star(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        n: usize = 5,
        outer_radius: f32 = 100.0,
        inner_radius: f32 = 38.2,
    }
    paint: Paint::outline(t);
    path(s) {
        let c = pt(s.cx, s.cy);
        let n = s.n.max(2);
        closed((0..2 * n).map(|i| {
            let r = if i % 2 == 0 { s.outer_radius } else { s.inner_radius };
            polar(c, f64::from(r), FRAC_PI_2 + TAU * i as f64 / (2 * n) as f64)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::Geometry, shapes::assert_bbox};

    #[test]
    fn round_shapes() {
        let c = Circle {
            cx: 10.0,
            cy: 20.0,
            radius: 5.0,
            ..Circle::default()
        };
        assert_bbox(&c, 5.0, 15.0, 15.0, 25.0);
        assert!((c.length() - std::f64::consts::TAU * 5.0).abs() < 0.05);
        assert_bbox(
            &Dot {
                cx: 1.0,
                ..Dot::default()
            },
            -7.0,
            -8.0,
            9.0,
            8.0,
        );
        assert_bbox(&Ellipse::default(), -100.0, -50.0, 100.0, 50.0);
        assert_bbox(&Annulus::default(), -100.0, -100.0, 100.0, 100.0);
        assert!(Dot::default().fill.is_some() && Circle::default().fill.is_none());
    }

    #[test]
    fn arcs_turn_counterclockwise_on_screen() {
        // a quarter from the right to the top: y goes up the screen, so negative
        assert_bbox(&Arc::default(), 0.0, -100.0, 100.0, 0.0);
        assert_bbox(&Sector::default(), 0.0, -100.0, 100.0, 0.0);
    }

    #[test]
    fn lines() {
        assert_bbox(
            &Line {
                y2: 50.0,
                ..Line::default()
            },
            0.0,
            0.0,
            200.0,
            50.0,
        );
        assert_eq!(DashedLine::default().dash, Some((12.0, 12.0)));
        assert_bbox(&Polygon::default(), -100.0, -100.0, 100.0, 100.0);
    }

    #[test]
    fn polygons() {
        assert_bbox(&RegularPolygon::default(), -100.0, -86.6, 100.0, 86.6);
        // a triangle points up: its top corner is at -radius
        assert_bbox(&Triangle::default(), -86.6, -100.0, 86.6, 50.0);
        assert_bbox(&Rectangle::default(), -200.0, -100.0, 200.0, 100.0);
        assert_bbox(
            &Square {
                cx: 100.0,
                ..Square::default()
            },
            0.0,
            -100.0,
            200.0,
            100.0,
        );
        assert_bbox(&RoundedRectangle::default(), -200.0, -100.0, 200.0, 100.0);
        assert_bbox(&Star::default(), -95.1, -100.0, 95.1, 80.9);
    }
}
