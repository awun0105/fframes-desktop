---
phase: 3
title: "Todo panel"
status: pending
priority: P2
effort: "2-3d"
dependencies: []
---

# Phase 3: Todo panel

## Goal

The agent's current plan shows as a todo panel docked above the composer, updated live.

## Port sources

Zeron `crates/ui/src/todo_panel.rs` (1,144 lines; depends on frost, edge fade, queue and composer modules, replaced per spike S4). Zeron's `TodoItem` maps to ACP plan entries (content, priority, status).

## Tasks

1. Driver: add `AgentEventKind::Plan` for ACP `Plan` updates (today dropped at the catch-all in `driver/runtime.rs`), Claude `TodoWrite` input and Codex `turn/plan/updated` (both mapped in M5a); update the comment there to say plans are displayed but carry no workflow authority.
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
