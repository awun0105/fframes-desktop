---
phase: 2
title: "Switch the app to Cap's GPUI fork"
status: pending
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

- [ ] Scout gpui uses
- [ ] Pins on the mirror; `cargo deny` in CI
- [ ] Fix compile errors
- [ ] Tests, clippy, qualification green
- [ ] Manual parity click-through on Windows
- [ ] macOS CI builds and tests `desktop/`
- [ ] Spike directory removed

## Success criteria

`cargo test` and `just clippy` pass in `desktop/` on Windows, Linux and macOS CI; qualification jobs pass; the app opens with its look unchanged.

## Risks

- Text shaping or font metrics differ on the fork and move probe bounds: re-baseline qualification expectations rather than change layout.
- Cap's patches change Windows frame handling: keep today's window chrome in this phase; Cap's caption arrives in M2 phase 4.
- Moving the fork later means re-applying Cap's patches: the mirror keeps them as separate commits on a branch, so a rebase shows each conflict.
