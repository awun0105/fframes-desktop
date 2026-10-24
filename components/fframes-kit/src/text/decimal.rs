use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    anim::ValueTracker,
    motion::thousands,
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line, width};

/// `value` with `decimals` digits after the point, digits grouped by three with commas when
/// `group_with_commas` is set and a `+` before positive numbers when `include_sign` is set.
/// A value that rounds to zero has no minus.
pub(crate) fn format_number(
    value: f64,
    decimals: usize,
    include_sign: bool,
    group_with_commas: bool,
) -> String {
    let fixed = format!("{:.*}", decimals, value.abs());
    let (int, frac) = fixed.split_once('.').unwrap_or((&fixed, ""));
    let int = match (group_with_commas, int.parse::<u64>()) {
        (true, Ok(n)) => thousands(n),
        _ => int.to_owned(),
    };
    // -0 shows as 0
    let negative = value < 0.0 && fixed.chars().any(|c| matches!(c, '1'..='9'));
    let sign = match (negative, include_sign) {
        (true, _) => "-",
        (false, true) => "+",
        (false, false) => "",
    };
    let dot = if frac.is_empty() { "" } else { "." };
    format!("{sign}{int}{dot}{frac}")
}

/// Manim's `DecimalNumber`: a number that follows a [`ValueTracker`], drawn so its digits do
/// not jitter while it changes. Every digit gets a slot as wide as the widest digit of the
/// font, so a number keeps its width while its digit count stays the same, and with
/// `anchor: End` the digits stay in place across a carry (`9.99 -> 10.00`). The value is an
/// `f32`, exact to about seven significant digits.
///
/// `(x, y)` is the anchor point on the baseline; `unit` follows the number (`" kg"`). Without
/// a measurable font it is one plain `<text>`.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct DecimalNumber<'a> {
    pub value: ValueTracker<'a>,
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub anchor: TextAnchor,
    pub num_decimal_places: usize,
    /// A `+` before positive numbers.
    pub include_sign: bool,
    pub group_with_commas: bool,
    pub unit: &'a str,
    pub opacity: f32,
}

impl Themed for DecimalNumber<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            value: 0.0.into(),
            x: 0.0,
            y: 0.0,
            style: t.typography.title,
            fill: t.colors.title,
            anchor: TextAnchor::Start,
            num_decimal_places: 2,
            include_sign: false,
            group_with_commas: true,
            unit: "",
            opacity: 1.0,
        }
    }
}

impl Default for DecimalNumber<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl DecimalNumber<'_> {
    /// The text shown for `value`, with the unit.
    pub fn format(&self, value: f64) -> String {
        let number = format_number(
            value,
            self.num_decimal_places,
            self.include_sign,
            self.group_with_commas,
        );
        format!("{number}{}", self.unit)
    }
}

/// The x of every character of `number` and the unit and the total width, from the left
/// edge: digits sit in the middle of slots `digit` wide, other characters take their own
/// width. `None` when the font cannot be measured.
fn layout(
    ui: &Ui<'_, '_, '_>,
    style: TextStyle,
    number: &str,
    unit: &str,
) -> Option<(Vec<f32>, f32)> {
    let digit = ('0'..='9')
        .map(|d| width(ui, style, d.encode_utf8(&mut [0; 4])))
        .try_fold(0.0_f32, |widest, w| Some(widest.max(w?)))?;
    let mut xs = Vec::with_capacity(number.len() + 1);
    let mut at = 0.0;
    for c in number.chars() {
        let w = width(ui, style, c.encode_utf8(&mut [0; 4]))?;
        if c.is_ascii_digit() {
            xs.push(at + (digit - w) / 2.0);
            at += digit;
        } else {
            xs.push(at);
            at += w;
        }
    }
    xs.push(at);
    Some((xs, at + width(ui, style, unit)?))
}

impl<'a> Component<'a> for DecimalNumber<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let value = f64::from(ui.sample(&self.value));
        let number = format_number(
            value,
            self.num_decimal_places,
            self.include_sign,
            self.group_with_commas,
        );
        let s = self.style;
        let Some((xs, total)) = layout(ui, s, &number, self.unit) else {
            return line(
                self.x,
                self.y,
                s,
                self.fill,
                self.anchor,
                0.0,
                self.opacity,
                Cow::Owned(self.format(value)),
            );
        };
        let left = self.anchor.left(self.x, total);
        let unit = (!self.unit.is_empty()).then_some(self.unit);
        let glyphs: Vec<Svgr<'a>> = number
            .chars()
            .map(String::from)
            .chain(unit.map(str::to_owned))
            .zip(&xs)
            .map(|(text, dx)| {
                let x = left + dx;
                fframes::svgr!(<tspan x={x}>{text}</tspan>)
            })
            .collect();
        fframes::svgr!(
            <text y={self.y} font-family={s.family} font-size={s.size} font-weight={s.weight}
                  fill={self.fill} opacity={self.opacity}>
                {glyphs}
            </text>
        )
    }
}

/// Manim's `Integer`: a [`DecimalNumber`] without decimals, rounded to the nearest whole
/// number.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Integer<'a> {
    pub value: ValueTracker<'a>,
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub anchor: TextAnchor,
    pub include_sign: bool,
    pub group_with_commas: bool,
    pub unit: &'a str,
    pub opacity: f32,
}

impl Themed for Integer<'_> {
    fn themed(t: &Theme) -> Self {
        let d = DecimalNumber::themed(t);
        Self {
            value: d.value,
            x: d.x,
            y: d.y,
            style: d.style,
            fill: d.fill,
            anchor: d.anchor,
            include_sign: d.include_sign,
            group_with_commas: d.group_with_commas,
            unit: d.unit,
            opacity: d.opacity,
        }
    }
}

impl Default for Integer<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> From<Integer<'a>> for DecimalNumber<'a> {
    fn from(n: Integer<'a>) -> Self {
        Self {
            value: n.value,
            x: n.x,
            y: n.y,
            style: n.style,
            fill: n.fill,
            anchor: n.anchor,
            num_decimal_places: 0,
            include_sign: n.include_sign,
            group_with_commas: n.group_with_commas,
            unit: n.unit,
            opacity: n.opacity,
        }
    }
}

impl<'a> Component<'a> for Integer<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        DecimalNumber::from(self).render(ui)
    }
}

/// Manim's `Variable`: `label = value`, the value a [`DecimalNumber`] that follows a
/// [`ValueTracker`]. `(x, y)` is the start of the label on the baseline; the label stays in
/// place and the number grows to the right. For a value computed from time, pass
/// `value: f(ui.seconds()).into()` in `render_frame`, which runs every frame.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Variable<'a> {
    pub label: Cow<'a, str>,
    pub value: ValueTracker<'a>,
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    /// The color of the number; the label and `=` take `fill`.
    pub value_fill: Color,
    pub num_decimal_places: usize,
    pub unit: &'a str,
    pub opacity: f32,
}

impl Themed for Variable<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            label: Cow::Borrowed(""),
            value: 0.0.into(),
            x: 0.0,
            y: 0.0,
            style: t.typography.heading,
            fill: t.colors.text,
            value_fill: t.colors.accent,
            num_decimal_places: 2,
            unit: "",
            opacity: 1.0,
        }
    }
}

impl Default for Variable<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Variable<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let s = self.style;
        let head = format!("{} = ", self.label);
        let mut number = DecimalNumber {
            value: self.value,
            y: self.y,
            style: s,
            fill: self.value_fill,
            anchor: TextAnchor::Start,
            num_decimal_places: self.num_decimal_places,
            unit: self.unit,
            opacity: self.opacity,
            ..DecimalNumber::default()
        };
        let Some(head_width) = width(ui, s, &head) else {
            let value = f64::from(ui.sample(&self.value));
            let text = format!("{head}{}", number.format(value));
            return line(
                self.x,
                self.y,
                s,
                self.fill,
                TextAnchor::Start,
                0.0,
                self.opacity,
                Cow::Owned(text),
            );
        };
        let label = line(
            self.x,
            self.y,
            s,
            self.fill,
            TextAnchor::Start,
            0.0,
            self.opacity,
            Cow::Owned(head),
        );
        number.x = self.x + head_width;
        let number = number.render(ui);
        fframes::svgr!(<g>{label}{number}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_manim() {
        let n = DecimalNumber::default();
        assert_eq!(n.format(1234.567), "1,234.57");
        assert_eq!(n.format(-0.001), "0.00");
        assert_eq!(n.format(-3.0), "-3.00");
        let signed = DecimalNumber {
            include_sign: true,
            group_with_commas: false,
            num_decimal_places: 1,
            unit: " m",
            ..DecimalNumber::default()
        };
        assert_eq!(signed.format(12_345.0), "+12345.0 m");
        assert_eq!(signed.format(-2.26), "-2.3 m");
        assert_eq!(signed.format(0.0), "+0.0 m");
        assert_eq!(DecimalNumber::from(Integer::default()).format(41.6), "42");
    }
}
