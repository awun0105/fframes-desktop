---
phase: 2
title: "Switch the app to Cap's GPUI fork"
status: in-progress
priority: P1
effort: "3-4d"
dependencies: [1]
---

# Phase 2: Switch the app to Cap's GPUI fork

## Goal

The current app, with its current structure and behavior, builds and runs on Cap's GPUI fork (`wingleeio/zed` `5d1f83d` with Cap's three patches, from the `khanhthanhdev/zed-cap` mirror). No refactoring in this phase.

## Context links

- API-change list and patch review: `reports/spike-cap-gpui-api-delta.md` (from phase 1)
- Pins today: `desktop/Cargo.toml:40` (upstream Zed `1a84d5d`)
- Cap's pins: `~/code/Cap/apps/desktop-gpui/Cargo.toml:29-39`; patches `apps/desktop-gpui/patches/zed-{gpui,windows,linux}.patch`; `scripts/prepare-gpui-dependency.sh`
- 41 files in `desktop/app/src` import `gpui`

## Tasks

1. Scout: list every `gpui`/`gpui_platform` use by module and mark those named in the API-change list.
2. Change `desktop/Cargo.toml` workspace deps (`gpui`, `gpui_platform`, `gpui_tokio`) to the mirror at the pinned commit. Align the per-OS `gpui_platform` features in `desktop/app/Cargo.toml:50-56` with Cap's (`font-kit`, `runtime_shaders`; `wayland` and `x11` on Linux). Align the `image` crate with the fork's own pin, as Cap does, so frame types do not split into two versions. Add `desktop/deny.toml` allowing only crates.io and the mirror as sources, and a `cargo deny check sources advisories` step to `.github/workflows/desktop.yml`.
3. Fix compile errors module by module, applying the API-change list. No behavior or layout changes.
4. Run `cargo test` in `desktop/`, `just clippy`, and the existing qualification jobs (self-hosted Linux and Windows) to confirm the bounds probes still report.
5. Open the app on this VM (or the user's machine, see S1) and click through the parity list by hand.
6. Delete `desktop/spikes/cap-gpui-hello/`; keep the macOS CI job on `pull_request`, renamed `desktop-macos` (so no `spike` name survives, see M6) and pointed at `desktop/` (`cargo build` and `cargo test`).

## Todo

- [x] Scout gpui uses
- [x] Pins on the mirror; `cargo deny` in CI
- [x] Fix compile errors
- [ ] Tests, clippy, qualification green
- [x] Manual parity click-through on Windows
- [ ] macOS CI builds and tests `desktop/`
- [x] Spike directory removed

## Progress

- Scout: 41 files in `desktop/app/src` use gpui and no other desktop crate does; only `preview_element.rs` (`paint_image`) and `text_input.rs` (`EntityInputHandler::paste`) touch the API-change list.
- Pins: `gpui` and `gpui_platform` on the mirror at `c32bfd39` with Cap's four features in `desktop/Cargo.toml`; the app takes `gpui_platform` once instead of per OS. No `gpui_tokio` is used. `image` already resolves to one version (0.25.10), which matches the fork's `0.25.1` requirement.
- Compile errors were the two from the API-change list: `preview_element.rs` uses the fork's five-argument `paint_image` (clip and bounds were the same), and `TextInput::paste` moved from `EntityInputHandler` to the inherent impl. The `cfg(macos/linux)` code is covered by CI only.
- `desktop/deny.toml`: crates.io, the mirror, and Zed's `font-kit` and `xim-rs` forks (pinned by the GPUI fork). "Unmaintained" notices count only for direct dependencies; the `ttf-parser` notice (RUSTSEC-2026-0192, a direct dependency of studio-presets before this change) is ignored with a reason. Vulnerabilities fail for every crate. Job `desktop-deny` in `desktop.yml`.
- Windows: `cargo test` and clippy (`-D warnings`) pass for the CI package set; the app opens with its look and native chrome unchanged, and the user clicked through the parity list with no differences ([screenshot](./reports/cap-gpui-switch-studio-window.png)).
- CI: the spike jobs are replaced by `desktop-macos` (test and clippy of `desktop/`, pinned FFmpeg built and cached as on Linux) for every pull request. Linux keeps the self-hosted job and `fork-tests`.

## Success criteria

`cargo test` and `just clippy` pass in `desktop/` on Windows, Linux and macOS CI; qualification jobs pass; the app opens with its look unchanged.

## Risks

- Text shaping or font metrics differ on the fork and move probe bounds: re-baseline qualification expectations rather than change layout.
- Cap's patches change Windows frame handling: keep today's window chrome in this phase; Cap's caption arrives in M2 phase 4.
- Moving the fork later means re-applying Cap's patches: the mirror keeps them as separate commits on a branch, so a rebase shows each conflict.
