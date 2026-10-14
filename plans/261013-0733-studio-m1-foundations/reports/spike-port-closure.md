# Spike S4: port closure for Zeron and Cap UI code

Date: 2026-10-14. Host: this Windows Server 2022 VM, no GPU, Rust 1.98.1 (MSVC).
Sources: Cap `a32e4806b` (`apps/desktop-gpui/src`), Zeron `09beace0` (`crates/ui/src`, on zui
`361ff20b`), and the mirror `khanhthanhdev/zed-cap` at `c32bfd39` (the S1 pin).

## Result

**Pass.** Cap's kit ports with no source edits. Zeron's UI code needs about one GPUI API change
per 5,000 lines on Cap's fork. Almost all of the cost is in replacing data sources and dropping
features, not in GPUI.

- Cap's `theme.rs` plus `ui/{button,menu,select,tooltip,kbd,slider,text_input}.rs` compile and
  pass their tests on the fork **verbatim**. The only stand-ins are those S1 already wrote, plus a
  three-function test helper.
- Zeron's `todo_panel.rs` compiles on the fork against fframes stand-in types, and its tests pass.
  The file itself needs no GPUI API change: 17 scripted edits (data source, theme, kit) and 6
  dropped transcript-scan tests or helpers.
- Zeron's `ComposerInput` compiles too: 4,732 lines with its pure helpers, plus `chip.rs`,
  `composer_markdown.rs` and four `zeron_proto` mention modules.
  - It needs one GPUI API change: `Window::blur()` became `blur(cx)`.
  - Dictation is removed: about 350 lines across 20 sites.
  - Two smoke tests type through the fork's `EntityInputHandler`, run a bound Backspace and Undo,
    and paint the custom element.
- Zui adds three `ListState` methods and two `EdgeFade` fields that the fork lacks. No other
  zui-only API is used by `composer.rs`, `transcript.rs`, `changes.rs`, `todo_panel.rs`,
  `markdown/*.rs` or `edge_fade.rs`.
- Re-estimates: M2 goes from 26-35d to 27-38d, mostly because phase 4 is a rewrite against
  snapshots rather than a port. M4 goes from 24-34d to 24-33d. M5 goes from 17-25d to 16-24d.
  Two plan corrections:
  - Cap's timeline does snap, so M4 phase 5 ports its snapping rather than designing it.
  - M5 phase 2 should keep `ComposerInput` rather than build on the kit `TextInput`.

| Exit criterion | Status |
|---|---|
| Closure tables for Cap `ui/`, `theme.rs` and assets, the Zeron four, and Cap's editor files | Done (below) |
| `todo_panel.rs` and `ComposerInput` compile on the fork against stand-ins | Done; tests green |
| Cap types per layout file and the replacing `studio-proto` snapshot | Done (below) |
| Re-estimates written into M2, M4 and M5 | Done |

## Method

- **Closure.** A script resolves `crate::` references, including `use crate::{…}` blocks, after
  stripping comments and `#[cfg(test)]` modules and skipping `*_tests.rs` files. It follows them
  transitively, but stops at each module marked replace or drop, so a table lists what crosses a
  cut, not the whole app.
  - Without cuts, Cap's `ui/` reaches 58 modules (162k lines) through `platform`, `store`, `menus`
    and `feeds`, and so does every Zeron file. Those totals measure nothing.
  - External crates are matched against each crate's `Cargo.toml`.
  - Three edges from a plain grep were comments or test code and are excluded:
    - `text_input.rs:2665` (`editor_sidebar` hex helpers) is test-only.
    - `ui/mod.rs:62` (`crate::menus::build_menus`) is a comment.
    - `slider.rs:429,438` (`settings_window`, `teleprompter_window`) are comments.
- **zui against the fork.** I took a sparse checkout of zui's `crates/gpui/src` at `361ff20b` and
  listed every `fn`, `struct`, `enum` and `trait` name that zui defines and the fork lacks. I then
  intersected those names with the method calls and `gpui::` types in each Zeron file.
  - This catches missing APIs, not changed signatures or fields; the compile check covers those
    for the two compiled files.
- **Compile check.** I restored the S1 spike crate (`git archive b960d6e`) into the scratch
  directory, with its lockfile and the fork pin unchanged, and made two additions:
  - Cap's five extra `ui` files go into `cap-gpui-hello` (replacing S1's `Menu` stand-in).
  - A new member, `zeron-port`, holds the Zeron code and its stand-ins.
  - All edits to Zeron sources are scripted string replacements, logged by category.
  - The timings below are wall clock for this agent with the stand-ins already designed. They show
    how small the adaptation is, not engineer-days.

## Closure: Cap `ui/`, `theme.rs` and assets

Cut points, which are the only places Cap's kit leaves `ui/` and `theme`:

| Crosses to | Symbols | Decision |
|---|---|---|
| `platform` | `ForcedAppearance`, `MaterialKind`, `apply_window_theme`, `window_is_dark`, `active_material` (theme); `native_window`, `zoom_native` (caption); `window_is_visible` (mic level) | Replace: fframes appearance with no material. S1's stand-ins have Cap's exact signatures |
| `store` | `AppTheme`, `GeneralSettings::load().theme` | Replace: the fframes app settings `ThemePreference` (M2 phase 2 task 5) |
| `menus` | `close_window_by_handle` (caption) | Replace: the fframes window registry |
| `feeds` | `Feeds`, `picker_level` (`microphone_level.rs` only) | Drop with `microphone_level` |
| `editor_sidebar` | `hex_digit_count`, `hex_to_rgb` (`text_input.rs` tests only) | Port: 50 lines, copied verbatim into the spike |

Per file (non-test lines; "theme" means `crate::theme`):

| File | Lines | Reaches (`crate::`) | External | Decision |
|---|---|---|---|---|
| `button.rs` | 701 | `menu` (`OpenHandler`, `Menu::trigger`), theme | gpui | Port (verbatim, S1) |
| `menu.rs` | 564 | `button::ClickHandler`, theme | gpui | Port (verbatim, S4) |
| `select.rs` | 188 | `menu`, `button`, theme | gpui | Port (verbatim, S4) |
| `tooltip.rs` | 122 | `kbd`, theme | gpui | Port (verbatim, S4) |
| `kbd.rs` | 154 | theme | gpui | Port (verbatim, S4) |
| `slider.rs` | 489 | theme (`thumb_shadow`) | gpui | Port (verbatim, S4) |
| `text_input.rs` | 2,680 | theme | gpui, smallvec, unicode-segmentation | Port (verbatim, S4) |
| `windows_caption.rs` | 114 | theme, `platform`, `menus` | gpui | Port (verbatim, S1) |
| `collapsible`, `color_picker`, `field`, `number_field`, `progress`, `radio_cards`, `segmented`, `selection_header`, `surface`, `tab_rail`, `toggle`, `editor_button` | 3,446 | theme, and `button`, `tooltip` or `kbd` only | gpui | Port; the same import shape as the files above |
| `mod.rs` | 128 | re-exports | none | Port, trimmed |
| `microphone_level.rs` | 79 | `feeds`, `platform` | gpui | Drop (recording) |
| `position_pad.rs` | 231 | `slider`, theme | gpui | Drop (camera position) |
| `theme.rs` | 1,356 | `platform`, `store` | gpui | Port; replace the cut points above; leave out `MaterialTokens` (M2 phase 2) |
| `assets.rs` | 439 | none | gpui | Port the `AssetSource` and only what the kit uses: Geist regular, medium and bold, and 8 icons (`caption-{close,maximize,minimize,restore}-windows`, `check`, `chevron-down`, `search`, `trash`). Drop the rest (onboarding art, music, tray) |

Closure total: about 10,600 lines (8,810 in `ui/`, 1,356 in `theme.rs`, 439 in `assets.rs`) and
no external crate besides gpui, smallvec and unicode-segmentation. The requested subset
`ui/{select,menu,tooltip,kbd,slider}.rs` closes over itself plus `button` and theme: 2,218 lines.

Licensing: Cap's app and its `kernels` crate are AGPL-3.0 (`Cargo.toml:15`, `kernels/Cargo.toml:5`).
The roadmap records the owners' permission to port (roadmap, "Licence").

## Closure: Cap editor files

| Source | Non-test lines | Reaches | External | Port | Replace with a fframes type | Drop |
|---|---|---|---|---|---|---|
| `editor_timeline.rs` (+`playback_follow.rs`, 63) | 3,535 | `editor_edits` (`Selection`, `clip_is_muted`, `split_clip_segment`, `set_clip_volume`, `set_clip_muted`), `editor_panels` (camera3d shot helpers, 4 symbols), theme (`rgba_alpha`) | gpui; `cap_project` (`TimelineConfiguration`, `ProjectConfiguration`, 7 segment types), `cap_editor::PreparingEditorProgress`, `cap_gpui_kernels::waveform::peaks` (a 3-line wrapper) | About 1,800 lines: layout constants, `track_color`, `Transform`, `time_from_x`, `content_width`, ruler markings, `waveform_path` and amplitude maths, `visible_box`, ruler, minimap and `MinimapDrag`, `render_row`, `render_segment`, `render_handle`, labels, playhead lines, `PlaybackFollow` | `TrackKind`, `TrackLanes`, `Segment`, `TimelineModel`, `build_rows`, `clip_rows` (about 840 lines) become a model over `TimelineSnapshot`; waveform peaks come from the engine | Camera3D ghosts, gap ghosts, fades, holds, clip markings, the nine Cap track kinds' label bodies (about 900 lines) |
| `editor_edits.rs` | 2,611 | `editor_timeline` (`Segment`, `TrackKind`, `clip_timeline_offsets`), `editor_clips::drop_clip_transition`, `editor_panels` (camera3d) | `cap_project` (21 types), `cap_rendering::camera3d::sample_track` | About 900 lines: `Selection`, `click_selection`, `ProjectHistory` (snapshot undo with pause and resume), `Hit` and `hit_test`, `DragBounds`, trim and move bounds, `split_at`, `snap_split_time`, clip trim and split | `ProjectConfiguration` and `TimelineConfiguration` become `TimingDoc` (M4 phase 3) | Per-track inserts and defaults, ripple deletes, Camera3D, clip speed, merge and transitions (about 1,700 lines) |
| `editor_window.rs` frame path | about 350 | `editor_canvas` (`CanvasRect`, `PlayerFrame`), theme (`preview_shadow`) | gpui, image, smallvec, flume, `cap_gpui_kernels::frame::unpad_rgba_to_bgra` (in `kernels/src/frame.rs`, 291 lines), `cap_editor` (`EditorFrameCallback`, `EditorFrameOutput`, `EditorState`), `cap_rendering` (`RenderedFrame`, `FrameLayout`, `SurfaceFrame`) | `PreviewFrameView` (canvas paint, shadow, ring, `letterbox`), `PumpStats` and `StatsSnapshot`, `PlayheadSignal`, `playhead_extrapolation` | `RenderedFrame` becomes the worker's RGBA frame (fframes `frame_image.rs` already converts and calls `drop_image`); `make_frame_callback`'s channel becomes a single slot (M4 phase 6); `CanvasRect` becomes fframes' canvas hit rect | `EditorPreviewFrame::Surface`, `surface_preview_frame`, `cidre` (macOS zero-copy) |

## Closure: Zeron files

Cut list used throughout:
- **Drop:** frost, dictation, voice, Zeron settings, appshots, account usage, composer dock,
  new-thread backgrounds, comments, PR and browser modules.
- **Replace with fframes:** `state` (`AppState`), `history`, `attachments`, `pickers`, `notice`,
  `queue`, `popover`, theme, motion, typography, icons, `file_icons`, `loaders`.

| File | Non-test lines | Reaches across a cut | Zeron crates and externals | Ported beyond the file | zui-only API |
|---|---|---|---|---|---|
| `todo_panel.rs` | 685 | `composer` (`Composer`, `QUEUE_COMPOSER_OVERLAP`), `queue` (surface, radii), frost, `settings::widgets::text_tooltip`, icons, motion, `loaders::mini_glyph_spinner`, theme | `zeron_doc` (`MessagePart`, `MessageRole`, `SessionMessageEntry`), `zeron_proto` (`TodoItem`, `TodoStatus`, `ToolCall`) | `edge_fade.rs` (533; per-pixel fades on gpui `EdgeFade`) | none in the file; `edge_fade.rs` uses `EdgeFade { band_left, band_right }` |
| `composer.rs` (`ComposerInput` part, compiled) | about 4,700 | dictation (about 350 lines), frost, `popover::popover_card`, `markdown::{selection::selected_text, render::range_rects}`, `typography::ui_rems`, `settings::{ComposerSendBehavior, platform_combo, SkillCompletionSettings}`, `file_icons`, icons, motion | `zeron_proto::{file,attachment,chat}_mentions`, `invocation` (1,260 lines, pure apart from serde), `zeron_syntax`, pulldown-cmark, serde_json, unicode-segmentation | `composer/chip.rs` (296), `composer_markdown.rs` (977, pulldown-cmark only) | none (but `blur()` changed signature, below) |
| `composer.rs` (the `Composer` wrapper, not compiled) | about 6,600 | `state`, `pickers`, `attachments`, `appshots`, `account_usage`, `composer_dock`, `queue`, `notice`, `badges`, `comments`, `todo_panel` | `zeron_rpc` (20), `zeron_doc`, `zeron_proto::{RunRequest, SlashCommand, UserInput*}`, tokio, chrono, uuid, base64 | none | none |
| `transcript.rs` | about 10,300 | `attachments` (25 symbols), `markdown` (selection, render, veil, mermaid), `composer` (chips, `sent_mention_display`), `changes` (`FileDiff`, `DiffHighlights`), `popover`, `state`, `tool_images`, `image_media`, `workspace_links` | `zeron_doc` (66), `zeron_proto` (32), `zeron_syntax`, `similar`, `usvg`, `image` | `roll_text` (504), `badges` (252), `syntax_cache` (199), `edge_fade` | `ListState::{set_tail_reservation, tail_reservation_filled, offset_for_item}` (about 180 lines in zui's `list.rs`) |
| `changes.rs` | 5,214 | `surface_chrome` (84, port), `comment_ui` and `comments` (review comments: drop), `history`, `popover`, `markdown::render`, `composer` | `zeron_syntax` (33), `zeron_proto::WatchCheckoutDiffs`, `zeron_rpc`, serde_json, unicode-width | `surface_chrome` | none |

Decisions for every Zeron crate and module these files reach:

| Dependency | Decision |
|---|---|
| `zeron_doc` (`MessagePart`, `MessageRole`, `SessionMessageEntry`, …) | Replace with `studio-proto` transcript rows and `WorkflowSnapshot` |
| `zeron_proto` mention modules and `invocation` | Port (compiled verbatim) |
| `zeron_proto::{TodoItem, TodoStatus, ToolCall}` | Replace with the ACP plan entry and tool-call rows in `WorkflowSnapshot` |
| `zeron_proto::{RunRequest, SlashCommand, UserInput*, WatchCheckoutDiffs}` | Replace with `EngineCommand` and the driver trait |
| `zeron_rpc` | Replace with engine commands and the M5a driver trait |
| `zeron_syntax` | Replace with the five-grammar module from M5 phase 1 |
| `zeron_voice` | Drop |
| `zeron_markdown`, `zeron_veil` | Port (pulldown-cmark only) |
| `gpui_base` | Replace: Cap's `Toggle` for the checkbox, `cap_gpui_hello::init`-style kit init in tests |
| `edge_fade`, `composer/chip`, `composer_markdown`, `badges`, `roll_text`, `syntax_cache`, `surface_chrome` | Port |
| `markdown` (render, selection, veil, links) | Port in M5 phase 1; `workspace_links` is rewritten; Mermaid is dropped |
| `tool_images`, `image_media` | Replace with the existing `prompt_with_images` path and fframes image decoding |
| `history`, `state`, `attachments`, `pickers`, `notice`, `queue`, `popover` | Replace with fframes snapshots, the project-confined attachment rules (M5 phase 2) and Cap kit popovers |
| `browser`, `appshots`, `account_usage`, `composer_dock`, `comments`, `comment_ui`, frost, dictation, settings | Drop |
| pulldown-cmark, serde, serde_json, unicode-segmentation, unicode-width, similar | Keep as dependencies (similar for diff rows in M5 phase 4) |
| usvg | Drop (used for Mermaid and SVG tool images only) |
| image | Keep (already an app dependency) |
| tokio, chrono, uuid, base64, futures, tracing | Wrapper-only (`Composer`, transcript, changes): the fframes rewrite uses the app's own async runtime, ids and logging, so none is added for the ported files |

Two dependencies are smaller than they look:
- `gpui-base` (Zeron's gpui-component fork) appears in these files only as `gpui_base::init` in
  tests and one `Checkbox::new` in `markdown/`. Cap's `Toggle` replaces it.
- The markdown UI module (8,357 lines, of which Mermaid is 763) has no zui-only API and reaches
  only theme, icons, typography, motion, `popover`, `settings`, `workspace_links` (1,104 lines,
  rewritten under the link rules in M5 phase 1), `image_media` and one `browser` call.

### Theme tokens

Zeron reads a global `Theme::of(cx)`, while Cap's theme is a `Copy` value built per window
(`Theme::for_window`). The four files use 38 Zeron tokens. I mapped them once, in
`AgentTheme::from_cap`, which keeps Zeron's field names over Cap's Radix steps:

| Zeron | Cap |
|---|---|
| `text` | `gray(12)` |
| `text_muted` | `gray(11)` |
| `text_faint` | `gray(9)` |
| `element_hover` | `gray(4)` |
| `accent`, `glyph`, `caret` | `blue_9` |
| `selection` | `blue_9` at 30% |
| `ink(a)` | `gray(12)` at `a` |
| `code_wash` | `gray(3)` |

Cap has no green step, no mono font, no syntax palette and no diff colours. `studio-theme`
therefore needs a small agent extension: `success`, `font_mono`, `code_font_size`, `SyntaxPalette`,
`diff_add`, `diff_del` and `diff_hunk_bg`.

For `cx`-only paint code (10 `Theme::of(cx)` sites in the input element), the adapter keeps the last
resolved theme in a gpui global that `AgentTheme::of(window, cx)` refreshes. The other way is to
thread the theme into the element. Either is about 10 call sites.

## Compile check

### What was compiled

`desktop/spikes/cap-gpui-hello` (restored from `b960d6e`) on the mirror `c32bfd39`:

| Crate | Contents | Tests |
|---|---|---|
| `cap-gpui-hello` | S1's crate, plus Cap's `ui/{menu,select,tooltip,kbd,slider,text_input}.rs` **verbatim**, Cap's own `ui/mod.rs` re-export lines, and `editor_sidebar.rs` (the two hex helpers, test-only) | 53 pass: Cap theme and kit state tests, `TextInputState` tests, and S1's `#[gpui::test]` |
| `ff-ui` | unchanged from S1 | 28 pass |
| `zeron-port` | `todo_panel.rs`, `edge_fade.rs`, `composer_input/{mod,chip}.rs`, `composer_markdown.rs`, and `zp/{file,attachment,chat}_mentions.rs` and `invocation.rs` (verbatim apart from `crate::zp::` paths), plus stand-ins | 54 pass, 2 ignored |

The two ignored tests are `composer_markdown`'s injected-language tests. They need a real
highlighter, and the `zeron_syntax` stand-in knows no language; M5 phase 1 supplies one.

Stand-ins, kept small on purpose:
- `proto.rs` (39 lines): an ACP-shaped `PlanEntry` and `WorkflowSnapshot`.
- `host.rs` (47): the `Composer` host and `AppState` reduced to session key and workflow.
- `agent_theme.rs` (110): the token adapter above.
- `syntax.rs` (83): types only.
- `settings.rs` (27).
- `small_stand_ins.rs` (96): typography, `popover_card` on Cap's look, two markdown hooks, file
  icons.
- `motion.rs`: Zeron's `CubicBezier` copied verbatim, `lerp`, `fade_quick`.
- `icons.rs`, `loaders.rs`: a pulsing dot stands in for the 2×3 glyph spinner.

### Changes needed, by category

| Category | `todo_panel.rs` (+`edge_fade.rs`) | `ComposerInput` (+chip, markdown faces, mentions) |
|---|---|---|
| **GPUI API (zui to fork)** | 1: `EdgeFade { band_left, band_right }` are zui-only fields. I dropped them, so a horizontal fade band no longer shrinks to the hidden overflow. Vertical fades, which the todo panel uses, are unchanged | 1 site: `Window::blur()` is `blur(cx)` on the mirror (upstream `f66ed399`, cherry-picked in S1), so the `on_mouse_down_out` closure must name `cx` |
| **Data source (stand-in for fframes types)** | 9: `TodoItem` and `TodoStatus` become the plan entry (`.text` to `.content`, `.status()` to `.status`); `latest_todo` and `TodoCache` (transcript scan) become `WorkflowSnapshot.plan`; `selected_chat` becomes the session key; the queue radii become constants | 1 import block: `zeron_doc`, `zeron_rpc`, `appshots`, `attachments`, `pickers`, `notice` and `state` are used only by the `Composer` wrapper and were removed; `zeron_proto` and `zeron_syntax` were aliased to the stand-ins |
| **Theme** | 2: `use … AgentTheme as Theme`; `Theme::of(cx)` becomes `Theme::of(window, cx)` | 12: 10 × `Theme::of(cx)` become `Theme::current(cx)`, one owned-versus-borrowed fix, and the `SyntaxPalette` path |
| **Kit** | 5: two `text_tooltip` calls become Cap's `Tooltip::new(..).view(cx)`; `queue_panel_surface` becomes a Cap-token tray surface | 0 |
| **Drop** | frost wrapper; 4 transcript-scan tests and 2 helpers; one `edge_fade` test that needs Zeron's popover | Dictation and voice: 6 types (98 lines), 4 fields, 13 methods (236 lines), 9 `cancel_dictation()` calls, 4 guards, 3 listeners, 1 action; frost around the mention tooltip |
| **Test setup** | `composer_tests` setup rewritten: `cap_gpui_hello::init` and the `AppState` stand-in replace `gpui_base`, `app_menus`, `history` and `settings` init | 2 new smoke tests |
| **Extraction** | none | 1: a dangling derive from cutting `composer.rs` at `ComposerEvent` |

Everything else compiled unchanged on the fork. That includes accessibility (`role`,
`aria_label`), `tab_index`, `focus_visible`, `tooltip_show_delay`, `track_scroll`,
`with_animation`, `EntityInputHandler`, custom `Element` layout, prepaint and paint, `paint_svg`,
`paint_image`, `StyledText::with_font_family_overrides`, `canvas`, `actions!` and `KeyBinding`
contexts.

### Time

| Step | Wall clock (agent) |
|---|---|
| Cold `cargo check --workspace --all-targets` of the restored S1 crate | 1 min 39 s |
| `cargo test --workspace` build and run after adding Cap's kit files and the todo panel | 2 min 11 s |
| Todo panel: first edit to compiling | 3 min 16 s (07:11:01 to 07:14:17) |
| Todo panel: to tests green | 5 min 51 s (to 07:16:52) |
| `ComposerInput`: extraction to compiling (74, then 5, then 0 errors) | 6 min 28 s (to 07:23:20) |
| `ComposerInput`: to smoke tests green | 7 min 18 s (to 07:24:10) |

Not included: about 25 minutes of reading and closure work that designed the stand-ins first.
Incremental `cargo check -p zeron-port` runs took 20 to 60 seconds.

The times show that GPUI and theme adaptation is a matter of minutes per file, so it does not drive
the estimates. Engineer-days go to the parts the spike stubbed:
- the `Composer` wrapper's 6,600 lines over `AppState` and `zeron_rpc`;
- transcript data from `zeron_doc`;
- driver events in three drivers;
- the engine and journal work in M5 phase 4;
- splitting files to 1,500 lines;
- real tests.

### Findings for later phases

- **`ComposerInput` is a complete text input.** It has its own `EntityInputHandler`, custom
  element, undo with coalescing, IME marked ranges, Markdown faces and inline chips. It duplicates
  Cap's `TextInput` (2,680 lines) rather than building on it.
  - M5 phase 2's pattern 1 says to build the composer on the kit `TextInput`. Re-implementing
    chips, mention projection and Markdown faces on Cap's input would cost about 3 extra days.
  - Recommendation: keep `ComposerInput` for the composer, and Cap's `TextInput` for every other
    field.
  - Both bind their editing chords in their own key context, so the M2 phase 4 keymap rule (a
    focused field consumes Ctrl+Z) holds for both. This is M5 phase 2's decision to record.
- **Cap's timeline snaps.** This corrects M4 `plan.md:34` and phase 5, which say it does not.
  - `editor_window.rs:4612` `arm_drag_snap` gathers zero, the playhead, the total, and every other
    segment's edges once when a drag arms. Shift bypasses it. The radius is `DRAG_SNAP_PX` = 8.
  - `snap_dragged_time` (`:4649`) applies it.
  - `editor_edits.rs:2170` `snap_split_time` snaps splits to the playhead and boundaries within
    `SPLIT_SNAP_PX` = 7, rejecting slivers.
  - This is the design M4 phase 5 task 4 describes, so snapping is a port.
- **Tail reservation.** Zui's `ListState` tail reservation implements Zeron's "own turn anchors at
  the top with reserved space below". It is not stick-to-bottom, which uses
  `STICK_THRESHOLD_PX`/`OVERDRAW_PX` on the fork's existing `ListState`.
  - M2 phase 6 does not need it.
  - If that anchoring is wanted later, the 180 lines go into the mirror as one commit.
- **`EdgeFade` horizontal bands.** zui's per-edge horizontal band overrides are about 15 lines in
  `window.rs`. The fork's shader parameters (`scene.rs:570`) already carry `band_left` and
  `band_right`, but its public `EdgeFade` (`window.rs:682`) has only `band_top` and `band_bottom`
  overrides. Add the two fields to the mirror only if a horizontally scrolling strip needs them.
- **Fixed in the fork version (pinned by S1).** Do not take zui's `Window::blur()` call sites
  verbatim, because the mirror carries upstream's `blur(cx)`.

## Cap types in the layout files and their `studio-proto` replacements

`studio-proto` does not exist yet. `desktop/crates/` holds `studio-agent-spike`,
`studio-bootstrap`, `studio-engine`, `studio-presets`, `studio-project`, `studio-sdk` and
`studio-test-support`. The snapshots below are the names M2 phase 3 plans: `ProjectSnapshot`,
`PreviewSnapshot`, `AudioSnapshot`, `BuildSnapshot` and `WorkflowSnapshot`, plus M4's
`TimelineSnapshot`. Counts are `cap_*` symbol uses in non-test code.

| File (non-test lines) | Cap types it depends on | Replaced by | Dropped |
|---|---|---|---|
| `editor_window.rs`, layout parts only (about 1,930 lines). The parts are `EditorSectionView` (`:712`), `render_header` (`:9247`, 180), `render_export_button`, `render_player`, `render_notice`, `render_player_toolbar` (`:9646`), `render_player_actions`, `render_preview_quality`, `render_preview_canvas`, `render_error_state`, `render_transport` (`:9942`, 223), `render_timeline` and its header, and `impl Render` (`:11141`, 686). `render_timeline_body` (681) belongs to M4 phase 4 | `cap_editor::{EditorInstance, EditorState}` and `LoadState` with `editor_preparing` (load and prepare state); `cap_project::{RecordingMeta, StudioRecordingMeta, ProjectRecordingsMeta, SharingMeta}` (header: name and path); `AspectRatio` (20, toolbar menu); `ProjectConfiguration` and `TimelineConfiguration` (sections); `EditorState.playhead_position` and `TransportHandle` (transport); `store::EditorPreviewQuality`; `presets` (11) | `BuildSnapshot` and `PreviewSnapshot` (load, build and first-frame state); `ProjectSnapshot` (name, path, video size, scenes, presets); `TimelineSnapshot` (timeline section); `AudioSnapshot` and the playback clock (transport, until M4 phase 6); app settings (preview quality) | `AspectRatio` (fframes' size is fixed in code); sharing; cursor, captions and keyboard data; `cap_recording::recovery`; `editor_preparing`; `import`; `transcription`; the clip-speed, add-track and preset dialogs |
| `editor_panels.rs` (7,934) | `cap_project` segment types (27): Camera3D, Crop, Mask, Text, Zoom, Scene, Caption, Keyboard, Audio and Clip configurations; `cap_rendering::{system_font_families, camera3d}` | `TimelineSnapshot` (selected clip's trim and order) and `ProjectSnapshot` (scene list and properties) for an inspector built from the kit's `Field`, `NumberField`, `Slider` and `SegmentedControl` | Every per-segment inspector (text 437, camera3d 428, scene, zoom, mask, audio panels, presets, motion templates): about 90% of the file. Only the panel frame and row patterns are reused |
| `editor_sidebar.rs` (4,068) | `cap_project` style types (10): `BackgroundSource`, `BorderConfiguration`, `ShadowConfiguration`, `NotchConfiguration`, `CornerStyle`, …; `cap_cursor_info::CursorShape`; `cap_gpui_kernels::{codec, texture}` | `ProjectSnapshot` (presets, project, scenes) and `WorkflowSnapshot` (agent tab badge) for the left column; the tab strip from `SidebarState` and the kit's `TabRail` | Background, wallpaper, gradient, border and notch panes. Also `editor_tabs.rs` (3,570 lines of Camera, Audio, Cursor, Keyboard and Captions tabs), which the M2 phase 4 list does not name |
| `settings_window.rs` (4,658) | No `cap_*` types. It reaches `auth` (20), `updates` (10), `app_windows`, `platform`, `upload`, `import`, `devices` | A setup snapshot from the adapter-detection cache (M2 phase 3 pattern 2; the plan does not name its type) and app settings (`ThemePreference`) | Account, recordings list, upload, self-host, storage, recording quality, excluded windows. Port only the window frame, the page rail (`TabRail`) and `render_appearance` (74) |
| `settings_pages.rs` (8,130) | `cap_project::XY` (2); `auth`, `hotkeys`, `updates`, `session`, `library` | Setup snapshot (adapter setup page); provider profiles (M2 phase 6) | Shortcuts, CLI status, feedback, diagnostics, license, S3 and Google Drive, and the template rule editor: everything but the page and row layout patterns |

Gap for M2 phase 3: settings and setup need a snapshot that the planned five do not cover. That is
adapter detection, provider profiles and app settings. Name it there (for example `SetupSnapshot`)
rather than in phase 4.

## Re-estimates

| Phase | Old | New | Reason |
|---|---|---|---|
| M2 phase 2: Cap theme and kit | 3-4d | 3-4d (unchanged) | The kit and theme compile verbatim, so porting them is about half a day. The time goes to threading the theme through about 111 call sites, replacing 18 components and tests, which S4 does not shrink |
| M2 phase 4: shell split into Cap's layout | 5-7d | 6-9d | The layout parts are a rewrite against snapshots, not a port. Every `render_*` reads `EditorWindow` state built on `cap_editor`/`cap_project`. `editor_panels`, `editor_sidebar` and the settings files are 85-95% Cap-specific content, so only frames and row patterns carry over. This comes on top of moving `studio_shell.rs` and the parity suite |
| M2 phase 6: transcript row model | 3-4d (placeholder) | 3-5d | Row identity, list state and stick-to-bottom work on the fork's `ListState`. But `transcript.rs` is 10.3k lines tangled with `zeron_doc` (66 uses) and `attachments`, so the kept part is written new with Zeron as reference. Skip zui's tail reservation |
| M4 phase 4: timeline view | 4-6d | 5-7d | About 1,800 of 3,535 `editor_timeline` lines port with type substitution. `TimelineModel`/`TrackLanes` and most of `editor_clips.rs` (3,392 lines over `cap_project`, `recording`, `import`) must be rewritten over `TimelineSnapshot`. No GPUI changes |
| M4 phase 5: trim, split, reorder, snapping | 3-4d | 2.5-3.5d | Snapping is a port (`arm_drag_snap`, `snap_dragged_time`, `snap_split_time`), not new design. The gesture geometry (`hit_test`, `DragBounds`, `split_at`, about 900 lines) ports with type substitution |
| M4 phase 6: playback pump | 3-4d | 2.5-3.5d | The UI half is small: `PreviewFrameView`, `PumpStats`, `PlayheadSignal` and `playhead_extrapolation` (about 350 lines). fframes already has `frame_image.rs` with `drop_image`. What remains is the engine's packed epoch value, the counter, removing the timer, and tests |
| M4 phase 7: export panel | 3-5d | 3-4d | Only the settings fields, `ExportPhase` and the progress overlay port (about 40% of 2,984 lines) with kit components that compile verbatim. The preview pane (`cap_export::preview`), share, link, upload and auth flows are dropped |
| M5 phase 1: streaming markdown | 3-5d | 3-5d (unchanged) | No zui-only API in `markdown/*.rs`. `zeron_markdown` (2,160 lines, pulldown-cmark only) and `zeron_veil` (606) port, and one `gpui_base::Checkbox` becomes Cap's `Toggle`. The five-grammar module, the link rewrite and the CI timing check are new work S4 does not change |
| M5 phase 2: composer | 5-7d | 5-7d (unchanged), with `ComposerInput` kept | The input ported in minutes with one API change. The effort is the wrapper rewrite over `WorkflowSnapshot` and commands, the driver commands event in three drivers, image transcoding, confined attachments, steering and the fake-agent tests. Building on the kit `TextInput` instead would make it 8-10d |
| M5 phase 3: todo panel | 2-3d | 1-2d | The panel and its gpui tests already run on the fork against an ACP-shaped plan entry. What remains is the `Plan` event in three drivers, the snapshot field and the driver test |
| M5 phase 4: diff review | 4-6d | 4-6d (unchanged) | `changes.rs` has no zui-only API. Its viewer ports after replacing `zeron_syntax` (phase 1) and dropping review comments. The estimate is dominated by engine work: per-file deltas, rejection into a validated candidate, per-path revert with a journal format bump, and three-platform tests |
| M5 phase 5: session history | 3-4d | 3-4d (unchanged) | Nothing is ported (scoring idea only) |

Milestone totals: M2 is 27-38d (was 26-35d), M4 is 24-33d (was 24-34d), and M5 is 16-24d (was
17-25d). The roadmap's milestone table and its 110-156d sum still show the old figures.

## Scratch artifacts

These are not in the repository; they live in the session scratch directory under `s4/`:
- `desktop/spikes/cap-gpui-hello/` (with `zeron-port/`);
- `zeron-port/{todo_edits,edge_fade_edits,ci_edits}.txt` (the categorized edit logs);
- `timelog.txt`;
- `tools/closure.py`, `tools/apidiff.py`, `tools/captypes.py`;
- `zui/` (the sparse zui checkout).

Re-run with `RUSTUP_TOOLCHAIN=1.98.1-x86_64-pc-windows-msvc cargo test --workspace` in the spike.
The `zeron-port` member adds pulldown-cmark 0.12, serde and serde_json, so run it without
`--locked`.

## Open items

- M5 phase 2: confirm the `ComposerInput` choice when the phase starts.
- M2 phase 3: name the setup snapshot.
- M4 `plan.md` and phase 5 "Port sources" said Cap only snaps to zero. Both sentences are now
  corrected to say snapping ports from `arm_drag_snap`, `snap_dragged_time` and `snap_split_time`.
