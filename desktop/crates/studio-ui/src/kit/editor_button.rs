//! `EditorButton` -- the editor's dominant small button
//! (`routes/editor/ui.tsx:317-424`): the header's undo/redo/delete, the
//! player toolbar's triggers, the background section's Reset and Import
//! actions, and every selection panel's Done/Delete pair.
//!
//! Visually it is the editor's ghost button: 28px tall, transparent at rest
//! with a `text-2` label, `ctl-hover` + `text-1` on hover and `ctl-active`
//! while pressed. `danger` keeps its solid red pressed state, and `text`
//! promotes the resting label to `text-1` for the player toolbar's triggers.
//!
//! fframes' own buttons are the same box with a resting fill: [`filled`]
//! (`ctl`), [`accent`] (the main action) and [`destructive`] (a soft red).
//!
//! [`filled`]: EditorButton::filled
//! [`accent`]: EditorButton::accent
//! [`destructive`]: EditorButton::destructive
use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px, svg,
};

use studio_theme::{Theme, mix};

use super::{
    ClickHandler, Tooltip,
    interact::{BoundsHandler, DISABLED_OPACITY, bounds_probe, focusable},
    menu::OpenHandler,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorButtonVariant {
    Primary,
    Danger,
}

#[derive(IntoElement)]
pub struct EditorButton {
    id: ElementId,
    label: Option<SharedString>,
    left_icon: Option<SharedString>,
    right_icon: Option<SharedString>,
    /// `rightIconEnd` -- pushes the right icon to `ml-auto` and the row to
    /// `justify-between`, which is how the select-style triggers park their
    /// chevron.
    right_icon_end: bool,
    icon_size: Pixels,
    right_icon_size: Pixels,
    width: Option<Pixels>,
    /// `h-7` by default; the 3D panel's group-row actions are `h-6`.
    height: Pixels,
    full_width: bool,
    padding_x: Pixels,
    text_size: Pixels,
    variant: EditorButtonVariant,
    disabled: bool,
    /// `data-pressed` / `data-expanded`.
    pressed: bool,
    text: Hsla,
    hover_text: Hsla,
    disabled_text: Hsla,
    /// A fill at rest (and while disabled); the ghost has none.
    rest_bg: Option<Hsla>,
    hover_bg: Hsla,
    active_bg: Hsla,
    pressed_bg: Hsla,
    pressed_text: Hsla,
    chevron: Hsla,
    tooltip: Option<(Theme, SharedString)>,
    on_click: Option<ClickHandler>,
    on_open: Option<OpenHandler>,
    on_bounds: Option<BoundsHandler>,
}

impl EditorButton {
    pub fn plain(theme: &Theme, id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            left_icon: None,
            right_icon: None,
            right_icon_end: false,
            icon_size: px(16.),
            right_icon_size: px(10.),
            width: None,
            height: px(28.),
            full_width: false,
            padding_x: px(7.),
            text_size: px(13.),
            variant: EditorButtonVariant::Primary,
            disabled: false,
            pressed: false,
            text: Hsla::from(theme.editor.text_2),
            hover_text: Hsla::from(theme.editor.text_1),
            disabled_text: Hsla::from(theme.editor.text_3),
            rest_bg: None,
            hover_bg: Hsla::from(theme.editor.ctl_hover),
            active_bg: Hsla::from(theme.editor.ctl_active),
            pressed_bg: Hsla::from(theme.editor.ctl_hover),
            pressed_text: Hsla::from(theme.editor.text_1),
            chevron: Hsla::from(theme.editor.text_3),
            tooltip: None,
            on_click: None,
            on_open: None,
            on_bounds: None,
        }
    }

    /// A neutral control with a subtle `ctl` fill that it keeps while disabled.
    pub fn filled(mut self, theme: &Theme) -> Self {
        self.rest_bg = Some(Hsla::from(theme.editor.ctl));
        self
    }

    /// The main action of a view: white on the accent, the lighter accent on
    /// hover. Selected (`pressed`) looks like rest.
    pub fn accent(mut self, theme: &Theme) -> Self {
        let accent = Hsla::from(theme.editor.accent);
        let text = Hsla::from(theme.on_accent());
        self.rest_bg = Some(accent);
        self.hover_bg = Hsla::from(theme.editor.accent_2);
        self.active_bg = accent;
        self.pressed_bg = accent;
        self.text = text;
        self.hover_text = text;
        self.pressed_text = text;
        self.disabled_text = text;
        self.chevron = text;
        self
    }

    /// A soft red: red text on the danger surface, deepening on hover and press.
    pub fn destructive(mut self, theme: &Theme) -> Self {
        let text = Hsla::from(theme.red_11);
        self.rest_bg = Some(Hsla::from(theme.red_3));
        self.hover_bg = Hsla::from(mix(theme.red_3, theme.red_11, 0.12));
        self.active_bg = Hsla::from(mix(theme.red_3, theme.red_11, 0.22));
        self.text = text;
        self.hover_text = text;
        self.disabled_text = text;
        self
    }

    /// Stretches across its parent.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Reports the painted bounds, in window coordinates, on every prepaint.
    pub fn on_bounds(mut self, report: impl Fn(gpui::Bounds<Pixels>, &mut App) + 'static) -> Self {
        self.on_bounds = Some(Rc::new(report));
        self
    }

    /// `variant="danger"`: the pressed/expanded state is a solid `bg-red-300`
    /// with `text-gray-1` on it, not a wash.
    pub fn danger(mut self, theme: &Theme) -> Self {
        self.variant = EditorButtonVariant::Danger;
        self.pressed_bg = Hsla::from(theme.red_300);
        self.pressed_text = Hsla::from(theme.gray_1);
        self
    }

    /// The "text" ghost: the resting label is `text-1` rather than `text-2`.
    pub fn text(mut self, theme: &Theme) -> Self {
        self.text = Hsla::from(theme.editor.text_1);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn left_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.left_icon = Some(icon.into());
        self
    }

    pub fn right_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.right_icon = Some(icon.into());
        self
    }

    pub fn right_icon_end(mut self, end: bool) -> Self {
        self.right_icon_end = end;
        self
    }

    pub fn icon_size(mut self, size: Pixels) -> Self {
        self.icon_size = size;
        self
    }

    /// A shorter button, for a control sitting on a group's own label row.
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    pub fn padding_x(mut self, padding: Pixels) -> Self {
        self.padding_x = padding;
        self
    }

    pub fn text_size(mut self, size: Pixels) -> Self {
        self.text_size = size;
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn tooltip(mut self, theme: &Theme, label: impl Into<SharedString>) -> Self {
        self.tooltip = Some((*theme, label.into()));
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&gpui::Bounds<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for EditorButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::for_window(window, cx);
        let EditorButton {
            id,
            label,
            left_icon,
            right_icon,
            right_icon_end,
            icon_size,
            right_icon_size,
            width,
            height,
            full_width,
            padding_x,
            text_size,
            variant: _,
            disabled,
            pressed,
            text,
            hover_text,
            disabled_text,
            rest_bg,
            hover_bg,
            active_bg,
            pressed_bg,
            pressed_text,
            chevron,
            tooltip,
            on_click,
            on_open,
            on_bounds,
        } = self;

        let foreground = if disabled {
            disabled_text
        } else if pressed {
            pressed_text
        } else {
            text
        };
        // gpui svgs do not inherit a parent's text colour, so the icon follows
        // the label through a group rather than through the cascade.
        let group: SharedString = match &id {
            ElementId::Name(name) => SharedString::from(format!("eb-{name}")),
            other => SharedString::from(format!("eb-{other:?}")),
        };
        let icon_hover = (!disabled && !pressed).then_some(hover_text);
        let name = label
            .clone()
            .or_else(|| tooltip.as_ref().map(|(_, text)| text.clone()));

        focusable(div().id(id), &theme, disabled)
            .role(Role::Button)
            .when_some(name, |this, name| this.aria_label(name))
            .group(group.clone())
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .px(padding_x)
            .gap(px(6.))
            .h(height)
            .min_w(height)
            .rounded(px(7.))
            .flex_shrink_0()
            .when(right_icon_end, |this| this.justify_between())
            .when_some(width, |this, width| this.w(width))
            .when(full_width, |this| this.w_full())
            .text_size(text_size)
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(foreground)
            .when(disabled, |this| this.opacity(DISABLED_OPACITY))
            .when_some(rest_bg, |this, bg| this.bg(bg))
            .when(pressed, |this| this.bg(pressed_bg))
            .when(!disabled && !pressed, |this| {
                this.cursor_pointer()
                    .hover(move |this| this.bg(hover_bg).text_color(hover_text))
                    .active(move |this| this.bg(active_bg))
            })
            .children(left_icon.map(|icon| {
                let group = group.clone();
                svg()
                    .path(icon)
                    .size(icon_size)
                    .flex_shrink_0()
                    .text_color(foreground)
                    .when_some(icon_hover, |this, color| {
                        this.group_hover(group, move |style| style.text_color(color))
                    })
            }))
            .children(label.map(|label| div().truncate().child(label)))
            .children(right_icon.map(|icon| {
                svg()
                    .path(icon)
                    .size(right_icon_size)
                    .flex_shrink_0()
                    .when(right_icon_end, |this| this.ml_auto())
                    .text_color(if disabled { disabled_text } else { chevron })
            }))
            .when_some(tooltip, |this, (theme, label)| {
                this.tooltip_show_delay(crate::kit::TOOLTIP_SHOW_DELAY)
                    .tooltip(move |_window, cx| Tooltip::new(&theme, label.clone()).view(cx))
            })
            .when_some(on_click.filter(|_| !disabled), |this, handler| {
                this.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when_some(on_open.filter(|_| !disabled), crate::kit::Menu::trigger)
            .children(on_bounds.map(bounds_probe))
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{Bounds, Context, Render, TestAppContext, VisualTestContext};
    use studio_theme::mix;

    use super::*;
    use crate::kit::test_support::{Log, log, press, tab};

    type Seen = Rc<Cell<Option<Bounds<Pixels>>>>;

    /// A disabled button, an 80px one reporting its bounds, and a third, each
    /// logging its clicks.
    struct Row {
        clicks: Log<&'static str>,
        seen: Seen,
    }

    impl Render for Row {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::for_window(window, cx);
            let click = |name: &'static str| {
                let clicks = self.clicks.clone();
                move |_: &ClickEvent, _: &mut Window, _: &mut App| clicks.borrow_mut().push(name)
            };
            let seen = self.seen.clone();
            div()
                .flex()
                .w(px(400.))
                .child(
                    EditorButton::plain(&theme, "off")
                        .label("Off")
                        .disabled(true)
                        .on_click(click("off")),
                )
                .child(
                    EditorButton::plain(&theme, "on")
                        .label("On")
                        .width(px(80.))
                        .on_bounds(move |bounds, _| seen.set(Some(bounds)))
                        .on_click(click("on")),
                )
                .child(
                    EditorButton::plain(&theme, "go")
                        .label("Go")
                        .on_click(click("go")),
                )
        }
    }

    fn row(cx: &mut TestAppContext) -> (Log<&'static str>, Seen, &mut VisualTestContext) {
        let clicks = log();
        let seen = Seen::default();
        let (shared, bounds) = (clicks.clone(), seen.clone());
        let (_, cx) = cx.add_window_view(|_, _| Row {
            clicks: shared,
            seen: bounds,
        });
        cx.run_until_parked();
        (clicks, seen, cx)
    }

    #[gpui::test]
    fn tab_skips_a_disabled_button_and_enter_or_space_activates(cx: &mut TestAppContext) {
        let (clicks, _, cx) = row(cx);
        tab(cx);
        press(cx, "enter");
        tab(cx);
        press(cx, "space");
        assert_eq!(*clicks.borrow(), ["on", "go"]);
    }

    #[gpui::test]
    fn the_reported_bounds_are_the_buttons_own(cx: &mut TestAppContext) {
        let (_, seen, _) = row(cx);
        let bounds = seen.get().expect("painted");
        assert_eq!(
            (f32::from(bounds.size.width), f32::from(bounds.size.height)),
            (80., 28.)
        );
    }

    #[test]
    fn fills_reproduce_the_editor_variants() {
        let theme = Theme::light();
        let hsla = Hsla::from;

        let filled = EditorButton::plain(&theme, "a").filled(&theme);
        assert_eq!(filled.rest_bg, Some(hsla(theme.editor.ctl)));
        assert_eq!(filled.hover_bg, hsla(theme.editor.ctl_hover));

        let accent = EditorButton::plain(&theme, "b").accent(&theme);
        assert_eq!(accent.rest_bg, Some(hsla(theme.editor.accent)));
        assert_eq!(accent.hover_bg, hsla(theme.editor.accent_2));
        assert_eq!(accent.active_bg, hsla(theme.editor.accent));
        assert_eq!(
            (accent.text, accent.hover_text),
            (hsla(theme.on_accent()), hsla(theme.on_accent()))
        );

        let destructive = EditorButton::plain(&theme, "c").destructive(&theme);
        assert_eq!(destructive.rest_bg, Some(hsla(theme.red_3)));
        assert_eq!(destructive.text, hsla(theme.red_11));
        assert_eq!(destructive.hover_text, hsla(theme.red_11));
        assert_eq!(
            destructive.hover_bg,
            hsla(mix(theme.red_3, theme.red_11, 0.12))
        );
        assert_eq!(
            destructive.active_bg,
            hsla(mix(theme.red_3, theme.red_11, 0.22))
        );
    }
}
