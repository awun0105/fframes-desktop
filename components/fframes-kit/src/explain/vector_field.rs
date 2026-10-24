//! Manim's `VectorField` and `ArrowVectorField`.

use fframes::{Color, Svgr, animation::Animatable};
use kurbo::Point;

use crate::{
    animate::{Emphasis, EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    shapes::straight,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{Range, Space};

/// A vector field: the vector at a point, both in units of `ui.space`. A plain `fn`, so
/// stream lines traced through it are cached by the function's address.
pub type Field = fn(f64, f64) -> (f64, f64);

/// What [`ArrowVectorField`] and [`StreamLines`](super::StreamLines) share (Manim
/// `VectorField`): the function, the box it is shown in, and the colour of a vector by its
/// length, from `colors[0]` at `min_magnitude` to the last colour at `max_magnitude`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
#[allow(unpredictable_function_pointer_comparisons)] // equal props, not equal functions
pub struct VectorField {
    pub f: Field,
    /// Where the arrows or the lines start, every `step`, in units of `ui.space`.
    pub x_range: Range,
    pub y_range: Range,
    pub colors: Vec<Color>,
    pub min_magnitude: f64,
    pub max_magnitude: f64,
    /// One colour for every vector instead of the gradient.
    pub color: Option<Color>,
}

impl Themed for VectorField {
    fn themed(t: &Theme) -> Self {
        Self {
            f: |x, y| (-y, x),
            x_range: Range::new(-7.0, 7.0, 0.5),
            y_range: Range::new(-4.0, 4.0, 0.5),
            colors: vec![
                t.colors.accent,
                t.colors.highlight,
                Color::rgb(0xe8, 0x4a, 0x5f),
            ],
            min_magnitude: 0.0,
            max_magnitude: 2.0,
            color: None,
        }
    }
}

impl Default for VectorField {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl VectorField {
    /// The vector at `(x, y)`.
    pub fn at(&self, x: f64, y: f64) -> (f64, f64) {
        (self.f)(x, y)
    }

    /// The colour of a vector `magnitude` long.
    pub fn color_for(&self, magnitude: f64) -> Color {
        if let Some(color) = self.color {
            return color;
        }
        ramp(
            &self.colors,
            self.min_magnitude,
            self.max_magnitude,
            magnitude,
        )
    }

    /// The colour of the vector at `(x, y)`.
    pub fn color_at(&self, x: f64, y: f64) -> Color {
        let (vx, vy) = self.at(x, y);
        self.color_for(vx.hypot(vy))
    }

    /// The points the arrows or lines start at.
    pub fn grid(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        let ys = self.y_range.ticks();
        self.x_range
            .ticks()
            .into_iter()
            .flat_map(move |x| ys.clone().into_iter().map(move |y| (x, y)))
    }
}

/// The colour of `magnitude` on a gradient through `colors`, from the first at `min` to the
/// last at `max`; white without colours.
pub(crate) fn ramp(colors: &[Color], min: f64, max: f64, magnitude: f64) -> Color {
    if colors.is_empty() {
        return Color::WHITE;
    }
    let span = max - min;
    let t = if span > 0.0 && magnitude.is_finite() {
        ((magnitude - min) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let at = t * (colors.len() - 1) as f64;
    let i = (at.floor() as usize).min(colors.len() - 1);
    match colors.get(i + 1) {
        Some(next) => colors[i].apply_progress(next, (at - i as f64) as f32),
        None => colors[i],
    }
}
/// Manim's default `length_func`: `0.45 * sigmoid(norm)`.
pub fn sigmoid_length(norm: f64) -> f64 {
    0.45 / (1.0 + (-norm).exp())
}

/// An arrow at every grid point of a [`VectorField`] (Manim `ArrowVectorField`), starting
/// at the point, pointing along the vector, `length(magnitude)` units long and coloured by
/// the magnitude. Zero vectors draw nothing.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
#[allow(unpredictable_function_pointer_comparisons)] // equal props, not equal functions
pub struct ArrowVectorField {
    pub field: VectorField,
    /// The arrow's length in units for a vector of this magnitude.
    pub length: fn(f64) -> f64,
    pub stroke_width: f32,
    pub tip_length: f32,
    pub tip_width: f32,
    pub opacity: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub emphasis: Option<Emphasis>,
}

impl Themed for ArrowVectorField {
    fn themed(t: &Theme) -> Self {
        Self {
            field: VectorField::themed(t),
            length: sigmoid_length,
            stroke_width: t.stroke.regular,
            tip_length: 14.0,
            tip_width: 12.0,
            opacity: 1.0,
            enter: None,
            exit: None,
            emphasis: None,
        }
    }
}

impl Default for ArrowVectorField {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl ArrowVectorField {
    /// The arrows as shapes, in pixels of `space`.
    pub fn shapes(&self, space: Space) -> Vec<Shape> {
        self.field
            .grid()
            .filter_map(|(x, y)| {
                let (vx, vy) = self.field.at(x, y);
                let norm = vx.hypot(vy);
                if !(norm.is_finite() && norm > 0.0) {
                    return None;
                }
                let length = (self.length)(norm);
                let (to_x, to_y) = (x + vx / norm * length, y + vy / norm * length);
                let (from, to): (Point, Point) = (space.to_px(x, y), space.to_px(to_x, to_y));
                let pixels = (to - from).hypot();
                // the tip never takes more than half of a short arrow
                let tip = f64::from(self.tip_length).min(pixels / 2.0);
                let width = f64::from(self.tip_width) * tip / f64::from(self.tip_length).max(1e-9);
                let color = self.field.color_for(norm);
                Some(Shape {
                    path: straight(from, to, tip, width, false),
                    fill: Some(color),
                    stroke: Some(color),
                    stroke_width: self.stroke_width,
                    dash: None,
                    opacity: self.opacity,
                })
            })
            .collect()
    }
}

impl<'a> Component<'a> for ArrowVectorField {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let shapes = self.shapes(ui.space);
        draw_animated(
            ui,
            &shapes,
            self.enter.as_ref(),
            self.exit.as_ref(),
            self.emphasis.as_ref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use kurbo::Shape as _;

    use super::*;

    #[test]
    fn colours_follow_the_magnitude() {
        let (a, b) = (Color::rgb(0, 0, 0), Color::rgb(200, 100, 0));
        let field = VectorField {
            colors: vec![a, b],
            ..VectorField::default()
        };
        assert_eq!(field.color_for(0.0), a);
        assert_eq!(field.color_for(2.0), b);
        assert_eq!(field.color_for(9.0), b);
        assert_eq!(field.color_for(1.0), Color::rgb(100, 50, 0));
        // (x, y) -> (-y, x) is as long as the point is far from the middle
        assert_eq!(field.color_at(0.0, 1.0), Color::rgb(100, 50, 0));
        let one = VectorField {
            color: Some(a),
            ..VectorField::default()
        };
        assert_eq!(one.color_for(1.5), a);
    }

    #[test]
    fn arrows_start_at_their_points_and_skip_zero_vectors() {
        let arrows = ArrowVectorField {
            field: VectorField {
                f: |x, _| (x, 0.0),
                x_range: Range::new(-1.0, 1.0, 1.0),
                y_range: Range::new(0.0, 0.0, 1.0),
                ..VectorField::default()
            },
            ..ArrowVectorField::default()
        };
        let shapes = arrows.shapes(Space::FULL_HD);
        // x = 0 is a zero vector
        assert_eq!(shapes.len(), 2);
        let start = |s: &Shape| match s.path.elements()[0] {
            kurbo::PathEl::MoveTo(p) => p,
            _ => unreachable!(),
        };
        assert_eq!(start(&shapes[0]), Point::new(960.0 - 135.0, 540.0));
        assert_eq!(start(&shapes[1]), Point::new(960.0 + 135.0, 540.0));
        let length = sigmoid_length(1.0) * 135.0;
        let right = shapes[1].path.bounding_box();
        assert!(
            (right.x1 - (960.0 + 135.0 + length)).abs() < 1.0,
            "{right:?}"
        );
    }
}
