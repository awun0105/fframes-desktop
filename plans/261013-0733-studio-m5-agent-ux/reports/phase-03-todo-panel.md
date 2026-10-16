# Todo panel (panel UI only)

File: `desktop/crates/studio-ui/src/agent/todo_panel.rs` (about 560 lines with tests). `agent/mod.rs` is untouched.

## API

- `TodoStatus { Pending, InProgress, Completed }` and `TodoItem { text: String, status: TodoStatus }` (with `TodoItem::new`). Same names as the driver's `PlanStatus` and `PlanEntry`.
- `TodoSummary::of(&[TodoItem])` gives `total`, `done`, `active`, `next`, `finished()`, `headline()` and `label()` (`"2/5"`).
- `TodoPanel` is a gpui entity that implements `Render`.
  - `TodoPanel::new()`
  - `set_plan(session: impl Into<SharedString>, items: Vec<TodoItem>, cx)` replaces the shown list, so an empty list hides the panel. It is a no-op when nothing changed.
  - `set_live(bool, cx)` makes the in-progress glyph spin (`Icon::RefreshCw` rotating). When not live it is a still ring.
  - `toggle(cx)`, `is_expanded()`, `is_expanded_for(session)`, `is_visible()`, `summary()`, `items()`, `session()`.
- Collapse state is a `HashMap<session, bool>` of explicit choices. Without a choice the panel is open while work remains and compact once every item is done. A choice survives `set_plan` updates and session switches.
- Rendering:
  - The header is a button showing `Todo 2/5`. When collapsed it also shows a truncated headline (the item in progress, else the next one, else "All done").
  - The expanded list scrolls past 220 px.
  - Rows show a check in the success colour for completed items, with muted strike-through text. In-progress rows are bold with the accent ring or spinner. Pending rows have an empty ring.
  - Item text wraps and is clamped to 3 lines, ending in an ellipsis.
  - Chrome comes from the Cap kit (`icon`, `focusable`) and `studio_theme` (`editor.*` tokens, `text_style`). The toggle is keyboard-focusable.

## Mapping from PlanEntry (host side, later)

```rust
impl From<PlanStatus> for TodoStatus { /* Pending, InProgress, Completed one to one */ }
impl From<PlanEntry> for TodoItem { fn from(e) -> Self { TodoItem { text: e.text, status: e.status.into() } } }
```
The `From` impls belong where both types are visible. studio-ui does not depend on studio-harness for this. The host calls `panel.set_plan(session_key, entries.into_iter().map(Into::into).collect(), cx)` on each snapshot, and `set_live` from the run state. `WorkflowSnapshot` was not touched.

## Dropped from Zeron

- Frost, the queue-tray surface, the composer overlap and the stacked inset: replaced by a plain card.
- `edge_fade` and the fade-in animation: not kept (the list scrolls without edge fades).
- Folding of long lists to a 3-item focus window with "N earlier / N later" rows: the list scrolls instead.
- Per-list dismiss button and signature, and the "tidy once when settled" reset: not in the requirements.
- Transcript scan (`latest_todo`, `TodoCache`) and the `Composer` hooks: the host passes the list in.
- Tooltips on the buttons: aria labels only.
- The `mini_glyph_spinner` loader: the kit has no spinner (`CircularProgress` does not animate), so a rotating `RefreshCw` is used.

## Tests (`cargo test -p studio-ui --lib agent::todo_panel`: 7 pass)

- Pure: summary counts, headline choice and the `2/5` label.
- gpui, using debug selectors and a real click on the toggle:
  - an empty plan hides the panel, and clearing the list removes it;
  - an update replaces the list, including to a shorter one;
  - the collapsed header shows `completed/total` and follows updates;
  - the collapse choice survives same-session updates, is separate per session, and returns on switching back;
  - a finished list defaults to compact and new work reopens it;
  - render while live and idle (smoke test only; it does not assert that the spinner is animating).

Verified: `cargo clippy -p studio-ui --all-targets -- -D warnings` is clean. rustfmt was run on this file only. Nothing was committed or staged.

Status: DONE
Summary: Standalone `TodoPanel` entity and `TodoItem`/`TodoStatus` view types in `agent/todo_panel.rs`, with 7 passing tests and clean clippy.
