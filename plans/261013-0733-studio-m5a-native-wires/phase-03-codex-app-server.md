---
phase: 3
title: "Codex app-server driver"
status: completed
priority: P1
effort: "3-4d"
dependencies: [1]
---

# Phase 3: Codex app-server driver

> **Spike S5 (2026-10-14) contradicts parts of this phase.** Read the [native-wires report, Result section](../261013-0733-studio-m1-foundations/reports/spike-native-wires.md#result) before building: neither CLI's turn-end frame means the work is done (Claude runs unprompted turns with their own `result`; Codex completes items after `turn/completed`), Codex has no `turn/aborted` and its interrupt leaves the command running, only `untrusted` asks before every command, and session stop needs interrupt, stdin close, a grace period and a containment kill.

> **Decided 2026-10-16 (user and S5 evidence):**
> - The user's Codex login and home stay, but `mcp_servers` is overridden with `-c` so only the studio MCP server runs.
> - The approval policy is `untrusted` with `workspace-write` (S5: `on-request` asked for nothing).
> - Stop is `turn/interrupt`, recognised by `turn/completed` with `status: "interrupted"`. 0.162.1 has no `turn/aborted`.
> - Late `item/completed` frames after `turn/completed` update their tool rows and do not end another turn.
> - Session stop is interrupt, then stdin close, then grace, then the containment kill.
> - Approvals are answered from `availableDecisions`, with `decline` or `cancel` used to refuse.
>
> **Corrected 2026-10-16:** checked with the 0.162.1 schema (`codex app-server generate-json-schema`) and probes on this VM.
>
> - **MCP.** `-c mcp_servers={}` does not remove the user's servers; the overrides merge. A whole `mcp_servers` object in `thread/start` config does not remove them either. What works:
>   - `config/read {cwd: draft}` lists the server names;
>   - `thread/start` `config` sets `mcp_servers.<name>.enabled: false` for each of them, plus `features.apps: false`;
>   - the same `config` adds `mcp_servers.studio.{command,args,env}`.
>
>   With that, only `studio` starts. This is the user's outcome (studio MCP only), reached through `thread/start` config instead of `-c`. It also keeps the server's secrets off the command line.
> - **Decisions.** `availableDecisions` is not in the schema, though S5 frames carried it, so the driver treats it as optional. Refusing always sends `decline`; S5 showed it works. `cancel` also interrupts the turn, so a request closed as cancelled is answered `decline` too, and Stop sends `turn/interrupt`.
> - **Login.** `account/read` returns `{account, requiresOpenaiAuth}`. A null account with `requiresOpenaiAuth: true` means signed out, so the probe can report the login. Only `account.type` and null-ness are read; the email is never logged or exported.
> - **Interrupt.** `turn/interrupt` is sent only after `turn/started`. Sent earlier, it fails with "no active turn to interrupt" (Zeron).
> - **Background.** Commands still `inProgress` at `turn/completed` are reported as background work before the turn's end, so the engine's capture deferral covers them. The late `item/completed` lowers the count. Codex runs no turn of its own.
> - **Steer.** `turn/steer {threadId, expectedTurnId, input}` is accepted only while the Codex turn runs. `steer()` waits for the reply and returns a refusal as its error. The driver never starts a turn by itself.
> - Codex 0.162.1 is installed and signed in on this VM, so the real-CLI checks run here.

## Goal

Codex runs as `codex app-server` over JSON-RPC, with every approval answered by the user.

## Context

- Spike findings: `../261013-0733-studio-m1-foundations/reports/spike-native-wires.md` (S5).
- Zeron: `codex/mod.rs` (2,532 lines; `:770-792` policies, `:1542` `turn/plan/updated`), `codex/normalize.rs` (1,202). Zeron validated against codex-cli 0.153.4 with the experimental API.
- Codex 0.162.1 is installed and signed in on this VM.
- Schema: `codex app-server generate-json-schema --out DIR`.

## Tasks

1. Launch `codex app-server` inside the M3 writer containment. Handshake: `initialize {clientInfo: {name: "fframes-studio"}, capabilities: {experimentalApi: true}}`, then `initialized`, then `thread/start` (cwd = the task's draft directory, as ACP uses, never the live project) or `thread/resume`. Pass `studio-mcp` through Codex's `config` overrides.
2. Turns: `turn/start {threadId, input, approvalPolicy, sandboxPolicy: workspace-write with the draft directory as the only writable root}`, with `approvalPolicy` `on-request` (asks only outside the sandbox) or `untrusted` (asks before most commands), chosen from the S5 findings; never `"never"` with `danger-full-access`. Steer with `turn/steer {expectedTurnId, ...}`; stop with `turn/interrupt`.
3. Approvals: every `requestApproval` request (commands and file changes) becomes a permission row; `item/tool/requestUserInput` becomes a `Question` row. No answer means declined.
4. Turn end: `turn/completed`, `turn/failed` or `turn/aborted` only.
5. Normalise: message and reasoning deltas, command and file-change items as tool rows (16 KB output cap), `turn/plan/updated` to `Plan`, token usage. Slash commands: `/compact` maps to `thread/compact/start`, `/review` to `review/start`; others are passed as text.
6. Split as `codex/{mod,rpc,turn,approvals,normalize}.rs`, each at most 1,500 lines. Version gate as in phase 2, at the version S5 validated.

## Todo

- [x] Launch, handshake, thread start and resume (a thread that is gone starts a new one, reported as not restored)
- [x] Turns with `untrusted` approvals and the workspace-write sandbox (the draft is the only writable root, no network)
- [x] Approvals and user input routed to the user. This includes requests from subagent threads. Requests the driver does not handle are refused with -32601.
- [x] Turn end on `turn/completed` only. Commands still running at that point are reported as `Background` before the end.
- [x] Normaliser and slash commands:
  - message and reasoning deltas;
  - command, file-change, MCP and web-search items as tool rows;
  - `turn/plan/updated` as `Plan`;
  - `/compact` as `thread/compact/start` and `/review [instructions]` as `review/start` (inline).

  Probes showed that a review announces a turn id other than its reply's, so its interrupt uses the announced id. A refused slash command ends only its turn, as incomplete.

  Not done, as in the other drivers: tool output and token usage. `ToolEvent` carries no output, and no driver reports usage, so the 16 KB cap has nothing to apply to.
- [x] Version gate (0.162.1, shared with Claude in `runtime/native_launch.rs`). The files are split as `codex/{mod,launch,rpc,connection}.rs`, all under 900 lines, instead of the planned `turn`, `approvals` and `normalize` modules.
- [x] Engine workflow cases with a Codex-wire fake (`app/tests/agent_workflow/codex_wire.rs`):
  - capture waits for a command that outlives its turn;
  - Stop during that wait captures nothing and leaves no process.
- [x] Checks with real Codex on this VM ([report](../../desktop/plans/reports/native-wires-checks.md#codex-2026-10-16)):
  - decline blocks and accept writes inside the draft;
  - interrupt keeps the server up;
  - an accepted `ping` stopped mid-run keeps running and is reported as background work;
  - session stop then ends it.

## Success criteria

The Codex tests pass with the fake CLI. With real Codex on this VM:
- an action that needs approval under `untrusted` is asked;
- decline blocks it, and accept runs it inside the draft;
- interrupt ends the turn with `turn/completed` status `interrupted`, and the server stays up;
- only the `studio` MCP server starts;
- no command runner is left after the session stops.

## Risks

- The app-server API is experimental and changes between releases: the version gate pins the validated range, and the fake CLI fixtures are recorded from that version.
