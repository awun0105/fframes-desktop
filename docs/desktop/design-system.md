# Studio design system

The Studio window is built from one theme and one component kit, each its own crate: `desktop/crates/studio-theme` holds the tokens and `desktop/crates/studio-ui/src/kit` the components. Screens use these and do not declare colours, sizes or fonts of their own.

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
| `SegmentedControl` | exclusive options; Left and Right move, disabled options are skipped; `.on_option_bounds` reports each option's bounds |
| `Select` and `Menu` | `Select` is the trigger and reports its bounds through `on_open`; build a `MenuState::anchored(bounds, &items)` and render `Menu::editor(..)` while open (Up, Down, Home, End, Enter, Escape; flips above near the window bottom). Disabled `MenuItem`s show in `text_3` with no hover; arrow keys skip them and click or Enter does nothing. `MenuItem::id(..)` gives a row a stable element id |
| `Slider` | `Slider::new(id, fraction, SliderTrack::default())` with `.track`, `.fill`, `.thumb`, `.on_change(step, ..)`; drag, arrows, Home and End; pure `fraction_from_x` and `snap_to_step` |
| `TabRail` | icon tabs with tooltips; `TabRailItem::id(..)` names an item (otherwise `{prefix}-{index}`) and `.dot(true)` draws a 6 px accent dot on it |
| `SettingsFrame` | Cap's settings window: a 227 px nav column (`SettingsNavItem` rows, an optional Back row `settings-back` and an optional `nav_header` strip above them) beside a scrolling page of `Section`s; pages are built from `Section::settings`, `Card::settings_rows` and `SettingRow::settings` |
| `Tooltip`, `KbdChip` | the tooltip (200 ms delay, optional key chip) and the key chip |
| `field_row`, `section_title`, `heading`, `label`, `muted` | label plus right-aligned control; section heading; text in the theme's styles |
| `selectable` | text that inherits the surrounding style and can be selected and copied: drag for a range, double click for a word, triple click for the whole block, Ctrl+C (Cmd+C) to copy; one selection in the app, ended by a press anywhere else or by any other key in a focused text field, never across two blocks. With nothing selected the copy key goes on to a focused text field. `.id(..)` keeps a selection on text that streams; `.over_drag_area()` makes title-bar text selectable instead of moving the window. The kit text helpers (except `section_title`, which heads clickable rows), notices, the agent panel and the status lines use it; button, tab, menu, radio-row and list-row labels stay unselectable. Over the Windows title bar, dragging or double-clicking the project name or path selects text instead of moving or maximizing the window |
| `Badge`, `StatusDot` | small tone-coloured labels and dots (`Tone::{Neutral, Info, Success, Warning, Danger}`) |
| `Notice` | inline message with a tone bar, optional actions, collapsible details (160 px high at most, then they scroll) and dismiss |
| `CollapsibleSection` | header plus content with an animated height; the caller owns `open` |
| `Card`, `divider` | the editor's card surfaces and hairline |
| `TextInput` | 32 px high (28 px compact), no border until focused |

Icons are Lucide SVGs embedded by `desktop/app/src/assets.rs` and named by `kit::Icon`; adding one means the file in `desktop/app/assets/icons`, its entry in `assets.rs`, and the variant in `Icon` and `Icon::ALL` (a test checks the three agree).

## Screen behaviours

- The overflow menu in the transport bar always lists Refresh, Save checkpoint, Restore as copy, Reveal retained draft and Reveal checkpoint. Actions that do not apply are disabled rows, and the rows keep the ids their buttons had (`refresh`, `checkpoint`, `restore`, `draft`, `reveal-checkpoint`).
- The composer's send control is a labelled `Button` (`composer-send`) whose label is the current send action (Send, Steer, Send at boundary); while a turn runs a second outline button (`composer-send-now`) cancels and sends. When sending is not possible the reason shows as text beside the controls (`desktop/crates/studio-ui/src/agent/composer/mod.rs`).
- Reasoning rows stay collapsed. While one streams, its header shows a "Thinking…" badge; the header has a fixed height so the row does not change size when the badge goes away.
- The setup screen remembers which step was active when setup failed and marks it failed (earlier steps done, later ones pending). Install, build and render run as one pipeline, so a failure inside it is attributed to "Install SDK".

## Keyboard and focus

Every control is a tab stop (disabled ones are skipped), shows a 2 px `accent` focus ring while it has keyboard focus, and activates on Enter and Space. Disabled controls ignore clicks and keys and drop to 45 % opacity.

## Telemetry ids

Tests find controls by the bounds the app reports, through the observation telemetry described in [architecture, production probes](architecture.md#13-production-probes). Element ids and labels are chosen by the caller and must not change. A control that is measured passes `.on_bounds(BoundsProbe::new(..).into_bounds_handler())` (`BoundsProbe` is in `desktop/crates/studio-ui/src/probe.rs`), and the callback receives `[left, top, width, height]` on every prepaint. Components never invent ids that a test looks up.

While telemetry is being written, the agent panel's `buttons` telemetry reports the ids listed in `probed_button` in `desktop/crates/studio-ui/src/agent/mod.rs` (the panel's tabs, Apply, Undo, Stop, review, setup and question controls), so the X11 shell test clicks measured bounds instead of fixed window coordinates and the parity tests can check what was laid out.

## Deliberate differences from Cap

- A visible focus ring and Enter or Space activation on every control; Cap's widgets mostly have neither.
- The system UI font on every platform; no bundled font.
- A 16 px `Heading` size for empty-state titles, and the `Notice`, `Badge` and `StatusDot` components, which Cap does not have.
- The window follows Cap's editor layout: under the 52 px header, the player card and a 416 px sidebar card share a row, and the timeline card spans the full width below them. The sidebar has four tabs (`sidebar-tab-Agent`, `sidebar-tab-Project`, `sidebar-tab-Assets`, `sidebar-tab-Style`) in place of Cap's editing tabs. Agent is selected at start. While another tab is open, the Agent tab shows an accent dot when the agent waits for an answer, and a new permission request selects the Agent tab. The grip on the sidebar's left edge (`agent-resize-handle`) keeps it between 320 and 520 px and leaves the player 480 px; a window too narrow for both shrinks them in proportion (`sidebar_width` in `shell/layout.rs`). The grip above the timeline (`timeline-resize-handle`) drags the split between the row and the timeline with Cap's limits. Neither split is saved between launches.
- The timeline card has no header row. Its zoom, Fit, Split and scene-cycle controls sit in the player's transport bar (`timeline-zoom-out`, `timeline-zoom-slider`, `timeline-zoom-in`, `timeline-fit`, `timeline-split`, `timeline-cycle`), and undo and redo are in the header (`timeline-undo`, `timeline-redo`). The transport keeps Cap's three parts: the clock, playback in the centre, and the tools. Playback stays centred while the tools fit half the bar, so below a 1290 px player the zoom slider and the Split and Build labels go, and below 820 px the zoom buttons and Fit (`shell/transport.rs` has the measured widths). Studio-only controls stay: mute, build and cancel preview, and the overflow menu. `FocusPrompt` focuses the composer. Source, checkpoint and job sit in a 28 px status bar at the window's foot (`status-strip`), whose **Details** button (`status-details`) shows the other status lines above it; an error opens them.
- The header is flush with the window's top edge on every system, as in Cap's editor. On Windows it is the title bar: the row around the header is the drag area and Cap's caption buttons (`caption-minimize`, `caption-maximize`, `caption-close`) sit at its right end. Cap presses them through its own Win32 calls; here they are native window control areas, so Windows itself minimizes, maximizes, restores and closes, and offers Snap Layouts over maximize on Windows 11. They live outside the cached header because a cached view does not replay control areas. Like the system's own caption buttons, and unlike every other control, they are not tab stops and have no focus ring: Alt+F4 and Alt+Space reach the same commands from the keyboard. On macOS the header leaves 82 px for the traffic lights, inset to (20, 20), and its empty middle moves the window; Linux keeps the system's title bar, as Cap's editor does. The actions are undo and redo, a divider, Create, Open, Import Rust and Close project, Settings, and Export last with the accent fill, as Cap ends its header with Export. When the window is too narrow for the title, the labelled actions and the window controls, the actions show only their icons (with the label as tooltip); Close project has no icon and keeps its label. Undo, redo and Settings are always icon-only.
- Settings open in a window of their own, as in Cap (782 × 775, at least 780 × 560): the header's Settings button (`settings`) opens it or brings it to the front, and the editor keeps playing beside it. On Windows a 36 px title strip across the window holds the caption buttons; on macOS the traffic lights sit above the nav column. Its close button or Ctrl+W (Cmd+W on macOS) closes it; only closing the main window quits. It does not open while the first-run setup shows, and Run setup again closes it. Its pages hold every setting of the app: **General** (theme, audio output, Run setup again, updates), **Agent** (review policy, permissions, then the agent panel's adapter setup), **SDK** (the managed SDK), **Presets** (the project's style presets) and **Export** (the defaults every export starts from); nav rows `settings-general` to `settings-export`. Per-project parts ask for an open project when none is open. The adapter setup and the preset panel are drawn by their own views, so their fields and telemetry stay theirs; the panel's settings button and `OpenAgentSetup` open the Agent page, the Style tab's **Style presets…** (`open-presets`) the Presets page. The editor keeps only read-outs of these settings: the audio device above the preview and the review policy in the agent's Changes tab. The export panel's choices apply to that export only. On General a `SegmentedControl` (System, Light, Dark; options `theme-0` to `theme-2`) replaces Cap's preview tiles; the choice applies at once and is saved in `app-settings.json`. Cap's nav rows answer only the mouse; here they are tab stops that Enter and Space open.
- Exact Cap colours, including the contrast shortfalls recorded in the `studio-theme` tests (white on `accent` 4.02 light and 3.65 dark, light `text_2` on `window` 4.49, light `text_3` on `card` 2.60).

## Licensing

The component kit in `studio-ui` is ported from Cap's desktop app, with the permission of Cap's owners. Icons are Lucide (ISC); the licence is in `desktop/packaging/sdk/notices/lucide-LICENSE.txt`. The four Windows caption glyphs (`caption-*-windows.svg`) come from Cap.
