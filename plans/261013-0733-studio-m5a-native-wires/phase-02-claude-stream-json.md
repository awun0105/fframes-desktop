---
phase: 2
title: "Claude Code stream-json driver"
status: pending
priority: P1
effort: "3-4d"
dependencies: [1]
---

# Phase 2: Claude Code stream-json driver

## Goal

Claude Code runs as one persistent process over its stream-json control protocol, with every tool permission answered by the user.

## Context

- Spike findings: `../261013-0733-studio-m1-foundations/reports/spike-native-wires.md` (S5).
- Zeron: `claude/mod.rs:213-277` (launch), `:1326-1368` (control answers), `claude/wire.rs`, `claude/normalize.rs` (`TodoWrite` at `:102`).
- This VM: Claude Code 2.1.296 at `C:\Users\Administrator\.local\bin\claude.exe`.

## Tasks

1. Launch (port of Zeron `claude/mod.rs:213-277`, with fframes' permission mode):
   `claude --print --input-format stream-json --output-format stream-json --verbose --include-partial-messages --replay-user-messages --permission-prompt-tool stdio --permission-mode default [--model M] [--resume=ID] --mcp-config <studio-mcp JSON>`. Never `bypassPermissions` or `--dangerously-skip-permissions`. Working directory: the task's draft directory, as ACP uses (`actor.rs:1764`), never the live project. Inside the M3 writer containment, with `CLAUDECODE*` removed from the environment. Which user settings load (`--setting-sources`, `--strict-mcp-config`, `--bare`; the user's CLAUDE.md, rules, hooks, skills and permission allow lists) is an open question for the user, decided before this phase from the S5 findings.
2. stdin: first line the `initialize` control request; prompts as `{"type":"user",...}` lines with images as base64 blocks (5 MB cap, same count cap as ACP); steers as user lines with `priority: "now"` (step boundary) or `"next"`.
3. Permissions: each `can_use_tool` control request becomes a permission row; the user's answer goes back as `{"behavior":"allow"}` or `{"behavior":"deny","message":...}`. No answer means no tool. `AskUserQuestion` becomes a `Question` row; answers return in `updatedInput`.
4. Stop: the `interrupt` control request (`cancel_queued: true`); the process stays up (`stops_turn_in_place`). Stop of the session closes stdin, waits `shutdown_grace`, then terminates through containment.
5. Turn end: the `result` frame only (`deterministic_turn_end`); candidate capture still waits for writer quiescence.
6. Normalise frames: text and thinking deltas, tool use and results (16 KB output cap as today), `TodoWrite` input to `Plan`, frames with `parent_tool_use_id` grouped under their parent tool row, usage. Split as `claude/{mod,launch,wire,control,normalize}.rs`, each at most 1,500 lines.
7. Version gate: run `claude --version` (2 s timeout, cached by path, mtime and size); refuse versions below the one S5 validated, with the reason on the setup page.

## Todo

- [ ] Launch with user-answered permissions and MCP config
- [ ] stdin protocol, images, steering
- [ ] `can_use_tool` and `AskUserQuestion` routed to the user
- [ ] Interrupt and session stop
- [ ] Turn end on `result`
- [ ] Normaliser, split files
- [ ] Version gate

## Success criteria

The phase 4 Claude tests pass with the fake CLI; on this VM with real Claude Code, an edit is blocked when denied and applied when allowed, and interrupt keeps the process running.

## Risks

- `--permission-prompt-tool stdio` is undocumented (Zeron validated it on 2.1.228). If a Claude release drops it, the version gate refuses that release with a reason until the driver is updated.
