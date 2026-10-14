//! Behaviour shared by every interactive component: the tab stop, the focus
//! ring, keyboard intents and the disabled look.
//!
//! gpui turns Enter and Space on a focused element into its click listener
//! (`ClickEvent::Keyboard`), so a component that is a tab stop and has an
//! `on_click` is keyboard-activatable with nothing more. Components whose keys
//! mean more than "activate" (arrows in a segmented control, a slider or a
//! menu) read them through [`nav_key`].

use gpui::{BoxShadow, Div, InteractiveElement, Stateful, Styled, point, px};

use studio_theme::Theme;

/// Width of the focus ring in pixels.
pub const FOCUS_RING_PX: f32 = 2.;
/// Opacity of a disabled control.
pub const DISABLED_OPACITY: f32 = 0.45;

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
pub fn focus_ring(theme: &Theme) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: theme.editor.accent.into(),
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: px(FOCUS_RING_PX),
        inset: false,
    }]
}

/// Tab stop and focus ring. A disabled element is skipped by Tab.
pub fn focusable(element: Stateful<Div>, theme: &Theme, disabled: bool) -> Stateful<Div> {
    let ring = focus_ring(theme);
    element
        .tab_index(0)
        .tab_stop(!disabled)
        .focus_visible(move |style| style.shadow(ring.clone()))
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
        for theme in [Theme::light(), Theme::dark()] {
            let ring = focus_ring(&theme);
            assert_eq!(ring.len(), 1);
            assert_eq!(f32::from(ring[0].spread_radius), 2.);
            assert_eq!(ring[0].color, gpui::Hsla::from(theme.editor.accent));
        }
    }

    #[test]
    fn step_enabled_skips_and_stops_at_ends() {
        let skip = |i: usize| i == 1;
        assert_eq!(step_enabled(0, 1, 3, skip), Some(2));
        assert_eq!(step_enabled(2, -1, 3, skip), Some(0));
        assert_eq!(step_enabled(2, 1, 3, skip), None);
        assert_eq!(step_enabled(0, -1, 3, skip), None);
    }
}
