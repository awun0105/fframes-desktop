---
phase: 1
title: "Phase 1: Runtime renderer selection"
status: completed
priority: P1
effort: 2d
dependencies: []
---

# Phase 1: Runtime renderer selection

## Goal

`fframes-studio-runtime` renders preview frames and exports with the Skia GPU backend whenever a
GPU works. Otherwise it uses the built-in tiny-skia CPU renderer, which is the default without a
GPU (validation session 1). Skia on the CPU runs only when it is chosen explicitly. The public
entry points keep their signatures, so project worker code does not change.
<!-- Updated: Validation Session 1 - no-GPU default is tiny-skia; Skia CPU only on explicit choice -->


## Context

- Preview hard-codes `CpuFrameRenderer::default()` at `fframes-studio-runtime/src/preview_worker.rs:344`
  and reports `backend: "cpu"` and `capability_gaps: ["shader_preview"]` at `:433` and `:439`.
- Export hard-codes `fframes::cpu::CpuRenderingBackend::default()` at `src/export.rs:125-130`.
- Real Studio projects call `run_export` and `serve_preview_worker` from
  `desktop/crates/studio-project/templates/src/bin/studio_worker.rs:32` and `:52`.
  (`desktop/fixtures/annotated-video-overlay/src/main.rs:194` is the SDK qualification fixture.)
- `Previewer::render*` takes `&mut dyn FrameRenderer` (`fframes/src/renderer/preview.rs:551`).
- Skia constructors:
  - `SkiaVulkanCtx::new(w, h)` (`fframes-skia-renderer/src/backends/vulkan.rs:58`). It loads Vulkan
    through `Entry::load`, so a missing driver is an error, not a link failure. It takes the first
    enumerated physical device (`vulkan.rs:116-124`).
  - `SkiaMetalCtx::new(w, h)` (`backends/metal.rs:29`).
  - The infallible `SkiaCpuCtx::new(w, h)` (`backends/mod.rs:108`).
- `SkiaFrameRenderer::new(&ctx)` (`frame_renderer.rs:105`) borrows the context and is probably
  `!Send`, so it is built on the worker loop thread.
- **Size changes are expensive.** `SkiaFrameRenderer::render_tree` calls `release_surface()` on any
  size change (`frame_renderer.rs:139-141`). That drops the `DirectContext` and the whole
  `RenderCache`, including the compiled shaders (`:79-95`, `render/mod.rs:153-154`). The
  coordinator renders main frames and timeline thumbnails through the same worker at different
  scales (`desktop/crates/studio-engine/src/services/preview_coordinator.rs:896` vs `:945`), so the
  sizes alternate during normal use.
- Shaders need `fframes/compile-time-svgtree` (`fframes/src/lib.rs:84-85`). `fframes_skia_renderer`
  does not enable it for its normal dependency.
- `SkiaCpuCtx` is documented as slower than the built-in CPU backend (`backends/mod.rs:101-104`).
  That is why the user chose tiny-skia as the no-GPU default. Skia CPU stays available as an
  explicit choice: it is the only GPU-less path that draws shaders, and on this VM it is the only way
  to exercise the Skia code path.
- The GPU setup to follow is `examples/fframes-studio-intro/src/render_cli.rs:90-126`. It does not
  probe, which this phase adds. Here the fallback goes to tiny-skia, not to Skia CPU.

## Files to create / modify

- Modify: `fframes-studio-runtime/Cargo.toml`.
  - Add `fframes_skia_renderer = { workspace = true, optional = true }`, with the `vulkan` feature on
    non-macOS targets and `metal` on macOS.
  - `default = ["skia"]`, `skia = ["dep:fframes_skia_renderer", "fframes/compile-time-svgtree"]`.
  - Do not add `skia-safe/no-compile` here. Features unify across the 40-member core workspace,
    whose Skia key differs from the SDK worker's, so a key miss would break every dev build. Phase 2
    adds it only to the SDK's copy of the manifest.
  - Dev-dependency: `fframes` with `compile-time-svgtree`.
- Create: `fframes-studio-runtime/src/renderer.rs`: `RendererPreference`, the selection, the probe,
  the per-size renderer set, and `BackendChoice { name, detail }`.
- Modify: `fframes-studio-runtime/src/preview_worker.rs`, `src/export.rs`, `src/lib.rs`.
- Modify: `fframes-studio-protocol/src/preview.rs`. Add
  `#[serde(default, skip_serializing_if = "Option::is_none")] pub backend_detail: Option<String>` to
  `PreviewHelloResponse`, and the same optional field to `ScaledFrameResponse` so a mid-session
  fallback is visible. The protocol has no `deny_unknown_fields`, so older readers ignore the field
  and the contract version stays 1.
- Modify: `fframes-studio-runtime/tests/preview_worker.rs` and `tests/export.rs`.

## Tasks & steps

- [x] Add the Cargo features. Both `cargo build -p fframes-studio-runtime` and
      `cargo build -p fframes-studio-runtime --no-default-features` must compile.
- [x] `RendererPreference` (`Auto | Gpu | SkiaCpu | Cpu`) comes from a worker argument,
      `--renderer <auto|gpu|skia-cpu|cpu>`, not from an environment variable. Studio's child
      environment is cleared, allowlisted and hashed into the build key
      (`desktop/crates/studio-bootstrap/src/process/mod.rs:204`,
      `desktop/crates/studio-engine/src/services/build_service.rs:209-221`). A variable would
      either never reach the worker or force a rebuild whenever the preference changes.
  - `PreviewWorkerConfig::new` parses `--renderer` from `std::env::args()`, so the template
    (`studio_worker.rs`) and projects already generated from it need no change. `run_export`
    parses it from the `args` it already receives. With no argument the preference is `Auto`.
    An unknown value also gives `Auto`, plus a stderr line.
  - Tests set `PreviewWorkerConfig.renderer` directly.
- [x] Selection with an injectable GPU constructor, so tests do not depend on the host's drivers:
  - `Auto`: GPU (`SkiaMetalCtx` on macOS, `SkiaVulkanCtx` elsewhere), then tiny-skia.
  - `Gpu`: GPU, or a hard error that names the reason.
  - `SkiaCpu`: Skia on the CPU (`SkiaCpuCtx`).
  - `Cpu`, or the feature off: tiny-skia.
  - Names: `skia-gpu` (detail: `vulkan <device name> <device type>` or `metal`), `skia-cpu`, and `cpu`
    (detail: the GPU fallback reason, if any).
  - Vulkan device order: prefer a discrete or integrated GPU over a `CPU`-type device (lavapipe,
    SwiftShader). If only a CPU-type device exists, treat it as no GPU and fall back to tiny-skia. This needs
    a device-preference parameter on `SkiaVulkanCtx`, or a small `new_with_device_filter` constructor,
    in `fframes-skia-renderer/src/backends/vulkan.rs`.
- [x] **Probe with a synthetic frame, not the user's frame 0.**
  - Draw a fixed 64×64 internal tree: a solid rectangle plus a one-line SkSL `Shader` with a known
    color. Read it back and compare two pixels. Only the Skia backends are probed; tiny-skia needs no probe.
  - Run the probe eagerly, before Hello, at 64×64, so Hello always names the backend that actually
    runs. Everything downstream trusts Hello: the client gate, the shader warning and the thumbnail
    cache key. A 64×64 synthetic frame costs milliseconds once the device exists, and Vulkan instance
    creation fits easily inside the client's 30 s deadline
    (`desktop/crates/studio-engine/src/services/worker_client.rs:112`). The deadline concern applies
    only to rendering user content at full size, which the probe never does.
  - After Hello, the backend changes only through the mid-session fallback below.
  - Probe failure on the GPU under `Auto`: use tiny-skia, report `cpu` with the gap, and put the reason
    in `detail`. Probe failure under `SkiaCpu`: the same, with that reason.
  - An error from the user's own `render_frame` is never treated as a backend failure. The probe never
    renders user content.
- [x] **One renderer per size class.** Keep two `SkiaFrameRenderer`s on the same context, one for the main
      size and one for the thumbnail size (the last two sizes requested, least recently used evicted).
      Then interleaved thumbnails do not drop the main renderer's cache. If this does not fit
      `SkiaFrameRenderer`'s borrowing, change `release_surface` in
      `fframes-skia-renderer/src/frame_renderer.rs` to replace only the surface and keep the
      `DirectContext` and `RenderCache` instead.
- [x] **Mid-session GPU failure.** When a GPU frame fails with a renderer error (not a user render error),
      switch once to tiny-skia, render the frame again, and set `backend` (now `cpu`, with the
      shader gap) and `backend_detail` on that frame
      response. The worker does not retry the GPU afterwards. A GPU that hangs instead of failing
      (for example an expensive shader, then a Windows driver reset) cannot be stopped from inside
      the worker. The engine's request deadline handles that case (Phase 3).
- [x] Cap `backend_detail` at one line and 256 bytes, as `validation.rs:347` does for gap strings.
- [x] `serve_preview_worker`: Hello sends `backend`, `backend_detail`, and `capability_gaps` =
      `["shader_preview"]` only when the backend is `cpu`.
- [x] `run_export`:
  - Select at the export size (`VideoSize::new_scaled`).
  - For Skia, build `SkiaFFramesRenderer::new_vulkan/new_metal/new_cpu` with
    `SkiaPipelineConcurrencyPolicy::MaxPerformance`, as `render_cli.rs:85-88` does.
  - Probe through the encode path, `negotiate_encoder_input` plus one `encoder_frame_renderer` frame
    of the synthetic tree, so the probe covers GPU YUV conversion and does not need a second `Previewer`.
  - Log the chosen backend at info level on the JSON logger and any fallback at warn level.
- [x] Tests:
  - `skia_cpu_preview_draws_shader_layer` (`#[cfg(feature = "skia")]`): a test `Video` with an SkSL
    `Shader` filling a known rectangle with one color. Under `SkiaCpu` the rectangle has that color
    within tolerance. Under `Cpu` it equals the background.
  - `gpu_failure_falls_back_to_cpu`: under `Auto`, an injected GPU constructor that fails gives
    `backend == "cpu"`, the `shader_preview` gap, and the reason in `detail`.
  - `gpu_success_is_used`: under `Auto`, an injected GPU constructor that succeeds gives `skia-gpu` and
    no gap. This is the case the user cares about, and it cannot be checked on this VM without injection.
  - A second injected backend that fails after its first frame switches to tiny-skia mid-session.
  - `forced_gpu_failure_is_an_error`: the `Gpu` preference with a failing constructor returns a clear
    error.
  - `hello_reports_backend_and_gaps`: `SkiaCpu` reports no `shader_preview` gap; `Cpu` reports the gap.
  - `user_frame_error_is_not_a_backend_failure`: a video whose frame 0 returns an error keeps its Skia
    backend, and the request reports `RENDER_ERROR`.
  - `alternating_sizes_keep_both_renderers`: render main, thumbnail, main. The second main render does
    not rebuild the context. Assert it through a context-creation counter on the injected backend.
  - Export: extend `tests/export.rs` to run under `SkiaCpu` and check the decoded first frame has the
    shader color.

## Verification

- `cargo test -p fframes-studio-runtime`
- `cargo test -p fframes-studio-runtime --no-default-features` (tiny-skia only; the Skia tests are cfg-gated)
- `cargo test -p fframes-studio-protocol -p fframes_skia_renderer`
- `cargo build --workspace` at the repository root, to show dev builds of the other members are unaffected.
- `just clippy`

## Risks

- On GPU-less machines the default preview still skips shaders, by the user's choice. Studio reports
  the gap as a warning, and Skia CPU can be chosen in Settings (Phase 4) to check shader output.
- `SkiaFrameRenderer` lifetime and `!Send`. The renderer set lives on the loop thread and owns nothing
  across threads.

## Implementation notes

- Export runs `renderer::probe` (the shader frame) at selection, then an encode-path probe:
  `VideoEncoderInfo::for_output`, `negotiate_encoder_input`, and one `encoder_frame_renderer` frame of
  a static rounded rect and circle. A probe failure exports with tiny-skia and emits a `warning` event.
  A missing encoder is not a probe failure and fails the export.
- The Skia pipeline cannot encode at another frame rate (`supports_frame_stride` is false). An export
  with `--fps` below the video's therefore uses tiny-skia up front, with a `warning`, rather than
  failing over after an error.
- A Skia export that fails with `Skia`, `Internal` or `ConcurrencyError` is rendered again once
  with tiny-skia, with a `warning`. Encoder errors, panics, missing media and cancellation are not
  retried. The chosen backend is logged as one `{"event":"backend"}` stderr line, which the export
  service keeps with the diagnostics.
- tiny-skia draws `FastShape` nodes, building their outline (`svgr/src/render.rs:119`). So
  rendering the frame again after a GPU failure is needed only because that frame was never drawn.
- A preview or export asking for `gpu` or `skia-cpu` from a worker built without `skia` gets
  `RENDERER_UNAVAILABLE` at Hello, or an export error.
- Windows builds need FFmpeg's `bin` directory on `PATH`: the `fframes-media-dir-macro` proc macro links
  FFmpeg, and rustc reports `E0463 can't find crate` when its DLLs do not load.
- `cargo build --workspace` cannot run on this VM, and could not before this change. Most examples
  enable `h264`/`libav-agree-gpl`, which need an FFmpeg built from source; there is no prebuilt
  binary for them (404), and the source build fails here. Instead, every non-example member was built:
  protocol, runtime, svgr-macro, webvtt-parser, fframes, test utils, media, Skia renderer, native
  player, media-dir-macro, cargo-fframes, editor controller. `cargo check --workspace --tests` in `desktop/` passes.
- `just clippy` (all targets) also fails on lints this phase did not touch, in Skia renderer test
  files (`chunks_exact` in `tests/fast_shapes.rs` and `vulkan_lifetime_tests.rs`). These pass:
  - `cargo clippy -p fframes-studio-runtime -p fframes-studio-protocol --all-targets -- -D warnings`, with and without default features
  - `cargo clippy -p fframes_skia_renderer --features vulkan --lib -- -D warnings`
- Tests:
  - `renderer.rs` unit tests: selection, size classes, mid-session fallback, the probe, and a rebuilt frame equal to a CPU render.
  - `tests/preview_worker.rs`:
    - Skia CPU draws the shader and reports no gap.
    - `cpu` skips the shader and reports the gap.
    - A user panic keeps Skia.
    - A no-skia build refuses forced Skia.
  - `tests/export.rs`:
    - A `skia-cpu` export draws the shader.
    - A `cpu` export does not.
    - A `skia-cpu` export at a lower frame rate uses the CPU renderer.
