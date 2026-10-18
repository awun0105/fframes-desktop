---
phase: 2
title: "Claude Code stream-json driver"
status: completed
priority: P1
effort: "3-4d"
dependencies: [1]
---

# Phase 2: Claude Code stream-json driver

> **Spike S5 (2026-10-14) contradicts parts of this phase.** Read the [native-wires report, Result section](../261013-0733-studio-m1-foundations/reports/spike-native-wires.md#result) before building: neither CLI's turn-end frame means the work is done (Claude runs unprompted turns with their own `result`; Codex completes items after `turn/completed`), Codex has no `turn/aborted` and its interrupt leaves the command running, only `untrusted` asks before every command, and session stop needs interrupt, stdin close, a grace period and a containment kill.

> **Decided 2026-10-14:** draft folders are untrusted, so Claude ignores allow rules in their shared `.claude/settings.json` ([spike S5](../261013-0733-studio-m1-foundations/reports/spike-native-wires.md)). The driver passes the studio-owned allow list with `--settings` on every launch, writes nothing into the project, and surfaces the CLI's stderr notice about ignored project rules.

> **Decided 2026-10-16 (user):**
> - Settings: launch with `--setting-sources project,local --strict-mcp-config` and only the studio MCP server. User hooks, skills, agents and MCP servers do not load.
> - Autonomous follow-up turns (a `result` preceded by `task_notification` and no replayed user frame) are surfaced as their own agent turn, marked as continued after a background task, with their own candidate once writers are quiet.
> - The permission channel stays live between turns.
>
> **Decided 2026-10-16: defer capture while background work is live.**
>
> - **The problem.** At turn end the actor hands the driver to the pipeline, which reaps it first (`actor/apply.rs` `active.driver.take()`, `jobs.rs` `reap_and_observe`). That would kill a background task before Claude's follow-up turn.
> - **Why the turns cannot get separate candidates.** The background command is itself a live process in the writer tree. In the S5 background run:
>   - the first `result` arrived at 5.6 s, while `sleep 45` was running;
>   - the task completed at 49.3 s, and the autonomous turn started straight away;
>   - the tree went quiet at 51.2 s;
>   - the second `result` arrived at 52.1 s.
>
>   The first turn's writers are never quiet before the autonomous turn begins. A separate candidate for the first turn is therefore impossible under any design, including capturing while the adapter idles, which would also break M3's live-writer gate. Deferring is the design that keeps the user's decision: the autonomous turn is its own turn, and the candidate is captured once writers are quiet.
> - **Mechanism (in the driver trait, shared with Codex).** Codex's `turn/completed` also arrives while a background command still runs.
>   - The driver reports whether background work is live. ACP always reports none.
>   - The actor starts the pipeline only when no background work is live and no turn is in progress. It re-checks on every turn end and every change to the task set.
>   - A turn is in progress from its `init`. S5 shows `background_tasks_changed {tasks: []}` before the autonomous turn's `init`, so an empty task set alone must not start capture.
> - **While deferred:**
>   - the actor keeps pumping driver events, so a permission request in the autonomous turn becomes a card;
>   - the task shows "Waiting for a background task", with Stop enabled;
>   - a new brief goes through the existing queue.
>   - A background task that never exits, such as a dev server, ends only through Stop: interrupt, close stdin, wait the grace period, kill through containment, cancel. That leaves no candidate. No "capture anyway" action is added.
> - `PromptOutcome` gains `origin` (prompted or autonomous) so the UI can label the continued turn.
> - **Fake-CLI cases (phase 4):** the full background replay starts the pipeline exactly once, after the second `result`. A gated tool call in the autonomous turn shows a permission card. Stop during the wait leaves no process behind.

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

- [x] Launch with user-answered permissions and MCP config
- [x] stdin protocol, images, steering
- [ ] `can_use_tool` and `AskUserQuestion` routed to the user. Driver done and tested: every question and free text reach `updatedInput`. Still open: the actor's question card shows only the first question's options and fills the rest with defaults, and a free-text question with no options leaves only Cancel. Fix the card or the actor's answer mapping in the M5 composer work.
- [x] Interrupt and session stop
- [x] Turn end on `result`
- [x] Normaliser, split files
- [x] Version gate
- [x] Engine: capture deferred while background work is live (`TaskPhase::WaitingBackground`); the agent's own turn is announced with `TurnStarted`
- [x] Engine workflow cases with a Claude-wire fake (`app/tests/agent_workflow/claude_wire.rs`):
  - nothing is compiled until the agent's own turn ends, and that turn's permission card shows and its edit is captured;
  - capture starts once Claude does not continue;
  - Stop during the wait captures nothing and leaves no process.

  All three fail when the deferral is removed.
- [x] Checks with real Claude Code on this VM ([report](../../desktop/plans/reports/native-wires-checks.md)): deny blocks the edit, allow applies it, and interrupt keeps the process

## Result so far (2026-10-16)

- Code: `desktop/crates/studio-harness/src/driver/runtime/claude/` — `mod.rs` (handle, `AgentDriver`), `launch.rs` (flags, private launch files, version gate), `wire.rs` (outbound lines), `connection.rs` (loop and frame handling). ACP and Claude share `Shared` (waits, stop, permissions, turns).
- Bounds: Claude has 15 s after a `task_notification` with no turn open to start its own turn before the driver stops reporting `continuing`; a turn end is held up to 30 s while a steer has not been replayed back.
- Login: `initialize` cannot prove a login, because `account` looks the same signed in or out. The probe reports Ready after the version gate and `initialize`, and a signed-out CLI fails its first turn as `AuthRequired`.
- The version cache key includes the program's prefix arguments and their file times, so an npm entry script update is noticed.
- Tests: `crates/studio-harness/tests/claude_stream_json.rs` with `tests/fake-claude.py` (12 cases: launch flags and private files, deny/allow, multi-question answers, background then autonomous turn with a gated call, interrupt, steer taken up after the reported end, plan, images, auth, session mismatch, old version and probe, refused flags). The engine's workflow cases reuse the same fake. Matching a keyword needs it as a whole uppercase word, so a brief's prose cannot trigger one.
- Real CLI: `crates/studio-harness/tests/claude_real_cli.rs`, ignored by default, passed on this VM on 2026-10-16.

## Success criteria

The phase 4 Claude tests pass with the fake CLI; on this VM with real Claude Code, an edit is blocked when denied and applied when allowed, and interrupt keeps the process running.

## Risks

- `--permission-prompt-tool stdio` is undocumented (Zeron validated it on 2.1.228). If a Claude release drops it, the version gate refuses that release with a reason until the driver is updated.
