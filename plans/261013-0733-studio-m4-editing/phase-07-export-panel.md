---
phase: 7
title: "Export panel"
status: pending
priority: P2
effort: "3-5d"
dependencies: [2, 3]
---

# Phase 7: Export panel

## Goal

A Cap-style export panel with container, codec, scale and fps presets, a size estimate, progress and cancel, using the export options added in phase 1 and bundled in phase 2.

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

- [ ] Scout export service
- [ ] Presets
- [ ] Estimate
- [ ] Progress, cancel, persistence
- [ ] Unmigrated state
- [ ] Tests

## Success criteria

An export from the panel on this VM matches preview frames by the M4 comparison rule (pre-encode pixel equality, decoded output within the `odiff` threshold); cancel stops the encoder within a second; parity export test green.
