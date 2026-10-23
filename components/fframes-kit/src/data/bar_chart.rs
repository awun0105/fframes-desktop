use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    anim::RateFunc,
    motion::progress,
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// Bars that grow one after another from a baseline, each with its value counting up above
/// it and its label below. `(x, y)` is the left end of the baseline; the tallest bar is
/// `max_height` high when `max_value` is `None`. A bar is never thinner than half a pixel,
/// so a growing bar starts as a hairline.
#[derive(Debug, Clone, PartialEq)]
pub struct BarChart<'a> {
    /// `(label, value)` of each bar, left to right.
    pub bars: Vec<(Cow<'a, str>, f64)>,
    pub x: f32,
    pub y: f32,
    pub bar_width: f32,
    pub gap: f32,
    pub max_height: f32,
    /// The value drawn `max_height` high; `None` uses the largest value.
    pub max_value: Option<f64>,
    pub radius: f32,
    pub fill: Color,
    /// The index of a bar drawn in `highlight_fill`.
    pub highlight: Option<usize>,
    pub highlight_fill: Color,
    pub value_style: TextStyle,
    pub value_fill: Color,
    pub label_style: TextStyle,
    pub label_fill: Color,
    pub decimals: usize,
    pub delay: f32,
    pub stagger: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Themed for BarChart<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            bars: Vec::new(),
            x: 0.0,
            y: 0.0,
            bar_width: 126.0,
            gap: 62.0,
            max_height: 330.0,
            max_value: None,
            radius: 8.0,
            fill: t.colors.muted,
            highlight: None,
            highlight_fill: t.colors.accent,
            value_style: t.typography.body,
            value_fill: t.colors.title,
            label_style: t.typography.caption,
            label_fill: t.colors.muted,
            decimals: 0,
            delay: 0.0,
            stagger: t.motion.stagger.loose,
            run_time: t.motion.duration.slow,
            rate_func: t.motion.easing.enter,
        }
    }
}

impl Default for BarChart<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl BarChart<'_> {
    /// How far bar `i` has grown at `t`, from 0 to 1 (past 1 while a spring overshoots).
    pub fn grown(&self, i: usize, t: f32) -> f32 {
        let start = self.delay + i as f32 * self.stagger;
        self.rate_func
            .apply(progress(t, start, start + self.run_time), self.run_time)
    }

    /// Pixels per unit of value.
    fn scale(&self) -> f32 {
        let max = self
            .max_value
            .unwrap_or_else(|| self.bars.iter().map(|(_, v)| *v).fold(0.0, f64::max));
        if max > 0.0 {
            self.max_height / max as f32
        } else {
            0.0
        }
    }
}

impl<'a> Component<'a> for BarChart<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let scale = self.scale();
        let step = self.bar_width + self.gap;
        let bars: Vec<Svgr<'a>> = (0..self.bars.len())
            .map(|i| {
                let p = self.grown(i, t);
                let value = self.bars[i].1;
                let height = (value as f32 * scale * p).max(0.5);
                let x = self.x + i as f32 * step;
                let center = x + self.bar_width / 2.0;
                let fill = if self.highlight == Some(i) {
                    self.highlight_fill
                } else {
                    self.fill
                };
                let shown = format!("{:.*}", self.decimals, value * f64::from(p));
                let number = line(
                    center,
                    self.y - height - self.value_style.size as f32 * 0.64,
                    self.value_style,
                    self.value_fill,
                    TextAnchor::Middle,
                    0.0,
                    1.0,
                    Cow::Owned(shown),
                );
                let label = line(
                    center,
                    self.y + self.label_style.size as f32 * 1.75,
                    self.label_style,
                    self.label_fill,
                    TextAnchor::Middle,
                    0.0,
                    1.0,
                    self.bars[i].0.clone(),
                );
                let top = self.y - height;
                fframes::svgr!(
                    <g>
                        <rect x={x} y={top} width={self.bar_width} height={height} rx={self.radius} fill={fill} />
                        {number}
                        {label}
                    </g>
                )
            })
            .collect();
        fframes::svgr!(<g>{bars}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tallest_bar_is_max_height_and_bars_grow_in_turn() {
        let chart = BarChart {
            bars: vec![("a".into(), 20.0), ("b".into(), 40.0)],
            rate_func: RateFunc::Linear,
            delay: 0.0,
            stagger: 0.5,
            run_time: 1.0,
            ..BarChart::default()
        };
        assert_eq!(chart.scale() * 40.0, chart.max_height);
        assert_eq!(chart.grown(0, 0.5), 0.5);
        assert_eq!(chart.grown(1, 0.5), 0.0);
        assert_eq!(chart.grown(1, 2.0), 1.0);
        let empty = BarChart {
            bars: vec![("a".into(), 0.0)],
            ..BarChart::default()
        };
        assert_eq!(empty.scale(), 0.0);
    }
}
