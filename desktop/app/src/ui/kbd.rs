//! Key chip: a small rounded label for a keyboard shortcut.

use gpui::{
    App, IntoElement, ParentElement, RenderOnce, Rgba, SharedString, Styled, Window, div, px,
};

use super::interact::mix;
use crate::design_system::{TextStyle, text_style, tokens};

/// Corner radius of the chip.
pub const RADIUS: f32 = 6.;
/// Horizontal padding of the chip.
pub const PAD_X: f32 = 5.;
/// Vertical padding of the chip.
pub const PAD_Y: f32 = 1.;

#[derive(IntoElement)]
pub struct Kbd {
    text: SharedString,
    inverted: bool,
}

impl Kbd {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            inverted: false,
        }
    }

    /// Colours for use on the dark tooltip surface.
    pub fn inverted(mut self) -> Self {
        self.inverted = true;
        self
    }
}

/// `(fill, text)` of the chip.
pub fn kbd_colors(inverted: bool) -> (Rgba, Rgba) {
    let palette = tokens().palette;
    if inverted {
        (
            mix(palette.tooltip_bg, palette.tooltip_text, 0.2),
            palette.tooltip_text,
        )
    } else {
        (palette.ctl, palette.text_2)
    }
}

impl RenderOnce for Kbd {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (fill, text) = kbd_colors(self.inverted);
        text_style(div(), TextStyle::Caption)
            .flex_none()
            .px(px(PAD_X))
            .py(px(PAD_Y))
            .rounded(px(RADIUS))
            .bg(fill)
            .text_color(text)
            .child(self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_constants_match_spec() {
        assert_eq!(RADIUS, 6.);
        assert_eq!(TextStyle::Caption.size(), 10.);
    }

    #[test]
    fn chip_fill_is_the_control_token() {
        let (fill, _) = kbd_colors(false);
        assert_eq!(fill, tokens().palette.ctl);
    }
}
