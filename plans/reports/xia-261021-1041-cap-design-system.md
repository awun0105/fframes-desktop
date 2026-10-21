# Cap Design System: Extraction Overview for fframes Studio

Mode: `--compare`. This is a report only, with no implementation plan.
Date: 2026-10-21

## Outcome

0. **The video editor is the main target. Studio's editor already matches Cap's design system almost exactly.** Part A covers it; the settings page is a bonus in Part B.
   - Every `EditorPalette` value matches in light and dark: the layered surfaces, text, control fills, accent, playhead and segment tinting.
   - Every editor layout measurement matches too: header, sidebar, timeline, track rows, ruler and gutter.
   - Seven editor components are copies with at most a couple of changed lines.
   - The gaps that remain: the timeline has 3 track colours where Cap has 11, `PositionPad` is missing, and Cap's staggered entry animations are missing.
1. **The screenshot comes from Cap's own GPUI app, not the Tauri app.**
   - The evidence: v0.6.1 matches `apps/desktop-gpui/Cargo.toml`. The selected nav row is a 12% accent tint (`settings_window.rs:1801-1805`). The window has Windows caption buttons.
   - Because Cap already builds this UI in GPUI, we can use its values directly, like Zed does. We do not need to translate them from CSS.
2. **Studio already contains most of this design system.**
   - `desktop/crates/studio-ui/src/kit/mod.rs:1` says "ported from Cap's desktop app".
   - The gpui fork is `khanhthanhdev/zed-cap`.
   - Theme tokens, Toggle, Select, Menu, SettingRow, Section, SegmentedControl and RadioCards all exist.
   - The settings nav frame is the same 227 px wide.
   - So the remaining work is a short gap list, not a new system.
3. **Licensing: the port is documented as permitted.**
   - Cap's `apps/desktop-gpui` is `license = "AGPL-3.0"`, and fframes is MIT.
   - `docs/desktop/design-system.md:79` records that the `studio-ui` kit was "ported from Cap's desktop app, with the permission of Cap's owners". It also records that the Windows caption glyphs come from Cap.
   - That permission covers the kit as it was ported. Anything new taken from Cap, such as the account card, the sidebar footer or the theme-tile images, should be confirmed as within the same permission.
   - This report does not verify the permission itself.
   - Studio's existing design-system doc is `docs/desktop/design-system.md`. Its "Deliberate differences from Cap" section already explains several of the gaps listed below.

## Source manifest

| Field | Value |
| --- | --- |
| Repo | `CapSoftware/Cap` |
| Commit | `0443319f22c655153d2892336a7a109cc5fb42a8` (shallow clone, 2026-10-21) |
| Path scope | `apps/desktop-gpui/src/{theme.rs, ui/, settings_window.rs, editor_window.rs, editor_timeline.rs, editor_sidebar.rs, editor_panels.rs}`, `apps/desktop/src/styles/theme.css`, `apps/desktop/src/routes/(window-chrome)/{settings.tsx, settings/Setting.tsx}`, `apps/desktop/src/components/Toggle.tsx`, `packages/ui-solid/src/main.css` |
| GPUI | `wingleeio/zed` fork at rev `5d1f83d9`, with local patches (`apps/desktop-gpui/patches/zed-*.patch`) |
| License | AGPL-3.0 (`apps/desktop-gpui/Cargo.toml`, `LICENSE`) |

## How the values were read

- **Platform.** All values below are for Windows with no native window material. That is the `None` branch of each `Theme::settings_*` method in `theme.rs:956-1059`.
  - The `--macos-settings-*` rgba tokens apply only on macOS when vibrancy or Liquid Glass is installed. They are listed separately at the end.
- **Scale.** The screenshot is at about 150% scale: 1183 px wide against the window's 782 logical px (`settings_window.rs:41`). Measurements from the screenshot are divided by 1.5.
  - They match the source: the nav row pitch is 48 → 32, the sidebar is about 347 → 227, the toggle is 50×30 → 34×20, and a theme tile is 137 high → 93.
- **Disagreements.** Where the Tauri CSS and the GPUI code differ, this report uses the GPUI value. For example, a row's padding is `p(12)` in GPUI and `px-4 py-3.5` in Tauri.

# Part A: Video editor

## A1. Editor surface tokens (`EditorPalette`)

Cap's editor is a layered, low-contrast system: window, then card, then control, with a single accent. The values come from `theme.rs:434-590` and mirror the `--ed-*` variables at `theme.css:39-64`.

| Token | Light | Dark | Role |
| --- | --- | --- | --- |
| window | `#f1f1f3` | `#131315` | Editor background |
| card | `#ffffff` | `#1b1b1e` | Sidebar, timeline and toolbar cards |
| card_2 | `#f6f6f7` | `#202024` | Nested card |
| stage | `#e9e9ec` | `#121214` | Area behind the preview |
| line / line_strong | black at 7.5% / 12% | white at 8% / 13% | Hairlines and borders |
| text_1 / text_2 / text_3 | `#1c1c1e` / `#6e6e75` / `#a0a0a8` | `#f4f4f5` / `#9a9aa3` / `#62626a` | Primary, secondary and tertiary text |
| ctl / ctl_hover / ctl_active | black at 4.5% / 7.5% / 11% | white at 6% / 9% / 13% | Control fills |
| accent / accent_2 | `#007aff` / `#2b8cff` | `#0a84ff` / `#3d9bff` | Selection, toggles, primary actions |
| playhead | `#ff3b30` | `#ff453a` | Timeline playhead |
| thumb | `#ffffff` | `#e8e8ea` | Slider knob |

**Derived colours** (`theme.rs:528-590`). A timeline segment is built from its track colour:
- The fill is the card colour mixed with 13% of the track colour (20% in dark). A selected segment adds 10 points.
- The border is the track colour at 34% alpha (42% in dark).
- The label is the track colour mixed 58% toward black (55% toward white in dark).
- The gutter icon tile is the track colour at 16%, with its glyph mixed 62% toward black or white. Trim handles are mixed 70%.

**Shadows:**
- Card: `0 1 2`, black at 4% (30% in dark).
- Popover: `0 12 32 -12`, black at 25% (`0 16 40` at 60% in dark).
- Preview: `0 12 32 -8` at 35%.
- Slider knob: `0 1 3` at 25%.
- Raised pill: `0 1 2` at 12% plus a 0.5 px ring at 6%.

**Studio:** `studio-theme/src/palette.rs:25-85` holds the same values for every token in both themes. **EXISTS, identical.**

## A2. Editor layout

| Measurement | Cap | Studio `METRICS` (`metrics.rs:54-96`) |
| --- | --- | --- |
| Default window | 1275 × 800 (`editor_window.rs:90`) | — |
| Header | 52 (`:167`) | `header_h` 52 |
| Sidebar width | 416 (`:172`) | `sidebar_w` 416 (resizable 320 to 520) |
| Sidebar tab bar | 46 (`:174`) | `tab_bar_h` 46 |
| Player toolbar / bar | 44 / 48 (`:148-149`) | `toolbar_h` 44 / `transport_h` 48 |
| Player minimum height | 320 (`:144`) | `player_min_h` 320 |
| Timeline height | 260 by default, 240 minimum (`:139-142`) | 260 / 240 |
| Stage padding / preview radius | 16 / 6 (`:605-610`) | `radius_frame` 6 |
| Timeline padding | 11 sides, 9 top, 11 bottom (`editor_timeline.rs:65-67`) | 11 / 9 / 11 |
| Ruler height and ticks | 26; major 8, minor 5 (`:71, :115-116`) | `ruler_h` 26; 8 / 5 (`timeline/ruler.rs:7-8`) |
| Track row | 44 tall, 6 gap, band radius 8 (`:86-89`) | `track_h` 44, `track_gap` 6, `radius_segment` 8 |
| Track gutter / icon tile | 104 / 22 (`:69, :118`) | `track_gutter_w` 104 / `track_tile` 22 |
| Segment | radius 8, 3 px accent bar, label padding 13 left and 10 right (`:122-125`) | the same (`timeline/segment.rs:10-12`) |
| Minimap | 78 max width, 10 tall, 4 px bar, 20 px minimum chip (`:165-168`) | the same (`timeline/minimap.rs:8-11`) |

**EXISTS, identical.** Studio adds things Cap does not have: a 28 px status bar, a 480 px minimum width for the editor, and a resizable sidebar. These are recorded in `docs/desktop/design-system.md` under "Deliberate differences from Cap".

## A3. Editor components

| Cap `ui/` file | What it is | Studio `kit/` | Lines that differ |
| --- | --- | --- | --- |
| `editor_button.rs` | The editor's main ghost button: 28 tall, padding x 7, radius 7, 13 px text, 16 px icon. Transparent with `text_2` at rest, then `ctl_hover` and `text_1` on hover, then `ctl_active` when pressed. | present | 243 (Studio has extended it) |
| `slider.rs` | 4 px track, 16 px row, tracks the drag outside the track | present | 337 (extended) |
| `field.rs` | Labelled rows in plain, section, stacked and inline (34 px) shapes | present | 2 |
| `number_field.rs` | Numeric input | present | 2 |
| `color_picker.rs` | Colour picker | present | 6 |
| `selection_header.rs` | Header of a selection panel | present | 2 |
| `tab_rail.rs` | The sidebar's icon tabs: 46 tall, 30 px boxes, 16 px icons | present | 171 (extended) |
| `collapsible.rs` | Collapsible section | present | 267 (extended) |
| `progress.rs`, `kbd.rs`, `tooltip.rs` | Progress ring, keyboard chip, tooltip | present | 0 to 7 |
| `position_pad.rs` | A 3×3 grid of anchor points for placing an overlay or the camera bubble | **missing** | — |
| `microphone_level.rs` | Microphone level meter (for recording) | missing; not relevant to a video editor | — |

## A4. Editor gaps

1. **Track colours.**
   - Cap defines 11 track colours (`theme.css:21-36`): clip `#3b82f6`, zoom `#64748b`, caption `#0ea5e9`, keyboard `#f97316`, style `#ec4899`, image `#f59e0b`, text `#14b8a6`, mask `#ef4444`, scene `#8b5cf6`, audio `#22c55e` and 3D `#6366f1`.
   - Studio's `palette.rs:205-218` has three: clip, overlay (which uses Cap's scene violet) and audio.
   - Add more only when Studio's timeline gets those kinds of track. The segment-tint formulas above work for any colour.
2. **`PositionPad`.** This is NEW. Port it if Studio's style panel needs anchor placement.
3. **Motion.**
   - Cap staggers the "Add track" tray as it opens (`editor_window.rs:1621-1626`): the tray slides 12 px in 220 ms, and each tile slides 10 px in 320 ms after a 60 ms delay, with 24 ms between tiles.
   - Studio has only `FAST_MS` 120 and `NORMAL_MS` 200, and `reduced_motion()` always returns false (`studio-theme/src/motion.rs:3-11`).
4. **Not design-system items.** Cap's sidebar panels for background wallpapers, cursor, zoom, captions and 3D camera are product features. Their layout constants, such as the 6-column wallpaper grid with gap 6 and 34 px tiles (`editor_sidebar.rs:2445-2452`), belong with those features if Studio adds them.

# Part B: Settings page (bonus) and shared tokens

## 1. Typography

| Token | Value | Source |
| --- | --- | --- |
| UI font | `.SystemUIFont` on macOS and Windows (Segoe UI on Windows, SF Pro on macOS); bundled Geist on Linux only | `theme.rs:19-23`, `assets/fonts/Geist*.ttf` |
| Base text | 13 px (nav items, row titles, select labels) | `settings_window.rs:1800`, `ui/surface.rs:174` |
| Small text | 12 px with a 16 px line height (descriptions, tile labels, menu items) | `ui/surface.rs:177-178`, `ui/menu.rs:387` |
| Caption | 11 px with a 13 px line height (the account subtitle) | `settings_window.rs` profile block (~1755) |
| Section title | 13 px semibold in GPUI (`text-sm font-semibold tracking-tight` in Tauri) | `ui/surface.rs:265`, `Setting.tsx:27` |
| Badge | 10 px medium, uppercase | `Setting.tsx:31`, `ui/field.rs:266` |
| Weights | normal for body text; medium for the selected nav item and tile labels; semibold for headings | `settings_window.rs:1806` |

**Studio today:** `studio-theme/src/typography.rs:6` already uses `UI_FONT = ".SystemUIFont"`, with a 10 to 16 px scale. **EXISTS.**

## 2. Color (light and dark, no material)

The colors are Radix scales, transcribed in `theme.rs:688-799`. In dark mode, gray steps 1-6 and 11 are Cap's own overrides, not stock Radix.

| Role | Light | Dark | Theme method |
| --- | --- | --- | --- |
| Content background | gray_1 `#fcfcfc` | `#111111` | `settings_content_bg` |
| Sidebar and card | gray_2 `#f9f9f9` | `#1c1c1c` | `settings_sidebar_bg`, `settings_card_bg` |
| Fill, hover, border, divider | gray_3 `#f0f0f0` | `#282828` | `settings_fill`, `settings_hover`, `settings_border` |
| Text | gray_12 `#202020` | `#eeeeee` | `settings_text` |
| Muted text and nav icons | gray_10 `#838383` | `#7b7b7b` | `settings_muted` |
| Accent | `#007aff` | `#007aff` | `Theme::SETTINGS_ACCENT` (`theme.rs:1147`) |
| Selected nav row | accent at 12% alpha | accent at 22% alpha | `settings_window.rs:1801-1805` |
| Selected nav icon | accent | accent | `settings_window.rs:1816` |
| Toggle on / off (`Toggle::settings`) | accent / black at 13% | accent / white at 16% | `ui/toggle.rs:79-91` |
| Destructive | red_9 `#e5484d` | `#e5484d` | `theme.rs` |

**Studio today:** every Radix hex value in Cap's `theme.rs` also appears in Studio's `palette.rs` and `lib.rs`. The only Cap values Studio lacks are the macOS material tints, the traffic-light red and the target-overlay blue. Studio adds a green scale and the timeline track colours. In addition, `lib.rs:349-391` has the same `settings_*` fallbacks. `SETTINGS_ACCENT` is `0x007aff` (`lib.rs:447`). **EXISTS.**

## 3. Spacing, radius and layout

| Element | Value | Source |
| --- | --- | --- |
| Window | 782 × 775, minimum 780 × 560; shell radius 16 | `settings_window.rs:41-44`, `theme.rs:956` |
| Sidebar | 227 wide, horizontal padding 10; a 44 px spacer strip at the top | `settings_window.rs:67-76` |
| Content page | padding 18 top / 16 sides / 28 bottom; 28 between sections | `settings_window.rs:2111-2117` |
| Section header | gap 10 to its card; header padding x 4; title and description gap 2 | `ui/surface.rs:242-255` |
| Card | radius 12, 1 px gray_3 border, gray_2 fill; dividers between rows are inset 12 px | `ui/surface.rs:17-31` |
| Setting row | padding 12, min height 46, gap 16 between text and control, gap 2 between title and description | `ui/surface.rs:164-178` |
| Nav item | height 30 to 32, padding x 6, gap 8, radius 8, 16 px icon, 2 px between items | `settings_window.rs:1779-1800` |
| Radius scale | 4 (menu item), 6 (segment), 7 to 8 (control, button, nav), 10 (settings menu), 12 (card, popover), 16 (window) | `ui/*` |

**Studio today:** `studio-theme/src/metrics.rs:54-96` has a 4/8/12/16/24 spacing scale. Its radii are control 7, item 6, segment 8, card 12 and popover 12. **EXISTS.**

## 4. Components

| Component (screenshot) | Cap GPUI spec | Studio | Status |
| --- | --- | --- | --- |
| **Toggle** | Sm 34×20 with a 16 px thumb, Md 44×24 with a 20 px thumb, Lg 56×28 with a 24 px thumb; 2 to 3 px padding; white thumb with a `0 1px 3px rgba(0,0,0,.22)` shadow. The screenshot uses Sm. (`ui/toggle.rs:30-40`, `Toggle.tsx:5-32`) | `kit/toggle.rs:49`, `Toggle::settings` | EXISTS |
| **Select trigger** ("3 seconds") | padding 10 / 6, radius 8, 12 px text, 14 px chevron, gap 6, `settings_fill` background, `settings_border` border. It measures about 95×32. (`ui/select.rs:49-69`) | `kit/select.rs:20` | EXISTS |
| **Dropdown menu** | min width 180, max height 320, radius 10, padding 4, `0 6 18` shadow; items are 24 tall with padding x 6, radius 4 and a 12 px check column. It flips above the trigger when there is no room below, with a 4 px gap and a 12 px viewport margin. Keyboard: arrow keys (wrapping), Home/End, Enter/Space, Escape. (`ui/menu.rs:173-200, 290-406`) | `kit/menu.rs:205` | EXISTS (diverged, see below) |
| **Setting row** (title + description + control) | See section 3. (`ui/surface.rs:122-180`) | `kit/surface.rs:101` | EXISTS |
| **Section** (title + description + optional PRO pill) | `ui/surface.rs:190-270` | `kit/surface.rs:169` | EXISTS |
| **Sidebar nav** (icon + label) | See section 3. The selected row uses the accent tint and an accent icon; unselected rows get `settings_hover` on hover. | `kit/settings_frame.rs:107` (`nav_row`, 227 wide) | EXISTS |
| **Theme tiles** (System / Light / Dark) | A 3-column grid with gap 12. Each tile is 93 tall, radius 8, with a `border_2` border: blue_9 `#0090ff` when selected, gray_4 otherwise and gray_6 on hover. Each shows a picture (`images/{auto,light,dark}.jpg`) with a 12 px medium label below, gray_12 when selected and gray_10 when not. (`settings_window.rs:2967-3015`) | The Appearance page uses `SegmentedControl` (`shell/settings.rs:401-433`), a documented deliberate difference (`docs/desktop/design-system.md`) | **CONFLICT (intentional)** |
| **Account card** | 32 px round avatar (gray_3 background, 16 px user icon), min height 44, padding 6 / 4, radius 8, gap 8. Name is 13/15 and the subtitle 11/13 muted. It has a hover fill. (`settings_window.rs` ~1690-1760, `settings.tsx:473-505`) | None found | **NEW** |
| **Sidebar footer** | A version chip (12 px, padding 4 / 2, radius 4, copies the version on click), then the "View previous versions" and "Check for updates" links, then a full-width accent "Sign In" button (radius 8, about 33 tall). Padding is 8 / 10 / 10, with a top border in `settings_border`. (`settings.tsx:524-560`, `theme.css:937-941`) | None found | **NEW** |
| **Settings button** | Accent or `control_fill` background, radius 8, no hover state. Font size and height follow `ButtonSize`: Xs is 12 px text and 20 tall, Sm 12 px and 28, Md 13 px (its height is not quoted here), Lg 14 px and 36. (`ui/button.rs:59-96, 191-235`) | `kit/button.rs:190` | EXISTS |
| RadioCards | radius 8, padding 12, 16 px radio dot; selected is `border-blue-8 bg-blue-3/40` (`ui/radio_cards.rs`) | `kit/radio_cards.rs:51`, no callers | EXISTS, unused |
| SegmentedControl (settings) | container padding 2, radius 8; items 24 tall with padding x 10 and an 11 px icon (`ui/segmented.rs:133-142`) | `kit/segmented.rs:104` | EXISTS |

### How far Studio has drifted from Cap's files

The comparison ignores whitespace and line endings. It counts lines that differ between the two files, so a high number means Studio has extended the file, not that it is broken.

| File | Cap lines | Studio lines | Lines that differ |
| --- | --- | --- | --- |
| radio_cards.rs | 185 | 185 | 2 |
| select.rs | 188 | 230 | 60 |
| surface.rs | 298 | 277 | 95 |
| toggle.rs | 160 | 231 | 119 |
| segmented.rs | 397 | 607 | 230 |
| button.rs | 701 | 968 | 345 |
| menu.rs | 564 | 873 | 419 |

## 5. Decision matrix

| Decision | Cap's way | Studio's way | Recommendation |
| --- | --- | --- | --- |
| Using the design "directly like Zed" | `ui` is a private module inside an AGPL binary crate, so it cannot be added as a dependency | The kit was ported as transcribed code | Continue porting into `studio-ui/kit` under the recorded permission. Confirm that the permission covers any newly ported pieces. |
| Theme picker | Picture tiles | Segmented pill | Keep the segmented pill unless you want to reverse the documented decision. If you do, build the tiles on `RadioCards` using the specification in section 4. Cap's `.jpg` files are Cap's assets. |
| Account card and footer | Present | Absent | Add them only if Studio gets accounts or sign-in. Otherwise the version and update links alone would be enough. |
| Fonts | System font, Geist on Linux | System font | Keep it. Geist is OFL-licensed if Linux ever needs a bundled font. |
| Window material (macOS) | Vibrancy or Liquid Glass tokens | No material | Out of scope for this report. |

## Appendix: macOS material tokens (not used on Windows)

From `apps/desktop/src/styles/theme.css:304-353`.

| Token | Light | Dark |
| --- | --- | --- |
| sidebar | `rgba(250,250,249,.74)` | `rgba(22,22,22,.90)` |
| content | `rgba(244,244,243,.84)` | `rgba(17,17,17,.94)` |
| card | `rgba(249,249,248,.94)` | `rgba(28,28,28,.96)` |
| fill / hover | `rgba(0,0,0,.055)` | `rgba(255,255,255,.05)` |
| selection | `rgba(0,0,0,.10)` | `#2c2c2c` |
| border | `rgba(0,0,0,.08)` | `rgba(255,255,255,.07)` |
| text / muted | `rgba(0,0,0,.88)` / `rgba(0,0,0,.48)` | `rgba(255,255,255,.95)` / `#a1a1a1` |

Liquid Glass overrides some of these again (`theme.css:361-380`, sidebar radius 18).

## Risk

**Low.**

- Every gap is small: two new sidebar pieces, plus picture tiles if you want them.
- The licensing basis for the port is recorded in `docs/desktop/design-system.md:79`.
- `menu.rs` and `button.rs` have drifted most from Cap. Re-syncing them with Cap would need a careful merge.

## Open questions

0. For the editor: which new track kinds are planned for Studio's timeline? That decides which of Cap's 11 track colours to add. Is `PositionPad` needed for overlay placement?
1. Does Studio need an account card or sign-in at all? If not, those NEW items can be dropped.
2. Should the Appearance page keep the documented segmented pill, or switch to picture tiles? If it switches, who provides the tile artwork?
3. Does the recorded permission from Cap's owners cover porting further components and assets?
4. The search for an existing account component covered only file names containing "account", "avatar" or "profile" in `studio-ui/src`. None of those files is a sidebar card, but a component under another name was not ruled out.
