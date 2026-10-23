use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line, width};

/// One line of text at the largest size, up to `style.size`, that fits into `max_width`
/// pixels. It shrinks by `step` pixels down to `min_size`, where it stays even if it is
/// still too wide. Without a measurable font it keeps `style.size`.
#[derive(Debug, Clone, PartialEq)]
pub struct FitText<'a> {
    pub text: Cow<'a, str>,
    /// The anchor point on the baseline.
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub max_width: f32,
    pub min_size: usize,
    pub step: usize,
    pub fill: Color,
    pub anchor: TextAnchor,
    pub opacity: f32,
}

impl Themed for FitText<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            text: Cow::Borrowed(""),
            x: 0.0,
            y: 0.0,
            style: t.typography.title,
            max_width: 1600.0,
            min_size: 24,
            step: 4,
            fill: t.colors.title,
            anchor: TextAnchor::Start,
            opacity: 1.0,
        }
    }
}

impl Default for FitText<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl FitText<'_> {
    /// The font size the text is drawn at.
    pub fn size(&self, ui: &Ui<'_, '_, '_>) -> usize {
        let step = self.step.max(1);
        let mut style = self.style;
        loop {
            match width(ui, style, &self.text) {
                Some(w) if w > self.max_width && style.size > self.min_size => {
                    style.size = style.size.saturating_sub(step).max(self.min_size);
                }
                _ => return style.size,
            }
        }
    }
}

impl<'a> Component<'a> for FitText<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let style = TextStyle {
            size: self.size(ui),
            ..self.style
        };
        line(
            self.x,
            self.y,
            style,
            self.fill,
            self.anchor,
            0.0,
            self.opacity,
            self.text,
        )
    }
}
