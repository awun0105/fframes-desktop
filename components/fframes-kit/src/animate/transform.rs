use fframes::{Color, Svgr, animation::Animatable};
use kurbo::{Affine, BezPath, Vec2};

use crate::{
    anim::{Enter, RateFunc},
    geometry::{Shape, matrix},
    motion::lerp,
    paths,
    ui::{Component, Ui},
};

use super::enter_alpha;

/// Cubic segments per subpath a morph runs on at least: a square's corners then meet a
/// circle's diagonals.
pub(crate) const MORPH_SEGMENTS: usize = 16;

/// Paint between `a` and `b`; a missing paint is the other one, transparent.
pub(crate) fn lerp_paint(a: Option<Color>, b: Option<Color>, t: f32) -> Option<Color> {
    match (a, b) {
        (None, None) => None,
        (Some(a), None) => Some(a.apply_progress(&a.with_alpha(0), t)),
        (None, Some(b)) => Some(b.with_alpha(0).apply_progress(&b, t)),
        (Some(a), Some(b)) => Some(a.apply_progress(&b, t)),
    }
}

/// The shape `t` of the way from `from` to `to`, on paths aligned with
/// [`paths::align_points`].
pub(crate) fn morph(from: &Shape, to: &Shape, a: &BezPath, b: &BezPath, t: f32) -> Shape {
    Shape {
        path: paths::lerp_aligned(f64::from(t), a, b),
        fill: lerp_paint(from.fill, to.fill, t),
        stroke: lerp_paint(from.stroke, to.stroke, t),
        stroke_width: lerp(from.stroke_width, to.stroke_width, t),
        dash: if t < 1.0 { from.dash } else { to.dash },
        opacity: lerp(from.opacity, to.opacity, t),
    }
}

/// Morphs `from` into `to`: path, paint and width (Manim's `Transform`). Before it starts
/// it draws `from`, after it ends the morphed shape, which looks like `to`. The prelude
/// calls it `Morph`, because `Transform` there is the SVG transform.
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub from: Shape,
    pub to: Shape,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Transform {
    /// From `from` to `to` in Manim's 1 second, smooth.
    pub fn new(from: Shape, to: Shape) -> Self {
        Self {
            from,
            to,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for Transform {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return self.from.draw();
        }
        let (a, b) = paths::align_points(&self.from.path, &self.to.path, MORPH_SEGMENTS);
        morph(&self.from, &self.to, &a, &b, p).draw()
    }
}

/// `Transform` that draws `to` itself once the morph has finished (Manim's
/// `ReplacementTransform`).
#[derive(Debug, Clone, PartialEq)]
pub struct ReplacementTransform {
    pub from: Shape,
    pub to: Shape,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ReplacementTransform {
    /// From `from` to `to` in Manim's 1 second, smooth.
    pub fn new(from: Shape, to: Shape) -> Self {
        Self {
            from,
            to,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for ReplacementTransform {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p >= 1.0 && ui.seconds() >= self.delay + self.run_time {
            return self.to.draw();
        }
        Transform {
            from: self.from,
            to: self.to,
            delay: self.delay,
            run_time: self.run_time,
            rate_func: self.rate_func,
        }
        .render(ui)
    }
}

/// Moves content along `path` (Manim's `MoveAlongPath`). Draw the content at the start of
/// the path; it ends at the path's end.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveAlongPath {
    pub path: BezPath,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl MoveAlongPath {
    /// Along `path` in Manim's 1 second, smooth.
    pub fn new(path: BezPath) -> Self {
        Self {
            path,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl Enter for MoveAlongPath {
    fn apply<'a>(&self, ui: &Ui<'a, '_, '_>, content: Svgr<'a>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let length = paths::get_length(&self.path);
        let (Some(start), Some(at)) = (
            paths::get_point_at_length(&self.path, 0.0),
            paths::get_point_at_length(&self.path, length * f64::from(p)),
        ) else {
            return content;
        };
        let offset: Vec2 = at - start;
        let transform = matrix(Affine::translate(offset));
        fframes::svgr!(<g transform={transform}>{content}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_morphs_from_and_to_nothing() {
        let red = Color::hex("#ff0000");
        assert_eq!(lerp_paint(Some(red), None, 1.0), Some(red.with_alpha(0)));
        assert_eq!(lerp_paint(None, Some(red), 1.0), Some(red));
        assert_eq!(lerp_paint(None, None, 0.5), None);
    }

    #[test]
    fn morph_ends_on_the_target_geometry() {
        let square = Shape {
            path: paths::parse_path("M0 0 L10 0 L10 10 L0 10 Z").unwrap(),
            fill: None,
            stroke: Some(Color::WHITE),
            stroke_width: 4.0,
            dash: None,
            opacity: 1.0,
        };
        let circle = Shape {
            path: kurbo::Shape::to_path(&kurbo::Circle::new((5.0, 5.0), 5.0), 1e-6),
            fill: Some(Color::WHITE),
            ..square.clone()
        };
        let (a, b) = paths::align_points(&square.path, &circle.path, MORPH_SEGMENTS);
        let end = morph(&square, &circle, &a, &b, 1.0);
        assert!((paths::get_length(&end.path) - paths::get_length(&circle.path)).abs() < 1e-3);
        assert_eq!(end.fill, Some(Color::WHITE));
    }
}
