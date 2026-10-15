---
phase: 5
title: "Agent tool broker on Windows"
status: completed
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

- [x] Scout transports
- [x] Transport trait
- [x] Named-pipe transport with SID checks
- [x] Fallback limited to real failures
- [x] Windows tests (Desktop self-hosted validation run 38753289498 and Desktop CI run 38753289654 on commit 34a342f, 2026-10-15). The restricted-token client first failed DLL initialisation (`0xC0000142`) on the runner; it now uses every enabled group of the test's token as restricting SIDs.

## Progress (2026-10-15)

- **Transport.** `agent_tools::transport` holds everything platform-specific: listener, stream, client connect, the private runtime directory and the private capability files. The broker and client cores are now platform-neutral, and `BrokerError::Unsupported` and `ClientError::Unsupported` are gone. **Deviation:** it is a module with one implementation per platform under the same names, not a trait object. Nothing dispatches dynamically, so a trait would only add indirection. The Unix half is the previous code moved unchanged (cross-checked with clippy for Linux and macOS in a scratch crate; `studio-engine` itself cannot be cross-built here because of `alsa-sys`).
- **Named pipe.** `\\.\pipe\fframes-studio-<uuid>`, created with `FILE_FLAG_FIRST_PIPE_INSTANCE`, byte mode, overlapped I/O and `PIPE_REJECT_REMOTE_CLIENTS`. Its descriptor is `O:<user>D:P(A;;GA;;;<user>)` (an elevated administrator's default owner would be `Administrators`). One instance always listens, so the name never disappears between two clients.
- **Peer check.** After the hello line the server reads the client token through `ImpersonateNamedPipeClient` (a client cannot be impersonated before it sends anything). It refuses another user, a restricted token and an AppContainer. **Deviation:** the plan named `GetNamedPipeClientProcessId`; impersonation reads the token of the connection itself, without the PID-reuse race. The client opens the pipe with `SECURITY_IDENTIFICATION` and checks that the server process runs as the current user.
- **Runtime directory and capability files** are created with an owner-and-only-ACE descriptor of the current user. They are verified on the opened handle: no reparse point, owned by the current user, and every allow ACE names that user. The directory is not canonicalized, which would add a `\\?\` prefix. Because the pipe name is random, an exclusive delete-on-close `broker.lock` marks a live broker in the directory.
- **Close semantics.** Ending a connection after its replies only closes the handle; `DisconnectNamedPipe` (which discards unread replies) is used only by `shutdown()`.
- **Tests on this VM.** The whole `app/tests/agent_tools` suite (40 tests, including the restricted-token test below: limits, drippers, shutdown, CLI and MCP subprocesses) now runs on Windows too. Its file-mode, symlink and socket-path cases stay Unix-only, with Windows counterparts next to them. The transport unit tests assert on the handle that the pipe DACL grants only the current user and that `GetNamedPipeInfo` reports `PIPE_REJECT_REMOTE_CLIENTS`; they also cover private files and directories, a junction, a second listener, timeouts and close. `a_client_under_a_restricted_token_is_refused_by_the_token_check` launches `studio-tools` with `CreateProcessAsUserW` under a restricted token whose restricting SIDs include the user, so the DACL lets it in and the broker's token check must refuse it (`rejected_auth` = 1, exit code 3). Disabling the `IsTokenRestricted` check makes that test fail. AppContainer is not exercised: creating its profile is a lasting change on the host.
- **Agent turn.** The scripted agent has an `mcp_call` script field: it starts the MCP server named in `session/new` and calls a tool. `app/tests/agent_project_tools.rs` (not platform-gated) checks that a turn gets this task's `project_context` from the broker and that the capability ends with the task.
- **CLI-only route.** `cli_project_tools_remain_available_without_the_mcp_route` in `app/tests/agent_workflow_apply_gate.rs` is no longer Unix-only, and the test that expected an "unsupported broker" note on Windows is gone.
- **Fallback.** No platform gate is left on the tool route; the "no project tools" notes only report a missing helper, a disabled policy or a real start failure.

## Success criteria

An agent turn with the fake agent on this VM uses a project tool; the three tests pass on Windows CI; Unix behavior and tests are unchanged.
