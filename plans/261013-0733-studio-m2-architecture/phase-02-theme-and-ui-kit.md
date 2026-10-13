---
phase: 2
title: "Cap theme and component kit"
status: pending
priority: P1
effort: "3-4d"
dependencies: [1]
---

# Phase 2: Cap theme and component kit

## Goal

`studio-theme` holds Cap's theme model with the Cap tokens, light and dark following the OS, and `studio-ui/src/kit/` holds Cap's component library. Every fframes view draws with these components.

## Port sources (Cap `a32e4806b`, `apps/desktop-gpui/src`, same GPUI fork as the app)

- `theme.rs` (1,356 lines): the `Copy` `Theme` struct of resolved Radix steps (around `:345`), `Theme::for_window`, `sync_appearance`, and `theme::bind_window` with `observe_window_appearance`. Leave out `MaterialTokens` and every glass, vibrancy or Liquid Glass value (non-goal). Split into `studio-theme/src/{lib,palette,appearance}.rs`.
- `ui/`: `button`, `collapsible`, `color_picker`, `editor_button`, `field`, `kbd`, `menu`, `number_field`, `progress`, `radio_cards`, `segmented`, `select`, `selection_header`, `slider`, `surface`, `tab_rail`, `text_input`, `toggle`, `tooltip`, `windows_caption` and `mod`. Skip the recording-only `microphone_level` and `position_pad`. Target `studio-ui/src/kit/` with the same file names.
- Token values: today's `desktop/app/src/design_system/` holds the Cap values from the [Cap spec](../reports/xia-261011-0016-cap-design-system-spec.md); check them against Cap's `theme.rs` and keep Cap's where they differ.
- Zeron's theme crate, `ui/theme.rs`, `motion.rs` and `gpui-base` are no longer ported.

## Tasks

1. Scout `design_system/`, every caller of the static atomic appearance and of `tokens()` (about 111 sites), and pair each of today's 18 `desktop/app/src/ui/*` components with its closest Cap component.
2. Port Cap's `theme.rs` into `studio-theme` as listed. Cap's per-window `Theme` replaces the static atomic appearance: a render reads it once (`Theme::for_window(window, cx, with_material)` around `:659`, always without material) and passes it to children and components; the user's light/dark choice is the `ThemePreference` global (`:44`). Helpers without a window context take the theme as a parameter. That threads the theme through about 111 call sites the old plan avoided; re-estimate this phase from the S4 closure and split the call-site migration over several commits by module. Engine code (`agent_tools/backend.rs`) reads no UI tokens after phase 1.
3. Port Cap's `ui/` into `studio-ui/src/kit/` with only import and theme-path changes; the GPUI fork is the same, so no API adaptation is expected. Cap's `text_input.rs` replaces fframes' only if IME, selection, undo and clipboard keep working (parity); otherwise keep fframes' and record why.
4. Replace each of today's 18 components with its Cap equivalent at every call site. A fframes component with no Cap equivalent is rewritten in Cap's pattern (below). Remove duplicate local helpers (`conversation_panel.rs:1354-1489` button, segment and tab helpers; `studio_shell.rs:2591` `ui_button`).
5. Port Cap's `ThemePreference` global (`theme.rs:44`) as the user's System, Light or Dark choice, default System, persisted with the app settings and shown as a `SegmentedControl` on the settings page (phase 4 page frame). System follows the OS through `bind_window`.
6. Delete `desktop/app/src/ui/` and `desktop/app/src/design_system/`.
7. Tests: a gpui test that the preference overrides the OS appearance and that System follows it; Cap's state-struct unit tests come with the port (`MenuState`, `CollapsibleState`, `NumberFieldState`, `TextInputState`); a gpui test sets the window appearance and asserts the theme follows it.

## Reference patterns (Cap)

From [the Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md); see the roadmap's pattern map.

1. Components are `#[derive(IntoElement)]` structs with `impl RenderOnce` (Cap `src/ui/button.rs:113, 410`), with one named constructor per surface instead of style flags. Cap has `::body`, `::settings`, `::glass` and `::plain`; fframes uses `::body` (main window), `::plain` (editor panels) and `::settings` (setup and provider pages), and drops `::glass`. Click handlers are `impl Fn(&ClickEvent, &mut Window, &mut App)`.
2. Stateful parts are plain structs tested without a window.
3. Appearance: today's `observe_window_appearance` hook in `app/src/main.rs` becomes Cap's `theme::bind_window`.
4. Motion: `with_animation` for one-shot reveals; ticker tasks that `notify()` for eases. The fork has no CSS-like transforms or masks, so the effects Cap skipped stay skipped.

## Todo

- [ ] Scout callers and component pairs
- [ ] Cap theme ported, no material tokens
- [ ] Cap kit ported
- [ ] Call sites on Cap components; duplicates removed
- [ ] Old `ui/` and `design_system/` deleted
- [ ] Tests

## Success criteria

Parity tests pass; the appearance gpui test passes and switching the OS appearance flips the theme live on Windows; `desktop/app/src/ui` and `desktop/app/src/design_system` no longer exist; no file in `studio-theme` or `studio-ui/src/kit` exceeds 1,500 lines; `grep -rn "fn button\|fn ui_button\|fn segment_button" desktop/app/src desktop/crates/studio-ui/src --include=*.rs | grep -v /kit/` is empty.
