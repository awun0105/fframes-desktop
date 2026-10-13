//! Inline notice: a message with a tone bar, optional actions and details.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Rgba,
    Role, SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use super::badge::{Tone, tone_color, tone_surface};
use super::button::{Button, IconButton, IconButtonSize};
use super::collapsible::Collapsible;
use super::icon::{Icon, icon};
use super::interact::Handler;
use crate::design_system::{TextStyle, text_style, tokens};

pub const BAR_W: f32 = 3.;
pub const PADDING: f32 = 12.;
pub const GAP: f32 = 8.;
/// Tallest the expanded details grow before they scroll, so a long error
/// never pushes the rest of the window down.
pub const DETAILS_MAX_H: f32 = 160.;

/// Glyph for a tone.
pub fn tone_icon(tone: Tone) -> Icon {
    match tone {
        Tone::Neutral | Tone::Info => Icon::Info,
        Tone::Success => Icon::CircleCheck,
        Tone::Warning | Tone::Danger => Icon::AlertTriangle,
    }
}

/// Fill of the notice card: the status surfaces, the plain card otherwise.
pub fn notice_surface(tone: Tone) -> Rgba {
    match tone {
        Tone::Neutral | Tone::Info => tokens().palette.card,
        tone => tone_surface(tone),
    }
}

#[derive(IntoElement)]
pub struct Notice {
    id: ElementId,
    tone: Tone,
    title: SharedString,
    body: Option<SharedString>,
    details: Option<SharedString>,
    actions: Vec<AnyElement>,
    on_dismiss: Option<Handler>,
}

impl Notice {
    pub fn new(id: impl Into<ElementId>, tone: Tone, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            tone,
            title: title.into(),
            body: None,
            details: None,
            actions: Vec::new(),
            on_dismiss: None,
        }
    }

    pub fn body(mut self, text: impl Into<SharedString>) -> Self {
        self.body = Some(text.into());
        self
    }

    /// Longer text shown under a collapsible "Details" header.
    pub fn details(mut self, text: impl Into<SharedString>) -> Self {
        self.details = Some(text.into());
        self
    }

    pub fn action(mut self, button: Button) -> Self {
        self.actions.push(button.into_any_element());
        self
    }

    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Notice {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = tokens().palette;
        let metrics = tokens().metrics;
        let accent = tone_color(self.tone);

        let details = self.details.map(|text| {
            let open =
                window.use_keyed_state((self.id.clone(), "details".to_string()), cx, |_, _| false);
            let is_open = *open.read(cx);
            Collapsible::new((self.id.clone(), "collapse".to_string()), is_open)
                .header(
                    text_style(div(), TextStyle::Label)
                        .text_color(palette.text_2)
                        .child("Details"),
                )
                .content(
                    text_style(
                        div().id((self.id.clone(), "details-text".to_string())),
                        TextStyle::Small,
                    )
                    .mt(px(metrics.space_1))
                    .max_h(px(DETAILS_MAX_H))
                    .overflow_y_scroll()
                    .text_color(palette.text_2)
                    .child(text),
                )
                .on_toggle(move |next, _, cx| {
                    open.update(cx, |value, cx| {
                        *value = next;
                        cx.notify();
                    });
                })
        });

        let dismiss = self.on_dismiss.map(|handler| {
            IconButton::new((self.id.clone(), "dismiss".to_string()), Icon::X)
                .size(IconButtonSize::Small)
                .label("Dismiss")
                .on_click(move |window, cx| handler(window, cx))
        });

        let body = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap(px(metrics.space_1))
            .child(
                text_style(div(), TextStyle::BodyStrong)
                    .text_color(palette.text_1)
                    .child(self.title),
            )
            .children(self.body.map(|body| {
                text_style(div(), TextStyle::Body)
                    .text_color(palette.text_2)
                    .child(body)
            }))
            .children(details)
            .children((!self.actions.is_empty()).then(|| {
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(GAP))
                    .pt(px(metrics.space_1))
                    .children(self.actions)
            }));

        div()
            .id(self.id)
            .role(Role::Alert)
            .flex()
            .w_full()
            .overflow_hidden()
            .rounded(px(metrics.radius_card))
            .border_1()
            .border_color(palette.line)
            .bg(notice_surface(self.tone))
            .child(div().w(px(BAR_W)).flex_none().bg(accent))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(GAP))
                    .p(px(PADDING))
                    .child(icon(tone_icon(self.tone), metrics.icon_md, accent))
                    .child(body)
                    .children(dismiss),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_constants_match_spec() {
        assert_eq!(tokens().metrics.radius_card, 12.);
        assert_eq!((BAR_W, PADDING, GAP), (3., 12., 8.));
        assert_eq!(DETAILS_MAX_H, 160.);
    }

    #[test]
    fn surfaces_follow_tone() {
        let p = tokens().palette;
        assert_eq!(notice_surface(Tone::Info), p.card);
        assert_eq!(notice_surface(Tone::Success), p.success_surface);
        assert_eq!(notice_surface(Tone::Warning), p.warning_surface);
        assert_eq!(notice_surface(Tone::Danger), p.danger_surface);
    }

    #[test]
    fn tones_have_distinct_icons() {
        assert_eq!(tone_icon(Tone::Info), Icon::Info);
        assert_eq!(tone_icon(Tone::Success), Icon::CircleCheck);
        assert_eq!(tone_icon(Tone::Danger), Icon::AlertTriangle);
    }
}
