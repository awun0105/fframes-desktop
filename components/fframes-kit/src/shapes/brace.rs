//! Manim's `Brace` and `ArcBrace`, drawn as outlines.

use kurbo::{BezPath, Point, Vec2};

use super::{Paint, basic::polar, pt, shape};
use crate::paths::warp_path;

shape! {
    /// A curly brace along `(x1, y1)` to `(x2, y2)` whose tip points `depth` pixels to the
    /// right of that direction on screen: below for a brace drawn left to right (Manim
    /// `Brace` under a mobject). [`Brace::tip`] is where a label goes.
    Brace(t) {
        x1: f32 = 0.0,
        y1: f32 = 0.0,
        x2: f32 = 200.0,
        y2: f32 = 0.0,
        depth: f32 = 24.0,
    }
    paint: Paint::outline(t);
    path(s) {
        let (from, to) = (pt(s.x1, s.y1), pt(s.x2, s.y2));
        let Some((u, n, length)) = s.frame() else {
            let mut path = BezPath::new();
            path.move_to(from);
            return path;
        };
        // a negative depth turns the whole brace over, its ends still curl toward the middle
        let n = n * f64::from(s.depth.signum());
        let h = (f64::from(s.depth.abs()) / 2.0).min(length / 4.0);
        let mid = from.midpoint(to);
        let mut path = BezPath::new();
        path.move_to(from);
        path.quad_to(from + n * h, from + n * h + u * h);
        path.line_to(mid + n * h - u * h);
        path.quad_to(mid + n * h, mid + n * (2.0 * h));
        path.quad_to(mid + n * h, mid + n * h + u * h);
        path.line_to(to + n * h - u * h);
        path.quad_to(to + n * h, to);
        path
    }
}

impl Brace {
    /// Direction along the brace, the side its tip points to, and its length.
    fn frame(&self) -> Option<(Vec2, Vec2, f64)> {
        let d = pt(self.x2, self.y2) - pt(self.x1, self.y1);
        let length = d.hypot();
        (length > 1e-9).then(|| {
            let u = d / length;
            (u, Vec2::new(-u.y, u.x), length)
        })
    }

    /// The point of the tip.
    pub fn tip(&self) -> (f32, f32) {
        let mid = pt(self.x1, self.y1).midpoint(pt(self.x2, self.y2));
        let p: Point = match self.frame() {
            Some((_, n, length)) => {
                mid + n
                    * f64::from(self.depth.abs()).min(length / 2.0)
                    * f64::from(self.depth.signum())
            }
            None => mid,
        };
        (p.x as f32, p.y as f32)
    }
}

shape! {
    /// A brace bent along an arc (Manim `ArcBrace`): round the circle of `radius` about
    /// `(cx, cy)` from `start_angle` turning `angle` (radians, counterclockwise on screen), its
    /// tip `depth` pixels outward, or toward the center when `inside`. [`ArcBrace::tip`] is
    /// where a label goes.
    ArcBrace(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        radius: f32 = 135.0,
        start_angle: f32 = -1.0,
        angle: f32 = 2.0,
        depth: f32 = 24.0,
        inside: bool = false,
    }
    paint: Paint::outline(t);
    path(s) {
        let r = f64::from(s.radius.abs()).max(1e-6);
        let length = r * f64::from(s.angle).abs();
        let straight = Brace { x2: length as f32, depth: s.depth, ..Brace::default() };
        // along the straight brace is along the arc, its tip side is away from the center
        warp_path(&straight.path(), 4.0, |p| s.place(p.x, p.y))
    }
}

impl ArcBrace {
    /// The canvas point `along` pixels into the arc and `out` pixels off it, outward.
    fn place(&self, along: f64, out: f64) -> Point {
        let r = f64::from(self.radius.abs()).max(1e-6);
        let side = if self.inside { -1.0 } else { 1.0 };
        let turn = f64::from(self.angle).signum() * along / r;
        polar(
            pt(self.cx, self.cy),
            r + side * out,
            f64::from(self.start_angle) + turn,
        )
    }

    /// The point of the tip.
    pub fn tip(&self) -> (f32, f32) {
        let length = f64::from(self.radius.abs()) * f64::from(self.angle).abs();
        let depth = f64::from(self.depth);
        let p = self.place(length / 2.0, depth.abs().min(length / 2.0) * depth.signum());
        (p.x as f32, p.y as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::Geometry, shapes::assert_bbox};

    #[test]
    fn brace_points_down_when_drawn_left_to_right() {
        let b = Brace::default();
        assert_bbox(&b, 0.0, 0.0, 200.0, 24.0);
        assert_eq!(b.tip(), (100.0, 24.0));
        let up = Brace {
            x1: 200.0,
            x2: 0.0,
            ..b
        };
        assert_bbox(&up, 0.0, -24.0, 200.0, 0.0);
    }

    #[test]
    fn a_negative_depth_puts_the_tip_on_the_other_side() {
        let b = Brace {
            depth: -24.0,
            ..Brace::default()
        };
        assert_bbox(&b, 0.0, -24.0, 200.0, 0.0);
        assert_eq!(b.tip(), (100.0, -24.0));
    }

    #[test]
    fn an_arc_brace_stays_beside_its_arc() {
        let brace = ArcBrace::default();
        let distance = |p: Point| p.to_vec2().hypot();
        let points: Vec<Point> = brace
            .path()
            .elements()
            .iter()
            .filter_map(kurbo::PathEl::end_point)
            .collect();
        assert!(
            points
                .iter()
                .all(|&p| (135.0 - 1e-6..=159.0 + 1e-6).contains(&distance(p)))
        );
        let (x, y) = brace.tip();
        // the middle of the arc is at angle 0, to the right of the center
        assert!((x - 159.0).abs() < 1e-3 && y.abs() < 1e-3, "{x} {y}");
        let inside = ArcBrace {
            inside: true,
            ..brace
        };
        let points: Vec<Point> = inside
            .path()
            .elements()
            .iter()
            .filter_map(kurbo::PathEl::end_point)
            .collect();
        assert!(
            points
                .iter()
                .all(|&p| (111.0 - 1e-6..=135.0 + 1e-6).contains(&distance(p)))
        );
        assert!((inside.tip().0 - 111.0).abs() < 1e-3);
    }
}
