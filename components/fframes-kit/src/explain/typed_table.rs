//! Manim's `DecimalTable`, `IntegerTable`, `MobjectTable` and `MathTable`.

use std::borrow::Cow;

use fframes::Svgr;
use kurbo::Rect;

use crate::{
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{
    Table,
    table::{Cell, TextGrid, text_sizes},
    typed_matrix::{Mobject, place, written},
};

/// A [`Table`] of numbers written with `num_decimal_places` places (Manim `DecimalTable`, one
/// place by default). `table` gives the labels, place and look; its `rows` are replaced.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct DecimalTable<'a> {
    pub values: Vec<Vec<f64>>,
    pub num_decimal_places: usize,
    pub include_sign: bool,
    pub table: Table<'a>,
}

impl Themed for DecimalTable<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            values: Vec::new(),
            num_decimal_places: 1,
            include_sign: false,
            table: Table::themed(t),
        }
    }
}

impl Default for DecimalTable<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> DecimalTable<'a> {
    /// The [`Table`] it draws, with the numbers written.
    pub fn to_table(&self) -> Table<'a> {
        Table {
            rows: written(&self.values, self.num_decimal_places, self.include_sign),
            ..self.table.clone()
        }
    }
}

impl<'a> Component<'a> for DecimalTable<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.to_table().render(ui)
    }
}

/// A [`Table`] of whole numbers (Manim `IntegerTable`). `table` gives the labels, place and
/// look; its `rows` are replaced.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct IntegerTable<'a> {
    pub values: Vec<Vec<i64>>,
    pub table: Table<'a>,
}

impl Themed for IntegerTable<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            values: Vec::new(),
            table: Table::themed(t),
        }
    }
}

impl Default for IntegerTable<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> IntegerTable<'a> {
    /// The [`Table`] it draws, with the numbers written.
    pub fn to_table(&self) -> Table<'a> {
        Table {
            rows: self
                .values
                .iter()
                .map(|row| row.iter().map(|v| Cow::Owned(v.to_string())).collect())
                .collect(),
            ..self.table.clone()
        }
    }
}

impl<'a> Component<'a> for IntegerTable<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.to_table().render(ui)
    }
}

/// A [`Table`] whose body cells are drawn [`Mobject`]s (Manim `MobjectTable`), each in the
/// middle of its cell; the labels stay text. `table` gives the labels, place, lines and
/// animations; its `rows` are not used.
#[derive(Debug, Clone)]
pub struct MobjectTable<'a> {
    pub entries: Vec<Vec<Mobject<'a>>>,
    pub table: Table<'a>,
}

impl Themed for MobjectTable<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            entries: Vec::new(),
            table: Table::themed(t),
        }
    }
}

impl Default for MobjectTable<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> MobjectTable<'a> {
    /// The table with empty text where the mobjects go, its cells, the grid sized by the
    /// labels' text and the mobjects, and every mobject with its cell's index.
    fn layout(
        &self,
        ui: &Ui<'_, '_, '_>,
    ) -> (
        Table<'a>,
        Vec<Cell<'a>>,
        TextGrid,
        Vec<(Mobject<'a>, usize)>,
    ) {
        let table = Table {
            rows: self
                .entries
                .iter()
                .map(|row| vec![Cow::Borrowed(""); row.len()])
                .collect(),
            ..self.table.clone()
        };
        let (cells, columns) = table.cells();
        let (label_rows, label_cols) = table.label_offsets();
        let mut sizes = text_sizes(ui, &cells);
        let mut mobjects = Vec::new();
        for (r, row) in self.entries.iter().enumerate() {
            for (c, m) in row.iter().enumerate() {
                let i = (r + label_rows) * columns + c + label_cols;
                if let Some(size) = sizes.get_mut(i) {
                    *size = (m.width, m.height);
                    mobjects.push((m.clone(), i));
                }
            }
        }
        let (bx, by) = table.buff;
        let grid = TextGrid::new(
            &sizes,
            columns,
            (f64::from(bx), f64::from(by)),
            (f64::from(table.x), f64::from(table.y)),
        );
        (table, cells, grid, mobjects)
    }

    /// The area of body cell `(row, column)` between its lines. `None` outside the table.
    pub fn cell(&self, ui: &Ui<'_, '_, '_>, row: usize, column: usize) -> Option<Rect> {
        let (table, _, grid, _) = self.layout(ui);
        let (label_rows, label_cols) = table.label_offsets();
        grid.cell(row + label_rows, column + label_cols)
    }
}

impl<'a> Component<'a> for MobjectTable<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (table, cells, grid, mobjects) = self.layout(ui);
        if cells.is_empty() {
            return Svgr::empty();
        }
        let placed = place(mobjects.into_iter().map(|(m, i)| (m, grid.boxes[i])));
        let labels = grid.texts(cells);
        table.draw(ui, &grid, fframes::svgr!(<g>{labels}{placed}</g>))
    }
}

#[cfg(feature = "latex")]
pub use math_table::MathTable;

#[cfg(feature = "latex")]
mod math_table {
    use fframes::{Color, Svgr};

    use super::{Mobject, MobjectTable, Table};
    use crate::{
        latex::{Formula, FormulaView},
        layout::Anchor,
        theme::Theme,
        ui::{Component, Themed, Ui},
    };

    /// A [`Table`] of typeset formulas (Manim `MathTable`), typeset beforehand with a
    /// `latex::TexEngine` like any `MathTex`. `size` is the height in pixels of 10 pt text, as
    /// in `FormulaView`; `table` gives the labels, place, lines and animations.
    #[derive(Debug, Clone, PartialEq)]
    #[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
    pub struct MathTable<'a> {
        pub formulas: Vec<Vec<&'a Formula>>,
        pub size: f32,
        pub color: Color,
        pub table: Table<'a>,
    }

    impl Themed for MathTable<'_> {
        fn themed(t: &Theme) -> Self {
            Self {
                formulas: Vec::new(),
                size: 40.0,
                color: t.colors.title,
                table: Table::themed(t),
            }
        }
    }

    impl Default for MathTable<'_> {
        fn default() -> Self {
            Self::themed(&Theme::default())
        }
    }

    impl<'a> MathTable<'a> {
        /// The [`MobjectTable`] it draws: every formula centered on the origin.
        pub fn to_mobject_table(&self, ui: &mut Ui<'a, '_, '_>) -> MobjectTable<'a> {
            let entries = self
                .formulas
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|&of| {
                            let view = FormulaView {
                                size: self.size,
                                anchor: Anchor::Center,
                                color: self.color,
                                ..FormulaView::new(of)
                            };
                            let b = view.bounds();
                            Mobject::new(ui.show(view), b.width(), b.height())
                        })
                        .collect()
                })
                .collect();
            MobjectTable {
                entries,
                table: self.table.clone(),
            }
        }
    }

    impl<'a> Component<'a> for MathTable<'a> {
        fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
            let table = self.to_mobject_table(ui);
            ui.show(table)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_fill_the_body_and_labels_stay() {
        let t = DecimalTable {
            values: vec![vec![0.5, 12.0]],
            table: Table {
                col_labels: vec!["p".into(), "n".into()],
                ..Table::default()
            },
            ..DecimalTable::default()
        };
        let table = t.to_table();
        assert_eq!(table.rows, vec![vec!["0.5", "12.0"]]);
        assert_eq!(table.col_labels, vec!["p", "n"]);
        let ints = IntegerTable {
            values: vec![vec![7], vec![-8]],
            ..IntegerTable::default()
        };
        assert_eq!(ints.to_table().rows, vec![vec!["7"], vec!["-8"]]);
    }
}
