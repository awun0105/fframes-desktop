//! The user's light/dark preference and how a window resolves it.

use gpui::{App, Context, Global, Window};

use crate::{AppTheme, Appearance, Theme};

/// The app-wide preference, loaded from the app settings at start.
struct ThemePreference(AppTheme);

impl Global for ThemePreference {}

/// Installs the preference the app settings hold.
pub fn init(cx: &mut App, preference: AppTheme) {
    cx.set_global(ThemePreference(preference));
}

/// The preference in force; `System` until [`init`] ran.
pub fn current_preference(cx: &App) -> AppTheme {
    cx.try_global::<ThemePreference>()
        .map(|preference| preference.0)
        .unwrap_or_default()
}

/// Changes the preference and repaints every window. Persisting it is the
/// caller's job.
pub fn set_preference(preference: AppTheme, cx: &mut App) {
    cx.set_global(ThemePreference(preference));
    cx.refresh_windows();
}

/// Light and Dark force that appearance; System follows the window, which
/// follows the operating system.
pub fn resolve_appearance(preference: AppTheme, system: Appearance) -> Appearance {
    match preference {
        AppTheme::System => system,
        AppTheme::Light => Appearance::Light,
        AppTheme::Dark => Appearance::Dark,
    }
}

impl Theme {
    /// The theme `window` draws with now: the preference, or the window's own
    /// appearance under `System`.
    pub fn for_window(window: &Window, cx: &App) -> Self {
        let system = Appearance::from_window(window.appearance());
        Self::new(resolve_appearance(current_preference(cx), system))
    }

    /// Replaces `self` with the window's current theme; whether it changed.
    pub fn refresh(&mut self, window: &Window, cx: &App) -> bool {
        let next = Self::for_window(window, cx);
        if next.appearance != self.appearance {
            *self = next;
            true
        } else {
            false
        }
    }
}

/// Repaints the view when the operating system flips between light and dark,
/// so a `System` preference follows it without waiting for another event.
pub fn bind_window<T: 'static>(window: &mut Window, cx: &mut Context<T>) {
    cx.observe_window_appearance(window, |_, _, cx| cx.notify())
        .detach();
}

#[cfg(test)]
mod tests {
    use gpui::{Empty, Render, TestAppContext};

    use super::*;

    struct Probe;

    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
            Empty
        }
    }

    fn appearance_in_window(cx: &mut TestAppContext, preference: Option<AppTheme>) -> Appearance {
        if let Some(preference) = preference {
            cx.update(|cx| init(cx, preference));
        }
        let window = cx.add_window(|_, _| Probe);
        window
            .update(cx, |_, window, cx| Theme::for_window(window, cx).appearance)
            .unwrap()
    }

    #[gpui::test]
    fn a_dark_preference_overrides_a_light_window(cx: &mut TestAppContext) {
        // The test platform's windows always report a light appearance.
        assert_eq!(appearance_in_window(cx, None), Appearance::Light);
        assert_eq!(
            appearance_in_window(cx, Some(AppTheme::Dark)),
            Appearance::Dark
        );
        assert_eq!(
            appearance_in_window(cx, Some(AppTheme::System)),
            Appearance::Light
        );
    }

    #[gpui::test]
    fn changing_the_preference_is_seen_by_the_next_render(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| Probe);
        let mut theme = window
            .update(cx, |_, window, cx| Theme::for_window(window, cx))
            .unwrap();
        cx.update(|cx| set_preference(AppTheme::Dark, cx));
        let changed = window
            .update(cx, |_, window, cx| theme.refresh(window, cx))
            .unwrap();
        assert!(changed);
        assert!(theme.is_dark());
        assert_eq!(cx.update(|cx| current_preference(cx)), AppTheme::Dark);
    }
}
