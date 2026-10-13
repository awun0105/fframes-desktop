# Phase 6: Timeline card

Status: implemented (native layout and qualification unverified here). Depends on: Phase 3 (and Phase 4 for its card slot). Effort: 1-2 days.

## Goal

Restyle `desktop/app/src/timeline_view.rs` (637 lines) to Cap's timeline using the [design spec](../reports/xia-261011-0016-cap-design-system-spec.md), sections 1 (track colours, segment mix) and 2 (geometry). Behavior and the model (`crates/studio-engine/src/timeline.rs`) do not change.

## Current state

- `TimelineView` (`:26`), events `TimelineEvent` (`:16`: Seek, Step, TogglePlayback, ToggleMute, BeginScrub, EndScrub, ScopeChanged).
- Header buttons via `fn button` (`:286`): `timeline-zoom-out` "−" (`:368`), `timeline-zoom-in` "+" (`:369`), `timeline-fit` "Fit" (`:370`, its bounds are captured as `fit_bounds` at `:324` and reported as `timeline-fit` in qualification telemetry), `timeline-cycle` "Cycle scene (S)" (`:377`).
- Regions `compiled-timeline` (`:408`) and `timeline-lanes` (`:502`).
- `qualification_metrics()` (`:120`) reports `ruler_bounds` (height 20 today), `fit_bounds`, selection and pixels per second.
- `pointer_down` (`:175`): range selection when Shift is held or when `y - top > 24`, otherwise scrub. The 24 is tied to today's 20 px ruler.
- Keys (`:232`): Space, `m`, Left, Right, Home, End, `+`/`=`, `-`, PageUp, PageDown, `s`, `[`, `]`.
- Colour sites: `:2`, `:392-417`, `:496`, `:532-536`, `:579`, `:592`, `:609-617`.

## Target

| Element | Target |
| --- | --- |
| Card | `card` fill, radius 12, `line` border, padding 11 (top 9) |
| Header row | `SectionTitle` "Timeline" left; right: `IconButton` ZoomOut (`timeline-zoom-out`), zoom `Slider` 72 wide with track 3 (new, drives the same zoom state), `IconButton` ZoomIn (`timeline-zoom-in`), `Button` `Editor` "Fit" (`timeline-fit`, keeps `fit_bounds` capture), `IconButton` Layers with tooltip "Cycle scene" and `Kbd` S (`timeline-cycle`) |
| Ruler | height 26; major ticks 8, minor 5, `line_strong`; labels Small `text_3`; tick ladder chosen so labels are at least 60 px apart |
| Track gutter | 104 wide plus 8 gap: icon and lane name (Small medium `text_2`) |
| Track rows | height 44, 6 apart, band radius 8, band fill `card_2` |
| Segments | radius 8, 3 px accent bar on the left in the track colour, label padding 13 left and 10 right, `segment_colors(kind, selected)` for fill, border and label; labels anchored to the visible part of long segments and truncated with an ellipsis |
| Track colours | repeated or clip segments `TrackKind::Clip`, overlays `TrackKind::Overlay`, audio `TrackKind::Audio` |
| Playhead | 1 px line, 12 px head with a 4 px `card` ring, `playhead` colour |
| Selection range | `selection` fill with 1 px `accent` edges |
| Hover ghost line | only if the view already tracks hover position; otherwise left out |

Cap puts zoom in the transport bar; fframes keeps it in the timeline header because `TimelineView` owns zoom state and the `timeline-fit` telemetry lives there (deliberate difference).

## Steps

1. Introduce geometry constants from `Metrics` (`ruler_h`, `track_h`, `track_gap`, `track_gutter_w`) and use them everywhere the current code uses literal sizes.
2. Fix the scrub or select boundary in `pointer_down`: replace the literal 24 with `padding_top + ruler_h` so clicking the ruler scrubs and clicking lanes selects, as today.
3. `qualification_metrics()` must report the real new `ruler_bounds` (height 26) and `fit_bounds`.
4. Header row with components; zoom slider maps its fraction to the existing zoom range (log scale between the current min and max zoom).
5. Restyle ruler, gutter, rows, segments, playhead and selection with tokens.
6. Remove all colour literals and old `colors::` uses (Phase 1 already changed their types).

## Tests

- `cargo test -p fframes-studio --test timeline_controls` (non-ignored parts) and the unit tests in `timeline_view.rs`.
- New (pure functions): `ruler_click_scrubs_and_lane_click_selects` over a `hit_zone(y, top) -> Ruler | Lanes` helper used by `pointer_down`; `zoom_slider_maps_to_zoom_range_and_back`; `segment_label_anchors_to_visible_slice`.
- Ignored SDK tests (`timeline_controls`, `preview_coordination`) on a machine with an SDK bundle; record otherwise.
- `tests/x11_shell.rs` (`:696`, `:808`, `:891`) and `qualify-m2-playback.py` (`:217`, `:288`) read `ruler_bounds` and click on it; they must pass with the 26 px ruler.

## Validation

On the user's machine: zoom with buttons, slider and `+`/`-`, Fit, scroll, scrub on the ruler, Shift-drag a range, cycle scenes with S, both appearances; labels never spill outside segments.

## Done when

The timeline matches the target, telemetry `ruler_bounds` and `timeline-fit` report correct new geometry, and the scrub or select boundary works at the new ruler height.

## Risks and rollback

Taller rows (44) show fewer lanes in a 260 px card: the lanes area scrolls. Changed ruler height shifts qualification click points: they come from `ruler_bounds`, which step 3 keeps accurate. Rollback: revert the file.
