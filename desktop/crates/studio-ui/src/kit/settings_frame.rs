//! The settings frame, after Cap's settings window (`settings_window.rs`): a 227 px nav
//! column of icon-and-label rows beside a scrolling content pane that holds the page's
//! sections.
//!
//! Cap's nav rows answer the mouse only. Here every row is a tab stop with the focus
//! ring, and Enter or Space opens it, like every other control. The frame can start with
//! a Back row, for a frame shown in place of another view rather than in its own window.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px, rgb,
};

use studio_theme::Theme;

use super::interact::{BoundsHandler, Handler, bounds_probe, focusable};
use super::{Icon, icon};

/// Cap's `SIDEBAR_WIDTH`.
pub const SETTINGS_NAV_W: f32 = 227.;
const NAV_PADDING_X: f32 = 10.;
const CONTENT_PADDING_X: f32 = 16.;

/// One page in the nav column.
#[derive(Debug, Clone)]
pub struct SettingsNavItem {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Icon,
    pub selected: bool,
}

impl SettingsNavItem {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: Icon,
        selected: bool,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            selected,
        }
    }
}

type SelectHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SettingsFrame {
    id: ElementId,
    theme: Theme,
    items: Vec<SettingsNavItem>,
    on_select: Option<SelectHandler>,
    back: Option<Handler>,
    back_bounds: Option<BoundsHandler>,
    content: Vec<AnyElement>,
}

impl SettingsFrame {
    pub fn settings(theme: &Theme, id: impl Into<ElementId>, items: Vec<SettingsNavItem>) -> Self {
        Self {
            id: id.into(),
            theme: *theme,
            items,
            on_select: None,
            back: None,
            back_bounds: None,
            content: Vec::new(),
        }
    }

    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Adds the Back row (`settings-back`) at the top of the nav column.
    pub fn on_back(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.back = Some(Rc::new(handler));
        self
    }

    /// Reports where the Back row was painted.
    pub fn on_back_bounds(
        mut self,
        report: impl Fn(gpui::Bounds<gpui::Pixels>, &mut App) + 'static,
    ) -> Self {
        self.back_bounds = Some(Rc::new(report));
        self
    }

    /// The page's sections, 28 px apart.
    pub fn children(mut self, content: impl IntoIterator<Item = AnyElement>) -> Self {
        self.content.extend(content);
        self
    }
}

/// A nav row: Cap's `.cap-settings-nav-item`, 30 px high with a 16 px icon, tinted with
/// the accent when selected.
fn nav_row(
    theme: &Theme,
    id: SharedString,
    label: SharedString,
    row_icon: Icon,
    selected: bool,
) -> gpui::Stateful<gpui::Div> {
    let accent = rgb(Theme::SETTINGS_ACCENT);
    let hover = theme.settings_hover();
    focusable(div().id(id), theme, false)
        .role(Role::Tab)
        .aria_selected(selected)
        .aria_label(label.clone())
        .relative()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .h(px(30.))
        .px(px(6.))
        .gap(px(8.))
        .rounded(px(8.))
        .text_size(px(13.))
        .cursor_pointer()
        .when(selected, |row| {
            row.bg(Theme::with_alpha(
                accent,
                if theme.is_dark() { 0.22 } else { 0.12 },
            ))
            .font_weight(FontWeight::MEDIUM)
        })
        .when(!selected, |row| row.hover(move |style| style.bg(hover)))
        .child(
            icon(
                row_icon,
                16.,
                if selected {
                    accent.into()
                } else {
                    theme.settings_muted()
                },
            )
            .flex_shrink_0(),
        )
        .child(label)
}

impl RenderOnce for SettingsFrame {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let SettingsFrame {
            id,
            theme,
            items,
            on_select,
            back,
            back_bounds,
            content,
        } = self;

        let back = back.map(|handler| {
            nav_row(
                &theme,
                "settings-back".into(),
                "Back".into(),
                Icon::ArrowLeft,
                false,
            )
            .role(Role::Button)
            .children(back_bounds.map(bounds_probe))
            .on_click(move |_, window, cx| handler(window, cx))
        });
        let rows = items.into_iter().enumerate().map(|(index, item)| {
            let handler = on_select.clone();
            nav_row(&theme, item.id, item.label, item.icon, item.selected)
                .when_some(handler, |row, handler| {
                    row.on_click(move |_, window, cx| handler(&index, window, cx))
                })
        });

        div()
            .id(id)
            .size_full()
            .flex()
            .flex_row()
            .overflow_hidden()
            .text_color(theme.settings_text())
            .child(
                div()
                    .id("settings-nav")
                    .role(Role::TabList)
                    .flex()
                    .flex_col()
                    .flex_none()
                    .h_full()
                    .w(px(SETTINGS_NAV_W))
                    .p(px(NAV_PADDING_X))
                    .gap(px(2.))
                    .overflow_y_scroll()
                    .bg(theme.settings_sidebar_bg())
                    .children(back)
                    .children(rows),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_hidden()
                    .bg(theme.settings_content_bg())
                    .border_l_1()
                    .border_color(theme.settings_divider())
                    .child(
                        div()
                            .id("settings-page")
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .pt(px(18.))
                            .px(px(CONTENT_PADDING_X))
                            .pb(px(28.))
                            .gap(px(28.))
                            .children(content),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, Render, TestAppContext};

    use super::*;
    use crate::kit::test_support::{Log, log, press, tab};

    struct Frame {
        selected: usize,
        picks: Log<&'static str>,
    }

    impl Render for Frame {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::for_window(window, cx);
            let items = ["Appearance", "Setup"]
                .into_iter()
                .enumerate()
                .map(|(i, label)| {
                    SettingsNavItem::new(label, label, Icon::Palette, i == self.selected)
                })
                .collect();
            let picks = self.picks.clone();
            let back = self.picks.clone();
            SettingsFrame::settings(&theme, "frame", items)
                .on_select(cx.listener(move |this, index: &usize, _, cx| {
                    picks.borrow_mut().push(["Appearance", "Setup"][*index]);
                    this.selected = *index;
                    cx.notify();
                }))
                .on_back(move |_, _| back.borrow_mut().push("back"))
        }
    }

    #[gpui::test]
    fn back_and_every_page_are_tab_stops_that_activate_with_the_keyboard(cx: &mut TestAppContext) {
        let picks = log();
        let shared = picks.clone();
        let (_, cx) = cx.add_window_view(|_, _| Frame {
            selected: 0,
            picks: shared,
        });
        tab(cx);
        press(cx, "enter");
        tab(cx);
        tab(cx);
        press(cx, "space");
        assert_eq!(*picks.borrow(), ["back", "Setup"]);
    }
}
