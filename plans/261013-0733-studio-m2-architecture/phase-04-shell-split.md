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
5. Move the parity suite to `desktop/crates/studio-ui/tests/parity/` and switch merge gate 3 to `cargo test -p studio-ui --test parity` in CI.

## Reference patterns (Cap, Zeron)

From [the Zeron report](../reports/research-261013-1749-zeron-agent-control.md) and [the Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md); see the roadmap's pattern map.

1. App bootstrap in `desktop/app/src/main.rs`, in Cap's order (`main.rs:242-560`): `gpui_platform::application().with_assets(..)` with icons and fonts embedded through an `AssetSource` and fonts registered with `text_system().add_fonts` (if fframes loads them from disk today, embed them this way); inside `run`, `gpui_tokio::init_from_handle`, the text-input key bindings and the theme global; then `open_window` with per-OS options; native window work only after the handle exists, from `cx.spawn`.
2. Never change native window state inside a gpui update: AppKit and Win32 callbacks re-enter `App` and panic with "RefCell already borrowed". Take the window handle in the update and act from a spawned task.
3. Keymap by key context, most specific first: text input, composer, timeline and player, workspace. Every editing chord (Backspace, Delete, Ctrl/Cmd+Z, Ctrl/Cmd+A, clipboard) is bound in the text-input context (`key_context` already exists in `text_input.rs`), so a focused field consumes them before timeline shortcuts. Test: Ctrl+Z in a focused text field does not reach timeline undo.
4. Player hosting: the preview is its own child entity in `studio-ui/src/player/`, painting through `canvas`, and the only view notified per frame. Header, left column, status strip and timeline are separate entities mounted with `.cached(..)`. No per-frame `window.refresh()` (Cap measured 224 of 298 frames dropped with it, `editor_window.rs:6354`). Keep releasing replaced frames with `drop_image` (`frame_image.rs` does this today). Cap's fork provides `.cached(..)` (Cap mounts its editor sections this way).

## Todo

- [ ] Bootstrap order, native-call rule, key contexts and cached sections applied

- [ ] Scout sections
- [ ] Shell modules
- [ ] Actions and keymap
- [ ] Remaining views moved
- [ ] Parity suite moved and CI updated

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
- **The shell stays one entity in this phase.** `StudioShell` keeps its state. Its methods are split across `impl StudioShell` blocks in section modules: `studio-ui/src/shell/{mod,header,status_strip,left_column,player,transport,keymap,layout,engine_events,qualification}.rs`. This is a pure move, with no change in behaviour, and every file stays under 1,500 lines. The free helpers and their 11 unit tests move with their sections.
- **Qualification code.** The qualification code and `BoundsProbe` move into `studio-ui/src/shell/qualification.rs`, alone in one file, so phase 7 deletes a single module. The success criterion's "app holds the qualification code" is an allowance, not a requirement.
- **Order of commits.** Each is behaviour-preserving, with parity green:
  1. Add the crate dependencies, and move into `studio-ui`, one commit each:
     - `frame_image` (the gpui half), `canvas_view`, `actions` and `ui/probe.rs`;
     - `preset_panel`, `setup_view` and `timeline_view`;
     - `conversation_panel/`.

     The app re-exports them for its tests until the parity move. `probe.rs` lands at `studio-ui/src/probe.rs` first and is folded into `shell/qualification.rs` when the shell moves, so phase 7 still deletes one module. The views keep their own telemetry methods, which phase 7 removes as well: `TimelineView::qualification_metrics`, `PresetPanel::telemetry` and the conversation panel's telemetry.
  2. Move `studio_shell.rs` whole into `studio-ui/src/shell/mod.rs`, then split it section by section, one commit each.
  3. Move the parity suite to `studio-ui/tests/parity/`. Switch merge gate 3 in `desktop.yml`, `desktop-self-hosted.yml` and `desktop-release.yml`.
  4. Keymap: one `bind_keys` with key contexts, most specific first: `TextInput`, then `Composer`, then `Timeline` and `Player`, then `Workspace`. The preview surface's raw `on_key_down` (Space, arrows, Home, End, `c`, `0`, Escape) becomes actions bound in the `Player` context. The root Tab handler stays. Test first: Ctrl+Z in a focused text field does not reach timeline undo, and Space on the preview still toggles playback.
  5. Cap's layout, a visible change:
     - a 52 px header;
     - a player card (44 px toolbar, stage, 48 px transport) beside a fixed sidebar;
     - a timeline card below with a vertical resize grip, clamped like Cap's `EditorVerticalLayout`;
     - the left column's tabs become the sidebar's tab rail;
     - header, sidebar, transport and timeline are mounted as `EditorSectionView`-style entities with `.cached(..)`;
     - the player is the only view notified per frame.

     Cap's sidebar is 416 px. fframes keeps the agent column, which moves into the sidebar in phase 6, so the sidebar width is decided against the 1,275 x 800 default window when this step starts. Element ids that qualification and parity read are kept.
  6. Window chrome:
     - On Windows, a custom frame with Cap's `windows_caption_controls` (114 lines, ported into the kit) and an opaque background. It closes the phase 2 native-caption gap, because the painted caption follows the theme.
     - On macOS, a hidden titlebar; on Linux, client decorations.
     - Cap's editor header draws `windows_caption_controls` at its right end (`editor_window.rs:9415`), with a drag area (`window_control_area(Drag)`, `:9265`). It switches to compact header controls on narrow windows when captions are shown (`compact_header_controls`, `:232`, tested at `:12192-12203`). fframes ports that header, its compact rule and the rule's tests.
     - This step is pushed on its own and waits for the self-hosted result, because the native harness drives this window on the VM.
  7. Settings frame: `setup_view` on Cap's settings shell (page rail, `::settings` components, `setting_row`). An Appearance page carries the System, Light or Dark control, persisted through `app_settings`; it is a `SegmentedControl` per the phase 2 plan, not Cap's preview tiles.
     - The pages open in the main window, not in a second window as in Cap: a header Settings icon button (`Icon::Settings`) switches the window to the settings frame, and the frame's Back returns to the editor. Setup stays the first-run page.
     - A second window would add window management and a second surface for the native harness, for no user gain here.
- **Guards for every move.**
  - Scripts read source by text (phase 2): before each move, Grep `desktop/scripts` and `.github` for the moved file's path and its distinctive strings, with a positive control. Also check the workflows' `paths:` filters on `desktop/app/**`. `py desktop/scripts/test-packaging.py` runs with every commit's checks.
  - Linux-only tests do not compile here. When the app's re-exports are removed at the parity move, Grep `app/tests` (including `x11_shell.rs` and `cfg(target_os = "linux")` blocks) for `fframes_studio::(studio_shell|timeline_view|preset_panel|setup_view|canvas_view|frame_image|actions|ui|conversation_panel)`.
  - Before the parity move, list what the suite resolves by path: `build_fixture` through `#[path]` from `studio-engine/tests/support`, the `studio_test_support` dev-dependency, and any `CARGO_MANIFEST_DIR` fixture paths. `studio-ui` gains those dev-dependencies. After the keymap step, the harness calls the shell's `bind_keys`.
  - Only `app/tests/preset_workflow.rs` reads a view's source by text (`include_str!` of `preset_panel.rs`, Linux-only); its path follows the move. Every other `include_str!` or `read_to_string` of a `src/` path reads project fixtures.
  - Follow-up: that test, `panel_colors_never_come_from_preset_data`, has checked nothing since phase 2. It scans for `rgb(` literals, and the panel now takes every colour from `Theme` fields. A colour derived from preset data would pass it. It needs a check that fits the theme, outside the move commits.
  - The design-system doc's "deliberate differences" bullet about the 248 px left column and the zoom placement is updated in the layout commit.
- **Qualification jobs stay advisory** from this phase until phase 7. The layout change moves bounds that the native jobs click, but the jobs click reported bounds, not fixed coordinates.

## Success criteria

Parity tests pass from `studio-ui`; the window uses Cap's layout and caption on Windows; every file under `studio-ui/src/shell` and `studio-ui/src/panels` is under 1,500 lines; the app crate's `src/` holds only setup, window creation, bins, the qualification code that phase 7 removes, and `conversation_panel*`, which phase 6 moves.
