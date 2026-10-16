//! The agent's past sessions as a searchable list.
//!
//! [`HistoryList`] is a standalone view over plain [`SessionEntry`] values (the host builds
//! them from `studio_engine::agent::session_store::history::SessionSummary`, see the `From`
//! impl). It only reports what the user chose: [`HistoryEvent::Open`] for a row and
//! [`HistoryEvent::NewSession`] for the button. Reading, resuming and switching sessions
//! belong to the host.
//!
//! The query box filters and ranks with [`search::matches`]; an empty query shows every
//! session in the order given (newest first). Return opens the best match, Escape clears the
//! query.

use gpui::{
    AnyElement, AppContext as _, Context, Entity, EventEmitter, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, Role, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, div, prelude::FluentBuilder as _, px,
};
use studio_engine::agent::OutcomeKind;
use studio_engine::agent::session_store::history::SessionSummary;
use studio_theme::{TextStyle, Theme, text_style};

use crate::kit::{
    Badge, Button, ButtonSize, ButtonVariant, TextInput, TextInputEvent, TextInputState, Tone,
    focusable,
};

pub mod search;
#[cfg(test)]
mod tests;

/// One session in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    pub id: String,
    pub title: String,
    /// Provider that ran the session.
    pub agent: Option<String>,
    /// Unix seconds of the last activity; `0` when unknown.
    pub updated_unix: u64,
    pub outcome: Option<OutcomeKind>,
    /// The user's prompts, searched after the title.
    pub prompt_text: String,
    /// The session the conversation is showing now.
    pub current: bool,
    /// The session's log could not be read.
    pub unreadable: bool,
}

impl SessionEntry {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            agent: None,
            updated_unix: 0,
            outcome: None,
            prompt_text: String::new(),
            current: false,
            unreadable: false,
        }
    }
}

impl From<&SessionSummary> for SessionEntry {
    fn from(summary: &SessionSummary) -> Self {
        Self {
            id: summary.id.clone(),
            title: summary.title.clone(),
            agent: summary.agent.clone(),
            updated_unix: summary.updated_unix,
            outcome: summary.outcome,
            prompt_text: summary.prompt_text.clone(),
            current: summary.is_current,
            unreadable: summary.unreadable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryEvent {
    /// The user picked a session.
    Open(String),
    /// The user asked for a fresh session.
    NewSession,
}

/// Label and colour of an outcome chip.
pub fn outcome_chip(outcome: OutcomeKind) -> (&'static str, Tone) {
    match outcome {
        OutcomeKind::Accepted => ("Accepted", Tone::Success),
        OutcomeKind::Undone => ("Undone", Tone::Neutral),
        OutcomeKind::AwaitingReview => ("In review", Tone::Info),
        OutcomeKind::Conflict => ("Conflict", Tone::Warning),
        OutcomeKind::Failed => ("Failed", Tone::Danger),
        OutcomeKind::Cancelled => ("Cancelled", Tone::Neutral),
        OutcomeKind::Interrupted => ("Interrupted", Tone::Warning),
        OutcomeKind::Discarded => ("Discarded", Tone::Neutral),
    }
}

/// `5m ago`; empty when the time is unknown.
pub fn relative_time(now_unix: u64, then_unix: u64) -> String {
    if then_unix == 0 {
        return String::new();
    }
    let secs = now_unix.saturating_sub(then_unix);
    match secs {
        0..=59 => "just now".to_owned(),
        60..=3599 => format!("{}m ago", secs / 60),
        3600..=86_399 => format!("{}h ago", secs / 3600),
        86_400..=2_591_999 => format!("{}d ago", secs / 86_400),
        _ => format!("{}mo ago", secs / 2_592_000),
    }
}

const ROW_PAD_X: f32 = 8.0;
const LIST_GAP: f32 = 2.0;

pub struct HistoryList {
    entries: Vec<SessionEntry>,
    now_unix: u64,
    query_input: Entity<TextInputState>,
    /// Indices into `entries`, best match first.
    shown: Vec<usize>,
}

impl EventEmitter<HistoryEvent> for HistoryList {}

impl HistoryList {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query_input = cx.new(|cx| {
            let mut input = TextInputState::single_line(window, cx);
            input.set_placeholder("Search sessions");
            input
        });
        cx.subscribe(
            &query_input,
            |list, input, event: &TextInputEvent, cx| match event {
                TextInputEvent::Changed => list.refilter(cx),
                TextInputEvent::Confirmed => {
                    // With nothing typed there is no "best match" to open.
                    if !list.query_input.read(cx).text().trim().is_empty() {
                        list.open(0, cx);
                    }
                }
                TextInputEvent::Cancelled if !input.read(cx).is_empty() => {
                    input.update(cx, |input, cx| input.set_text("", cx));
                    list.refilter(cx);
                }
                _ => (),
            },
        )
        .detach();
        Self {
            entries: Vec::new(),
            now_unix: 0,
            query_input,
            shown: Vec::new(),
        }
    }

    /// Replaces the list. `entries` are expected newest first; `now_unix` anchors the
    /// relative times. The query is kept and applied to the new entries.
    pub fn set_entries(
        &mut self,
        entries: Vec<SessionEntry>,
        now_unix: u64,
        cx: &mut Context<Self>,
    ) {
        self.entries = entries;
        self.now_unix = now_unix;
        self.refilter(cx);
    }

    pub fn query(&self, cx: &gpui::App) -> String {
        self.query_input.read(cx).text().to_owned()
    }

    pub fn set_query(&mut self, query: &str, cx: &mut Context<Self>) {
        self.query_input
            .update(cx, |input, cx| input.set_text(query.to_owned(), cx));
        self.refilter(cx);
    }

    /// Entries now shown, best match first.
    pub fn shown(&self) -> Vec<&SessionEntry> {
        self.shown.iter().map(|&ix| &self.entries[ix]).collect()
    }

    /// Emits [`HistoryEvent::Open`] for the `ix`-th shown row.
    pub fn open(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(&entry) = self.shown.get(ix) {
            cx.emit(HistoryEvent::Open(self.entries[entry].id.clone()));
        }
    }

    /// Emits [`HistoryEvent::Open`] for the session `id`; unknown ids (the list changed since
    /// the row was drawn) are ignored.
    pub fn open_id(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.entries.iter().any(|entry| entry.id == id) {
            cx.emit(HistoryEvent::Open(id.to_owned()));
        }
    }

    pub fn new_session(&mut self, cx: &mut Context<Self>) {
        cx.emit(HistoryEvent::NewSession);
    }

    fn refilter(&mut self, cx: &mut Context<Self>) {
        let query = self.query_input.read(cx).text().to_owned();
        self.shown = search::matches(&self.entries, &query);
        cx.notify();
    }

    fn row(
        &self,
        ix: usize,
        entry: &SessionEntry,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let meta = [
            entry.agent.clone().unwrap_or_default(),
            relative_time(self.now_unix, entry.updated_unix),
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
        let hover = theme.editor.ctl_hover;
        let id = entry.id.clone();
        let row = div()
            .id(("history-row", ix))
            .debug_selector(move || format!("history-row-{ix}"))
            .role(Role::Button)
            .aria_label(SharedString::from(entry.title.clone()))
            .w_full()
            .px(px(ROW_PAD_X))
            .py(px(6.))
            .rounded(px(6.))
            .flex()
            .items_center()
            .gap(px(8.))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .on_click(cx.listener(move |this, _, _, cx| this.open_id(&id, cx)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        text_style(div().truncate(), TextStyle::Label)
                            .text_color(theme.editor.text_1)
                            .child(SharedString::from(entry.title.clone())),
                    )
                    .when_some((!meta.is_empty()).then_some(meta), |col, meta| {
                        col.child(
                            text_style(div().truncate(), TextStyle::Small)
                                .text_color(theme.editor.text_3)
                                .child(SharedString::from(meta)),
                        )
                    }),
            )
            .when(entry.current, |row| {
                row.child(Badge::new("Current").tone(Tone::Info))
            })
            .when(entry.unreadable, |row| {
                row.child(Badge::new("Unreadable").tone(Tone::Danger))
            })
            .when_some(entry.outcome, |row, outcome| {
                let (label, tone) = outcome_chip(outcome);
                row.child(Badge::new(label).tone(tone))
            });
        focusable(row, theme, false).into_any_element()
    }
}

impl Render for HistoryList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_window(window, cx);
        let query = self.query_input.read(cx).text().trim().to_owned();
        let body: AnyElement = if self.shown.is_empty() {
            let text = if self.entries.is_empty() {
                "No sessions yet".to_owned()
            } else {
                format!("No sessions match \u{201c}{query}\u{201d}")
            };
            text_style(
                div()
                    .debug_selector(|| "history-empty".into())
                    .px(px(ROW_PAD_X))
                    .py(px(12.)),
                TextStyle::Label,
            )
            .text_color(theme.editor.text_3)
            .child(SharedString::from(text))
            .into_any_element()
        } else {
            div()
                .id("history-rows")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(LIST_GAP))
                .children(
                    self.shown
                        .iter()
                        .enumerate()
                        .map(|(ix, &entry)| self.row(ix, &self.entries[entry], &theme, cx)),
                )
                .into_any_element()
        };
        div()
            .id("history-list")
            .debug_selector(|| "history-list".into())
            .size_full()
            .p(px(2.))
            .rounded(px(8.))
            .border_1()
            .border_color(theme.editor.line)
            .bg(theme.editor.card)
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .p(px(4.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .debug_selector(|| "history-search".into())
                            .child(TextInput::plain(
                                &theme,
                                "history-search-input",
                                &self.query_input,
                            )),
                    )
                    .child(
                        div().debug_selector(|| "history-new".into()).child(
                            Button::plain(
                                &theme,
                                "history-new-button",
                                ButtonVariant::Gray,
                                ButtonSize::Sm,
                            )
                            .label("New session")
                            .on_click(cx.listener(|this, _, _, cx| this.new_session(cx))),
                        ),
                    ),
            )
            .child(body)
    }
}
