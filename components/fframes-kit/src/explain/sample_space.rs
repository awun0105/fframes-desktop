//! Manim's `SampleSpace`.

use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::{Rect, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    shapes::Brace,
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::Space;

/// A part of a [`SampleColumn`]: its probability given the column and its label.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleRow<'a> {
    pub p: f64,
    pub label: Cow<'a, str>,
}

impl<'a> SampleRow<'a> {
    pub fn new(p: f64, label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            p,
            label: label.into(),
        }
    }
}

/// A vertical strip of a [`SampleSpace`] (Manim `divide_vertically`), as wide as its
/// probability, divided into `rows` from the top (Manim `divide_horizontally`).
#[derive(Debug, Clone, PartialEq)]
pub struct SampleColumn<'a> {
    pub p: f64,
    /// Written under a brace below the column.
    pub label: Cow<'a, str>,
    pub rows: Vec<SampleRow<'a>>,
}

impl<'a> SampleColumn<'a> {
    pub fn new(p: f64, label: impl Into<Cow<'a, str>>, rows: Vec<SampleRow<'a>>) -> Self {
        Self {
            p,
            label: label.into(),
            rows,
        }
    }
}

/// A rectangle of probabilities (Manim `SampleSpace`): `columns` split it left to right by
/// their `p`, and each column's `rows` split it top to bottom, so the area of a cell is the
/// probability of both. Probabilities that do not add up to 1 are scaled so they fill the
/// rectangle. A row is coloured by its index in `colors`, a column without rows by its own.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct SampleSpace<'a> {
    pub columns: Vec<SampleColumn<'a>>,
    /// The middle, in units of `ui.space`.
    pub center: (f64, f64),
    /// In units of `ui.space`.
    pub width: f64,
    pub height: f64,
    pub colors: Vec<Color>,
    pub fill_opacity: f32,
    pub stroke: Color,
    pub stroke_width: f32,
    pub label_style: TextStyle,
    pub label_fill: Color,
    pub brace_depth: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    /// How the labels and braces appear and go; the cells use `enter` and `exit`.
    pub labels_enter: Option<FadeIn>,
    pub labels_exit: Option<FadeOut>,
}

impl Themed for SampleSpace<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            columns: Vec::new(),
            center: (0.0, 0.0),
            width: 6.0,
            height: 4.0,
            colors: vec![t.colors.accent, t.colors.highlight, t.colors.muted],
            fill_opacity: 0.75,
            stroke: t.colors.text,
            stroke_width: t.stroke.thin,
            label_style: t.typography.body,
            label_fill: t.colors.title,
            brace_depth: 18.0,
            enter: None,
            exit: None,
            labels_enter: None,
            labels_exit: None,
        }
    }
}

impl Default for SampleSpace<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The share of the total of each of `ps`, in order; negative values count as 0.
fn shares(ps: impl Iterator<Item = f64> + Clone) -> Vec<f64> {
    let total: f64 = ps.clone().map(|p| p.max(0.0)).sum();
    ps.map(|p| if total > 0.0 { p.max(0.0) / total } else { 0.0 })
        .collect()
}

impl SampleSpace<'_> {
    /// The whole rectangle in pixels of `space`.
    pub fn bounds_in(&self, space: Space) -> Rect {
        let (cx, cy) = self.center;
        let (w, h) = (self.width / 2.0, self.height / 2.0);
        Rect::from_points(space.to_px(cx - w, cy + h), space.to_px(cx + w, cy - h))
    }

    /// The pixel rectangle of every column and of every cell in it, `(column, cells)`.
    pub fn cells_in(&self, space: Space) -> Vec<(Rect, Vec<Rect>)> {
        let bounds = self.bounds_in(space);
        let mut x = bounds.x0;
        let widths = shares(self.columns.iter().map(|c| c.p));
        self.columns
            .iter()
            .zip(widths)
            .map(|(column, share)| {
                let rect = Rect::new(x, bounds.y0, x + share * bounds.width(), bounds.y1);
                x = rect.x1;
                let mut y = rect.y0;
                let cells = shares(column.rows.iter().map(|r| r.p))
                    .into_iter()
                    .map(|share| {
                        let cell = Rect::new(rect.x0, y, rect.x1, y + share * rect.height());
                        y = cell.y1;
                        cell
                    })
                    .collect();
                (rect, cells)
            })
            .collect()
    }

    fn color(&self, i: usize) -> Color {
        match self.colors.len() {
            0 => self.stroke,
            n => self.colors[i % n],
        }
    }
}

impl<'a> Component<'a> for SampleSpace<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let cells = self.cells_in(ui.space);
        let cell = |rect: Rect, fill: Color| Shape {
            path: rect.to_path(0.1),
            fill: Some(fill.with_alpha((self.fill_opacity.clamp(0.0, 1.0) * 255.0).round() as u8)),
            stroke: Some(self.stroke),
            stroke_width: self.stroke_width,
            dash: None,
            opacity: 1.0,
        };
        let mut shapes = Vec::new();
        for (i, (column, rows)) in cells.iter().enumerate() {
            if rows.is_empty() {
                shapes.push(cell(*column, self.color(i)));
            }
            for (j, row) in rows.iter().enumerate() {
                shapes.push(cell(*row, self.color(j)));
            }
        }
        let drawn = draw_animated(ui, &shapes, self.enter.as_ref(), self.exit.as_ref(), None);
        let s = self.label_style;
        let size = s.size as f32;
        let mut labels: Vec<Svgr<'a>> = Vec::new();
        for (column, (rect, rows)) in self.columns.iter().zip(&cells) {
            for (row, rect) in column.rows.iter().zip(rows) {
                let c = rect.center();
                labels.push(line(
                    c.x as f32,
                    c.y as f32 + size * 0.36,
                    s,
                    self.label_fill,
                    TextAnchor::Middle,
                    0.0,
                    1.0,
                    row.label.clone(),
                ));
            }
            if column.label.is_empty() {
                continue;
            }
            let gap = f64::from(self.stroke_width) + 6.0;
            let brace = Brace {
                x1: rect.x0 as f32,
                y1: (rect.y1 + gap) as f32,
                x2: rect.x1 as f32,
                y2: (rect.y1 + gap) as f32,
                depth: self.brace_depth,
                stroke: Some(self.stroke),
                ..ui.defaults()
            };
            let (tx, ty) = brace.tip();
            labels.push(ui.show(brace));
            labels.push(line(
                tx,
                ty + size * 1.1,
                s,
                self.label_fill,
                TextAnchor::Middle,
                0.0,
                1.0,
                column.label.clone(),
            ));
        }
        let labels = fade(
            ui,
            labels.into_iter().collect(),
            self.labels_enter.as_ref(),
            self.labels_exit.as_ref(),
        );
        fframes::svgr!(<g>{drawn}{labels}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_areas_are_the_joint_probabilities() {
        let space = SampleSpace {
            columns: vec![
                SampleColumn::new(
                    0.3,
                    "A",
                    vec![SampleRow::new(0.8, "B"), SampleRow::new(0.2, "not B")],
                ),
                SampleColumn::new(0.7, "not A", vec![SampleRow::new(0.1, "B")]),
            ],
            ..SampleSpace::default()
        };
        let bounds = space.bounds_in(Space::FULL_HD);
        assert_eq!(bounds.width(), 6.0 * 135.0);
        let cells = space.cells_in(Space::FULL_HD);
        let area = |r: &Rect| r.area() / bounds.area();
        assert!((area(&cells[0].1[0]) - 0.24).abs() < 1e-9);
        assert!((area(&cells[0].1[1]) - 0.06).abs() < 1e-9);
        // a single row fills its column, the columns fill the rectangle
        assert!((area(&cells[1].1[0]) - 0.7).abs() < 1e-9);
        assert!((cells[1].0.x1 - bounds.x1).abs() < 1e-9);
        // the first row is at the top
        assert_eq!(cells[0].1[0].y0, bounds.y0);
        assert_eq!(shares([0.0, 0.0].into_iter()), [0.0, 0.0]);
    }
}
