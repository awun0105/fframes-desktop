---
phase: 3
title: "Todo panel"
status: pending
priority: P2
effort: "1-2d"
dependencies: []
---

# Phase 3: Todo panel

## Goal

The agent's current plan shows as a todo panel docked above the composer, updated live.

Effort was 2-3d before [spike S4](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md), which compiled `todo_panel.rs` and its gpui tests on the Cap fork against an ACP-shaped plan entry; the `Plan` event in three drivers remains.

## Port sources

Zeron `crates/ui/src/todo_panel.rs` (1,144 lines; depends on frost, edge fade, queue and composer modules, replaced per spike S4). Zeron's `TodoItem` maps to ACP plan entries (content, priority, status).

## Tasks

1. Driver: map ACP `Plan` updates to `AgentEventKind::Plan`, which M5a added and which Claude `TodoWrite` and Codex `turn/plan/updated` already emit. ACP plans are still dropped in `driver/runtime/shared.rs` (`SessionUpdate::Plan`); update the comment there to say plans are displayed but carry no workflow authority. The engine ignores `Plan` today (`agent/actor/prompt.rs`), so task 2 starts there. Once ACP emits plans, check them for ACP in `studio-harness/tests/cross_driver.rs` too.
2. Publish the latest plan entries in `WorkflowSnapshot`.
3. Port the panel: collapsed summary (`2/5`), expanded list with status icons, collapse state per session.
4. Tests: a driver test that a `Plan` update produces the event; a gpui test that updates replace the list and an empty plan hides the panel.

## Todo

- [ ] Driver plan event
- [ ] Plan in snapshot
- [ ] Panel ported
- [ ] Tests

## Success criteria

The fake agent's plan updates show live; both tests pass.
