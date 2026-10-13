//! `Button` and `IconButton`.

use gpui::{
    App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Rgba, Role,
    SharedString, Stateful, StatefulInteractiveElement, Styled, Window, div, px,
};

use super::icon::{Icon, icon};
use super::interact::{CONTROL_GROUP, Handler, focusable, mix, on_activate, with_tooltip};
use super::probe::{BoundsProbe, with_probe};
use crate::design_system::{TextStyle, text_style, tokens};

/// Horizontal padding of a labelled button.
pub const BUTTON_PAD_X: f32 = 7.;
/// Gap between icon and label.
pub const BUTTON_GAP: f32 = 6.;
/// Icon-only box sizes.
pub const ICON_BUTTON_SMALL: f32 = 24.;
pub const ICON_BUTTON_MEDIUM: f32 = 28.;
pub const ICON_BUTTON_LARGE: f32 = 32.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Neutral control with a subtle fill.
    Editor,
    /// The main action of a view.
    Primary,
    Destructive,
    /// No fill until hovered.
    Ghost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Rest,
    Hover,
    Pressed,
    Selected,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonColors {
    /// `None`: no fill.
    pub background: Option<Rgba>,
    pub foreground: Rgba,
}

/// Fill and text colour of `variant` in `state`, from the current tokens.
pub fn button_colors(variant: ButtonVariant, state: ButtonState) -> ButtonColors {
    let p = tokens().palette;
    let colors = |background: Option<Rgba>, foreground: Rgba| ButtonColors {
        background,
        foreground,
    };
    match (variant, state) {
        (ButtonVariant::Primary, ButtonState::Hover) => colors(Some(p.accent_2), p.on_accent),
        (ButtonVariant::Primary, _) => colors(Some(p.accent), p.on_accent),
        (ButtonVariant::Destructive, ButtonState::Hover) => {
            colors(Some(mix(p.danger_surface, p.danger, 0.12)), p.danger)
        }
        (ButtonVariant::Destructive, ButtonState::Pressed) => {
            colors(Some(mix(p.danger_surface, p.danger, 0.22)), p.danger)
        }
        (ButtonVariant::Destructive, _) => colors(Some(p.danger_surface), p.danger),
        (_, ButtonState::Selected) => colors(Some(p.ctl_hover), p.text_1),
        (_, ButtonState::Hover) => colors(Some(p.ctl_hover), p.text_1),
        (_, ButtonState::Pressed) => colors(Some(p.ctl_active), p.text_1),
        (ButtonVariant::Editor, ButtonState::Disabled) => colors(Some(p.ctl), p.text_3),
        (ButtonVariant::Ghost, ButtonState::Disabled) => colors(None, p.text_3),
        (ButtonVariant::Editor, ButtonState::Rest) => colors(Some(p.ctl), p.text_2),
        (ButtonVariant::Ghost, ButtonState::Rest) => colors(None, p.text_2),
    }
}

fn rest_state(selected: bool, disabled: bool) -> ButtonState {
    if disabled {
        ButtonState::Disabled
    } else if selected {
        ButtonState::Selected
    } else {
        ButtonState::Rest
    }
}

/// Paints the button look (fill, radius, text, focus ring, hover and pressed
/// states) on `element`. Shared by [`Button`], [`IconButton`] and
/// `design_system::button`, so every button resolves colours the same way.
pub fn style_button(
    element: Stateful<Div>,
    variant: ButtonVariant,
    selected: bool,
    disabled: bool,
) -> Stateful<Div> {
    let metrics = tokens().metrics;
    let rest = button_colors(variant, rest_state(selected, disabled));
    let mut element = focusable(
        text_style(element, TextStyle::Body)
            .font_weight(gpui::FontWeight::MEDIUM)
            .group(CONTROL_GROUP)
            .flex()
            .items_center()
            .justify_center()
            .gap(px(BUTTON_GAP))
            .h(px(metrics.control_h))
            .min_w(px(metrics.control_h))
            .px(px(BUTTON_PAD_X))
            .rounded(px(metrics.radius_control))
            .text_color(rest.foreground),
        disabled,
    );
    if let Some(background) = rest.background {
        element = element.bg(background);
    }
    if disabled || (selected && variant != ButtonVariant::Primary) {
        return element;
    }
    let hover = button_colors(variant, ButtonState::Hover);
    let pressed = button_colors(variant, ButtonState::Pressed);
    element
        .hover(move |style| {
            let style = style.text_color(hover.foreground);
            match hover.background {
                Some(background) => style.bg(background),
                None => style,
            }
        })
        .active(move |style| match pressed.background {
            Some(background) => style.bg(background),
            None => style,
        })
}

/// A glyph that follows the hover colour of its button.
fn glyph(icon_kind: Icon, size: f32, variant: ButtonVariant, state: ButtonState) -> gpui::Svg {
    let rest = button_colors(variant, state);
    let hover = button_colors(variant, ButtonState::Hover);
    let glyph = icon(icon_kind, size, rest.foreground);
    if state == ButtonState::Rest {
        glyph.group_hover(CONTROL_GROUP, move |style| {
            style.text_color(hover.foreground)
        })
    } else {
        glyph
    }
}

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    icon: Option<Icon>,
    trailing_icon: Option<Icon>,
    selected: bool,
    disabled: bool,
    tooltip: Option<SharedString>,
    probe: Option<BoundsProbe>,
    on_click: Option<Handler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Editor,
            icon: None,
            trailing_icon: None,
            selected: false,
            disabled: false,
            tooltip: None,
            probe: None,
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn trailing_icon(mut self, icon: Icon) -> Self {
        self.trailing_icon = Some(icon);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Reports the button's bounds for native qualification.
    pub fn probe(mut self, probe: Option<BoundsProbe>) -> Self {
        self.probe = probe;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let icon_size = tokens().metrics.icon_md;
        let state = rest_state(self.selected, self.disabled);
        let element = div()
            .id(self.id)
            .relative()
            .role(Role::Button)
            .aria_label(self.label.clone());
        let element = style_button(element, self.variant, self.selected, self.disabled);
        let element = on_activate(element, self.disabled, self.on_click);
        let element = with_tooltip(element, self.tooltip)
            .children(
                self.icon
                    .map(|kind| glyph(kind, icon_size, self.variant, state)),
            )
            .child(self.label)
            .children(
                self.trailing_icon
                    .map(|kind| glyph(kind, icon_size, self.variant, state)),
            );
        with_probe(element, self.probe)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconButtonSize {
    Small,
    Medium,
    Large,
}

impl IconButtonSize {
    pub const fn box_size(self) -> f32 {
        match self {
            Self::Small => ICON_BUTTON_SMALL,
            Self::Medium => ICON_BUTTON_MEDIUM,
            Self::Large => ICON_BUTTON_LARGE,
        }
    }

    /// Glyph size: 14 in the small box, 16 otherwise.
    pub fn icon_size(self) -> f32 {
        let metrics = tokens().metrics;
        match self {
            Self::Small => metrics.icon_sm,
            Self::Medium | Self::Large => metrics.icon_md,
        }
    }
}

#[derive(IntoElement)]
pub struct IconButton {
    id: ElementId,
    glyph: Icon,
    size: IconButtonSize,
    variant: ButtonVariant,
    round: bool,
    label: Option<SharedString>,
    selected: bool,
    disabled: bool,
    probe: Option<BoundsProbe>,
    on_click: Option<Handler>,
}

impl IconButton {
    pub fn new(id: impl Into<ElementId>, glyph: Icon) -> Self {
        Self {
            id: id.into(),
            glyph,
            size: IconButtonSize::Medium,
            variant: ButtonVariant::Ghost,
            round: false,
            label: None,
            selected: false,
            disabled: false,
            probe: None,
            on_click: None,
        }
    }

    pub fn size(mut self, size: IconButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn round(mut self, round: bool) -> Self {
        self.round = round;
        self
    }

    /// Accessible name, also shown as the tooltip.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn probe(mut self, probe: Option<BoundsProbe>) -> Self {
        self.probe = probe;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }
}

impl RenderOnce for IconButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let side = px(self.size.box_size());
        let state = rest_state(self.selected, self.disabled);
        let mut element = div().id(self.id).relative().role(Role::Button);
        if let Some(label) = &self.label {
            element = element.aria_label(label.clone());
        }
        let mut element = style_button(element, self.variant, self.selected, self.disabled)
            .w(side)
            .h(side)
            .px(px(0.));
        if self.round {
            element = element.rounded_full();
        }
        let element = on_activate(element, self.disabled, self.on_click);
        let element = with_tooltip(element, self.label).child(glyph(
            self.glyph,
            self.size.icon_size(),
            self.variant,
            state,
        ));
        with_probe(element, self.probe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_colours_come_from_tokens() {
        let p = tokens().palette;
        let editor = |state| button_colors(ButtonVariant::Editor, state);
        assert_eq!(editor(ButtonState::Rest).foreground, p.text_2);
        assert_eq!(editor(ButtonState::Hover).foreground, p.text_1);
        assert_eq!(editor(ButtonState::Hover).background, Some(p.ctl_hover));
        assert_eq!(editor(ButtonState::Pressed).background, Some(p.ctl_active));
        assert_eq!(editor(ButtonState::Selected).background, Some(p.ctl_hover));
        assert_eq!(editor(ButtonState::Selected).foreground, p.text_1);
        assert_eq!(editor(ButtonState::Disabled).foreground, p.text_3);

        let primary = |state| button_colors(ButtonVariant::Primary, state);
        assert_eq!(primary(ButtonState::Rest).background, Some(p.accent));
        assert_eq!(primary(ButtonState::Rest).foreground, p.on_accent);
        assert_eq!(primary(ButtonState::Hover).background, Some(p.accent_2));

        let danger = button_colors(ButtonVariant::Destructive, ButtonState::Rest);
        assert_eq!(danger.background, Some(p.danger_surface));
        assert_eq!(danger.foreground, p.danger);

        let ghost = |state| button_colors(ButtonVariant::Ghost, state);
        assert_eq!(ghost(ButtonState::Rest).background, None);
        assert_eq!(ghost(ButtonState::Hover).background, Some(p.ctl_hover));
    }

    #[test]
    fn probe_is_stored_for_forwarding() {
        let button = Button::new("probe", "Probe").probe(Some(BoundsProbe::new(|_, _| {})));
        assert!(button.probe.is_some());
        assert!(Button::new("probe", "Probe").probe.is_none());
        let icon = IconButton::new("probe", Icon::Play).probe(Some(BoundsProbe::new(|_, _| {})));
        assert!(icon.probe.is_some());
    }

    #[test]
    fn disabled_state_wins_over_selected() {
        assert_eq!(rest_state(true, true), ButtonState::Disabled);
        assert_eq!(rest_state(true, false), ButtonState::Selected);
        assert_eq!(rest_state(false, false), ButtonState::Rest);
    }

    #[test]
    fn size_constants_match_spec() {
        let m = tokens().metrics;
        assert_eq!((m.control_h, m.radius_control, m.icon_md), (28., 7., 16.));
        assert_eq!((BUTTON_PAD_X, BUTTON_GAP), (7., 6.));
        assert_eq!(
            [
                IconButtonSize::Small.box_size(),
                IconButtonSize::Medium.box_size(),
                IconButtonSize::Large.box_size()
            ],
            [24., 28., 32.]
        );
        assert_eq!(IconButtonSize::Small.icon_size(), 14.);
        assert_eq!(IconButtonSize::Large.icon_size(), 16.);
    }
}
