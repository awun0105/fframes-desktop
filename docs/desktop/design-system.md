# Studio design system

The Studio window is built from one set of tokens and one component kit, both in `desktop/app/src`: `design_system/` holds the tokens and `ui/` the components. Screens use these and do not declare colours, sizes or fonts of their own. `app.rs` and `agent_spike.rs` (the spike UI) are the only exceptions.

Studio follows the OS light or dark appearance at runtime (no manual setting). Surfaces are flat; there is no glass.

## Tokens

Read them with `design_system::tokens()` (colours in `.palette`, sizes in `.metrics`), `TextStyle` with `text_style(element, style)`, `Shadow` with `shadow(kind)`, and the motion constants. `set_appearance` switches the whole set; only `main.rs` and the gallery call it.

| Role | Use for |
| --- | --- |
| `window`, `card`, `card_2`, `stage` | window background, panels, nested surfaces, the player stage |
| `line`, `line_strong` | 1 px borders and dividers |
| `text_1`, `text_2`, `text_3` | primary, secondary and placeholder or disabled text |
| `ctl`, `ctl_hover`, `ctl_active` | control fill at rest, hovered or selected, pressed; translucent, so they work on any surface |
| `accent`, `accent_2`, `on_accent` | primary action, toggle on, focus ring, selection; hover shade; text on accent |
| `success`, `warning`, `danger` and their `_surface` | status text or icon colour and the soft background behind it |
| `selection`, `thumb`, `playhead`, `tooltip_bg`, `tooltip_text` | text selection, slider and toggle thumbs, the playhead, tooltips |

Type sizes are `Caption` 10, `Small` 11, `Label` 12, `Body` 13, `BodyStrong` 13 semibold, `Title` 14 and `Heading` 16. Shadows are `Card`, `Pop`, `Preview`, `RaisedPill` and `Thumb`. Collapsible animations use `NORMAL_MS` (200 ms, ease-out).

## Components

All components are gpui `RenderOnce` structs with a fluent API. Stateful parts (a menu, a text input) are owned by the caller. Run the gallery to see every state: `cargo run --manifest-path desktop/Cargo.toml -p fframes-studio --example ui_gallery`.

| Component | One line |
| --- | --- |
| `Button` | `Button::new(id, label).variant(Editor, Primary, Destructive or Ghost).icon(..).selected(..).disabled(..).tooltip(..).probe(..).on_click(..)` |
| `IconButton` | square icon-only button, sizes 24, 28 and 32; `label` is the accessible name and tooltip |
| `Toggle` | `Toggle::new(id, checked).on_change(..)` |
| `Segmented` | exclusive options in a trough; Left and Right move, disabled options are skipped |
| `Select` and `Menu` | `Select` is the trigger and records its bounds in a `MenuState`; render `Menu` while open (Up, Down, Home, End, Enter, Escape; flips above near the window bottom). `MenuState::with_disabled(..)` shows rows in `text_3` with no hover; arrow keys skip them and click or Enter does nothing. `Menu::row_ids(..)` gives rows stable element ids |
| `Slider` | `Slider::new(id, fraction).track(h).on_change(..)`; drag, arrows, Home and End; pure `fraction_from_x` and `snap_to_step` |
| `TabRail` | icon tabs with tooltips |
| `Tooltip`, `Kbd` | the dark tooltip (200 ms delay, optional key chip) and the key chip; `.tooltip(text)` on `Button` |
| `FieldRow`, `SectionTitle` | label plus right-aligned control with an optional hint; section heading with a trailing element |
| `Badge`, `StatusDot` | small tone-coloured labels and dots (`Tone::{Neutral, Info, Success, Warning, Danger}`) |
| `Notice` | inline message with a tone bar, optional actions, collapsible details (160 px high at most, then they scroll) and dismiss |
| `Collapsible` | header plus content with an animated height |

Icons are Lucide SVGs embedded by `assets.rs` and named by `ui::Icon`; adding one means the file in `desktop/app/assets/icons`, its entry in `assets.rs`, and the variant in `Icon` and `Icon::ALL` (a test checks the three agree).

## Screen behaviours

- The overflow menu in the transport bar always lists Refresh, Save checkpoint, Restore as copy, Reveal retained draft and Reveal checkpoint. Actions that do not apply are disabled rows, and the rows keep the ids their buttons had (`refresh`, `checkpoint`, `restore`, `draft`, `reveal-checkpoint`).
- `agent-send` is an icon button (`Icon::ArrowUp`); its tooltip and accessible name are the submit label, and the reason it is blocked stays visible as text under the composer.
- Reasoning rows stay collapsed. While one streams, its header shows a "Thinking..." badge; the header has a fixed height so the row does not change size when the badge goes away.
- The setup screen remembers which step was active when setup failed and marks it failed (earlier steps done, later ones pending). Install, build and render run as one pipeline, so a failure inside it is attributed to "Install SDK".
| `TextInput` | the existing input restyled: 32 px high (28 px compact), no border until focused |

`design_system::button()` is the old wrapper; it delegates to `ui::button::style_button` so old call sites look like the new kit until they move to `ui::Button`.

## Keyboard and focus

Every control is a tab stop (disabled ones are skipped), shows a 2 px `accent` focus ring while it has keyboard focus, and activates on Enter and Space. Disabled controls ignore clicks and keys and drop to 45 % opacity.

## Telemetry ids

Native tests and qualification scripts find controls by the bounds the app reports. Element ids and labels are chosen by the caller and must not change. A control that is measured passes `.probe(Some(BoundsProbe::new(..)))`, and the callback receives `[left, top, width, height]` on every prepaint. Components never invent ids that a test looks up.

While qualifying, the agent panel's `buttons` telemetry also reports `agent-tab-Chat`, `agent-tab-Project` and `agent-undo` next to `agent-policy` and `agent-apply`, so the native shell test clicks measured bounds instead of fixed window coordinates.

## Deliberate differences from Cap

- A visible focus ring and Enter or Space activation on every control; Cap's widgets mostly have neither.
- The system UI font on every platform; no bundled font.
- A 16 px `Heading` size for empty-state titles, and the `Notice`, `Badge` and `StatusDot` components, which Cap does not have.
- Zoom controls stay in the timeline header, and the left column keeps a fixed 248 px width.
- Exact Cap colours, including the contrast shortfalls recorded in `design_system/mod.rs` tests (white on `accent` 4.02 light and 3.65 dark, light `text_2` on `window` 4.49, light `text_3` on `card` 2.60).

## Licensing

The values (colours, sizes, radii, shadows, motion) come from Cap's design as facts. The code is fframes' own and does not copy or closely paraphrase Cap's AGPL-3.0 source. Icons are Lucide (ISC); the licence is in `desktop/packaging/sdk/notices/lucide-LICENSE.txt`.
