//! The tab rail: `ConfigSidebar`'s six icon-only triggers under a
//! `KTabs.Indicator` box (`ConfigSidebar.tsx:593-692`).
//!
//! **This is not the settings sidebar's shape.** The usage matrix lists `KTabs`
//! in the editor and the screenshot editor only; the settings window's sidebar
//! is a vertical nav *list* (icon plus label rows, `.cap-settings-nav`), and
//! `settings.tsx` renders it with `<For>` over `settingsItems`, not with tabs.
//! They were checked against each other and kept apart deliberately -- merging
//! them would mean inventing a shared component neither app has.
//!
//! The indicator does not slide (no transform in this gpui rev); the selected
//! item paints the `size-9 rounded-lg bg-gray-3` box itself, which is what the
//! editor window has always done.
//!
//! Each tab is exactly one element -- the pill. A wrapper carrying the hit box,
//! the tooltip anchor or a focus ring would paint its own square corners around
//! the pill's radius, which is what the first pass shipped.

use gpui::{
    App, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px, svg,
};

use studio_theme::Theme;

use super::interact::{Nav, focusable, nav_key, step_enabled};

#[derive(Debug, Clone)]
pub struct TabRailItem {
    pub icon: SharedString,
    /// The tab's name. It is not drawn -- the rail is icon-only -- so it is
    /// carried as the tooltip instead.
    pub label: SharedString,
    pub selected: bool,
    pub disabled: bool,
    /// The tab's element id; `{rail id}-{index}` when `None`.
    pub id: Option<SharedString>,
    /// A small accent dot on the tab: something behind it waits for the user.
    pub dot: bool,
}

impl TabRailItem {
    pub fn new(
        icon: impl Into<SharedString>,
        label: impl Into<SharedString>,
        selected: bool,
        disabled: bool,
    ) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            selected,
            disabled,
            id: None,
            dot: false,
        }
    }

    pub fn id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn dot(mut self, dot: bool) -> Self {
        self.dot = dot;
        self
    }
}

/// Diameter of a tab's attention dot.
const DOT: f32 = 6.;

/// The tab a Left or Right key moves to: one step, no wrap, skipping disabled
/// tabs.
pub fn tab_target(from: usize, key: Nav, items: &[TabRailItem]) -> Option<usize> {
    let delta = match key {
        Nav::Left => -1,
        Nav::Right => 1,
        _ => return None,
    };
    step_enabled(from, delta, items.len(), |i| items[i].disabled)
}

/// The index arrives by reference, the same shape the other index-dispatching
/// components use, so `cx.listener` builds the handler directly.
type TabHandler = Box<dyn Fn(&usize, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct TabRail {
    id: ElementId,
    items: Vec<TabRailItem>,
    height: Pixels,
    box_size: Pixels,
    box_width: Pixels,
    icon_size: Pixels,
    bg: Hsla,
    border: Hsla,
    indicator: Hsla,
    hover: Hsla,
    selected_icon: Hsla,
    idle_icon: Hsla,
    theme: Theme,
    on_select: Option<TabHandler>,
}

impl TabRail {
    /// The editor's config sidebar rail: a 46px bar on the card surface under
    /// an `ed-line` hairline, with 40x30 tab boxes and 16px icons.
    pub fn editor(
        theme: &Theme,
        id: impl Into<ElementId>,
        panel_bg: Hsla,
        items: Vec<TabRailItem>,
    ) -> Self {
        Self {
            id: id.into(),
            items,
            height: px(46.),
            box_size: px(30.),
            box_width: px(40.),
            icon_size: px(16.),
            bg: panel_bg,
            border: Hsla::from(theme.editor.line),
            indicator: Hsla::from(theme.editor.ctl_hover),
            hover: Hsla::from(theme.editor.ctl),
            selected_icon: Hsla::from(theme.editor.text_1),
            idle_icon: Hsla::from(theme.editor.text_2),
            theme: *theme,
            on_select: None,
        }
    }

    /// `h-16` on the editor's rail. Kept a parameter so the call site can go
    /// on quoting its own metric.
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for TabRail {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let TabRail {
            id,
            items,
            height,
            box_size,
            box_width,
            icon_size,
            bg,
            border,
            indicator,
            hover,
            selected_icon,
            idle_icon,
            theme,
            on_select,
        } = self;

        let prefix: SharedString = match &id {
            ElementId::Name(name) => name.clone(),
            other => SharedString::from(format!("{other:?}")),
        };
        let handler: Option<std::rc::Rc<TabHandler>> = on_select.map(std::rc::Rc::new);
        let keyed = std::rc::Rc::new(items.clone());

        div()
            .id(id)
            .role(Role::TabList)
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .h(height)
            .flex_none()
            .overflow_hidden()
            .rounded_t(px(11.))
            .justify_around()
            .px(px(10.))
            .border_b_1()
            .border_color(border)
            .bg(bg)
            .children(items.into_iter().enumerate().map(|(index, item)| {
                let handler = handler.clone();
                let keyed = keyed.clone();
                let selected = item.selected;
                let disabled = item.disabled;
                let label = item.label.clone();
                let item_id = item
                    .id
                    .clone()
                    .unwrap_or_else(|| SharedString::from(format!("{prefix}-{index}")));

                focusable(div().id(item_id), &theme, disabled)
                    .role(Role::Tab)
                    .aria_selected(selected)
                    .aria_label(item.label.clone())
                    .relative()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .h(box_size)
                    .w(box_width)
                    .rounded(px(9.))
                    .when(disabled, |this| this.opacity(0.6))
                    .when(selected, |this| this.bg(indicator))
                    .when(!selected && !disabled, |this| {
                        this.cursor_pointer().hover(move |style| style.bg(hover))
                    })
                    .tooltip_show_delay(crate::kit::TOOLTIP_SHOW_DELAY)
                    .tooltip(move |_, cx| crate::kit::Tooltip::new(&theme, label.clone()).view(cx))
                    .child(
                        svg()
                            .path(item.icon)
                            .size(icon_size)
                            .text_color(if selected { selected_icon } else { idle_icon }),
                    )
                    .when(item.dot, |this| {
                        this.child(
                            div()
                                .absolute()
                                .top(px(4.))
                                .right(px(8.))
                                .size(px(DOT))
                                .rounded_full()
                                .bg(theme.editor.accent),
                        )
                    })
                    .when_some(handler.filter(|_| !disabled), |this, handler| {
                        let keys = handler.clone();
                        this.on_click(move |_, window, cx| handler(&index, window, cx))
                            .on_key_down(move |event, window, cx| {
                                let Some(key @ (Nav::Left | Nav::Right)) =
                                    nav_key(event.keystroke.key.as_str())
                                else {
                                    return;
                                };
                                // Every enabled item is a tab stop, so the key belongs to the
                                // focused one, which need not be the selected one.
                                if let Some(target) = tab_target(index, key, &keyed) {
                                    keys(&target, window, cx);
                                    if key == Nav::Left {
                                        window.focus_prev(cx);
                                    } else {
                                        window.focus_next(cx);
                                    }
                                    cx.stop_propagation();
                                }
                            })
                    })
            }))
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, Render, TestAppContext};

    use super::*;
    use crate::kit::interact::Nav;
    use crate::kit::test_support::{Log, log, press, tab};

    fn items(disabled: &[bool]) -> Vec<TabRailItem> {
        disabled
            .iter()
            .enumerate()
            .map(|(i, d)| TabRailItem::new("icons/x.svg", format!("t{i}"), i == 0, *d))
            .collect()
    }

    #[test]
    fn keys_move_selection_without_wrapping() {
        let t = items(&[false, false, false]);
        assert_eq!(tab_target(1, Nav::Right, &t), Some(2));
        assert_eq!(tab_target(1, Nav::Left, &t), Some(0));
        assert_eq!(tab_target(2, Nav::Right, &t), None);
        assert_eq!(tab_target(0, Nav::Left, &t), None);
        assert_eq!(tab_target(0, Nav::Down, &t), None);
        assert_eq!(
            tab_target(0, Nav::Right, &items(&[false, true, false])),
            Some(2)
        );
    }

    struct Rail {
        selected: usize,
        picks: Log<usize>,
    }

    impl Render for Rail {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::for_window(window, cx);
            let picks = self.picks.clone();
            let items = ["Project", "Style", "Agent"]
                .into_iter()
                .enumerate()
                .map(|(i, label)| TabRailItem::new("icons/x.svg", label, i == self.selected, false))
                .collect();
            TabRail::editor(&theme, "rail", theme.editor.card.into(), items).on_select(cx.listener(
                move |this, index: &usize, _, cx| {
                    picks.borrow_mut().push(*index);
                    this.selected = *index;
                    cx.notify();
                },
            ))
        }
    }

    #[gpui::test]
    fn arrows_step_from_the_focused_tab_not_the_selected_one(cx: &mut TestAppContext) {
        let picks = log();
        let shared = picks.clone();
        let (_, cx) = cx.add_window_view(|_, _| Rail {
            selected: 2,
            picks: shared,
        });
        tab(cx);
        press(cx, "right");
        press(cx, "right");
        assert_eq!(*picks.borrow(), [1, 2]);
    }

    #[gpui::test]
    fn arrows_move_between_tabs(cx: &mut TestAppContext) {
        let picks = log();
        let shared = picks.clone();
        let (_, cx) = cx.add_window_view(|_, _| Rail {
            selected: 0,
            picks: shared,
        });
        tab(cx);
        press(cx, "right");
        press(cx, "right");
        press(cx, "right");
        press(cx, "left");
        assert_eq!(*picks.borrow(), [1, 2, 1]);
    }
}
