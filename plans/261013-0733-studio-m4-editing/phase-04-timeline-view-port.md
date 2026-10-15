---
phase: 4
title: "Port Cap's timeline view"
status: completed
priority: P1
effort: "5-7d"
dependencies: [3]
---

# Phase 4: Port Cap's timeline view

## Goal

Replace today's timeline view with Cap's structure: a `Transform` zoom and pan model, off-screen culling, a minimap, a ruler, segment rendering and an audio waveform, fed by `TimelineSnapshot`.

Effort was 4-6d before [spike S4](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md): about half of `editor_timeline.rs` ports with type substitution, but `TimelineModel`, `TrackLanes` and the clip track are rewritten over `TimelineSnapshot`.

## Port sources

Cap `editor_timeline.rs` (4,813 lines) and `editor_clips.rs` (4,103 lines, clip track rendering and selection; add `clips.rs`). Target `studio-ui/src/timeline/`: `transform.rs` (`Transform`, `time_from_x`, `segment_visible`, fit on open), `model.rs` (`TimelineModel`, `TrackKind` mapped to clips and audio tracks), `ruler.rs`, `segment.rs` (labels and trim handles; no fades or holds), `waveform.rs`, `minimap.rs`, `view.rs`.

## Tasks

1. Scout the timeline module moved in M2 phase 4: keys, `TimelineSelection`, scene cycling, range selection.
2. Port the files above, mapping Cap's project and segment types to `TimelineSnapshot` (no GPUI API changes on the same fork) and keeping Cap's colours from `studio-theme` (segment tints from the Cap spec, section 1).
3. Waveform peaks computed in the engine from the audio service and cached by file hash; the view only draws.
4. Keep scene cycling, range selection, zoom and ruler seek working.
5. gpui tests: `Transform` maths (zoom around cursor, fit), culling counts, minimap click seeks.

## Reference patterns (Cap, Zeron)

From [the Zeron report](../reports/research-261013-1749-zeron-agent-control.md) and [the Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md); see the roadmap's pattern map.

1. The timeline is its own entity, mounted with `.cached(..)` by the shell (M2 phase 4). During playback only the playhead changes, so draw it in a small child view or overlay `canvas` that the playback pump notifies, not the whole timeline.
2. Port Cap's pure maths first (`Transform`, `time_from_x`, `segment_visible`) with unit tests, then the rendering that uses them.

## Todo

- [x] Timeline cached; playhead drawn outside it; maths ported first

- [x] Scout current behaviors
- [x] Transform and model
- [x] Ruler, segments, minimap
- [x] Waveform peaks and drawing
- [x] Tests

## Success criteria

Parity timeline tests pass; a 30-clip project paints only visible segments (counted in a test); no file under `studio-ui/src/timeline` exceeds 1,500 lines.

## Delivery notes

What was built:

- `studio_engine::TimelineTransform` replaces `TimelineViewport`. It is Cap's `Transform` (`zoom` visible seconds, `position` at the left edge) with `update_zoom`, `set_position`, `segment_visible` and the 2 s render padding. Zoom runs from 100,000 px/s to the whole video. Fit shows the whole video and is also the view a preview opens with; Cap's 600 s cap and the old 80 px/s rule are gone. A resize keeps the visible seconds. The telemetry keys `pixels_per_second` and `scroll_x` are kept, derived from the transform.
- The ruler uses Cap's resolution ladder, extended below 0.5 s to 0.01 s so single frames stay readable. Whole seconds are labelled `M:SS`; at 0.1 s spacing and below, every fifth mark is labelled with fractions. The engine emits every mark once, so the UI no longer computes minor ticks.
- `studio-ui/src/timeline/` (`view`, `model`, `ruler`, `segment`, `minimap`, `waveform`, `playhead`) replaces `timeline_view.rs`. The largest file is `view.rs` at about 1,150 lines.
- The scene rows come from `Presentation::timeline` (`TimelineView::set_snapshot`). Each clip maps to a report scene by timing key and occurrence (`clip_index`) and takes that scene's frames and lane, so `Overlap` scenes keep their overlapped start and row. A clip the displayed report does not have yet follows the previous segment on the clip track and cannot be selected. Report scenes no clip plays are appended. Without clips, the rows are the report's scenes. Audio rows come from the report.
- The waveform is a **Mix** row under the audio rows. Its peaks are the mean absolute level per 0.1 s (Cap's `waveform_peaks`), computed in `retain_audio_source`'s hash pass.
- The shell mounts the timeline with `.cached(size_full)`. The playhead is painted by the shell over it (`timeline::playhead_overlay`). `set_position` stores the frame without a notify. The footer no longer repeats the playhead time, which the transport shows.

Deviations:

- The transform lives in the engine (`studio-engine/src/timeline.rs`), not in `studio-ui/src/timeline/transform.rs`. Geometry, culling and thumbnails were already engine-owned, and one viewport serves both.
- Peaks are stored on `PreparedAudioSource`, next to the file whose `sha256` they belong to, instead of in a separate cache keyed by hash. The hash pass already reads every sample once, and the source lives as long as the preview that draws it, so a second cache would add nothing.
- The playhead is not a child view. In this gpui fork, a notify marks every ancestor view dirty, so a child of the cached timeline would render the timeline on every tick. The playhead is painted by the shell, which playback renders anyway.
- Trim handles on segments are left to the trim, split and reorder phase, which adds their interaction.

Evidence (Windows only):

- `cargo test -p studio-engine --test timeline` passes 14 tests: the ported guards plus Fit, exact doubling, the origin fraction, culling and the ruler ladder.
- The engine's waveform unit tests pass.
- `studio-ui` unit tests pass, 22 of them under `timeline`. They cover the model's clip paths, overlapping scenes keeping their frames and row and the retime match, the 30-clip culling count (9 of 30 painted at 3 s), the snapshot driving the clip track, the minimap chip and drag maths, and a minimap press that centres the view and is ignored at Fit.
- The parity suite passes 55 tests. The transport test now also asserts that playback does not render the timeline.
- `clippy --workspace --all-targets -D warnings` and `fmt` are clean.

Open:

- While the displayed report lags the document (a split not yet compiled), a clip it lacks follows the previous segment on the clip track and can sit over a mapped scene there until the retimed report arrives.
