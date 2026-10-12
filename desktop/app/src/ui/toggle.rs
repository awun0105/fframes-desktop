//! On/off switch.

use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Rgba, Role,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use super::interact::{focusable, on_activate};
use crate::design_system::{Shadow, shadow, tokens};

pub const TRACK_W: f32 = 34.;
pub const TRACK_H: f32 = 20.;
pub const PADDING: f32 = 2.;
pub const THUMB: f32 = 16.;

/// Track fill for the given state.
pub fn track_color(checked: bool) -> Rgba {
    let palette = tokens().palette;
    if checked {
        palette.accent
    } else {
        palette.ctl_active
    }
}

/// The value after an activation, or `None` when disabled.
pub fn toggled(checked: bool, disabled: bool) -> Option<bool> {
    (!disabled).then_some(!checked)
}

type Change = Rc<dyn Fn(bool, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    checked: bool,
    disabled: bool,
    label: Option<gpui::SharedString>,
    on_change: Option<Change>,
}

impl Toggle {
    pub fn new(id: impl Into<ElementId>, checked: bool) -> Self {
        Self {
            id: id.into(),
            checked,
            disabled: false,
            label: None,
            on_change: None,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Accessible name.
    pub fn label(mut self, label: impl Into<gpui::SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn on_change(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Toggle {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let palette = tokens().palette;
        let mut element = div().id(self.id).role(Role::Switch);
        if let Some(label) = self.label {
            element = element.aria_label(label);
        }
        let element = element
            .flex()
            .flex_none()
            .items_center()
            .w(px(TRACK_W))
            .h(px(TRACK_H))
            .p(px(PADDING))
            .rounded_full()
            .bg(track_color(self.checked));
        let element = if self.checked {
            element.justify_end()
        } else {
            element.justify_start()
        };
        let element = focusable(element, self.disabled);
        let handler = self.on_change.map(|change| {
            let next = toggled(self.checked, self.disabled);
            Rc::new(move |window: &mut Window, cx: &mut App| {
                if let Some(value) = next {
                    change(value, window, cx);
                }
            }) as super::interact::Handler
        });
        on_activate(element, self.disabled, handler).child(
            div()
                .size(px(THUMB))
                .flex_none()
                .rounded_full()
                .bg(palette.thumb)
                .shadow(shadow(Shadow::Thumb).into_vec()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_constants_match_spec() {
        assert_eq!((TRACK_W, TRACK_H, PADDING, THUMB), (34., 20., 2., 16.));
        assert_eq!(TRACK_H - 2. * PADDING, THUMB);
    }

    #[test]
    fn track_colours_follow_state() {
        let p = tokens().palette;
        assert_eq!(track_color(true), p.accent);
        assert_eq!(track_color(false), p.ctl_active);
    }

    #[test]
    fn activation_flips_unless_disabled() {
        assert_eq!(toggled(false, false), Some(true));
        assert_eq!(toggled(true, false), Some(false));
        assert_eq!(toggled(true, true), None);
    }
}
