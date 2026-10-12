//! Labelled field rows and section titles.

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px,
};

use crate::design_system::{TextStyle, text_style, tokens};

pub const LABEL_MIN_W: f32 = 96.;
pub const GAP: f32 = 8.;
pub const SECTION_TITLE_MIN_H: f32 = 22.;

#[derive(IntoElement)]
pub struct FieldRow {
    label: SharedString,
    control: AnyElement,
    hint: Option<SharedString>,
}

impl FieldRow {
    pub fn new(label: impl Into<SharedString>, control: impl IntoElement) -> Self {
        Self {
            label: label.into(),
            control: control.into_any_element(),
            hint: None,
        }
    }

    /// A one-line explanation under the row.
    pub fn hint(mut self, text: impl Into<SharedString>) -> Self {
        self.hint = Some(text.into());
        self
    }
}

impl RenderOnce for FieldRow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let metrics = tokens().metrics;
        let palette = tokens().palette;
        let row = text_style(div(), TextStyle::Body)
            .flex()
            .items_center()
            .gap(px(GAP))
            .min_h(px(metrics.field_row_h))
            .text_color(palette.text_1)
            .child(div().min_w(px(LABEL_MIN_W)).flex_none().child(self.label))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .justify_end()
                    .min_w_0()
                    .child(self.control),
            );
        div()
            .flex()
            .flex_col()
            .child(row)
            .children(self.hint.map(|hint| {
                text_style(div(), TextStyle::Small)
                    .text_color(palette.text_3)
                    .pb(px(metrics.space_1))
                    .child(hint)
            }))
    }
}

/// Value text in the `Small` style, for the right side of a field row.
pub fn field_value(text: impl Into<SharedString>) -> gpui::Div {
    text_style(div(), TextStyle::Small)
        .text_color(tokens().palette.text_2)
        .child(text.into())
}

#[derive(IntoElement)]
pub struct SectionTitle {
    text: SharedString,
    trailing: Option<AnyElement>,
}

impl SectionTitle {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            trailing: None,
        }
    }

    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing = Some(element.into_any_element());
        self
    }
}

impl RenderOnce for SectionTitle {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        text_style(div(), TextStyle::Label)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(GAP))
            .min_h(px(SECTION_TITLE_MIN_H))
            .text_color(tokens().palette.text_2)
            .child(self.text)
            .children(self.trailing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_constants_match_spec() {
        assert_eq!(tokens().metrics.field_row_h, 34.);
        assert_eq!((LABEL_MIN_W, GAP, SECTION_TITLE_MIN_H), (96., 8., 22.));
        assert_eq!(TextStyle::Small.size(), 11.);
    }
}
