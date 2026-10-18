---
phase: 1
title: "Timing hook and export options in fframes and the runtime"
status: completed
priority: P1
effort: "4-5d"
dependencies: []
---

# Phase 1: Timing hook and export options in fframes and the runtime

## Goal

Turn spike S2 into production code: an opt-in, bounded timing hook in `fframes`, a `SetTiming` capability in the preview worker, export options (container, codec, scale, fps) in the worker export path, and caches that know about timing.

## Context

- Spike rules: `../261013-0733-studio-m1-foundations/reports/spike-timing-hook.md`
- `fframes/src/scenes.rs`, `fframes/src/frame.rs` (`clone_with_scene_offset`), `fframes/src/fframes_context.rs` (`get_scene_info`, `render_scenes`)
- Protocol: `fframes-studio-protocol/src/{lib,preview}.rs` (versions, Hello capabilities, `PreviewRequest`, `MAX_*` bounds); consumers of `PreviewRequest`: `desktop` preview worker client, `fframes-studio-runtime/src/preview_worker.rs`, `fframes-studio-runtime/tests/{preview_worker,editor_metadata}.rs`
- Caches: `ThumbnailKey` (thumbnail cache), prepared audio in the preview coordinator, `PrepareAudioRequest`
- Export: `ExportRequest` in the export service; template worker `run_export` (`--export-v1 --output`, MPEG-4 hardcoded)

## Tasks

1. `fframes::timing`: `TimingDoc`, `TimingClip`, strict serde (`deny_unknown_fields`, known versions), bounds (clip count, total frames, trims below scene length), checked arithmetic, whole-document rejection with a reason. Key rule and derived editor keys `key#clip-index` from the spike. Wrapper scene over `Frame::clone_with_scene_offset`. `get_scene_info` answers for wrapped scenes per the spike rule. Unit tests for each rule and for rejection.
2. `Previewer` and the render path take `Option<TimingDoc>`; `None` is byte-identical to today (snapshot test on `examples/hello-world`).
3. `fframes::cli`: opt-in only, through a builder call `cli::new(..).timing_file(path)` that the studio template uses. No automatic pickup and no new global flag for other users.
4. Protocol: offer `SetTiming` as a Hello capability; keep the contract version. `SetTiming` carries the envelope (identity validated like other requests) and a `timing_generation`. The worker's startup config may carry an initial doc, and a worker that offers the capability renders untimed only until the first `SetTiming`. Update all four `PreviewRequest` consumers.
5. Caches: add `timing_generation` to `ThumbnailKey` and to prepared audio; a generation change re-prepares audio and misses thumbnails.
6. Export options: extend the worker export arguments with container, codec, scale and fps (H.264, HEVC, VP9/WebM through the encoders `fframes` already supports), keeping `--export-v1` defaults; `ExportRequest` gains the same fields and the timing doc. Add a parity test: with the same doc, the render path's frames before encoding are pixel-identical to preview frames at three times, and the decoded export is within the e2e `odiff` threshold (encoders are lossy, so the encoded file is never compared exactly).
7. Document `timeline.json` in `AGENTS.md` (under Scenes) and in the template's agent guide, so agents edit timing there and keep `editor_instance_key` on editable scenes.
8. `just check-examples` and `just check-wasm hello-world` pass.

## Todo

- [x] `fframes::timing` with bounds and tests
- [x] Previewer and render take the doc
- [x] CLI builder opt-in
- [x] `SetTiming` capability, envelope, startup doc
- [x] Thumbnail cache key carries the timing generation (the prepared-audio key moved to phase 3)
- [x] Export options in worker and request
- [x] Agent docs
- [x] Examples and WASM unaffected

## Delivery notes

- Timing is applied to timeline entries (`ResolvedSceneClip`, `Frame::clone_with_clip_offset`)
  as the accepted spike report decided, not through a wrapper scene as task 1 first said.
- Moved to phase 3: the coordinator retime lane (send `SetTiming` to the displayed
  worker, re-prepare audio under the new generation, release the old prepared
  artifact, publish the change). Its end-to-end test needs a materialized build, and
  phase 3 is where edits first produce generations. Thumbnails already miss on a
  generation change, and the pump renders only for the displayed generation.
- Export options live in `fframes_studio_runtime::run_export` and the protocol's
  `export` module. The template worker keeps calling its own MPEG-4 `run_export`
  until phase 2 switches it to the runtime; the engine asks the worker for
  `--export-capabilities-v1` before passing any non-default setting or timing doc and
  refuses with "migrate the project" when the worker cannot answer.
- `RenderOptions::override_fps` is now a frame stride (encode every n-th frame) on
  backends that report `supports_frame_stride` (the CPU backend).
- Not run here: the SDK-backed native export test (`native-export.rs`, ignored, needs
  `SDK_BUNDLE`). It now also asserts the legacy refusal; run it with the phase 2 SDK.

## Success criteria

`cargo test -p fframes timing` and `cargo test -p fframes-studio-runtime` pass; a malformed or oversized doc is rejected with a reason and never panics (fuzz-style test with random docs); reorder then same-index thumbnail request misses the cache.
