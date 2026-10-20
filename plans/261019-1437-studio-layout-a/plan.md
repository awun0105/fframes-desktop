# Studio layout A: rail, preview first, agent on the right

Status: implemented, uncommitted · Branch: main · Scope chosen by the user: layout only, existing kit and Cap tokens.

## Outcome

The Studio window uses layout A from `.amp/in/artifacts/studio-layout-mock.html`:

```
header (unchanged, caption buttons on Windows)
notices (unchanged)
[rail 56] [drawer 320, when open] [player over timeline] [grip] [agent column 360]
status bar (source · checkpoint · job, details toggle)
```

## Decisions

- Left rail: vertical icon + label buttons for Project, Assets and Style. Clicking the open
  tab closes the drawer; the drawer starts closed so the preview gets the width. The Agent
  tab goes away: the agent is always visible.
- The drawer pushes the editor instead of overlaying it, so it never covers the canvas
  selection surface and needs no z-ordering.
- The agent column is resizable from a grip on its left edge (`agent-resize-handle`),
  360 px default, clamped to 320-520 px and so the editor keeps 480 px; not saved between
  launches, like the timeline split.
- The status strip above the editor becomes a bottom status bar with the same summary and a
  details toggle; errors still expand it. Ids `status-strip`, `status-details` stay.
- `FocusPrompt` only focuses the composer.
- No project open: same rail and agent column, the empty state in the middle (SDK actions
  in the Project drawer stay reachable).
- Cached sections: `Section::Sidebar` ("sidebar") becomes `Section::Rail` ("rail"), the rail
  plus the open drawer, and the agent column is a cached `Section::Agent` ("agent-column")
  so playback does not re-render the transcript. The panel sits flush in its card (it pads
  its own rows), which keeps its header on one line at 320 px.
- Narrow windows: the agent keeps 320-520 px and leaves the editor 480 px; when both
  minimums do not fit they shrink in proportion (`agent_width`, unit-tested at 800, 1280,
  1536 px with the drawer open and closed).

## Non-goals

New palette or typography, agent card restyle, player toolbar, header changes, docking.

## Steps

1. `studio-theme` metrics: replace `left_column_w` with `rail_w`, `drawer_w`,
   `agent_column_w` (+ min/max), `editor_min_w`, `status_bar_h`.
2. `shell/left_column.rs`: rail + drawer; `LeftTab` without Agent, `Option<LeftTab>` state.
3. `shell/layout.rs`: new composition, agent grip and pointer tracking, pure
   `agent_width(available, preferred)` clamp with unit tests.
4. `shell/status_strip.rs`: bottom bar.
5. `shell/sections.rs`, `keymap.rs`, `mod.rs`, `observation.rs`: renames and state.
6. Parity tests (`window.rs`, `timeline.rs`, `transport.rs`): new geometry, rail toggle,
   agent resize.
7. Docs: `docs/desktop/design-system.md`, `docs/desktop/README.md`, architecture mentions.

## Acceptance

- `cargo fmt`, `cargo clippy -D warnings`, `cargo test -p studio-theme -p studio-ui`
  (parity suite included) pass on Windows.
- Parity: rail left of the player, agent column right of it and full workspace height, drawer
  opens and closes from the rail, agent grip clamps, header/rail/timeline still do not
  re-render while playing.
