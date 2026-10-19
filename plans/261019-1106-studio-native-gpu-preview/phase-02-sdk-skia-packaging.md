---
phase: 2
title: "Phase 2: SDK Skia packaging"
status: in-progress
priority: P1
effort: 1.5d
dependencies: [1]
---

# Phase 2: SDK Skia packaging

## Goal

The managed SDK builds a Skia worker with no network access and no libclang. It does this by
shipping the vendored Skia crates and a hash-pinned prebuilt Skia archive. When the archive is
missing or does not match, the build fails fast with a clear message instead of compiling Skia
or downloading anything.

## Context

- Allowlist: `CRATES` at `desktop/scripts/assemble-phase-zero-sdk.py:19` does not include
  `fframes-skia-renderer`. `write_framework` (`:93-99`) keeps only the workspace path dependencies listed in `CRATES`.
- Studio builds in a private staging tree. `build_materialization.rs:217-229` writes
  `.cargo/config.toml` with the vendored source `studio-vendor`, deletes the copied `Cargo.lock`
  (`:230-233`), and `ensure_lockfile` then runs `cargo generate-lockfile` offline before
  `cargo build --locked --offline` (`desktop/crates/studio-sdk/src/project.rs:88-113`, `:301-309`).
  Cargo locks dependencies for every target, so the vendor directory must hold `skia-safe`,
  `skia-bindings`, `ash`, `metal` and `core-foundation` regardless of the host.
- skia-bindings 0.153.3 build script, in crate mode:
  - It reads `skia-binaries-{key}.tar.gz` from `SKIA_BINARIES_URL`, with `file://` supported
    (`build_support/binary_cache/env.rs:11-25`, `utils.rs:35-36`).
  - Any other URL is fetched with `curl`, which ignores `CARGO_NET_OFFLINE` (`utils.rs:54-76`).
  - When the archive fails, it falls back to a full source build that fetches Skia from
    codeload.github.com (`download.rs:76`, `:124-165`, `build.rs:113-124`).
  - It does not verify the archive it unpacks (`download.rs:212`).
  - With the `no-compile` feature, that fallback panics instead (`build.rs:75`). `skia-safe` forwards
    the feature (`skia-safe-0.153.3/Cargo.toml:109`).
  - The key is the rust-skia repository hash, the target triple, the sorted features and
    `static`/`debug` (`binaries.rs:73-100`). The repository hash comes from `.cargo_vcs_info.json`
    (`build_support/cargo.rs:178-190`). `svg` already implies `textlayout` (skia-bindings `Cargo.toml:90`).
- The installer unpacks every manifest artifact into its `destination_subdir`
  (`desktop/crates/studio-sdk/src/install.rs:513-515`). A Skia artifact of its own would leave no
  `.tar.gz` for `file://` to point at.
- The FFmpeg `sha256` in `phase-zero-sdk.json` is computed from whatever was staged
  (`assemble-phase-zero-sdk.py:37-44`, `:256-257`), so it does not pin anything.
- The assembly qualification builds with the host environment, minus `CARGO_*` and `RUST*`
  (`assemble-phase-zero-sdk.py:239-249`), so host `SKIA_*` variables leak into it.
- Shipped native code is listed in the signed release manifest's `runtime_inventory`, with a
  `license_id` (`desktop/crates/studio-sdk/src/release-manifest.rs:20-27`, `:195-207`). Notices live
  in `desktop/packaging/sdk/notices/`.

## Files to create / modify

- Modify, in the SDK copy only: `assemble-phase-zero-sdk.py` rewrites the copied
  `fframes-skia-renderer/Cargo.toml` to add `no-compile` to its `skia-safe` features, as
  `configure_bundled_ffmpeg` (`:81-89`) rewrites `fframes-media`. A key miss in an SDK build then
  fails at once. The core workspace manifest is not changed, so dev builds can still compile Skia
  from source.
- **Decision: framework crate versions stay `0.2.0` in SDK v5.** `bind` requires each project's
  framework dependency to name the SDK's exact version
  (`desktop/crates/studio-engine/src/build_materialization.rs:323-346`), and the template pins
  `=0.2.0`. The runtime and protocol changes are additive (a new default feature and optional
  serde fields), and the SDK pin already tells the two SDKs apart. Keeping the version means the
  repin only rewrites `studio.json`, not the user's `Cargo.toml`. If a release bump is required
  anyway, the repin must also rewrite those version requirements; raise that with the user first.
- Create: `desktop/packaging/sdk/skia-binaries.json`. Records the reviewed sha256, key and size of the
  archive for each target. It is checked in, and assembly compares against it without writing it.
- Modify: `desktop/scripts/assemble-phase-zero-sdk.py`:
  - add `fframes-skia-renderer` to `CRATES` and strip its `[dev-dependencies]`;
  - fetch and verify the archive against `skia-binaries.json`, then nest it inside the framework
    artifact at `framework/skia/` so it is installed as a file;
  - strip `SKIA_*` and `FORCE_SKIA_*` from the qualification environment and set the SDK's own
    `SKIA_BINARIES_URL` there;
  - fail unless the qualification build log shows the `file://` archive installed;
  - update `worker_manifest` and the Hello probe;
  - set `SDK_RELEASE = "v5"`.
- Modify: `desktop/crates/studio-sdk/src/environment.rs`. `resolve_child_environment` sets
  `SKIA_BINARIES_URL=file://<sdk>/framework/skia/skia-binaries-{key}.tar.gz` (`{key}` stays literal)
  and strips host `SKIA_*` and `FORCE_SKIA_*`, next to the existing RUSTFLAGS stripping. Both are
  stable values, so the build-key digest changes only with the SDK.
- Modify: the release-manifest assembly. Add the Skia archive to `runtime_inventory` with
  `license_id = "BSD-3-Clause"`.
- Create: `desktop/packaging/sdk/notices/skia-LICENSE.txt`, plus the licenses of the third-party
  code inside the prebuilt archive (FreeType, HarfBuzz, libpng, libwebp, zlib, expat; take the list
  from the archive).

## Tasks & steps

- [ ] **Key and vendor spike (do first).**
  - Assemble a v5 framework tree locally, then build the qualification worker from it with a fresh
    `CARGO_TARGET_DIR` and `-vv`. The dev workspace merges skia-safe features from 40+ members, so
    its key would be wrong. Record the key from the "TRYING TO DOWNLOAD" line.
  - Check that the vendored `skia-bindings` keeps `.cargo_vcs_info.json`. Without it there is no
    crate-mode key at all.
  - Look up the key in the rust-skia/skia-binaries `0.153.3` release.
  - Repeat on Linux and macOS CI runners for their keys; they cannot be computed on Windows.
  - Write the keys and full feature lists to the phase notes.
- [x] Download each target's archive once and review it (source, size, contents). Record its sha256 in
      `skia-binaries.json` in a reviewed commit. Assembly only compares.
- [x] Add `fframes-skia-renderer` to `CRATES` and strip its dev-dependencies. Fail the assembly if the
      manifest shape changes, as `configure_bundled_ffmpeg` does.
- [ ] Check that `cargo vendor` brings in every target's Skia dependencies. Then, on each of the
      three targets, run `cargo generate-lockfile --offline` and `cargo build --locked --offline` from
      the vendored tree.
- [x] Nest the archive in the framework artifact. Add an install test: after install,
      `<sdk>/framework/skia/skia-binaries-<key>.tar.gz` exists and matches its pin.
- [x] Set and strip the `SKIA_*` variables in `resolve_child_environment` and in the qualification
      environment. Unit test: the URL points inside the SDK, `{key}` stays literal, and a host
      `SKIA_SOURCE_DIR` does not reach the child.
- [x] Update the Hello probe. Run it twice:
  - With `--renderer skia-cpu`, require `backend == "skia-cpu"` and no `shader_preview` gap. This
    proves Skia is linked and draws on any assembly host.
  - With the default, accept `skia-gpu`, or `cpu` with the gap. The no-GPU default is tiny-skia.
  <!-- Updated: Validation Session 1 - no-GPU default is tiny-skia -->

- [ ] Add the inventory entry and the notices. Set `SDK_RELEASE = "v5"`.
- [x] Record the SDK size before and after, compressed and installed.

## Verification

- Assemble with the network actually blocked during the qualification build, not just `--offline`:
  a firewall rule, or `HTTPS_PROXY=http://127.0.0.1:9` and `HTTP_PROXY=http://127.0.0.1:9`.
  The log shows the `file://` archive installed and no Skia compilation.
- Negative check: corrupt the nested archive or change the key. The build fails within seconds
  with the `no-compile` message and makes no network request.
- `verify_preview_worker` passes on this VM: `skia-cpu` with the explicit argument, and `cpu` by default.
- `cargo test -p studio-sdk`

## Risks

- No published archive for a target's key. The spike runs first, before anything depends on the features.
- SDK download growth (expect tens of MB compressed). Recorded as an acceptance number.
- A user project that adds its own `fframes_skia_renderer` features changes the key. With
  `no-compile` this fails fast, naming the key. Supporting extra Skia features is out of scope.

## Implementation notes

**Spike**
- Windows key, from a real build log: `b7f043e0b1e2a850e702-x86_64-pc-windows-msvc-ganesh-jpegd-jpege-pdf-svg-textlayout-vulkan-webpd-webpe`.
  The vendored `skia-bindings` keeps `.cargo_vcs_info.json` (sha `b7f043e0b1e2…`).
- Linux and macOS keys follow from the same features: `vulkan`+`webp` on Linux, `metal`+`gl` on macOS.
- All three archives exist in the `0.153.3` release. They were downloaded, and their sha256 equals GitHub's asset digests (uploader `pragmatrix`).
- The Linux and macOS keys are not yet confirmed by a build on those runners.

**Pins, assembly and environment**
- Pins: `desktop/packaging/sdk/skia-binaries.json`.
- Assembly (`stage_skia`):
  - verifies size and sha256;
  - takes cached archives from `SKIA_ARCHIVE_DIR`, otherwise downloads the pinned URL;
  - nests the archive at `framework/skia/`;
  - strips `SKIA_*` and `FORCE_SKIA_*` from the qualification environment and sets the `file://` URL;
  - fails unless the skia-bindings build output shows the `file://` install (`check_skia_from_archive`).
- `configure_bundled_skia` adds `no-compile` to the copy's `skia-safe` and drops its dev-dependencies, examples and tests. `no-compile` is not part of the key.
- `resolve_child_environment` sets `SKIA_BINARIES_URL=file://<sdk>/framework/skia/skia-binaries-{key}.tar.gz` and removes host `SKIA_*`/`FORCE_SKIA_*` (unit test).

**Assembly run** (Windows VM, 8 min)
- Network blocked: `HTTPS_PROXY`/`HTTP_PROXY`/`ALL_PROXY=http://127.0.0.1:9`, a pre-vendored graph via `--vendor`, and `SKIA_ARCHIVE_DIR`.
- Both offline builds installed the shipped archive.
- Hello probes passed: `--renderer skia-cpu` gave `skia-cpu` with no gap, and the default gave `cpu` with the `shader_preview` gap.
- The embeddable Python 3.12.10 from python.org ran the script; this VM has no Python.

**Install and negative checks** (`desktop/crates/studio-sdk/tests/installed_skia_archive.rs`, ignored, run with `SDK_BUNDLE`)
- The installed archive matches its pin.
- With the archive removed and a dead proxy, `cargo build` fails after 38 s, spent compiling other crates first, with `Refusing to full-build skia with no-compile feature`. Nothing was compiled or downloaded.

**Size** (compressed artifacts, Windows): the framework artifact grows from 44.5 MB (v4) to 76.1 MB (v5); toolchain (284.6 MB) and FFmpeg (84.1 MB) are unchanged. Installed, `framework/` grows from 334 MB (the installed v4) to 393 MB.

**Notices**: `notices/skia-LICENSE.txt` holds Skia's BSD-3-Clause license and the license texts of Expat, FreeType, HarfBuzz, ICU, libjpeg-turbo, libpng, libwebp (with PATENTS), zlib, Wuffs and Brotli, at the revisions of Skia's `chrome/m153` DEPS. `LICENSE.txt` lists it.

**Open**
- `runtime_inventory`: the signed release manifest (`release-manifest.rs`) has a verifier, but no script or workflow in this repository writes it; `package-phase-zero.py` writes only the package file inventory. Where the Skia entry with `license_id = "BSD-3-Clause"` is added needs the release owner.
- The Linux and macOS lockfile and offline builds remain for those CI runners.
