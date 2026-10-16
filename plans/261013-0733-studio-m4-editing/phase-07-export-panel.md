---
phase: 7
title: "Export panel"
status: done-with-concerns
priority: P2
effort: "3-4d"
dependencies: [2, 3]
---

# Phase 7: Export panel

## Goal

A Cap-style export panel with container, codec, scale and fps presets, a size estimate, progress and cancel, using the export options added in phase 1 and bundled in phase 2.

Effort was 3-5d before [spike S4](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md): only the settings fields, `ExportPhase` and the progress overlay port; preview pane, share and upload flows are dropped.

## Port sources

Cap `editor_export.rs` (3,120 lines): `ExportPhase` (`is_busy`, `shows_progress`), `ExportUi` (load and persist settings globally, like Cap), `start_export`. Target `studio-ui/src/export/{panel,settings,progress}.rs`, keeping Cap's panel layout and kit components (`Select`, `SegmentedControl`, `Progress`, `Button`); backend calls become `EngineCommand`s to the export service.

## Tasks

1. Scout the export service and `tests/native-export.rs` (moved to the engine): progress reporting and cancel.
2. Presets: MP4 H.264, MP4 HEVC, WebM VP9, each at 100%, 50% and the video's fps or 30 fps; only combinations the worker accepts.
3. Estimate: bitrate times timing-aware duration, labelled as an estimate.
4. Progress events and cancel through the engine; settings persisted globally.
5. Unmigrated projects: only the default MPEG-4 export, with the migrate notice.
6. gpui tests: phase transitions, cancel, failure shown with its reason, unmigrated state.

## Todo

- [x] Scout export service
- [x] Presets
- [x] Estimate
- [x] Progress, cancel, persistence
- [x] Unmigrated state
- [x] Tests

## Success criteria

An export from the panel on this VM matches preview frames by the M4 comparison rule (pre-encode pixel equality, decoded output within the `odiff` threshold); cancel stops the encoder within a second; parity export test green.

## Implementation notes

Deviations and open items:

- The worker is probed when the panel opens and when the accepted revision changes, not in the background once per displayed build (a cache hit for an unchanged project).
- `StartExport` is a new action; `ExportVideo` now opens the panel. The header action id `export-mp4` became `export`.
- The destination extension is forced to the container's (`.mp4` / `.webm`).
- The saved choice persists in app settings and is validated against the worker's reported formats before use.
- Unmigrated projects get only the default MPEG-4 export, with a migrate notice.
- Open: the real-export pixel match against the preview needs `SDK_BUNDLE`, which is not available on this host.
- This SDK's FFmpeg reports mpeg4, libx264, libx265, libvpx-vp9, libopus and aac; `every_reported_format_exports_at_the_requested_size_and_rate` exports each reported format. Presets follow the worker's reported formats.
- Open: manual VM checks for phases 5, 6 and 7 (panel on screen, cancel latency).
- Tests: gpui parity tests cover the fake worker answering as a legacy SDK (support unmigrated; the migration-available route is covered by the second test), an unmigrated project ignoring a saved non-default choice and still starting, cancel, failure with its reason and a running export. The `Ready` path (worker formats, estimate) has unit tests in `studio-engine/src/export_options.rs` and `studio-ui/src/export/` only.
- With no project or no SDK the panel does not open and an export cannot start (`the_export_panel_does_not_open_without_a_project`, `the_export_panel_does_not_open_without_an_sdk`).
