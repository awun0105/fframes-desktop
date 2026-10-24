use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line};

/// A small tracked label, like the annotations of an engineering drawing ("01 / RENDER").
/// `uppercase` turns the text to capitals.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct HudLabel<'a> {
    pub text: Cow<'a, str>,
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub anchor: TextAnchor,
    /// Pixels between letters.
    pub letter_spacing: f32,
    pub uppercase: bool,
    pub opacity: f32,
}

impl Themed for HudLabel<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            text: Cow::Borrowed(""),
            x: 0.0,
            y: 0.0,
            style: t.typography.caption,
            fill: t.colors.muted,
            anchor: TextAnchor::Start,
            letter_spacing: 2.5,
            uppercase: true,
            opacity: 1.0,
        }
    }
}

impl Default for HudLabel<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> HudLabel<'a> {
    /// The label as a `<text>` element; it needs no `Ui`, so it can be drawn anywhere.
    pub fn draw(self) -> Svgr<'a> {
        let text = if self.uppercase {
            Cow::Owned(self.text.to_uppercase())
        } else {
            self.text
        };
        line(
            self.x,
            self.y,
            self.style,
            self.fill,
            self.anchor,
            self.letter_spacing,
            self.opacity,
            text,
        )
    }
}

impl<'a> Component<'a> for HudLabel<'a> {
    fn render(self, _ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.draw()
    }
}
