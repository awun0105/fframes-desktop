//! Segmented control: a trough of mutually exclusive options.

use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use super::interact::{Nav, focusable, nav_key, step_enabled};
use crate::design_system::{Shadow, TextStyle, shadow, text_style, tokens};

pub const TROUGH_PADDING: f32 = 2.;
pub const TROUGH_RADIUS: f32 = 8.;
pub const ITEM_H: f32 = 24.;
pub const ITEM_PAD_X: f32 = 10.;
pub const ITEM_RADIUS: f32 = 6.;

#[derive(Debug, Clone)]
pub struct SegmentOption {
    pub label: SharedString,
    pub disabled: bool,
}

impl SegmentOption {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// The option a Left or Right key moves to: one step, no wrap, skipping
/// disabled options. `None` when the key is not Left or Right or nothing
/// enabled lies that way.
pub fn segment_target(selected: usize, key: Nav, options: &[SegmentOption]) -> Option<usize> {
    let delta = match key {
        Nav::Left => -1,
        Nav::Right => 1,
        _ => return None,
    };
    step_enabled(selected, delta, options.len(), |i| options[i].disabled)
}

type Select = Rc<dyn Fn(usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Segmented {
    id: ElementId,
    options: Vec<SegmentOption>,
    selected: usize,
    on_select: Option<Select>,
}

impl Segmented {
    pub fn new(id: impl Into<ElementId>, options: Vec<SegmentOption>, selected: usize) -> Self {
        Self {
            id: id.into(),
            options,
            selected,
            on_select: None,
        }
    }

    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Segmented {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let palette = tokens().palette;
        let options = Rc::new(self.options);
        let selected = self.selected;
        let items = options.iter().enumerate().map(|(index, option)| {
            let is_selected = index == selected;
            let item = text_style(
                div()
                    .id((self.id.clone(), index.to_string()))
                    .role(Role::RadioButton),
                TextStyle::Label,
            )
            .aria_label(option.label.clone())
            .flex()
            .items_center()
            .justify_center()
            .h(px(ITEM_H))
            .px(px(ITEM_PAD_X))
            .rounded(px(ITEM_RADIUS));
            let item = focusable(item, option.disabled);
            let item = if is_selected {
                item.bg(palette.card)
                    .text_color(palette.text_1)
                    .shadow(shadow(Shadow::RaisedPill).into_vec())
            } else {
                item.text_color(palette.text_2)
                    .hover(|style| style.bg(palette.ctl_hover))
            };
            let item = if option.disabled {
                item
            } else if let Some(on_select) = self.on_select.clone() {
                let click = on_select.clone();
                let options = options.clone();
                item.on_click(move |_, window, cx| click(index, window, cx))
                    .on_key_down(move |event, window, cx| {
                        match nav_key(event.keystroke.key.as_str()) {
                            Some(Nav::Activate) => {
                                on_select(index, window, cx);
                                cx.stop_propagation();
                            }
                            Some(key @ (Nav::Left | Nav::Right)) => {
                                if let Some(target) = segment_target(selected, key, &options) {
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
            } else {
                item
            };
            item.child(option.label.clone())
        });
        div()
            .id(self.id.clone())
            .role(Role::RadioGroup)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(TROUGH_PADDING))
            .p(px(TROUGH_PADDING))
            .rounded(px(TROUGH_RADIUS))
            .bg(palette.ctl)
            .children(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(disabled: &[bool]) -> Vec<SegmentOption> {
        disabled
            .iter()
            .enumerate()
            .map(|(i, d)| SegmentOption::new(format!("o{i}")).disabled(*d))
            .collect()
    }

    #[test]
    fn size_constants_match_spec() {
        assert_eq!(
            (
                TROUGH_PADDING,
                TROUGH_RADIUS,
                ITEM_H,
                ITEM_PAD_X,
                ITEM_RADIUS
            ),
            (2., 8., 24., 10., 6.)
        );
    }

    #[test]
    fn arrows_move_one_step_without_wrapping() {
        let o = options(&[false, false, false]);
        assert_eq!(segment_target(1, Nav::Right, &o), Some(2));
        assert_eq!(segment_target(1, Nav::Left, &o), Some(0));
        assert_eq!(segment_target(2, Nav::Right, &o), None);
        assert_eq!(segment_target(0, Nav::Left, &o), None);
        assert_eq!(segment_target(0, Nav::Up, &o), None);
    }

    #[test]
    fn disabled_options_are_skipped() {
        let o = options(&[false, true, false]);
        assert_eq!(segment_target(0, Nav::Right, &o), Some(2));
        assert_eq!(segment_target(2, Nav::Left, &o), Some(0));
        let trailing = options(&[false, true]);
        assert_eq!(segment_target(0, Nav::Right, &trailing), None);
    }
}
