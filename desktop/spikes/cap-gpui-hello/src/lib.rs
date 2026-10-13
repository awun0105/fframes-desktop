//! Spike: Cap's theme, button and Windows caption, and fframes' design-system
//! button, rendered on Cap's GPUI fork.

pub mod menus;
pub mod platform;
pub mod store;
pub mod theme;
pub mod ui;

use std::borrow::Cow;

use gpui::{
    App, AssetSource, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    Result, SharedString, Styled, Window, actions, div, prelude::FluentBuilder, px,
};

use theme::Theme;

actions!(hello, [Increment]);

/// Cap's caption glyphs, then fframes' icon set.
pub struct SpikeAssets;

const CAP_ICONS: &[(&str, &[u8])] = &[
    (
        "icons/caption-close-windows.svg",
        include_bytes!("../assets/icons/caption-close-windows.svg"),
    ),
    (
        "icons/caption-maximize-windows.svg",
        include_bytes!("../assets/icons/caption-maximize-windows.svg"),
    ),
    (
        "icons/caption-minimize-windows.svg",
        include_bytes!("../assets/icons/caption-minimize-windows.svg"),
    ),
    (
        "icons/caption-restore-windows.svg",
        include_bytes!("../assets/icons/caption-restore-windows.svg"),
    ),
];

impl AssetSource for SpikeAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = CAP_ICONS.iter().find(|(name, _)| *name == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        ff_ui::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut listed = ff_ui::assets::Assets.list(path)?;
        listed.extend(
            CAP_ICONS
                .iter()
                .filter(|(name, _)| name.starts_with(path))
                .map(|(name, _)| SharedString::from(*name)),
        );
        Ok(listed)
    }
}

pub struct HelloView {
    pub count: usize,
    focus: FocusHandle,
}

impl HelloView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self { count: 0, focus }
    }

    fn increment(&mut self, _: &Increment, _: &mut Window, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
}

impl Render for HelloView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_window(window, cx, false);

        let header = div()
            .when(cfg!(target_os = "windows"), |header| {
                header.window_control_area(gpui::WindowControlArea::Drag)
            })
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .h(px(40.))
            .flex_shrink_0()
            .bg(theme.header_bg())
            .child(div().px(px(12.)).child("cap-gpui-hello"))
            .child(div().flex_1().h_full());
        #[cfg(target_os = "windows")]
        let header = header.child(ui::windows_caption::windows_caption_controls(
            theme,
            window.is_window_active(),
            window.is_maximized(),
            true,
            true,
        ));

        let card = div()
            .id("card")
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(24.))
            .rounded(px(16.))
            .bg(theme.body_fill(2))
            .border_1()
            .border_color(theme.body_border(6))
            .child(
                div()
                    .text_size(px(20.))
                    .child("Rounded card on Cap's GPUI fork"),
            )
            .child(format!("Increments: {}", self.count))
            .child(
                ui::button::Button::plain(
                    &theme,
                    "cap-button",
                    ui::button::ButtonVariant::Primary,
                    ui::button::ButtonSize::Md,
                )
                .label("Cap button")
                .on_click(|_, window, cx| window.dispatch_action(Box::new(Increment), cx)),
            )
            .child(
                ff_ui::ui::Button::new("ff-button", "fframes button")
                    .on_click(|window, cx| window.dispatch_action(Box::new(Increment), cx)),
            );

        div()
            .key_context("HelloView")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::increment))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.shell_bg())
            .text_color(theme.body_text())
            .child(header)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(card),
            )
    }
}

pub fn init(cx: &mut App) {
    theme::init(cx);
}

#[cfg(test)]
mod tests {
    use gpui::{TestAppContext, VisualTestContext};

    use super::*;

    #[gpui::test]
    fn increment_action_updates_the_view(cx: &mut TestAppContext) {
        cx.update(init);
        let window = cx.add_window(HelloView::new);
        let mut cx = VisualTestContext::from_window(*window, cx);

        cx.dispatch_action(Increment);
        cx.dispatch_action(Increment);

        window
            .read_with(&cx, |view, _| assert_eq!(view.count, 2))
            .unwrap();
    }
}
