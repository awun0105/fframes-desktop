//! The panel's small building blocks: key routing in fields, buttons, segments, tabs and text lines.

use super::*;

impl ConversationPanel {
    /// Key routing for one text field. The composition state is the field's own, read at
    /// the moment of the key: the prompt and every setup field share this one path.
    pub(super) fn key_in_field(
        &mut self,
        field: &Entity<TextInputState>,
        event: &gpui::KeyDownEvent,
        submit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let composing = field.read(cx).is_composing();
        self.key_in_input(event, composing, submit, window, cx);
    }

    pub(super) fn key_in_input(
        &mut self,
        event: &gpui::KeyDownEvent,
        composing: bool,
        submit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = &event.keystroke.modifiers;
        let mods = KeyMods {
            shift: modifiers.shift,
            alt: modifiers.alt,
            secondary: modifiers.control || modifiers.platform,
        };
        match route_input_key(event.keystroke.key.as_str(), mods, composing) {
            InputKey::Submit => {
                if submit {
                    self.submit(cx);
                }
                cx.stop_propagation();
            }
            InputKey::FocusNext => {
                window.focus_next(cx);
                cx.stop_propagation();
            }
            InputKey::FocusPrev => {
                window.focus_prev(cx);
                cx.stop_propagation();
            }
            InputKey::Leave => {
                cx.emit(PanelEvent::Leave);
                cx.stop_propagation();
            }
            InputKey::Composition => cx.stop_propagation(),
            // Typing, IME text, Space and the arrows belong to the input.
            InputKey::Text => (),
        }
    }

    // ---- building blocks -----------------------------------------------------------------------

    pub(super) fn activate(
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl Fn(&mut Window, &mut gpui::App) + 'static {
        let entity = cx.entity();
        move |window, app| {
            entity.update(app, |this, cx| action(this, window, cx));
        }
    }

    /// Reports the bounds of the buttons and tabs native qualification clicks.
    pub(super) fn bounds_probe(id: &ElementId, cx: &mut Context<Self>) -> Option<BoundsProbe> {
        let name = probed_button(id)?.to_owned();
        let entity = cx.entity();
        Some(BoundsProbe::new(move |bounds, cx| {
            entity.update(cx, |panel, _| {
                panel.button_bounds.insert(name.clone(), bounds);
            });
        }))
    }

    /// [`Self::activate`] in the shape of a click handler.
    pub(super) fn click(
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static {
        let activate = Self::activate(cx, action);
        move |_, window, app| activate(window, app)
    }

    pub(super) fn action_button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        enabled: bool,
        tone: Tone,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> EditorButton {
        let theme = self.theme;
        let id: ElementId = id.into();
        let probe = Self::bounds_probe(&id, cx);
        let button = EditorButton::plain(&theme, id)
            .label(label)
            .disabled(!enabled)
            .on_click(Self::click(cx, action));
        let button = match tone {
            Tone::Normal => button.filled(&theme),
            Tone::Primary => button.accent(&theme),
            Tone::Danger => button.destructive(&theme),
        };
        match probe {
            Some(probe) => button.on_bounds(probe.into_bounds_handler()),
            None => button,
        }
    }

    /// An icon-only button; `label` is its tooltip and accessible name. `Normal` has no fill.
    pub(super) fn icon_button(
        &self,
        id: impl Into<ElementId>,
        glyph: Icon,
        label: &'static str,
        tone: Tone,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> IconButton {
        let theme = self.theme;
        let id: ElementId = id.into();
        let probe = Self::bounds_probe(&id, cx);
        let mut button = IconButton::editor(&theme, id, glyph.path())
            .editor_size(IconButtonSize::Small)
            .label(label)
            .tooltip(&theme, label)
            .on_click(Self::click(cx, action));
        if let Some(probe) = probe {
            button = button.on_bounds(probe.into_bounds_handler());
        }
        match tone {
            Tone::Normal => button,
            Tone::Primary => button.accent(&theme),
            Tone::Danger => button.destructive(&theme),
        }
    }

    /// One item of a segmented rail: the selected item is highlighted and inert.
    pub(super) fn segment_item(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        selected: bool,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> EditorButton {
        let id: ElementId = id.into();
        let probe = Self::bounds_probe(&id, cx);
        let mut button = EditorButton::plain(&self.theme, id)
            .label(label)
            .pressed(selected);
        if let Some(probe) = probe {
            button = button.on_bounds(probe.into_bounds_handler());
        }
        if selected {
            button
        } else {
            button.on_click(Self::click(cx, action))
        }
    }

    pub(super) fn segment_rail(&self) -> gpui::Div {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.))
            .p(px(2.))
            .rounded(px(METRICS.radius_control + 2.))
            .bg(self.theme.editor.ctl)
    }

    pub(super) fn tab_item(
        &self,
        tab: Tab,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.tab == tab;
        self.segment_item(
            format!("agent-tab-{label}"),
            label,
            active,
            cx,
            move |this, _, cx| {
                this.tab = tab;
                cx.notify();
            },
        )
    }

    /// A titled group on a nested card.
    pub(super) fn section(&self, title: &str) -> gpui::Div {
        let theme = self.theme;
        Card::editor_nested(&theme)
            .flex()
            .flex_col()
            .gap(px(METRICS.space_2))
            .p(px(METRICS.space_3))
            .child(
                text_style(div(), TextStyle::Label)
                    .text_color(theme.editor.text_1)
                    .child(selectable(title.to_owned())),
            )
    }

    pub(super) fn line(&self, text: impl Into<SharedString>) -> gpui::Div {
        line(&self.theme, text)
    }

    // ---- rows ------------------------------------------------------------------------------------

    /// Rows get their spacing as padding inside the measured element, so the list's
    /// height measurement includes it and stays stable while a row streams.
    pub(super) fn row_shell() -> gpui::Div {
        div().w_full().pb(px(METRICS.space_2))
    }

    pub(super) fn caption(&self, text: &str) -> gpui::Div {
        text_style(div(), TextStyle::Small)
            .text_color(self.theme.editor.text_3)
            .child(selectable(text.to_owned()))
    }
}
