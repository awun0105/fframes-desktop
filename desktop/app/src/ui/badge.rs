//! Badge and status dot.

use gpui::{
    App, IntoElement, ParentElement, RenderOnce, Rgba, SharedString, Styled, Window, div, px,
};

use crate::design_system::{TextStyle, text_style, tokens};

pub const PAD_X: f32 = 6.;
pub const PAD_Y: f32 = 2.;
pub const DOT: f32 = 6.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

/// Solid colour that represents `tone` (dot fill, notice bar, icon).
pub fn tone_color(tone: Tone) -> Rgba {
    let palette = tokens().palette;
    match tone {
        Tone::Neutral => palette.text_3,
        Tone::Info => palette.accent,
        Tone::Success => palette.success,
        Tone::Warning => palette.warning,
        Tone::Danger => palette.danger,
    }
}

/// Soft background for `tone`; neutral and info use the control fill.
pub fn tone_surface(tone: Tone) -> Rgba {
    let palette = tokens().palette;
    match tone {
        Tone::Neutral | Tone::Info => palette.ctl,
        Tone::Success => palette.success_surface,
        Tone::Warning => palette.warning_surface,
        Tone::Danger => palette.danger_surface,
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
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (fill, text) = match self.tone {
            Tone::Neutral => (tokens().palette.ctl, tokens().palette.text_2),
            tone => (tone_surface(tone), tone_color(tone)),
        };
        text_style(div(), TextStyle::Caption)
            .flex_none()
            .px(px(PAD_X))
            .py(px(PAD_Y))
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
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .size(px(DOT))
            .flex_none()
            .rounded_full()
            .bg(tone_color(self.tone))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_constants_match_spec() {
        assert_eq!((PAD_X, PAD_Y, DOT), (6., 2., 6.));
        assert_eq!(TextStyle::Caption.size(), 10.);
    }

    #[test]
    fn tones_map_to_tokens() {
        let p = tokens().palette;
        assert_eq!(tone_color(Tone::Info), p.accent);
        assert_eq!(tone_color(Tone::Danger), p.danger);
        assert_eq!(tone_surface(Tone::Warning), p.warning_surface);
        assert_eq!(tone_surface(Tone::Neutral), p.ctl);
    }
}
