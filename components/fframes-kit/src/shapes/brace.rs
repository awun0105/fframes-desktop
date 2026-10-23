//! Manim's `Brace`, drawn as an outline.

use kurbo::{BezPath, Point, Vec2};

use super::{Paint, pt, shape};

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
        let h = (f64::from(s.depth) / 2.0).min(length / 4.0);
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
            Some((_, n, length)) => mid + n * f64::from(self.depth).min(length / 2.0),
            None => mid,
        };
        (p.x as f32, p.y as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::assert_bbox;

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
}
