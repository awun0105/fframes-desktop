use fframes::{Color, Svgr, animation::Animatable};

use crate::{
    anim::RateFunc,
    animate::draw_animated,
    explain::AxesMap,
    geometry::Shape,
    motion::progress,
    shapes::Rectangle,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// A grid of coloured cells on axes: `values` row by row, the first row at the top, covering
/// `x_range` and `y_range` in data units. A cell's colour runs from `low` at `min` to `high`
/// at `max` (the smallest and largest value when `None`). From `delay` the cells fade in as
/// a wave from the top left corner, `stagger` seconds per step along the diagonal. Values
/// that are not finite leave their cell empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Heatmap {
    pub values: Vec<Vec<f64>>,
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
    pub axes: AxesMap,
    pub low: Color,
    pub high: Color,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// Pixels between the cells.
    pub gap: f32,
    pub delay: f32,
    pub stagger: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Themed for Heatmap {
    fn themed(t: &Theme) -> Self {
        Self {
            values: Vec::new(),
            x_range: (0.0, 1.0),
            y_range: (0.0, 1.0),
            axes: AxesMap::default(),
            low: t.colors.surface,
            high: t.colors.highlight,
            min: None,
            max: None,
            gap: 2.0,
            delay: 0.0,
            stagger: 0.05,
            run_time: t.motion.duration.base,
            rate_func: RateFunc::default(),
        }
    }
}

impl Default for Heatmap {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Heatmap {
    fn finite(&self) -> impl Iterator<Item = f64> + '_ {
        self.values
            .iter()
            .flatten()
            .copied()
            .filter(|v| v.is_finite())
    }

    /// The colour of value `v`.
    pub fn color_at(&self, v: f64) -> Color {
        let min = self
            .min
            .unwrap_or_else(|| self.finite().fold(f64::INFINITY, f64::min));
        let max = self
            .max
            .unwrap_or_else(|| self.finite().fold(f64::NEG_INFINITY, f64::max));
        let t = if max > min {
            ((v - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.low.apply_progress(&self.high, t as f32)
    }

    /// How far cell `(row, column)` has faded in at `t`.
    fn shown(&self, row: usize, column: usize, t: f32) -> f32 {
        let start = self.delay + (row + column) as f32 * self.stagger;
        self.rate_func
            .apply(progress(t, start, start + self.run_time), self.run_time)
    }
}

impl<'a> Component<'a> for Heatmap {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let rows = self.values.len();
        let columns = self.values.iter().map(Vec::len).max().unwrap_or(0);
        if rows == 0 || columns == 0 {
            return Svgr::empty();
        }
        let t = ui.seconds();
        let ((x0, x1), (y0, y1)) = (self.x_range, self.y_range);
        let (w, h) = ((x1 - x0) / columns as f64, (y1 - y0) / rows as f64);
        let mut cells: Vec<Shape> = Vec::new();
        for (r, row) in self.values.iter().enumerate() {
            for (c, &v) in row.iter().enumerate() {
                let p = self.shown(r, c, t);
                if !v.is_finite() || p <= 0.0 {
                    continue;
                }
                // row 0 is the top row
                let left = x0 + w * c as f64;
                let top = y1 - h * r as f64;
                let a = self.axes.c2p(left, top);
                let b = self.axes.c2p(left + w, top - h);
                let center = a.midpoint(b);
                cells.push(
                    Rectangle {
                        cx: center.x as f32,
                        cy: center.y as f32,
                        width: ((b.x - a.x).abs() as f32 - self.gap).max(0.0),
                        height: ((b.y - a.y).abs() as f32 - self.gap).max(0.0),
                        fill: Some(self.color_at(v)),
                        stroke: None,
                        opacity: p.clamp(0.0, 1.0),
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
        }
        draw_animated(ui, &cells, None, None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_run_from_low_to_high() {
        let map = Heatmap {
            values: vec![vec![0.0, 5.0], vec![10.0, f64::NAN]],
            low: Color::rgb(0, 0, 0),
            high: Color::rgb(200, 100, 0),
            ..Heatmap::default()
        };
        assert_eq!(map.color_at(0.0), Color::rgb(0, 0, 0));
        assert_eq!(map.color_at(10.0), Color::rgb(200, 100, 0));
        assert_eq!(map.color_at(5.0), Color::rgb(100, 50, 0));
        assert_eq!(map.color_at(99.0), Color::rgb(200, 100, 0));
    }

    #[test]
    fn cells_fade_in_along_the_diagonal() {
        let map = Heatmap {
            delay: 1.0,
            stagger: 0.5,
            run_time: 1.0,
            rate_func: RateFunc::Linear,
            ..Heatmap::default()
        };
        assert_eq!(map.shown(0, 0, 1.5), 0.5);
        assert_eq!(map.shown(1, 0, 1.5), 0.0);
        assert_eq!(map.shown(0, 1, 2.0), map.shown(1, 0, 2.0));
    }
}
