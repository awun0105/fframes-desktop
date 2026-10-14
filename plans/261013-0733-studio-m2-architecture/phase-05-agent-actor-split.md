---
phase: 5
title: "Split the agent actor"
status: in-progress
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
- [ ] Snapshots through engine events
- [ ] Agent tests green

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
