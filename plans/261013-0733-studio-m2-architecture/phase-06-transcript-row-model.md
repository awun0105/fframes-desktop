---
phase: 6
title: "Agent panel on a transcript row model"
status: pending
priority: P2
effort: "3-4d"
dependencies: [4, 5, 7]
---

# Phase 6: Agent panel on a transcript row model

## Goal

The conversation panel moves to `studio-ui/src/agent/` and renders through Zeron's transcript structure (stable row ids, gpui `list()`, stick-to-bottom), still showing today's row kinds. Markdown, todos and diffs come in M5.

## Port sources

- Zeron `crates/ui/src/transcript.rs` (16,091 lines). Take only the row identity scheme (`msgId#blockId`), list state handling and stick-to-bottom (`STICK_THRESHOLD_PX` 70, `OVERDRAW_PX` 320). Leave out tool-group folding and anything that needs `zeron_doc` or `crate::attachments` (see `reports/spike-port-closure.md`). Target: `studio-ui/src/agent/transcript/{mod,rows,scroll}.rs`.
- Map Zeron's `AgentEvent` / `ToolCall` rows to fframes `RowKind` (`agent_workflow/model.rs:84`: `User`, `Agent{streaming}`, `Thought`, `Tool`, `Permission`, `Notice`, `Error`, `Validation`, `Changes`, `Outcome`).

## Tasks

1. Scout `conversation_panel.rs` (3,217 lines) and `conversation_panel/{controls,rows,provider_setup}.rs`; host and provider profiles moved to the engine in phase 1, and the qualification UI was removed in phase 7, which runs first so no dead UI is ported.
2. Create `studio-ui/src/agent/{panel,setup_tab,controls}.rs` and the transcript modules; move code over. The panel is a tab in the editor sidebar next to Presets and Scenes, laid out for the sidebar's width: transcript above, composer pinned at the bottom. Its chrome is Cap's: frame and tab strip (`TabRail`), controls (`Button`, `SegmentedControl`, `Select`, `Menu`, `Tooltip`), and the provider setup tab on the settings page frame from phase 4. Zeron's transcript supplies only row identity, list state and stick-to-bottom, adapted from zui to Cap's fork.
3. Replace the panel's list handling with the ported transcript rows; keep every row kind's current look.

## Todo

- [ ] Scout panel modules
- [ ] Panel modules moved
- [ ] Transcript rows ported and mapped to `RowKind`
- [ ] Parity agent test green

## Success criteria

Parity agent test passes; streaming text stays pinned to the bottom unless the user scrolls up more than 70 px; no file under `studio-ui/src/agent` exceeds 1,500 lines; `desktop/app/src/conversation_panel*` no longer exists.
