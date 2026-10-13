---
phase: 3
title: "Codex app-server driver"
status: pending
priority: P1
effort: "3-4d"
dependencies: [1]
---

# Phase 3: Codex app-server driver

## Goal

Codex runs as `codex app-server` over JSON-RPC, with every approval answered by the user.

## Context

- Spike findings: `../261013-0733-studio-m1-foundations/reports/spike-native-wires.md` (S5).
- Zeron: `codex/mod.rs` (2,532 lines; `:770-792` policies, `:1542` `turn/plan/updated`), `codex/normalize.rs` (1,202). Zeron validated against codex-cli 0.153.4 with the experimental API.
- Codex is not installed on this VM; real-CLI checks run where it is installed and signed in.

## Tasks

1. Launch `codex app-server` inside the M3 writer containment. Handshake: `initialize {clientInfo: {name: "fframes-studio"}, capabilities: {experimentalApi: true}}`, then `initialized`, then `thread/start` (cwd = the task's draft directory, as ACP uses, never the live project) or `thread/resume`. Pass `studio-mcp` through Codex's `config` overrides.
2. Turns: `turn/start {threadId, input, approvalPolicy, sandboxPolicy: workspace-write with the draft directory as the only writable root}`, with `approvalPolicy` `on-request` (asks only outside the sandbox) or `untrusted` (asks before most commands), chosen from the S5 findings; never `"never"` with `danger-full-access`. Steer with `turn/steer {expectedTurnId, ...}`; stop with `turn/interrupt`.
3. Approvals: every `requestApproval` request (commands and file changes) becomes a permission row; `item/tool/requestUserInput` becomes a `Question` row. No answer means declined.
4. Turn end: `turn/completed`, `turn/failed` or `turn/aborted` only.
5. Normalise: message and reasoning deltas, command and file-change items as tool rows (16 KB output cap), `turn/plan/updated` to `Plan`, token usage. Slash commands: `/compact` maps to `thread/compact/start`, `/review` to `review/start`; others are passed as text.
6. Split as `codex/{mod,rpc,turn,approvals,normalize}.rs`, each at most 1,500 lines. Version gate as in phase 2, at the version S5 validated.

## Todo

- [ ] Launch, handshake, thread start and resume
- [ ] Turns with on-request approvals and workspace sandbox
- [ ] Approvals and user input routed to the user
- [ ] Turn end
- [ ] Normaliser and slash command mapping
- [ ] Split files and version gate

## Success criteria

The phase 4 Codex tests pass with the fake CLI; on a machine with Codex installed, an action that needs approval under the chosen policy is asked, decline blocks it, and interrupt ends the turn with `turn/aborted`.

## Risks

- The app-server API is experimental and changes between releases: the version gate pins the validated range, and the fake CLI fixtures are recorded from that version.
