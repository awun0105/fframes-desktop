---
phase: 6
title: "Agent panel on a transcript row model"
status: done
priority: P2
effort: "3-5d"
dependencies: [4, 5, 7]
---

# Phase 6: Agent panel on a transcript row model

## Goal

The conversation panel moves to `studio-ui/src/agent/` and renders through Zeron's transcript structure (stable row ids, gpui `list()`, stick-to-bottom), still showing today's row kinds. Markdown, todos and diffs come in M5.

Effort was 3-4d before [spike S4](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md): row identity, list state and stick-to-bottom are written new on the fork's `ListState`, with Zeron's `transcript.rs` as reference; zui's tail reservation is not needed.

## Port sources

- Zeron `crates/ui/src/transcript.rs` (16,091 lines). Take only the row identity scheme (`msgId#blockId`), list state handling and stick-to-bottom (`STICK_THRESHOLD_PX` 70, `OVERDRAW_PX` 320). Leave out tool-group folding and anything that needs `zeron_doc` or `crate::attachments` (see `reports/spike-port-closure.md`). Target: `studio-ui/src/agent/transcript/{mod,rows,scroll}.rs`.
- Map Zeron's `AgentEvent` / `ToolCall` rows to fframes `RowKind` (`agent_workflow/model.rs:84`: `User`, `Agent{streaming}`, `Thought`, `Tool`, `Permission`, `Notice`, `Error`, `Validation`, `Changes`, `Outcome`).

## Tasks

1. Scout `conversation_panel.rs` (3,217 lines) and `conversation_panel/{controls,rows,provider_setup}.rs`; host and provider profiles moved to the engine in phase 1, and the qualification UI was removed in phase 7, which runs first so no dead UI is ported.
2. Create `studio-ui/src/agent/{panel,setup_tab,controls}.rs` and the transcript modules; move code over. The panel is a tab in the editor sidebar next to Presets and Scenes, laid out for the sidebar's width: transcript above, composer pinned at the bottom. Its chrome is Cap's: frame and tab strip (`TabRail`), controls (`Button`, `SegmentedControl`, `Select`, `Menu`, `Tooltip`), and the provider setup tab on the settings page frame from phase 4. Zeron's transcript supplies only row identity, list state and stick-to-bottom, adapted from zui to Cap's fork.
3. Replace the panel's list handling with the ported transcript rows; keep every row kind's current look.

## Todo

- [x] Scout panel modules
- [x] Panel modules moved
- [x] Transcript rows ported and mapped to `RowKind`
- [x] Parity agent test green

## Success criteria

Parity agent test passes; streaming text stays pinned to the bottom unless the user scrolls up more than 70 px; no file under `studio-ui/src/agent` exceeds 1,500 lines; `desktop/app/src/conversation_panel*` no longer exists.

## Progress (2026-10-15)

- The panel lives in `studio-ui/src/agent/`: `mod.rs` (types, core methods, render), `commands.rs`, `widgets.rs`, `cards.rs`, `chat_tab.rs`, `project_tab.rs`, `setup_page.rs`, `controls.rs`, `provider_setup.rs` and `transcript/{mod,rows,scroll}.rs`. The largest is `controls.rs` (1,388 lines). `studio-ui/src/panels/` and `desktop/app/src/conversation_panel*` are gone.
- Rows keep their `RowKey` identity in the fork's `ListState`. `transcript/scroll.rs` holds `OVERDRAW_PX` (320) and `STICK_THRESHOLD_PX` (70): new rows re-pin the list while the reader is within 70 px of the end, and the panel telemetry reports `scroll.following` and `scroll.distance_from_bottom`. Parity test: `new_rows_stay_pinned_unless_the_reader_scrolled_up_past_the_threshold`, mutation-checked both ways.
- The agent column is now the sidebar's **Agent** tab, next to Project, Assets and Style. It is the default tab because a new window starts with the agent. The sidebar is 416 px (Cap's), and `FocusPrompt` opens the tab. Parity: `the_sections_fill_their_boxes` checks the panel against the sidebar's inner box.
- Adapter setup is the settings frame's **Agent** page. The panel draws the page through `AgentSetupPage`, so the fields, their state and their telemetry ids stay the panel's. The panel header's settings button (`agent-open-setup`) and the `OpenAgentSetup` action open the page, and the guidance names Settings › Agent. Parity: `the_agent_page_edits_and_saves_the_adapter`, in which a Space typed in a field is saved as text.
- Verified on Windows: `studio-ui` 238 lib and 53 parity tests, workspace clippy with `-D warnings`. The Linux-only `agent_workflow_ui` and X11 tests are verified only in CI.
