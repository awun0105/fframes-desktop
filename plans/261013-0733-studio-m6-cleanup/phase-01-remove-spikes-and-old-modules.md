---
phase: 1
title: "Remove spike names and old modules"
status: pending
priority: P2
effort: "1-2d"
dependencies: []
---

# Phase 1: Remove spike names and old modules

## Goal

No spike leftovers or dead modules remain, and the app crate is a thin binary.

## Tasks

1. Scout: `agent_spike.rs`, `selection_spike.rs`, `app.rs` (`StudioSpikeApp`), the `spike-ui` route in `main.rs`, and any `spike` identifiers in crates and tests.
2. Delete what has no caller; rename what is still used (for example selection logic into `studio-ui/src/player/selection.rs`).
3. Check that `desktop/app/src` holds only runtime setup, window creation and the two bins; move anything else to its crate.
4. Run the M6 harness-identifier grep and fix any leftover.

## Todo

- [ ] Scout spike leftovers
- [ ] Delete or rename
- [ ] App crate thin
- [ ] Harness grep clean

## Success criteria

`grep -rni "spike" desktop --include=*.rs --include=Cargo.toml` and `grep -rni "spike" .github/workflows` return nothing (no `-w`: it treats `_` as part of a word and misses `agent_spike`); the M6 plan's grep criterion holds; all tests pass.
