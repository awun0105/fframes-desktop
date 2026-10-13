---
phase: 5
title: "Agent tool broker on Windows"
status: pending
priority: P1
effort: "3-5d"
dependencies: []
---

# Phase 5: Agent tool broker on Windows

## Goal

Agents on Windows get the same project tools (build, preview, inspect through `studio-tools` and `studio-mcp`) as on Linux and macOS. Starts after M2 phase 1 moves `agent_tools` into the engine.

## Context

- `agent_tools/broker.rs` module doc: Unix socket with mode 0600 and a peer-uid check; `#[cfg(not(unix))]` returns `BrokerError::Unsupported` (around `:1124-1135`); `client.rs` likewise (around `:620-631`); the actor falls back to "no project tools" (`agent_workflow/actor.rs` around `:1677-1681`).

## Tasks

1. Scout the broker and client transports and every `cfg(unix)` site (9 in the broker, 14 in the client on 2026-10-13).
2. Put the transport behind a small trait with the existing Unix socket implementation and a Windows named-pipe implementation: a pipe with a per-session random name, a DACL that grants only the current user's SID, `PIPE_REJECT_REMOTE_CLIENTS`, and a check of the client's process user SID (`GetNamedPipeClientProcessId`, token user) equivalent to the peer-uid check.
3. Keep the capability grant protocol unchanged.
4. Remove the "no project tools" fallback message for Windows; keep it only for real failures.
5. Tests on Windows CI, without new accounts or network changes on the host: a granted capability round-trips; a client launched under a restricted token or in an AppContainer (a different SID, same account) is refused; the created pipe's DACL grants only the current user's SID and its mode includes `PIPE_REJECT_REMOTE_CLIENTS` (asserted on the handle, not by connecting from another machine).

## Todo

- [ ] Scout transports
- [ ] Transport trait
- [ ] Named-pipe transport with SID checks
- [ ] Fallback limited to real failures
- [ ] Windows tests

## Success criteria

An agent turn with the fake agent on this VM uses a project tool; the three tests pass on Windows CI; Unix behavior and tests are unchanged.
