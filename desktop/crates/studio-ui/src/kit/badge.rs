//! Badge and status dot, coloured by a [`Tone`].

use gpui::{
    App, IntoElement, ParentElement, RenderOnce, Rgba, SharedString, Styled, Window, div, px,
};

use studio_theme::{TextStyle, Theme, text_style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

/// Solid colour that represents `tone` (dot fill, notice bar, icon).
pub fn tone_color(theme: &Theme, tone: Tone) -> Rgba {
    match tone {
        Tone::Neutral => theme.editor.text_3,
        Tone::Info => theme.editor.accent,
        Tone::Success => theme.green_11,
        Tone::Warning => theme.amber_11,
        Tone::Danger => theme.red_11,
    }
}

/// Soft background for `tone`; neutral and info use the control fill.
pub fn tone_surface(theme: &Theme, tone: Tone) -> Rgba {
    match tone {
        Tone::Neutral | Tone::Info => theme.editor.ctl,
        Tone::Success => theme.green_3,
        Tone::Warning => theme.amber_3,
        Tone::Danger => theme.red_3,
    }
}

#[derive(IntoElement)]
pub struct Badge {
    text: SharedString,
    tone: Tone,
}

impl Badge {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            tone: Tone::Neutral,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

impl RenderOnce for Badge {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::for_window(window, cx);
        let (fill, text) = match self.tone {
            Tone::Neutral => (theme.editor.ctl, theme.editor.text_2),
            tone => (tone_surface(&theme, tone), tone_color(&theme, tone)),
        };
        text_style(div(), TextStyle::Caption)
            .flex_none()
            .px(px(6.))
            .py(px(2.))
            .rounded_full()
            .bg(fill)
            .text_color(text)
            .child(self.text)
    }
}

#[derive(IntoElement)]
pub struct StatusDot {
    tone: Tone,
}

impl StatusDot {
    pub fn new(tone: Tone) -> Self {
        Self { tone }
    }
}

impl RenderOnce for StatusDot {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::for_window(window, cx);
        div()
            .size(px(6.))
            .flex_none()
            .rounded_full()
            .bg(tone_color(&theme, self.tone))
    }
}

#[cfg(test)]
mod tests {
    use studio_theme::Appearance;

    use super::*;

    #[test]
    fn tones_map_to_the_theme() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let theme = Theme::new(appearance);
            assert_eq!(tone_color(&theme, Tone::Neutral), theme.editor.text_3);
            assert_eq!(tone_color(&theme, Tone::Info), theme.editor.accent);
            assert_eq!(tone_color(&theme, Tone::Success), theme.green_11);
            assert_eq!(tone_color(&theme, Tone::Warning), theme.amber_11);
            assert_eq!(tone_color(&theme, Tone::Danger), theme.red_11);
            assert_eq!(tone_surface(&theme, Tone::Neutral), theme.editor.ctl);
            assert_eq!(tone_surface(&theme, Tone::Info), theme.editor.ctl);
            assert_eq!(tone_surface(&theme, Tone::Success), theme.green_3);
            assert_eq!(tone_surface(&theme, Tone::Warning), theme.amber_3);
            assert_eq!(tone_surface(&theme, Tone::Danger), theme.red_3);
        }
    }
}
