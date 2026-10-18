---
phase: 5
title: "Split the agent actor"
status: done
priority: P1
effort: "2-3d"
dependencies: [3]
---

# Phase 5: Split the agent actor

## Goal

`studio-engine/src/agent/actor.rs` (4,491 lines after the move) is split by responsibility, with no behavior change. `log.rs` (1,255) stays under the limit.

## Tasks

1. Scout `actor.rs`: the `Msg` enum (around line 61), the `Actor` struct (around 229), and the handler groups (turn lifecycle, permissions, apply and validation, session restore, tool calls, checkpoints).
2. Split into `agent/actor/{mod,turn,permissions,apply,session,tools}.rs`; `mod.rs` keeps `Actor` and the message loop and dispatches to the others.
3. Route the actor's snapshot publishing through the M2 phase 3 event channels instead of its own inbox wake.
4. Keep the moved `studio-engine/tests/agent_workflow.rs`, `provider_handoff.rs`, `studio-engine/tests/agent_workflow_apply_gate.rs` (moved in phase 1; asserts apply is blocked off Linux until M3 phase 3 rewrites it) and the parity agent test green.

## Todo

- [x] Scout handler groups
- [x] Split modules
- [x] Snapshots through engine events
- [x] Agent tests green

## Success criteria

All agent integration tests and parity tests pass; no file in `studio-engine/src/agent` exceeds 1,500 lines.

## Progress (2026-10-14)

Tasks 1 and 2 were done ahead of phase 3, since they need nothing from it. The split followed the file's own section markers rather than the planned names:

| Module | Contents |
|--------|----------|
| `actor/mod.rs` (804) | `Msg`, `Actor` state, the run loop, message dispatch, `conflict_of_report` |
| `actor/rows.rs` (503) | transcript rows and the published snapshot |
| `actor/turn.rs` (543) | adapter probe, starting a task: tools, the writer process, the task phase |
| `actor/prompt.rs` (680) | pumping the live driver, scope evidence, prompts, agent events, permissions |
| `actor/session.rs` (770) | provider failure and stop, provider handoff, session restoration, new chats |
| `actor/apply.rs` (773) | the validation pipeline, review and publication |
| `actor/finish.rs` (446) | Undo, review policy and recovery, finishing a task, closing the actor |

Permissions live with the turn that raises them (`prompt.rs`). Tool settings live with task start (`turn.rs`). Each child is an `impl Actor` block with `use super::*`. The only edits were `pub(super)` on methods that a sibling calls. A line-multiset diff against `HEAD` shows only module wiring and rustfmt reflows. The full workspace run matches the baseline per binary. The moved code has no `cfg` gates, so the Windows-only check covers it.

Task 3 (snapshots through engine events) waits for phase 3's channels.

## Progress (2026-10-15)

Task 3 is done. The actor itself did not change: it already calls the workflow's `notify` on every snapshot publish and evidence image. What changed is where that call goes.

- `providers::host::WorkflowWake` is one workflow's wake: the first publish after `take` signals, later ones only mark it pending. `host::open_workflow` takes one, and `Opened` returns it.
- The engine builds it in `open_workflow` with a signal that sends `EngineEvent::WorkflowChanged { session, serial }`. `AppState` drops the event like any other session-scoped one, so a window that left the session never hears its workflow.
- The shell keeps the attached workflow's wake. An event whose serial is the attached one refreshes the agent half: it takes the wake, then reports the displayed revision, retries the build selection and polls the panel. Every refresh (also one woken by the inbox) takes the wake first, so no read leaves it pending. That includes the refresh right after an open is adopted: a workflow publishes while it opens, before the shell attaches it, and its event is ignored for not being attached yet. Without a take there, every later wake was lost.
- Kept on the inbox, on purpose: open results, hand-offs (a hand-off of an old serial is dropped and its staged worker reaped) and the panel's history-page wake. The inbox's single `dirty` flag is why it was not moved to the engine channel as a whole: a stale wake would strand the next session.

Tests: a `WorkflowWake` unit test; an engine-handle test (five publishes give one event stamped with the session and serial, nothing more until it is taken); an `AppState` test (a change from a session no longer open is dropped); and the parity test `the_agent_of_the_project_switched_to_still_updates_the_panel` (a reply streams to its last chunk in the project opened mid-turn). Two mutations each fail all six agent parity tests: the shell ignoring the event, and never taking the wake. An earlier version took the wake at adoption separately; removing that take also failed all six, and the take now belongs to the refresh that follows adoption. `app/tests/agent_workflow_ui.rs` (Linux only) passes a wake to `host::open_workflow`; it was type-checked here with its gate removed. Task 4 is closed by the Linux CI job, which runs it.

Task 4 closed: CI run 38614584722 passed on both jobs. On Linux, `agent_workflow_ui` ran its 13 tests, and the engine's agent integration tests and the agent parity tests passed on Windows and Linux. No file in `studio-engine/src/agent` exceeds 1,500 lines (the largest is `log.rs` at 1,255 lines).
