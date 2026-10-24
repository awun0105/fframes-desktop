//! Manim's `DashedVMobject` (dashes along any path) and `CurvesAsSubmobjects` (a path split
//! into its curves).

use fframes::{Color, Svgr, animation::Animatable};
use kurbo::{BezPath, PathEl, Shape as _};

use super::{Paint, shape};
use crate::{
    animate::{Emphasis, EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    paths,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// Whether the path ends where it starts, as Manim's `is_closed`.
fn is_closed(path: &BezPath) -> bool {
    let els = path.elements();
    if els.last() == Some(&PathEl::ClosePath) {
        return true;
    }
    let (Some(first), Some(last)) = (els.first(), els.last()) else {
        return false;
    };
    match (first, last.end_point()) {
        (PathEl::MoveTo(a), Some(b)) => (*a - b).hypot() < 1e-6,
        _ => false,
    }
}

/// `num_dashes` dashes along `path`, spaced by arc length like Manim's `DashedVMobject`:
/// `dashed_ratio` of the length is dashes, the rest gaps. An open path starts and ends with
/// a dash; a closed one has as many gaps as dashes. `dash_offset` (0 to 1 of a dash and its
/// gap) moves the pattern along. Each dash is one subpath.
pub fn dash_path(
    path: &BezPath,
    num_dashes: usize,
    dashed_ratio: f32,
    dash_offset: f32,
) -> BezPath {
    let length = paths::get_length(path);
    let n = num_dashes;
    if n == 0 || length <= 0.0 {
        return BezPath::new();
    }
    let ratio = f64::from(dashed_ratio.clamp(0.0, 1.0));
    let closed = is_closed(path);
    let dash = ratio / n as f64;
    let gap = match (closed, n) {
        (true, _) => (1.0 - ratio) / n as f64,
        (false, 1) => 1.0 - ratio,
        (false, _) => (1.0 - ratio) / (n - 1) as f64,
    };
    let period = dash + gap;
    let shift = f64::from(dash_offset).rem_euclid(1.0) * period;
    // an open path ends with a dash, so its pattern is one gap longer than the path
    let pattern = if closed { 1.0 } else { 1.0 + gap };
    let mut out = BezPath::new();
    let mut add = |from: f64, to: f64| {
        if to > from {
            out.extend(paths::slice_path(path, from * length, to * length));
        }
    };
    for i in 0..n {
        let start = (i as f64 * period + shift).rem_euclid(pattern);
        let end = start + dash;
        add(start.min(1.0), end.min(1.0));
        if end > pattern {
            add(0.0, end - pattern);
        }
    }
    out
}

shape! {
    /// Dashes along any path, spaced by arc length (Manim `DashedVMobject`). See
    /// [`dash_path`].
    DashedVMobject(t) {
        path: BezPath = BezPath::new(),
        num_dashes: usize = 15,
        dashed_ratio: f32 = 0.5,
        dash_offset: f32 = 0.0,
    }
    paint: Paint::outline(t);
    path(s) {
        dash_path(&s.path, s.num_dashes, s.dashed_ratio, s.dash_offset)
    }
}

/// Every curve (segment) of `path` as its own path.
pub fn curves(path: &BezPath) -> Vec<BezPath> {
    path.segments()
        .map(|seg| BezPath::from_vec(seg.path_elements(0.1).collect()))
        .collect()
}

/// A path split into its curves (Manim `CurvesAsSubmobjects`), each drawn on its own so
/// `Write` draws them one after another. The stroke runs from the first to the last of
/// `colors` along the curves, like Manim's `set_color_by_gradient`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct CurvesAsSubmobjects {
    pub path: BezPath,
    /// The gradient along the curves; one colour paints them all.
    pub colors: Vec<Color>,
    pub stroke_width: f32,
    pub opacity: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub emphasis: Option<Emphasis>,
}

impl Themed for CurvesAsSubmobjects {
    fn themed(t: &Theme) -> Self {
        Self {
            path: BezPath::new(),
            colors: vec![t.colors.accent],
            stroke_width: t.stroke.regular,
            opacity: 1.0,
            enter: None,
            exit: None,
            emphasis: None,
        }
    }
}

impl Default for CurvesAsSubmobjects {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The colour `p` (0 to 1) along evenly spaced `colors`.
fn gradient(colors: &[Color], p: f32) -> Option<Color> {
    let last = colors.len().checked_sub(1)?;
    let at = p.clamp(0.0, 1.0) * last as f32;
    let i = (at.floor() as usize).min(last.saturating_sub(1));
    let next = (i + 1).min(last);
    Some(Animatable::apply_progress(
        &colors[i],
        &colors[next],
        at - i as f32,
    ))
}

impl CurvesAsSubmobjects {
    /// One shape per curve, in path order.
    pub fn shapes(&self) -> Vec<Shape> {
        let parts = curves(&self.path);
        let steps = parts.len().saturating_sub(1).max(1) as f32;
        parts
            .into_iter()
            .enumerate()
            .map(|(i, path)| Shape {
                path,
                fill: None,
                stroke: gradient(&self.colors, i as f32 / steps),
                stroke_width: self.stroke_width,
                dash: None,
                opacity: self.opacity,
            })
            .collect()
    }
}

impl<'a> Component<'a> for CurvesAsSubmobjects {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        draw_animated(
            ui,
            &self.shapes(),
            self.enter.as_ref(),
            self.exit.as_ref(),
            self.emphasis.as_ref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lengths(path: &BezPath) -> Vec<f64> {
        paths::get_subpaths(path)
            .iter()
            .map(paths::get_length)
            .collect()
    }

    #[test]
    fn dashes_on_a_curve_have_equal_lengths() {
        let circle = kurbo::Circle::new((0.0, 0.0), 100.0).to_path(0.05);
        let total = paths::get_length(&circle);
        let dashes = lengths(&dash_path(&circle, 10, 0.5, 0.0));
        assert_eq!(dashes.len(), 10);
        for d in dashes {
            assert!((d - total / 20.0).abs() < 0.05, "{d}");
        }
        // a shifted pattern wraps around the closing point: still ten dashes of the same
        // length, the one over the seam in two pieces
        let shifted = dash_path(&circle, 10, 0.5, 0.3);
        assert!((paths::get_length(&shifted) - total / 2.0).abs() < 0.1);
    }

    #[test]
    fn an_open_path_starts_and_ends_with_a_dash() {
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((100.0, 0.0));
        let dashed = dash_path(&line, 3, 0.6, 0.0);
        let subs = paths::get_subpaths(&dashed);
        assert_eq!(subs.len(), 3);
        let b = dashed.bounding_box();
        assert!(b.x0.abs() < 1e-6 && (b.x1 - 100.0).abs() < 1e-6, "{b:?}");
        for d in lengths(&dashed) {
            assert!((d - 20.0).abs() < 1e-3, "{d}");
        }
    }

    #[test]
    fn curves_split_and_take_the_gradient() {
        let square = kurbo::Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.1);
        let split = CurvesAsSubmobjects {
            path: square,
            colors: vec![Color::rgb(0, 0, 0), Color::rgb(255, 255, 255)],
            ..CurvesAsSubmobjects::default()
        };
        let shapes = split.shapes();
        assert_eq!(shapes.len(), 4);
        assert_eq!(shapes[0].stroke, Some(Color::rgb(0, 0, 0)));
        assert_eq!(shapes[3].stroke, Some(Color::rgb(255, 255, 255)));
    }
}
