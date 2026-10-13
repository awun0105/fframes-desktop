//! Select trigger: shows the current value and opens a [`super::menu::Menu`].

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, Window, canvas, div, px,
};

use super::icon::{Icon, icon};
use super::interact::{Handler, focusable, on_activate};
use super::menu::MenuState;
use crate::design_system::{TextStyle, text_style, tokens};

pub const PAD_X: f32 = 10.;
pub const GAP: f32 = 8.;
pub const CHEVRON: f32 = 12.;

#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    value: SharedString,
    anchor: Option<MenuState>,
    disabled: bool,
    on_open: Option<Handler>,
}

impl Select {
    pub fn new(id: impl Into<ElementId>, value_label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            value: value_label.into(),
            anchor: None,
            disabled: false,
            on_open: None,
        }
    }

    /// The menu this trigger opens; the trigger reports its bounds into the
    /// state so the menu can be placed.
    pub fn menu(mut self, state: &MenuState) -> Self {
        self.anchor = Some(state.clone());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(std::rc::Rc::new(handler));
        self
    }
}

impl RenderOnce for Select {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let metrics = tokens().metrics;
        let palette = tokens().palette;
        let element = text_style(
            div()
                .id(self.id)
                .relative()
                .role(Role::ComboBox)
                .aria_label(self.value.clone()),
            TextStyle::Body,
        )
        .flex()
        .items_center()
        .justify_between()
        .gap(px(GAP))
        .h(px(metrics.select_h))
        .px(px(PAD_X))
        .rounded(px(metrics.radius_control))
        .bg(palette.ctl)
        .text_color(palette.text_1)
        .hover(|style| style.bg(palette.ctl_hover));
        let element = focusable(element, self.disabled);
        let element = on_activate(element, self.disabled, self.on_open);
        let element = match self.anchor {
            Some(state) => {
                let cell = state.anchor_cell();
                element.child(
                    canvas(move |bounds, _, _| cell.set(bounds), |_, _, _, _| {})
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                )
            }
            None => element,
        };
        element
            .child(div().min_w_0().overflow_hidden().child(self.value))
            .child(icon(Icon::ChevronDown, CHEVRON, palette.text_3))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_constants_match_spec() {
        let m = tokens().metrics;
        assert_eq!((m.select_h, m.radius_control), (30., 7.));
        assert_eq!((PAD_X, GAP, CHEVRON), (10., 8., 12.));
    }
}
