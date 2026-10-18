//! The agent's current plan as a todo panel docked above the composer.
//!
//! Every driver that plans sends the *whole* list on each update, so the panel
//! holds the latest list and replaces it wholesale. A plan is a display
//! surface only: nothing here feeds back into the workflow.
//!
//! The first half of the file is pure (what the header says, the default open
//! state) and is unit-tested without a window; the second half is the
//! [`TodoPanel`] view.
//!
//! The panel is collapsed to one line (`Todo 2/5` and the item being worked
//! on) or expanded to the full list. The choice is remembered per session key
//! for the life of the view, so switching sessions and back keeps it.

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, Context, Empty, FontWeight, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, Role, SharedString, StatefulInteractiveElement as _,
    Styled as _, Transformation, Window, div, prelude::FluentBuilder as _, px, radians,
};
use studio_theme::{TextStyle, Theme, text_style};

use crate::kit::{Icon, focusable, icon};

/// Where an item stands. Same variants as the driver's plan status, so a
/// `From` impl on the host side is a one-to-one match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

/// One line of the agent's plan. Same fields as the driver's plan entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoItem {
    pub text: String,
    pub status: TodoStatus,
}

impl TodoItem {
    pub fn new(text: impl Into<String>, status: TodoStatus) -> Self {
        Self {
            text: text.into(),
            status,
        }
    }
}

impl From<studio_engine::agent::PlanStatus> for TodoStatus {
    fn from(status: studio_engine::agent::PlanStatus) -> Self {
        use studio_engine::agent::PlanStatus;
        match status {
            PlanStatus::Pending => Self::Pending,
            PlanStatus::InProgress => Self::InProgress,
            PlanStatus::Completed => Self::Completed,
        }
    }
}

impl From<studio_engine::agent::PlanEntry> for TodoItem {
    fn from(entry: studio_engine::agent::PlanEntry) -> Self {
        Self::new(entry.text, entry.status.into())
    }
}

/// What the collapsed header reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TodoSummary {
    pub total: usize,
    pub done: usize,
    /// First in-progress item.
    pub active: Option<usize>,
    /// First item not yet completed.
    pub next: Option<usize>,
}

impl TodoSummary {
    pub fn of(items: &[TodoItem]) -> Self {
        let mut summary = Self {
            total: items.len(),
            done: 0,
            active: None,
            next: None,
        };
        for (ix, item) in items.iter().enumerate() {
            match item.status {
                TodoStatus::Completed => summary.done += 1,
                TodoStatus::InProgress => {
                    summary.active.get_or_insert(ix);
                    summary.next.get_or_insert(ix);
                }
                TodoStatus::Pending => {
                    summary.next.get_or_insert(ix);
                }
            }
        }
        summary
    }

    pub fn finished(&self) -> bool {
        self.total > 0 && self.done == self.total
    }

    /// The item the collapsed header names: what is being worked on, else what
    /// is next.
    pub fn headline(&self) -> Option<usize> {
        self.active.or(self.next)
    }

    /// `2/5`.
    pub fn label(&self) -> String {
        format!("{}/{}", self.done, self.total)
    }
}

const HEADER_HEIGHT: f32 = 32.0;
const ROW_PAD_X: f32 = 8.0;
const GLYPH_SLOT: f32 = 14.0;
/// An expanded list longer than this scrolls instead of growing the panel.
const LIST_MAX_HEIGHT: f32 = 220.0;
/// A long item wraps up to this many lines, then ends in an ellipsis.
const ITEM_MAX_LINES: usize = 3;
const SPIN_PERIOD: Duration = Duration::from_millis(1100);

/// The panel view. Hosts keep one entity and call [`TodoPanel::set_plan`]
/// whenever the session's plan changes.
pub struct TodoPanel {
    session: SharedString,
    items: Vec<TodoItem>,
    /// Whether the agent is working right now: only then does the in-progress
    /// glyph spin, so a stopped run does not look busy forever.
    live: bool,
    /// The user's explicit choice per session; absent follows the default
    /// (open while work remains, compact once everything is done).
    expanded: HashMap<SharedString, bool>,
}

impl Default for TodoPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoPanel {
    pub fn new() -> Self {
        Self {
            session: SharedString::default(),
            items: Vec::new(),
            live: false,
            expanded: HashMap::new(),
        }
    }

    /// Shows `items` for `session`, replacing whatever was shown. An empty list
    /// hides the panel. Collapse choices made for other sessions are kept.
    pub fn set_plan(
        &mut self,
        session: impl Into<SharedString>,
        items: Vec<TodoItem>,
        cx: &mut Context<Self>,
    ) {
        let session = session.into();
        if self.session == session && self.items == items {
            return;
        }
        self.session = session;
        self.items = items;
        cx.notify();
    }

    pub fn set_live(&mut self, live: bool, cx: &mut Context<Self>) {
        if self.live != live {
            self.live = live;
            cx.notify();
        }
    }

    pub fn session(&self) -> &str {
        &self.session
    }

    pub fn items(&self) -> &[TodoItem] {
        &self.items
    }

    /// Whether the panel draws anything: an empty plan hides it.
    pub fn is_visible(&self) -> bool {
        !self.items.is_empty()
    }

    pub fn summary(&self) -> TodoSummary {
        TodoSummary::of(&self.items)
    }

    /// Whether the current session's list is open.
    pub fn is_expanded(&self) -> bool {
        self.is_expanded_for(&self.session)
    }

    /// Whether `session`'s list is open, judged against the list now shown.
    pub fn is_expanded_for(&self, session: &str) -> bool {
        let finished = self.summary().finished();
        self.expanded.get(session).copied().unwrap_or(!finished)
    }

    /// Flips the current session's list between open and collapsed.
    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        let open = !self.is_expanded();
        self.expanded.insert(self.session.clone(), open);
        cx.notify();
    }

    fn header(&self, expanded: bool, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let summary = self.summary();
        let label = if expanded {
            "Collapse todo list"
        } else {
            "Expand todo list"
        };
        let headline: Option<(SharedString, gpui::Hsla)> = if summary.finished() {
            Some(("All done".into(), theme.editor.text_3.into()))
        } else {
            summary.headline().map(|ix| {
                (
                    self.items[ix].text.clone().into(),
                    theme.editor.text_2.into(),
                )
            })
        };
        let lead = if summary.finished() {
            icon(Icon::Check, 14., theme.green_11)
        } else {
            icon(Icon::Layers, 14., theme.editor.text_2)
        };

        let toggle = div()
            .id("todo-panel-toggle")
            .debug_selector(|| "todo-panel-toggle".into())
            .role(Role::Button)
            .aria_label(label)
            .w_full()
            .h(px(HEADER_HEIGHT))
            .px(px(ROW_PAD_X))
            .rounded(px(6.))
            .flex()
            .items_center()
            .gap(px(8.))
            .cursor_pointer()
            .hover({
                let hover = theme.editor.ctl_hover;
                move |style| style.bg(hover)
            })
            .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
            .child(lead)
            .child(
                text_style(
                    div().flex_none().flex().items_baseline().gap(px(6.)),
                    TextStyle::Label,
                )
                .child(div().text_color(theme.editor.text_1).child("Todo"))
                .child(
                    div()
                        .debug_selector(|| "todo-panel-count".into())
                        .text_color(theme.editor.text_3)
                        .child(SharedString::from(summary.label())),
                ),
            )
            // The expanded list already names every item; the headline is the
            // collapsed state's whole point.
            .when(!expanded, |row| {
                row.when_some(headline, |row, (text, color)| {
                    row.child(
                        text_style(div().flex_1().min_w_0().truncate(), TextStyle::Label)
                            .font_weight(FontWeight::NORMAL)
                            .text_color(color)
                            .child(text),
                    )
                })
            })
            .when(expanded, |row| row.child(div().flex_1()))
            .child(icon(
                if expanded {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                },
                13.,
                theme.editor.text_3,
            ));
        focusable(toggle, theme, false).into_any_element()
    }

    fn item_row(&self, ix: usize, item: &TodoItem, theme: &Theme) -> AnyElement {
        let (color, weight) = match item.status {
            TodoStatus::Completed => (theme.editor.text_3, FontWeight::NORMAL),
            TodoStatus::InProgress => (theme.editor.text_1, FontWeight::MEDIUM),
            TodoStatus::Pending => (theme.editor.text_2, FontWeight::NORMAL),
        };
        let text = text_style(
            div().flex_1().min_w_0().line_clamp(ITEM_MAX_LINES),
            TextStyle::Label,
        )
        .font_weight(weight)
        .text_color(color)
        .when(item.status == TodoStatus::Completed, |el| el.line_through())
        .child(SharedString::from(item.text.clone()));
        div()
            .id(("todo-item", ix))
            .debug_selector(move || format!("todo-item-{ix}"))
            .px(px(ROW_PAD_X))
            .py(px(4.))
            .flex()
            .items_start()
            .gap(px(8.))
            .child(
                div()
                    .w(px(GLYPH_SLOT))
                    .h(px(TextStyle::Label.line_height()))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.status_glyph(ix, item.status, theme)),
            )
            .child(text)
            .into_any_element()
    }

    /// Check, spinner (or a still ring when the agent is idle), empty ring.
    fn status_glyph(&self, ix: usize, status: TodoStatus, theme: &Theme) -> AnyElement {
        match status {
            TodoStatus::Completed => icon(Icon::Check, 12., theme.green_11).into_any_element(),
            TodoStatus::InProgress if self.live => icon(Icon::RefreshCw, 12., theme.editor.accent)
                .with_animation(
                    ("todo-spin", ix),
                    Animation::new(SPIN_PERIOD).repeat(),
                    |glyph, t| glyph.with_transformation(Transformation::rotate(radians(t * TAU))),
                )
                .into_any_element(),
            TodoStatus::InProgress => div()
                .size(px(10.))
                .rounded_full()
                .border_1()
                .border_color(theme.editor.accent)
                .flex()
                .items_center()
                .justify_center()
                .child(div().size(px(4.)).rounded_full().bg(theme.editor.accent))
                .into_any_element(),
            TodoStatus::Pending => div()
                .size(px(10.))
                .rounded_full()
                .border_1()
                .border_color(theme.editor.text_3)
                .into_any_element(),
        }
    }
}

impl Render for TodoPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_visible() {
            return Empty.into_any_element();
        }
        let theme = Theme::for_window(window, cx);
        let expanded = self.is_expanded();
        div()
            .id("todo-panel")
            .debug_selector(|| "todo-panel".into())
            .w_full()
            .p(px(2.))
            .rounded(px(8.))
            .border_1()
            .border_color(theme.editor.line)
            .bg(theme.editor.card)
            .flex()
            .flex_col()
            .child(self.header(expanded, &theme, cx))
            .when(expanded, |panel| {
                panel.child(
                    div()
                        .id("todo-panel-rows")
                        .max_h(px(LIST_MAX_HEIGHT))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .children(
                            self.items
                                .iter()
                                .enumerate()
                                .map(|(ix, item)| self.item_row(ix, item, &theme)),
                        ),
                )
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};

    use super::*;

    #[test]
    fn an_engine_plan_entry_becomes_the_same_item() {
        use studio_engine::agent::{PlanEntry, PlanStatus};
        let entry = |text: &str, status| PlanEntry {
            text: text.into(),
            status,
        };
        let items: Vec<TodoItem> = [
            entry("a", PlanStatus::Completed),
            entry("b", PlanStatus::InProgress),
            entry("c", PlanStatus::Pending),
        ]
        .into_iter()
        .map(TodoItem::from)
        .collect();
        assert_eq!(
            items,
            vec![
                TodoItem::new("a", TodoStatus::Completed),
                TodoItem::new("b", TodoStatus::InProgress),
                TodoItem::new("c", TodoStatus::Pending),
            ]
        );
    }

    /// x = completed, > = in progress, . = pending
    fn items(spec: &str) -> Vec<TodoItem> {
        spec.chars()
            .enumerate()
            .map(|(i, c)| {
                TodoItem::new(
                    format!("item {i}"),
                    match c {
                        'x' => TodoStatus::Completed,
                        '>' => TodoStatus::InProgress,
                        _ => TodoStatus::Pending,
                    },
                )
            })
            .collect()
    }

    fn panel(cx: &mut TestAppContext) -> (Entity<TodoPanel>, &mut VisualTestContext) {
        cx.add_window_view(|_, _| TodoPanel::new())
    }

    fn set(
        panel: &Entity<TodoPanel>,
        session: &str,
        list: Vec<TodoItem>,
        cx: &mut VisualTestContext,
    ) {
        panel.update(cx, |panel, cx| panel.set_plan(session.to_owned(), list, cx));
        cx.run_until_parked();
    }

    fn click(selector: &'static str, cx: &mut VisualTestContext) {
        let bounds = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} is not on screen"));
        cx.simulate_click(bounds.center(), Modifiers::none());
        cx.run_until_parked();
    }

    fn shown_items(cx: &mut VisualTestContext, count: usize) -> usize {
        const NAMES: [&str; 8] = [
            "todo-item-0",
            "todo-item-1",
            "todo-item-2",
            "todo-item-3",
            "todo-item-4",
            "todo-item-5",
            "todo-item-6",
            "todo-item-7",
        ];
        NAMES[..count]
            .iter()
            .filter(|name| cx.debug_bounds(name).is_some())
            .count()
    }

    #[test]
    fn summary_counts_and_names_the_current_item() {
        let s = TodoSummary::of(&items("xx>.."));
        assert_eq!(
            (s.total, s.done, s.active, s.next),
            (5, 2, Some(2), Some(2))
        );
        assert_eq!(s.label(), "2/5");
        assert_eq!(s.headline(), Some(2));
        assert!(!s.finished());
        // Nothing started: the first unfinished item is the headline.
        let s = TodoSummary::of(&items("x..."));
        assert_eq!((s.active, s.headline()), (None, Some(1)));
        // An in-progress item later in the list wins over an earlier pending
        // one: the agent is working on it.
        assert_eq!(TodoSummary::of(&items(".>.")).headline(), Some(1));
        let s = TodoSummary::of(&items("xxx"));
        assert!(s.finished());
        assert_eq!(s.headline(), None);
        assert!(!TodoSummary::of(&[]).finished());
    }

    #[gpui::test]
    fn an_empty_plan_hides_the_panel(cx: &mut TestAppContext) {
        let (panel, cx) = panel(cx);
        assert!(!panel.read_with(cx, |p, _| p.is_visible()));
        assert!(cx.debug_bounds("todo-panel").is_none());

        set(&panel, "a", items("x.."), cx);
        assert!(cx.debug_bounds("todo-panel").is_some());

        // The agent clearing its list takes the panel away again.
        set(&panel, "a", Vec::new(), cx);
        assert!(!panel.read_with(cx, |p, _| p.is_visible()));
        assert!(cx.debug_bounds("todo-panel").is_none());
        assert!(cx.debug_bounds("todo-panel-toggle").is_none());
    }

    #[gpui::test]
    fn an_update_replaces_the_list(cx: &mut TestAppContext) {
        let (panel, cx) = panel(cx);
        set(&panel, "a", items("x>."), cx);
        assert_eq!(shown_items(cx, 6), 3);

        set(&panel, "a", items("xx>..."), cx);
        assert_eq!(
            panel.read_with(cx, |p, _| p.items().to_vec()),
            items("xx>...")
        );
        assert_eq!(shown_items(cx, 6), 6);

        // A shorter list leaves nothing of the longer one behind.
        set(&panel, "a", items(">"), cx);
        assert_eq!(shown_items(cx, 6), 1);
        assert_eq!(panel.read_with(cx, |p, _| p.summary().label()), "0/1");
    }

    #[gpui::test]
    fn the_collapsed_summary_reads_completed_over_total(cx: &mut TestAppContext) {
        let (panel, cx) = panel(cx);
        set(&panel, "a", items("xx>.."), cx);
        // Open while work remains.
        assert!(panel.read_with(cx, |p, _| p.is_expanded()));
        assert_eq!(shown_items(cx, 5), 5);

        click("todo-panel-toggle", cx);
        assert!(!panel.read_with(cx, |p, _| p.is_expanded()));
        assert_eq!(shown_items(cx, 5), 0);
        assert!(cx.debug_bounds("todo-panel-count").is_some());
        assert_eq!(panel.read_with(cx, |p, _| p.summary().label()), "2/5");

        // The count follows the list while collapsed.
        set(&panel, "a", items("xxx>."), cx);
        assert_eq!(panel.read_with(cx, |p, _| p.summary().label()), "3/5");
        assert!(!panel.read_with(cx, |p, _| p.is_expanded()));
    }

    #[gpui::test]
    fn the_collapse_choice_is_kept_per_session_across_updates(cx: &mut TestAppContext) {
        let (panel, cx) = panel(cx);
        set(&panel, "a", items("x.."), cx);
        click("todo-panel-toggle", cx);
        assert!(!panel.read_with(cx, |p, _| p.is_expanded_for("a")));

        // An update of the same session keeps the choice.
        set(&panel, "a", items("xx."), cx);
        assert!(!panel.read_with(cx, |p, _| p.is_expanded()));

        // Another session starts from its own default and keeps its own choice.
        set(&panel, "b", items("x.."), cx);
        assert!(panel.read_with(cx, |p, _| p.is_expanded()));
        assert_eq!(shown_items(cx, 3), 3);

        // Back on the first session its choice is still there.
        set(&panel, "a", items("xx."), cx);
        assert!(!panel.read_with(cx, |p, _| p.is_expanded()));
        assert_eq!(shown_items(cx, 3), 0);
        assert!(panel.read_with(cx, |p, _| p.is_expanded_for("b")));

        // Reopening it by hand sticks too.
        click("todo-panel-toggle", cx);
        assert!(panel.read_with(cx, |p, _| p.is_expanded()));
        set(&panel, "a", items("xxx"), cx);
        assert!(panel.read_with(cx, |p, _| p.is_expanded()));
    }

    #[gpui::test]
    fn a_finished_list_defaults_to_compact(cx: &mut TestAppContext) {
        let (panel, cx) = panel(cx);
        set(&panel, "a", items("xxx"), cx);
        assert!(!panel.read_with(cx, |p, _| p.is_expanded()));
        assert!(panel.read_with(cx, |p, _| p.summary().finished()));
        assert_eq!(shown_items(cx, 3), 0);
        // New work reopens it.
        set(&panel, "a", items("xxx."), cx);
        assert!(panel.read_with(cx, |p, _| p.is_expanded()));
    }

    #[gpui::test]
    fn the_list_renders_while_live_and_idle(cx: &mut TestAppContext) {
        let (panel, cx) = panel(cx);
        set(&panel, "a", items("x>."), cx);
        panel.update(cx, |p, cx| p.set_live(true, cx));
        cx.run_until_parked();
        assert_eq!(shown_items(cx, 3), 3);
        panel.update(cx, |p, cx| p.set_live(false, cx));
        cx.run_until_parked();
        assert_eq!(shown_items(cx, 3), 3);
    }
}
