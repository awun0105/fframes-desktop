use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Rect};

use super::table::{Cell, TextGrid};
use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// Entries in rows and columns between square brackets (Manim `Matrix`), centered on
/// `(x, y)`. The brackets draw with `enter` (`Create`), the entries fade with
/// `entries_enter`.
#[derive(Debug, Clone, PartialEq)]
pub struct Matrix<'a> {
    /// Row by row; a shorter row is padded with empty entries.
    pub entries: Vec<Vec<Cow<'a, str>>>,
    pub x: f32,
    pub y: f32,
    /// Pixels between the columns and between the rows (Manim `h_buff`, `v_buff`).
    pub buff: (f32, f32),
    /// Pixels between the entries and the brackets (Manim `bracket_h_buff`).
    pub bracket_buff: f32,
    pub style: TextStyle,
    pub fill: Color,
    /// The brackets' colour.
    pub color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub entries_enter: Option<FadeIn>,
    pub entries_exit: Option<FadeOut>,
}

impl Themed for Matrix<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            entries: Vec::new(),
            x: 960.0,
            y: 540.0,
            buff: (t.spacing.xl, t.spacing.lg + t.spacing.md),
            bracket_buff: t.spacing.lg,
            style: t.typography.body,
            fill: t.colors.text,
            color: t.colors.title,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            entries_enter: Some(FadeIn::themed(t)),
            entries_exit: None,
        }
    }
}

impl Default for Matrix<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Matrix<'a> {
    /// Every entry row by row, and the number of columns.
    fn cells(&self) -> (Vec<Cell<'a>>, usize) {
        let columns = self.entries.iter().map(Vec::len).max().unwrap_or(0);
        let cells = self
            .entries
            .iter()
            .flat_map(|row| (0..columns).map(|c| row.get(c).cloned().unwrap_or_default()))
            .map(|text| Cell {
                text,
                style: self.style,
                fill: self.fill,
            })
            .collect();
        (cells, columns)
    }

    fn grid(&self, ui: &Ui<'_, '_, '_>) -> (Vec<Cell<'a>>, TextGrid) {
        let (cells, columns) = self.cells();
        let grid = TextGrid::measure(ui, &cells, columns, self.buff, (self.x, self.y));
        (cells, grid)
    }

    /// The box the brackets span around the entries' `extent`: half a row gap above and
    /// below, `bracket_buff` left and right.
    fn bracket_box(&self, extent: Rect) -> Rect {
        let (dx, dy) = (f64::from(self.bracket_buff), f64::from(self.buff.1) / 2.0);
        Rect::new(
            extent.x0 - dx,
            extent.y0 - dy,
            extent.x1 + dx,
            extent.y1 + dy,
        )
    }

    /// The left and right bracket, each a bar with two short arms pointing inward.
    fn brackets(&self, b: Rect) -> [BezPath; 2] {
        let arm = self.style.size as f64 * 0.3;
        [(b.x0, arm), (b.x1, -arm)].map(|(x, arm)| {
            let mut path = BezPath::new();
            path.move_to((x + arm, b.y0));
            path.line_to((x, b.y0));
            path.line_to((x, b.y1));
            path.line_to((x + arm, b.y1));
            path
        })
    }

    /// The area of entry `(row, column)`, half the gaps around it (Manim `get_entries`), to
    /// put a highlight behind it. `None` outside the matrix.
    pub fn cell(&self, ui: &Ui<'_, '_, '_>, row: usize, column: usize) -> Option<Rect> {
        self.grid(ui).1.cell(row, column)
    }

    /// The box around the brackets, to place what comes next to the matrix. `None` for a
    /// matrix without entries.
    pub fn bounds(&self, ui: &Ui<'_, '_, '_>) -> Option<Rect> {
        self.grid(ui).1.extent().map(|e| self.bracket_box(e))
    }
}

impl<'a> Component<'a> for Matrix<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (cells, grid) = self.grid(ui);
        let Some(extent) = grid.extent() else {
            return Svgr::empty();
        };
        let brackets = self.brackets(self.bracket_box(extent)).map(|path| Shape {
            path,
            fill: None,
            stroke: Some(self.color),
            stroke_width: self.stroke_width,
            dash: None,
            opacity: 1.0,
        });
        let drawn = draw_animated(ui, &brackets, self.enter.as_ref(), self.exit.as_ref(), None);
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
    use crate::geometry::Geometry;

    #[test]
    fn ragged_rows_are_padded() {
        let m = Matrix {
            entries: vec![vec!["1".into(), "2".into()], vec!["3".into()]],
            ..Matrix::default()
        };
        let (cells, columns) = m.cells();
        assert_eq!(columns, 2);
        let texts: Vec<&str> = cells.iter().map(|c| c.text.as_ref()).collect();
        assert_eq!(texts, ["1", "2", "3", ""]);
        assert_eq!(Matrix::default().cells(), (Vec::new(), 0));
    }

    #[test]
    fn brackets_frame_the_entries() {
        let m = Matrix {
            buff: (10.0, 10.0),
            bracket_buff: 8.0,
            ..Matrix::default()
        };
        let b = m.bracket_box(Rect::new(45.0, 85.0, 155.0, 115.0));
        assert_eq!(b, Rect::new(37.0, 80.0, 163.0, 120.0));
        let arm = m.style.size as f64 * 0.3;
        let [left, right] = m.brackets(b);
        let shape = |path: BezPath| Shape {
            path,
            fill: None,
            stroke: None,
            stroke_width: 0.0,
            dash: None,
            opacity: 1.0,
        };
        assert_eq!(shape(left).bbox(), Rect::new(37.0, 80.0, 37.0 + arm, 120.0));
        assert_eq!(
            shape(right).bbox(),
            Rect::new(163.0 - arm, 80.0, 163.0, 120.0)
        );
    }
}
