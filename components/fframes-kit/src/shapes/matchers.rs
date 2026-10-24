//! Manim's matchers: shapes drawn around, behind, across or under a box. The box is a
//! [`kurbo::Rect`] in canvas pixels: `shape.bbox()` of any [`Geometry`](crate::geometry::Geometry),
//! `label.bounds(ui)` of a [`Label`](crate::label::Label) or `view.bounds()` of a formula.

use fframes::Color;
use kurbo::{BezPath, Rect, RoundedRect, Shape as _};

use super::{Paint, TOLERANCE, shape};

/// Manim's `SMALL_BUFF` (0.1 units) on a 1080 px tall canvas.
pub const SMALL_BUFF: f32 = 13.5;

/// Manim's `RED`, the colour of a `Cross`.
const RED: Color = Color::hex("#fc6255");

/// `around` grown by `buff` on every side, its corners rounded by `corner_radius`.
fn padded(around: Rect, buff: f32, corner_radius: f32) -> BezPath {
    let r = around.inflate(f64::from(buff), f64::from(buff));
    RoundedRect::from_rect(r, f64::from(corner_radius)).to_path(TOLERANCE)
}

shape! {
    /// A rectangle `buff` outside a box (Manim `SurroundingRectangle`), in the highlight
    /// colour.
    SurroundingRectangle(t) {
        around: Rect = Rect::ZERO,
        buff: f32 = SMALL_BUFF,
        corner_radius: f32 = 0.0,
    }
    paint: Paint { stroke: Some(t.colors.highlight), ..Paint::outline(t) };
    path(s) {
        padded(s.around, s.buff, s.corner_radius)
    }
}

shape! {
    /// A filled rectangle behind a box, so it reads over a busy picture (Manim
    /// `BackgroundRectangle`): the background colour at Manim's 75% opacity, no outline.
    BackgroundRectangle(t) {
        around: Rect = Rect::ZERO,
        buff: f32 = 0.0,
        corner_radius: f32 = 0.0,
    }
    paint: Paint {
        fill: Some(t.colors.background.with_alpha(191)),
        stroke: None,
        ..Paint::outline(t)
    };
    path(s) {
        padded(s.around, s.buff, s.corner_radius)
    }
}

shape! {
    /// Two lines from corner to corner of a box, scaled by `scale_factor` about its center
    /// (Manim `Cross`): red and thick.
    Cross(t) {
        around: Rect = Rect::ZERO,
        scale_factor: f32 = 1.0,
    }
    paint: Paint { stroke: Some(RED), stroke_width: t.stroke.thick, ..Paint::outline(t) };
    path(s) {
        let c = s.around.center();
        let k = f64::from(s.scale_factor);
        let (w, h) = (s.around.width() / 2.0 * k, s.around.height() / 2.0 * k);
        let mut path = BezPath::new();
        path.move_to((c.x - w, c.y - h));
        path.line_to((c.x + w, c.y + h));
        path.move_to((c.x - w, c.y + h));
        path.line_to((c.x + w, c.y - h));
        path
    }
}

shape! {
    /// A line `buff` below a box, as wide as it (Manim `Underline`).
    Underline(t) {
        around: Rect = Rect::ZERO,
        buff: f32 = SMALL_BUFF,
    }
    paint: Paint::outline(t);
    path(s) {
        let y = s.around.y1 + f64::from(s.buff);
        let mut path = BezPath::new();
        path.move_to((s.around.x0, y));
        path.line_to((s.around.x1, y));
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::assert_bbox;

    const BOX: Rect = Rect::new(100.0, 50.0, 300.0, 150.0);

    #[test]
    fn matchers_keep_buff_around_the_box() {
        let around = SurroundingRectangle {
            around: BOX,
            ..SurroundingRectangle::default()
        };
        assert_bbox(&around, 86.5, 36.5, 313.5, 163.5);
        let rounded = SurroundingRectangle {
            buff: 0.0,
            corner_radius: 20.0,
            ..around
        };
        assert_bbox(&rounded, 100.0, 50.0, 300.0, 150.0);
        let back = BackgroundRectangle {
            around: BOX,
            ..BackgroundRectangle::default()
        };
        assert_bbox(&back, 100.0, 50.0, 300.0, 150.0);
        assert!(back.stroke.is_none() && back.fill.is_some_and(|c| !c.is_opaque()));
        let under = Underline {
            around: BOX,
            ..Underline::default()
        };
        assert_bbox(&under, 100.0, 163.5, 300.0, 163.5);
    }

    #[test]
    fn cross_scales_about_the_center() {
        let cross = Cross {
            around: BOX,
            scale_factor: 0.5,
            ..Cross::default()
        };
        assert_bbox(&cross, 150.0, 75.0, 250.0, 125.0);
        assert_eq!(crate::paths::get_subpaths(&cross.shape().path).len(), 2);
    }
}
