//! Card, divider and section surfaces.

use gpui::{Div, ParentElement, SharedString, Styled, div, px};

use super::text::label;
use crate::design_system::{Shadow, shadow, tokens};

/// Card fill, 1 px line border, card radius and the card shadow.
pub fn card() -> Div {
    let tokens = tokens();
    div()
        .rounded(px(tokens.metrics.radius_card))
        .border_1()
        .border_color(tokens.palette.line)
        .bg(tokens.palette.card)
        .shadow(shadow(Shadow::Card).into_vec())
}

/// Nested surface inside a card, no shadow.
pub fn card_2() -> Div {
    let tokens = tokens();
    div()
        .rounded(px(tokens.metrics.radius_card))
        .border_1()
        .border_color(tokens.palette.line)
        .bg(tokens.palette.card_2)
}

/// A 1 px horizontal rule.
pub fn divider() -> Div {
    div()
        .h(px(1.))
        .w_full()
        .flex_none()
        .bg(tokens().palette.line)
}

/// A titled group: a `Label` title in `text_2` above a column of children.
pub fn section(title: impl Into<SharedString>) -> Div {
    let tokens = tokens();
    div()
        .flex()
        .flex_col()
        .gap(px(tokens.metrics.space_2))
        .child(label(title).text_color(tokens.palette.text_2))
}
