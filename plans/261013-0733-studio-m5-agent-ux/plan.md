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

What the ACP driver (`studio-harness`, `driver/runtime.rs`) does today (verified 2026-10-13):

- Handles message, thought, tool call, tool call update, mode and config updates, and cancel.
- **Drops** `Plan` and `AvailableCommandsUpdate` on purpose ("Plans, command lists, usage and session info carry no workflow authority"); `AgentEventKind` has no variant for them. M5 reverses that for display only: plans and commands stay without workflow authority.
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
| 2 | [Composer](./phase-02-composer.md) | 5-7d | none | Pending |
| 3 | [Todo panel](./phase-03-todo-panel.md) | 1-2d | none | Pending |
| 4 | [Diff review](./phase-04-diff-review.md) | 4-6d | 1 | Pending |
| 5 | [Session history](./phase-05-session-history.md) | 3-4d | none | Pending |

Spike S4 ([port closure](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md)) compiled the todo panel and `ComposerInput` on the Cap fork with one GPUI API change. Phase 3 shrinks; phase 2 holds at 5-7d because it keeps `ComposerInput` (decided 2026-10-14) rather than building on the kit `TextInput` (8-10d).

## Success criteria

- [ ] An agent turn against each fake agent (ACP, Claude, Codex) renders markdown with highlighted code while streaming, shows its todo list, and shows a diff with per-file accept (validated) and revert, on Windows and macOS.
- [ ] The composer supports multiline input, slash commands from the agent, file and image attachments within confinement rules, stop, and steering.
- [ ] Many past sessions per project can be listed, searched and reopened; the existing single session migrates.
- [ ] No link or file from agent output is ever executed by the OS.
- [ ] Parity agent tests stay green; new gpui tests cover each feature.
