---
phase: 4
title: "Port Cap's timeline view"
status: pending
priority: P1
effort: "4-6d"
dependencies: [3]
---

# Phase 4: Port Cap's timeline view

## Goal

Replace today's timeline view with Cap's structure: a `Transform` zoom and pan model, off-screen culling, a minimap, a ruler, segment rendering and an audio waveform, fed by `TimelineSnapshot`.

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

- [ ] Timeline cached; playhead in its own child; maths ported first

- [ ] Scout current behaviors
- [ ] Transform and model
- [ ] Ruler, segments, minimap
- [ ] Waveform peaks and drawing
- [ ] Tests

## Success criteria

Parity timeline tests pass; a 30-clip project paints only visible segments (counted in a test); no file under `studio-ui/src/timeline` exceeds 1,500 lines.
