---
phase: 4
title: "Phase 4: Renderer setting, metrics and backend label"
status: pending
priority: P1
effort: 1d
dependencies: [3, "261019-1437-studio-layout-a committed"]
---

# Phase 4: Renderer setting, metrics and backend label

<!-- Updated: Validation Session 1 - add the Settings renderer picker (Auto / GPU / Skia CPU / CPU) -->

## Goal

The user picks the renderer in Settings: Auto, GPU, Skia CPU or CPU. The running backend and the
worker's per-frame render time are visible in two places: the observation JSON, for measurement and
tests, and a short status bar label.

## Context

- App settings live in `desktop/crates/studio-engine/src/app_settings.rs` (`AppSettings`, with `theme`
  and the export choice). Loading tolerates unknown fields and keeps the valid ones when one field is
  invalid (tests at `:68-122`).
- The Settings view is `desktop/crates/studio-ui/src/shell/settings.rs`, with pages `Appearance` and
  `Agent` (`:28-36`). Saves go through `SettingsWriter`, and a later choice overtakes an earlier save
  (`:52-79`, test at `:243`).
- The worker sends `render_duration_micros` in every `ScaledFrameResponse`
  (`fframes-studio-protocol/src/preview.rs:165-174`, set at `fframes-studio-runtime/src/preview_worker.rs:552`),
  but the engine never reads it.
- `PumpMetrics` (`desktop/crates/studio-engine/src/services/preview_coordinator.rs:163-172`) keeps
  only `render_max_ms`. It times `worker.frame(...)` by wall clock (`:895-918`), so the number includes IPC.
- The metrics reach `desktop/crates/studio-ui/src/shell/observation.rs:327-328`.
- `settings.rs`, `status_strip.rs` and `observation.rs` have uncommitted edits from
  `261019-1437-studio-layout-a`. This phase starts after that work is committed.

## Files to create / modify

- Modify: `desktop/crates/studio-engine/src/app_settings.rs`. Add
  `renderer: RendererChoice { Auto, Gpu, SkiaCpu, Cpu }`, defaulting to `Auto` when the field is
  missing or invalid.
- Modify: `desktop/crates/studio-engine/src/services/renderer_preference.rs` (created in Phase 3). It
  reads the choice from `AppSettings`. Studio's `FFRAMES_STUDIO_RENDERER` still overrides it.
- Modify: `desktop/crates/studio-ui/src/shell/settings.rs`. Add a "Preview renderer" group to the
  Appearance page, built with the existing kit controls like the theme choice:
  - `Auto`: GPU when available, otherwise CPU.
  - `GPU`: fails when there is no GPU.
  - `Skia CPU`: draws shaders, slower.
  - `CPU`: fastest, no shaders.

  Saving uses `SettingsWriter`. After a save the coordinator relaunches the preview worker with the
  new `--renderer`. No rebuild is needed.
- Modify: `desktop/crates/studio-engine/src/services/preview_worker_client.rs`. `frame()` returns
  `render_duration_micros` with the frame.
- Modify: `desktop/crates/studio-engine/src/services/preview_coordinator.rs`. `PumpMetrics` gains a
  fixed window of the last 240 main-frame samples, holding worker render time and round trip. It
  exposes `worker_render_p50_ms`, `worker_render_p95_ms`, `round_trip_p95_ms` and `backend`. The
  window is a fixed array, so the pump does not allocate per frame.
- Modify: `desktop/crates/studio-ui/src/shell/observation.rs`. Add the new fields and the renderer choice.
- Modify: `desktop/crates/studio-ui/src/shell/status_strip.rs`. Add a label (`GPU`, `Skia CPU` or
  `CPU · no shaders`) with `backend_detail` as the tooltip: the Vulkan or Metal device, or the reason
  the GPU was not used.

## Tasks & steps

- [ ] Add `renderer` to `AppSettings`. Extend the existing tests in two ways. First, the choice
      survives a save and load. Second, an invalid value keeps the theme and gives `Auto`.
- [ ] Add the Settings group. A test like `the_options_are_system_light_and_dark` checks the four
      options and their order.
- [ ] Wire a save of the choice to a worker relaunch. Engine test: change `Auto` to `Cpu`. The new
      worker's Hello reports `cpu`, and no build starts.
- [ ] Return `render_duration_micros` from `PreviewWorkerClient::frame` and record it in the coordinator.
- [ ] Add the percentile window to `PumpMetrics`. Unit-test it with an asymmetric sample set where p50,
      p95 and max differ, for example 99 samples of 2 ms and one of 50 ms: p50 = 2, p95 = 2, max = 50.
      Also test a window that has wrapped, so old samples are gone.
- [ ] Expose the backend and the percentiles in the observation JSON. The backend comes from Hello. It
      changes when a frame response carries a new backend (a mid-session fallback, Phase 1), when the
      coordinator relaunches after a hang (Phase 3), or when the Settings choice changes.
- [ ] Add the status bar label with the existing kit and Cap tokens.
- [ ] Render and inspect screenshots of the Settings group and of three label states:
  - `skia-gpu`
  - `cpu` with a "no GPU" reason
  - `skia-cpu`
- [ ] Extend a parity test in `desktop/crates/studio-ui/tests/parity/` that builds a real worker. With the
      default on this VM, the observation JSON reports `backend == "cpu"`. After choosing Skia CPU it
      reports `skia-cpu`. Both have a non-zero `worker_render_p50_ms` after playback.

## Verification

- From `desktop/`: `cargo test -p studio-engine` and `cargo test -p studio-ui`
- The screenshots above, read and checked.

## Risks

- Status bar crowding at narrow widths. The label is short and follows the bar's existing overflow rules.
- Choosing `GPU` on a machine without one breaks the preview. The option's description says so, and
  the error names the reason.

## Carried over from Phase 3

These need the layout-a shell files, so they wait for that commit with the rest of this phase.

- [ ] The "Use SDK v5 for this project" action. Show it when `Presentation::sdk_repin` is set, and
      confirm before sending `Command::RepinSdk`.
- [ ] The automatic relaunch on `cpu` after a GPU hang, and the relaunch when the Settings choice
      changes. The session preference is already demoted. Mind that PCM artifact ids are
      nonce-keyed.
- [ ] Key thumbnails by the runtime backend from Hello. This means `thumbnail_cache.rs` and the
      key built in `timeline/view.rs`, plus the fixture at `app/tests/timeline_controls.rs:118`.
