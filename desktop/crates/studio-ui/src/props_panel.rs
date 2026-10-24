//! The props panel: the editable fields of the component selected on the canvas, after
//! the preview worker's answer to `PropsInfo`, with a control per kind and "reset to
//! code" per field.
//!
//! The panel never writes `props.json` itself: it emits [`PropsEvent`]s, and the shell
//! runs the commands through the engine, which writes the file and hands the document to
//! the preview. A slider drag is one gesture, so it is one undo step and one write.
use std::collections::HashMap;

use fframes_studio_protocol::{EditorObjectIdentity, PropField, PropKind, PropsInfo};
use gpui::{
    AppContext, Context, DispatchPhase, Entity, EventEmitter, Focusable, InteractiveElement,
    IntoElement, MouseButton, MouseUpEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, canvas, div, prelude::FluentBuilder, px,
};
use serde_json::Value;
use studio_engine::props::{PropsCommand, PropsEditing, PropsSnapshot};
use studio_theme::{METRICS, TextStyle, Theme, text_style};

use crate::kit::{
    EditorButton, Icon, IconButton, IconButtonSize, Notice, Slider, SliderTrack, TextInput,
    TextInputEvent, TextInputState, Toggle, Tone, section_title, selectable,
};

#[derive(Debug, Clone, PartialEq)]
pub enum PropsEvent {
    Run(PropsCommand),
    /// Escape in a field: focus returns to the shell.
    Leave,
}

impl EventEmitter<PropsEvent> for PropsPanel {}

pub struct PropsPanel {
    snapshot: Option<PropsSnapshot>,
    selected: Option<EditorObjectIdentity>,
    /// The worker's answer for `selected`; `None` until it arrives.
    info: Option<Result<Option<PropsInfo>, String>>,
    /// Text fields of the shown schema, by field name, with the value they were last set
    /// to from the preview.
    inputs: HashMap<String, (Entity<TextInputState>, String)>,
    /// What the inputs were made for: the object and its component.
    inputs_for: Option<(EditorObjectIdentity, String)>,
    /// The field a slider drag changes; the drag is one gesture.
    dragging: Option<String>,
    tracks: HashMap<String, SliderTrack>,
    /// Why the last entry could not be applied.
    error: Option<String>,
    /// Refreshed at the top of every render.
    theme: Theme,
}

impl PropsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            snapshot: None,
            selected: None,
            info: None,
            inputs: HashMap::new(),
            inputs_for: None,
            dragging: None,
            tracks: HashMap::new(),
            error: None,
            theme: Theme::for_window(window, cx),
        }
    }

    // ---- shell interface ---------------------------------------------------------------

    /// The latest props of the open project (`None` when none is open).
    pub fn set_snapshot(&mut self, snapshot: Option<PropsSnapshot>, cx: &mut Context<Self>) {
        if self.snapshot != snapshot {
            self.snapshot = snapshot;
            cx.notify();
        }
    }

    /// The object selected on the canvas; its props arrive with [`Self::set_info`].
    pub fn set_selection(
        &mut self,
        identity: Option<EditorObjectIdentity>,
        cx: &mut Context<Self>,
    ) {
        if self.selected != identity {
            self.selected = identity;
            self.info = None;
            self.error = None;
            self.end_drag(cx);
            cx.notify();
        }
    }

    /// The preview's answer for `identity`; ignored when another object is selected now.
    pub fn set_info(
        &mut self,
        identity: EditorObjectIdentity,
        info: Result<Option<PropsInfo>, String>,
        cx: &mut Context<Self>,
    ) {
        if self.selected.as_ref() == Some(&identity) {
            self.info = Some(info);
            cx.notify();
        }
    }

    /// The selection and what the panel shows for it; field names only, never values.
    pub fn telemetry(&self) -> serde_json::Value {
        let shown = match &self.info {
            Some(Ok(Some(info))) => serde_json::json!({
                "component": info.schema.component,
                "fields": info.schema.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>(),
            }),
            Some(Ok(None)) => serde_json::json!("not editable"),
            Some(Err(error)) => serde_json::json!({ "error": error }),
            None => serde_json::Value::Null,
        };
        serde_json::json!({
            "selected": self.selected.as_ref().map(|id| id.object_key.clone()),
            "shown": shown,
            "dragging": self.dragging,
        })
    }

    /// True while a field holds keyboard focus.
    pub fn has_text_focus(&self, window: &Window, cx: &gpui::App) -> bool {
        self.inputs
            .values()
            .any(|(input, _)| input.focus_handle(cx).is_focused(window))
    }

    // ---- edits -------------------------------------------------------------------------

    fn run(&mut self, command: PropsCommand, cx: &mut Context<Self>) {
        cx.emit(PropsEvent::Run(command));
    }

    fn set_field(&mut self, field: &str, value: Option<Value>, cx: &mut Context<Self>) {
        let Some(identity) = self.selected.clone() else {
            return;
        };
        self.error = None;
        self.run(
            PropsCommand::Set {
                identity,
                field: field.to_owned(),
                value,
            },
            cx,
        );
    }

    /// Return in a text field: the entry, read as the field's kind.
    fn commit(&mut self, field: &PropField, cx: &mut Context<Self>) {
        let Some((input, _)) = self.inputs.get(&field.name) else {
            return;
        };
        let text = input.read(cx).text().to_owned();
        match parse_entry(field, &text) {
            Ok(value) => self.set_field(&field.name, Some(value), cx),
            Err(error) => {
                self.error = Some(format!("{}: {error}", field.name));
                cx.notify();
            }
        }
    }

    fn drag(&mut self, field: &PropField, fraction: f32, cx: &mut Context<Self>) {
        let (Some(min), Some(max)) = (field.min, field.max) else {
            return;
        };
        if self.dragging.as_deref() != Some(field.name.as_str()) {
            self.end_drag(cx);
            self.dragging = Some(field.name.clone());
            self.run(PropsCommand::BeginGesture, cx);
        }
        let value = slider_value(field, min, max, fraction);
        self.set_field(&field.name, Some(value), cx);
    }

    fn end_drag(&mut self, cx: &mut Context<Self>) {
        if self.dragging.take().is_some() {
            self.run(PropsCommand::EndGesture, cx);
            cx.notify();
        }
    }

    /// Makes the text fields of `info`'s schema and puts the drawn values in the ones
    /// that are not being typed in.
    fn sync_inputs(&mut self, info: &PropsInfo, window: &mut Window, cx: &mut Context<Self>) {
        let Some(identity) = self.selected.clone() else {
            return;
        };
        let target = (identity, info.schema.component.clone());
        if self.inputs_for.as_ref() != Some(&target) {
            self.inputs.clear();
            self.tracks.clear();
            self.inputs_for = Some(target);
        }
        for field in info.schema.fields.iter().filter(|f| typed(&f.kind)) {
            let shown = info
                .values
                .get(&field.name)
                .map(display_value)
                .unwrap_or_default();
            if !self.inputs.contains_key(&field.name) {
                let input = cx.new(|cx| TextInputState::single_line(window, cx));
                let committed = field.clone();
                cx.subscribe(
                    &input,
                    move |panel, _, event: &TextInputEvent, cx| match event {
                        TextInputEvent::Confirmed => panel.commit(&committed, cx),
                        TextInputEvent::Cancelled => cx.emit(PropsEvent::Leave),
                        _ => (),
                    },
                )
                .detach();
                self.inputs
                    .insert(field.name.clone(), (input, String::new()));
            }
            let (input, synced) = self.inputs.get_mut(&field.name).expect("just inserted");
            if *synced != shown && !input.focus_handle(cx).is_focused(window) {
                input.update(cx, |input, cx| input.set_text(shown.clone(), cx));
                *synced = shown;
            }
        }
    }

    // ---- rendering ---------------------------------------------------------------------

    fn small(text: impl Into<SharedString>, color: gpui::Rgba) -> gpui::Div {
        text_style(div(), TextStyle::Small)
            .text_color(color)
            .child(selectable(text))
    }

    fn field_row(
        &self,
        field: &PropField,
        info: &PropsInfo,
        overridden: bool,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        let value = info.values.get(&field.name);
        let id = format!("prop-{}", field.name);
        let mut label = field.name.clone();
        if let Some(unit) = &field.unit {
            label.push_str(&format!(" ({unit})"));
        }
        let mut header = div().flex().items_center().gap(px(METRICS.space_2)).child(
            text_style(div().min_w_0().flex_1().truncate(), TextStyle::Body)
                .text_color(palette.text_1)
                .child(label),
        );
        if overridden {
            let entity = cx.entity();
            let name = field.name.clone();
            header = header.child(
                IconButton::editor(&theme, format!("{id}-reset"), Icon::RefreshCw.path())
                    .editor_size(IconButtonSize::Small)
                    .label("Reset to the code's value")
                    .tooltip(&theme, "Reset to the code's value")
                    .disabled(!enabled)
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |panel, cx| panel.set_field(&name, None, cx));
                    }),
            );
        }
        let control = match &field.kind {
            PropKind::Toggle => {
                let entity = cx.entity();
                let name = field.name.clone();
                Toggle::plain(
                    &theme,
                    id.clone(),
                    value.and_then(Value::as_bool).unwrap_or(false),
                )
                .label(field.name.clone())
                .disabled(&theme, !enabled)
                .on_change(move |checked, _, cx| {
                    entity.update(cx, |panel, cx| {
                        panel.set_field(&name, Some(Value::Bool(checked)), cx)
                    });
                })
                .into_any_element()
            }
            PropKind::Choice { options } => {
                let current = value.and_then(Value::as_str);
                let mut row = div().flex().flex_wrap().gap(px(METRICS.space_1));
                for option in options {
                    let entity = cx.entity();
                    let name = field.name.clone();
                    let chosen = option.clone();
                    let mut button = EditorButton::plain(&theme, format!("{id}-{option}"))
                        .label(option.clone())
                        .disabled(!enabled)
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |panel, cx| {
                                panel.set_field(&name, Some(Value::String(chosen.clone())), cx)
                            });
                        });
                    button = if current == Some(option.as_str()) {
                        button.accent(&theme)
                    } else {
                        button.filled(&theme)
                    };
                    row = row.child(button);
                }
                row.into_any_element()
            }
            _ => self.typed_control(field, value, enabled, cx),
        };
        div()
            .id(SharedString::from(id))
            .flex()
            .flex_col()
            .gap(px(METRICS.space_1))
            .child(header)
            .child(control)
            .into_any_element()
    }

    /// A text field, with a slider above it for a number with a range and a swatch for
    /// a colour.
    fn typed_control(
        &self,
        field: &PropField,
        value: Option<&Value>,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        let mut column = div().flex().flex_col().gap(px(METRICS.space_1));
        if let (PropKind::Number | PropKind::Integer, Some(min), Some(max)) =
            (&field.kind, field.min, field.max)
            && max > min
        {
            let fraction = value
                .and_then(Value::as_f64)
                .map_or(0., |v| ((v - min) / (max - min)).clamp(0., 1.))
                as f32;
            let track = self.tracks.get(&field.name).cloned().unwrap_or_default();
            let entity = cx.entity();
            let dragged = field.clone();
            column = column.child(
                Slider::new(format!("prop-{}-slider", field.name), fraction, track)
                    .flex()
                    .track(px(3.), palette.ctl_active.into())
                    .fill(palette.accent.into())
                    .thumb(px(12.), palette.thumb.into(), None)
                    .label(field.name.clone())
                    .disabled(!enabled)
                    .on_change(0., move |fraction, _, cx| {
                        entity.update(cx, |panel, cx| panel.drag(&dragged, *fraction, cx));
                    }),
            );
        }
        let mut entry = div().flex().items_center().gap(px(METRICS.space_2));
        if field.kind == PropKind::Color {
            let swatch = value
                .and_then(Value::as_str)
                .and_then(parse_hex)
                .unwrap_or(gpui::rgba(0x00000000));
            entry = entry.child(
                div()
                    .flex_none()
                    .size(px(20.))
                    .rounded(px(METRICS.radius_item))
                    .border_1()
                    .border_color(palette.line)
                    .bg(swatch),
            );
        }
        if let Some((input, _)) = self.inputs.get(&field.name) {
            entry = entry.child(div().min_w_0().flex_1().child(TextInput::plain(
                &theme,
                SharedString::from(format!("prop-{}-input", field.name)),
                input,
            )));
        }
        column.child(entry).into_any_element()
    }
}

impl Render for PropsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.theme.refresh(window, cx);
        let theme = self.theme;
        let palette = theme.editor;
        let mut root = div()
            .id("props-panel")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_3))
            .overflow_y_scroll()
            .child(section_title(&theme, "Props"));
        let Some(snapshot) = self.snapshot.clone() else {
            return root.child(Self::small("Open a project to edit props.", palette.text_2));
        };
        let enabled = snapshot.editing == PropsEditing::Enabled;
        let mut history = div().flex().gap(px(METRICS.space_1));
        for (id, glyph, label, possible, command) in [
            (
                "props-undo",
                Icon::Undo,
                "Undo props edit",
                snapshot.can_undo,
                PropsCommand::Undo,
            ),
            (
                "props-redo",
                Icon::Redo,
                "Redo props edit",
                snapshot.can_redo,
                PropsCommand::Redo,
            ),
        ] {
            let entity = cx.entity();
            history = history.child(
                IconButton::editor(&theme, id, glyph.path())
                    .editor_size(IconButtonSize::Small)
                    .label(label)
                    .tooltip(&theme, label)
                    .disabled(!(enabled && possible))
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |panel, cx| panel.run(command.clone(), cx));
                    }),
            );
        }
        root = root.child(history);
        if let PropsEditing::Disabled(reason) = &snapshot.editing {
            root = root.child(Notice::new(
                "props-disabled",
                Tone::Warning,
                format!("Props edits are disabled: {reason}"),
            ));
        }
        if let Some(notice) = &snapshot.notice {
            root = root.child(Notice::new("props-notice", Tone::Info, notice.clone()));
        }
        if let Some(error) = &self.error {
            root = root.child(Notice::new("props-error", Tone::Warning, error.clone()));
        }
        let Some(selected) = self.selected.clone() else {
            return root.child(Self::small(
                "Select a component on the canvas to edit its props.",
                palette.text_2,
            ));
        };
        let info = match self.info.clone() {
            None => {
                return root.child(Self::small(
                    "Reading the selection's props…",
                    palette.text_2,
                ));
            }
            Some(Err(error)) => {
                return root.child(Notice::new("props-unavailable", Tone::Info, error));
            }
            Some(Ok(None)) => {
                return root.child(Self::small(
                    format!(
                        "{} \"{}\" has no editable props. Mark it editable in the code (`.editable(\"key\")`).",
                        selected.component_key, selected.object_key
                    ),
                    palette.text_2,
                ));
            }
            Some(Ok(Some(info))) => info,
        };
        self.sync_inputs(&info, window, cx);
        root = root.child(Self::small(
            format!(
                "{} \"{}\" in {}",
                info.schema.component, selected.object_key, selected.scene_instance_key
            ),
            palette.text_2,
        ));
        let overrides = snapshot
            .doc
            .as_ref()
            .and_then(|doc| doc.values(&selected))
            .cloned()
            .unwrap_or_default();
        if info.schema.fields.is_empty() {
            root = root.child(Self::small(
                "This component has no editable fields.",
                palette.text_2,
            ));
        }
        for field in &info.schema.fields {
            let overridden = overrides.contains_key(&field.name);
            root = root.child(self.field_row(field, &info, overridden, enabled, cx));
        }
        if self.dragging.is_some() {
            // The slider's release can land anywhere in the window.
            let entity = cx.entity();
            root = root.child(
                canvas(
                    |_, _, _| {},
                    move |_, _, window, _| {
                        let entity = entity.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture && event.button == MouseButton::Left
                            {
                                entity.update(cx, |panel, cx| panel.end_drag(cx));
                            }
                        });
                    },
                )
                .absolute()
                .size_0(),
            );
        }
        root.when(!enabled, |root| root.opacity(0.9))
    }
}

/// Fields edited by typing: numbers, text and colours.
fn typed(kind: &PropKind) -> bool {
    matches!(
        kind,
        PropKind::Number | PropKind::Integer | PropKind::Text | PropKind::Color
    )
}

/// How a drawn value reads in its text field.
pub fn display_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Reads a text entry as the field's kind, checking its range.
pub fn parse_entry(field: &PropField, text: &str) -> Result<Value, String> {
    let text = text.trim();
    let number = |value: f64| {
        if field.min.is_some_and(|min| value < min) || field.max.is_some_and(|max| value > max) {
            Err(format!(
                "must be between {} and {}",
                field.min.map_or("-∞".into(), |v| v.to_string()),
                field.max.map_or("∞".into(), |v| v.to_string())
            ))
        } else {
            Ok(value)
        }
    };
    match &field.kind {
        PropKind::Number => {
            let value = text
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or("expected a number")?;
            let value = number(value)?;
            serde_json::Number::from_f64(value)
                .map(Value::Number)
                .ok_or_else(|| "expected a number".into())
        }
        PropKind::Integer => {
            let value = text.parse::<i64>().map_err(|_| "expected a whole number")?;
            number(value as f64)?;
            Ok(Value::from(value))
        }
        PropKind::Text => Ok(Value::String(text.to_owned())),
        PropKind::Color => parse_hex(text)
            .map(|_| Value::String(text.to_ascii_lowercase()))
            .ok_or_else(|| "expected #rrggbb or #rrggbbaa".into()),
        PropKind::Toggle | PropKind::Choice { .. } => Err("not a text field".into()),
    }
}

/// `#rrggbb` or `#rrggbbaa`.
fn parse_hex(text: &str) -> Option<gpui::Rgba> {
    let hex = text.strip_prefix('#')?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    match hex.len() {
        6 => Some(gpui::rgba((value << 8) | 0xff)),
        8 => Some(gpui::rgba(value)),
        _ => None,
    }
}

/// The value a slider of `field` at `fraction` stands for: snapped to the step, whole
/// for an integer.
pub fn slider_value(field: &PropField, min: f64, max: f64, fraction: f32) -> Value {
    let mut value = min + f64::from(fraction.clamp(0., 1.)) * (max - min);
    if let Some(step) = field.step.filter(|step| *step > 0.) {
        value = (min + ((value - min) / step).round() * step).clamp(min, max);
    }
    if field.kind == PropKind::Integer {
        Value::from(value.round() as i64)
    } else {
        // Two decimals are what a slider can aim at.
        let value = (value * 100.).round() / 100.;
        serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn field(kind: PropKind, min: Option<f64>, max: Option<f64>) -> PropField {
        PropField {
            name: "size".into(),
            kind,
            min,
            max,
            step: None,
            unit: None,
        }
    }

    #[test]
    fn entries_are_read_as_their_kind_and_range() {
        let number = field(PropKind::Number, Some(0.), Some(100.));
        assert_eq!(parse_entry(&number, " 42.5 ").unwrap(), json!(42.5));
        assert!(
            parse_entry(&number, "120")
                .unwrap_err()
                .contains("between 0 and 100")
        );
        assert!(parse_entry(&number, "big").is_err());
        let integer = field(PropKind::Integer, None, Some(10.));
        assert_eq!(parse_entry(&integer, "7").unwrap(), json!(7));
        assert!(parse_entry(&integer, "7.5").is_err());
        assert!(parse_entry(&integer, "11").is_err());
        let text = field(PropKind::Text, None, None);
        assert_eq!(parse_entry(&text, "Hello").unwrap(), json!("Hello"));
        let color = field(PropKind::Color, None, None);
        assert_eq!(parse_entry(&color, "#FF5500").unwrap(), json!("#ff5500"));
        assert_eq!(
            parse_entry(&color, "#ff550080").unwrap(),
            json!("#ff550080")
        );
        assert!(parse_entry(&color, "ff5500").is_err());
        assert!(parse_entry(&color, "#ff55").is_err());
    }

    #[test]
    fn values_read_back_into_their_fields() {
        assert_eq!(display_value(&json!("Hi")), "Hi");
        assert_eq!(display_value(&json!(3.5)), "3.5");
        assert_eq!(display_value(&json!(true)), "true");
        assert_eq!(display_value(&Value::Null), "");
    }

    #[test]
    fn sliders_snap_to_the_step_and_stay_whole_for_integers() {
        let mut stepped = field(PropKind::Number, Some(0.), Some(10.));
        stepped.step = Some(2.5);
        assert_eq!(slider_value(&stepped, 0., 10., 0.3), json!(2.5));
        assert_eq!(slider_value(&stepped, 0., 10., 1.), json!(10.0));
        let integer = field(PropKind::Integer, Some(0.), Some(9.));
        assert_eq!(slider_value(&integer, 0., 9., 0.5), json!(5));
        let plain = field(PropKind::Number, Some(0.), Some(1.));
        assert_eq!(slider_value(&plain, 0., 1., 0.333_333), json!(0.33));
    }
}
