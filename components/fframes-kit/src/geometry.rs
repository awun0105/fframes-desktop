//! [`Geometry`] (anything with a path) and [`Shape`] (a path with its paint), drawn as one
//! `<path>` element in a [`Look`] that animations change.

use fframes::{
    Color, Svgr,
    usvgr::svgtree::{
        SvgAttributeValue,
        svgrtypes::{self, PathSegment},
    },
};
use kurbo::{Affine, BezPath, PathEl, Rect, Shape as _};

use crate::paths;

/// Anything drawn from one path: every shape, and later formulas and text outlines.
pub trait Geometry {
    /// The outline in canvas units (`y` down).
    fn path(&self) -> BezPath;

    /// Arc length of the whole path.
    fn length(&self) -> f64 {
        paths::get_length(&self.path())
    }

    /// The smallest rectangle around the path.
    fn bbox(&self) -> Rect {
        self.path().bounding_box()
    }
}

impl Geometry for BezPath {
    fn path(&self) -> BezPath {
        self.clone()
    }
}

/// A path and its paint. Shape components build one and draw it; build one yourself in the
/// video's constructor for paths that never change.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub path: BezPath,
    /// `None` leaves the inside empty.
    pub fill: Option<Color>,
    /// `None` draws no outline.
    pub stroke: Option<Color>,
    pub stroke_width: f32,
    /// Dash and gap lengths of the outline.
    pub dash: Option<(f32, f32)>,
    pub opacity: f32,
}

impl Geometry for Shape {
    fn path(&self) -> BezPath {
        self.path.clone()
    }
}

/// How a shape is drawn this frame; animations change it, the path stays as built.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Applied around the shape, in canvas units.
    pub transform: Affine,
    pub opacity: f32,
    pub fill_opacity: f32,
    pub stroke_opacity: f32,
    /// The part of the outline that is drawn, as fractions of its length from the start
    /// (`Create` grows the end, a passing flash moves both), through the dash pattern.
    pub drawn: (f32, f32),
    /// Replaces the fill colour (`Indicate`).
    pub fill: Option<Color>,
    /// Replaces the stroke colour (`DrawBorderThenFill` on a shape without an outline).
    pub stroke: Option<Color>,
    /// Replaces the stroke width.
    pub stroke_width: Option<f32>,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            transform: Affine::IDENTITY,
            opacity: 1.0,
            fill_opacity: 1.0,
            stroke_opacity: 1.0,
            drawn: (0.0, 1.0),
            fill: None,
            stroke: None,
            stroke_width: None,
        }
    }
}

impl Look {
    /// Applies `affine` after the transform the look already has.
    pub fn then(&mut self, affine: Affine) {
        self.transform = affine * self.transform;
    }

    /// Keeps only the part of the drawn window between `from` and `to` of the full outline.
    pub fn draw_window(&mut self, from: f32, to: f32) {
        self.drawn = (self.drawn.0.max(from), self.drawn.1.min(to));
    }
}

/// The `d` attribute of a path. With the `compile-time-svgtree` feature of fframes it goes
/// into the tree as segments, without formatting and parsing a string; without it `svgr!`
/// formats it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PathData(pub(crate) BezPath);

impl std::fmt::Display for PathData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_svg())
    }
}

impl From<PathData> for SvgAttributeValue<'_> {
    fn from(d: PathData) -> Self {
        path_segments(&d.0).into()
    }
}

/// The `transform` attribute for an affine map, like [`PathData`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Matrix(pub(crate) Affine);

impl std::fmt::Display for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [a, b, c, d, e, g] = self.0.as_coeffs();
        write!(f, "matrix({a} {b} {c} {d} {e} {g})")
    }
}

impl From<Matrix> for SvgAttributeValue<'_> {
    fn from(m: Matrix) -> Self {
        let [a, b, c, d, e, f] = m.0.as_coeffs();
        svgrtypes::Transform::new(a, b, c, d, e, f).into()
    }
}

fn path_segments(path: &BezPath) -> Vec<PathSegment> {
    path.elements()
        .iter()
        .map(|el| match *el {
            PathEl::MoveTo(p) => PathSegment::MoveTo {
                abs: true,
                x: p.x,
                y: p.y,
            },
            PathEl::LineTo(p) => PathSegment::LineTo {
                abs: true,
                x: p.x,
                y: p.y,
            },
            PathEl::QuadTo(p1, p) => PathSegment::Quadratic {
                abs: true,
                x1: p1.x,
                y1: p1.y,
                x: p.x,
                y: p.y,
            },
            PathEl::CurveTo(p1, p2, p) => PathSegment::CurveTo {
                abs: true,
                x1: p1.x,
                y1: p1.y,
                x2: p2.x,
                y2: p2.y,
                x: p.x,
                y: p.y,
            },
            PathEl::ClosePath => PathSegment::ClosePath { abs: true },
        })
        .collect()
}

pub(crate) fn path_data(path: &BezPath) -> PathData {
    PathData(path.clone())
}

pub(crate) fn matrix(affine: Affine) -> Matrix {
    Matrix(affine)
}

fn paint<'a>(color: Option<Color>) -> SvgAttributeValue<'a> {
    color.map_or_else(|| "none".into(), Into::into)
}

impl Shape {
    /// Draws the shape as built.
    pub fn draw<'a>(&self) -> Svgr<'a> {
        self.draw_with(&Look::default())
    }

    /// Draws the shape in `look`.
    pub fn draw_with<'a>(&self, look: &Look) -> Svgr<'a> {
        let d = path_data(&self.path);
        let fill = paint(look.fill.or(self.fill));
        let stroke = paint(look.stroke.or(self.stroke));
        let opacity = (self.opacity * look.opacity).clamp(0.0, 1.0);
        let mut stroke_opacity = look.stroke_opacity.clamp(0.0, 1.0);
        let dash: SvgAttributeValue<'a>;
        let mut offset = 0.0_f32;
        let (from, to) = (look.drawn.0.max(0.0), look.drawn.1.min(1.0));
        if from > 0.0 || to < 1.0 {
            // one dash covering the drawn part, the gap longer than the whole outline
            let length = paths::get_length(&self.path) as f32;
            let visible = (to - from) * length;
            if visible <= 0.0 {
                stroke_opacity = 0.0;
                dash = "none".into();
            } else {
                dash = format!("{visible} {}", length + 1.0).into();
                offset = -from * length;
            }
        } else if let Some((on, off)) = self.dash {
            dash = format!("{on} {off}").into();
        } else {
            dash = "none".into();
        }
        let transform = matrix(look.transform);
        fframes::svgr!(
            <path
                d={d}
                transform={transform}
                fill={fill}
                stroke={stroke}
                stroke-width={look.stroke_width.unwrap_or(self.stroke_width)}
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-dasharray={dash}
                stroke-dashoffset={offset}
                opacity={opacity}
                fill-opacity={look.fill_opacity.clamp(0.0, 1.0)}
                stroke-opacity={stroke_opacity}
            />
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_data_keeps_every_element() {
        let path = BezPath::from_svg("M0 0 L10 0 Q10 10 0 10 C-5 10 -5 0 0 0 Z").unwrap();
        let d = path_segments(&path);
        assert_eq!(d.len(), 5);
        assert!(matches!(d[4], PathSegment::ClosePath { .. }));
    }

    #[test]
    fn matrix_matches_affine() {
        let m = matrix(Affine::translate((3.0, 4.0)) * Affine::scale(2.0));
        assert_eq!(m.to_string(), "matrix(2 0 0 2 3 4)");
    }
}
