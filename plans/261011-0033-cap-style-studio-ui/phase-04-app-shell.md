# Phase 4: App shell (frame, header, notices, status strip, empty state)

Status: done (see reports/fullstack-developer-261012-0602-phase-04.md). Depends on: Phase 3. Effort: 2-3 days.

## Goal

Rebuild the window frame of `StudioShell::render` (`desktop/app/src/studio_shell.rs:2603-3500`) into Cap's card layout: header, left column with icon rail, player area, agent column, notices instead of coloured banners, a collapsible status strip, and a proper empty state. Region contents (player, timeline, left-column lists, agent panel internals) are only moved here; Phases 5 to 8 restyle them.

## Target layout

```
window bg = window; 8 px gutter around and between cards
+------------------------------------------------------------------------+
| header card, h 52: [app icon] Project name / path      [Create][Open]  |
|                                        [Import][Export MP4][Close]     |
+------------------------------------------------------------------------+
| notices (0..n, full width): awaiting preview, error, export, recovery  |
| status strip (collapsible, one line when collapsed)                    |
+-------------+------------------------------------+---------------------+
| left column | player card (Phase 5)              | agent column 400    |
| 248 wide    |                                    | (Phase 8 internals) |
| tab rail 46 +------------------------------------+                     |
| content     | timeline card (Phase 6), h 260     |                     |
+-------------+------------------------------------+---------------------+
```

The left column is always visible at 248 px (user decision). The window still opens at 1280 x 800 (`main.rs:151`); at that size the player card is about 600 px wide, so the transport uses its compact layout (Phase 5).

## Element map (current to new)

| Current element (`studio_shell.rs`) | New place and component |
| --- | --- |
| Header row `:2736-2798`, bg `palette.surface`, `border_b` | Header card h 52; title `TextStyle::Title`, path `Small` `text_2` truncated with tooltip |
| `create` "Create…", `open` "Open…", `import` "Import Rust…" | Header right, `Button` `Editor` with icons Plus, FolderOpen, Upload; same ids and labels |
| `export-mp4` "Export MP4…" | Header right, `Button` `Primary` with Download icon; same id, label and enable rule |
| `close` "Close project" (telemetry id) | Header right, `Button` `Editor`; must keep its probe |
| `awaiting-preview` banner `:3443-3453` | `Notice` Warning, under the header; keep id `awaiting-preview` on the notice root |
| `error` banner `:3455-3468` (max h 120, scroll) | `Notice` Danger, keep id `error`; first line as title, the full text inside collapsible details (scrollable, max h 200) |
| `independent` "Use this folder as an independent project" `:3469-3482` | Action button on the error `Notice`, same id and condition |
| Export status row `:2903-2930` and `cancel-export` | `Notice` Info with progress text and the `cancel-export` button as its action |
| Interrupted and recovery lines `:2942-2956` | `Notice` Warning |
| Status lines `:2873-2901` (source, checkpoint, job; checkpoint match; history; worker bridge) | Status strip: collapsed shows `Source abc123 · Checkpoint def456 · Job <state>` plus a StatusDot (green when source matches the checkpoint, amber otherwise); expanded shows the other three lines; expanded automatically while a job runs or an error exists |
| Navigation column `:2799-2861` (`Project`, `Assets`, `Style` buttons, inventory, `asset` "Copy asset…") | Left column with `TabRail` (Phase 7 fills the content); tab ids stay `Project`, `Assets`, `Style` |
| Library block `:3331-3420` (Managed SDK, Recent projects, recents) | Moves into the Project tab (Phase 7) and the empty state below |
| Centre column `:2862-3328` | Player card and timeline card (Phases 5 and 6) |
| Preset panel shown when `navigation == "Style"` `:3326-3328` | Moves into the left column Style tab (Phase 7) |
| Agent panel column `:3490-3496` | Agent column card, w `agent_panel_w`, `card` fill, radius 12; internals in Phase 8 |
| Root `:3424-3442` (bg `BACKGROUND`, Tab and Shift-Tab focus cycling) | Root bg `window`; keep the focus cycling handlers |

## Empty state (no project open)

The player and timeline area shows one centred card (max width 520): `Heading` "fframes Studio", a `Body` `text_2` line ("A native workspace for your video projects", today's subtitle), the three primary actions (Create, Open, Import Rust, same ids as the header, so render them only once: hide them from the header while the empty state shows), and a "Recent projects" `SectionTitle` with the recents list (same `recent-{id}`, `locate-{id}`, `remove-{id}` buttons as today). The left column stays available.

Check: rendering header actions only once keeps unique element ids. If any test clicks `open` or `create` through telemetry while no project is open, keep them in the header instead (they are not in the telemetry list today).

## Steps

0. Confirm Cap's main card radius, border and gap by reading its `EditorSectionView` rendering in `~/code/Cap/apps/desktop-gpui/src/editor_window.rs` (around `EditorSection`, lines 704-773). If it differs from 12, update `Metrics::radius_card` and the spec. Read only to learn the values.
1. Introduce small private render functions in `studio_shell.rs` for each region (`render_header`, `render_notices`, `render_status_strip`, `render_left_column`, `render_player_area`, `render_agent_column`, `render_empty_state`). Move code first without visual change; tests pass. Commit point. Only create `studio_shell/` submodules if a function exceeds about 200 lines after the move.
2. Replace `navigation: &'static str` with `enum LeftTab { Project, Assets, Style }`; keep the tab element ids as today's labels.
3. Header, notices and status strip with Phase 3 components.
4. Card layout with the 8 px gutter; the left column is fixed at `left_column_w` (248).
5. Empty state.
6. Keep all telemetry: `close` probe; `start_qualification` (`:976`) and `start_agent_telemetry` (`:1032`) snapshot keys unchanged; `canvas_origin`, `canvas_extent`, `preview_extent` still written by the preview probe (Phase 5).

## Tests

- Existing: `cargo test -p fframes-studio` (unit and integration tests that do not need an SDK); `tests/promotion_handoff.rs` and `tests/agent_workflow_ui.rs` exercise shell state.
- New pure-function tests (no element-tree rendering; `TestAppContext` is unavailable): `status_strip_expanded(state)` on error and running job, `header_actions(state) -> Vec<&'static str>` (used by render and test) hides create, open and import while the empty state shows.
- Native (Linux with Xvfb, 1280 x 800): `cargo test -p fframes-studio --test x11_shell -- --ignored` if it is ignored; otherwise as-is. Record if not run.

## Validation

On the user's machine at 1280 x 800, 1440 x 900 and 1920 x 1080, both appearances: open and close a project, trigger an error (invalid project folder) and expand its details, start an export and cancel it, and switch the left tabs with mouse and keyboard.

## Done when

The frame matches the target layout, every element in the map is placed as listed with its id unchanged, and telemetry output is identical apart from coordinates.

## Risks and rollback

Ids rendered twice (header and empty state) would break lookups: render once. Rollback: revert this phase; tokens and components remain.
