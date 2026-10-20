//! `NavRail` -- the editor's vertical rail of labelled icon buttons, each of which opens a
//! drawer beside it.
//!
//! It is not a [`super::TabRail`]: nothing need be selected, pressing the open item closes
//! its drawer, and the items stack vertically with their name under the icon. The caller
//! owns which item is open and toggles it in `on_select`.

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px, svg,
};

use studio_theme::{METRICS, TextStyle, Theme, text_style};

use super::interact::{DISABLED_OPACITY, Nav, focusable, nav_key, step_enabled};

#[derive(Debug, Clone)]
pub struct NavRailItem {
    pub icon: SharedString,
    /// Drawn under the icon; also the accessible name and part of the element id.
    pub label: SharedString,
    /// The item's drawer is open.
    pub open: bool,
    pub disabled: bool,
}

impl NavRailItem {
    pub fn new(icon: impl Into<SharedString>, label: impl Into<SharedString>, open: bool) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            open,
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

type SelectHandler = Box<dyn Fn(&usize, &mut Window, &mut App) + 'static>;

/// Width and height of an item's hit box.
const ITEM_W: f32 = 46.;
const ITEM_H: f32 = 48.;
const ICON: f32 = 18.;

#[derive(IntoElement)]
pub struct NavRail {
    id: ElementId,
    items: Vec<NavRailItem>,
    theme: Theme,
    on_select: Option<SelectHandler>,
}

impl NavRail {
    /// The editor's rail: `METRICS.rail_w` wide on the window background, items `{id}-{label}`.
    pub fn editor(theme: &Theme, id: impl Into<ElementId>, items: Vec<NavRailItem>) -> Self {
        Self {
            id: id.into(),
            items,
            theme: *theme,
            on_select: None,
        }
    }

    /// Receives the index of the pressed item, open or not.
    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for NavRail {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let NavRail {
            id,
            items,
            theme,
            on_select,
        } = self;
        let prefix: SharedString = match &id {
            ElementId::Name(name) => name.clone(),
            other => SharedString::from(format!("{other:?}")),
        };
        let handler = on_select.map(std::rc::Rc::new);
        let colors = theme.editor;
        let disabled_items: std::rc::Rc<[bool]> = items.iter().map(|item| item.disabled).collect();

        div()
            .id(id)
            .role(Role::Group)
            .w(px(METRICS.rail_w))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(METRICS.space_2))
            .pt(px(METRICS.space_2))
            .children(items.into_iter().enumerate().map(|(index, item)| {
                let NavRailItem {
                    icon,
                    label,
                    open,
                    disabled,
                } = item;
                let tint = if open { colors.text_1 } else { colors.text_2 };
                let disabled_items = disabled_items.clone();
                focusable(
                    div().id(SharedString::from(format!("{prefix}-{label}"))),
                    &theme,
                    disabled,
                )
                .role(Role::Button)
                .aria_label(label.clone())
                .aria_expanded(open)
                .w(px(ITEM_W))
                .h(px(ITEM_H))
                .flex_none()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(METRICS.space_1))
                .rounded(px(METRICS.radius_item))
                .when(open, |this| this.bg(colors.ctl_active))
                .when(disabled, |this| this.opacity(DISABLED_OPACITY))
                .when(!open && !disabled, |this| {
                    this.cursor_pointer()
                        .hover(move |style| style.bg(colors.ctl_hover))
                })
                .child(svg().path(icon).size(px(ICON)).text_color(tint))
                .child(
                    text_style(div(), TextStyle::Caption)
                        .text_color(tint)
                        .child(label),
                )
                .when_some(
                    handler.clone().filter(|_| !disabled),
                    |this, handler| {
                        this.on_click(move |_, window, cx| handler(&index, window, cx))
                            .on_key_down(move |event, window, cx| {
                                // Up and Down walk the rail and stop at its ends; the other
                                // keys belong to the window.
                                let delta = match nav_key(event.keystroke.key.as_str()) {
                                    Some(Nav::Up) => -1,
                                    Some(Nav::Down) => 1,
                                    _ => return,
                                };
                                let len = disabled_items.len();
                                if step_enabled(index, delta, len, |i| disabled_items[i]).is_some()
                                {
                                    if delta < 0 {
                                        window.focus_prev(cx);
                                    } else {
                                        window.focus_next(cx);
                                    }
                                }
                                cx.stop_propagation();
                            })
                    },
                )
            }))
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, Render, TestAppContext};

    use super::*;
    use crate::kit::test_support::{Log, log, press, tab};

    struct Rail {
        open: Option<usize>,
        picks: Log<usize>,
    }

    impl Render for Rail {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::for_window(window, cx);
            let picks = self.picks.clone();
            let items = ["Project", "Assets", "Style"]
                .into_iter()
                .enumerate()
                .map(|(i, label)| NavRailItem::new("icons/x.svg", label, self.open == Some(i)))
                .collect();
            NavRail::editor(&theme, "rail", items).on_select(cx.listener(
                move |this, index: &usize, _, cx| {
                    picks.borrow_mut().push(*index);
                    this.open = (this.open != Some(*index)).then_some(*index);
                    cx.notify();
                },
            ))
        }
    }

    #[gpui::test]
    fn enter_presses_the_focused_item_and_down_moves_on(cx: &mut TestAppContext) {
        let picks = log();
        let shared = picks.clone();
        let (view, cx) = cx.add_window_view(|_, _| Rail {
            open: None,
            picks: shared,
        });
        tab(cx);
        press(cx, "enter");
        press(cx, "down");
        press(cx, "enter");
        press(cx, "enter");
        assert_eq!(*picks.borrow(), [0, 1, 1]);
        // The second press on Assets closed its drawer again.
        view.read_with(cx, |rail, _| assert_eq!(rail.open, None));
    }

    #[gpui::test]
    fn up_and_down_stop_at_the_ends_of_the_rail(cx: &mut TestAppContext) {
        let picks = log();
        let shared = picks.clone();
        let (_, cx) = cx.add_window_view(|_, _| Rail {
            open: None,
            picks: shared,
        });
        tab(cx);
        press(cx, "up");
        press(cx, "enter");
        press(cx, "down");
        press(cx, "down");
        press(cx, "down");
        press(cx, "enter");
        assert_eq!(*picks.borrow(), [0, 2]);
    }
}
