use fframes::Svgr;
use kurbo::{Affine, BezPath, Rect, Vec2};

use crate::{
    animate::{Emphasis, EnterAnim, ExitAnim, draw_animated},
    geometry::{Geometry, Shape},
    layout::{self, Anchor},
    ui::{Component, Ui},
};

/// Shapes moved and animated together (Manim `VGroup`). Build one from the `shape()` of
/// shape components, line them up with [`VGroup::arrange`] or [`VGroup::arrange_in_grid`]
/// and draw it with one `enter`, `exit` and `emphasis`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VGroup {
    pub parts: Vec<Shape>,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub emphasis: Option<Emphasis>,
}

impl VGroup {
    pub fn new(parts: Vec<Shape>) -> Self {
        Self {
            parts,
            ..Self::default()
        }
    }

    /// The box around every part, `None` for an empty group.
    pub fn bounds(&self) -> Option<Rect> {
        self.parts
            .iter()
            .map(Geometry::bbox)
            .reduce(|a, b| a.union(b))
    }

    /// Moves every part by `(dx, dy)` pixels (Manim `shift`).
    pub fn shift(mut self, dx: f64, dy: f64) -> Self {
        let by = Affine::translate((dx, dy));
        for part in &mut self.parts {
            part.path.apply_affine(by);
        }
        self
    }

    /// Moves the group so the center of its box is at `(x, y)` (Manim `move_to`).
    pub fn move_to(self, x: f64, y: f64) -> Self {
        match self.bounds() {
            Some(b) => {
                let c = b.center();
                self.shift(x - c.x, y - c.y)
            }
            None => self,
        }
    }

    /// Lines the parts up one after another in `direction`, `buff` pixels apart, around the
    /// group's current center (Manim `arrange(direction, buff)`).
    pub fn arrange(self, direction: Anchor, buff: f64) -> Self {
        self.place(|sizes| layout::arrange(sizes, direction, buff))
    }

    /// Puts the parts in rows of `columns` cells (Manim `arrange_in_grid`), around the
    /// group's current center. `buff` is `(between columns, between rows)`.
    pub fn arrange_in_grid(self, columns: usize, buff: (f64, f64)) -> Self {
        self.place(|sizes| layout::grid(sizes, columns, buff))
    }

    /// Moves each part's box onto the box `layout` gives it, keeping the group's center.
    fn place(mut self, layout: impl FnOnce(&[(f64, f64)]) -> Vec<Rect>) -> Self {
        let Some(center) = self.bounds().map(|b| b.center()) else {
            return self;
        };
        let boxes: Vec<Rect> = self.parts.iter().map(Geometry::bbox).collect();
        let sizes: Vec<(f64, f64)> = boxes.iter().map(|b| (b.width(), b.height())).collect();
        let targets = layout(&sizes);
        for ((part, from), to) in self.parts.iter_mut().zip(&boxes).zip(targets) {
            let by: Vec2 = (to.center() - from.center()) + center.to_vec2();
            part.path.apply_affine(Affine::translate(by));
        }
        self
    }
}

impl Geometry for VGroup {
    fn path(&self) -> BezPath {
        let mut path = BezPath::new();
        for part in &self.parts {
            path.extend(part.path.iter());
        }
        path
    }
}

impl<'a> Component<'a> for VGroup {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        draw_animated(
            ui,
            &self.parts,
            self.enter.as_ref(),
            self.exit.as_ref(),
            self.emphasis.as_ref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::{Circle, Square};

    #[test]
    fn arrange_keeps_the_center_and_spaces_the_boxes() {
        let group = VGroup::new(vec![
            Circle {
                cx: 0.0,
                cy: 0.0,
                radius: 50.0,
                ..Circle::default()
            }
            .shape(),
            Square {
                cx: 500.0,
                cy: 300.0,
                side_length: 40.0,
                ..Square::default()
            }
            .shape(),
        ]);
        let before = group.bounds().unwrap().center();
        let row = group.arrange(Anchor::Right, 20.0);
        let after = row.bounds().unwrap();
        assert!((after.center() - before).hypot() < 1e-6, "{after:?}");
        let (a, b) = (row.parts[0].bbox(), row.parts[1].bbox());
        assert!((b.x0 - a.x1 - 20.0).abs() < 1e-6);
        assert!((a.center().y - b.center().y).abs() < 1e-6);

        let moved = row.move_to(960.0, 540.0).bounds().unwrap().center();
        assert!((moved.x - 960.0).abs() < 1e-6 && (moved.y - 540.0).abs() < 1e-6);
        assert_eq!(
            VGroup::default().arrange(Anchor::Right, 1.0),
            VGroup::default()
        );
    }

    #[test]
    fn grid_puts_parts_in_rows() {
        let dots: Vec<Shape> = (0..6)
            .map(|i| {
                Circle {
                    cx: i as f32 * 7.0,
                    cy: 0.0,
                    radius: 10.0,
                    ..Circle::default()
                }
                .shape()
            })
            .collect();
        let grid = VGroup::new(dots).arrange_in_grid(3, (10.0, 10.0));
        let centers: Vec<_> = grid.parts.iter().map(|p| p.bbox().center()).collect();
        // two rows of three, 30 px apart
        assert!((centers[3].y - centers[0].y - 30.0).abs() < 1e-6);
        assert!((centers[1].x - centers[0].x - 30.0).abs() < 1e-6);
        assert!((centers[4].x - centers[1].x).abs() < 1e-6);
    }
}
