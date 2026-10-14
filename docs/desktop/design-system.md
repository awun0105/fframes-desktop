# Studio design system

The Studio window is built from one theme and one component kit, each its own crate: `desktop/crates/studio-theme` holds the tokens and `desktop/crates/studio-ui/src/kit` the components. Screens use these and do not declare colours, sizes or fonts of their own. `app.rs` and `agent_spike.rs` (the spike UI) are the only exceptions.

Studio follows the `theme` setting: `System` (the default) tracks the OS light or dark appearance at runtime, `Light` and `Dark` fix it. Surfaces are flat; there is no glass.

## Tokens

A view resolves the theme once per render with `Theme::for_window(window, cx)` and passes it to the components. Editor colours are in `theme.editor`, sizes in `METRICS`, text styles through `text_style(element, TextStyle::..)`, shadows through `shadow_for(appearance, Shadow::..)`, and the motion constants are `FAST_MS` and `NORMAL_MS`. `studio_theme::init(cx, preference)` sets the preference at start-up, `set_preference` changes it and repaints every window, and each window calls `bind_window` so it repaints when the OS appearance changes.

| Role | Use for |
| --- | --- |
| `window`, `card`, `card_2`, `stage` | window background, panels, nested surfaces, the player stage |
| `line`, `line_strong` | 1 px borders and dividers |
| `text_1`, `text_2`, `text_3` | primary, secondary and placeholder or disabled text |
| `ctl`, `ctl_hover`, `ctl_active` | control fill at rest, hovered or selected, pressed; translucent, so they work on any surface |
| `accent`, `accent_2`, `theme.on_accent()` | primary action, toggle on, focus ring, selection; hover shade; text on accent |
| `theme.green_11`/`green_3`, `amber_11`/`amber_3`, `red_11`/`red_3` | status text or icon colour and the soft background behind it (`kit::tone_color` and `tone_surface` pick them by `Tone`) |
| `theme.selection()`, `thumb`, `playhead`, `theme.tooltip_bg()`, `theme.tooltip_text()` | text selection, slider and toggle thumbs, the playhead, tooltips |

Type sizes are `Caption` 10, `Small` 11, `Label` 12, `Body` 13, `BodyStrong` 13 semibold, `Title` 14 and `Heading` 16. Shadows are `Card`, `Pop`, `Preview`, `RaisedPill` and `Thumb`. Collapsible animations use `NORMAL_MS` (200 ms, ease-out).

## Components

All components are gpui `RenderOnce` structs with a fluent API. Each has a named constructor per surface (`editor` or `plain` for the editor, `settings`, `body`), and every colour can still be overridden at the call site. Stateful parts (a menu, a text input) are owned by the caller. Run the gallery to see every state and switch System, Light or Dark: `cargo run --manifest-path desktop/Cargo.toml -p fframes-studio --example ui_gallery`.

| Component | One line |
| --- | --- |
| `EditorButton` | the editor's text button: `EditorButton::plain(theme, id).label(..)` with `.filled`, `.accent`, `.destructive`, `.pressed`, `.left_icon`, `.right_icon`, `.tooltip`, `.on_bounds`, `.on_click` |
| `Button` | the window-body and settings button with Cap's variants (`Primary`, `Gray`, `Destructive`, ...) and sizes |
| `IconButton` | icon-only button; `IconButton::editor(theme, id, icon.path())` at `IconButtonSize::{Small, Medium, Large}` (24, 28, 32 px); `label` is the accessible name |
| `Toggle` | `Toggle::plain(theme, id, checked).on_change(..)` |
| `SegmentedControl` | exclusive options; Left and Right move, disabled options are skipped |
| `Select` and `Menu` | `Select` is the trigger and reports its bounds through `on_open`; build a `MenuState::anchored(bounds, &items)` and render `Menu::editor(..)` while open (Up, Down, Home, End, Enter, Escape; flips above near the window bottom). Disabled `MenuItem`s show in `text_3` with no hover; arrow keys skip them and click or Enter does nothing. `MenuItem::id(..)` gives a row a stable element id |
| `Slider` | `Slider::new(id, fraction, SliderTrack::default())` with `.track`, `.fill`, `.thumb`, `.on_change(step, ..)`; drag, arrows, Home and End; pure `fraction_from_x` and `snap_to_step` |
| `TabRail` | icon tabs with tooltips |
| `Tooltip`, `KbdChip` | the tooltip (200 ms delay, optional key chip) and the key chip |
| `field_row`, `section_title`, `heading`, `label`, `muted` | label plus right-aligned control; section heading; text in the theme's styles |
| `Badge`, `StatusDot` | small tone-coloured labels and dots (`Tone::{Neutral, Info, Success, Warning, Danger}`) |
| `Notice` | inline message with a tone bar, optional actions, collapsible details (160 px high at most, then they scroll) and dismiss |
| `CollapsibleSection` | header plus content with an animated height; the caller owns `open` |
| `Card`, `divider` | the editor's card surfaces and hairline |
| `TextInput` | 32 px high (28 px compact), no border until focused |

Icons are Lucide SVGs embedded by `desktop/app/src/assets.rs` and named by `kit::Icon`; adding one means the file in `desktop/app/assets/icons`, its entry in `assets.rs`, and the variant in `Icon` and `Icon::ALL` (a test checks the three agree).

## Screen behaviours

- The overflow menu in the transport bar always lists Refresh, Save checkpoint, Restore as copy, Reveal retained draft and Reveal checkpoint. Actions that do not apply are disabled rows, and the rows keep the ids their buttons had (`refresh`, `checkpoint`, `restore`, `draft`, `reveal-checkpoint`).
- `agent-send` is an icon button (`Icon::ArrowUp`); its tooltip and accessible name are the submit label, and the reason it is blocked stays visible as text under the composer.
- Reasoning rows stay collapsed. While one streams, its header shows a "Thinking..." badge; the header has a fixed height so the row does not change size when the badge goes away.
- The setup screen remembers which step was active when setup failed and marks it failed (earlier steps done, later ones pending). Install, build and render run as one pipeline, so a failure inside it is attributed to "Install SDK".

## Keyboard and focus

Every control is a tab stop (disabled ones are skipped), shows a 2 px `accent` focus ring while it has keyboard focus, and activates on Enter and Space. Disabled controls ignore clicks and keys and drop to 45 % opacity.

## Telemetry ids

Native tests and qualification scripts find controls by the bounds the app reports. Element ids and labels are chosen by the caller and must not change. A control that is measured passes `.on_bounds(BoundsProbe::new(..).into_bounds_handler())` (`BoundsProbe` is in `desktop/crates/studio-ui/src/probe.rs`), and the callback receives `[left, top, width, height]` on every prepaint. Components never invent ids that a test looks up.

While qualifying, the agent panel's `buttons` telemetry also reports `agent-tab-Chat`, `agent-tab-Project` and `agent-undo` next to `agent-policy` and `agent-apply`, so the native shell test clicks measured bounds instead of fixed window coordinates.

## Deliberate differences from Cap

- A visible focus ring and Enter or Space activation on every control; Cap's widgets mostly have neither.
- The system UI font on every platform; no bundled font.
- A 16 px `Heading` size for empty-state titles, and the `Notice`, `Badge` and `StatusDot` components, which Cap does not have.
- Zoom controls stay in the timeline header. The layout follows Cap's editor: the player and the sidebar sit above a full-width timeline, and the grip above the timeline (`timeline-resize-handle`) drags the split between them. The split uses Cap's limits and is not saved between launches. The sidebar keeps a fixed 248 px width rather than Cap's 416 px until the agent column moves into it, and the agent column stays at full height to the right of the editor.
- Exact Cap colours, including the contrast shortfalls recorded in the `studio-theme` tests (white on `accent` 4.02 light and 3.65 dark, light `text_2` on `window` 4.49, light `text_3` on `card` 2.60).

## Licensing

The component kit in `studio-ui` is ported from Cap's desktop app, with the permission of Cap's owners. Icons are Lucide (ISC); the licence is in `desktop/packaging/sdk/notices/lucide-LICENSE.txt`.
