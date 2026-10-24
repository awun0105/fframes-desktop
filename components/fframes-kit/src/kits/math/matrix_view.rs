use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::Rect;

use crate::{
    anim::FadeIn,
    animate::{EnterAnim, draw_animated},
    explain::Matrix,
    label::Label,
    layout::Anchor,
    shapes::Rectangle,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::short;

/// The product of two matrices worked out entry by entry: `a`, `b` and `a b` side by side
/// around `(x, y)`, with `×` and `=` between them. From `delay`, every `step` seconds the next
/// entry of the product is written, row by row, while its row of `a` and column of `b` are
/// highlighted; entries and highlights fade in over a third of a step. After the last entry
/// the highlights fade out. Entries are written with at most
/// `decimals` decimals.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct MatrixView {
    pub a: Vec<Vec<f64>>,
    pub b: Vec<Vec<f64>>,
    pub x: f32,
    pub y: f32,
    /// Pixels between the matrices and the signs.
    pub gap: f32,
    pub delay: f32,
    pub step: f32,
    pub decimals: usize,
    pub highlight: Color,
    pub result_color: Color,
    pub matrices_enter: Option<FadeIn>,
}

impl Themed for MatrixView {
    fn themed(t: &Theme) -> Self {
        Self {
            a: vec![vec![1.0, 2.0], vec![3.0, 4.0]],
            b: vec![vec![5.0, 6.0], vec![7.0, 8.0]],
            x: 960.0,
            y: 540.0,
            gap: t.spacing.lg,
            delay: 0.5,
            step: 0.8,
            decimals: 2,
            highlight: t.colors.highlight,
            result_color: t.colors.accent,
            matrices_enter: Some(FadeIn::themed(t)),
        }
    }
}

impl Default for MatrixView {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// `a b`, or `None` when the columns of `a` do not match the rows of `b` (or `a` is empty).
pub fn product(a: &[Vec<f64>], b: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let inner = a.first()?.len();
    let columns = b.first().map_or(0, Vec::len);
    if inner == 0
        || b.len() != inner
        || a.iter().any(|row| row.len() != inner)
        || b.iter().any(|row| row.len() != columns)
    {
        return None;
    }
    Some(
        a.iter()
            .map(|row| {
                (0..columns)
                    .map(|j| row.iter().zip(b).map(|(x, b_row)| x * b_row[j]).sum())
                    .collect()
            })
            .collect(),
    )
}

impl MatrixView {
    /// How many entries of the product are written `t` seconds in, out of `count`.
    fn written(&self, t: f32, count: usize) -> usize {
        if t < self.delay || self.step <= 0.0 {
            return if t < self.delay { 0 } else { count };
        }
        (((t - self.delay) / self.step) as usize + 1).min(count)
    }

    /// Seconds an entry and its highlights take to fade in.
    fn fade_time(&self) -> f32 {
        (self.step * 0.3).max(0.0)
    }

    /// The highlights fade in at every step and out after the last one.
    fn highlight_opacity(&self, t: f32, count: usize) -> f32 {
        let (fade, end) = (self.fade_time(), self.delay + self.step * count as f32);
        if t < self.delay || count == 0 || fade <= 0.0 {
            return 0.0;
        }
        if t >= end {
            return (1.0 - (t - end) / fade).max(0.0);
        }
        let since = (t - self.delay).rem_euclid(self.step);
        (since / fade).min(1.0)
    }

    fn entries(m: &[Vec<f64>], decimals: usize) -> Vec<Vec<Cow<'static, str>>> {
        m.iter()
            .map(|row| {
                row.iter()
                    .map(|&v| Cow::Owned(short(v, decimals)))
                    .collect()
            })
            .collect()
    }
}

/// The smallest rectangle around `rects`.
fn union(rects: impl Iterator<Item = Rect>) -> Option<Rect> {
    rects.reduce(|a, b| a.union(b))
}

impl<'a> Component<'a> for MatrixView {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(c) = product(&self.a, &self.b) else {
            return Svgr::empty();
        };
        let matrix = |ui: &Ui<'_, '_, '_>, m: &[Vec<f64>]| Matrix {
            entries: Self::entries(m, self.decimals),
            enter: self.matrices_enter.map(EnterAnim::FadeIn),
            entries_enter: self.matrices_enter,
            ..ui.defaults()
        };
        let mut parts = [matrix(ui, &self.a), matrix(ui, &self.b), matrix(ui, &c)];
        // the product's entries are written one by one over its invisible layout
        parts[2].fill = Color::TRANSPARENT;
        let widths = parts
            .each_ref()
            .map(|m| m.bounds(ui).map_or(0.0, |b| b.width() as f32));
        let style = ui.theme.typography.body;
        let sign = |text| crate::label::text_size(ui, style, text).0;
        let (times, equals) = (sign("×"), sign("="));
        let total = widths.iter().sum::<f32>() + times + equals + self.gap * 4.0;
        let mut left = self.x - total / 2.0;
        let mut signs = Vec::new();
        for (i, m) in parts.iter_mut().enumerate() {
            m.x = left + widths[i] / 2.0;
            m.y = self.y;
            left += widths[i] + self.gap;
            if let Some((text, w)) = [("×", times), ("=", equals)].get(i) {
                signs.push((*text, left + w / 2.0));
                left += w + self.gap;
            }
        }

        let rows = c.len();
        let columns = c.first().map_or(0, Vec::len);
        let count = rows * columns;
        let t = ui.seconds();
        let written = self.written(t, count);
        let glow = self.highlight_opacity(t, count);
        let mut lit = Vec::new();
        if written > 0 {
            lit.push((written - 1, glow));
            // the step before fades out while this one fades in
            let stepping = t < self.delay + self.step * count as f32;
            if stepping && written > 1 && glow < 1.0 {
                lit.push((written - 2, 1.0 - glow));
            }
        }
        let mut highlights = Vec::new();
        for (n, glow) in lit.into_iter().filter(|&(_, glow)| glow > 0.0) {
            let (i, j) = (n / columns, n % columns);
            let inner = self.b.len();
            let boxes = [
                union((0..inner).filter_map(|k| parts[0].cell(ui, i, k))),
                union((0..inner).filter_map(|k| parts[1].cell(ui, k, j))),
                parts[2].cell(ui, i, j),
            ];
            for b in boxes.into_iter().flatten() {
                let center = b.center();
                highlights.push(
                    Rectangle {
                        cx: center.x as f32,
                        cy: center.y as f32,
                        width: b.width() as f32,
                        height: b.height() as f32,
                        fill: Some(self.highlight.with_alpha(48)),
                        stroke: Some(self.highlight),
                        stroke_width: ui.theme.stroke.thin,
                        opacity: glow,
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
        }
        let highlights = draw_animated(ui, &highlights, None, None, None);
        let values: Vec<Svgr> = (0..written)
            .filter_map(|n| {
                let (i, j) = (n / columns, n % columns);
                let b = parts[2].cell(ui, i, j)?;
                let center = b.center();
                Some(ui.show(Label {
                    text: short(c[i][j], self.decimals).into(),
                    x: center.x as f32,
                    y: center.y as f32,
                    direction: Anchor::Center,
                    buff: 0.0,
                    style,
                    fill: self.result_color,
                    enter: Some(FadeIn {
                        delay: self.delay + n as f32 * self.step,
                        run_time: self.fade_time(),
                        ..ui.defaults()
                    }),
                    ..ui.defaults()
                }))
            })
            .collect();
        let signs: Vec<Svgr> = signs
            .into_iter()
            .map(|(text, x)| {
                ui.show(Label {
                    text: text.into(),
                    x,
                    y: self.y,
                    direction: Anchor::Center,
                    buff: 0.0,
                    style,
                    enter: self.matrices_enter,
                    ..ui.defaults()
                })
            })
            .collect();
        let [a, b, c] = parts.map(|m| ui.show(m));
        let values: Svgr = values.into();
        let signs: Svgr = signs.into();
        fframes::svgr!(<g>{highlights}{a}{b}{c}{signs}{values}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn products_follow_rows_and_columns() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
        assert_eq!(
            product(&a, &b),
            Some(vec![vec![19.0, 22.0], vec![43.0, 50.0]])
        );
        // 1x3 times 3x1
        let row = vec![vec![1.0, 2.0, 3.0]];
        let column = vec![vec![4.0], vec![5.0], vec![6.0]];
        assert_eq!(product(&row, &column), Some(vec![vec![32.0]]));
        assert_eq!(product(&a, &row), None);
        assert_eq!(product(&[], &b), None);
    }

    #[test]
    fn entries_are_written_one_step_apart() {
        let view = MatrixView {
            delay: 1.0,
            step: 0.5,
            ..MatrixView::default()
        };
        assert_eq!(view.written(0.9, 4), 0);
        assert_eq!(view.written(1.0, 4), 1);
        assert_eq!(view.written(1.6, 4), 2);
        assert_eq!(view.written(9.0, 4), 4);
        // each step's highlights fade in over 0.15 s; after the last step they fade out
        assert_eq!(view.highlight_opacity(0.5, 4), 0.0);
        assert!((view.highlight_opacity(1.075, 4) - 0.5).abs() < 1e-4);
        assert_eq!(view.highlight_opacity(1.4, 4), 1.0);
        assert!((view.highlight_opacity(3.075, 4) - 0.5).abs() < 1e-4);
        assert_eq!(view.highlight_opacity(3.5, 4), 0.0);
    }
}
