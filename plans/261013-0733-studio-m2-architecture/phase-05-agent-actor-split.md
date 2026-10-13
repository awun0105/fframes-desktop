---
phase: 5
title: "Split the agent actor"
status: pending
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

- [ ] Scout handler groups
- [ ] Split modules
- [ ] Snapshots through engine events
- [ ] Agent tests green

## Success criteria

All agent integration tests and parity tests pass; no file in `studio-engine/src/agent` exceeds 1,500 lines.
