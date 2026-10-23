use std::f64::consts::TAU;

use fframes::{Color, Svgr, animation::Animatable};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::RateFunc,
    geometry::{Geometry, Look, Shape},
    ui::Ui,
};

use super::{ShapeAnimation, about, create::themed, enter_alpha};

/// Scales the shape up and tints it, then back (Manim's `Indicate`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Indicate {
    pub scale_factor: f32,
    pub color: Color,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(Indicate, |t| Indicate {
    scale_factor: 1.2,
    color: t.colors.highlight,
    delay: 0.0,
    run_time: 1.0,
    rate_func: RateFunc::ThereAndBack,
});

impl ShapeAnimation for Indicate {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return;
        }
        let s = 1.0 + f64::from((self.scale_factor - 1.0) * p);
        look.then(about(shape.bbox().center(), s, 0.0));
        look.fill = shape.fill.map(|c| c.apply_progress(&self.color, p));
        look.stroke = shape.stroke.map(|c| c.apply_progress(&self.color, p));
    }
}

/// The part of a line drawn by a passing flash at progress `p`: the head runs ahead, the tail
/// follows, so the line appears and leaves in one direction.
fn passing(p: f32, look: &mut Look) {
    look.draw_window((2.0 * p - 1.0).max(0.0), (2.0 * p).min(1.0));
    if p >= 1.0 {
        look.stroke_opacity = 0.0;
    }
}

fn line_shape(path: BezPath, color: Color, width: f32) -> Shape {
    Shape {
        path,
        fill: None,
        stroke: Some(color),
        stroke_width: width,
        dash: None,
        opacity: 1.0,
    }
}

/// A rectangle that runs once around the shape (Manim's `Circumscribe`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circumscribe {
    pub color: Color,
    /// Space between the shape and the rectangle.
    pub buff: f32,
    pub stroke_width: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(Circumscribe, |t| Circumscribe {
    color: t.colors.highlight,
    buff: t.spacing.md,
    stroke_width: t.stroke.regular,
    delay: 0.0,
    run_time: 1.0,
    rate_func: RateFunc::Smooth,
});

impl ShapeAnimation for Circumscribe {
    fn look(&self, _ui: &Ui<'_, '_, '_>, _shape: &Shape, _look: &mut Look) {}

    fn overlay<'a>(&self, ui: &Ui<'a, '_, '_>, shape: &Shape) -> Option<Svgr<'a>> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 || p >= 1.0 {
            return None;
        }
        let b = shape
            .bbox()
            .inflate(f64::from(self.buff), f64::from(self.buff));
        let mut path = BezPath::new();
        path.move_to((b.x0, b.y0));
        path.line_to((b.x1, b.y0));
        path.line_to((b.x1, b.y1));
        path.line_to((b.x0, b.y1));
        path.close_path();
        let mut look = Look::default();
        passing(p, &mut look);
        Some(line_shape(path, self.color, self.stroke_width).draw_with(&look))
    }
}

/// Lines that shoot out of a point once (Manim's `Flash`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flash {
    /// The center; `None` is the center of the shape.
    pub point: Option<(f32, f32)>,
    pub color: Color,
    pub num_lines: usize,
    /// Distance from the center to where the lines start.
    pub flash_radius: f32,
    pub line_length: f32,
    pub stroke_width: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(Flash, |t| Flash {
    point: None,
    color: t.colors.highlight,
    num_lines: 12,
    flash_radius: 60.0,
    line_length: 40.0,
    stroke_width: t.stroke.regular,
    delay: 0.0,
    run_time: 1.0,
    rate_func: RateFunc::Smooth,
});

impl ShapeAnimation for Flash {
    fn look(&self, _ui: &Ui<'_, '_, '_>, _shape: &Shape, _look: &mut Look) {}

    fn overlay<'a>(&self, ui: &Ui<'a, '_, '_>, shape: &Shape) -> Option<Svgr<'a>> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 || p >= 1.0 {
            return None;
        }
        let center = self.point.map_or_else(
            || shape.bbox().center(),
            |(x, y)| Point::new(f64::from(x), f64::from(y)),
        );
        let (r0, r1) = (
            f64::from(self.flash_radius),
            f64::from(self.flash_radius + self.line_length),
        );
        let mut look = Look::default();
        passing(p, &mut look);
        let n = self.num_lines.max(1);
        Some(
            (0..n)
                .map(|i| {
                    let dir = Vec2::from_angle(TAU * i as f64 / n as f64);
                    let mut path = BezPath::new();
                    path.move_to(center + dir * r0);
                    path.line_to(center + dir * r1);
                    line_shape(path, self.color, self.stroke_width).draw_with(&look)
                })
                .collect(),
        )
    }
}

/// Shakes the shape: it turns back and forth while it grows a little (Manim's `Wiggle`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wiggle {
    pub scale_value: f32,
    /// Largest turn in radians.
    pub rotation_angle: f32,
    pub n_wiggles: usize,
    pub delay: f32,
    pub run_time: f32,
}

themed!(Wiggle, |_t| Wiggle {
    scale_value: 1.1,
    rotation_angle: 0.01 * std::f32::consts::TAU,
    n_wiggles: 6,
    delay: 0.0,
    run_time: 2.0,
});

impl ShapeAnimation for Wiggle {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, RateFunc::Linear);
        if p <= 0.0 || p >= 1.0 {
            return;
        }
        let envelope = RateFunc::ThereAndBack.apply(p, self.run_time);
        let s = 1.0 + f64::from((self.scale_value - 1.0) * envelope);
        let wiggle = (self.n_wiggles as f64 * TAU * f64::from(p)).sin();
        let angle = f64::from(self.rotation_angle * envelope) * wiggle;
        look.then(about(shape.bbox().center(), s, angle));
    }
}
