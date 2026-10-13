//! Text helpers over the design system text styles.

use gpui::{Div, ParentElement, SharedString, Styled, div};

use crate::design_system::{TextStyle, text_style, tokens};

/// `Label` style text in the primary text colour.
pub fn label(text: impl Into<SharedString>) -> Div {
    text_style(div(), TextStyle::Label)
        .text_color(tokens().palette.text_1)
        .child(text.into())
}

/// `Body` style text in the secondary text colour.
pub fn muted(text: impl Into<SharedString>) -> Div {
    text_style(div(), TextStyle::Body)
        .text_color(tokens().palette.text_2)
        .child(text.into())
}

/// `Heading` style text in the primary text colour.
pub fn heading(text: impl Into<SharedString>) -> Div {
    text_style(div(), TextStyle::Heading)
        .text_color(tokens().palette.text_1)
        .child(text.into())
}
