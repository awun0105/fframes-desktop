//! Icon tab rail.

use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Rgba, Role,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use super::icon::{Icon, icon};
use super::interact::{Nav, focusable, nav_key, step_enabled, with_tooltip};
use crate::design_system::tokens;

pub const BAR_PAD_X: f32 = 10.;
pub const TAB_W: f32 = 40.;
pub const TAB_H: f32 = 30.;
/// Gap between tabs.
pub const TAB_GAP: f32 = 4.;

#[derive(Debug, Clone)]
pub struct TabItem {
    pub icon: Icon,
    /// Accessible name and tooltip.
    pub label: SharedString,
    /// Element id of the tab; defaults to the rail id plus the tab index.
    pub id: Option<SharedString>,
}

impl TabItem {
    pub fn new(icon: Icon, label: impl Into<SharedString>) -> Self {
        Self {
            icon,
            label: label.into(),
            id: None,
        }
    }

    /// Fixes the element id so lookups by name keep working.
    pub fn id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = Some(id.into());
        self
    }
}

/// `(fill, icon colour)` of a tab; `None` fill means transparent.
pub fn tab_colors(selected: bool) -> (Option<Rgba>, Rgba) {
    let palette = tokens().palette;
    if selected {
        (Some(palette.ctl_hover), palette.text_1)
    } else {
        (None, palette.text_2)
    }
}

/// The tab a Left or Right key moves to (one step, no wrap).
pub fn tab_target(selected: usize, key: Nav, len: usize) -> Option<usize> {
    let delta = match key {
        Nav::Left => -1,
        Nav::Right => 1,
        _ => return None,
    };
    step_enabled(selected, delta, len, |_| false)
}

type Select = Rc<dyn Fn(usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct TabRail {
    id: ElementId,
    tabs: Vec<TabItem>,
    selected: usize,
    on_select: Option<Select>,
}

impl TabRail {
    pub fn new(id: impl Into<ElementId>, tabs: Vec<TabItem>, selected: usize) -> Self {
        Self {
            id: id.into(),
            tabs,
            selected,
            on_select: None,
        }
    }

    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TabRail {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let metrics = tokens().metrics;
        let palette = tokens().palette;
        let len = self.tabs.len();
        let selected = self.selected;
        let tabs = self.tabs.into_iter().enumerate().map(|(index, tab)| {
            let (fill, glyph) = tab_colors(index == selected);
            let element = div()
                .id(match tab.id.clone() {
                    Some(id) => ElementId::from(id),
                    None => ElementId::from((self.id.clone(), index.to_string())),
                })
                .role(Role::Tab)
                .aria_label(tab.label.clone())
                .flex()
                .items_center()
                .justify_center()
                .w(px(TAB_W))
                .h(px(TAB_H))
                .rounded(px(metrics.radius_tab))
                .child(icon(tab.icon, metrics.icon_md, glyph));
            let element = focusable(element, false);
            let element = match fill {
                Some(fill) => element.bg(fill),
                None => element.hover(|style| style.bg(palette.ctl)),
            };
            let element = with_tooltip(element, Some(tab.label));
            match self.on_select.clone() {
                Some(on_select) => {
                    let click = on_select.clone();
                    element
                        .on_click(move |_, window, cx| click(index, window, cx))
                        .on_key_down(move |event, window, cx| {
                            match nav_key(event.keystroke.key.as_str()) {
                                Some(Nav::Activate) => {
                                    on_select(index, window, cx);
                                    cx.stop_propagation();
                                }
                                Some(key @ (Nav::Left | Nav::Right)) => {
                                    if let Some(target) = tab_target(selected, key, len) {
                                        on_select(target, window, cx);
                                        if key == Nav::Left {
                                            window.focus_prev(cx);
                                        } else {
                                            window.focus_next(cx);
                                        }
                                        cx.stop_propagation();
                                    }
                                }
                                _ => {}
                            }
                        })
                }
                None => element,
            }
        });
        div()
            .id(self.id.clone())
            .role(Role::TabList)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(TAB_GAP))
            .w_full()
            .h(px(metrics.tab_bar_h))
            .px(px(BAR_PAD_X))
            .border_b_1()
            .border_color(palette.line)
            .children(tabs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_constants_match_spec() {
        let m = tokens().metrics;
        assert_eq!((m.tab_bar_h, m.radius_tab, m.icon_md), (46., 9., 16.));
        assert_eq!((BAR_PAD_X, TAB_W, TAB_H), (10., 40., 30.));
    }

    #[test]
    fn selected_tab_uses_control_hover() {
        let p = tokens().palette;
        assert_eq!(tab_colors(true), (Some(p.ctl_hover), p.text_1));
        assert_eq!(tab_colors(false), (None, p.text_2));
    }

    #[test]
    fn keys_move_selection_without_wrapping() {
        assert_eq!(tab_target(1, Nav::Right, 3), Some(2));
        assert_eq!(tab_target(1, Nav::Left, 3), Some(0));
        assert_eq!(tab_target(2, Nav::Right, 3), None);
        assert_eq!(tab_target(0, Nav::Left, 3), None);
        assert_eq!(tab_target(0, Nav::Down, 3), None);
    }
}
