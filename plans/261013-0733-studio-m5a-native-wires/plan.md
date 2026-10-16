---
title: "M5a Native agent wires: Claude stream-json and Codex app-server"
description: "Put the agent driver behind one trait, keep ACP for other agents, and add native drivers for Claude Code (stream-json control protocol) and Codex (app-server JSON-RPC), ported from Zeron's harness, with every approval routed to the user."
status: in-progress
priority: P1
effort: "10-14d"
branch: feat/studio-rewrite
tags: [desktop, backend, agent, security]
blockedBy: [261013-0733-studio-m2-architecture, 261013-0733-studio-m3-durable-writes]
blocks: [261013-0733-studio-m5-agent-ux]
created: 2026-10-13
---

# M5a Native agent wires

## Overview

Part of [the studio rewrite roadmap](../261013-0733-studio-rewrite-roadmap/plan.md). Zeron runs Claude Code and Codex over their native protocols, not ACP, because their ACP adapters hold turns open for background work and report false "still running" and "done" states ([Zeron report](../reports/research-261013-1749-zeron-agent-control.md), headline). fframes captures an agent's changes when a turn ends, so the turn end must be reliable. This milestone adds those two native drivers; ACP stays for every other agent (`acp_v1` tests unchanged).

Unlike Zeron, fframes never answers a request on the user's behalf: Claude's `can_use_tool` requests and Codex's approval requests become permission rows the user answers (parity item 9). What each CLI runs without asking is its own policy (Claude runs read-only tools in default mode; Codex runs with `untrusted` approvals in the `workspace-write` sandbox and asks before most commands, because S5 showed `on-request` asked for nothing). Both drivers start in the task's draft directory, like ACP (`actor.rs:1764` `cwd: active.ctx.draft`), so writer containment (M3 phase 4) and candidate validation apply as today.

Order with one executor: after M4, before M5. M1 spike S5 proves both protocols on real CLIs first.

## Port sources (Zeron `09beace0`, `crates/harness/src/`)

| Source | Lines | Take |
|--------|-------|------|
| `lib.rs` | 630 | the `Harness` trait shape (`:153-262`): run returning an event stream that ends in `Done`, `RunControls` (request input, steering mailbox, interrupt, stop turn), capability flags `deterministic_turn_end`, `stops_turn_in_place`, `reconfigures_in_place` |
| `claude/{mod,wire,normalize}.rs` | 1,512 / 507 / 1,474 | launch flags, stdin control protocol, frame normalisation (`TodoWrite` to plan entries, `parent_tool_use_id` to subagent rows) |
| `codex/{mod,normalize}.rs` | 2,532 / 1,202 | `app-server` JSON-RPC client, thread and turn calls, `turn/plan/updated`, approval requests |
| `process/owned.rs` | 127 | environment hygiene (`CLAUDECODE*` removal) |
| `tests/claude.rs`, `tests/codex.rs` | 1,459 / 2,005 | test cases against fake CLIs (`tests/fixtures/fake-claude.sh`); rewritten as Python fakes so they run on Windows |

Not taken: Zeron's always-allow answers, `approvalPolicy: "never"` with `danger-full-access`, `PATH` and login-shell discovery, model catalogs, Codex realtime and subagent tailing (`codex/realtime*.rs`, `subagents.rs`), and the Cursor, OpenCode and Pi drivers. Every file over 1,500 lines is split when ported.

## Phases

| # | Phase | Effort | Depends on | Status |
|---|-------|--------|-----------|--------|
| 1 | [Driver trait and wire selection](./phase-01-driver-trait.md) | 2-3d | none | Completed |
| 2 | [Claude Code stream-json driver](./phase-02-claude-stream-json.md) | 3-4d | 1 | Completed except question answering: the engine's card answers only the first question and sends empty answers for the rest. Handed to [M5 phase 2](../261013-0733-studio-m5-agent-ux/phase-02-composer.md). |
| 3 | [Codex app-server driver](./phase-03-codex-app-server.md) | 3-4d | 1 | Completed |
| 4 | [Fake CLIs, cross-driver tests and platform checks](./phase-04-fakes-and-checks.md) | 2-3d | 2, 3 | In progress. CI green on Windows, macOS and Linux; manual checks on macOS are open. |

## Success criteria

- [x] The agent actor talks only to the driver trait; ACP, Claude and Codex implement it. The `acp_v1` tests pass with their test code unchanged; the ACP peer's `permission` mode now writes only on allow.
- [ ] With Claude Code on this VM: a turn streams text, asks permission before an edit (deny blocks it, allow lets it through), steers, stops without killing the process, resumes, and ends only when the `result` frame arrives; the candidate is captured after writer quiescence.
  - Against the real CLI: text, deny and allow, and stop.
  - With the fake CLI only: steer, resume, and capture after quiescence.
- [ ] The same with Codex on the user's machine (or wherever Codex is installed and signed in).
  - Against the real CLI on this VM: decline and accept, stop, and an interrupted command ended by session stop.
  - With the fake CLI only: steer and resume.
- [x] No driver answers a request on the user's behalf. The engine still does for questions 2..n of a multi-question prompt; M5 phase 2 owns that fix. `cross_driver.rs` proves for all three wires that an unanswered permission request blocks the tool, and both native drivers run in the draft directory.
- [x] Parity agent tests and the new driver tests pass on Windows, macOS and Linux CI with the fake CLIs: `6225af8`, [self-hosted Linux and Windows](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/38905684378) and [macOS 15](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/38905719933), 2026-10-16. The first Linux attempt hit the known intermittent export-cancel hang in the parity `services::` tests after every agent test had passed; the re-run passed.
