# Phase 5: session lifecycle wiring

Date: 2026-10-16. Branch `feat/studio-m5-sessions`, nothing committed.

## AgentWorkflow API (studio-engine/src/agent.rs)

```rust
pub fn sessions(&self) -> Result<Vec<session_store::history::SessionSummary>, WorkflowError>;
pub fn open_session(&self, id: &str) -> Result<(), WorkflowError>;
pub fn new_session(&self) -> Result<(), WorkflowError>;
pub fn new_chat(&self) -> Result<(), WorkflowError>; // unchanged signature; now creates a session
```

- `sessions()` lists newest first (ties: id descending); reads every stored log, so call it off the UI thread. The running session's entry can lag one publish.
- `new_session()` and `open_session(id)` return `WorkflowError::Busy(reason)` while a task, queued brief or undo is active (`Registry::busy()`). `open_session` returns `WorkflowError::History("unknown session")` for an invalid or missing id and `Ok(())` as a no-op for the current session. The actor re-checks (`switch_blocker`, also covers conflict/switch-pending states) and shows a warning row if the state changed.
- `new_chat()` keeps its old behaviour of showing a refusal row instead of an error.

## Open and migration order

1. `AgentWorkflow::open` (already off the UI thread) calls `SessionStore::prepare(paths, project)` (new, `session_store/startup.rs`):
   `migrate_legacy` -> log `imported` / `kept_legacy` / `failed` on stderr -> `failed` or `Err` becomes a warning notice row (`Actor::new` receives `startup.notices`, deduped by text) -> create and select a first session when none exists and no legacy file remains -> log path from `current_log_path`, row-id floor from the session meta.
2. If migration failed and legacy files remain, the legacy log is still served (nothing is hidden behind an empty new session); the next open retries.
3. `RowStore::open_with_floor(log, limits, floor)`; the actor then offers the saved session as before.

## New chat and switching (actor/session.rs)

- New chat: flush old log (refuse on failure) -> `create_session` (seeds `first_row_id` from the current session) -> open new log with floor = max(session floor, old `sequence_boundary()`) -> `set_current` -> swap. An empty session dir is removed if the swap fails. The old session keeps its manifest, so it stays resumable. `RowStore::start_new` is no longer used by the workflow.
- Open: `load_session(..,1)` validates the id and returns the manifest -> flush -> open the stored log with the floor -> `set_current` -> `replace_conversation` (drops stream, tool and permission rows, task, restore offer, `chat += 1`) -> `resume_or_read_only(manifest)`.
- The stored log is the only row source. The log path comes from `SessionStore::current_log_path` (via `prepare`); `present::log_path` was removed, and `session_log_path` is used when opening a chosen session.

## Resume rules

Decision in `resume_or_read_only`, using `self.capabilities` else `manifest.capability_snapshot`; unknown capabilities mean attempt.

- Resume path: manifest resumable, provider equals the configured one, and not (load_session=false and resume_session=false) -> `offer_saved_session()` + existing `restore_session()` (project, draft, revision checks; sets `pending_resume_session`). The agent is launched lazily on the next prompt, and the driver picks the call.
- Read-only fallback (no manifest, provider mismatch, not resumable, or known caps without load/resume): the stored rows stay visible, a non-resumable `SessionRestoreView` and a notice row say so, and the next prompt starts a fresh agent session appended to the same stored session.
- Drivers: ACP prefers `resume_session` over `load_session` (`Plan::Resume` / `Plan::Load` in studio-harness `connection.rs`, untouched); the scripted agent test uses `loadSession`. Claude uses `--resume` (caps resume_session=true, load_session=false). Codex uses `thread/resume` (resume_session=true). A driver failure (`RestoreUnsupported` or load error) falls back to a fresh session through the existing `on_provider_failure` path.

## Files touched

- `crates/studio-engine/src/agent.rs` (open, three methods, busy consts)
- `crates/studio-engine/src/agent/actor/mod.rs` (`Msg::OpenSession`, `Actor::new` startup warnings)
- `crates/studio-engine/src/agent/actor/session.rs` (switching, resume, helpers)
- `crates/studio-engine/src/agent/log.rs` (`RowStore::limits`)
- `crates/studio-engine/src/agent/present.rs` (removed `log_path`)
- `crates/studio-engine/src/agent/session_store.rs`, `session_store/startup.rs` (new), `session_store/layout.rs` (zero-padded id counter so same-second sessions sort newest first; docs)
- Tests: `app/tests/agent_workflow/{sessions.rs (new), main.rs, stop.rs, regressions.rs}`, `crates/studio-engine/tests/session_history.rs`

## Tests

- Engine (`session_history.rs`): `prepare` migrates a legacy project and serves its log; creates the first session once; a failed item becomes one notice while the rest opens; an unwritable sessions dir keeps the legacy log and a notice.
- App (`agent_workflow/sessions.rs`): legacy single-session project migrates on first open and rows still show (no warning); two sessions list newest first and opening the older shows original rows; new chat row ids continue; open/new refused mid-turn with `Busy`; resume with load capability calls the scripted agent's session load with the stored wire id on the next prompt; without capability the rows are read-only with a notice, no load, and the fresh prompt joins the same stored session.
- `stop.rs` new-chat test now asserts two stored sessions instead of an archived file.

## Limitations

- `sessions()` reads every log; fine for tens of sessions, no pagination or cache.
- Migration notices are deduped by text within the resident window only.
- Resume is lazy and refused by the existing revision/draft checks if the project changed since that session; the capability decision uses the capabilities stored with the opened session (see "Review fixes").
- Concurrent migration from two processes is unguarded.
- Pre-existing mojibake in `session_store.rs` `redacted_id` left alone.

## Review fixes

Each item answers a ranked finding of `phase-05-lifecycle-review.md`. Sections above describe the first delivery; where they differ, this section is current (open and new chat no longer read the log, opening writes no rows, resume gating uses the manifest).

1. Partial migration: `SessionStore::current_session_id` never repairs the pointer while legacy files are unmigrated (`legacy_files_unmigrated`); the legacy log keeps being served. Test: `a_legacy_conversation_that_is_not_migrated_is_never_hidden_behind_another_session`.
2. `replace_conversation` clears `next_handoff_context`, `next_draft_choice`, `resume_attempt` and `resume_failed`, together with the other per-conversation state. Tests: `the_handoff_of_a_provider_switch_never_reaches_another_conversation` and, as control, `..._goes_with_the_next_prompt_of_the_same_conversation` (`provider_handoff.rs`).
3. Resume failure: a session-phase or `RestoreUnsupported` failure while a resume was attempted (`resume_attempt`, set in `spawn_writer`, cleared when the session is ready) shows the "restoration failed" view plus a notice row and clears the queued resume; the stored manifest is left untouched (`finalize` skips its rewrite through `resume_failed`), so the next prompt starts a fresh agent session in the same stored session and a transient failure does not end resumption. Unsafe marking stays for revision, draft and digest mismatches and for non-resume session failures. The capability gate (`resume_blocker`) uses the manifest's own provider and `capability_snapshot`; unknown capabilities mean attempt. Test: `unsupported_expired_resume_failure_falls_back_to_fresh_session` now also checks the manifest stays `Ready` and the notice row.
4. New chat with no user row (and no older rows) in the conversation is a silent no-op. Test: `a_new_chat_in_an_empty_conversation_does_nothing`.
5. Opening a session adds no rows and touches none of its files: the outcome is shown only through `session_restore.notice`. A row is written only when something happens afterwards (the resume that follows the next prompt, a failed resume). Test: `opening_a_session_leaves_its_stored_files_and_recency_alone` (size and modification time of every file in the session directory, list order).
6. `open_session` reads the manifest (`SessionStore::session_manifest`) and the opened log once (`RowStore::open`); `begin_session` reads no log at all. The floors come from the in-memory `sequence_boundary()` and `project_row_floor` (meta files only); `create_session_from` replaces the log scan. The log read still runs on the actor thread, as the resident window is built there.
7. The caller-side "already current" shortcut in `AgentWorkflow::open_session` is gone; the actor decides (silent no-op).
8. A legacy file that cannot be deleted is recorded: each migrated session's meta stores `source_hash` (hash and length of the legacy files); `settle_item` removes a legacy file whose hash is already imported instead of importing it again. Test: `a_legacy_file_that_cannot_be_removed_is_not_imported_again`.
9. Row ids are unique per project across restarts: `SessionMeta::last_row_id` is recorded when a conversation is left (`leave_conversation`) and when a legacy item is staged; `project_row_floor` takes the maximum over every meta and feeds `prepare`, `begin_session` and `open_session`. The record is written only when the last row id grew, so a visit that added no rows leaves the session's files alone; a visit that added rows touches the meta, and those rows already moved the session's recency. Tests: `row_ids_stay_unique_across_sessions_and_restarts` (app: ids after a restart in the older session stay above the other session's) and `row_ids_stay_unique_when_a_session_that_used_higher_ids_is_not_current_after_a_restart` (engine).
10. History pages carry the conversation (`Msg::History { chat, .. }`, stamped from the latest published snapshot); the actor answers a page asked for another conversation with `WorkflowError::History("the conversation changed")`. A view that lags the snapshot still needs its own comparison: studio-ui already drops a page whose `chat` differs from its own (`agent/mod.rs`). Test: `a_history_page_asked_for_an_earlier_conversation_is_refused` (the actor is held on the controller lock so the page is stamped before the new chat is handled).
11. `RestoreSession` uses the same `resume_blocker`. Test: `a_stored_session_that_cannot_resume_refuses_a_restore_request`.
12. A fresh agent session that has no storable wire id (or no verifiable launch identity) clears `wire_session_id` of the stored manifest (`forget_stored_wire_id`), so the old id is never offered again. No test: the scripted agent always returns an id.
13. Busy reasons name the state (candidate under review, Undo, queued briefs, running task), chosen from the snapshot. Switching refusal tests: `switching_sessions_is_refused_while_a_candidate_awaits_review_and_while_it_is_applied`, `..._while_the_compile_of_a_task_runs` (validate), and `switching_sessions_is_refused_while_the_remaining_files_validate` (reject, `agent_workflow_review.rs`).

Seam edits outside `actor/session.rs`: `actor/mod.rs` (two fields, `Msg::History.chat`), `turn.rs` (`spawn_writer` sets `resume_attempt`), `finish.rs` (`finalize` honours `resume_failed`). Existing resume tests now write `load-session` before the seed task so the stored capability snapshot allows resuming.

Remaining limitations:

- An unresolved recovery conflict blocks switching (the actor refuses with a warning), and a log that cannot be flushed cannot be left (refusal with a warning); both are deliberate.
- `sessions()` still reads every log. Resume stays lazy. A stored capability snapshot older than the installed agent can refuse a resume the agent would now accept (the user then starts fresh).
- The offer made when the project opens (`offer_saved_session`) does not apply the stored-capability gate, so it can show a session as resumable that Restore or an open then refuses with a notice.
- A new chat started while the legacy log is still served (migration failing every time) moves the pointer to the new session; if the migration keeps failing, the legacy conversation stays unmigrated and hidden after a restart.
- A resume failure that arrives after the session was ready (a later turn) is an ordinary task failure and does not touch the manifest rules above.

## Verification (all run on the VM, Windows)

- `cargo fmt --all --check`: clean
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo test -p studio-engine`: all suites pass (session_history 31 passed)
- `cargo test -p fframes-studio`: all pass (agent_workflow 79 passed, 2 ignored; provider_handoff 23; agent_workflow_review 7). Tests added after the full run were re-run on their own suites (session_history, agent_workflow, provider_handoff, agent_workflow_review), then fmt and clippy again.
- `cargo test -p studio-ui --lib`: 466 passed
- `cargo test -p studio-ui --test parity agent::`: 10 passed (filtered)

Status: DONE_WITH_CONCERNS
Summary: Migration, session-backed log path, new chat, `sessions()/open_session()/new_session()` and capability-chosen resume with read-only fallback are wired and tested; all acceptance commands pass.
Concerns/Blockers: Limitations above (listing reads every log, lazy resume refused by revision checks, partial-migration edge case); no panel UI by design.
