---
title: "M6 Cleanup: spike names, docs, flaky tests, platform checks"
description: "Remove spike leftovers and dead modules, fix the intermittent tests and handed-over limitations, update the docs to the real system, and hand-check Windows; macOS is covered by CI only (not fully tested)."
status: pending
priority: P2
effort: "4-5d"
branch: feat/studio-m6
tags: [desktop, refactor, docs, tech-debt, tests]
blockedBy: []
blocks: []
created: 2026-10-13
---

# M6 Cleanup

## Overview

Part of [the studio rewrite roadmap](../261013-0733-studio-rewrite-roadmap/plan.md). M4 and M5 are merged on this branch. The bounds-probe harness is already gone (M2 phase 7); this milestone removes the spike leftovers, fixes what M5 and M5a handed over, updates the docs and runs the final Windows checks. When it merges, the roadmap's Done-means list is met. Scouting counts and decisions for the plan owner: [reports/m6-readiness.md](./reports/m6-readiness.md).

## Phases

| # | Phase | Effort | Depends on | Status |
|---|-------|--------|-----------|--------|
| 1 | [Remove spike names and old modules](./phase-01-remove-spikes-and-old-modules.md) | 1d | none | Pending |
| 2 | [Docs and platform checks](./phase-02-docs-and-platform-checks.md) | 1.5d | 1 | Pending |
| 3 | [Flaky tests and leftover limitations](./phase-03-flaky-tests-and-leftover-limitations.md) | 2d | none | Pending |

Phases 1 and 3 touch disjoint files and can run in parallel; phase 2 goes last so the docs describe the final tree. The export-cancel hang in phase 3 is the one high-risk item: its cause is unproven and may need CI rounds.

## Success criteria

- [ ] No `spike` text remains under `desktop/` or `.github` (plans excluded); the app crate holds only `main.rs`, `assets.rs`, an assets-only `lib.rs` and the two bins; the CI step "Real worker frame, anchor, crash and restart" still runs a real test.
- [ ] The harness identifiers `qualify_`, `presentation_stress`, `stress_metrics` and `qualification::` are absent; `start_agent_telemetry` and `bounds_probe` remain only at the production sites M2 phase 7 kept (listed in phase 1).
- [ ] `services::cancelling_an_export_leaves_no_file` has a proven cause, passes in seconds, and passes 30 consecutive runs on Windows and 3 on self-hosted Linux CI; the other four intermittent tests pass 30 consecutive runs each.
- [ ] Every M5 and M5a handed-over limitation is fixed with a test or recorded as an owner decision.
- [ ] `docs/desktop/architecture.md` and `README.md` describe the real system; `AGENTS.md` points to `docs/desktop`; links resolve.
- [ ] Every roadmap Done-means item is hand-checked on Windows in `reports/platform-checks.md`. macOS is marked not fully tested: green CI only, no hand checks, no Mac available (decided 2026-10-16).
