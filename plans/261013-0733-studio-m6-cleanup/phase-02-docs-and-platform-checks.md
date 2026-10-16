---
phase: 2
title: "Docs and platform checks"
status: pending
priority: P2
effort: "2d"
dependencies: [1]
---

# Phase 2: Docs and platform checks

## Goal

Docs describe the real system, and the roadmap's Done-means list is checked by hand on Windows. macOS is not fully tested: it is covered by CI only, because no Mac is available ([roadmap, Platforms](../261013-0733-studio-rewrite-roadmap/plan.md#confirmed-contract), decided 2026-10-16).

## Tasks

1. Rewrite `docs/desktop/architecture.md`: real crate layout, engine command and event flow with session lifetimes, timing doc and timing revision, playback pipeline, containment, tool broker transports, test strategy. Remove the "proposed" wording and the non-existent `studio-agent/` and `studio-ui/` entries.
2. Update `docs/desktop/README.md` (setup, project migration, editing, agent panel, the one keymap) and the `AGENTS.md` sections touched in M4 phase 1.
3. Mark the "facts only" licensing line in `plans/261011-0033-cap-style-studio-ui/plan.md` as superseded by the roadmap's porting rules.
4. Manual checks recorded in `reports/platform-checks.md`: every roadmap Done-means item on Windows. For macOS, the report lists the latest green macOS CI run and marks each hand check as not run.

## Todo

- [ ] Architecture doc
- [ ] README and AGENTS.md
- [ ] Old licensing line superseded
- [ ] Windows checks
- [ ] macOS: record the latest green macOS CI run and mark the hand checks as not run (macOS not fully tested). If a Mac becomes available, the native-wire real-CLI checks are `cargo test -p studio-harness --test claude_real_cli --test codex_real_cli -- --ignored` ([checks report](../../desktop/plans/reports/native-wires-checks.md)).
- [ ] Report written

## Success criteria

Every link and file path in the updated docs resolves; `reports/platform-checks.md` shows each Done-means item passing by hand on Windows, and on macOS shows green CI and marks the platform not fully tested.
