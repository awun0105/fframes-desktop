---
title: "M4 Editing: timing doc, SDK release, Cap timeline, playback, export panel"
description: "Add the timeline.json timing model end to end with its own timing revision, ship it and the extended export contract in an SDK release with project migration, and port Cap's timeline, edit history, playhead wake and export panel."
status: pending
priority: P1
effort: "24-33d"
branch: feat/studio-rewrite
tags: [desktop, feature, gpui, frontend]
blockedBy: [261013-0733-studio-m2-architecture, 261013-0733-studio-m3-durable-writes]
blocks: [261013-0733-studio-m6-cleanup]
created: 2026-10-13
---

# M4 Editing

## Overview

Part of [the studio rewrite roadmap](../261013-0733-studio-rewrite-roadmap/plan.md). Code defines what each scene draws; `timeline.json` defines when it plays. Three facts shape the design:

- Every file under the project root is source: a change bumps the source revision, which drops the build and interrupts running work (`studio-project/src/revision.rs` `excluded()`, `studio-engine/src/state.rs` `reconcile_source`/`invalidate_source`). So `timeline.json` gets its own timing revision outside the build key.
- Projects pin `fframes =1.2.0` and the studio runtime and protocol `=0.1.0`, enforced by `build_materialization.rs`, and own their worker and export code (`templates/src/bin/studio_worker.rs`, copied at creation). So the hook ships in an SDK release with a project migration.
- The studio template has no scenes. The migration and new projects get a multi-scene `Duration::Auto` template.

Accepted design from [spike S2](../261013-0733-studio-m1-foundations/reports/spike-timing-hook.md) (decided 2026-10-14), which phases 1 and 2 build on:

- Trims and splits live on the resolved timeline entries, not in wrapper scenes, so `get_scene_info`'s pointer lookup keeps working. It answers with a scene's first clip and keeps the scene's own `duration_in_frames`.
- Timing keys are `editor_instance_key()` or `name()`. Later repeats get `#2`, `#3`, â€¦ in code order.
- A clip's editor key is the scene's key for its first clip, and `key#n` for later clips of a split.
- A trimmed edge is a hard cut: the overlap applies only on untrimmed edges, and an overlap reaching before frame 0 is clamped.
- Scene audio moves and is trimmed with its clip. Top-level `Video::audio()` music stays on video time when scenes are reordered.

Cap sources (pinned `a32e4806b`, `apps/desktop-gpui/src`; on the same patched GPUI fork as the app, so ports map Cap's project types to fframes types but need no GPUI API changes):

| Source | Lines | What to take |
|--------|-------|--------------|
| `editor_edits.rs` | 4,360 | snapshot history with `pause()`/`resume()` drag coalescing; split, trim and move maths |
| `editor_timeline.rs` | 4,813 | `Transform`, `segment_visible`, `time_from_x`, `TrackKind`, `TimelineModel`, `waveform_path`, `render_minimap`, `render_segment` |
| `editor_window.rs` | 12,661 | drag state, `split_at_playhead`, undo/redo wiring, the latest-wins wake and frame pump around `PlayheadSignal` |
| `editor_export.rs` | 3,120 | `ExportPhase`, `ExportUi` (global persistence), `start_export`, and the panel built from Cap's kit |
| `editor_clips.rs` | 4,103 | clip track rendering, clip selection and clip drag on the timeline |

Snapping (part of the confirmed editing scope) ports from Cap: `arm_drag_snap` and `snap_dragged_time` in `editor_window.rs` snap drags to zero, the playhead, the total and other segment edges, and `snap_split_time` in `editor_edits.rs` snaps splits (corrected by spike S4).

## Phases

| # | Phase | Effort | Depends on | Status |
|---|-------|--------|-----------|--------|
| 1 | [Timing hook and export options in fframes and the runtime](./phase-01-timing-doc-runtime.md) | 4-5d | none | Completed |
| 2 | [SDK release, template and project migration](./phase-02-sdk-release-and-migration.md) | 4-6d | 1 | Completed |
| 3 | [Engine timing document and edit history](./phase-03-engine-timeline-doc.md) | 3-4d | 1 (manual check needs 2) | Completed |
| 4 | [Port Cap's timeline view](./phase-04-timeline-view-port.md) | 5-7d | 3 | Pending |
| 5 | [Trim, split, reorder and snapping](./phase-05-timeline-edit-interactions.md) | 2.5-3.5d | 2, 4 | Pending |
| 6 | [Playback pipeline](./phase-06-playback-pump.md) | 2.5-3.5d | 3 | Pending |
| 7 | [Export panel](./phase-07-export-panel.md) | 3-4d | 2, 3 | Pending |

Spike S4 ([port closure](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md)) re-estimated phases 4-7 and found that Cap's timeline snaps, so phase 5 ports snapping.

## Success criteria

- [ ] In a project created from (or migrated to) the new template, trimming, splitting or reordering a scene updates the preview with no build queued, writes `timeline.json` at gesture end, and Ctrl+Z restores the previous file.
- [ ] An export with an edited `timeline.json` matches preview frames at three times: the renderer's frames before encoding are pixel-identical to the preview's, and the decoded output is within the `odiff` threshold the e2e tests use.
- [ ] Projects not yet migrated keep previewing; editing shows "migrate project" with a reason.
- [ ] Playback runs without the 16 ms tick; a new out-of-order presentation counter stays at zero across seek and timing changes.
- [ ] The export panel offers container, codec, scale and fps presets, an estimate, progress and cancel.
- [ ] Parity tests stay green; gpui tests cover each edit, undo/redo, playhead, migration and export state.
