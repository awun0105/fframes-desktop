---
title: "M6 Cleanup: spike names, docs, platform checks"
description: "Remove spike leftovers and dead modules, update the docs to the real system, and check every roadmap Done-means item by hand on Windows and macOS."
status: pending
priority: P2
effort: "3-4d"
branch: feat/studio-rewrite
tags: [desktop, refactor, docs, tech-debt]
blockedBy: [261013-0733-studio-m4-editing, 261013-0733-studio-m5-agent-ux]
blocks: []
created: 2026-10-13
---

# M6 Cleanup

## Overview

Part of [the studio rewrite roadmap](../261013-0733-studio-rewrite-roadmap/plan.md). The bounds-probe harness was deleted in M2 phase 7 and the size rule is enforced by merge gate 6, so this milestone only removes leftovers, updates docs and runs the final checks. When it merges, the roadmap's Done-means list is met.

## Phases

| # | Phase | Effort | Depends on | Status |
|---|-------|--------|-----------|--------|
| 1 | [Remove spike names and old modules](./phase-01-remove-spikes-and-old-modules.md) | 1-2d | none | Pending |
| 2 | [Docs and platform checks](./phase-02-docs-and-platform-checks.md) | 2d | 1 | Pending |

## Success criteria

- [ ] No `*_spike` names, `StudioSpikeApp` or `spike-ui` route remain; the app crate is a thin binary.
- [ ] No harness identifiers remain (`grep -rnE "qualify_|presentation_stress|stress_metrics|bounds_probe|start_agent_telemetry" desktop --include=*.rs` is empty), while the production probes listed in M2 phase 7 still exist.
- [ ] Every roadmap Done-means item is checked on Windows and on macOS, recorded in `reports/platform-checks.md`.
