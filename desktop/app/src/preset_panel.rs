//! Native style-preset controls: bundled and imported preset selection, the CSS import
//! report, export, basic project override editing and an explicit reset.
//!
//! The panel never touches project files itself: it renders a
//! [`studio_engine::presets::PresetView`] and emits [`PresetEvent`]s, and the shell runs
//! the commands through the engine. The video preset's tokens never reach the Studio
//! chrome: the panel uses the application design system, separate from project styling.
use crate::{
    design_system::{self, TextStyle, text_style},
    text_input::TextInput,
    ui::{
        Button, ButtonVariant, Collapsible, Icon, IconButton, IconButtonSize, Notice, SectionTitle,
        Tone, divider, icon,
        interact::{Handler, focusable, on_activate, with_tooltip},
    },
};
use gpui::{
    AppContext, Context, ElementId, Entity, EventEmitter, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};
use std::rc::Rc;
use studio_engine::PresetMutation;
use studio_engine::presets::{
    CssView, MAX_CSS_LINES, Outcome, PresetCommand, PresetReport, PresetView,
};

// ---- GPUI panel ------------------------------------------------------------------------

/// Native pickers the panel asks the shell to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetPick {
    ImportFolder,
    ImportCss,
    Export { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetEvent {
    Run(PresetCommand),
    Pick(PresetPick),
    /// Escape in an input: focus returns to the shell.
    Leave,
}

impl EventEmitter<PresetEvent> for PresetPanel {}

pub struct PresetPanel {
    view: Option<PresetView>,
    selected: Option<String>,
    last: Option<PresetReport>,
    css: Option<CssView>,
    css_open: bool,
    token: Entity<TextInput>,
    value: Entity<TextInput>,
}

fn input(placeholder: &str, cx: &mut Context<PresetPanel>) -> Entity<TextInput> {
    let placeholder = placeholder.to_owned();
    cx.new(move |cx| {
        let mut input = TextInput::new(cx);
        input.placeholder = placeholder.into();
        input.compact = true;
        input
    })
}

impl PresetPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            view: None,
            selected: None,
            last: None,
            css: None,
            css_open: true,
            token: input("Token, for example color.accent", cx),
            value: input("Value, for example #ff5533 or 120", cx),
        }
    }

    // ---- shell interface ---------------------------------------------------------------

    /// The latest presentation of the open project (`None` when none is open).
    pub fn set_view(&mut self, view: Option<PresetView>, cx: &mut Context<Self>) {
        match &view {
            None => {
                self.last = None;
                self.css = None;
                self.selected = None;
            }
            Some(view) => {
                let still = self
                    .selected
                    .as_ref()
                    .is_some_and(|key| view.entries.iter().any(|e| &e.key == key));
                if !still {
                    // Start on what the project uses, else the first bundled preset.
                    self.selected = view
                        .entries
                        .iter()
                        .find(|e| e.applied)
                        .or_else(|| view.entries.first())
                        .map(|e| e.key.clone());
                }
            }
        }
        self.view = view;
        cx.notify();
    }

    /// The outcome of the command the shell just ran.
    pub fn report(&mut self, report: PresetReport, cx: &mut Context<Self>) {
        if let Some(css) = &report.css {
            self.css = Some(css.clone());
        }
        self.last = Some(report);
        cx.notify();
    }

    pub fn selected_key(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// The catalog, the applied preset, why mutation is blocked (if it is) and the last
    /// command's status lines; preset ids and token names only, never values.
    pub fn telemetry(&self) -> serde_json::Value {
        let Some(view) = &self.view else {
            return serde_json::json!({ "entries": [] });
        };
        serde_json::json!({
            "entries": view.entries.iter().map(|entry| serde_json::json!({
                "key": entry.key,
                "id": entry.id,
                "applied": entry.applied,
            })).collect::<Vec<_>>(),
            "selected": self.selected,
            "applied": view.applied.as_ref().map(|applied| serde_json::json!({
                "id": applied.id,
                "overrides": applied.overridden.len(),
            })),
            "blocked": match &view.mutation {
                PresetMutation::Enabled => None,
                PresetMutation::Disabled(reason) => Some(reason.clone()),
            },
            "status": self.last.as_ref().map(PresetReport::status_lines),
        })
    }

    /// True while an override field holds keyboard focus.
    pub fn has_text_focus(&self, window: &Window, cx: &gpui::App) -> bool {
        use gpui::Focusable;
        self.token.focus_handle(cx).is_focused(window)
            || self.value.focus_handle(cx).is_focused(window)
    }

    // ---- building blocks ---------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    /// A stacked, full-width `ui::Button` that runs `action` on this panel.
    fn action_button(
        &self,
        id: impl Into<ElementId>,
        label: &'static str,
        enabled: bool,
        variant: ButtonVariant,
        glyph: Option<Icon>,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> Button {
        let entity = cx.entity();
        let mut button = Button::new(id, label)
            .variant(variant)
            .disabled(!enabled)
            .on_click(move |_, cx| {
                entity.update(cx, |panel, cx| action(panel, cx));
            });
        if let Some(glyph) = glyph {
            button = button.icon(glyph);
        }
        button
    }

    /// One preset in the list: a radio-style row with a check on the selected one.
    /// The accessible name and tooltip carry the full label.
    fn preset_row(
        &self,
        key: String,
        label: String,
        shown: String,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let tokens = design_system::tokens();
        let entity = cx.entity();
        let id = format!("preset-{key}");
        let handler: Handler = Rc::new(move |_, cx| {
            entity.update(cx, |panel, cx| {
                panel.selected = Some(key.clone());
                cx.notify();
            });
        });
        let check = if selected {
            icon(Icon::Check, tokens.metrics.icon_sm, tokens.palette.accent)
        } else {
            icon(Icon::Check, tokens.metrics.icon_sm, tokens.palette.text_3).opacity(0.)
        };
        let mut row = div()
            .id(id)
            .role(gpui::Role::RadioButton)
            .aria_label(label.clone())
            .flex()
            .flex_none()
            .items_center()
            .gap(px(tokens.metrics.space_2))
            .h(px(PRESET_ROW_H))
            .px(px(tokens.metrics.space_2))
            .min_w_0()
            .rounded(px(tokens.metrics.radius_item))
            .hover(|style| style.bg(design_system::tokens().palette.ctl_hover))
            .child(check)
            .child(
                text_style(div().min_w_0().flex_1().truncate(), TextStyle::Body)
                    .text_color(tokens.palette.text_1)
                    .child(shown),
            );
        if selected {
            row = row.bg(tokens.palette.ctl);
        }
        let row = on_activate(focusable(row, false), false, Some(handler));
        with_tooltip(row, Some(label.into()))
    }

    fn small(text: impl Into<SharedString>, color: gpui::Rgba) -> gpui::Div {
        text_style(div(), TextStyle::Small)
            .text_color(color)
            .child(text.into())
    }

    fn run(&mut self, command: PresetCommand, cx: &mut Context<Self>) {
        cx.emit(PresetEvent::Run(command));
    }

    fn set_override(&mut self, cx: &mut Context<Self>) {
        let token = self.token.read(cx).content().trim().to_owned();
        let value = self.value.read(cx).content().trim().to_owned();
        if !token.is_empty() && !value.is_empty() {
            self.run(PresetCommand::SetOverride { token, value }, cx);
        }
    }

    fn field(
        &self,
        id: &'static str,
        input: Entity<TextInput>,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let field = input.clone();
        div()
            .id(id)
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if field.read(cx).is_composing() {
                        cx.stop_propagation();
                        return;
                    }
                    match event.keystroke.key.as_str() {
                        "enter" => {
                            this.set_override(cx);
                            cx.stop_propagation();
                        }
                        "tab" => {
                            if event.keystroke.modifiers.shift {
                                window.focus_prev(cx);
                            } else {
                                window.focus_next(cx);
                            }
                            cx.stop_propagation();
                        }
                        "escape" => {
                            cx.emit(PresetEvent::Leave);
                            cx.stop_propagation();
                        }
                        _ => (),
                    }
                }),
            )
            .child(input)
    }
}

impl Render for PresetPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = design_system::tokens();
        let palette = tokens.palette;
        let mut root = div()
            .id("preset-panel")
            .flex()
            .flex_col()
            .gap(px(tokens.metrics.space_3))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(tokens.metrics.space_1))
                    .child(SectionTitle::new("Style presets"))
                    .child(Self::small(
                        "Presets style the video only; Studio's own interface never changes.",
                        palette.text_2,
                    )),
            );
        let Some(view) = self.view.clone() else {
            return root.child(Self::small(
                "Open a project to choose a preset.",
                palette.text_2,
            ));
        };
        let enabled = view.mutation.is_enabled();
        if let PresetMutation::Disabled(reason) = &view.mutation {
            root = root.child(Notice::new(
                "preset-disabled",
                Tone::Warning,
                format!("Preset changes are disabled: {reason}"),
            ));
        }
        match &view.applied {
            Some(applied) => {
                root = root.child(Self::small(
                    format!(
                        "Applied: {} {} ({})",
                        applied.name,
                        applied.version,
                        &applied.hash[..12]
                    ),
                    palette.text_1,
                ));
            }
            None => {
                root = root.child(Self::small(
                    "No preset applied yet; this project keeps its own styles until you choose one.",
                    palette.text_2,
                ));
            }
        }
        let mut list = div().flex().flex_col();
        for entry in &view.entries {
            let detail = format!(
                "{} {} · {}{}",
                entry.name,
                entry.version,
                if entry.bundled { "bundled" } else { "imported" },
                if entry.applied { " · applied" } else { "" }
            );
            let label = format!("{}{detail}", if entry.applied { "✓ " } else { "" });
            list = list.child(self.preset_row(
                entry.key.clone(),
                label,
                detail,
                self.selected.as_deref() == Some(entry.key.as_str()),
                cx,
            ));
        }
        for skipped in &view.skipped {
            list = list.child(Self::small(format!("Skipped: {skipped}"), palette.danger));
        }
        root = root.child(list);
        let selected = self
            .selected
            .as_ref()
            .and_then(|key| view.entries.iter().find(|e| &e.key == key))
            .cloned();
        if let Some(entry) = &selected {
            root = root.child(Self::small(
                format!("{} — {}", entry.id, entry.description),
                palette.text_2,
            ));
        }
        let key = selected.as_ref().map(|e| e.key.clone());
        let can_select = enabled && key.is_some();
        let reapply = enabled && selected.as_ref().is_some_and(|e| e.same_id_applied);
        let reset = enabled && view.applied.is_some() && key.is_some();
        // At 216 px the buttons stack; a row would not fit their labels.
        let mut actions = div().flex().flex_col().gap(px(tokens.metrics.space_2));
        let apply_key = key.clone();
        actions = actions.child(self.action_button(
            "preset-apply",
            "Apply",
            can_select,
            ButtonVariant::Primary,
            Some(Icon::Check),
            cx,
            move |this, cx| {
                if let Some(key) = apply_key.clone() {
                    this.run(PresetCommand::Apply { key }, cx);
                }
            },
        ));
        let reapply_key = key.clone();
        actions = actions.child(self.action_button(
            "preset-reapply",
            "Reapply (keep overrides)",
            reapply,
            ButtonVariant::Editor,
            Some(Icon::RefreshCw),
            cx,
            move |this, cx| {
                if let Some(key) = reapply_key.clone() {
                    this.run(PresetCommand::Reapply { key }, cx);
                }
            },
        ));
        let reset_key = key.clone();
        actions = actions.child(self.action_button(
            "preset-reset",
            "Reset overrides",
            reset,
            ButtonVariant::Editor,
            Some(Icon::History),
            cx,
            move |this, cx| {
                if let Some(key) = reset_key.clone() {
                    this.run(PresetCommand::Reset { key }, cx);
                }
            },
        ));
        root = root.child(actions);

        let export_key = key.clone();
        root = root.child(divider()).child(
            div()
                .flex()
                .flex_col()
                .gap(px(tokens.metrics.space_2))
                .child(self.action_button(
                    "preset-import",
                    "Import preset folder…",
                    true,
                    ButtonVariant::Editor,
                    Some(Icon::Upload),
                    cx,
                    |_, cx| {
                        cx.emit(PresetEvent::Pick(PresetPick::ImportFolder));
                    },
                ))
                .child(self.action_button(
                    "preset-export",
                    "Export selected…",
                    key.is_some(),
                    ButtonVariant::Editor,
                    Some(Icon::Download),
                    cx,
                    move |_, cx| {
                        if let Some(key) = export_key.clone() {
                            cx.emit(PresetEvent::Pick(PresetPick::Export { key }));
                        }
                    },
                ))
                .child(self.action_button(
                    "preset-css",
                    "CSS import report…",
                    true,
                    ButtonVariant::Editor,
                    Some(Icon::File),
                    cx,
                    |_, cx| {
                        cx.emit(PresetEvent::Pick(PresetPick::ImportCss));
                    },
                )),
        );

        // Project overrides: the user's layer, kept across Reapply and reopen.
        root = root
            .child(divider())
            .child(SectionTitle::new("Project overrides"));
        if let Some(applied) = &view.applied {
            let mut overrides = div().flex().flex_col();
            if applied.overridden.is_empty() {
                overrides = overrides.child(Self::small("No project overrides.", palette.text_2));
            }
            for (token, layer, value) in &applied.overridden {
                let clear = token.clone();
                let entity = cx.entity();
                overrides = overrides.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(tokens.metrics.space_2))
                        .min_h(px(tokens.metrics.field_row_h))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    text_style(div().truncate(), TextStyle::Body)
                                        .text_color(palette.text_1)
                                        .child(token.clone()),
                                )
                                .child(
                                    text_style(div().truncate(), TextStyle::Small)
                                        .text_color(palette.text_3)
                                        .child(format!("{layer} · {value}")),
                                ),
                        )
                        .child(
                            IconButton::new(format!("clear-{token}"), Icon::X)
                                .size(IconButtonSize::Small)
                                .label("Clear")
                                .disabled(!enabled)
                                .on_click(move |_, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.run(
                                            PresetCommand::ClearOverride {
                                                token: clear.clone(),
                                            },
                                            cx,
                                        );
                                    });
                                }),
                        ),
                );
            }
            root = root.child(overrides);
            for (index, diagnostic) in applied.diagnostics.iter().enumerate() {
                root = root.child(Notice::new(
                    ElementId::from(("preset-diagnostic", index)),
                    Tone::Warning,
                    format!("Kept, not applied: {diagnostic}"),
                ));
            }
            let token = self.field("override-token", self.token.clone(), cx);
            let value = self.field("override-value", self.value.clone(), cx);
            root = root.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(tokens.metrics.space_2))
                    .child(token)
                    .child(value)
                    .child(self.action_button(
                        "override-set",
                        "Set override",
                        enabled,
                        ButtonVariant::Editor,
                        Some(Icon::Plus),
                        cx,
                        |this, cx| this.set_override(cx),
                    )),
            );
        }

        if let Some(last) = &self.last {
            let tone = match &last.outcome {
                Outcome::Committed { .. } | Outcome::Unchanged { .. } | Outcome::Done(_) => {
                    Tone::Success
                }
                Outcome::Refused(_) => Tone::Danger,
            };
            let (title, body) = status_text(last.status_lines());
            let mut status = Notice::new("preset-status", tone, title);
            if let Some(body) = body {
                status = status.body(body);
            }
            root = root.child(status);
        }
        if let Some(css) = &self.css {
            let mut lines = div()
                .id("css-lines")
                .max_h(px(200.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(tokens.metrics.space_1))
                .pt(px(tokens.metrics.space_1))
                .child(Self::small(
                    format!(
                        "CSS report · {}: {} accepted · {} unsupported · {} rejected",
                        css.file, css.accepted, css.unsupported, css.rejected
                    ),
                    palette.text_1,
                ));
            for line in &css.lines {
                lines = lines.child(Self::small(line.clone(), palette.text_2));
            }
            if css.truncated {
                lines = lines.child(Self::small(
                    format!("Showing the first {MAX_CSS_LINES} declarations."),
                    palette.text_2,
                ));
            }
            let entity = cx.entity();
            root = root.child(
                div().id("css-report").child(
                    Collapsible::new("css-report-section", self.css_open)
                        .header(SectionTitle::new("CSS import report"))
                        .content(lines)
                        .on_toggle(move |open, _, cx| {
                            entity.update(cx, |panel, cx| {
                                panel.css_open = open;
                                cx.notify();
                            });
                        }),
                ),
            );
        }
        root
    }
}

/// Height of a preset list row.
const PRESET_ROW_H: f32 = 34.;

/// The first status line as the notice title and the remaining lines as its body.
fn status_text(lines: Vec<String>) -> (String, Option<String>) {
    let mut lines = lines.into_iter();
    let title = lines.next().unwrap_or_default();
    let rest = lines.collect::<Vec<_>>().join("\n");
    (title, (!rest.is_empty()).then_some(rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_text_splits_title_from_body() {
        assert_eq!(status_text(vec![]), (String::new(), None));
        assert_eq!(status_text(vec!["a".into()]), ("a".into(), None));
        assert_eq!(
            status_text(vec!["a".into(), "b".into(), "c".into()]),
            (
                "a".into(),
                Some(
                    "b
c"
                    .into()
                )
            )
        );
    }
}

/// The override fields' keyboard contract: Tab walks from token to value, Enter sets
/// the override, Escape leaves, and an IME composition keeps Enter and Escape.
#[cfg(test)]
mod field_keys {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{Entity, EntityInputHandler, Focusable, TestAppContext, VisualTestContext};
    use studio_engine::PresetMutation;
    use studio_engine::presets::{AppliedView, PresetCommand, PresetView};

    use super::{PresetEvent, PresetPanel};

    type Events = Rc<RefCell<Vec<PresetEvent>>>;

    fn applied() -> PresetView {
        PresetView {
            entries: Vec::new(),
            skipped: Vec::new(),
            applied: Some(AppliedView {
                id: "studio.basic".into(),
                name: "Basic".into(),
                version: "1.0.0".into(),
                hash: "0123456789abcdef".repeat(4),
                overridden: Vec::new(),
                diagnostics: Vec::new(),
            }),
            mutation: PresetMutation::Enabled,
        }
    }

    fn open(cx: &mut TestAppContext) -> (Entity<PresetPanel>, Events, &mut VisualTestContext) {
        cx.update(crate::text_input::bind_keys);
        let (panel, cx) = cx.add_window_view(|_, cx| {
            let mut panel = PresetPanel::new(cx);
            panel.set_view(Some(applied()), cx);
            panel
        });
        let events = Events::default();
        let log = events.clone();
        cx.update(|_, cx| {
            cx.subscribe(&panel, move |_, event: &PresetEvent, _| {
                log.borrow_mut().push(event.clone())
            })
            .detach()
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let token = panel.read(cx).token.focus_handle(cx);
            window.focus(&token, cx);
        });
        (panel, events, cx)
    }

    #[gpui::test]
    fn tab_then_enter_sets_the_override(cx: &mut TestAppContext) {
        let (_, events, cx) = open(cx);
        cx.simulate_input("color.accent");
        cx.simulate_keystrokes("tab");
        cx.simulate_input("#ff5533");
        cx.simulate_keystrokes("enter");
        assert_eq!(
            *events.borrow(),
            [PresetEvent::Run(PresetCommand::SetOverride {
                token: "color.accent".into(),
                value: "#ff5533".into(),
            })]
        );
    }

    #[gpui::test]
    fn escape_leaves_the_field(cx: &mut TestAppContext) {
        let (_, events, cx) = open(cx);
        cx.simulate_keystrokes("escape");
        assert_eq!(*events.borrow(), [PresetEvent::Leave]);
    }

    #[gpui::test]
    fn a_composition_keeps_enter_and_escape(cx: &mut TestAppContext) {
        let (panel, events, cx) = open(cx);
        cx.simulate_input("color.accent");
        cx.update(|window, cx| {
            let token = panel.read(cx).token.clone();
            token.update(cx, |input, cx| {
                input.replace_and_mark_text_in_range(None, "\u{304b}", None, window, cx)
            });
        });
        cx.simulate_keystrokes("enter");
        cx.simulate_keystrokes("escape");
        assert!(events.borrow().is_empty(), "{:?}", events.borrow());
    }
}
