# Phase 3 report: component library

Outcome: all components in the phase table exist in `desktop/app/src/ui/` with pure-function tests, the `TextInput` is restyled, `design_system::button()` delegates to the new kit, the gallery example and `docs/desktop/design-system.md` are written, and the doc is linked from `docs/desktop/README.md`.

Files (all under desktop/app unless noted):
- New in `src/ui/`: `interact.rs` (shared focus ring, Enter/Space activation, tooltip hook, `step_enabled`, `mix`), `button.rs` (`Button`, `IconButton`, `button_colors`, `style_button`), `toggle.rs`, `segmented.rs`, `tooltip.rs`, `kbd.rs`, `menu.rs` (`Menu`, `MenuState`, `menu_key`, `menu_placement`), `select.rs`, `slider.rs` (`Slider`, `fraction_from_x`, `snap_to_step`, `slider_key`), `tab_rail.rs`, `field.rs` (`FieldRow`, `SectionTitle`, `field_value`), `badge.rs` (`Badge`, `StatusDot`, `Tone`), `collapsible.rs`, `notice.rs`; `mod.rs` re-exports.
- Edited: `src/design_system/mod.rs` (`button()` now calls `ui::button::style_button`; same signature and `ButtonStyle` mapping), `src/text_input.rs` (`Render`, placeholder colour, `field_height`; entity input handler, key bindings and IME untouched).
- New: `examples/ui_gallery.rs`, `docs/desktop/design-system.md`; one line added to `docs/desktop/README.md`; phase file status updated.
- Nothing under `examples/` (repo root) touched; no commit; no background processes left.

Validation:
- `cargo fmt --check` passes.
- clippy `-D warnings -A deprecated` on the CI package set with `--all-targets`: clean.
- `ui::` tests: 47 pass. Full lib run: 206 pass, 2 fail (the known `python` spawn failures). Integration run: only the known python-dependent failures (about 24 across tests); no new failures. `preset_workflow` runs 0 tests on this host.
- The gallery builds (`cargo check --examples`); not launched, since there is no GPU here. Rendering and the keyboard-only pass need the user's machine.
- No `rgb(`/`rgba(` literal under `ui/` or the gallery.

Decisions and deviations:
- Handlers take `Fn(&mut Window, &mut App)` (value variants add the value first) rather than gpui's `ClickEvent` listener, so one handler serves click and Enter/Space; callers wrap with an entity update (see the gallery).
- `Notice::new` takes an id (needed for its keyed details state); `Select::menu(&MenuState)` links the trigger to its menu and `Menu::new` takes the `FocusHandle` the caller focuses on open. Slider drag state and the collapsible's measured height live in gpui keyed state, so the caller owns only the value.
- `Segmented` and `TabRail` move focus with `focus_next`/`focus_prev` after a Left/Right selection so the ring follows the selection.
- The focus ring is a 0-blur 2 px accent box shadow (no layout shift). Editor button rest fill is `ctl`; destructive hover and pressed blend `danger_surface` toward `danger` (12 % and 22 %), since the spec gave no values.
- `design_system::button()` keeps the old wrapper form (it styles a caller-built element) because the old call sites attach their own handlers and probes; it now has the new look (h 28, `ctl` fills, ring) instead of the old border style.
- `Role::Switch`, `Slider`, `Menu`, `Tab` etc. are set as accessible roles.

Not verified here: visual match, hover/pressed states, menu placement and flip on screen, slider drag outside the track, collapsible animation, live appearance switch in the gallery.

Status: DONE_WITH_CONCERNS
Summary: Phase 3 is implemented with tests, gallery and docs; fmt, clippy and the lib tests pass apart from the known python failures.
Concerns/Blockers: Rendering, drag, menu placement and the collapsible animation are untested on screen (no GPU on this VM) and need the gallery run on the user's machine.
