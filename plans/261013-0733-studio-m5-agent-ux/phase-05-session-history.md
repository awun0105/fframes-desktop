---
phase: 5
title: "Session history"
status: pending
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

- [ ] Stored log as the single row source; replay dropped on load, with test

- [ ] Scout
- [ ] Multi-session storage and migration
- [ ] Engine list, load, resume via existing path
- [ ] List and search UI
- [ ] Tests

## Success criteria

After two sessions with the fake agent both appear, search finds each, and reopening shows the original rows; an existing project's session survives migration.
