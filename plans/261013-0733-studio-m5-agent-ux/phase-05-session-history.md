---
phase: 5
title: "Session history"
status: completed
priority: P2
effort: "3-4d"
dependencies: []
---

# Phase 5: Session history

## Goal

Each project keeps many past agent sessions; a list shows them, can be searched, and reopens one into the transcript, resuming it when the agent supports that.

## Context

Today: one `session-manifest.json` and one `conversation.jsonl` per project (`studio-engine/src/app_paths.rs`, `session_store` with `deny_unknown_fields` and a schema version). Restore already exists (`restore_session` in the actor; driver prefers `resume_session` over `load_session`).

## Tasks

1. Scout `session_store`, `app_paths`, the conversation log writer and `restore_session`.
2. Storage: one directory per session (`sessions/<id>/manifest.json` and `conversation.jsonl`), a schema version bump, and a one-time migration that moves the current single session into the new layout. Attachment bytes are never stored.
3. Engine: list sessions with title (first user prompt when none is stored), agent, time and outcome; load a session's rows; resume through the driver trait (ACP `resume_session` or `load_session`, Claude `--resume`, Codex `thread/resume`) via the existing `restore_session` path, falling back to read-only with a notice when the agent cannot resume.
4. UI: list and search in `studio-ui/src/agent/history/`, scoring title and prompt text (idea from Zeron's `thread_search.rs`; fframes has no branch, device or PR fields).
5. Tests: migration of a single-session project; list order; search by a prompt word; reopen shows stored rows; resume path chosen by capability.

## Reference patterns (Cap, Zeron)

From [the Zeron report](../reports/research-261013-1749-zeron-agent-control.md) and [the Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md); see the roadmap's pattern map.

1. Scout which source restore uses for rows today: the stored `conversation.jsonl` or the agent's replay. Make the stored log the single source of rows and drop `session/update` notifications replayed during `session/load`, as Zeron does, so a reopened session shows no duplicates. `resume_session` replays nothing and needs no filter.
2. Test: reopening a session through `load_session` while the fake agent replays history shows each row once.

## Todo

- [x] Stored log as the single row source; replay dropped on load, with test (`sessions::a_reopened_session_restored_by_loading_shows_each_stored_row_once_while_the_agent_replays_it`, [merge checks](./reports/merge-verification.md))
- [x] Scout
- [x] Multi-session storage and migration ([report](./reports/phase-05-storage-and-list.md), [review](./reports/phase-05-review.md)); the migration runs when a project opens
- [x] Engine list, load and resume: new chat starts a new session; resume follows the capabilities stored with the session, read-only otherwise ([lifecycle](./reports/phase-05-lifecycle.md), [review](./reports/phase-05-lifecycle-review.md))
- [x] List and search UI in the panel's History tab ([wiring](./reports/wiring-agent-panel.md))
- [x] Tests: storage, migration, list, search, load, reopen and resume by capability; parity `composer::the_history_lists_searches_and_reopens_stored_sessions`

Open, handed to M6: the startup offer skips the stored-capability gate (the read-only fallback still applies), and listing reads every manifest.

## Success criteria

After two sessions with the fake agent both appear, search finds each, and reopening shows the original rows; an existing project's session survives migration.
