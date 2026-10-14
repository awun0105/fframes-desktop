---
phase: 6
title: "Playback pipeline"
status: pending
priority: P1
effort: "2.5-3.5d"
dependencies: [3]
---

# Phase 6: Playback pipeline

## Goal

Replace the 16 ms tick with Cap's latest-wins wake and frame pump while keeping the existing `PlaybackClock` and its epoch fencing as the transport.

Effort was 3-4d before [spike S4](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md): the UI frame path (`PreviewFrameView`, `PumpStats`, `PlayheadSignal`, `playhead_extrapolation`) is about 350 lines and fframes already converts frames; the engine side dominates.

## Port sources

Cap `editor_window.rs`: the one-slot wake (`flume::bounded(1)`) and main-thread pump around `PlayheadSignal`, and `playhead_extrapolation`. Not Cap's `AtomicU32` position: it has no epoch. Keep Cap's rule that foreign-thread callbacks never touch a gpui context.

## Tasks

1. Scout the tick path (moved behind the engine in M2 phase 3), `PlaybackClock` (`studio-engine/src/playback_clock.rs`: epoch on seek, pause, reinstall, fallback; `accept_output` rejects other epochs), the audio output snapshot, and `ImagePresentationManager`.
2. Engine side: on each audio output snapshot or clock step, publish a packed value (epoch, timing generation, frame) into an atomic and wake the one-slot channel. Stale epochs or generations are dropped at publish and again at present.
3. UI pump drains the wake on the main thread, requests the frame for the newest value, and drops frames older than the last presented one.
4. Playhead drawn from extrapolated time between samples, clamped to the current timing-aware duration.
5. Add an out-of-order presentation counter to `ImagePresentationManager`.
6. Remove the 16 ms timer.
7. Tests: burst of updates is latest-wins; a pre-seek audio callback after a seek is dropped; after a `SetTiming` that shortens the video the playhead clamps; the counter stays zero.

## Reference patterns (Cap, Zeron)

From [the Zeron report](../reports/research-261013-1749-zeron-agent-control.md) and [the Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md); see the roadmap's pattern map.

1. Producer: preview frames arrive on foreign threads. The callback (Cap `make_frame_callback`, `editor_window.rs:11779`) never touches a gpui context and never blocks. Do not copy Cap's channel as is: `try_send` on a full channel discards the new frame and keeps the old one, and Cap's Windows `send_timeout(100 ms)` blocks the producer. Instead the producer overwrites a single slot (`Mutex<Option<Frame>>`, or a one-slot channel it drains before sending) and wakes the pump, so the newest frame always wins.
2. Conversion: un-pad the row stride and swizzle RGBA to BGRA into a `gpui::RenderImage` (Cap `frame_image`, `:563`). Measure the conversion at 1080p in a debug build: Cap measured 42.7 ms unoptimised and 1.0 ms at `opt-level = 3`. If it exceeds 4 ms, add `[profile.dev.package.<crate holding the loop>] opt-level = 3` to `desktop/Cargo.toml` (no new crate).
3. Painting: a preview entity whose `canvas` calls `paint_image` (Cap `PreviewFrameView`, `:608`); notify only that entity; release the replaced image with `window.drop_image`. No zero-copy surface: the fork's `paint_surface_fitted` paints CVPixelBuffer/IOSurface frames, while fframes preview frames arrive from the worker process as RGBA.
4. Test: a burst of 100 frames from a foreign thread never blocks the producer, and the frame the pump presents next is the 100th.

## Todo

- [ ] Non-blocking frame channel, measured conversion, single-entity paint, burst test

- [ ] Scout
- [ ] Packed value with epoch and generation
- [ ] UI pump
- [ ] Extrapolated, clamped playhead
- [ ] Out-of-order counter
- [ ] Timer removed
- [ ] Tests

## Success criteria

`grep -rn "from_millis(16)" desktop/` is empty; parity transport tests pass; the counter reads zero after seeking during playback on this VM.
