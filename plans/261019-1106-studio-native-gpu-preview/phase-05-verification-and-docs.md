---
phase: 5
title: "Phase 5: Verification and docs"
status: pending
priority: P1
effort: 1d
dependencies: [4]
---

# Phase 5: Verification and docs

## Goal

Show with measurements that Studio previews and exports shaders with Skia. Record what each
backend costs, and update the docs that still describe a CPU-only worker that is kept off the GPU.

## Context

<!-- Updated: Validation Session 1 - no-GPU default is tiny-skia; Settings picker; both GPU hosts available -->

- This VM has only the Microsoft Remote Display Adapter and a VirtIO GPU, with no Vulkan driver.
  Here it checks the default (`cpu`) and the explicit `skia-cpu` choice. `skia-gpu` is checked on
  both GPU hosts the user has: a Windows machine with a Vulkan GPU (Vulkan path) and an Apple
  Silicon Mac (Metal path).
- The acceptance project is the Studio-openable copy of `fframes-studio-intro` from the previous
  thread. Today tiny-skia skips its SkSL grain, contour and grid layers
  (`examples/fframes-studio-intro/src/shaders.rs`).
- Parity reference: the original example's CLI (`examples/fframes-studio-intro/src/render_cli.rs`).
  It has no `--renderer` flag. It renders with Vulkan when available and with Skia CPU otherwise
  (`:105-126`), and with Metal on macOS (`:90-100`). On this VM it therefore renders with Skia CPU.
- The preference comes from the Settings choice (Phase 4). Studio's own `FFRAMES_STUDIO_RENDERER`
  overrides it for scripted runs.
- Docs to update:
  - `docs/desktop/architecture.md:71-73`: the worker process "keeps the UI's GPU stack out of every
    project's renderer", which is no longer true.
  - `architecture.md:371`: the preview and export backend, and its selection order (GPU, then CPU).
  - §10: SDK contents.
  - `docs/desktop/README.md`: the renderer setting, and where the docs mention the CPU-only preview
    or SDK setup.
  - The generated AGENTS.md text in `prepare_examples.rs:196` ("independent CPU-rendered copy").

## Tasks & steps

- [ ] **Default on this VM.** Open the acceptance project. The observation JSON reports `cpu`, the
      status bar shows `CPU · no shaders`, the tooltip gives the reason (no Vulkan), and the shader
      warning is shown.
- [ ] **Shader fidelity on this VM.** Capture the preview at a frame with a visible grain or contour
      layer under the default (`cpu`), then after choosing Skia CPU in Settings. Check the backend of
      each capture in the observation JSON. Read both images: the shader layer appears only in the
      second.
- [ ] **Parity.** Render the same time with
      `cargo run --release -p fframes-studio-intro -- frame <t> --scale <s>` (Skia CPU on this VM) and
      compare it with the Studio `skia-cpu` frame at the same scale using `odiff`. They must match
      within the e2e tolerance.
- [ ] **Export.** Export with Skia CPU chosen, decode the frame at the same time, and confirm the
      shader layer is in it. The export log names the same backend as the preview's Hello. With the
      default, the export uses `cpu`, as the preview does.
- [ ] **Speed.** Record in `reports/benchmark.md`:
  - Preview: 10 s of playback at 1280×720 per backend, with the timeline thumbnails visible and
    scrubbed during the run: `worker_render_p50_ms`, `worker_render_p95_ms`, `round_trip_p95_ms`
    and `late_frames`.
  - Export: wall time for the same 10 s clip per backend.
  - Build: cold and warm Studio build times, before and after Skia.
  - Rows: `cpu` and `skia-cpu` on this VM; `skia-gpu` (Vulkan) on the Windows GPU host;
    `skia-gpu` (Metal) on the Mac.
- [ ] **Resize during playback.** Resize the window back and forth while playing. There are no
      errors and the preview keeps updating. Record p95 during the resize.
- [ ] **GPU choice on this VM.** Choose GPU in Settings. The preview shows a clear "GPU unavailable"
      error rather than hanging. Choosing Auto again returns to `cpu`.
- [ ] **GPU host runs, on both the Windows machine and the Mac.** On each:
  - Auto reports `skia-gpu` with the device in `backend_detail`.
  - The shader layers show.
  - The parity frame matches the CLI's GPU render.
  - The speed rows are filled in.
  - VRAM is recorded with the preview, an agent tool and an export running at once.
  - A deliberately expensive shader (a large constant loop) triggers the hang recovery from Phase 3:
    the worker restarts on `cpu`, and Studio's own window survives or the failure is recorded.
- [ ] Update the docs listed in Context. Read each section before editing it and check its links after.
- [ ] Final gates:
  - `just clippy`
  - `cargo fmt --all`
  - `cargo test -p fframes-studio-runtime`
  - from `desktop/`: `cargo test -p studio-engine -p studio-sdk -p studio-project -p studio-ui -p fframes-studio`

## Verification

- `reports/benchmark.md` holds the preview, export and build tables and the SDK size change.
- Inspected screenshots for the shader comparison and the status bar label.
- Command output of the final gates.

## Risks

- GPU-less machines keep the faster tiny-skia preview without shaders by default. The table records
  what Skia CPU costs for users who choose it.
