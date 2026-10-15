---
title: "M2 Architecture: crates, engine events, Cap kit and layout, harness removal, file splits"
description: "Move GPUI-free services into the engine, drive the UI from typed engine events instead of timers, port Cap's theme, component kit, editor layout and settings pages, split every oversize file, and delete the bounds-probe harness, keeping every behavior."
status: in-progress
priority: P1
effort: "27-38d"
branch: feat/studio-rewrite
tags: [desktop, refactor, gpui, tech-debt]
blockedBy: [261013-0733-studio-m1-foundations]
blocks: [261013-0733-studio-m4-editing, 261013-0733-studio-m5-agent-ux]
created: 2026-10-13
---

# M2 Architecture

## Overview

Part of [the studio rewrite roadmap](../261013-0733-studio-rewrite-roadmap/plan.md). A behavior-preserving restructure: the M1 parity tests stay green after every phase. The visible change is the move to Cap's components, editor layout and settings pages (phases 2, 4 and 6); features come in M4 and M5. Because phases 2 and 4 change layout on purpose, the bounds-probe qualification jobs are advisory from phase 2 until phase 7 deletes them; merge gate 4 is checked after phase 7.

The app crate holds about 47k lines. About 18k are services with no direct GPUI import (`build_service`, `preview_coordinator`, `preview_worker_client`, `audio_service`, `export_service`, `worker_client`, `worker_project`, `candidate_runner`, `agent_workflow/*`, `agent_tools/*`), but they reach into UI modules in several places (phase 1 lists them). The shell shares an `Arc<Mutex<Controller>>` with the agent workflow, host and preset panel, and polls agent, preview and audio state on a 16 ms tick.

Writer-containment ownership (`conversation_panel/qualification.rs` `resolve_ownership`, `launch_identity`) is a production safety gate, not test code. It moves to `studio-engine::containment` in phase 1 and survives phase 7; M3 phase 4 replaces its ledger with runtime verification.

## Target layout

```
desktop/crates/
  studio-proto     typed EngineCommand / EngineEvent and snapshot types (no gpui)
  studio-theme     Cap's theme model and tokens, light/dark from the OS (gpui)
  studio-engine    existing engine + services, containment, agent, tools (no gpui)
  studio-harness   renamed studio-agent-spike: ACP driver, discovery (no gpui)
  studio-ui        Cap kit, Cap editor layout (shell, panels, sidebar), player, timeline, settings pages, agent panel; parity tests (gpui)
  (studio-markdown is added in M5)
desktop/app        thin binary: runtime setup, window, bins studio-tools / studio-mcp
```

## State lifetimes

- `EngineHandle`: one per app process. It owns the process-wide services (shared build service, teardown) and at most one open `ProjectSession`.
- `ProjectSession`: one per open project; owns the `Controller`, agent actor, timing history (M4), transport (M4). Every event carries the session id; `AppState` drops events from a closed session.
- `AppState`: one gpui entity per window, reset when the session id changes.

## Phases

| # | Phase | Effort | Depends on | Status |
|---|-------|--------|-----------|--------|
| 1 | [Crate skeleton and service move](./phase-01-crate-skeleton.md) | 4-5d | none | Done (Linux + Windows CI) |
| 2 | [Cap theme and component kit](./phase-02-theme-and-ui-kit.md) | 3-4d | 1 | Done (Windows) except the hand check of the live OS flip; the preference control moved to phase 4 and is done there (the settings frame's Appearance page) |
| 3 | [Engine handle and typed events](./phase-03-engine-handle-and-events.md) | 4-5d | 1 | Done (Windows) |
| 4 | [Split the shell into Cap's editor layout](./phase-04-shell-split.md) | 6-9d | 2, 3 | Done (Windows): the views and the shell are in `studio-ui`, split into sections, with the parity suite; the keymap, Cap's layout, the Windows caption and the settings frame with its Appearance page. Adapter setup joins the frame in phase 6. Not verified: macOS, and real-input checks of the title bar on Windows (a drag, the hover colours, and section renders under real mouse moves) |
| 5 | [Split the agent actor](./phase-05-agent-actor-split.md) | 2-3d | 3 | Done |
| 6 | [Agent panel on a transcript row model](./phase-06-transcript-row-model.md) | 3-5d | 4, 5, 7 | Pending |
| 7 | [Remove the bounds-probe harness](./phase-07-remove-probe-harness.md) | 2-3d | 4 | Done (release dry run left to the user; full CI runs fail on the X11 task test's process cleanup, a separate fix) |
| 8 | [Split the remaining oversize files](./phase-08-split-oversize-files.md) | 3-4d | 1 | In progress |

Spike S4 ([port closure](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md)) re-checked phases 2, 4 and 6: the kit and theme port verbatim, but the layout and the transcript are rewrites against snapshots, so phases 4 and 6 grew.

## Success criteria

- [ ] Parity tests, `cargo test` and `just clippy` pass after each phase.
- [ ] No `.rs` file under `desktop/` exceeds 1,500 lines (merge gate 6).
- [ ] `studio-engine`, `studio-proto` and `studio-harness` have no `gpui` dependency (`cargo tree -p <crate> -e normal | grep -c gpui` is 0).
- [ ] UI crates hold no `Mutex<Controller>` and no agent or preview polling; the 16 ms playback tick (which reads the audio snapshot) stays until M4 phase 6; the engine may keep internal locking.
- [ ] The bounds-probe harness, qualification scripts and their CI jobs are gone; containment, writer qualification (`qualify-writer`, until M3 phase 4) and the release gate replacement work.
