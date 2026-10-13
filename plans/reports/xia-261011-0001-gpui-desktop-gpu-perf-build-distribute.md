# Report: GPU preview, performance, build and distribution for the gpui desktop app

Date: 2026-10-11. Mode: compare (no implementation plan, no code changed).

Sources studied, all read-only and untrusted as data:

| Source | Path | Commit | License |
| --- | --- | --- | --- |
| zeron | `~/code/zeron` | `344436efb28298a8b122ad1a970aff6be3880524` | MIT |
| Cap | `~/code/Cap` (gpui app in `apps/desktop-gpui`) | `877d3a3208ebbdcffd9882c7cf4aaf629f7c1494` | `desktop-gpui` is AGPL-3.0 |

Local project: `desktop/` in this repository (`fframes-studio` app plus `studio-*` crates). Cap's code must be treated as a design reference only, because copying AGPL code would pull its license into fframes. zeron is MIT, so its techniques can be reimplemented with attribution.

Nothing below was built or run. Performance figures are quoted from the sources' own documents and were measured on their hardware, not on this machine.

## Outcome in one paragraph

The local desktop app already has gpui, a preview worker, signing scripts and a release workflow, but its preview is CPU-only end to end: the worker accepts only `backend == "cpu"` (`desktop/app/src/worker_project.rs:105`), and every frame goes through a scalar per-pixel RGBA-to-BGRA loop (`desktop/app/src/frame_image.rs:41-83`) before becoming a gpui `RenderImage`. In the main studio preview path that conversion runs on the UI thread, in the same update that installs the image (`desktop/app/src/studio_shell.rs:2709`); the older spike path in `app.rs` converts on a background executor. Stale frames are already rejected before conversion (`accepts_frame` plus a clock-epoch check, and the client validates revision, generation, request id and seek serial on receipt). The biggest wins available, in order, are: (1) make that conversion fast and move it off the UI thread, (2) add a GPU render path behind a capability check with the CPU path kept as the fallback, (3) stop redrawing when nothing changed, and (4) tune build profiles. Distribution is mostly written already; the release policy (`desktop/packaging/release-policy.json`) still lists signing, notarization and an update channel as blockers.

## 1. How to make the desktop app use the GPU

There are two different GPU questions, and the sources answer them differently.

**A. The gpui window itself.** gpui draws through Metal on macOS, through wgpu (Vulkan) on Linux, and through Direct3D on Windows. Cap's README says wgpu is reached only through `gpui_wgpu` from the Linux and web platform crates, and zeron's Windows setup needs `GPUI_FXC_PATH` for the Direct3D shader compiler. This needs no work beyond a working driver for the platform. On this VM, the missing Vulkan driver affects Linux-style wgpu paths and the Skia Vulkan backend; it does not by itself mean the gpui window cannot open on Windows, which I did not test. A future zero-copy path on Windows would be a Direct3D concern, not wgpu. Both zeron and Cap pin a gpui fork rather than upstream because they need renderer fixes (transparent-window alpha blending, bounded blur memory, idle release of GPU intermediates). The local app pins upstream Zed at `1a84d5d9` (`desktop/Cargo.toml:39`), which is the lower-risk choice until a specific bug forces a fork.

**B. The video preview frames.** This is where the local app differs. Options compared:

| Approach | Source | What it costs | What it gains |
| --- | --- | --- | --- |
| CPU render, CPU convert, upload as `RenderImage` | Local today | A full-frame CPU loop per frame plus an upload | Works on any machine, including this CPU-only VM |
| GPU render (wgpu), read back padded RGBA, SIMD convert, upload | Cap, non-macOS path | One GPU-to-CPU readback per frame, still one upload | Large render speedup; row padding must be removed (wgpu aligns rows to 256 bytes, so 1080x702 arrives as 4352 bytes per row, not 4320) |
| GPU render, paint the platform surface directly | Cap, macOS `CVPixelBuffer` with `paint_surface_fitted` | Platform-specific `unsafe` glue | No readback and no copy |
| Render in the gpui renderer itself (custom primitives or shaders) | zeron's gpui fork | Maintaining a gpui fork | Pixels never leave the GPU |

Recommendation: keep the worker process boundary and add a GPU backend to the worker, selected by a capability probe, with CPU as the automatic fallback. fframes already has the pieces: `fframes_skia_renderer` provides Skia on Vulkan or Metal (`gpu`, `metal` features in `fframes-skia-renderer/Cargo.toml`) and a Skia CPU context. The wire protocol already carries `ChannelOrder`, `AlphaMode` and stride in `FrameHeader`, so a GPU backend can return frames in the layout it produces natively and the UI converts only if needed. A zero-copy surface path is a later, macOS-only step and should not block the first GPU milestone.

Constraint that shapes the plan: the project memory records that this VM has no Vulkan, so a GPU backend cannot be exercised here. GPU work needs a machine with a real GPU, and the CPU fallback must stay the default on machines where the probe fails. The existing `qualification` evidence already states "no GPU presentation" for the Linux run, so GPU claims need new evidence, not reuse.

## 2. How to optimize performance

Ordered by expected value for the local app. Items marked "source" have a measurement in the source repositories; the rest are reasoning from the local code.

1. **Replace the scalar conversion loop and take it off the UI thread.** Cap documents that unoptimized frame conversion costs 42.7 ms per frame against 1.0 ms optimized, the difference between 23 fps and 60 fps preview (`apps/desktop-gpui/README.md`). The local loop indexes with bounds checks and does float math per pixel when un-premultiplying. Move conversion into a small crate built at `opt-level = 3` (Cap's `kernels/` pattern), process rows with slice iterators, and use a lookup table or integer math for the alpha divide. Skip un-premultiply entirely when the worker already reports `AlphaMode::Opaque`. Then do the conversion where the frame arrives (the preview pump thread or a background executor) and hand the UI only the finished `RenderImage`, as the spike path in `app.rs` already does. The local conversion time is recorded by `stress_metrics.rs`, so measure first; the 42.7 ms figure is Cap's, not a local number.
2. **Keep the existing frame-dropping and add a bound.** Stale frames are already rejected before conversion locally (`accepts_frame`, the clock-epoch check, and identity validation in `preview_worker_client.rs`). What Cap adds is a small bounded channel that drops on full and counts the drops, so a slow UI cannot build a backlog. Check whether the local pending-frame slot is already single-entry (it looks like one `pending_frame`); if so this item is already satisfied and only the drop counter is missing.
3. **Keep the preview at the negotiated cap.** The protocol caps preview at 1280x720 (`fframes-studio-protocol/src/preview.rs:6-7`) and `preview_scale` never upscales. Keep that, and scale to the physical pane size so a small pane never converts a full 720p frame.
4. **Do not redraw when nothing changed.** zeron's resource report (`docs/performance-resource-usage.md`, Linux software Vulkan, four workers) shows idle CPU falling from 27.4% to 7.5%. That document credits specialized fill shaders (quads were the largest remaining cost), shared 30 Hz and 15 Hz clocks for text dissolves and spinners, a cached sidebar view identity, and cached Markdown trees. A separate macOS document (`docs/performance-macos.md`) describes parking clean-window frame callbacks and reusing the transcript scene on unrelated animation frames; its numbers are not part of that 27.4% to 7.5% figure. The transferable rules are the clocks and view caching: put the timeline playhead and spinners on a shared low-rate clock and cache views that did not change. The fill-shader work is renderer-specific and does not apply to an unforked upstream gpui.
5. **Reuse, do not rebuild.** zeron caches parsed and laid-out content across frames and shares immutable completed blocks; only the changing tail is reparsed. The equivalent locally is the thumbnail and waveform caches (`thumbnail_cache.rs`, `timeline_view.rs`).
6. **Use a different allocator on macOS only if profiling shows retained memory.** zeron uses mimalloc v2 on macOS because libmalloc kept its high-water mark as resident memory, and measured glibc as flat on Linux. This is evidence-driven for them; do not adopt it without a local measurement.
7. **Measure before and after.** zeron ships a resource-profiling driver (`scripts/resource-profile.mjs`) and commits raw result JSON beside its reports. The local `stress_metrics.rs` and `presentation_stress.rs` are the natural place to record frame-conversion time, dropped frames and idle CPU.

Not worth copying now: zeron's custom blur shaders, edge-fade primitives and gpui fork patches. They exist for a frosted-glass UI that the local app does not have.

## 3. How to build

Current local state: `desktop/` is a separate Cargo workspace with its own `Cargo.lock`. Neither `desktop/Cargo.toml` nor the repository root `Cargo.toml` has `[profile.*]` sections, and the root `.cargo/config.toml` only sets macOS flags, so local release builds use cargo defaults. CI is the exception: the release workflow sets `CARGO_INCREMENTAL=0` and `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0` through environment variables. Both sources tune profiles:

| Setting | zeron | Cap | Local today |
| --- | --- | --- | --- |
| Release LTO | `lto = "thin"`, `strip = "symbols"` | Sidecars built without debug info and stripped | None |
| Dev dependencies | `[profile.dev.package."*"] opt-level = 2` | Dependencies at `opt-level = 2`, app crate at 0 | None |
| Hot loops | Not separated | Separate `kernels/` crate at `opt-level = 3` | None |
| Windows stack | Not noted | Links with a 16 MB stack because export constructs large futures on the UI thread | Not set |

Recommendations:

- Add `[profile.dev.package."*"] opt-level = 2` to `desktop/Cargo.toml`. gpui is very slow unoptimized, and both sources do this.
- Add a release profile with thin LTO and symbol stripping, then compare binary size and conversion speed locally before keeping it.
- Add the `kernels/` crate for frame conversion (item 1 above).
- Keep the separate workspace. Cap's reason applies equally here: gpui pulls its own wgpu and font stacks, and a shared lockfile would force unrelated version conflicts.
- Expect long first builds. Cap reports about 11 minutes and 6.4 GB of `target/` for a cold gpui build; incremental builds then take tens of seconds. Cache `target/` and the cargo registry in CI.
- Windows needs the MSVC build tools, Windows SDK and CMake; zeron notes `GPUI_FXC_PATH` must point to the SDK's `fxc.exe` if shader compiler discovery fails.

## 4. How to build and distribute releases

The local repository already has `.github/workflows/desktop-release.yml` (three-OS matrix, tag-triggered, optional signing input), `desktop/scripts/sign-windows-package.ps1`, `sign-macos-package.sh`, `package-linux-deb.py`, a release manifest schema and a release policy. The gap is not tooling but the open blockers in `release-policy.json`: no trusted release keys, `consumer_release_enabled: false`, and per-target blockers (signing, notarization, native package qualification, an FFmpeg GPL source review on Windows, a pinned Linux baseline).

What the sources add:

| Area | zeron | Cap | Local state | Recommendation |
| --- | --- | --- | --- | --- |
| Windows installer | Inno Setup 6, per-user in `%LOCALAPPDATA%`, no elevation, plus a portable zip carrying `zeron-update.json` | NSIS with a 128 MB LZMA window and install hooks that delete old files | Per-user installer listed as the target | Per-user install avoids elevation and simplifies updates; adopt it |
| macOS | dmg, codesign with hardened runtime, notarytool plus staple, all optional behind secrets | `Cap.app.tar.gz` recompressed with zopfli, dmg converted to LZMA | Signing script present, not yet qualified | Keep signing optional in CI, fail loudly when asked to sign without secrets (already the local behavior) |
| Linux | Headless tarball plus `install.sh`; Cap ships deb, rpm, AppImage | deb recompressed with a large xz window, AppImage pruned to loaded libraries | AppImage and deb targeted | Ship AppImage and deb; prune after the first working build |
| Update feed | `manifest.json` with a sha256 per artifact on object storage; `latest.txt` fallback | Signed artifacts under per-platform names; minisign verified while streaming | Release manifest schema exists; trusted keys not configured | Configure the trusted release key before enabling consumer releases |
| Updater | Download as soon as a release is seen, install on restart or quit; install kinds decide the update path | Verifies signature while downloading, never installs during recording or export, relaunches after exit | Not implemented in the app | Port the behavior (verify while streaming, refuse to install during export, relaunch after exit); do not copy Cap's code |
| Crash safety | Not noted | Writes a handoff marker before switching apps and clears the setting if the marker survives | Not applicable | Not needed unless a second app is introduced |

Release order proposed: Linux first because it needs no code signing and is testable in CI, then Windows with the per-user installer and code signing, then macOS with notarization. Linux does not escape the licensing and codec work, though: `release-policy.json` lists blockers for Linux too ("HEVC builds depend on host libx265.so.199 not included in the SDK", "codec and source-distribution review", "pinned system baseline"), and for Windows it lists the GPL FFmpeg license and corresponding-source review. These reviews are legal and packaging prerequisites, not engineering tasks, and should start early because they can block publication on both platforms.

## 5. Decision matrix

| Decision | Source's way | Our way | Recommendation |
| --- | --- | --- | --- |
| gpui dependency | Forked gpui with renderer patches | Upstream Zed at a pinned rev | Stay on upstream; fork only for a proven bug |
| Preview GPU path | Cap: wgpu render, readback or platform surface | CPU only | Add a GPU backend in the worker with CPU fallback; zero-copy later |
| Frame conversion | Cap: separate `opt-level = 3` crate | Scalar loop in the app crate | Adopt the separate-crate pattern |
| Idle cost | zeron: park frames, shared clocks | Not audited | Audit playhead and spinner redraws |
| Workspace layout | Cap: separate workspace | Already separate | Keep |
| Windows installer | zeron: Inno Setup per-user | Per-user target | Adopt Inno Setup per-user |
| Update channel | Both: signed manifest plus streaming verification | Schema only | Implement after trusted keys exist |
| Cap's code | AGPL-3.0 | MIT project | Reimplement from behavior; do not copy |

## Risk score

Medium-high. The conversion and build-profile changes are low risk and local. The GPU backend is high risk because it cannot be tested on the available VM and touches the worker protocol. Distribution risk is mostly external: certificates, notarization accounts, and the FFmpeg license review.

## Unresolved questions

1. Is there a machine with a real GPU (Vulkan on Windows or Linux, or a Mac) available for qualifying a GPU backend? Without one the GPU milestone cannot be verified.
2. Which platform should ship first: Linux, Windows or macOS?
3. Is a code-signing certificate available for Windows, and an Apple developer account for macOS notarization?
4. Where should update artifacts be hosted (GitHub Releases or an object-storage bucket as zeron does)?
5. Have the codec and licensing reviews started (the FFmpeg GPL source review on Windows, and the codec, HEVC and source-distribution review on Linux)? Both are stated blockers in the release policy.
6. Should the app update itself, or is manual download acceptable for the first release?
