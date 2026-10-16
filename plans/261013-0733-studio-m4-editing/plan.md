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
| 4 | [Port Cap's timeline view](./phase-04-timeline-view-port.md) | 5-7d | 3 | Completed |
| 5 | [Trim, split, reorder and snapping](./phase-05-timeline-edit-interactions.md) | 2.5-3.5d | 2, 4 | Done, manual check open |
| 6 | [Playback pipeline](./phase-06-playback-pump.md) | 2.5-3.5d | 3 | Done, manual check open |
| 7 | [Export panel](./phase-07-export-panel.md) | 3-4d | 2, 3 | Done with concerns |

Spike S4 ([port closure](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md)) re-estimated phases 4-7 and found that Cap's timeline snaps, so phase 5 ports snapping.

## Success criteria

Checked on 2026-10-16 against the code, the default test runs and CI (the self-hosted Linux and Windows validation and the macOS run all passed on `ab47dc9`).

- [ ] In a project created from (or migrated to) the new template, trimming, splitting or reordering a scene updates the preview with no build queued, writes `timeline.json` at gesture end, and Ctrl+Z restores the previous file. **Code and engine tests done; the check by hand in a real window is open.** The engine tests (`studio-engine/tests/timing.rs`) cover retime without a build, the single write per gesture and undo; the view tests (`studio-ui/src/timeline/view.rs`) cover trim, split, reorder and snapping; the parity suite covers a split through the fake worker's `timing` mode, and undo through a real worker is only in the `SDK_BUNDLE` tests, which are ignored here.
- [x] An export with an edited `timeline.json` matches preview frames at three times. `fframes-studio-runtime/tests/export.rs::an_exported_timing_document_matches_the_preview_frames` exports with a timing file, renders frames 0, 50 and 100 of the same video through `fframes::Previewer` with that document and compares each preview centre pixel with the decoded export (colour tolerance, since the export is lossy). **Proxy:** it runs at the runtime level on the CPU backend, not through the assembled SDK bundle, and it uses a pixel tolerance rather than `odiff` and does not check pre-encode pixel identity. `export_plays_the_timing_file_and_fails_without_it` and `fframes/src/renderer/tests/timing_tests.rs` cover the halves.
- [x] Projects not yet migrated keep previewing; editing shows "migrate project" with a reason. `studio-ui/tests/parity/timeline.rs::edits_wait_for_a_migrated_project_and_say_why`, the engine's `project_migration.rs`, and the export panel test for an unmigrated project.
- [x] Playback runs without the 16 ms tick; a new out-of-order presentation counter stays at zero across seek and timing changes. The grep finds only the test harness's fake clock step. The counter is asserted across seeks during playback (`transport.rs`), across a rebuild (`services.rs`) and across a split followed by a seek (`timeline.rs::a_split_then_a_seek_does_not_present_frames_out_of_order`, against the fake worker's opt-in `timing` mode).
- [x] The export panel offers container, codec, scale and fps presets, an estimate, progress and cancel. Unit tests for the options and the estimate; parity tests for a running export, cancel, a failure with its reason, the unmigrated state, and no project or no SDK (the panel does not open). `every_reported_format_exports_at_the_requested_size_and_rate` exports every format the SDK's FFmpeg reports (mpeg4, libx264, libx265, libvpx-vp9 here).
- [x] Parity tests stay green; gpui tests cover each edit, undo/redo, playhead, migration and export state. The parity suite passes, and the timeline view tests cover the gestures. The parity fixture can now run timeline-ready (fake worker `timing` mode) for a split; undo and redo are covered by a key binding test and the engine, and through a real worker only by the ignored `SDK_BUNDLE` tests.

## Open items

- Hand checks on the VM for phases 5, 6 and 7 (trim, split, reorder and undo in a window; seeking during playback; the export panel and the cancel time).
- Export versus preview through the assembled SDK bundle with `odiff` (the runtime-level proxy test passes).
- Real-SDK qualification (`SDK_BUNDLE`, ignored here), including undo through a real timeline-ready worker. A different SDK bundle's FFmpeg may report other formats; the panel follows what the worker reports.
- Phase 6 note: `position` reached 90 (one past the last frame) while the shown frame was 86 on the slow macOS runner; the last frame is only asserted once painted.
