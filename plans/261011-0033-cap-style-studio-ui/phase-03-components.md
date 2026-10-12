# Phase 3: Component library

Status: done (see plans/reports/fullstack-developer-261012-0602-phase-03.md). Depends on: Phase 2. Effort: 4-5 days.

## Goal

Build the controls every screen uses, each matched to Cap's editor widget in the [design spec](../reports/xia-261011-0016-cap-design-system-spec.md), section 3, plus fframes' focus and keyboard behavior. Finish with a gallery and a short design-system doc.

## Conventions for every component

- A struct implementing gpui `RenderOnce` with `#[derive(IntoElement)]`, built with a fluent API: `Button::new(id, label).variant(..).on_click(..)`. Stateful parts (menu open state, slider drag, text input) use an `Entity` owned by the caller, matching how `TextInput` works today.
- Colours, sizes and type come only from `design_system::tokens()`, `TextStyle` and `Shadow`. No literal values in `ui/`.
- Every interactive component: `track_focus` with a `FocusHandle` or `tab_index(0)`, a 2 px `accent` focus ring on `focus_visible` (fframes deviation from Cap), Enter and Space activate (today's shell behavior at `studio_shell.rs:2569-2576`), disabled blocks clicks and keys and lowers opacity.
- Every clickable component accepts `.probe(Option<BoundsProbe>)` from Phase 2 so telemetry keeps working.
- Element ids are passed in by callers unchanged; components never invent ids that tests look up.

## Components

| File | Component | Spec (sizes, states) | API sketch |
| --- | --- | --- | --- |
| `ui/button.rs` | `Button` | variants `Editor` (default: h 28, min w 28, pad x 7, gap 6, radius 7, Body 13 medium, icon 16; rest `text_2`, hover `text_1` on `ctl_hover`, pressed `ctl_active`, disabled 45% + `text_3`), `Primary` (`accent` fill, `on_accent` text, hover `accent_2`), `Destructive` (`danger_surface` fill, `danger` text), `Ghost` (no fill until hover); `Selected` state = `ctl_hover` fill + `text_1` | `Button::new(id, label).variant(v).icon(Icon).trailing_icon(Icon).selected(bool).disabled(bool).tooltip(text).probe(p).on_click(f)` |
| `ui/button.rs` | `IconButton` | box 28 (24 and 32 variants for transport), icon 14 or 16, round or radius 7 | `IconButton::new(id, Icon).size(Small/Medium/Large).label(a11y_text)...` |
| `ui/toggle.rs` | `Toggle` | track 34 x 20, padding 2, thumb 16 `thumb` + `Shadow::Thumb`; off `ctl_active`, on `accent` | `Toggle::new(id, checked).disabled(b).on_change(f)` |
| `ui/segmented.rs` | `Segmented` | trough `ctl`, padding 2, radius 8; items h 24, pad x 10, radius 6, Label 12 medium; selected `card` + `Shadow::RaisedPill`, `text_1`; idle `text_2`; hover `ctl_hover` | `Segmented::new(id, options, selected).on_select(f)`; Left and Right keys move selection |
| `ui/select.rs` | `Select` (trigger) | h 30, pad x 10, radius 7, Body 13, chevron 12 `text_3`, gap 8, fill `ctl`, no border | `Select::new(id, value_label).on_open(f)` opens a `Menu` |
| `ui/menu.rs` | `Menu`, `MenuState` | radius 12, padding 4, rows h 24, pad x 6, radius 4, Label 12; min w 180, max h 320; `card` fill, `line` border, `Shadow::Pop`; placed 4 below the anchor, flips above when short, 12 margin; checkmark for the selected row | `MenuState::new(items, selected)`; `Menu::new(id, &state).on_select(f).on_dismiss(f)`; keys: Up and Down wrap, Home, End, Enter or Space commit, Escape and click-away dismiss |
| `ui/slider.rs` | `Slider`, `SliderDrag` | row 16, track 4 (3 for zoom) `ctl_active`, fill `accent`, thumb 14 with `Shadow::Thumb` | `Slider::new(id, fraction).track(h).on_change(f)`; Left and Right step, Home and End jump; pure `fraction_from_x()` and `snap_to_step()` |
| `ui/tab_rail.rs` | `TabRail` | bar h 46, pad x 10, bottom `line` border; tabs 40 x 30, radius 9, icon 16; selected `ctl_hover` + `text_1`; idle `text_2`; hover `ctl`; tooltip | `TabRail::new(id, tabs, selected).on_select(f)`; Left and Right move |
| `ui/tooltip.rs` | `Tooltip` | pad 6 x 4, radius 6, Label 12 medium, 200 ms delay, optional key chip; `tooltip_bg` fill, `tooltip_text` text (Cap's dark tooltip style: grey 12 on grey 1) | `.tooltip(text)` on components, built on gpui's tooltip support |
| `ui/kbd.rs` | `Kbd` | Caption 10, radius 6, `ctl` fill | `Kbd::new("Space")` |
| `ui/field.rs` | `FieldRow`, `SectionTitle` | row h 34, label min w 96, Body 13, control right aligned, gap 8, value Small 11 tabular; section title Label 12 medium `text_2`, min h 22 | `FieldRow::new(label, control).hint(text)`; `SectionTitle::new(text).trailing(el)` |
| `ui/notice.rs` | `Notice` | not in Cap: card radius 12, `line` border, 3 px left bar in the tone colour, icon, title BodyStrong, body Body, optional actions, optional collapsible details; tones Info (`accent`), Success, Warning, Danger with their surfaces | `Notice::new(tone, title).body(text).details(text).action(button).on_dismiss(f)` |
| `ui/collapsible.rs` | `Collapsible` | height animates 200 ms ease-out (motion constants), chevron rotates | `Collapsible::new(id, open).header(el).content(el).on_toggle(f)` |
| `ui/badge.rs` | `Badge`, `StatusDot` | Caption 10 medium, pad 6 x 2, radius full, `ctl` fill `text_2`; dot 6 px in tone colour | `Badge::new(text).tone(t)` |
| `text_input.rs` (existing) | `TextInput` restyle | Cap plain input: h 32 (compact h 28), radius 7, Body 13, fill `ctl`, no border, focus border `accent`, placeholder `text_3`, caret 2 px `accent`, selection `selection` | API unchanged; only `Render` and paint colours and sizes change |

## Steps

1. Implement in this order, each with its tests before the next: `Button` and `IconButton`, `Toggle`, `Segmented`, `Tooltip` and `Kbd`, `Menu` and `Select`, `Slider`, `TabRail`, `FieldRow` and `SectionTitle`, `Badge`, `Collapsible`, `Notice`, `TextInput` restyle.
2. `TextInput`: change only `Render for TextInput` (`text_input.rs:437`) and the paint colours (`:465-619`): height 24 to 32 (compact 28), padding, fill, focus border, placeholder, caret, selection. Do not touch the `EntityInputHandler` methods (`:290-430`), the key bindings (`bind_keys`, `:37`) or IME handling.
3. Reimplement `design_system::button()` on top of `ui::Button` (mapping `ButtonStyle::Secondary` to `Editor`, `Primary` to `Primary`, `Selected` to `Editor` with `selected(true)`, `Destructive` to `Destructive`) so old callers keep compiling until Phases 4 to 9 move them.
4. Gallery example `desktop/app/examples/ui_gallery.rs`: opens one window listing every component in every state (rest, hover hint, selected, disabled, focused), with a Light or Dark switch that calls `set_appearance`. Used for screenshots and for comparing with Cap's `docs/sidebar-*.png`.
5. Write `docs/desktop/design-system.md` (short): token roles and when to use each, the component list with one-line usage, the deliberate differences from Cap, the probe rule for telemetry ids, and the licensing line (Cap values, own code). Link it from `docs/desktop/README.md`.

## Tests

| Component | Unit tests |
| --- | --- |
| Button | variant colours per state come from tokens; disabled ignores click and Enter; probe forwarded |
| Toggle | click and Space flip; disabled does not |
| Segmented | Left and Right move selection, wrap off at ends; disabled option skipped |
| Menu | key handling: Down wraps from last to first, Up from first to last, Home, End, Enter commits, Escape dismisses; placement flips above when the anchor is near the bottom |
| Slider | `fraction_from_x` (0 width returns None), `snap_to_step`, key steps |
| TabRail | selection by click and keys |
| Collapsible | height interpolation at 0, 100 and 200 ms |
| TextInput | existing UTF-16 test still passes; height and colour constants match the spec |
| All | a size-constants test per component against the spec values |

These are pure-function tests over each component's state and style resolution (for example `button_colors(variant, state) -> (bg, fg)` and `menu_key(state, key) -> MenuKey`). gpui's `TestAppContext` is not available in this workspace, so no test renders an element tree; rendering is checked in the gallery and the native runs.

## Validation

- `cargo test --locked --manifest-path desktop/Cargo.toml -p fframes-studio ui::` and `--test` suites that touch text input and presets (`preset_workflow`).
- `cargo run --manifest-path desktop/Cargo.toml -p fframes-studio --example ui_gallery` on the user's machine, both appearances, keyboard-only pass through every control.
- `cargo clippy ... -D warnings`.

## Done when

Every component in the table exists with its tests, the gallery shows all states in both appearances, `docs/desktop/design-system.md` is written, and the old `button()` delegates to the new kit.

## Risks and rollback

Building a general library instead of what screens need: the table is the full list; add nothing else. TextInput regressions: style-only changes, and the IME path is untouched. Rollback: per-component files can be reverted independently.
