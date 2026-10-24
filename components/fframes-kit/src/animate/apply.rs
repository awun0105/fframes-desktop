use fframes::Svgr;
use kurbo::{Affine, BezPath, Point};

use crate::{
    anim::RateFunc,
    explain::Space,
    geometry::{Geometry, Look, Shape},
    paths,
    ui::{Component, Ui},
};

use super::{Transform, enter_alpha, homotopy::warp_in, transform_ii::lerp_along_arc};

/// `path` with every segment a cubic and every control point mapped through `f`, so a
/// linear `f` maps it exactly and the result lines up with the identity point for point.
fn mapped(path: &BezPath, f: impl Fn(Point) -> Point) -> BezPath {
    paths::warp_path(path, f64::INFINITY, f)
}

/// `shape` with its path `t` of the way from `a` to `b` (aligned) on arcs of `arc` radians.
fn between<'a>(shape: Shape, a: &BezPath, b: &BezPath, t: f32, arc: f64) -> Svgr<'a> {
    let path = lerp_along_arc(f64::from(t), a, b, arc);
    Shape { path, ..shape }.draw()
}

/// Moves every point of the shape by the 2×2 `matrix` about `about_point` (Manim's
/// `ApplyMatrix`, about the origin of `ui.space` in 3 seconds). The matrix and the point are
/// in the units of `ui.space`, `y` up: `[[0, -1], [1, 0]]` turns a quarter counterclockwise.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ApplyMatrix {
    pub shape: Shape,
    pub matrix: [[f64; 2]; 2],
    /// In units; `None` is the origin of the space.
    pub about_point: Option<(f64, f64)>,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ApplyMatrix {
    /// About the origin in Manim's 3 seconds, smooth.
    pub fn new(shape: Shape, matrix: [[f64; 2]; 2]) -> Self {
        Self {
            shape,
            matrix,
            about_point: None,
            delay: 0.0,
            run_time: 3.0,
            rate_func: RateFunc::Smooth,
        }
    }

    /// Where the pixel `p` ends up.
    pub fn map(&self, space: Space, p: Point) -> Point {
        let (cx, cy) = self.about_point.unwrap_or((0.0, 0.0));
        let (x, y) = space.from_px(p);
        let (x, y) = (x - cx, y - cy);
        let m = self.matrix;
        space.to_px(
            cx + m[0][0] * x + m[0][1] * y,
            cy + m[1][0] * x + m[1][1] * y,
        )
    }
}

impl<'a> Component<'a> for ApplyMatrix {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return self.shape.draw();
        }
        let space = ui.space;
        let a = mapped(&self.shape.path, |q| q);
        let b = mapped(&self.shape.path, |q| self.map(space, q));
        between(self.shape, &a, &b, p, 0.0)
    }
}

/// Moves every point of the shape to `function(x, y)` (Manim's `ApplyPointwiseFunction`, 3
/// seconds), in the units of `ui.space`, `y` up. The path is cut into small pieces first
/// like a [`super::Homotopy`], so straight lines bend.
#[derive(Debug, Clone)]
pub struct ApplyPointwiseFunction {
    pub shape: Shape,
    pub function: fn(f64, f64) -> (f64, f64),
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ApplyPointwiseFunction {
    /// Manim's 3 seconds, smooth.
    pub fn new(shape: Shape, function: fn(f64, f64) -> (f64, f64)) -> Self {
        Self {
            shape,
            function,
            delay: 0.0,
            run_time: 3.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for ApplyPointwiseFunction {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return self.shape.draw();
        }
        let a = warp_in(&self.shape.path, ui.space, |x, y| (x, y));
        let b = warp_in(&self.shape.path, ui.space, self.function);
        between(self.shape, &a, &b, p, 0.0)
    }
}

/// Moves every point of the shape to `function(re, im)` of the complex number at it
/// (Manim's `ApplyComplexFunction`, 1 second), in the units of `ui.space`. As in Manim the
/// points travel on arcs as wide as the angle of `function(1)`, so a rotation turns.
#[derive(Debug, Clone)]
pub struct ApplyComplexFunction {
    pub shape: Shape,
    pub function: fn((f64, f64)) -> (f64, f64),
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ApplyComplexFunction {
    /// Manim's 1 second, smooth.
    pub fn new(shape: Shape, function: fn((f64, f64)) -> (f64, f64)) -> Self {
        Self {
            shape,
            function,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }

    /// The angle of `function(1)`: how far the points turn on their way.
    pub fn path_arc(&self) -> f64 {
        let (re, im) = (self.function)((1.0, 0.0));
        im.atan2(re)
    }
}

impl<'a> Component<'a> for ApplyComplexFunction {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return self.shape.draw();
        }
        let a = warp_in(&self.shape.path, ui.space, |x, y| (x, y));
        let b = warp_in(&self.shape.path, ui.space, |x, y| (self.function)((x, y)));
        let arc = self.path_arc();
        between(self.shape, &a, &b, p, arc)
    }
}

/// Morphs the shape into `function(shape)` (Manim's `ApplyFunction`, a [`Transform`] in 1
/// second).
#[derive(Debug, Clone)]
pub struct ApplyFunction {
    pub shape: Shape,
    pub function: fn(Shape) -> Shape,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ApplyFunction {
    /// Manim's 1 second, smooth.
    pub fn new(shape: Shape, function: fn(Shape) -> Shape) -> Self {
        Self {
            shape,
            function,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for ApplyFunction {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        Transform {
            to: (self.function)(self.shape.clone()),
            from: self.shape,
            delay: self.delay,
            run_time: self.run_time,
            rate_func: self.rate_func,
        }
        .render(ui)
    }
}

/// Moves the whole shape so its center goes to `function(center)` (Manim's
/// `ApplyPointwiseFunctionToCenter`, 3 seconds), in the units of `ui.space`. The shape keeps
/// its form.
#[derive(Debug, Clone)]
pub struct ApplyPointwiseFunctionToCenter {
    pub shape: Shape,
    pub function: fn(f64, f64) -> (f64, f64),
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ApplyPointwiseFunctionToCenter {
    /// Manim's 3 seconds, smooth.
    pub fn new(shape: Shape, function: fn(f64, f64) -> (f64, f64)) -> Self {
        Self {
            shape,
            function,
            delay: 0.0,
            run_time: 3.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for ApplyPointwiseFunctionToCenter {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let center = self.shape.bbox().center();
        let (x, y) = ui.space.from_px(center);
        let (x, y) = (self.function)(x, y);
        let offset = (ui.space.to_px(x, y) - center) * f64::from(p);
        let look = Look {
            transform: Affine::translate(offset),
            ..Look::default()
        };
        self.shape.draw_with(&look)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fframes::Color;

    fn shape(d: &str) -> Shape {
        Shape {
            path: paths::parse_path(d).unwrap(),
            fill: None,
            stroke: Some(Color::WHITE),
            stroke_width: 2.0,
            dash: None,
            opacity: 1.0,
        }
    }

    #[test]
    fn a_quarter_turn_matrix_turns_right_to_up() {
        let m = ApplyMatrix::new(shape("M0 0 L1 1"), [[0.0, -1.0], [1.0, 0.0]]);
        let space = Space::FULL_HD;
        let right = space.to_px(1.0, 0.0);
        let up = m.map(space, right);
        assert!((up - space.to_px(0.0, 1.0)).hypot() < 1e-9, "{up:?}");
        assert!(up.y < right.y, "up is a smaller y on screen");
    }

    #[test]
    fn mapped_paths_line_up_with_the_original() {
        let square = shape("M0 0 L10 0 L10 10 L0 10 Z").path;
        let a = mapped(&square, |q| q);
        let b = mapped(&square, |q| Point::new(q.x * 2.0, q.y));
        assert_eq!(a.elements().len(), b.elements().len());
        let mid = paths::lerp_aligned(0.5, &a, &b);
        let bb = kurbo::Shape::bounding_box(&mid);
        assert!((bb.x1 - 15.0).abs() < 1e-9, "{bb:?}");
    }

    #[test]
    fn a_half_turn_function_moves_points_on_half_circles() {
        // f(z) = -z: f(1) is negative real, the arc is a half turn with its center between
        let f = ApplyComplexFunction::new(shape("M0 0 L1 1"), |(re, im)| (-re, -im));
        assert!((f.path_arc().abs() - std::f64::consts::PI).abs() < 1e-12);
        let s = Point::new(100.0, 0.0);
        let e = Point::new(-100.0, 0.0);
        let mid = super::super::transform_ii::arc_point(s, e, f.path_arc(), 0.5);
        assert!(
            mid.x.abs() < 1e-9 && (mid.y.abs() - 100.0).abs() < 1e-9,
            "{mid:?}"
        );
    }
}
