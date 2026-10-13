---
title: "M1 Foundations: feasibility spikes, Cap GPUI switch, parity tests"
description: "Prove the five risky assumptions, move the current app unchanged onto Zeron's GPUI fork, and pin today's behavior with gpui parity tests before any restructuring."
status: pending
priority: P1
effort: "12-17d"
branch: feat/studio-rewrite
tags: [desktop, gpui, infra, critical]
blockedBy: [261013-1010-faster-desktop-ci]
blocks: [261013-0733-studio-m2-architecture, 261013-0733-studio-m3-durable-writes]
created: 2026-10-13
---

# M1 Foundations

## Overview

Part of [the studio rewrite roadmap](../261013-0733-studio-rewrite-roadmap/plan.md). Five spikes decide whether the roadmap holds and size M2 to M5. Then the existing app moves to Cap's GPUI fork with no behavior change, and gpui tests pin the parity list so M2 to M5 can restructure safely.

## Phases

| # | Phase | Effort | Depends on | Status |
|---|-------|--------|-----------|--------|
| 1 | [Feasibility spikes](./phase-01-feasibility-spikes.md) | 6-8d | none | In progress |
| 2 | [Switch the app to Cap's GPUI fork](./phase-02-cap-gpui-switch.md) | 3-4d | 1 | In progress |
| 3 | [Parity gpui tests](./phase-03-parity-gpui-tests.md) | 3-5d | 2 | Pending |

## Success criteria

- [ ] All five spikes pass, or the user has chosen a fallback before M2 starts; M2 and M5 estimates are updated from spike S4.
- [ ] The app runs on Cap's GPUI fork on Windows (this VM) and builds and tests on macOS CI.
- [ ] `cargo test -p fframes-studio --test parity` covers every parity item and passes on Windows, Linux and macOS CI.
- [ ] Existing qualification jobs still pass (they are removed in M2 phase 7).

## Parity list

Used by phase 3 and by every later milestone as the regression net. Items marked (gated) depend on mutation: on Windows and macOS they assert "blocked, with the reason shown" until M3 opens the gates, and M3 phase 3 flips those assertions.

1. Preset/Style panel: apply and reset (gated).
2. Setup screen and adapter detection.
3. Canvas `select_at` and source-click mapping.
4. Build, preview and export services (start, cancel, failure surfaced).
5. Checkpoints: create and restore (gated).
6. Audio: mute and output switching.
7. Timeline: scene cycling, range selection, zoom, seek from the ruler.
8. Transport: play/pause, frame stepping.
9. Agent panel: send a prompt to the fake ACP agent, stream a reply, answer a permission request, apply (gated).
