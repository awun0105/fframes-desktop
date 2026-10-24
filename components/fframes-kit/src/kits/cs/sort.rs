use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::{Rect, RoundedRect};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{
    array::{CellColors, notes, travel},
    trace::ArrayTrace,
};

/// A sort as bars, `width` by `height` pixels around `center` (units of `ui.space`), each
/// as tall as its value. It plays an [`ArrayTrace`] from [`ArrayTrace::bubble`],
/// [`insertion`](ArrayTrace::insertion), [`merge`](ArrayTrace::merge) or
/// [`quick`](ArrayTrace::quick), one step every `step_time` seconds from `delay`: the
/// compared bars light up, moved bars slide to their new place, and bars in their final
/// place turn `colors.sorted`. `name` is written above the bars with the comparisons and
/// moves so far; `values` writes each value under its bar and `notes` the step under the
/// figure. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct SortVisualizer<'a> {
    pub trace: Option<&'a ArrayTrace>,
    pub name: Cow<'a, str>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub values: bool,
    pub counts: bool,
    pub notes: bool,
    pub colors: CellColors,
    pub muted: Color,
    pub name_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for SortVisualizer<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            trace: None,
            name: Cow::Borrowed(""),
            delay: 0.0,
            step_time: 0.25,
            center: (0.0, 0.0),
            width: 900.0,
            height: 420.0,
            values: true,
            counts: true,
            notes: true,
            colors: CellColors {
                base: t.colors.accent,
                compare: t.colors.highlight,
                moved: Color::hex("#ff6b6b"),
                ..CellColors::themed(t)
            },
            muted: t.colors.muted,
            name_style: t.typography.body,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for SortVisualizer<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for SortVisualizer<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(trace) = self.trace else {
            return Svgr::empty();
        };
        let Some(at) = trace.at(ui.seconds() - self.delay, self.step_time) else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (w, h) = (f64::from(self.width), f64::from(self.height));
        let (left, base) = (middle.x - w / 2.0, middle.y + h / 2.0);
        let n = at.step.values.len().max(1);
        let pitch = w / n as f64;
        let bar = pitch * 0.72;
        // the smallest value still gets a sliver of a bar
        let (lo, hi) = trace.steps()[0]
            .values
            .iter()
            .fold((i64::MAX, i64::MIN), |(a, b), &v| (a.min(v), b.max(v)));
        let span = (hi - lo).max(1) as f64;
        let move_time = self.step_time * 0.8;
        let blend = (self.step_time * 0.4).min(0.15);
        let mut bars = Vec::new();
        let mut labels = Vec::new();
        for (k, &value) in at.step.values.iter().enumerate() {
            let (slot, _) = travel(&at, k, move_time);
            let x = left + pitch * (slot + 0.5);
            let top = base - (0.08 + 0.92 * (value - lo) as f64 / span) * h;
            let (fill, opacity) = self.colors.blended(&at, k, blend, 0.35);
            let rect = Rect::new(x - bar / 2.0, top, x + bar / 2.0, base);
            bars.push(Shape {
                path: kurbo::Shape::to_path(
                    &RoundedRect::from_rect(rect, bar.min(16.0) * 0.25),
                    0.1,
                ),
                fill: Some(fill),
                stroke: None,
                stroke_width: 0.0,
                dash: None,
                opacity,
            });
            if self.values {
                labels.push(line(
                    x as f32,
                    (base + 8.0 + self.small_style.size as f64 * 0.8) as f32,
                    self.small_style,
                    self.colors.text,
                    TextAnchor::Middle,
                    0.0,
                    opacity,
                    value.to_string().into(),
                ));
            }
        }
        let bars: Vec<Svgr<'a>> = bars.iter().map(Shape::draw).collect();
        let mut parts = vec![bars.into(), labels.into()];
        let header = (middle.y - h / 2.0 - 24.0) as f32;
        if !self.name.is_empty() {
            parts.push(line(
                left as f32,
                header,
                self.name_style,
                self.colors.text,
                TextAnchor::Start,
                0.0,
                1.0,
                self.name.clone(),
            ));
        }
        if self.counts {
            let (compares, moves) = at.counts;
            parts.push(line(
                (left + w) as f32,
                header,
                self.small_style,
                self.muted,
                TextAnchor::End,
                0.0,
                1.0,
                format!("{compares} comparisons, {moves} moves").into(),
            ));
        }
        if self.notes {
            let below = if self.values { 2.6 } else { 1.4 };
            let y = base + below * self.small_style.size as f64 + 12.0;
            parts.push(notes(
                &at,
                middle.x as f32,
                y as f32,
                self.small_style,
                self.muted,
                blend,
            ));
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}
