# Cap design system spec, for fframes-studio

Date: 2026-10-11. Source: `~/code/Cap/apps/desktop-gpui` at commit `877d3a3`, read-only. Used by plan `plans/261011-0033-cap-style-studio-ui/`.

## How to use this document

This is a factual record of Cap's editor design: colours, sizes, spacing and behavior, with file and line references. fframes-studio implements its own code against these values. Do not copy, translate or paraphrase Cap's Rust code: `desktop-gpui` is AGPL-3.0 and fframes is MIT. Colour values and measurements are recorded here as design facts. Provenance differs by group: the Radix grey, blue, red and amber steps come from Radix Colors (MIT) and the track colours from Tailwind's palette (MIT), but the editor palette (section 1) is Cap's own selection, with `accent` and `playhead` matching Apple's system blue and red. This is an engineering judgement, not legal advice.

Values were extracted by three read-only scouts and the most-used ones were rechecked against the source by hand (editor palette, layout constants, track colours, editor button, tab rail, toggle, select).

## 1. Two colour systems

Cap has two layers, and fframes-studio needs mostly the second.

1. A Radix grey scale (steps 1 to 12) plus blue, red and amber steps, used by the main window, settings and generic widgets (`theme.rs:688-795`).
2. An editor palette, a small set of named roles used by every editor surface (`theme.rs:469-526`). The editor window is opaque and uses no glass.

### Editor palette (`theme.rs:469-526`, verified)

| Role | Light | Dark | Used for |
| --- | --- | --- | --- |
| window | `#f1f1f3` | `#131315` | window background behind cards |
| card | `#ffffff` | `#1b1b1e` | header, player card, sidebar, timeline card |
| card_2 | `#f6f6f7` | `#202024` | nested surfaces inside a card |
| stage | `#e9e9ec` | `#121214` | the area the preview frame sits on |
| line | black 7.5% (`#00000013`) | white 7.8% (`#ffffff14`) | hairline borders and dividers |
| line_strong | black 12% (`#0000001f`) | white 13% (`#ffffff21`) | stronger borders, ruler major ticks |
| text_1 | `#1c1c1e` | `#f4f4f5` | primary text |
| text_2 | `#6e6e75` | `#9a9aa3` | secondary text, idle icons |
| text_3 | `#a0a0a8` | `#62626a` | tertiary text, placeholders, disabled |
| ctl | black 4.3% | white 5.9% | control fill at rest |
| ctl_hover | black 7.5% | white 9% | control hover, selected tab |
| ctl_active | black 11% | white 13% | pressed control, toggle off track |
| accent | `#007aff` | `#0a84ff` | primary action, toggle on, focus |
| accent_2 | `#2b8cff` | `#3d9bff` | accent hover |
| playhead | `#ff3b30` | `#ff453a` | playhead line and head |
| thumb | `#ffffff` | `#e8e8ea` | slider and toggle thumbs |

Segment colouring is a mix of the track colour with `card`: fill tint 13% light / 20% dark, border 34% / 42%, label mixes the track colour toward black or white at 58% / 55%; a selected segment adds 10% tint (`theme.rs:486-566`).

### Radix steps used by generic widgets (`theme.rs:694-795`)

- Grey light 1 to 12: `fcfcfc f9f9f9 f0f0f0 e8e8e8 e0e0e0 d9d9d9 cecece bbbbbb 8d8d8d 838383 646464 202020`.
- Grey dark 1 to 12: `111111 1c1c1c 282828 323232 3a3a3a 444444 484848 606060 6e6e6e 7b7b7b a1a1a1 eeeeee` (steps 1 to 6 and 11 are Cap's overrides of stock Radix, `theme.rs:749`).
- Blue 9 is `#0090ff` in both; red 9 `#e5484d`; amber 11 `#ab6400` light, `#ffca16` dark. The full tables are at the cited lines.

### Track colours (`editor_timeline.rs:176-190`, verified; Tailwind 500 shades)

clip `#3b82f6`, zoom `#64748b`, caption `#0ea5e9`, keyboard `#f97316`, style `#ec4899`, image `#f59e0b`, text `#14b8a6`, mask `#ef4444`, scene `#8b5cf6`, audio `#22c55e`, 3D `#6366f1`.

Mapping for fframes tracks: repeated or clip segments use clip blue, overlays use scene violet, audio uses audio green.

### Shadows (`theme.rs:569-633`)

| Name | Offset y | Blur | Spread | Alpha (light / dark) | Use |
| --- | --- | --- | --- | --- | --- |
| card | 1 | 2 | 0 | 0.04 / 0.30 | header, cards |
| pop | 12 / 16 | 32 / 40 | -12 | 0.25 / 0.60 | menus, popovers, colour picker |
| preview | 12 | 32 | -8 | 0.35 / 0.35 | the preview frame |
| raised pill | 1 | 2 | 0, plus a 0.5 spread ring | 0.12 and 0.06 | selected segmented item |
| thumb | 1 | 3 | 0 | 0.25 | slider and toggle thumbs |

### Typography

The UI font is `.SystemUIFont` on macOS and Windows; Linux bundles Geist (`theme.rs:19`). fframes keeps the system font on every platform (decided). Sizes come from the widgets, not a type scale: 10 (badges, key chips), 11 (inline values), 12 (labels, segmented items, tooltips, menus), 13 (body, buttons, fields, select), 14 (large buttons, card titles). Weights used: normal, medium (buttons, labels), semibold (section titles). No mono size is defined in the files read.

## 2. Editor layout (`editor_window.rs`, `editor_timeline.rs`)

```
window 1275 x 800 minimum, opaque, background = window
+--------------------------------------------------------------------+
| header 52                                                          |
+--------------------------------------------+-----------------------+
| player card                                | 8 gap | sidebar 416   |
|   toolbar 44                               |       | tab rail 46   |
|   stage: padding 16, frame radius 6,       |       | content 382   |
|          canvas padding 4, preview shadow  |       | (416-2-32)    |
|   transport bar 48                         |       |               |
+---- resize grip 14 (gap 8) ----------------+       |               |
| timeline card: default 260, min 240        |       |               |
|   padding 11 (top 9), ruler header 26      |       |               |
|   track gutter 104 + gap 8                 |       |               |
|   track rows 44 high, 6 apart, radius 8    |       |               |
+--------------------------------------------+-----------------------+
```

| Measure | Value | Source |
| --- | --- | --- |
| Window minimum | 1275 x 800 | `editor_window.rs:89-90` |
| Header height | 52 | `:166` |
| Sidebar width, gutter | 416, 8 | `:168-171` |
| Sidebar tab bar height | 46 | `:173` |
| Player toolbar, transport bar | 44, 48 | `:147-148` |
| Player minimum height | 320 | `:143` |
| Timeline default, minimum | 260, 240 | `:138, :141` |
| Stage padding, frame radius, canvas padding | 16, 6, 4 | `:600-606, :176` |
| Ruler header | 26 | `editor_timeline.rs:68` |
| Track height, gap, band radius | 44, 6, 8 | `:83-86` |
| Track gutter | 104 (+8 gap) | `:65-66` |
| Segment radius, accent bar, label padding | 8, 3, 13 left / 10 right | `:119-122` |
| Ruler ticks | major 8, minor 5 | `:112-113` |
| Playhead | 1 px line, 12 px head (+4 ring), colour `playhead` | `:3484-3497` |
| Hover ghost line | 1 px, 10 px head, `text_3` at 50% | `:3500-3513` |
| Zoom limit | 3 s visible at most zoomed in | `:251` |

Transport bar, left to right (`editor_window.rs:9982-10140`): time readout `current / total` at 13 px (12 px and total hidden when the player is narrower than 620; compact under 760), previous (24 px), play or pause (32 px, Space), next, split tool with tooltip and `S` shortcut, then zoom out, a 72 px zoom slider, zoom in.

Behavior worth matching: the timeline follows the playhead and resumes following 1 s after a manual pan or zoom (`editor_timeline/playback_follow.rs:19-31`); the hover ghost line shows the hover time in the clock; seek on release with a 10 px snap to zero.

## 3. Widgets (`src/ui/*.rs`)

The editor uses the `editor` variants, which take colours from the editor palette.

| Widget | Size and shape | States (role) | Source |
| --- | --- | --- | --- |
| Editor button | height 28, min width 28, padding x 7, gap 6, radius 7, text 13 medium, icon 16 | rest text_2; hover text_1 on ctl_hover; pressed ctl_active; disabled 45% opacity, text_3 | `ui/editor_button.rs:27-279` (verified) |
| Button (generic) | xs 20, sm 28, md py 8, lg 36 high; radius full; text 12 to 14 | variants primary, blue, destructive, outline, white, ghost, grey, dark; disabled 50% | `ui/button.rs:22-458` |
| Icon button | box 28, icon 14, round | hover fill, disabled 45% | `ui/button.rs:504-676` |
| Toggle | sm 34 x 20, thumb 16, padding 2 (default); md 44 x 24 | off ctl_active, on accent, thumb white | `ui/toggle.rs:17-112` (verified) |
| Segmented (editor) | trough ctl, padding 2, radius 8; items 24 high, padding x 10, radius 6, text 12 medium | selected card fill, text_1; idle text_2; hover ctl_hover | `ui/segmented.rs:77-203` |
| Select (plain) | height 30, padding x 10, radius 7, text 13, chevron 12, gap 8 | fill ctl, text_1, chevron text_3, no border | `ui/select.rs:78-97` (verified) |
| Menu (editor) | radius 12, padding 4, rows 24 high, padding x 6, radius 4, text 12, min width 180, max height 320 | card fill, line border, hover ctl_hover, pop shadow; opens 4 below, flips above, 12 margin | `ui/menu.rs:151-364` |
| Slider | track 4 high (zoom slider uses 3), row 16 high | track ctl_active, fill accent, thumb with thumb shadow | `ui/slider.rs:74-192` |
| Tab rail (editor) | bar 46 high, padding x 10, top radius 11, bottom border line; tabs 40 x 30, radius 9, icon 16 | selected ctl_hover, icon text_1; idle text_2; hover ctl; tooltip with label | `ui/tab_rail.rs:57-185` (verified) |
| Tooltip | padding 6 x 4, radius 6, text 12 medium, 200 ms delay, optional key chips | dark style: grey 12 fill, grey 1 text | `ui/tooltip.rs:29-120` |
| Field row (inline) | row 34, label min width 96, text 13, control right aligned, gap 8, value 11 tabular | label text_1, disabled text_3 | `ui/field.rs:49-232` |
| Section title | text 12 medium, text_2, min height 22, gap 10 | | `ui/field.rs` section variant |
| Text input (plain) | height 32, radius 7, text 13, fill ctl, no border | focus border accent | `ui/text_input.rs:1574-1698` |
| Collapsible | animates height over 0.2 s ease-out cubic | | `ui/collapsible.rs:32-207` |
| Key chip | 10 to 11 px text, radius 6 | | `ui/kbd.rs:57-127` |

## 4. Where fframes deliberately differs

- **Keyboard and focus.** Cap's widgets mostly paint no focus ring and handle no keys (buttons, toggles, selects, sliders, segmented, tab rail). fframes keeps a visible focus ring (`accent` at 2 px) and Enter or Space activation on every control, because the current app already has `focus_visible` styling and keyboard paths.
- **No materials.** Cap's main and settings windows use glass or vibrancy tokens; fframes uses flat surfaces (decided).
- **No manual theme setting.** Cap stores System, Light or Dark; fframes follows the OS only (decided).
- **Agent column.** Cap has one right sidebar; fframes keeps the agent panel in its own right column and puts Project, Assets and Style tabs on the left (decided).
- **Font.** System UI font on Linux too, no Geist (decided).
- **Colours are not adjusted for contrast** (decided). Measured shortfalls against WCAG AA 4.5:1: white on `accent` 4.02 light and 3.65 dark, light `text_2` on `window` 4.49, light `text_3` on `card` 2.60.

## Gaps and conflicts found

- Cap's README disagrees with its code for two glass material values (sidebar and card alpha). Irrelevant here because fframes uses no materials; trust the code.
- The README's editor section is partly stale (says playback is future work while later sections describe it as done).
- The corner radius of the main player, sidebar and timeline cards was not confirmed. Popovers use 12 (`editor_window.rs:8169`), segments 8, the preview frame 6, and the tab rail's top radius is 11, which suggests cards of about 12. Phase 4 step 0 of the UI plan confirms this in `EditorSectionView` rendering; default to 12.
- Cap has no mono font size and no spacing scale; spacing is per widget. fframes keeps its existing 4, 8, 12, 16, 24 metric scale, which matches the values above.
