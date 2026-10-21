# Phase 2: Shell, player and sidebar row over a full-width timeline

## Context

- Cap's composition is in `editor_window.rs:11378-11428`: the root column holds the header,
  then a row with `px(8)` holding the player card (`flex_1`) and the sidebar (`pl(8)`,
  416 px), then `gap(8)` and the timeline card across the full width with `px(8)` and the
  timeline height. Vertical limits are in `:138-143` and `EditorVerticalLayout::new` (`:185`).
- Studio composes `[rail][drawer][editor: player over timeline][agent]` in
  `shell/layout.rs:280-400`. `measure` reads the editor column height for `vertical_split`.
  `agent_width(available, drawer_open, preferred)` clamps the agent column.

## Steps

1. `layout.rs`: the workspace becomes a column:
   - a top row: the player card (`flex_1`, `min_w(editor_min_w)`) and the sidebar card
     (`sidebar_column_width()`, with the `agent-resize-handle` grip on its left edge);
   - the `timeline-resize-handle` grip;
   - the timeline card across the full width, with the playhead overlay.
   The `measure` canvas moves to the workspace column, so `vertical_split` splits the whole
   height. The empty state (no project) takes the player card's place and the sidebar stays.
2. `agent_width` loses its `drawer_open` parameter (rename to `sidebar_width`); update its
   unit tests at 800, 1280 and 1536 px.
3. `sections.rs`: `Section::Rail` goes away. The sidebar host is uncached and holds the tab
   rail plus either the cached `Section::Agent` or a cached `Section::Drawer` for the
   selected tab, never both in one cached view. Settings still draw the agent entity, so
   the sidebar must not render it while settings are open. The `agent-column` probe goes on
   the uncached sidebar host, so it is still measured while Project, Assets or Style is
   selected (`parity/transport.rs:45`, `parity/window.rs`).
4. `mod.rs` state: `drawer: Option<LeftTab>` becomes `sidebar_tab: SidebarTab` (`Agent` by
   default). `observation.rs` reports the selected tab under the `drawer` key (`agent` when
   the agent is shown) and keeps the `split` key.
5. `keymap.rs`: `FocusPrompt` selects `SidebarTab::Agent`, then focuses the composer.

## Validation

`cargo test -p studio-ui shell::layout`; the app starts and both grips work.

## Risk

The parity tests in `window.rs:155-216` read `drawer` and `split`; phase 6 updates their
expectations.
