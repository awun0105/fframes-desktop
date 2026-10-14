---
phase: 4
title: "Split the studio shell"
status: pending
priority: P1
effort: "6-9d"
dependencies: [2, 3]
---

# Phase 4: Split the studio shell

## Goal

`studio_shell.rs` (4,696 lines) and the remaining views move into `studio-ui` modules laid out like Cap's editor window, none over 1,500 lines, and the parity suite moves with them.

Effort was 5-7d before [spike S4](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md): only Cap's chrome ports; every `render_*` and the panels, sidebar and settings pages are rewritten against snapshots.

## Tasks

1. Scout `studio_shell.rs` sections: layout, header, transport, left column, status strip, player hosting, keyboard handling, and the qualification part (around lines 1021-1270, removed in phase 7).
2. Create `studio-ui/src/shell/{mod,layout,header,transport,left_column,status_strip,keymap}.rs` and `studio-ui/src/player/mod.rs`; move code section by section, one commit each. Leave the qualification hooks where they are; phase 7 deletes them.
3. Wire keyboard input through gpui actions and one keymap, reusing the actions added in M1 phase 3.
3a. Cap's editor layout: port the window shell from Cap `editor_window.rs` (12,661 lines; only the shell, header, toolbar and section mounting, not recording or clip editing), `editor_panels.rs` (9,027) and `editor_sidebar.rs` (4,652) into `studio-ui/src/shell/` and `studio-ui/src/panels/`. Map Cap's sections: header to the project header and transport; sidebar to the left column (presets, project, scenes); preview to the player; panels to the inspector; the agent panel is a sidebar tab next to Presets and Scenes (phase 6); timeline section to the timeline. Each section is an `EditorSectionView`-style entity. Cap types are replaced by `studio-proto` snapshots per the S4 closure.
3b. Window chrome per OS from Cap's `main.rs` options: on Windows the custom frame with Cap's `windows_caption` (patched insets, `start_window_move`) and an opaque background; on macOS a hidden titlebar with an opaque background (no transparency, floating level or glass); on Linux client decorations.
3c. Setup screen: build `setup_view` on Cap's settings page frame (`settings_window.rs` 4,905 lines, `settings_pages.rs` 8,414): page rail (`TabRail`), `::settings` components and `field` rows. Port the frame and row patterns only; the fframes pages are adapter setup here and provider profiles in phase 6.
4. Move `timeline_view.rs`, `preset_panel.rs`, `setup_view.rs`, `project_view.rs`, `canvas_view.rs`, `preview_element.rs`, `frame_image.rs` (gpui half), `text_input.rs` into `studio-ui` modules.
5. Move the parity suite to `desktop/crates/studio-ui/tests/parity/` and switch merge gate 3 to `cargo test -p studio-ui --test parity` in CI. Done: CI already runs every `-p studio-ui` test, so the suite runs there without a workflow change.

## Reference patterns (Cap, Zeron)

From [the Zeron report](../reports/research-261013-1749-zeron-agent-control.md) and [the Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md); see the roadmap's pattern map.

1. App bootstrap in `desktop/app/src/main.rs`, in Cap's order (`main.rs:242-560`): `gpui_platform::application().with_assets(..)` with icons and fonts embedded through an `AssetSource` and fonts registered with `text_system().add_fonts` (if fframes loads them from disk today, embed them this way); inside `run`, `gpui_tokio::init_from_handle`, the text-input key bindings and the theme global; then `open_window` with per-OS options; native window work only after the handle exists, from `cx.spawn`.
2. Never change native window state inside a gpui update: AppKit and Win32 callbacks re-enter `App` and panic with "RefCell already borrowed". Take the window handle in the update and act from a spawned task.
3. Keymap by key context, most specific first: text input, composer, timeline and player, workspace. Every editing chord (Backspace, Delete, Ctrl/Cmd+Z, Ctrl/Cmd+A, clipboard) is bound in the text-input context (`key_context` already exists in `text_input.rs`), so a focused field consumes them before timeline shortcuts. Test: Ctrl+Z in a focused text field does not reach timeline undo.
4. Player hosting: the preview is its own child entity in `studio-ui/src/player/`, painting through `canvas`, and the only view notified per frame (done as a player section view whose state stays in the shell; see the design decisions). Header, left column, status strip and timeline are separate entities mounted with `.cached(..)`. No per-frame `window.refresh()` (Cap measured 224 of 298 frames dropped with it, `editor_window.rs:6354`). Keep releasing replaced frames with `drop_image` (`frame_image.rs` does this today). Cap's fork provides `.cached(..)` (Cap mounts its editor sections this way).

## Todo

- [ ] Bootstrap order, native-call rule, key contexts and cached sections applied

- [x] Scout sections
- [x] Shell modules
- [x] Actions and keymap
- [x] Remaining views moved
- [x] Parity suite moved and CI updated

## Design decisions (2026-10-14, after scouting)

Scouting found three facts that change the task list:
- `studio_shell.rs` is 4,313 lines. Its render code is already split by section: header, notices and status strip, left column, player and stage, transport and overflow, root layout. Engine events, transport state and qualification share one `StudioShell` struct of about 75 fields.
- `project_view.rs` and an app `text_input.rs` do not exist; the text input moved to the kit in phase 2. `preview_element.rs` is used only by the spike app (`app.rs`), not by the shell, so it stays in the app with the spike.
- The parity harness reads `StudioShell::telemetry` and calls `start_qualification`. Those methods move with the shell.

The decisions:
- **Crate boundary.** `studio-ui` gains the dependencies the views use:
  - `studio-project`, `studio-bootstrap`, `studio-sdk`, `studio-harness`, `fframes-studio-protocol`;
  - `image`, `parking_lot`, `serde_json`, `thiserror`.

  The app keeps `main.rs`, the bins, the spike (`app.rs`, `preview_element.rs`, `agent_spike.rs`, `selection_spike.rs`, `presentation_stress.rs`, `stress_metrics.rs`) and `assets.rs`.
- **The conversation panel moves too.** The shell owns `Entity<ConversationPanel>`, subscribes to its `PanelEvent`s and uses `Attachment`, and `studio-ui` cannot import from the app. So `conversation_panel/` moves into `studio-ui/src/panels/conversation/` before the shell does.
  - It is a pure move: the panel's only app-crate import is `ui::BoundsProbe` (`conversation_panel.rs:88`), which moves as well, and its submodules have none.
  - The success criterion's "`conversation_panel*`, which phase 6 moves" is an allowance. Phase 6 starts from the new path.
  - Injecting the panel from the app as an `AnyView` with an event bridge was rejected: it rewrites the shell's agent code in a phase that only moves code.
- **The shell stays one entity in this phase.** `StudioShell` keeps its state. Its methods are split across `impl StudioShell` blocks in section modules: `studio-ui/src/shell/{mod,header,status_strip,left_column,player,transport,overflow,keymap,layout,engine_events,qualification}.rs`. This is a pure move, with no change in behaviour, and every file stays under 1,500 lines. The free helpers and their 11 unit tests move with their sections.
- **Qualification code.** The shell's qualification code moves into `studio-ui/src/shell/qualification.rs`, alone in one file. `BoundsProbe` stays in `studio-ui/src/probe.rs`: the conversation panel and the timeline use it too, and folding it into the shell would make them import from the shell. Phase 7 deletes both modules. The success criterion's "app holds the qualification code" is an allowance, not a requirement.
- **Order of commits.** Each is behaviour-preserving, with parity green:
  1. Add the crate dependencies, and move into `studio-ui`, one commit each:
     - `frame_image` (the gpui half), `canvas_view`, `actions` and `ui/probe.rs`;
     - `preset_panel`, `setup_view` and `timeline_view`;
     - `conversation_panel/`.

     The app re-exported them for its tests until the parity move; since then the app and its tests import them from `studio-ui` and the re-exports are gone. `probe.rs` lands at `studio-ui/src/probe.rs` and stays there (see Qualification code). The views keep their own telemetry methods, which phase 7 removes as well: `TimelineView::qualification_metrics`, `PresetPanel::telemetry` and the conversation panel's telemetry.
  2. Move `studio_shell.rs` whole into `studio-ui/src/shell/mod.rs`, then split it section by section, one commit each.
     The sections, in commit order (done 2026-10-14; `mod.rs` is 484 lines and the largest section, `player.rs`, 939):
     - `transport.rs`: `render_transport`, the transport mode and clock helpers;
     - `header.rs`: the header and `header_actions`;
     - `overflow.rs`: the overflow menu, `OverflowAction` and its enable rules. Its trigger sits in the transport bar, so it is not part of the header;
     - `status_strip.rs`: notices, the status strip and its helpers;
     - `left_column.rs`: the tabs, project files, recents, the empty state, the agent column and `sdk_status_tone`;
     - `keymap.rs`: `on_actions`;
     - `qualification.rs`: telemetry, `start_qualification`, `start_agent_telemetry` and `bounds_probe`;
     - `player.rs`: preview, canvas pointer, audio and playback, the player area, toolbar and stage;
     - `engine_events.rs`: dispatch, engine events, presets, agent sync, opening, pickers and export, with `agent_build`, `close_workflow_detached` and `is_terminal`;
     - `layout.rs`: `impl Render`.

     `mod.rs` keeps the struct, the constructors and what two or more sections share: `Picker`, `PreparedInstall`, `AgentHost` and the `action_button`/`action_icon_button` pair. Unit tests move with the code they test. Methods called across sections are `pub(super)`; everything else stays private to its file.
  3. Move the parity suite to `studio-ui/tests/parity/`. Done 2026-10-14: the same 34 tests by name, now run by `cargo test -p studio-ui --test parity`. The workflows needed no change: `desktop.yml`, `desktop-self-hosted.yml` and `desktop-release.yml` run every test of `-p studio-ui`, which includes the suite.
  4. Keymap: one `bind_keys` with key contexts, most specific first: `TextInput`, then `Composer`, then `Timeline` and `Player`, then `Workspace`. The preview surface's raw `on_key_down` (Space, arrows, Home, End, `c`, `0`, Escape) becomes actions bound in the `Player` context. The root Tab handler stays. Test first: Ctrl+Z in a focused text field does not reach timeline undo, and Space on the preview still toggles playback.
     Done 2026-10-14. `shell::bind_keys` binds the text fields' keys and then the `Player` and `Timeline` keys; the harness and `main.rs` call it. Parity tests press every preview and timeline key as a real keystroke; they were written against the raw handlers first and fail when a binding is dropped. Recorded differences from the step as written:
     - There is no timeline undo, so the Ctrl+Z test pins that undo in the agent prompt undoes the prompt's own typing and leaves the playhead and the timeline selection alone.
     - `Composer` gets no context: the prompt's and the setup fields' Enter, Tab and Escape stay in their composition-aware key handlers, since an action binding would fire during an IME composition. `Workspace` is set on the window root with no bindings; a keymap unit test fails if a shell binding leaves `Player` or `Timeline`.
     - The timeline's keys now answer only without modifiers (and Shift+= for +); the raw handler ignored modifiers. Home and End on the preview still seek the shown preview: the preview surface handles `SeekToStart` and `SeekToEnd` itself, the window root sends them through the timeline.
     - New actions: `FocusPreview`, `FocusTimeline`, `CycleCanvasSelection`, `ClearCanvasSelection`, `ResetCanvasView`, `PageTimelineBack` and `PageTimelineForward`, all handled at the window root like the others.
  5. Cap's layout, a visible change:
     - a 52 px header;
     - a player card (44 px toolbar, stage, 48 px transport) beside a fixed sidebar;
     - a timeline card below with a vertical resize grip, clamped like Cap's `EditorVerticalLayout`;
     - the left column's tabs become the sidebar's tab rail;
     - header, sidebar, transport and timeline are mounted as `EditorSectionView`-style entities with `.cached(..)`;
     - the player is the only view notified per frame.

     Cap's sidebar is 416 px. fframes keeps the agent column, which moves into the sidebar in phase 6, so the sidebar width is decided against the 1,275 x 800 default window when this step starts. Element ids that qualification and parity read are kept.

     Geometry done 2026-10-14:
     - The body is the editor column beside the agent column, which stays at full height. The editor column holds the player beside the sidebar (the left column, moved to the player's right), above the timeline.
     - The grip `timeline-resize-handle` drags the timeline height. `vertical_split` clamps it like Cap's `EditorVerticalLayout`: 260 px default, 240 px timeline minimum, 320 px player minimum, both minimums scaled down together when they do not fit. The editor column measures its own height, so the clamp does not depend on Cap's header arithmetic.
     - The split lives in memory only.
     - While the grip is held, window-level mouse listeners follow the pointer past the window edge, where element listeners stop firing.
     - `main.rs` opens the window at 1,275 x 800.
     - The parity test `the_grip_resizes_the_timeline_within_its_limits` drags the grip by its reported bounds.

     Decided: the sidebar stays at 248 px until the agent column moves into it in phase 6, and switches to Cap's 416 px then. The left column keeps its tabs until then.

     Sections done 2026-10-14, as Cap does it, not by moving the player's state:
     - `shell/sections.rs` holds `SectionView`, after Cap's `EditorSectionView`. Each section observes the shell and renders the shell's own `render_*` method.
     - The header, sidebar and agent column are mounted `.cached(..)` with their fixed boxes. A cached view is laid out by its cached style alone, so the auto-height notices and status strip stay uncached in the root.
     - The player area (toolbar, stage, transport) is the uncached player section. The playback tick and arriving frames notify only that section, not the shell. Seeks, pauses and errors still notify the shell.
     - The conversation panel renders again only when its task scope changes; the scope is set with every presented frame.
     - The parity test `playback_renders_the_player_and_leaves_the_other_sections` reads per-section render counts from the telemetry. It fails if the tick, the frame news or the scope update notifies more than the player.
     - Deviation from the step as written: the player's state stays in `StudioShell`, and there is no `studio-ui/src/player/` module. The gain is in what is notified per frame, and moving the state (about 35 fields, shared with the engine events) is not needed for it.
  6. Window chrome:
     - On Windows, a custom frame with Cap's `windows_caption_controls` (114 lines, ported into the kit) and an opaque background. It closes the phase 2 native-caption gap, because the painted caption follows the theme.
     - On macOS, a hidden titlebar; on Linux, client decorations.
     - Cap's editor header draws `windows_caption_controls` at its right end (`editor_window.rs:9415`), with a drag area (`window_control_area(Drag)`, `:9265`). It switches to compact header controls on narrow windows when captions are shown (`compact_header_controls`, `:232`, tested at `:12192-12203`). fframes ports that header, its compact rule and the rule's tests.
     - This step is pushed on its own and waits for the self-hosted result, because the native harness drives this window on the VM.

     Done 2026-10-14:
     - The header is a flush row on every system (no card). `main.rs` sets `appears_transparent`, Cap's (20, 20) traffic-light inset and `app_owns_titlebar_drag`. Only the Windows and macOS backends of the fork read them.
     - Windows: the uncached row around the cached header is the `Drag` area, and `kit::windows_caption_controls` (Cap's look, glyphs and ids) sits at its right end. The header's action cluster occludes the drag area.
     - Deviation from Cap: the caption buttons are native window control areas (`Min`, `Max`, `Close`) without click handlers, as in Zed. The fork's Windows `zoom` only maximizes and never restores, and this way Windows handles the press itself, with Snap Layouts on Windows 11.
     - Why the row stays uncached: the fork's `reuse_prepaint` and `reuse_paint` do not copy `window_control_hitboxes`, so a control area inside a cached view stops answering once the view is reused. Cap's editor puts its drag area inside its cached header.
     - macOS: an 82 px spacer, and the empty middle of the header starts the window move (a double click zooms). Not verified here.
     - Linux keeps server decorations, as Cap's editor does (only Cap's main and panel windows use client decorations). This differs from "Linux client decorations" above. The phase 2 native-caption gap stays open on Linux.
     - The compact rule is fframes' own: Cap's action widths do not apply. The actions drop their labels when the title (200 px minimum), the labelled actions (600 px with a project open, 260 px without) and the window controls do not fit. The labelled actions measure 520 / 215 px in the Windows app and 566 / 246 px in the test platform. Close project has no icon and keeps its label.
     - Tests:
       - unit tests for the compact rule;
       - the parity test `the_sections_fill_their_boxes`: on Windows the header ends where the 138 px caption begins and the caption reaches the window's right edge; elsewhere the header spans the window;
       - the parity test `narrow_windows_show_the_header_actions_as_icons`: it resizes the window to 800 px and back. Create narrows and widens again, and Close project keeps its width. It fails when the rule is off, when Close loses its label, and when the viewport is read only once. The shell's `report_button_bounds` turns the button probes on for the harness.
     - Native check on the VM, with window messages because the remote session was disconnected:
       - `WM_NCHITTEST` answers `HTCAPTION` over the title and the empty middle, `HTCLIENT` over the header buttons and the body, and `HTMINBUTTON`, `HTMAXBUTTON` and `HTCLOSE` over the caption;
       - a double-click on the title maximizes and restores (`WM_NCLBUTTONDBLCLK` passes through gpui's non-client mouse-down handler first, so no listener swallows a press over the drag area);
       - maximize maximizes, the maximized client area starts at the work area and its top-right corner is Close;
       - restore restores, minimize minimizes, and Close quits with exit code 0.
     - Not checked with real input:
       - a real drag of the title;
       - Snap Layouts (Windows 11 only; the VM is Server 2022);
       - the negative control (a `Drag` area inside the cached header). Synthetic `WM_MOUSEMOVE` posts a mouse-leave each time, and the fork refreshes the whole window on hover-status changes, so the cached header is never reused under message input. The control needs real input in a connected session.
  7. Settings frame: `setup_view` on Cap's settings shell (page rail, `::settings` components, `setting_row`). An Appearance page carries the System, Light or Dark control, persisted through `app_settings`; it is a `SegmentedControl` per the phase 2 plan, not Cap's preview tiles.
     - The pages open in the main window, not in a second window as in Cap: a header Settings icon button (`Icon::Settings`) switches the window to the settings frame, and the frame's Back returns to the editor. Setup stays the first-run page.
     - A second window would add window management and a second surface for the native harness, for no user gain here.
- **Guards for every move.**
  - Scripts read source by text (phase 2): before each move, Grep `desktop/scripts` and `.github` for the moved file's path and its distinctive strings, with a positive control. Also check the workflows' `paths:` filters on `desktop/app/**`. `py desktop/scripts/test-packaging.py` runs with every commit's checks.
  - Linux-only tests do not compile here. When the app's re-exports are removed at the parity move, Grep `app/tests` (including `x11_shell.rs` and `cfg(target_os = "linux")` blocks) for `fframes_studio::(studio_shell|timeline_view|preset_panel|setup_view|canvas_view|frame_image|actions|ui|conversation_panel)`.
  - Before the parity move, list what the suite resolves by path: `build_fixture` through `#[path]` from `studio-engine/tests/support`, the `studio_test_support` dev-dependency, and any `CARGO_MANIFEST_DIR` fixture paths. `studio-ui` gains those dev-dependencies. After the keymap step, the harness calls the shell's `bind_keys`. The app version is passed in by the caller: the shell and the setup view take it, and a `studio-ui` unit test fails if the crate's sources read their own package version. After the move the harness passes `studio-ui`'s own version, so the parity test `the_shell_reports_the_version_it_was_given` checks only that the shell reports what it was given. That the app passes its own version is guarded by the source scan, and `main.rs` passes `env!("CARGO_PKG_VERSION")` of the app crate.
  - Only `app/tests/preset_workflow.rs` reads a view's source by text (`include_str!` of `preset_panel.rs`, Linux-only); its path follows the move. Every other `include_str!` or `read_to_string` of a `src/` path reads project fixtures.
  - Follow-up: that test, `panel_colors_never_come_from_preset_data`, has checked nothing since phase 2. It scans for `rgb(` literals, and the panel now takes every colour from `Theme` fields. A colour derived from preset data would pass it. It needs a check that fits the theme, outside the move commits.
  - The design-system doc's "deliberate differences" bullet about the 248 px left column and the zoom placement is updated in the layout commit.
- **Qualification jobs stay advisory** from this phase until phase 7. The layout change moves bounds that the native jobs click, but the jobs click reported bounds, not fixed coordinates.

## Success criteria

Parity tests pass from `studio-ui`; the window uses Cap's layout and caption on Windows; every file under `studio-ui/src/shell` and `studio-ui/src/panels` is under 1,500 lines; the app crate's `src/` holds only setup, window creation, bins, the qualification code that phase 7 removes, and `conversation_panel*`, which phase 6 moves.
