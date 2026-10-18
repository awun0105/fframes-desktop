---
phase: 2
title: "Cap theme and component kit"
status: completed
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

- [x] Scout callers and component pairs
- [x] Cap theme ported, no material tokens
- [x] Cap kit ported
- [x] Call sites on Cap components; duplicates removed
- [x] Old `ui/` and `design_system/` deleted
- [x] Tests

## Success criteria

Parity tests pass; the appearance gpui test passes and switching the OS appearance flips the theme live on Windows; `desktop/app/src/ui` and `desktop/app/src/design_system` no longer exist; no file in `studio-theme` or `studio-ui/src/kit` exceeds 1,500 lines; `grep -rn "fn button\|fn ui_button\|fn segment_button" desktop/app/src desktop/crates/studio-ui/src --include=*.rs | grep -v /kit/` is empty.

## Design decisions (2026-10-14, after scouting)

- **The palette already is Cap's.** fframes' `design_system::Palette` (`LIGHT`/`DARK`, 26 roles) is byte-identical to Cap's `EditorPalette` for the 16 roles they share (window, card, card_2, stage, line, line_strong, text_1-3, ctl, ctl_hover, ctl_active, accent, accent_2, playhead, thumb). The ten fframes-only roles map onto Cap's Radix steps, which hold the same values:
  - `danger`/`danger_surface` become `red_11`/`red_3`;
  - `warning`/`warning_surface` become `amber_11`/`amber_3`;
  - `tooltip_bg`/`tooltip_text` become `gray_12`/`gray_1`;
  - `selection` becomes `editor.accent` at 25 % (`with_alpha`);
  - `on_accent` stays white.
  - `success`/`success_surface` are Radix green 11/3, a scale Cap does not carry: `green_3` and `green_11` are added to `Theme` (light `0xe6f6eb`/`0x218358`, dark `0x132d21`/`0x3dd68c`).

  So `tokens().palette.X` becomes `theme.editor.X` or the Radix field above, and no rendered colour changes. The tooltip pair is the inverse of the window in both appearances (light `0x202020`/`0xfcfcfc`, dark `0xeeeeee`/`0x111111`), exactly Cap's `gray_12`/`gray_1`.
  - A pin test in the app (`roles_match_the_studio_theme`) asserts every old role, segment colour, shadow, metric and the UI font against `studio-theme`, for both appearances, while both exist; it goes with `design_system`.
  - Segment colours stay fframes' own: Cap's `seg_label`/`seg_border` mix the other way round and draw a translucent border, so `studio_theme::segment_colors_for` keeps fframes' opaque rule.
- **Metrics, type, shadows and motion** are fframes' appearance-independent constants that Cap's components hard-code. They move unchanged to `studio-theme` as `metrics`, `typography`, `elevation` (taking the `Theme`) and `motion`. The theme crate holds `lib.rs` (Theme, Appearance, helpers), `palette.rs` (`Theme::light`/`dark`, `EditorPalette`), `appearance.rs` (preference, `for_window`, `bind_window`), and those four.
- **Dropped from Cap's `theme.rs`.** Dropped entirely:
  - `MaterialTokens` and the `material` field;
  - `with_material`, `material_kind` and the `with_material` argument of `for_window`;
  - the teleprompter surfaces;
  - the six material tests.

  The `body_*`, `settings_*` and `ring_offset` helpers keep only their no-material arm. `apply_native` and `platform::*` (native window theming, `window_is_dark`) are dropped: fframes uses the native titlebar, and `window.appearance()` is the system appearance.
- **The preference.** `AppTheme { System, Light, Dark }` (serde, default System) lives in `studio-proto`, so the engine can persist it without depending on gpui. The `ThemePreference` global wraps it.
  - There is no app settings store today. A new `studio_engine::app_settings` holds `AppSettings { theme }` in `<data>/settings.json` (atomic write, missing file means defaults).
  - The app loads it at start and calls `studio_theme::init(cx, theme)`. The settings page that changes it is phase 4.
- **The kit** goes to `studio-ui/src/kit/` with Cap's file names, minus the following:
  - `windows_caption`, because fframes uses the native caption;
  - `microphone_level` and `position_pad`;
  - glass constructors (`IconButton::glass`, `Toggle::glass`, `Card::liquid_glass`, `Card::glass_pill`, `Popover::glass`) and `.material` reads.

  The kit depends on nothing outside itself and `studio-theme`. `studio-ui` gains `smallvec` and `unicode-segmentation`.
- **Keyboard and accessibility come from fframes.** Cap's components have almost none: three `tab_index(0)`, no roles, labels, focus ring or Enter/Space activation. fframes' controls have all of it, and their tests pin it: menu navigation that skips disabled rows and flips placement near the window edge, arrows on segmented controls, tab rails and sliders, toggle activation, the 2 px accent focus ring. Swapping components unchanged would regress keyboard and screen-reader use, so fframes' `interact` helpers move into `kit/interact.rs` (taking `&Theme`), and each interactive kit component gains the behaviour, test first: fframes' behaviour tests are rewritten against the Cap component, seen failing, then made to pass. Cap's sizes win; fframes' `size_constants_match_spec` tests are not ported. Menu keeps one state machine with both test sets. CI does not locate controls by role, so this is for users rather than for a CI step.
- **Icons.** The kit draws `icons/search.svg` (search field) and `icons/trash.svg` (selection header), which fframes did not bundle; both come from Cap's assets.
- **Components Cap lacks** are rewritten in the Cap pattern inside the kit: `badge` (`Badge`, `StatusDot`, `Tone`), `notice`, `icon` (fframes' icon set), plus `surface`'s `card`/`divider` helpers as `Card::plain`/`Divider`. `ui/probe.rs` (bounds probe) stays in the app until phase 7 removes it; `ui/interact.rs` is folded into the components that use it.
- **Text input.** Cap's `text_input.rs` (2,680 lines) replaces fframes' (734). It has IME through `EntityInputHandler`, grapheme and word movement, multi-line wrap, undo coalescing and clipboard, which is a superset of fframes'.
  - It is split under 1,500 lines per file into `kit/text_input/{mod, state, editing, element, keys, tests}.rs`.
  - Its key bindings are not a mechanical `cmd-` to `secondary-` swap. On macOS cmd-left/right go to the line ends and alt-left/right move by word; on Windows and Linux ctrl-left/right move by word and home/end go to the line ends. Each platform gets its own set; clipboard, undo and select-all use `secondary-`.
  - Before the swap, tests pin what the app relies on today: Enter submits the prompt, single-line fields stay single-line, and the action names the harness and the app keymap dispatch. The swap is its own commit.
  - The test that imports `crate::editor_sidebar` (a hex field) is dropped with that dependency.
  - Parity: the harness's `type_text` paths (prompt, setup fields, preset fields) must stay green.
  - Swapped (2026-10-14). While an IME composition is open, Enter and Escape call `cx.propagate()` instead of confirming or cancelling. Tests prove only that the field and its owner emit nothing; that the platform IME then receives the key (macOS especially) is unverified.
  - Single-line fields flatten newlines on purpose: a pasted line break becomes a space.
  - Fields are tab stops until disabled. Qualification reads the prompt through `text()` and clicks at `text_bounds()`; no script read the old "Composition: idle" status line.
- **Threading the theme.** A view's `render` takes `let theme = Theme::for_window(window, cx);` once and passes `&theme` (it is `Copy`) to helpers and components. Helpers with no window take `theme: &Theme`. The static `APPEARANCE` atomic and `tokens()` go away. `main.rs`'s `observe_window_appearance` hook becomes `studio_theme::bind_window`.
- **Tests.**
  - Cap's state tests come with the kit (collapsible, color picker, kbd, menu, number field, progress, segmented, selection header, slider, text input).
  - `resolve_appearance` covers the preference mapping.
  - A gpui test shows that the preference overrides the window: the test window always reports Light, so Dark must win.
  - A settings round-trip test covers persistence.

  gpui's test window cannot change its appearance (`on_appearance_changed` is a no-op), so the live OS flip is checked by hand on Windows and is not claimed by a test.
- **Order of commits:**
  1. theme crate, preference and settings;
  2. kit;
  3. call sites, by module (shell, conversation panel, timeline, setup and presets, the rest);
  4. deletion of `app/src/ui` and `app/src/design_system`, and the `ui_gallery` example moved onto the kit.

  Parity, clippy and fmt pass after each.
- **Scripts read source by text.** `desktop/scripts/test-packaging.py` asserts strings in `app/src` files (it broke when the SDK lookup moved to the engine). `grep -rn "app/src\|design_system\|text_input" desktop/scripts .github` is checked before each move, and `py desktop/scripts/test-packaging.py` runs with every commit's checks.
- **Native caption.** Without Cap's `apply_native`, a forced Dark preference does not darken the native Windows caption; only the painted surfaces follow it. The default is System and there is no control for it until the phase 4 settings page, which must either apply the native theme or record the gap.
- **Accepted differences at the call sites.** These are visible or behavioural changes that were checked and kept:
  - **Activation.** The old app activation helper fired twice per Enter or Space press: a key-down handler, then the click gpui synthesises on key-up. Two presses produced four activations. Kit buttons activate once, through the click only, and the interact test pins it. The shell's Space-to-play handlers sit on `preview-surface` and `compiled-timeline`, which hold no buttons.
  - **Icon button hover.** gpui svgs don't inherit a parent's text colour, so the kit glyph takes its hover colour through a group. A gpui test window can't observe hover paint, so no test covers it.
  - **Disclosure header.** The chevron is 12 px text_3 with a 6 px gap (Cap). It was 14 px text_2 with an 8 px gap.
  - **Left tab rail.** It uses Cap's look (`justify_around` rather than 4 px gaps). Ids are `left-tabs-{index}`; no test or script read the old name-based ids.
  - **Overflow menu.**
    - The highlight stays hidden until an arrow key is pressed.
    - Clicking the trigger while the menu is open closes it through the backdrop.
    - The menu is 2 px taller.
    - It is deferred at window coordinates, so it paints above, and dismisses across, the whole window instead of only the player card.
  - **Notice dismiss.** The button is 28 px, up from 24 px.
  - **Conversation panel.**
    - Its helpers are now `action_button`, `segment_item` and `tab_item`, thin wrappers that bind the panel's action and bounds probe onto `EditorButton`.
    - Selected segments keep the old `ctl_hover`/`text_1` look.
    - The send and stop buttons are fully rounded `IconButton::editor`s.
    - The agent-option menu is the kit `Menu::editor`. Its rows carry the check mark, it is anchored at the `Select` bounds passed to `on_open`, and it is at least 180 px wide.
    - Space can't reach playback from a panel button: `transport_for_key` is only called by its tests, no keymap binds Space, and the root key handler handles only Tab.
    - Labelled rows use the new kit `field_row`, which keeps the old `FieldRow` look. `hint` and `field_value` have no caller outside `examples/ui_gallery.rs`, and are handled when the gallery moves.
    - Re-opening an option menu after the agent republishes its options keeps the menu where it opened (`ConfigMenu::refresh`, tested).
  - **Timeline toolbar.**
    - The zoom slider is the kit `Slider` with the old 3 px track, 14 px knob, thumb shadow and 7 px half-knob inset. It is named "Timeline zoom" for screen readers.
    - The icon buttons gain tooltips ("Zoom out", "Zoom in", "Cycle scene (S)") and keep their old accessible names.
    - Fit is a filled `EditorButton`. A gpui test pins that it still reports the bounds native qualification clicks.
    - The view resolves its `Theme` in `render` and needs no field.
  - **Setup and presets.**
    - Setup's buttons are `EditorButton`s (accent or control fill). A test renders every setup state.
    - The preset actions are full-width `EditorButton`s with a left icon.
    - The clear-override icon button gains a "Clear" tooltip.
    - The CSS report uses `CollapsibleSection`.
    - A test walks the preset list from the keyboard: Tab to the row, Enter selects it, Tab to Apply, and Enter applies exactly once. The keys are sent as a key-down and key-up pair, because a focused kit control's click comes on key-up. `simulate_keystrokes("enter")` did not activate the kit button in that test.
- **Fork.** fframes pins `khanhthanhdev/zed-cap@c32bfd3` (Cap's fork plus upstream fixes); Cap pins `wingleeio/zed@5d1f83d`. Small API drift is fixed at port time and recorded here.

## Outcome (2026-10-14)

- Every view, `preview_element.rs` and `main.rs` draw from `studio-theme` and the kit. `main.rs` calls `studio_theme::bind_window` and no longer sets a global appearance.
- `desktop/app/src/design_system/` is deleted. `desktop/app/src/ui/` keeps only `probe.rs` (`BoundsProbe`, `bounds_to_record`) for qualification telemetry until phase 7. `with_probe` went with the old button, its only caller.
- Tests that outlive the deletion:
  - The Cap palette spec pins (`light_palette_matches_cap_spec`, `dark_palette_matches_cap_spec`) now run against `Theme` in `studio-theme`, through the role mapping above. They passed before the old module was removed.
  - The icon asset tests moved to `app/src/assets.rs`.
  - The bridging pin `roles_match_the_studio_theme` went with `design_system`, as planned.
- `examples/ui_gallery.rs` shows the kit components and switches System, Light or Dark through `set_preference`. The old `FieldRow::hint` and `field_value` became a `muted` line and a `label`.
- `docs/desktop/design-system.md` describes the theme crate, the kit and the preference.
- Acceptance checks:
  - no file in `studio-theme` or `studio-ui/src/kit` exceeds 1,000 lines;
  - the criterion's grep (`fn button|fn ui_button|fn segment_button`, substring) over `desktop/app/src` and `desktop/crates/studio-ui/src` finds nothing outside `/kit/`. The positive control `fn action_button|fn segment_item|fn tab_item` matches four files. Over all of `desktop` the only hit is `fn button_centre` in `app/tests/x11_shell.rs`, a test helper that reads telemetry coordinates, not a button builder;
  - fmt, clippy `-D warnings`, `cargo test -p fframes-studio` (parity 33/33), `cargo test -p studio-ui --lib` (132), `cargo test -p studio-theme` and the packaging tests all pass.
- Historical phase-2 limitation: the live Windows OS-appearance flip was not checked by the executor because gpui's test window cannot change appearance and the VM's personalization was left alone. The owner subsequently accepted the Windows app hand checks as a whole on 2026-10-18; [M6 platform checks](../261013-0733-studio-m6-cleanup/reports/platform-checks.md) record that owner-reported acceptance, rather than an independently observed OS flip.
- Carried to phase 4:
  - the settings-page `SegmentedControl` for the preference (task 5's UI);
  - the native caption gap under a forced preference.
