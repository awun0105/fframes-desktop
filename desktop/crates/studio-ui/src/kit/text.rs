//! Text helpers over the theme's text styles, for the editor palette.

use gpui::{Div, ParentElement, SharedString, Styled, div};

use studio_theme::{TextStyle, Theme, text_style};

/// `Label` style text in the primary text colour.
pub fn label(theme: &Theme, text: impl Into<SharedString>) -> Div {
    text_style(div(), TextStyle::Label)
        .text_color(theme.editor.text_1)
        .child(text.into())
}

/// `Body` style text in the secondary text colour.
pub fn muted(theme: &Theme, text: impl Into<SharedString>) -> Div {
    text_style(div(), TextStyle::Body)
        .text_color(theme.editor.text_2)
        .child(text.into())
}

/// `Heading` style text in the primary text colour.
pub fn heading(theme: &Theme, text: impl Into<SharedString>) -> Div {
    text_style(div(), TextStyle::Heading)
        .text_color(theme.editor.text_1)
        .child(text.into())
}
