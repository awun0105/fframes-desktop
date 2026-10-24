---
phase: 12
title: "Phase 12: Getting started and packaging"
status: completed
priority: P2
effort: 1d
dependencies: [4]
---

# Phase 12: Getting started and packaging

## Goal

Give `cargo fframes new` users a short "add the kit" guide and make the kit packageable,
without changing the upstream scaffolder, the release scripts or fframes Studio.

## Context

- Not edited: `cargo-fframes/` (the scaffolder and its templates), `scripts/publish.sh`,
  `skills/`, and fframes Studio (`desktop/`, including its project template).
- The kit is not published to crates.io in this plan; projects depend on it by git or path.

## Requirements

- `docs/components/getting-started.md`: add `fframes-kit` to a project made by
  `cargo fframes new` (one dependency line, git or path form), the one import, a first
  `Sequence` + `LowerThird` video, and overriding the `Theme`.
- `components/justfile` `package` recipe: `cargo package -p fframes-kit --allow-dirty
  --no-verify` succeeds (the doctest `include_str!` is under `cfg(doctest)`, so packaging does
  not need `docs/`).
- `fframes-kit` stays 0.x until two real videos use it (strategy risk "the API sets in early").

## Files

- Create: `docs/components/getting-started.md`.
- Modify: `components/justfile` (`package`), `docs/components/README.md`.
- Do not modify: `cargo-fframes/`, `scripts/`, `skills/`, `desktop/`.

## Tasks & Steps

1. Write `getting-started.md`; prove it by generating a project with
   `cargo run -p cargo-fframes -- fframes new kit-sample --dir <scratchpad>/kit-sample --yes`
   outside the repository, adding the kit by path, and running `inspect` on it (delete it after).
2. `package` recipe and dry run.
3. `docs/components/README.md` line.

## Verification

- The scratchpad sample builds and `inspect` reports no errors.
- `just -f components/justfile package` succeeds; `just -f components/justfile boundary`
  prints nothing.

## Todo

- [x] `docs/components/getting-started.md`, proven on a scratchpad project
- [x] `package` recipe and dry run
- [x] Docs index line
