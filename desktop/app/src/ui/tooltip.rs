//! Dark tooltip with an optional key chip, built on gpui's tooltip support.

use std::time::Duration;

use gpui::{
    AnyView, App, AppContext, Context, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div, px,
};

use super::kbd::Kbd;
use crate::design_system::{TextStyle, text_style, tokens};

/// Delay before a tooltip appears.
pub const SHOW_DELAY: Duration = Duration::from_millis(200);
pub const PAD_X: f32 = 6.;
pub const PAD_Y: f32 = 4.;
pub const RADIUS: f32 = 6.;
/// Gap between the tooltip text and its key chip.
pub const GAP: f32 = 6.;

pub struct Tooltip {
    text: SharedString,
    key: Option<SharedString>,
}

impl Tooltip {
    pub fn new(text: impl Into<SharedString>, key: Option<SharedString>) -> Self {
        Self {
            text: text.into(),
            key,
        }
    }
}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let palette = tokens().palette;
        text_style(div(), TextStyle::Label)
            .flex()
            .items_center()
            .gap(px(GAP))
            .px(px(PAD_X))
            .py(px(PAD_Y))
            .rounded(px(RADIUS))
            .bg(palette.tooltip_bg)
            .text_color(palette.tooltip_text)
            .child(self.text.clone())
            .children(self.key.clone().map(|key| Kbd::new(key).inverted()))
    }
}

/// A builder for gpui's `.tooltip(...)`.
pub fn tooltip_builder(
    text: SharedString,
    key: Option<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_, cx| {
        let text = text.clone();
        let key = key.clone();
        cx.new(|_| Tooltip::new(text, key)).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_constants_match_spec() {
        assert_eq!((PAD_X, PAD_Y, RADIUS), (6., 4., 6.));
        assert_eq!(SHOW_DELAY, Duration::from_millis(200));
    }
}
