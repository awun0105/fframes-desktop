# Spike S2: timing hook

Date: 2026-10-14. Host: this Windows Server 2022 VM, no GPU, Rust 1.98.1 (MSVC), CPU backend.
Code: throwaway branch `spike/m1-timing` (head `6c7864c`, pushed). M4 phases 1 and 2 rewrite it as
production code.

## Result

**Pass.** On a project created from the studio template and given three scenes with
`Duration::Auto`, reorder, trim and split change preview frames through `SetTiming`. The same
`studio_worker` process serves every request, with no rebuild or restart. The render path given the
same document draws frames byte-identical to the preview frames at four times, and the encoded MP4
matches them too. On `examples/fframes-studio-intro` (ten scenes) a narrower in-process check also
passes: `Previewer::set_timing` and `render_frame`, without the worker, `SetTiming` or an encoded file.
The template run is the one that meets the exit criterion.

| Exit criterion | Status |
|---|---|
| Strict, bounded data shape and key rule | Done: `fframes/src/timing.rs`, 4 unit tests |
| Timing applied by the previewer and the render path, opt-in | Done: `RenderOptions::timing` (default `None`), `Previewer::set_timing` |
| Existing examples and the WASM bridge compile unchanged | Done: 9 editor bridges (`just check-examples` set) for `wasm32`; `fframes-editor-controller`, `cargo-fframes`, `fframes_test_utils`, `e2e-tests`, `fframes-studio-intro` and the whole `desktop/` workspace compile with no source changes, apart from the four test literals and one client `match` arm listed under "Compatibility" below |
| `SetTiming` as a Hello capability, no contract version bump | Done: `set_timing_v1` in `OPTIONAL_PREVIEW_CAPABILITIES`; 2 worker tests |
| Overlap and audio rule written and tested | Done, see "Rules" |
| Template-based project: preview through `SetTiming`, render parity at three times | Done: 4 times, see "Template project run" |

## Data shape

```rust
#[serde(deny_unknown_fields)]
pub struct TimingDoc { pub version: u32, pub clips: Vec<TimingClip> }
#[serde(deny_unknown_fields)]
pub struct TimingClip { pub key: String, pub trim_start_frames: usize, pub trim_end_frames: usize }
```

- Clips play in `clips` order. A split is two clips with the same `key`.
- **Bounds:**
  - `version` must equal `TIMING_DOC_VERSION` (1).
  - A document holds at most 4096 clips.
  - A key is 1 to 512 bytes, with no control characters.
  - Each clip needs `trim_start + trim_end < scene length`, computed with `checked_add`.
  - The laid-out timeline is at most 24 hours at the video's fps.
- Any error rejects the whole document. `Previewer::set_timing` then keeps the previous layout, and the worker answers `INVALID_TIMING`.
- Negative numbers and unknown fields fail at deserialisation.

## Key rule

- **Timing key:** `editor_instance_key()` when present, else `Scene::name()`.
  - The second and later scenes with the same key get `#2`, `#3`, … in code order.
  - The first instance keeps the bare key, so adding an instance never renames existing ones. The plan's "`#n` for repeated keys" did not say whether the first instance is numbered; this choice is the stable one.
- **Orphan clips** (keys matching no code scene) are skipped and reported in `TimingReport::orphan_clips`.
- **Code scenes missing from the document** are appended untrimmed and reported in `appended_scenes`.
- **Editor object key of a clip:** the scene's own key for its first clip, `key#n` for the n-th later clip of a split scene.
  - This deviates from the plan's `key#clip-index` for every clip.
  - Reason: an unsplit scene keeps the key its source anchors and canvas selection already use, so a document that only reorders or trims changes no editor identity. Keys stay unique after a split.
  - Scenes without an `editor_instance_key` still emit no editor object, as today.

## Design: timeline entries instead of wrapper scenes

The plan proposed a wrapper `Scene` delegating to a borrowed `&dyn Scene`. Two things rule that out:

1. **Pointer identity.** `get_scene_info` finds a scene by comparing pointers with the timeline entry. A wrapper would make `ctx.get_scene_info(self)` return `None` in every existing scene.
2. **Lifetimes.** Wrappers built from a runtime document would have to be owned by the `Previewer` while its timeline borrows them as `&'a`, which makes the struct self-referential.

Instead, the resolved timeline entry became a struct, `ResolvedSceneClip { range, info, scene, trim_start, trim_end, timing_key, clip_index }`. It keeps the original `&dyn Scene`, and the trims live on the entry.

- `render_scenes` builds the scene frame with `Frame::clone_with_clip_offset`: `global - range.start + trim_start`.
- `resolve_timeline` keeps its signature and calls `resolve_timeline_with_timing(.., None)`. `FFramesRendererRuntime::new` likewise calls `new_with_timing(.., None)`.
- `FFramesRendererRuntime::resolve_timeline` re-resolves without reloading fonts. `Previewer::set_timing` uses it to swap the timeline and `TimelineIndex` in place.
- The timeline field was already `pub(crate)`, so no public API changed shape. Additions:
  - `ResolvedScenesTimeline::iter` and `timing_report`.
  - `TimelineScene` and `SceneReport` gain `timing_key`, `clip_index` and the two trims.

## Rules

- **Scene frames.** A trimmed scene renders exactly the frames it would render at those positions without the trim. Frame `trim_start` is drawn at the clip's first frame.
- **`get_scene_info`.** For a wrapped, trimmed or split scene it answers with the first clip of that scene: its `start_frame` and `end_frame`, and its position as `index`. `duration_in_frames` stays the scene's own (untrimmed) length, because trim cuts the window, not the scene, and scene code that times an exit against its own length keeps working. Tested.
- **Overlap.**
  - An overlap belongs to the scene's natural edge. A trimmed edge is a hard cut, so `Overlap::Previous` applies only when `trim_start == 0` and `Overlap::Next` only when `trim_end == 0`.
  - An overlap reaching before frame 0 is clamped. This happens when a reorder moves an overlapping scene first.
  - The old code computed `final_duration - overlap_prev` unchecked. That underflow was already possible for a first scene with `Overlap::Previous` and became reachable through reorder.
  - Without a document the layout is unchanged (tested against today's numbers).
- **Audio.**
  - Scene audio follows its clip and is cut with it.
  - Tracks are resolved in scene time and shifted by `clip.start - trim_start`. They are cut at an edge only where that edge is trimmed, and the cut start is added to the track's file offset (`TrackMix::offset`).
  - An untrimmed clip keeps today's behaviour exactly; a track may still run past the scene end.
  - A split scene plays each part of its audio under its own clip (tested: `0.0..0.4` plays file `0.0..0.4`, `1.4..2.0` plays file `0.4..1.0`).
  - Top-level `Video::audio()` is not moved by timing. In `fframes-studio-intro` the music stays where it was when scenes are reordered; M4 has to decide whether music should be re-cut.
  - Fades are kept unchanged on cut tracks. A trimmed track that started at 0 s now starts mid-file without a fade-in; the existing 5 ms de-click on cuts covers clicks only.
- **Prepared audio.** Audio prepared before `SetTiming` describes the old layout; the client prepares it again. The worker does not invalidate artifacts itself.

## Protocol

- `SET_TIMING_CAPABILITY = "set_timing_v1"`, offered in Hello through `OPTIONAL_PREVIEW_CAPABILITIES`. There is no contract version bump.
- `PreviewRequest::SetTiming(SetTimingRequest { envelope, timing_json: Option<String> })`. `None` restores code order.
- The worker parses with `fframes::TimingDoc`, so strictness lives in one place.
- Replies:
  - Success: `PreviewResponse::TimingApplied { envelope, total_frames, orphan_clips, appended_scenes }`.
  - Failure: `Error { code: "INVALID_TIMING" }`, and the previous timing stays.
- The next `Timeline` reflects the document. `PreviewSceneInfo` gains an optional `clip: Option<PreviewClipTiming>` (serde default, skipped when absent).
- **Old workers.** `PreviewRequest` is internally tagged, so a worker built before this change cannot parse `SetTiming`; its reader would fail and the worker would stop. Clients must send `SetTiming` only when Hello lists `set_timing_v1`. Without it they keep previewing in code order. A test pins that an old request enum rejects the message.
- **Compatibility.** The wire is additive. In source, a new `PreviewResponse` variant breaks exhaustive matches: `desktop/app/src/preview_worker_client.rs` needed one arm. The new `PreviewSceneInfo` field needed `clip: None` in four `studio-engine` test literals.

## Template project run

- **Project:** the five template files plus the DM Sans font, as `studio-project` writes them. Its `[patch.crates-io]` points `fframes`, `fframes-studio-runtime` and `fframes-studio-protocol` at the spike branch. Copy: `desktop/spikes/timing-template-probe/`.
- **Changes to the template:** `Duration::Auto`, and three one-second `Card` scenes keyed `intro`, `body` and `outro`. Each card draws its own color band and a bar 60 px wide per scene frame.
- **Probe:** `src/bin/timing_probe.rs` starts the project's real `studio_worker --preview-worker` process and talks to it over stdio and the frame port.

```
worker pid 15012, set_timing_v1 offered: true
timeline: 90 frames intro@0..30 body@30..60 outro@60..90
SetTiming applied: total_frames: 70, orphan_clips: [], appended_scenes: []
timeline: 70 frames outro@0..30 body@30..40 intro@40..60 body#1@60..70
  timed frame 15: ("outro", 15)   35: ("body", 5)   45: ("intro", 15)   65: ("body", 25)
worker 15012 served every request without a restart or rebuild
  render_frame 15/35/45/65: 0 of 2073600 bytes differ from preview
  encoded frame 15: mean abs diff to preview 1.02 (35: 1.08, 45: 0.89, 65: 1.04), same scene and scene frame
PASS
```

The document was `outro`, `body` trimmed to its first 10 frames, `intro` without its first 10, and `body` again from its frame 20. That is a reorder, two trims and a split.

The render step went through `fframes::render` with the CPU backend and MPEG-4 at 20 Mb/s, and `ffmpeg` decoded the frames.

## fframes-studio-intro run

`examples/fframes-studio-intro/tests/timing_spike.rs` (embedded plus dynamic media, scale 0.25) applies this document:

- `EndScene` first;
- `HookScene` without its first 20 frames;
- `InstallScene` split around a 20-frame gap.

The other seven scenes are appended, which brings the timeline from 6528 to 6488 frames. Frame 5 changes, and `fframes::render_frame` with `RenderOptions::timing` matches the previewer byte for byte at frames 5, 730 and 1057.

## Tests

- `cargo test -p fframes --lib timing`: 10 pass. Six of them are in `renderer/tests/timing_tests.rs`:
  - unchanged layout;
  - reorder, trim and split pixels plus `get_scene_info`;
  - overlap clamp;
  - rejected documents keep the previous timing, plus the orphan and appended report;
  - render path equals preview;
  - audio trim and split.
- `cargo test -p fframes-studio-runtime -p fframes-studio-protocol`: all pass, including `tests/preview_timing.rs` (Hello offers the capability, `SetTiming` changes the next `Timeline`, an invalid document is rejected whole, `None` restores the order) and the old-worker parse test.
- `cargo clippy -p fframes -p fframes-studio-runtime -p fframes-studio-protocol --lib --tests -D warnings`: clean.
  - `--all-targets` fails in `fframes/benches/yuv420_conversion.rs`, which this branch does not touch.
- Not built here: `render-bench`, `fframes_native_player` and `fframes_skia_renderer --all-targets`.
  - They need FFmpeg or Skia built from source, which fails on this VM on any branch.
  - They only call `FFramesRendererRuntime::new` and `resolve_timeline`, whose signatures are unchanged.

## What M4 phases 1 and 2 should carry

- Keep the timeline-entry design rather than wrapper scenes, and keep `get_scene_info` answering with the first clip.
- Decide whether top-level music is re-cut on reorder, and whether trimmed track starts get a short fade.
- Have the client re-prepare audio after `TimingApplied`, and gate `SetTiming` on `set_timing_v1`.
- Add `TimingDoc` to the publish/durable-write path in M3 (strict parse, whole-document rejection).

## Unresolved questions

- None. Decided 2026-10-14: top-level music stays on video time, and the design choices above are accepted.
