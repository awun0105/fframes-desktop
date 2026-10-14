//! Text helpers over the theme's text styles, for the editor palette.

use gpui::{Div, IntoElement, ParentElement, SharedString, Styled, div, px};

use studio_theme::{METRICS, TextStyle, Theme, text_style};

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

/// A section's title row: `Label` text in `text_2`, at least 22px high, with
/// any further child (a count, an action) pushed to the end.
pub fn section_title(theme: &Theme, text: impl Into<SharedString>) -> Div {
    text_style(div(), TextStyle::Label)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .min_h(px(22.))
        .text_color(theme.editor.text_2)
        .child(text.into())
}

/// A labelled control on one row: `Body` text in `text_1` at least 96px wide,
/// the control pushed to the end, the row at least a field row high.
pub fn field_row(theme: &Theme, label: impl Into<SharedString>, control: impl IntoElement) -> Div {
    text_style(div(), TextStyle::Body)
        .flex()
        .items_center()
        .gap(px(8.))
        .min_h(px(METRICS.field_row_h))
        .text_color(theme.editor.text_1)
        .child(div().min_w(px(96.)).flex_none().child(label.into()))
        .child(
            div()
                .flex()
                .flex_1()
                .items_center()
                .justify_end()
                .min_w_0()
                .child(control),
        )
}

#[cfg(test)]
mod tests {
    use gpui::{Context, InteractiveElement, IntoElement, Render, TestAppContext, Window, px};

    use super::*;

    struct Titled;

    impl Render for Titled {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::for_window(window, cx);
            div().w(px(300.)).child(
                section_title(&theme, "Files")
                    .debug_selector(|| "title".into())
                    .child(
                        div()
                            .debug_selector(|| "trailing".into())
                            .w(px(20.))
                            .h(px(10.)),
                    ),
            )
        }
    }

    #[gpui::test]
    fn a_section_title_pushes_its_trailing_element_to_the_end(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| Titled);
        cx.run_until_parked();
        let title = cx.debug_bounds("title").expect("title");
        let trailing = cx.debug_bounds("trailing").expect("trailing");
        assert_eq!(f32::from(title.size.height), 22.);
        assert_eq!(f32::from(trailing.right()), 300.);
    }

    struct Row;

    impl Render for Row {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::for_window(window, cx);
            div().w(px(300.)).child(
                field_row(
                    &theme,
                    "Model",
                    div()
                        .debug_selector(|| "control".into())
                        .w(px(40.))
                        .h(px(10.)),
                )
                .debug_selector(|| "row".into()),
            )
        }
    }

    #[gpui::test]
    fn a_field_row_puts_its_control_at_the_end_of_a_34px_row(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| Row);
        cx.run_until_parked();
        let row = cx.debug_bounds("row").expect("row");
        let control = cx.debug_bounds("control").expect("control");
        assert_eq!(f32::from(row.size.height), 34.);
        assert_eq!(f32::from(control.right()), 300.);
    }
}
