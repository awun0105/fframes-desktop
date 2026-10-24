use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line, width};

/// Manim's `Title`: a line of text centered on `x` near the top, over an underline
/// `underline_width` wide, or as wide as the text with `match_underline_width_to_text` (and
/// a measurable font).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Title<'a> {
    pub text: Cow<'a, str>,
    pub x: f32,
    /// The baseline.
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub include_underline: bool,
    pub match_underline_width_to_text: bool,
    pub underline_width: f32,
    /// Pixels from the baseline down to the underline.
    pub underline_buff: f32,
    pub underline_stroke: Color,
    pub underline_stroke_width: f32,
    pub opacity: f32,
}

impl Themed for Title<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            text: Cow::Borrowed(""),
            x: 960.0,
            y: 130.0,
            style: t.typography.title,
            fill: t.colors.title,
            include_underline: true,
            match_underline_width_to_text: false,
            underline_width: 1720.0,
            underline_buff: 28.0,
            underline_stroke: t.colors.title,
            underline_stroke_width: 3.0,
            opacity: 1.0,
        }
    }
}

impl Default for Title<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Title<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let underline = if self.include_underline {
            let w = if self.match_underline_width_to_text {
                width(ui, self.style, &self.text).unwrap_or(self.underline_width)
            } else {
                self.underline_width
            };
            let (x1, x2, y) = (
                self.x - w / 2.0,
                self.x + w / 2.0,
                self.y + self.underline_buff,
            );
            let (stroke, stroke_width) = (self.underline_stroke, self.underline_stroke_width);
            fframes::svgr!(<line x1={x1} y1={y} x2={x2} y2={y} stroke={stroke} stroke-width={stroke_width} />)
        } else {
            Svgr::empty()
        };
        let text = line(
            self.x,
            self.y,
            self.style,
            self.fill,
            TextAnchor::Middle,
            0.0,
            1.0,
            self.text,
        );
        fframes::svgr!(<g opacity={self.opacity}>{text}{underline}</g>)
    }
}
