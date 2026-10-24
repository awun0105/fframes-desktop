//! Manim's `DecimalMatrix`, `IntegerMatrix` and `MobjectMatrix`, and the [`Mobject`] cell
//! the mobject matrix and table share.

use std::borrow::Cow;

use fframes::{Svgr, Transform};
use kurbo::{Affine, Rect, Shape as _};

use crate::{
    geometry::Shape,
    text::format_number,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{Matrix, table::TextGrid};

/// Something drawn around the origin and the box it takes, `width` by `height` pixels: a
/// cell of a [`MobjectMatrix`] or a [`MobjectTable`](super::MobjectTable).
#[derive(Debug, Clone, Default)]
pub struct Mobject<'a> {
    pub svgr: Svgr<'a>,
    pub width: f64,
    pub height: f64,
}

impl<'a> Mobject<'a> {
    pub fn new(svgr: Svgr<'a>, width: f64, height: f64) -> Self {
        Self {
            svgr,
            width,
            height,
        }
    }

    /// Shapes moved so the middle of their box is at the origin, with that box's size.
    pub fn from_shapes(shapes: &[Shape]) -> Self {
        let Some(bounds) = shapes
            .iter()
            .map(|s| s.path.bounding_box())
            .reduce(|a, b| a.union(b))
        else {
            return Self::default();
        };
        let to_origin = Affine::translate(-bounds.center().to_vec2());
        let svgr = shapes
            .iter()
            .map(|s| {
                Shape {
                    path: to_origin * &s.path,
                    ..s.clone()
                }
                .draw()
            })
            .collect();
        Self::new(svgr, bounds.width(), bounds.height())
    }
}

/// Every mobject drawn in the middle of its box.
pub(super) fn place<'a>(cells: impl IntoIterator<Item = (Mobject<'a>, Rect)>) -> Svgr<'a> {
    cells
        .into_iter()
        .map(|(m, b)| {
            let at = b.center();
            let svgr = m.svgr;
            fframes::svgr!(<g transform={Transform::translate(at.x as f32, at.y as f32)}>{svgr}</g>)
        })
        .collect()
}

/// The numbers written with `decimals` places (and a `+` when `include_sign`), row by row.
pub(super) fn written<'a>(
    values: &[Vec<f64>],
    decimals: usize,
    include_sign: bool,
) -> Vec<Vec<Cow<'a, str>>> {
    values
        .iter()
        .map(|row| {
            row.iter()
                .map(|&v| Cow::Owned(format_number(v, decimals, include_sign, true)))
                .collect()
        })
        .collect()
}

/// A [`Matrix`] of numbers written with `num_decimal_places` places (Manim `DecimalMatrix`,
/// one place by default). `matrix` gives the place and look; its `entries` are replaced.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct DecimalMatrix<'a> {
    pub values: Vec<Vec<f64>>,
    pub num_decimal_places: usize,
    pub include_sign: bool,
    pub matrix: Matrix<'a>,
}

impl Themed for DecimalMatrix<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            values: Vec::new(),
            num_decimal_places: 1,
            include_sign: false,
            matrix: Matrix::themed(t),
        }
    }
}

impl Default for DecimalMatrix<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> DecimalMatrix<'a> {
    /// The [`Matrix`] it draws, with the numbers written.
    pub fn to_matrix(&self) -> Matrix<'a> {
        Matrix {
            entries: written(&self.values, self.num_decimal_places, self.include_sign),
            ..self.matrix.clone()
        }
    }
}

impl<'a> Component<'a> for DecimalMatrix<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.to_matrix().render(ui)
    }
}

/// A [`Matrix`] of whole numbers (Manim `IntegerMatrix`). `matrix` gives the place and look;
/// its `entries` are replaced.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct IntegerMatrix<'a> {
    pub values: Vec<Vec<i64>>,
    pub matrix: Matrix<'a>,
}

impl Themed for IntegerMatrix<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            values: Vec::new(),
            matrix: Matrix::themed(t),
        }
    }
}

impl Default for IntegerMatrix<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> IntegerMatrix<'a> {
    /// The [`Matrix`] it draws, with the numbers written.
    pub fn to_matrix(&self) -> Matrix<'a> {
        Matrix {
            entries: self
                .values
                .iter()
                .map(|row| row.iter().map(|v| Cow::Owned(v.to_string())).collect())
                .collect(),
            ..self.matrix.clone()
        }
    }
}

impl<'a> Component<'a> for IntegerMatrix<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.to_matrix().render(ui)
    }
}

/// A [`Matrix`] whose entries are drawn [`Mobject`]s (Manim `MobjectMatrix`), each in the
/// middle of its cell; a column is as wide as its widest entry. `matrix` gives the place,
/// gaps, brackets and animations; its `entries` are not used.
#[derive(Debug, Clone)]
pub struct MobjectMatrix<'a> {
    pub entries: Vec<Vec<Mobject<'a>>>,
    pub matrix: Matrix<'a>,
}

impl Themed for MobjectMatrix<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            entries: Vec::new(),
            matrix: Matrix::themed(t),
        }
    }
}

impl Default for MobjectMatrix<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> MobjectMatrix<'a> {
    /// Every entry row by row, ragged rows padded with empty ones, and the number of columns.
    fn cells(&self) -> (Vec<Mobject<'a>>, usize) {
        let columns = self.entries.iter().map(Vec::len).max().unwrap_or(0);
        let cells = self
            .entries
            .iter()
            .flat_map(|row| (0..columns).map(|c| row.get(c).cloned().unwrap_or_default()))
            .collect();
        (cells, columns)
    }

    fn grid(cells: &[Mobject<'_>], columns: usize, matrix: &Matrix<'_>) -> TextGrid {
        let sizes: Vec<(f64, f64)> = cells.iter().map(|m| (m.width, m.height)).collect();
        let (bx, by) = matrix.buff;
        TextGrid::new(
            &sizes,
            columns,
            (f64::from(bx), f64::from(by)),
            (f64::from(matrix.x), f64::from(matrix.y)),
        )
    }

    /// The area of entry `(row, column)`, half the gaps around it. `None` outside the matrix.
    pub fn cell(&self, row: usize, column: usize) -> Option<Rect> {
        let (cells, columns) = self.cells();
        Self::grid(&cells, columns, &self.matrix).cell(row, column)
    }
}

impl<'a> Component<'a> for MobjectMatrix<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (cells, columns) = self.cells();
        let grid = Self::grid(&cells, columns, &self.matrix);
        let body = place(cells.into_iter().zip(grid.boxes.iter().copied()));
        self.matrix.draw(ui, &grid, body)
    }
}

#[cfg(test)]
mod tests {
    use kurbo::Circle;

    use super::*;

    #[test]
    fn numbers_are_written_like_decimal_number() {
        let m = DecimalMatrix {
            values: vec![vec![1.0, -2.75], vec![1234.5]],
            ..DecimalMatrix::default()
        };
        let entries = m.to_matrix().entries;
        assert_eq!(entries, vec![vec!["1.0", "-2.8"], vec!["1,234.5"]]);
        let signed = DecimalMatrix {
            include_sign: true,
            num_decimal_places: 0,
            ..m
        };
        assert_eq!(signed.to_matrix().entries[0], vec!["+1", "-3"]);
        let ints = IntegerMatrix {
            values: vec![vec![3, -40]],
            ..IntegerMatrix::default()
        };
        assert_eq!(ints.to_matrix().entries, vec![vec!["3", "-40"]]);
    }

    #[test]
    fn mobjects_are_centered_in_cells_as_wide_as_their_column() {
        let dot = |r: f64| {
            Mobject::from_shapes(&[Shape {
                path: Circle::new((500.0, 500.0), r).to_path(0.1),
                fill: None,
                stroke: None,
                stroke_width: 0.0,
                dash: None,
                opacity: 1.0,
            }])
        };
        let small = dot(10.0);
        assert!((small.width - 20.0).abs() < 1e-6 && (small.height - 20.0).abs() < 1e-6);
        let m = MobjectMatrix {
            entries: vec![vec![dot(10.0), dot(30.0)], vec![dot(20.0)]],
            matrix: Matrix {
                x: 0.0,
                y: 0.0,
                buff: (10.0, 10.0),
                ..Matrix::default()
            },
        };
        // columns 40 and 60 wide; the missing entry is empty
        let first = m.cell(0, 0).unwrap();
        let second = m.cell(0, 1).unwrap();
        assert!((first.width() - 50.0).abs() < 1e-6, "{first:?}");
        assert!((second.width() - 70.0).abs() < 1e-6, "{second:?}");
        assert!(m.cell(1, 1).is_some() && m.cell(2, 0).is_none());
    }
}
