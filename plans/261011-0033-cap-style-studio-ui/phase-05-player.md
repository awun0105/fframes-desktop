# Phase 5: Player card (toolbar, preview stage, transport)

Status: done (unverified on screen; native qualification not run). Depends on: Phase 4. Effort: 2-3 days.

## Goal

Turn the centre column's preview and the wrapping rows of command buttons (`studio_shell.rs:2958-3324`) into Cap's player card: a 44 px toolbar, the preview stage, and a 48 px transport bar. Telemetry-reported buttons stay visible.

Values: [design spec](../reports/xia-261011-0016-cap-design-system-spec.md), section 2 (stage padding 16, frame radius 6, canvas padding 4, preview shadow, transport order).

## Target

```
player card (card fill, radius 12, line border)
+----------------------------------------------------------------------+
| toolbar 44: [Badge: CPU preview · shaders unsupported] (i)           |
|             ............ audio: Default · 48 kHz [Next output]       |
|                                  [Retry audio] [No audio]            |
+----------------------------------------------------------------------+
| stage (stage colour, padding 16)                                     |
|    frame: letterboxed, radius 6, line ring, preview shadow           |
|    overlays: selection box, rectangle scope, canvas message chip     |
+----------------------------------------------------------------------+
| transport 48: 0:01.20 / 0:05.50 | |< > >| | [Build preview] [Cancel] |
|               [Mute]                                     [...]       |
+----------------------------------------------------------------------+
```

## Element map

| Current (`studio_shell.rs`) | New place and component | Telemetry probe |
| --- | --- | --- |
| `preview-surface` `:2958-3038` (border, rounded_lg, bg canvas, keyboard `:2965-2993`) | Stage container, keep id `preview-surface`, focus handling and all keys (Space, Left, Right, Home, End, `c`, `0`, Escape) | preview probe `:3040-3083` keeps writing `canvas_origin`, `canvas_extent`, `preview_extent` |
| Image paint (`preview_element.rs:54-92`, `paint_image(bounds, image_bounds, Default::default(), ...)`) | Same element; pass `Corners::all(px(6))` as the corner radii; draw the `line` ring and `Shadow::Preview` behind the letterboxed frame | |
| Empty texts `:3104-3114` ("Empty video", "No preview available", "Build preview to compile…") | Centred in the stage, `Title` and `Body` `text_2` | |
| Selection box `:3117-3128` (border ACCENT) | 2 px `accent` border plus `selection` fill, clipped to the frame | |
| Rectangle scope `:3129-3142` (border WARNING_TEXT) | 2 px `warning` dashed-look border (solid if dashes are not available), clipped to the frame | |
| Hint line `:3148` ("Select: click · Rectangle: Shift-drag · …") | Tooltip on an Info icon in the toolbar, with `Kbd` chips | |
| Canvas message `:3150-3156` | Chip at the bottom centre of the stage (`card` fill, Small text), only when set | |
| Preview label `:3158-3163` | `Badge` in the toolbar; tone Warning on failure, Info while building | |
| Audio label `:3165-3183` | Toolbar right, Small `text_2` | |
| `next-audio-output` "Next output", `retry-audio-output` "Retry audio", `no-audio-output` "No audio" | Toolbar right, `Button` `Editor`, same ids and labels | yes, all three |
| Displayed line `:3186-3200` ("Displayed rev · frame n / total · s") | Transport time readout `current / total` (Body 13 tabular; 12 px and total hidden when the player is under 620 wide); revision and frame number in its tooltip | |
| `previous-frame` "Previous frame" | Transport, `IconButton` 24, StepBack icon, tooltip keeps the label | yes |
| `play-pause` "Pause"/"Play" | Transport, `IconButton` 32, Play or Pause icon, tooltip with `Kbd` Space | yes |
| `next-frame` "Next frame" | Transport, `IconButton` 24, StepForward icon | yes |
| `build-preview` "Build preview" | Transport, `Button` `Primary` with Hammer icon | yes |
| `cancel-preview` "Cancel build" | Transport, `Button` `Editor` with Square icon, always rendered and disabled when no build runs (as today), so its probe always reports | yes |
| `mute-audio` "Mute (M)"/"Unmute (M)" | Transport, `IconButton` Volume2 or VolumeX, tooltip with `Kbd` M | yes |
| `refresh`, `checkpoint` "Save checkpoint", `restore` "Restore as copy…", `draft` "Reveal retained draft", `reveal-checkpoint` | Overflow `Menu` from a MoreHorizontal `IconButton` at the right of the transport; menu items keep the labels, and each item's action is the same command | no (not in the telemetry list) |

Icon-only buttons keep their element ids, and their old text label becomes the tooltip and accessible label, so name lookups by id still work.

## Steps

1. Factor `render_player_area` (from Phase 4) into `render_player_toolbar`, `render_stage`, `render_transport`.
2. Stage geometry in one function `stage_layout(bounds, frame_size) -> StageLayout { frame_rect, scale }` used by painting, the overlays, the preview probe and `CanvasViewState::select_at` (`canvas_view.rs:140`), so click mapping and drawing agree. Keep `preview_scale` (`frame_image.rs`) behavior: no upscaling, cap at 1280 x 720.
3. Toolbar and transport with Phase 3 components; transport compact layout under 760 px player width (time readout shrinks, Build and Cancel become icon buttons with tooltips; ids unchanged). Compact layout is selected by a pure `transport_mode(player_width)` function with a unit test.
4. Overflow menu with the five actions.
5. Keep the enable conditions exactly as today (listed in the inventory: e.g. `play-pause` when `total_frames > 0`, `build-preview` when enabled and the worker is available).

## Tests

- `cargo test -p fframes-studio --test canvas_selection` and the `frame_image` tests: click mapping unchanged after padding and rounding.
- New: `stage_layout_letterboxes_and_never_upscales`, `stage_layout_round_trips_click_points` (frame point to window point and back).
- Native: `tests/x11_shell.rs` clicks `build-preview` and `next-frame`, and `desktop/scripts/qualify-m2-playback.py` clicks `play-pause`, `mute-audio`, `previous-frame`, `next-frame`, `timeline-fit`, `next-audio-output`, `no-audio-output` and `retry-audio-output` and clicks points on `ruler_bounds`; both must pass; `qualify-linux-native.py` and `qualify-windows-native.py` click `source_click` in the preview and must still select the marked title (run on a machine that can; record otherwise).

## Validation

On the user's machine: build a preview, play, pause with Space, step, mute with M, switch audio output, open the overflow menu with the keyboard, resize the window from 1280 to 1920 wide and check the frame stays centred and sharp-cornered at radius 6.

## Done when

The player card matches the target, all ten telemetry ids except `close` live in the toolbar or transport and report bounds, and selection and click mapping tests pass.

## Risks and rollback

Clipping overlays to a rounded frame: draw them inside the same frame rect with `overflow_hidden` and matching radius. Moving audio buttons into the toolbar changes their coordinates only; telemetry reports the new positions. Rollback: revert this phase.
