//! `Toggle` -- `apps/desktop/src/components/Toggle.tsx`, a Kobalte `Switch`.
//!
//! The `.cap-toggle` / `.cap-toggle-thumb` class names exist in the Solid app
//! *solely* so the macOS material layer can re-skin the track, which is why
//! this has one constructor per surface rather than one palette.

use gpui::{
    App, ClickEvent, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Toggled, Window, div,
    prelude::FluentBuilder, px,
};

use studio_theme::Theme;

use super::interact::focusable;

/// `cva` sizes. `lg` is `w-14 h-7 p-0.75` with a `size-6` thumb; no window
/// here uses it yet, but the scale is transcribed whole for the same reason
/// `Theme` carries the unused Radix steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleSize {
    /// `w-9 h-5 p-0.5`, `size-4` thumb.
    Sm,
    /// `w-11 h-6 p-0.5`, `size-5` thumb -- the `cva` default.
    Md,
    /// `w-14 h-7 p-0.75`, `size-6` thumb.
    Lg,
}

impl ToggleSize {
    fn track(self) -> (Pixels, Pixels, Pixels) {
        match self {
            ToggleSize::Sm => (px(34.), px(20.), px(2.)),
            ToggleSize::Md => (px(44.), px(24.), px(2.)),
            ToggleSize::Lg => (px(56.), px(28.), px(3.)),
        }
    }

    fn thumb(self) -> Pixels {
        match self {
            ToggleSize::Sm => px(16.),
            ToggleSize::Md => px(20.),
            ToggleSize::Lg => px(24.),
        }
    }
}

#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    checked: bool,
    size: ToggleSize,
    /// `bg-gray-6`, or whatever the surface remaps it to.
    off_fill: Hsla,
    /// `data-checked:bg-blue-500`, or `--macos-settings-accent`.
    on_fill: Hsla,
    /// `bg-white` -- the thumb is white on every surface.
    thumb: Hsla,
    disabled: bool,
    /// The accessible name.
    label: Option<SharedString>,
    on_click: Option<crate::kit::button::ClickHandler>,
}

/// The value after an activation, or `None` when disabled.
pub fn toggled(checked: bool, disabled: bool) -> Option<bool> {
    (!disabled).then_some(!checked)
}

impl Toggle {
    /// The editor track: `ed-ctl-active` off, `ed-accent` on, white knob.
    pub fn plain(theme: &Theme, id: impl Into<ElementId>, checked: bool) -> Self {
        Self {
            id: id.into(),
            checked,
            size: ToggleSize::Sm,
            off_fill: Hsla::from(theme.editor.ctl_active),
            on_fill: Hsla::from(theme.editor.accent),
            thumb: gpui::white(),
            disabled: false,
            label: None,
            on_click: None,
        }
    }

    /// The settings window: `[data-macos-native-material="settings"]
    /// .cap-toggle { background: var(--macos-settings-control-fill) }`, checked
    /// `var(--macos-settings-accent)`. The `inset 0 1px 2px rgba(0,0,0,.16)`
    /// bevel the same rule adds has no gpui equivalent (README deviation).
    pub fn settings(theme: &Theme, id: impl Into<ElementId>, checked: bool) -> Self {
        Self {
            size: ToggleSize::Sm,
            off_fill: Hsla::from(theme.gray_6),
            on_fill: gpui::rgb(Theme::SETTINGS_ACCENT).into(),
            ..Self::plain(theme, id, checked)
        }
    }

    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, theme: &Theme, disabled: bool) -> Self {
        self.disabled = disabled;
        if disabled && !self.checked {
            self.off_fill = Hsla::from(theme.editor.ctl);
        }
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Called with the new value when a click, Enter or Space flips the toggle.
    pub fn on_change(self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        let next = toggled(self.checked, self.disabled);
        self.on_click(move |_, window, cx| {
            if let Some(value) = next {
                handler(value, window, cx);
            }
        })
    }

    /// The name a screen reader announces for the switch.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl RenderOnce for Toggle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::for_window(window, cx);
        let Toggle {
            id,
            checked,
            size,
            off_fill,
            on_fill,
            thumb,
            disabled,
            label,
            on_click,
        } = self;
        let (width, height, padding) = size.track();

        focusable(div().id(id), &theme, disabled)
            .role(Role::Switch)
            .aria_toggled(if checked {
                Toggled::True
            } else {
                Toggled::False
            })
            .when_some(label, |this, label| this.aria_label(label))
            .w(width)
            .h(height)
            .p(padding)
            .flex()
            .flex_row()
            .flex_shrink_0()
            .rounded_full()
            // `data-checked:translate-x-[calc(100%)]` on the thumb -- the same
            // end position a flex justification reaches, without a transform.
            .when(checked, |this| this.justify_end())
            .when(!disabled, |this| this.cursor_pointer())
            .bg(if checked { on_fill } else { off_fill })
            .child(div().size(size.thumb()).rounded_full().bg(thumb))
            .when_some(on_click.filter(|_| !disabled), |this, handler| {
                this.on_click(move |event, window, cx| handler(event, window, cx))
            })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, Render, TestAppContext};

    use super::*;
    use crate::kit::test_support::{Log, log, press, tab};

    struct Switches {
        changes: Log<(&'static str, bool)>,
    }

    impl Render for Switches {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::for_window(window, cx);
            let change = |name: &'static str| {
                let changes = self.changes.clone();
                move |value: bool, _: &mut Window, _: &mut App| {
                    changes.borrow_mut().push((name, value))
                }
            };
            div()
                .child(
                    Toggle::plain(&theme, "locked", false)
                        .disabled(&theme, true)
                        .label("Locked")
                        .on_change(change("locked")),
                )
                .child(
                    Toggle::plain(&theme, "loop", true)
                        .label("Loop")
                        .on_change(change("loop")),
                )
        }
    }

    #[gpui::test]
    fn space_flips_the_focused_switch_and_tab_skips_a_disabled_one(cx: &mut TestAppContext) {
        let changes = log();
        let shared = changes.clone();
        let (_, cx) = cx.add_window_view(|_, _| Switches { changes: shared });
        tab(cx);
        press(cx, "space");
        assert_eq!(*changes.borrow(), [("loop", false)]);
    }

    #[test]
    fn activation_flips_unless_disabled() {
        assert_eq!(toggled(false, false), Some(true));
        assert_eq!(toggled(true, false), Some(false));
        assert_eq!(toggled(true, true), None);
    }
}
