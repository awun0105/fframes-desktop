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

## Success criteria

Parity tests pass from `studio-ui`; the window uses Cap's layout and caption on Windows; every file under `studio-ui/src/shell` and `studio-ui/src/panels` is under 1,500 lines; the app crate's `src/` holds only setup, window creation, bins, the qualification code that phase 7 removes, and `conversation_panel*`, which phase 6 moves.
