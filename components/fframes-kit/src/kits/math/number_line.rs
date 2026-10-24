use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    anim::{FadeIn, FadeOut, ValueTracker, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::NumberLine,
    label::Label,
    layout::Anchor,
    shapes::{Circle, Dot, Line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// A dot on a number line at `value`, with `label` above it (Manim's `Dot(line.n2p(x))`
/// with a `ValueTracker`). With `decimals` the value is written after the label:
/// `label: "x = "` and `decimals: Some(1)` write `x = 2.5`. The line is drawn separately.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct NumberPoint<'a> {
    pub line: NumberLine,
    pub value: ValueTracker<'a>,
    pub label: Cow<'a, str>,
    pub decimals: Option<usize>,
    pub radius: f32,
    pub color: Color,
    pub label_style: TextStyle,
    pub label_fill: Color,
    /// Pixels between the dot and the label.
    pub buff: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for NumberPoint<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            line: NumberLine::themed(t),
            value: 0.0.into(),
            label: Cow::Borrowed(""),
            decimals: None,
            radius: 12.0,
            color: t.colors.highlight,
            label_style: t.typography.caption,
            label_fill: t.colors.title,
            buff: t.spacing.md,
            enter: None,
            exit: None,
        }
    }
}

impl Default for NumberPoint<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl NumberPoint<'_> {
    /// The label's text when the value is `value`.
    fn text(&self, value: f32) -> String {
        match self.decimals {
            Some(d) => format!("{}{value:.d$}", self.label),
            None => self.label.clone().into_owned(),
        }
    }
}

impl<'a> Component<'a> for NumberPoint<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let value = ui.sample(&self.value);
        let (x, y) = self.line.n2p(ui, f64::from(value));
        let dot = ui.show(Dot {
            cx: x,
            cy: y,
            radius: self.radius,
            fill: Some(self.color),
            ..ui.defaults()
        });
        let text = self.text(value);
        let label = if text.is_empty() {
            Svgr::empty()
        } else {
            ui.show(Label {
                text: text.into(),
                x,
                y: y - self.radius,
                direction: Anchor::Top,
                buff: self.buff,
                style: self.label_style,
                fill: self.label_fill,
                ..ui.defaults()
            })
        };
        fade(
            ui,
            fframes::svgr!(<g>{dot}{label}</g>),
            self.enter.as_ref(),
            self.exit.as_ref(),
        )
    }
}

/// The numbers from `from` to `to` marked on a number line: a thick segment with a filled
/// dot at a closed end and a ring at an open one (`[a, b)` is `closed: (true, false)`).
/// The line is drawn separately.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Interval {
    pub line: NumberLine,
    pub from: f64,
    pub to: f64,
    pub closed: (bool, bool),
    pub color: Color,
    pub stroke_width: f32,
    /// Radius of the end dots and rings.
    pub radius: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
}

impl Themed for Interval {
    fn themed(t: &Theme) -> Self {
        Self {
            line: NumberLine::themed(t),
            from: 0.0,
            to: 1.0,
            closed: (true, true),
            color: t.colors.accent,
            stroke_width: t.stroke.thick,
            radius: 12.0,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Interval {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Interval {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (a, b) = (self.line.n2p(ui, self.from), self.line.n2p(ui, self.to));
        let mut parts = vec![
            Line {
                x1: a.0,
                y1: a.1,
                x2: b.0,
                y2: b.1,
                stroke: Some(self.color),
                stroke_width: self.stroke_width,
                ..ui.defaults()
            }
            .shape(),
        ];
        for ((cx, cy), closed) in [(a, self.closed.0), (b, self.closed.1)] {
            parts.push(if closed {
                Dot {
                    cx,
                    cy,
                    radius: self.radius,
                    fill: Some(self.color),
                    ..ui.defaults()
                }
                .shape()
            } else {
                Circle {
                    cx,
                    cy,
                    radius: self.radius,
                    fill: Some(ui.theme.colors.background),
                    stroke: Some(self.color),
                    stroke_width: self.stroke_width / 2.0,
                    ..ui.defaults()
                }
                .shape()
            });
        }
        draw_animated(ui, &parts, self.enter.as_ref(), self.exit.as_ref(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_value_follows_the_label() {
        let point = NumberPoint {
            label: "x = ".into(),
            decimals: Some(1),
            ..NumberPoint::default()
        };
        assert_eq!(point.text(2.46), "x = 2.5");
        let plain = NumberPoint {
            label: "a".into(),
            decimals: None,
            ..point
        };
        assert_eq!(plain.text(2.46), "a");
    }
}
