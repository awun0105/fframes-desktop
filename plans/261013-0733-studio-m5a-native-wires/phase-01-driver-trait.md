---
phase: 1
title: "Driver trait and wire selection"
status: completed
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
2. (Turn end: the existing `AgentEventKind::PromptFinished(PromptOutcome)` is the plan's `TurnEnded { reason }`. The name is kept so `acp_v1` passes unchanged; `PromptOutcome.stop_reason` is the reason. Turn ids come from the driver's counter, so a driver can end a turn it started itself, such as Claude's autonomous turn in phase 2.) Define `AgentDriver` in `studio-harness/src/driver/api.rs` (`trait` is a reserved word, so not `trait.rs`): start or resume a session, prompt (text and images), steer, cancel turn, stop, answer a permission request, answer a question; an event stream of `AgentEventKind` that ends each turn with exactly one `TurnEnded { reason }`. Capability flags after Zeron: `deterministic_turn_end`, `stops_turn_in_place` (stop keeps the process), `steering: None | StepBoundary | TurnBoundary`, `images`.
3. Implement it for the ACP driver by delegation; no change to ACP behavior or tests.
4. Adapter descriptor gains `wire: Acp | ClaudeStreamJson | CodexAppServer`, chosen on the setup page and stored in the adapter file. `AdapterFile` uses `deny_unknown_fields` (`conversation_panel/host.rs:47`), so add the field with `#[serde(default)]` = `Acp`: existing adapter files keep loading; builds older than this phase reject files that carry the field, which is acceptable on the branch. Discovery stays explicit (the adapter file names the executable; `detect.rs` resolves npm `.cmd` shims); no `PATH` or login-shell lookup.
5. Add the `Question` event (a request that asks the user for input rather than permission) to `AgentEventKind`; M5 phase 2 renders it.
   - **Corrected 2026-10-16:** only the Claude and Codex wires emit it. ACP never does in this milestone.
     - ACP v1 cannot carry a "kind-less" permission option: `PermissionOptionKind` is a required, closed enum (`agent-client-protocol-schema` 1.9.1 `v1/client.rs:1092`), so the SDK rejects such a request before the driver sees it.
     - The Zeron lines cited for it (`acp/normalize.rs:488-503`) only pick an auto-approve option.
     - ACP's own user-input request is `elicitation/create` (`v1/elicitation.rs`). It needs a client capability fframes does not advertise; supporting it is separate work.
6. Switch the actor to the trait; parity agent tests and `acp_v1` stay green.

## Todo

- [x] Scout actor-driver calls
- [x] Trait, flags and `TurnEnded`
- [x] ACP behind the trait
- [x] Wire field in the adapter descriptor and setup page
- [x] `Question` event
- [x] Actor on the trait; tests green

## Result (2026-10-16)

- Until phases 2 and 3 land, the native wires are refused before anything starts. The probe reports `Failed`, and a submit shows a `wire_unavailable` error row without creating a task. The setup page shows "Not available yet" (test `a_wire_this_build_cannot_drive_is_refused_by_the_probe_and_by_submit_without_a_task`).
- ACP launch digests are unchanged. Native wires prefix the digest with the wire name, so changing the wire starts a fresh session.
- `cargo test --locked -p studio-harness -p studio-engine -p studio-ui -p studio-proto -p fframes-studio` passed: 1,177 tests, 0 failed. `acp_v1` is unchanged. The parity harness and other test fixtures changed only to add `wire: Acp` to their adapter literals. Workspace clippy with `-D warnings` and `cargo fmt` are clean. Windows only; macOS and Linux run in CI.
- Not yet exercised: the question routing (no driver emits `Question` until phase 2).

## Success criteria

`cargo test -p studio-harness` and the parity agent tests pass unchanged; `grep -rn "AcpDriver" desktop/crates/studio-engine/src` matches only the driver factory.
