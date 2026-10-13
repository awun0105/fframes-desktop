---
phase: 1
title: "Driver trait and wire selection"
status: pending
priority: P1
effort: "2-3d"
dependencies: []
---

# Phase 1: Driver trait and wire selection

## Goal

The engine's agent actor drives any agent through one `AgentDriver` trait, and each adapter declares which wire it speaks. ACP is the first implementation, with no behavior change.

## Context

- ACP driver: `studio-harness` (`driver.rs`, `driver/runtime.rs` split in M2 phase 8, `session.rs`, `discovery.rs`, `detect.rs`); tests `studio-harness/tests/acp_v1.rs` (split in M2 phase 8).
- Actor: `studio-engine/src/agent/actor/` (split in M2 phase 5); it calls the ACP driver directly today.
- Zeron's trait: `crates/harness/src/lib.rs:153-262`.

## Tasks

1. Scout every call the actor makes into the ACP driver and every `AgentEventKind` it consumes.
2. Define `AgentDriver` in `studio-harness/src/driver/api.rs` (`trait` is a reserved word, so not `trait.rs`): start or resume a session, prompt (text and images), steer, cancel turn, stop, answer a permission request, answer a question; an event stream of `AgentEventKind` that ends each turn with exactly one `TurnEnded { reason }`. Capability flags after Zeron: `deterministic_turn_end`, `stops_turn_in_place` (stop keeps the process), `steering: None | StepBoundary | TurnBoundary`, `images`.
3. Implement it for the ACP driver by delegation; no change to ACP behavior or tests.
4. Adapter descriptor gains `wire: Acp | ClaudeStreamJson | CodexAppServer`, chosen on the setup page and stored in the adapter file. `AdapterFile` uses `deny_unknown_fields` (`conversation_panel/host.rs:47`), so add the field with `#[serde(default)]` = `Acp`: existing adapter files keep loading; builds older than this phase reject files that carry the field, which is acceptable on the branch. Discovery stays explicit (the adapter file names the executable; `detect.rs` resolves npm `.cmd` shims); no `PATH` or login-shell lookup.
5. Add the `Question` event (a request that asks the user for input rather than permission) to `AgentEventKind`; ACP maps kind-less permission options to it (M5 phase 2 renders it).
6. Switch the actor to the trait; parity agent tests and `acp_v1` stay green.

## Todo

- [ ] Scout actor-driver calls
- [ ] Trait, flags and `TurnEnded`
- [ ] ACP behind the trait
- [ ] Wire field in the adapter descriptor and setup page
- [ ] `Question` event
- [ ] Actor on the trait; tests green

## Success criteria

`cargo test -p studio-harness` and the parity agent tests pass unchanged; `grep -rn "AcpDriver" desktop/crates/studio-engine/src` matches only the driver factory.
