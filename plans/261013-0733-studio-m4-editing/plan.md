---
title: "M4 Editing: timing doc, SDK release, Cap timeline, playback, export panel"
description: "Add the timeline.json timing model end to end with its own timing revision, ship it and the extended export contract in an SDK release with project migration, and port Cap's timeline, edit history, playhead wake and export panel."
status: completed
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
| 4 | [Port Cap's timeline view](./phase-04-timeline-view-port.md) | 5-7d | 3 | Completed |
| 5 | [Trim, split, reorder and snapping](./phase-05-timeline-edit-interactions.md) | 2.5-3.5d | 2, 4 | Completed; owner Windows hand checks passed 2026-10-18 |
| 6 | [Playback pipeline](./phase-06-playback-pump.md) | 2.5-3.5d | 3 | Completed; owner Windows playback/audio hand checks passed 2026-10-18 |
| 7 | [Export panel](./phase-07-export-panel.md) | 3-4d | 2, 3 | Completed; real-SDK pixel comparison and owner hand checks passed 2026-10-18 |

Spike S4 ([port closure](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md)) re-estimated phases 4-7 and found that Cap's timeline snaps, so phase 5 ports snapping.

## Success criteria

Checked on 2026-10-16 against the code, the default test runs and CI (the self-hosted Linux and Windows validation and the macOS run all passed on `ab47dc9`).

**Closed 2026-10-18:** the owner confirmed all Windows app hand checks passed. The ignored real-SDK qualification was run, and commit `8fd8425` adds a stronger assembled-SDK export comparison: preview matches the pre-encode CPU renderer byte for byte at frames 0, 135 and 269 after reorder/trim; decoded H.264 passes the existing e2e `odiff -t 0.4` threshold at all three times; a visibly different image is rejected. This establishes the comparison for H.264, not every lossy codec: an initial MPEG-4 attempt differed at one pixel under that strict threshold. [Current evidence](../261013-0733-studio-m6-cleanup/reports/final-fixes-2026-10-18.md). M6's owner-approved refusal policy supersedes the historical migration criterion below; macOS remains CI-only under the accepted platform scope.

- [x] In a project created from the new template, trimming, splitting or reordering updates preview without a build, writes `timeline.json`, and Ctrl+Z restores it. Engine/view/parity checks and real-worker replays pass; the owner accepted the Windows app hand checks on 2026-10-18.
- [x] An export with edited `timeline.json` matches preview at three times: pixel identity before encoding and decoded output within the e2e `odiff` threshold. `studio-engine/tests/timing_retime.rs::edited_sdk_export_matches_preview_and_preencode_at_three_times` passes against the assembled SDK with H.264, as detailed above. The earlier runtime centre-pixel proxy is retained as additional coverage.
- [x] Projects not yet migrated keep previewing; editing shows "migrate project" with a reason. `studio-ui/tests/parity/timeline.rs::edits_wait_for_a_migrated_project_and_say_why`, the engine's `project_migration.rs`, and the export panel test for an unmigrated project.
- [x] Playback runs without the 16 ms tick; a new out-of-order presentation counter stays at zero across seek and timing changes. The grep finds only the test harness's fake clock step. The counter is asserted across seeks during playback (`transport.rs`), across a rebuild (`services.rs`) and across a split followed by a seek (`timeline.rs::a_split_then_a_seek_does_not_present_frames_out_of_order`, against the fake worker's opt-in `timing` mode).
- [x] The export panel offers container, codec, scale and fps presets, an estimate, progress and cancel. Unit tests for the options and the estimate; parity tests for a running export, cancel, a failure with its reason, the unmigrated state, and no project or no SDK (the panel does not open). `every_reported_format_exports_at_the_requested_size_and_rate` exports every format the SDK's FFmpeg reports (mpeg4, libx264, libx265, libvpx-vp9 here).
- [x] Parity tests stay green; gpui tests cover each edit, undo/redo, playhead, migration and export state. The parity suite passes, and the timeline view tests cover the gestures. The parity fixture can now run timeline-ready (fake worker `timing` mode) for a split; undo and redo are covered by a key binding test and the engine, and through a real worker only by the ignored `SDK_BUNDLE` tests.

## Open items

No outstanding Windows acceptance item. The real-SDK checks and owner hand checks close the original three follow-ups. The panel still follows the formats its SDK reports; the strict decoded pixel comparison above is qualified for H.264. The historical slow-macOS-runner observation remains documented in phase 6, with the test waiting for the last frame to paint; macOS hand checks remain outside the accepted scope.
