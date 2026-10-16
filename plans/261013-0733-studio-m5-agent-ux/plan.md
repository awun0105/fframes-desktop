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
| 2 | [Composer](./phase-02-composer.md) | 5-7d | none | In progress: component built and tested, not wired ([report](./reports/phase-02-composer-ui.md)) |
| 3 | [Todo panel](./phase-03-todo-panel.md) | 1-2d | none | In progress: component built and tested, not fed from agent events ([report](./reports/phase-03-todo-panel.md)) |
| 4 | [Diff review](./phase-04-diff-review.md) | 4-6d | 1 | In progress: engine, viewer and wiring done; awaiting three-platform CI of the wiring ([wiring](./reports/phase-04-wiring.md)) |
| 5 | [Session history](./phase-05-session-history.md) | 3-4d | none | In progress: storage, migration, list and load, and the search list built and tested, not called by the app ([report](./reports/phase-05-storage-and-list.md), [review](./reports/phase-05-review.md)); restore, resume and wiring wait for M5a |

Spike S4 ([port closure](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md)) compiled the todo panel and `ComposerInput` on the Cap fork with one GPUI API change. Phase 3 shrinks; phase 2 holds at 5-7d because it keeps `ComposerInput` (decided 2026-10-14) rather than building on the kit `TextInput` (8-10d).

## Open items before wiring (2026-10-16)

- **Composer base, for the plan owner to decide.** The reasons recorded for keeping `ComposerInput` were projection, inline mention and attachment chips, and Markdown faces; the port drops all three, so what remains (multiline wrap, undo, IME, the Enter keymap) overlaps heavily with the kit `TextInput`. Revisit before wiring, or restore projection when `@` mentions land. The 2026-10-14 decision stands until then.
- **Attachments.** A disabled attach kind gates new pastes and the attach button only: chips already accepted still go out with the next send, so the owner filters them, and a paste of a disabled kind is dropped without feedback.
- **Slash menu.** A `/` inside backticks still opens the completion menu (no code-span check).
- **Intermittent parity hang, not from M5.** `services::cancelling_an_export_leaves_no_file` sometimes never lets the frame loop settle after a cancelled export, and the harness watchdog ends the parity process with exit 101. On this VM it hung in 1 of 4 runs of `services::` without the M5 engine work and 2 of 4 with it, so it predates M5. CI has passed on every platform so far.
- **Viewer on screen.** Checked in a stand-in window with real clicks ([wiring report](./reports/phase-04-wiring.md)); not yet with a real agent in the running app.

## Success criteria

- [ ] An agent turn against each fake agent (ACP, Claude, Codex) renders markdown with highlighted code while streaming, shows its todo list, and shows a diff with per-file accept (validated) and revert, on Windows and macOS.
- [ ] The composer supports multiline input, slash commands from the agent, file and image attachments within confinement rules, stop, and steering.
- [ ] Many past sessions per project can be listed, searched and reopened; the existing single session migrates.
- [ ] No link or file from agent output is ever executed by the OS.
- [ ] Parity agent tests stay green; new gpui tests cover each feature.
