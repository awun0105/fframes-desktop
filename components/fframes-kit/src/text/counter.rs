use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    anim::RateFunc,
    motion::{progress, thousands},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line};

/// A number that counts from `from` to `to` over `run_time` seconds, starting `delay`
/// seconds into the container: `"0" -> "12,480"`. It is right aligned by default, so the
/// digits stay in place while the number grows.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Counter<'a> {
    pub from: f64,
    pub to: f64,
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub anchor: TextAnchor,
    /// Digits after the decimal point.
    pub decimals: usize,
    /// Groups the digits by three with commas.
    pub thousands: bool,
    pub prefix: Cow<'a, str>,
    pub suffix: Cow<'a, str>,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Themed for Counter<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            from: 0.0,
            to: 0.0,
            x: 0.0,
            y: 0.0,
            style: t.typography.title,
            fill: t.colors.title,
            anchor: TextAnchor::End,
            decimals: 0,
            thousands: true,
            prefix: Cow::Borrowed(""),
            suffix: Cow::Borrowed(""),
            delay: 0.0,
            run_time: t.motion.duration.slow * 2.0,
            rate_func: t.motion.easing.standard,
        }
    }
}

impl Default for Counter<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Counter<'_> {
    /// The value `t` seconds into the container.
    pub fn value(&self, t: f32) -> f64 {
        let p = self.rate_func.apply(
            progress(t, self.delay, self.delay + self.run_time),
            self.run_time,
        );
        self.from + f64::from(p) * (self.to - self.from)
    }

    /// The text shown for `value`.
    pub fn format(&self, value: f64) -> String {
        let fixed = format!("{:.*}", self.decimals, value.abs());
        let (int, frac) = fixed.split_once('.').unwrap_or((&fixed, ""));
        let int = match (self.thousands, int.parse::<u64>()) {
            (true, Ok(n)) => thousands(n),
            _ => int.to_owned(),
        };
        // -0 shows as 0
        let sign = if value < 0.0 && fixed.chars().any(|c| matches!(c, '1'..='9')) {
            "-"
        } else {
            ""
        };
        let dot = if frac.is_empty() { "" } else { "." };
        format!("{}{sign}{int}{dot}{frac}{}", self.prefix, self.suffix)
    }
}

impl<'a> Component<'a> for Counter<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let text = self.format(self.value(ui.seconds()));
        line(
            self.x,
            self.y,
            self.style,
            self.fill,
            self.anchor,
            0.0,
            1.0,
            Cow::Owned(text),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_formats() {
        let c = Counter {
            from: 0.0,
            to: 12_480.0,
            run_time: 2.0,
            rate_func: RateFunc::Linear,
            ..Counter::default()
        };
        assert_eq!(c.format(c.value(0.0)), "0");
        assert_eq!(c.format(c.value(1.0)), "6,240");
        assert_eq!(c.format(c.value(5.0)), "12,480");
        let money = Counter {
            decimals: 2,
            prefix: "$".into(),
            suffix: " M".into(),
            ..Counter::default()
        };
        assert_eq!(money.format(1234.5), "$1,234.50 M");
        assert_eq!(money.format(-0.001), "$0.00 M");
        assert_eq!(money.format(-3.25), "$-3.25 M");
        let plain = Counter {
            thousands: false,
            ..Counter::default()
        };
        assert_eq!(plain.format(12_480.0), "12480");
    }
}
