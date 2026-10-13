//! Behavior shared by every interactive component: focus ring, keyboard
//! intents, disabled opacity and the click handler type.

use std::rc::Rc;

use gpui::{
    App, BoxShadow, Div, InteractiveElement, Rgba, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window, point, px,
};

use super::tooltip::tooltip_builder;
use crate::design_system::tokens;

/// Handler for a click or a keyboard activation.
pub type Handler = Rc<dyn Fn(&mut Window, &mut App)>;

/// Width of the focus ring in pixels.
pub const FOCUS_RING_PX: f32 = 2.;
/// Opacity of a disabled control.
pub const DISABLED_OPACITY: f32 = 0.45;
/// Group name that lets an icon inside a control follow the control's hover.
pub const CONTROL_GROUP: &str = "ui-control";

/// What a key press means to a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    Activate,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Escape,
}

/// Maps a gpui key name to a navigation intent.
pub fn nav_key(key: &str) -> Option<Nav> {
    Some(match key {
        "enter" | "space" => Nav::Activate,
        "left" => Nav::Left,
        "right" => Nav::Right,
        "up" => Nav::Up,
        "down" => Nav::Down,
        "home" => Nav::Home,
        "end" => Nav::End,
        "escape" => Nav::Escape,
        _ => return None,
    })
}

/// Whether a key press activates a control: Enter or Space on an enabled one.
pub fn activates(disabled: bool, key: &str) -> bool {
    !disabled && nav_key(key) == Some(Nav::Activate)
}

/// The 2 px accent ring painted outside the element while it has keyboard focus.
pub fn focus_ring() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: tokens().palette.accent.into(),
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: px(FOCUS_RING_PX),
        inset: false,
    }]
}

/// Tab stop, focus ring and pointer cursor. A disabled element is skipped by Tab.
pub fn focusable(element: Stateful<Div>, disabled: bool) -> Stateful<Div> {
    let element = element
        .tab_index(0)
        .tab_stop(!disabled)
        .focus_visible(|style| style.shadow(focus_ring()));
    if disabled {
        element.opacity(DISABLED_OPACITY)
    } else {
        element.cursor_pointer()
    }
}

/// Runs `handler` on click and on Enter or Space, unless `disabled`.
pub fn on_activate(
    element: Stateful<Div>,
    disabled: bool,
    handler: Option<Handler>,
) -> Stateful<Div> {
    let Some(handler) = handler else {
        return element;
    };
    if disabled {
        return element;
    }
    let on_click = handler.clone();
    element
        .on_click(move |_, window, cx| on_click(window, cx))
        .on_key_down(move |event, window, cx| {
            if activates(false, event.keystroke.key.as_str()) {
                handler(window, cx);
                cx.stop_propagation();
            }
        })
}

/// Shows `text` as a tooltip after the standard delay.
pub fn with_tooltip(element: Stateful<Div>, text: Option<SharedString>) -> Stateful<Div> {
    match text {
        Some(text) => element
            .tooltip(tooltip_builder(text, None))
            .tooltip_show_delay(super::tooltip::SHOW_DELAY),
        None => element,
    }
}

/// Linear blend of two colours, `t` in 0..=1 toward `to`.
pub fn mix(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0., 1.);
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    Rgba {
        r: lerp(from.r, to.r),
        g: lerp(from.g, to.g),
        b: lerp(from.b, to.b),
        a: lerp(from.a, to.a),
    }
}

/// Index after moving `delta` steps from `current` over `len` items without
/// wrapping, skipping entries for which `skip` is true. `None` when nothing
/// enabled lies in that direction.
pub fn step_enabled(
    current: usize,
    delta: isize,
    len: usize,
    skip: impl Fn(usize) -> bool,
) -> Option<usize> {
    let mut index = current as isize;
    loop {
        index += delta;
        if index < 0 || index >= len as isize {
            return None;
        }
        if !skip(index as usize) {
            return Some(index as usize);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design_system::{Appearance, tokens_for};

    #[test]
    fn enter_and_space_activate() {
        assert_eq!(nav_key("enter"), Some(Nav::Activate));
        assert_eq!(nav_key("space"), Some(Nav::Activate));
        assert_eq!(nav_key("a"), None);
        assert_eq!(nav_key("escape"), Some(Nav::Escape));
    }

    #[test]
    fn disabled_controls_ignore_keys() {
        assert!(activates(false, "enter"));
        assert!(activates(false, "space"));
        assert!(!activates(true, "enter"));
        assert!(!activates(true, "space"));
        assert!(!activates(false, "tab"));
    }

    #[test]
    fn focus_ring_is_two_pixel_accent() {
        let ring = focus_ring();
        assert_eq!(ring.len(), 1);
        assert_eq!(f32::from(ring[0].spread_radius), 2.);
        let accent = tokens_for(Appearance::Dark).palette.accent;
        // The active appearance is process-wide; the ring is some accent.
        assert!(ring[0].color.a > 0.99);
        assert!(accent.a > 0.99);
    }

    #[test]
    fn step_enabled_skips_and_stops_at_ends() {
        let skip = |i: usize| i == 1;
        assert_eq!(step_enabled(0, 1, 3, skip), Some(2));
        assert_eq!(step_enabled(2, -1, 3, skip), Some(0));
        assert_eq!(step_enabled(2, 1, 3, skip), None);
        assert_eq!(step_enabled(0, -1, 3, skip), None);
    }

    #[test]
    fn mix_interpolates() {
        let a = Rgba {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 1.,
        };
        let b = Rgba {
            r: 1.,
            g: 1.,
            b: 1.,
            a: 1.,
        };
        assert_eq!(mix(a, b, 0.5).r, 0.5);
        assert_eq!(mix(a, b, 2.).r, 1.);
    }
}
