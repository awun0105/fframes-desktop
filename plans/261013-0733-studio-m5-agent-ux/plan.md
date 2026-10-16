---
title: "M5 Agent UX: streaming markdown, composer, todos, diff review, history"
description: "Port Zeron's markdown, composer, todo panel, change viewer and search onto the M2 transcript rows, extend the ACP driver for plans and commands, make partial apply validated, and store many sessions per project."
status: in-progress
priority: P1
effort: "16-24d"
branch: feat/studio-rewrite
tags: [desktop, feature, gpui, frontend, security]
blockedBy: [261013-0733-studio-m2-architecture, 261013-0733-studio-m3-durable-writes, 261013-0733-studio-m5a-native-wires]
blocks: [261013-0733-studio-m6-cleanup]
created: 2026-10-13
---

# M5 Agent UX

## Overview

Part of [the studio rewrite roadmap](../261013-0733-studio-rewrite-roadmap/plan.md). Zeron sources are pinned at `09beace0` on zui, a different GPUI from the Cap fork the app uses, so every port also adapts GPUI calls (spike S4 measures it). Its UI files also depend on `zeron_proto`, `zeron_doc`, `zeron_rpc`, `zeron_syntax` and many `crate::*` modules; `reports/spike-port-closure.md` (M1 spike S4) lists what each port keeps, replaces or drops. Frost, dictation, voice and Zeron settings are dropped everywhere.

Chrome is Cap's: the composer frame, todo panel frame, change viewer toolbar and file list, history list and every button, menu, select, tooltip, collapsible and text input come from the Cap kit (M2 phase 2), styled with Cap's theme. Zeron supplies the logic and paint internals in the table below.

Every M5 feature works through the driver trait from [M5a](../261013-0733-studio-m5a-native-wires/plan.md), for ACP, Claude Code and Codex alike; driver tasks below add events to all three. The panel is a tab in the editor sidebar.

What the ACP driver (`studio-harness`, `driver/runtime/shared.rs` since M5a) does today (verified 2026-10-13; plan events re-checked 2026-10-16):

- Handles message, thought, tool call, tool call update, mode and config updates, and cancel.
- **Drops** `Plan` and `AvailableCommandsUpdate` on purpose ("Plans, command lists, usage and session info carry no workflow authority"); M5a added `AgentEventKind::Plan`, which the Claude and Codex drivers emit, but ACP still drops its plans, there is no commands variant, and the engine ignores `Plan`. M5 reverses that for display only: plans and commands stay without workflow authority.
- **Already sends images** through `prompt_with_images`, gated on `prompt_capabilities.image`, PNG only, count- and size-capped, never persisted.
- **Already restores sessions** (`resume_session` preferred over `load_session`); restored non-text content becomes a placeholder.
- One session manifest and one `conversation.jsonl` per project (`session_store`, `deny_unknown_fields`).

| Zeron source | Target |
|--------------|--------|
| `crates/markdown/src/{lib,parser,mend}.rs` | `studio-markdown` (split `parser.rs`, 1,735 lines) |
| `crates/ui/src/markdown/{mod,render,veil,selection,links,link_*,inline_code_links}.rs` | `studio-ui/src/agent/markdown/` (no Mermaid) |
| `crates/ui/src/composer.rs` and `composer/` | `studio-ui/src/agent/composer/{mod,input,slash,attachments,steer,keymap}.rs` |
| `crates/ui/src/todo_panel.rs` | `studio-ui/src/agent/todo_panel.rs` |
| `crates/ui/src/changes.rs` | `studio-ui/src/agent/changes/{mod,file_list,unified,split,paint}.rs` |
| `crates/ui/src/shell/thread_search.rs` (scoring idea only) | `studio-ui/src/agent/history/search.rs` |

Syntax highlighting is written as a small module with five tree-sitter grammars (Rust, TOML, JSON, Markdown, shell), not a pruned copy of Zeron's 27-grammar crate.

## Phases

| # | Phase | Effort | Depends on | Status |
|---|-------|--------|-----------|--------|
| 1 | [Streaming markdown](./phase-01-streaming-markdown.md) | 3-5d | none | Completed |
| 2 | [Composer](./phase-02-composer.md) | 5-7d | none | Completed ([component](./reports/phase-02-composer-ui.md), [driver and engine](./reports/phase-02-03-driver-engine.md), [security review](./reports/phase-02-03-review.md), [panel wiring](./reports/wiring-agent-panel.md)) |
| 3 | [Todo panel](./phase-03-todo-panel.md) | 1-2d | none | Completed ([component](./reports/phase-03-todo-panel.md), [driver and engine](./reports/phase-02-03-driver-engine.md), [panel wiring](./reports/wiring-agent-panel.md)) |
| 4 | [Diff review](./phase-04-diff-review.md) | 4-6d | 1 | Completed ([engine](./reports/phase-04-engine.md), [viewer](./reports/phase-04-viewer.md), [wiring](./reports/phase-04-wiring.md)) |
| 5 | [Session history](./phase-05-session-history.md) | 3-4d | none | Completed ([storage](./reports/phase-05-storage-and-list.md), [review](./reports/phase-05-review.md), [lifecycle](./reports/phase-05-lifecycle.md), [lifecycle review](./reports/phase-05-lifecycle-review.md), [merge checks](./reports/merge-verification.md)) |

Spike S4 ([port closure](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md)) compiled the todo panel and `ComposerInput` on the Cap fork with one GPUI API change. Phase 3 shrinks; phase 2 holds at 5-7d because it keeps `ComposerInput` (decided 2026-10-14) rather than building on the kit `TextInput` (8-10d).

## Handed to M6 (2026-10-17)

The disabled-kind chips, the silent disabled paste and the slash menu inside backticks were closed by the wiring ([report](./reports/wiring-agent-panel.md)). What remains goes to [M6 cleanup](../261013-0733-studio-m6-cleanup/plan.md):

- **Composer base.** The reasons recorded for keeping `ComposerInput` were projection, inline mention and attachment chips, and Markdown faces; the port drops all three, so what remains (multiline wrap, undo, IME, the Enter keymap) overlaps heavily with the kit `TextInput`. Revisit when `@` mentions land. The 2026-10-14 decision stands.
- **Driver limitations** ([driver report](./reports/phase-02-03-driver-engine.md)): plan updates are not coalesced; the workspace `image` feature is enabled for the whole workspace; a plan or command update that arrives before the session id is stored is accepted (display only); an adapter change clears the command list, covered by a unit test only.
- **Session limitations** ([lifecycle report](./reports/phase-05-lifecycle.md)): the startup offer of the saved session skips the stored-capability gate (the read-only fallback still applies); a new chat while the legacy log is served hides the legacy conversation if its migration keeps failing; listing sessions reads every manifest.
- **Not driven in the real app:** native file dialogs and the Ctrl+Enter chord (both covered by parity tests); the review-banner layout fix is covered by a parity test, not by a screenshot.
- **Intermittent parity hang, not from the review engine.** `services::cancelling_an_export_leaves_no_file` sometimes never lets the frame loop settle after a cancelled export, and the harness watchdog ends the parity process with exit 101. On this VM it hung in 1 of 4 runs of `services::` without the per-file review engine and 2 of 4 with it (both branches already held the markdown phase), so it predates the engine work. It also failed the self-hosted Linux job of CI run 38917158146 once; a re-run passed.
- **Intermittent macOS failures outside M5.** `build_sharing::cancelling_the_ui_compile_scope_leaves_a_tool_subscriber_running` (subscriber count) and `agent_workflow::regressions::a_check_still_running_when_the_adapter_changes_is_redone_for_the_new_adapter` (check never started) each failed one macOS run and passed on re-run.
- **Other intermittent failures, all passing when re-run alone.** Parity: `services::an_unmigrated_project_exports_only_the_default_whatever_was_saved` (twice, both in runs that also stalled on exports), `agent::undo_in_the_prompt_undoes_the_typing_there`, and on macOS CI `agent::switching_projects_mid_turn_shows_nothing_of_the_old_turn`. Self-hosted CI: `agent_workflow::codex_wire::capture_waits_for_a_command_that_outlives_its_turn` (5 of 5 alone here). They point at timing waits in the harnesses under load. One M5 test had the same kind of failure and was fixed: `composer::the_history_lists_searches_and_reopens_stored_sessions` once clicked the History tab at bounds measured before a notice moved the tabs, so the composer cases now click only once two measurements a frame apart agree.

## Success criteria

- [ ] An agent turn against each fake agent (ACP, Claude, Codex) renders markdown with highlighted code while streaming, shows its todo list, and shows a diff with per-file accept (validated) and revert, on Windows and macOS.
- [ ] The composer supports multiline input, slash commands from the agent, file and image attachments within confinement rules, stop, and steering.
- [ ] Many past sessions per project can be listed, searched and reopened; the existing single session migrates.
- [ ] No link or file from agent output is ever executed by the OS.
- [ ] Parity agent tests stay green; new gpui tests cover each feature.
