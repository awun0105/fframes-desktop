//! [`Table`], and the grid of text cells it shares with [`Matrix`](super::Matrix).

use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::{Point, Rect, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    label::text_size,
    layout,
    shapes::Line,
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// The text of one cell and its look.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Cell<'a> {
    pub text: Cow<'a, str>,
    pub style: TextStyle,
    pub fill: Color,
}

/// Boxes of text cells on [`layout::grid`], row by row around a center, with the span of
/// every column and row in pixels.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TextGrid {
    pub boxes: Vec<Rect>,
    pub columns: Vec<(f64, f64)>,
    pub rows: Vec<(f64, f64)>,
    pub gap: (f64, f64),
}

/// The smallest start and the largest end.
fn span(spans: impl Iterator<Item = (f64, f64)>) -> (f64, f64) {
    spans.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), (x0, x1)| {
        (a.min(x0), b.max(x1))
    })
}

impl TextGrid {
    /// `sizes` fill whole rows of `columns` cells.
    pub fn new(sizes: &[(f64, f64)], columns: usize, gap: (f64, f64), center: (f64, f64)) -> Self {
        let columns = columns.max(1);
        let shift = Vec2::new(center.0, center.1);
        let boxes: Vec<Rect> = layout::grid(sizes, columns, gap)
            .into_iter()
            .map(|b| b + shift)
            .collect();
        let column_spans = (0..columns.min(boxes.len()))
            .map(|c| span(boxes.iter().skip(c).step_by(columns).map(|b| (b.x0, b.x1))))
            .collect();
        let row_spans = boxes
            .chunks(columns)
            .map(|row| span(row.iter().map(|b| (b.y0, b.y1))))
            .collect();
        Self {
            boxes,
            columns: column_spans,
            rows: row_spans,
            gap,
        }
    }

    /// Measures the text of `cells` with the font and places them like [`TextGrid::new`].
    pub fn measure(
        ui: &Ui<'_, '_, '_>,
        cells: &[Cell<'_>],
        columns: usize,
        gap: (f32, f32),
        center: (f32, f32),
    ) -> Self {
        let sizes: Vec<(f64, f64)> = cells
            .iter()
            .map(|c| {
                let (w, h) = text_size(ui, c.style, &c.text);
                (f64::from(w), f64::from(h))
            })
            .collect();
        Self::new(
            &sizes,
            columns,
            (f64::from(gap.0), f64::from(gap.1)),
            (f64::from(center.0), f64::from(center.1)),
        )
    }

    /// The area of a cell: the span of its column and row with half the gap around them.
    pub fn cell(&self, row: usize, column: usize) -> Option<Rect> {
        let (&(x0, x1), &(y0, y1)) = (self.columns.get(column)?, self.rows.get(row)?);
        let (gx, gy) = (self.gap.0 / 2.0, self.gap.1 / 2.0);
        Some(Rect::new(x0 - gx, y0 - gy, x1 + gx, y1 + gy))
    }

    /// The box around every cell's text, `None` without cells.
    pub fn extent(&self) -> Option<Rect> {
        self.boxes.iter().copied().reduce(|a, b| a.union(b))
    }

    /// Every non-empty cell as a `<text>` centered in its box.
    pub fn texts<'a>(&self, cells: Vec<Cell<'_>>) -> Svgr<'a> {
        cells
            .into_iter()
            .zip(&self.boxes)
            .filter(|(c, _)| !c.text.is_empty())
            .map(|(c, b)| {
                line(
                    b.center().x as f32,
                    b.y1 as f32,
                    c.style,
                    c.fill,
                    TextAnchor::Middle,
                    0.0,
                    1.0,
                    c.text,
                )
            })
            .collect()
    }

    /// The lines between the columns and between the rows, from one outer edge to the
    /// other, and the outer frame when `outer` is set.
    fn lines(&self, outer: bool) -> Vec<(Point, Point)> {
        let (Some(first_c), Some(last_c), Some(first_r), Some(last_r)) = (
            self.columns.first(),
            self.columns.last(),
            self.rows.first(),
            self.rows.last(),
        ) else {
            return Vec::new();
        };
        let (gx, gy) = (self.gap.0 / 2.0, self.gap.1 / 2.0);
        let (left, right) = (first_c.0 - gx, last_c.1 + gx);
        let (top, bottom) = (first_r.0 - gy, last_r.1 + gy);
        let between = |spans: &[(f64, f64)]| -> Vec<f64> {
            spans
                .windows(2)
                .map(|w| f64::midpoint(w[0].1, w[1].0))
                .collect()
        };
        let mut xs = between(&self.columns);
        let mut ys = between(&self.rows);
        if outer {
            xs.extend([left, right]);
            ys.extend([top, bottom]);
        }
        let vertical = xs
            .into_iter()
            .map(|x| (Point::new(x, top), Point::new(x, bottom)));
        let horizontal = ys
            .into_iter()
            .map(|y| (Point::new(left, y), Point::new(right, y)));
        vertical.chain(horizontal).collect()
    }
}

/// Text in rows and columns with lines between them (Manim `Table`), centered on `(x, y)`.
/// `col_labels` go above the columns and `row_labels` left of the rows, in `label_style`.
/// The lines draw with `enter` (`Create`), the text fades with `entries_enter`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Table<'a> {
    /// The body, row by row; a shorter row is padded with empty cells.
    pub rows: Vec<Vec<Cow<'a, str>>>,
    /// Labels above the columns, empty for none.
    pub col_labels: Vec<Cow<'a, str>>,
    /// Labels left of the rows, empty for none.
    pub row_labels: Vec<Cow<'a, str>>,
    pub x: f32,
    pub y: f32,
    /// Pixels between the columns and between the rows (Manim `h_buff`, `v_buff`); the
    /// lines run through the middle of them.
    pub buff: (f32, f32),
    pub style: TextStyle,
    pub fill: Color,
    pub label_style: TextStyle,
    pub label_fill: Color,
    pub line_color: Color,
    pub stroke_width: f32,
    /// Draws the frame around the table too (Manim `include_outer_lines`).
    pub outer_lines: bool,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub entries_enter: Option<FadeIn>,
    pub entries_exit: Option<FadeOut>,
}

impl Themed for Table<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            rows: Vec::new(),
            col_labels: Vec::new(),
            row_labels: Vec::new(),
            x: 960.0,
            y: 540.0,
            buff: (t.spacing.xl, t.spacing.lg + t.spacing.md),
            style: t.typography.body,
            fill: t.colors.text,
            label_style: TextStyle {
                weight: 600,
                ..t.typography.body
            },
            label_fill: t.colors.accent,
            line_color: t.colors.muted,
            stroke_width: t.stroke.thin,
            outer_lines: false,
            enter: None,
            exit: None,
            entries_enter: Some(FadeIn::themed(t)),
            entries_exit: None,
        }
    }
}

impl Default for Table<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Table<'a> {
    /// Rows and columns taken by the labels.
    fn label_offsets(&self) -> (usize, usize) {
        (
            usize::from(!self.col_labels.is_empty()),
            usize::from(!self.row_labels.is_empty()),
        )
    }

    /// Every cell row by row, labels included, and the number of columns.
    fn cells(&self) -> (Vec<Cell<'a>>, usize) {
        let (label_rows, label_cols) = self.label_offsets();
        let body_cols = self
            .rows
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0)
            .max(self.col_labels.len());
        let body_rows = self.rows.len().max(self.row_labels.len());
        if body_cols == 0 && body_rows == 0 {
            return (Vec::new(), 0);
        }
        let columns = body_cols + label_cols;
        let mut cells = Vec::with_capacity(columns * (body_rows + label_rows));
        for r in 0..body_rows + label_rows {
            for c in 0..columns {
                let (text, label) = match (r < label_rows, c < label_cols) {
                    (true, true) => (None, true),
                    (true, false) => (self.col_labels.get(c - label_cols), true),
                    (false, true) => (self.row_labels.get(r - label_rows), true),
                    (false, false) => (
                        self.rows
                            .get(r - label_rows)
                            .and_then(|row| row.get(c - label_cols)),
                        false,
                    ),
                };
                let (style, fill) = if label {
                    (self.label_style, self.label_fill)
                } else {
                    (self.style, self.fill)
                };
                cells.push(Cell {
                    text: text.cloned().unwrap_or_default(),
                    style,
                    fill,
                });
            }
        }
        (cells, columns)
    }

    fn grid(&self, ui: &Ui<'_, '_, '_>, cells: &[Cell<'_>], columns: usize) -> TextGrid {
        TextGrid::measure(ui, cells, columns, self.buff, (self.x, self.y))
    }

    /// The area of body cell `(row, column)` between its lines (Manim `get_cell`), to put a
    /// highlight behind it. `None` outside the table.
    pub fn cell(&self, ui: &Ui<'_, '_, '_>, row: usize, column: usize) -> Option<Rect> {
        let (cells, columns) = self.cells();
        let (label_rows, label_cols) = self.label_offsets();
        self.grid(ui, &cells, columns)
            .cell(row + label_rows, column + label_cols)
    }
}

impl<'a> Component<'a> for Table<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (cells, columns) = self.cells();
        if cells.is_empty() {
            return Svgr::empty();
        }
        let grid = self.grid(ui, &cells, columns);
        let lines: Vec<Shape> = grid
            .lines(self.outer_lines)
            .into_iter()
            .map(|(a, b)| {
                Line {
                    x1: a.x as f32,
                    y1: a.y as f32,
                    x2: b.x as f32,
                    y2: b.y as f32,
                    stroke: Some(self.line_color),
                    stroke_width: self.stroke_width,
                    ..ui.defaults()
                }
                .shape()
            })
            .collect();
        let drawn = draw_animated(ui, &lines, self.enter.as_ref(), self.exit.as_ref(), None);
        let texts = fade(
            ui,
            grid.texts(cells),
            self.entries_enter.as_ref(),
            self.entries_exit.as_ref(),
        );
        fframes::svgr!(<g>{drawn}{texts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2 x 2 grid: columns 40 and 60 wide, rows 10 tall, 10 apart, around (100, 100).
    fn two_by_two() -> TextGrid {
        TextGrid::new(
            &[(40.0, 10.0), (20.0, 10.0), (30.0, 10.0), (60.0, 10.0)],
            2,
            (10.0, 10.0),
            (100.0, 100.0),
        )
    }

    #[test]
    fn cells_span_their_columns_and_rows() {
        let grid = two_by_two();
        assert_eq!(grid.extent(), Some(Rect::new(45.0, 85.0, 155.0, 115.0)));
        assert_eq!(grid.columns, vec![(45.0, 85.0), (95.0, 155.0)]);
        assert_eq!(grid.rows, vec![(85.0, 95.0), (105.0, 115.0)]);
        // the 20 px entry is centered in its 60 px column
        assert_eq!(grid.boxes[1], Rect::new(115.0, 85.0, 135.0, 95.0));
        assert_eq!(grid.cell(0, 1), Some(Rect::new(90.0, 80.0, 160.0, 100.0)));
        assert_eq!(grid.cell(2, 0), None);
        assert_eq!(grid.cell(0, 2), None);
    }

    #[test]
    fn lines_run_between_cells() {
        let grid = two_by_two();
        assert_eq!(
            grid.lines(false),
            vec![
                (Point::new(90.0, 80.0), Point::new(90.0, 120.0)),
                (Point::new(40.0, 100.0), Point::new(160.0, 100.0)),
            ]
        );
        assert_eq!(grid.lines(true).len(), 6);
        assert_eq!(
            TextGrid::new(&[], 3, (1.0, 1.0), (0.0, 0.0)).lines(true),
            Vec::new()
        );
    }

    #[test]
    fn labels_and_ragged_rows_fill_the_grid() {
        let table = Table {
            rows: vec![vec!["1".into(), "2".into(), "3".into()], vec!["4".into()]],
            col_labels: vec!["a".into(), "b".into()],
            row_labels: vec!["x".into(), "y".into()],
            ..Table::default()
        };
        let (cells, columns) = table.cells();
        assert_eq!(columns, 4);
        let texts: Vec<&str> = cells.iter().map(|c| c.text.as_ref()).collect();
        assert_eq!(
            texts,
            ["", "a", "b", "", "x", "1", "2", "3", "y", "4", "", ""]
        );
        assert_eq!(cells[1].style, table.label_style);
        assert_eq!(cells[5].style, table.style);
        assert_eq!(Table::default().cells(), (Vec::new(), 0));
    }
}
