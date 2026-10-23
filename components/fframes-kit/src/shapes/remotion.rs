//! Remotion's `@remotion/shapes` that Manim has no name for. They are filled by default.

use std::f64::consts::{FRAC_PI_2, TAU};

use kurbo::{BezPath, Point, RoundedRect, Shape as _, Vec2};

use super::{
    Paint, TOLERANCE,
    basic::{arc, closed, polar},
    pt, shape,
};

shape! {
    /// A rectangle, square corners unless `corner_radius` is set (Remotion `Rect`).
    Rect(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        width: f32 = 200.0,
        height: f32 = 200.0,
        corner_radius: f32 = 0.0,
    }
    paint: Paint::solid(t);
    path(s) {
        let (w, h) = (f64::from(s.width) / 2.0, f64::from(s.height) / 2.0);
        let c = pt(s.cx, s.cy);
        RoundedRect::new(c.x - w, c.y - h, c.x + w, c.y + h, f64::from(s.corner_radius))
            .to_path(TOLERANCE)
    }
}

shape! {
    /// A block arrow `length` long pointing in `angle` (counterclockwise on screen, 0 is
    /// right), centered on `(cx, cy)` (Remotion `Arrow`).
    BlockArrow(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        length: f32 = 300.0,
        head_width: f32 = 185.0,
        head_length: f32 = 120.0,
        shaft_width: f32 = 80.0,
        angle: f32 = 0.0,
    }
    paint: Paint::solid(t);
    path(s) {
        let c = pt(s.cx, s.cy);
        let length = f64::from(s.length);
        let head = f64::from(s.head_length).min(length);
        let (hw, sw) = (f64::from(s.head_width) / 2.0, f64::from(s.shaft_width) / 2.0);
        let (x0, xh, x1) = (-length / 2.0, length / 2.0 - head, length / 2.0);
        let angle = f64::from(s.angle);
        let (along, side) = (
            Vec2::new(angle.cos(), -angle.sin()),
            Vec2::new(angle.sin(), angle.cos()),
        );
        let at = |x: f64, y: f64| c + along * x + side * y;
        closed([
            at(x0, -sw),
            at(xh, -sw),
            at(xh, -hw),
            at(x1, 0.0),
            at(xh, hw),
            at(xh, sw),
            at(x0, sw),
        ])
    }
}

shape! {
    /// A speech bubble: a rounded box with a tail that points at `(tail_x, tail_y)`. The
    /// tail overlaps the box, so give it a fill and no outline.
    Callout(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        width: f32 = 400.0,
        height: f32 = 200.0,
        corner_radius: f32 = t.radius.md,
        tail_x: f32 = -100.0,
        tail_y: f32 = 200.0,
        tail_width: f32 = 60.0,
    }
    paint: Paint::solid(t);
    path(s) {
        let c = pt(s.cx, s.cy);
        let (w, h) = (f64::from(s.width) / 2.0, f64::from(s.height) / 2.0);
        let mut path = RoundedRect::new(c.x - w, c.y - h, c.x + w, c.y + h, f64::from(s.corner_radius))
            .to_path(TOLERANCE);
        let point = pt(s.tail_x, s.tail_y);
        let d = point - c;
        if d.hypot() > 1e-9 {
            let side = Vec2::new(-d.y, d.x).normalize() * (f64::from(s.tail_width) / 2.0);
            // the same winding as the box, so the fill is their union
            let tail = closed([c + side, point, c - side]);
            let tail = if winding(&tail) == winding(&path) { tail } else { tail.reverse_subpaths() };
            path.extend(tail.iter());
        }
        path
    }
}

fn winding(path: &BezPath) -> bool {
    path.area() >= 0.0
}

shape! {
    /// A heart `height` tall and 1.1 times as wide (Remotion `Heart`).
    Heart(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        height: f32 = 200.0,
    }
    paint: Paint::solid(t);
    path(s) {
        let (h, c) = (f64::from(s.height), pt(s.cx, s.cy));
        let w = h * 1.1;
        let at = |x: f64, y: f64| Point::new(c.x + (x - 0.5) * w, c.y + (y - 0.5) * h);
        let mut path = BezPath::new();
        path.move_to(at(0.5, 0.3));
        path.curve_to(at(0.5, 0.15), at(0.35, 0.0), at(0.2, 0.0));
        path.curve_to(at(0.05, 0.0), at(0.0, 0.15), at(0.0, 0.3));
        path.curve_to(at(0.0, 0.55), at(0.25, 0.75), at(0.5, 1.0));
        path.curve_to(at(0.75, 0.75), at(1.0, 0.55), at(1.0, 0.3));
        path.curve_to(at(1.0, 0.15), at(0.95, 0.0), at(0.8, 0.0));
        path.curve_to(at(0.65, 0.0), at(0.5, 0.15), at(0.5, 0.3));
        path.close_path();
        path
    }
}

shape! {
    /// The `progress` (0 to 1) part of a disc, clockwise from the top turned by `rotation`
    /// radians clockwise (Remotion `Pie`). With `closed` false only the arc is drawn.
    Pie(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        radius: f32 = 100.0,
        progress: f32 = 0.5,
        rotation: f32 = 0.0,
        closed: bool = true,
    }
    paint: Paint::solid(t);
    path(s) {
        let (c, r) = (pt(s.cx, s.cy), f64::from(s.radius));
        let progress = f64::from(s.progress.clamp(0.0, 1.0));
        if progress >= 1.0 {
            return kurbo::Circle::new(c, r).to_path(TOLERANCE);
        }
        let start = FRAC_PI_2 - f64::from(s.rotation);
        let a = arc(c, r, start, -progress * TAU);
        if !s.closed {
            return a.to_path(TOLERANCE);
        }
        let mut path = BezPath::new();
        path.move_to(c);
        path.line_to(polar(c, r, start));
        a.to_cubic_beziers(TOLERANCE, |p1, p2, p| path.curve_to(p1, p2, p));
        path.close_path();
        path
    }
}

shape! {
    /// A sparkle: `points` sharp points joined by curves that pass `inner` times the size
    /// from the center (Remotion's star with rounded inner corners).
    Spark(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        size: f32 = 100.0,
        points: usize = 4,
        inner: f32 = 0.2,
    }
    paint: Paint::solid(t);
    path(s) {
        let (c, r) = (pt(s.cx, s.cy), f64::from(s.size));
        let n = s.points.max(2);
        let step = TAU / n as f64;
        let tip = |i: usize| polar(c, r, FRAC_PI_2 + step * i as f64);
        let mut path = BezPath::new();
        path.move_to(tip(0));
        for i in 0..n {
            let control = polar(c, r * f64::from(s.inner), FRAC_PI_2 + step * (i as f64 + 0.5));
            path.quad_to(control, tip(i + 1));
        }
        path.close_path();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::Geometry, shapes::assert_bbox};

    #[test]
    fn boxes() {
        assert_bbox(&Rect::default(), -100.0, -100.0, 100.0, 100.0);
        assert_bbox(&BlockArrow::default(), -150.0, -92.5, 150.0, 92.5);
        let up = BlockArrow {
            angle: FRAC_PI_2 as f32,
            ..BlockArrow::default()
        };
        assert_bbox(&up, -92.5, -150.0, 92.5, 150.0);
        assert_bbox(&Callout::default(), -200.0, -100.0, 200.0, 200.0);
        assert_bbox(&Heart::default(), -110.0, -100.0, 110.0, 100.0);
        assert_bbox(&Spark::default(), -100.0, -100.0, 100.0, 100.0);
    }

    #[test]
    fn pie_runs_clockwise_from_the_top() {
        // half a pie, clockwise from the top: the right half
        assert_bbox(&Pie::default(), 0.0, -100.0, 100.0, 100.0);
        let full = Pie {
            progress: 1.0,
            ..Pie::default()
        };
        assert!((full.length() - TAU * 100.0).abs() < 0.5);
        let open = Pie {
            closed: false,
            progress: 0.25,
            ..Pie::default()
        };
        assert!((open.length() - TAU * 25.0).abs() < 0.5);
    }

    #[test]
    fn callout_tail_joins_the_fill() {
        let path = Callout::default().path();
        let subpaths = crate::paths::get_subpaths(&path);
        assert_eq!(subpaths.len(), 2);
        assert_eq!(winding(&subpaths[0]), winding(&subpaths[1]));
    }
}
