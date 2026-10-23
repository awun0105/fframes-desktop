//! Linear algebra on the plane: [`LinearTransform`] and [`VectorSum`].

use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::Point;

use crate::{
    anim::{FadeIn, RateFunc, fade},
    animate::{EnterAnim, draw_animated},
    explain::{Matrix, Range, Space},
    geometry::Shape,
    label::Label,
    layout::Anchor,
    motion::progress,
    shapes::{Line, Vector},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::short;

/// A 2x2 matrix, row by row.
pub type Mat2 = [[f64; 2]; 2];

/// `m` times `(x, y)`.
pub fn apply(m: Mat2, (x, y): (f64, f64)) -> (f64, f64) {
    (m[0][0] * x + m[0][1] * y, m[1][0] * x + m[1][1] * y)
}

/// The matrix `p` of the way from the identity to `m` (Manim's `ApplyMatrix` moves every
/// point along a straight line).
pub fn towards(m: Mat2, p: f64) -> Mat2 {
    let id = [[1.0, 0.0], [0.0, 1.0]];
    std::array::from_fn(|r| std::array::from_fn(|c| id[r][c] + (m[r][c] - id[r][c]) * p))
}

/// A grid of lines and the two basis vectors moved by `matrix` (Manim's
/// `LinearTransformationScene` with `ApplyMatrix`): from the identity to the matrix over
/// `run_time` seconds after `delay`, over the untransformed grid drawn faintly. The grid
/// has a line at every integer of `x_range` and `y_range`, in units of `ui.space` around
/// `center`, and the moved grid is clipped to its box; `key` tells the clip paths of two
/// transforms apart. With `show_matrix` the matrix is written at `matrix_at` (pixels).
#[derive(Debug, Clone)]
pub struct LinearTransform<'a> {
    pub matrix: Mat2,
    pub x_range: Range,
    pub y_range: Range,
    pub center: (f64, f64),
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
    pub line_color: Color,
    pub stroke_width: f32,
    pub i_color: Color,
    pub j_color: Color,
    pub show_matrix: bool,
    pub matrix_at: (f32, f32),
    /// How the grids and the vectors appear.
    pub enter: Option<EnterAnim>,
    pub matrix_enter: Option<FadeIn>,
    pub key: Option<&'a str>,
}

impl Themed for LinearTransform<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            matrix: [[1.0, 1.0], [0.0, 1.0]],
            x_range: Range::new(-4.0, 4.0, 1.0),
            y_range: Range::new(-3.0, 3.0, 1.0),
            center: (0.0, 0.0),
            delay: 0.0,
            run_time: 2.0,
            rate_func: RateFunc::default(),
            line_color: t.colors.accent,
            stroke_width: t.stroke.thin,
            i_color: t.colors.highlight,
            j_color: t.colors.caption,
            show_matrix: true,
            matrix_at: (1600.0, 200.0),
            enter: None,
            matrix_enter: Some(FadeIn::themed(t)),
            key: None,
        }
    }
}

impl Default for LinearTransform<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl LinearTransform<'_> {
    /// The matrix applied `t` seconds in.
    fn at(&self, t: f32) -> Mat2 {
        let p = progress(t, self.delay, self.delay + self.run_time);
        towards(
            self.matrix,
            f64::from(self.rate_func.apply(p, self.run_time)),
        )
    }

    /// The ends of every grid line moved by `m`, in units of the space.
    fn lines(&self, m: Mat2) -> Vec<((f64, f64), (f64, f64))> {
        let (x, y) = (self.x_range, self.y_range);
        let moved = |p| {
            let (px, py) = apply(m, p);
            (self.center.0 + px, self.center.1 + py)
        };
        let columns = x.ticks().into_iter().map(|v| ((v, y.min), (v, y.max)));
        let rows = y.ticks().into_iter().map(|v| ((x.min, v), (x.max, v)));
        columns
            .chain(rows)
            .map(|(a, b)| (moved(a), moved(b)))
            .collect()
    }

    fn grid(&self, ui: &Ui<'_, '_, '_>, m: Mat2, color: Color, opacity: f32) -> Vec<Shape> {
        self.lines(m)
            .into_iter()
            .map(|(a, b)| {
                let (a, b) = (ui.space.to_px(a.0, a.1), ui.space.to_px(b.0, b.1));
                Line {
                    x1: a.x as f32,
                    y1: a.y as f32,
                    x2: b.x as f32,
                    y2: b.y as f32,
                    stroke: Some(color),
                    stroke_width: self.stroke_width,
                    opacity,
                    ..ui.defaults()
                }
                .shape()
            })
            .collect()
    }
}

fn vector(
    ui: &Ui<'_, '_, '_>,
    space: Space,
    from: (f64, f64),
    by: (f64, f64),
    color: Color,
) -> Shape {
    let a = space.to_px(from.0, from.1);
    let b = space.to_px(from.0 + by.0, from.1 + by.1);
    Vector {
        x: a.x as f32,
        y: a.y as f32,
        dx: (b.x - a.x) as f32,
        dy: (b.y - a.y) as f32,
        fill: Some(color),
        stroke: Some(color),
        ..ui.defaults()
    }
    .shape()
}

impl<'a> Component<'a> for LinearTransform<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let m = self.at(ui.seconds());
        let muted = ui.theme.colors.muted;
        let enter = self.enter.as_ref();
        let still = self.grid(ui, towards(self.matrix, 0.0), muted, 0.5);
        let still = draw_animated(ui, &still, enter, None, None);
        let mut moved = self.grid(ui, m, self.line_color, 1.0);
        moved.push(vector(
            ui,
            ui.space,
            self.center,
            apply(m, (1.0, 0.0)),
            self.i_color,
        ));
        moved.push(vector(
            ui,
            ui.space,
            self.center,
            apply(m, (0.0, 1.0)),
            self.j_color,
        ));
        let moved = draw_animated(ui, &moved, enter, None, None);

        let (x, y) = (self.x_range, self.y_range);
        let corner = |a: f64, b: f64| ui.space.to_px(self.center.0 + a, self.center.1 + b);
        let (top_left, bottom_right) = (corner(x.min, y.max), corner(x.max, y.min));
        let id = ui.id(self.key, 0, "linear-transform");
        let url = format!("url(#{id})");
        let (cx, cy) = (top_left.x as f32, top_left.y as f32);
        let (w, h) = (
            (bottom_right.x - top_left.x) as f32,
            (bottom_right.y - top_left.y) as f32,
        );
        let matrix = if self.show_matrix {
            let entries = self
                .matrix
                .iter()
                .map(|row| row.iter().map(|&v| Cow::Owned(short(v, 2))).collect())
                .collect();
            ui.show(Matrix {
                entries,
                x: self.matrix_at.0,
                y: self.matrix_at.1,
                enter: self.enter,
                entries_enter: self.matrix_enter,
                ..ui.defaults()
            })
        } else {
            Svgr::empty()
        };
        fframes::svgr!(
            <g>
                <defs>
                    <clipPath id={id}>
                        <rect x={cx} y={cy} width={w} height={h} />
                    </clipPath>
                </defs>
                {still}
                <g clip-path={url}>{moved}</g>
                {matrix}
            </g>
        )
    }
}

/// Vectors added tip to tail (Manim's vector addition): every vector starts at `origin`,
/// then moves over `run_time` seconds after `delay` to the tip of the one before, and the
/// resultant from `origin` to the last tip draws with `resultant_enter` (a `Create`
/// delayed past the move). Vectors are in units of `ui.space`; `labels` are written at the
/// middle of the vectors, on their left, and `resultant_label` on the side of the resultant
/// away from them.
#[derive(Debug, Clone)]
pub struct VectorSum<'a> {
    pub vectors: Vec<(f64, f64)>,
    pub origin: (f64, f64),
    /// Colours of the vectors in turn; empty for the theme's accent and highlight.
    pub colors: Vec<Color>,
    pub labels: Vec<Cow<'a, str>>,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
    pub resultant: bool,
    pub resultant_color: Color,
    pub resultant_label: Cow<'a, str>,
    pub enter: Option<EnterAnim>,
    pub resultant_enter: Option<EnterAnim>,
    pub labels_enter: Option<FadeIn>,
}

impl Themed for VectorSum<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            vectors: Vec::new(),
            origin: (0.0, 0.0),
            colors: Vec::new(),
            labels: Vec::new(),
            delay: 0.0,
            run_time: 1.5,
            rate_func: RateFunc::default(),
            resultant: true,
            resultant_color: t.colors.title,
            resultant_label: Cow::Borrowed(""),
            enter: None,
            resultant_enter: None,
            labels_enter: None,
        }
    }
}

impl Default for VectorSum<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl VectorSum<'_> {
    /// Where each vector starts `p` (0 to 1) of the way through the move.
    fn starts(&self, p: f64) -> Vec<(f64, f64)> {
        self.vectors
            .iter()
            .scan(self.origin, |tip, &(dx, dy)| {
                let start = *tip;
                *tip = (tip.0 + dx, tip.1 + dy);
                Some(start)
            })
            .map(|(x, y)| {
                (
                    self.origin.0 + (x - self.origin.0) * p,
                    self.origin.1 + (y - self.origin.1) * p,
                )
            })
            .collect()
    }

    /// The sum of the vectors.
    pub fn total(&self) -> (f64, f64) {
        self.vectors
            .iter()
            .fold((0.0, 0.0), |(x, y), &(dx, dy)| (x + dx, y + dy))
    }

    /// Whether the first vector turns clockwise from the resultant, so the vectors run on
    /// its right.
    fn chain_on_right(&self) -> bool {
        let ((tx, ty), (x, y)) = (
            self.total(),
            self.vectors.first().copied().unwrap_or_default(),
        );
        tx * y - ty * x < 0.0
    }
}

/// A label at the middle of the vector from `a` to `b` (pixels), on its left on screen.
fn side_label<'a>(ui: &Ui<'a, '_, '_>, text: Cow<'a, str>, a: Point, b: Point) -> Label<'a> {
    let d = b - a;
    let at = a.midpoint(b);
    Label {
        text,
        x: at.x as f32,
        y: at.y as f32,
        direction: Anchor::toward(d.y, -d.x),
        buff: ui.theme.spacing.md,
        style: ui.theme.typography.caption,
        ..ui.defaults()
    }
}

impl<'a> Component<'a> for VectorSum<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = progress(ui.seconds(), self.delay, self.delay + self.run_time);
        let p = f64::from(self.rate_func.apply(t, self.run_time));
        let defaults = [ui.theme.colors.accent, ui.theme.colors.highlight];
        let palette: &[Color] = if self.colors.is_empty() {
            &defaults
        } else {
            &self.colors
        };
        let space = ui.space;
        let starts = self.starts(p);
        let mut drawn = Vec::new();
        let mut labels = Vec::new();
        for (i, (&by, &from)) in self.vectors.iter().zip(&starts).enumerate() {
            let color = palette[i % palette.len()];
            let shape = vector(ui, space, from, by, color);
            drawn.push(draw_animated(ui, &[shape], self.enter.as_ref(), None, None));
            if let Some(text) = self.labels.get(i) {
                let (a, b) = (
                    space.to_px(from.0, from.1),
                    space.to_px(from.0 + by.0, from.1 + by.1),
                );
                labels.push(side_label(ui, text.clone(), a, b));
            }
        }
        if self.resultant && !self.vectors.is_empty() {
            let total = self.total();
            let shape = vector(ui, space, self.origin, total, self.resultant_color);
            drawn.push(draw_animated(
                ui,
                &[shape],
                self.resultant_enter.as_ref(),
                None,
                None,
            ));
            if !self.resultant_label.is_empty() {
                let (a, b) = (
                    space.to_px(self.origin.0, self.origin.1),
                    space.to_px(self.origin.0 + total.0, self.origin.1 + total.1),
                );
                // the resultant's label goes on the side away from the chain of vectors
                let (a, b) = if self.chain_on_right() {
                    (a, b)
                } else {
                    (b, a)
                };
                labels.push(side_label(ui, self.resultant_label.clone(), a, b));
            }
        }
        let labels: Vec<Svgr> = labels.into_iter().map(|l| ui.show(l)).collect();
        let labels = fade(ui, labels.into(), self.labels_enter.as_ref(), None);
        let drawn: Svgr = drawn.into();
        fframes::svgr!(<g>{drawn}{labels}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrices_move_points_from_the_identity() {
        let shear = [[1.0, 1.0], [0.0, 1.0]];
        assert_eq!(apply(shear, (2.0, 3.0)), (5.0, 3.0));
        assert_eq!(towards(shear, 0.0), [[1.0, 0.0], [0.0, 1.0]]);
        assert_eq!(towards(shear, 0.5), [[1.0, 0.5], [0.0, 1.0]]);
        let lt = LinearTransform {
            matrix: shear,
            x_range: Range::new(-1.0, 1.0, 1.0),
            y_range: Range::new(0.0, 1.0, 1.0),
            delay: 1.0,
            run_time: 2.0,
            rate_func: RateFunc::Linear,
            ..LinearTransform::default()
        };
        assert_eq!(lt.at(0.0), towards(shear, 0.0));
        assert_eq!(lt.at(2.0), towards(shear, 0.5));
        // three columns and two rows, the vertical x = 1 sheared to the right at the top
        let lines = lt.lines(shear);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[2], ((1.0, 0.0), (2.0, 1.0)));
    }

    #[test]
    fn vectors_move_tip_to_tail() {
        let sum = VectorSum {
            vectors: vec![(2.0, 0.0), (1.0, 1.0), (0.0, -3.0)],
            origin: (1.0, 1.0),
            ..VectorSum::default()
        };
        assert_eq!(sum.starts(0.0), vec![(1.0, 1.0); 3]);
        assert_eq!(sum.starts(1.0), vec![(1.0, 1.0), (3.0, 1.0), (4.0, 2.0)]);
        assert_eq!(sum.starts(0.5)[2], (2.5, 1.5));
        assert_eq!(sum.total(), (3.0, -2.0));
        // (2, 0) turns counterclockwise from (3, -2): the vectors run on its left
        assert!(!sum.chain_on_right());
    }
}
