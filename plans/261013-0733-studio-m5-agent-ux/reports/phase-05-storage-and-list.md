# Phase 5 (part): session storage, listing and history list

Scope: the parts of phase 5 that do not depend on the driver rewrite. Restore, resume and dropping replayed `session/update` on `session/load` are not done; this report lists the seams and call sites.

## Layout

```text
<data>/projects/<project>/agent/
  session-manifest.json            legacy single manifest (served until migration)
  sessions/
    current                        id the single-session API serves (absent = legacy layout)
    <id>/manifest.json             SessionManifest, schema version 2 (only once a wire session exists)
    <id>/meta.json                 SessionMeta { schema_version, title?, created_unix }
    <id>/conversation.jsonl        row log, same line format as before
<data>/projects/<project>/conversation.jsonl            legacy live log (unchanged until the wiring)
<data>/projects/<project>/conversation-<stamp>.jsonl    archives left by RowStore::start_new
```

Ids: letters, digits and `-`, at most 64 (validated everywhere an id comes from outside). Migrated ids: `legacy` for the single session, `archive-<stamp>` for each `start_new` archive. New ids: `s-<unix hex>-<counter>-<pid>`. Attachment bytes are never stored: a `User` row holds text only.

## Schema change

- `SESSION_MANIFEST_SCHEMA_VERSION` 1 -> 2. `SessionManifest` has the same fields (no field added: the actor and `app/tests/provider_handoff.rs` build it with full struct literals). A v1 file still parses; migration rewrites it as v2.
- `is_resumable` now accepts `1..=2` so an unmigrated v1 manifest is still offered for restore (behaviour unchanged).
- Title and creation time live in the `meta.json` sidecar. Outcome, agent, times and prompt text are derived at list time (outcome from the newest `Outcome` row, agent from `provider_id`).

## Migration (`SessionStore::migrate_legacy`)

Explicit, never implicit: the live log cannot move while the actor still opens `present::log_path`, so `load/save/clear` never touch the log. They follow `current` once it exists and use the legacy file before.

1. Delete leftover `sessions/.staging-*` directories.
2. For each legacy item (`legacy` = manifest + log, `archive-*` = log) whose `sessions/<id>` does not exist: copy the log (fsync) and write the v2 manifest and `meta.json` into `sessions/.staging-<id>/`, then rename the directory to `sessions/<id>` (so a session directory is absent or complete).
3. Write `current` (atomic temp + rename) = `legacy` if the legacy session exists and no valid pointer is set.
4. Remove each legacy file only if the new copy is byte-identical (manifest: identical after the v1 -> v2 rewrite). Otherwise keep it and report it in `kept_legacy`. An unparsable or newer-schema manifest is never migrated and stays in place; its log still migrates.

Idempotent: a rerun skips existing session directories and only finishes steps 3 and 4. Failure never removes a legacy file whose copy is not verified.

## API

`SessionStore` (all in `studio-engine/src/agent/session_store/`):

- `layout.rs`: `migrate_legacy(paths, project) -> MigrationReport`, `current_session_id`, `set_current`, `current_log_path`, `session_log_path`, `create_session(_at)`, `is_valid_session_id`, `SessionMeta`.
- `history.rs`: `list_sessions -> Vec<SessionSummary>` (id, title, agent, created/updated unix, outcome, prompt_text bounded to 8 KiB, rows, is_current, resumable), newest first; `load_session(paths, project, id, limit) -> StoredSession { manifest, rows, has_older }` (newest `limit` rows, clamped to the resident window, latest version per id).
- `log::read_rows(path)`: read-only replay (missing file = empty, torn lines skipped); list and load never create directories or repair logs.
- `AppPaths`: `agent_sessions`, `agent_session_dir`, `agent_session_current`, `legacy_conversation_log`.
- Title: stored title, else the first non-empty line of the first brief or clarification (automatic repair prompts ignored), cut to 80 characters, else "Untitled session".
- Declared by `pub mod history; pub mod layout;` inside `session_store.rs`; `agent.rs` is untouched.

UI (`studio-ui/src/agent/history/`, declared by one line `pub mod history;` in `agent/mod.rs`): `HistoryList` view over `SessionEntry` (plain struct, `From<&SessionSummary>`), search field (kit `TextInput::plain`), "New session" button, rows with agent, relative time, outcome chip and a "Current" chip. Events: `HistoryEvent::Open(String)`, `HistoryEvent::NewSession`. `set_entries(entries, now_unix, cx)`, `set_query`, `shown()`, `open(ix)`. Return opens the best match, Escape clears the query. `search.rs` is the scoring (every word must match; title 100, agent 40, prompt 30, x1.5 at a word start; ties by recency; empty query keeps the given order, unlike Zeron's `None`). Not wired into the panel. The type is named `SessionEntry` because `studio_engine::agent::HistoryEntry` (edit history) is already used by the panel.

## Call sites the restore wiring must change

Order matters: migrate while no actor owns the log.

1. `studio-engine/src/agent/present.rs:500` `log_path` -> `SessionStore::current_log_path`.
2. `studio-engine/src/agent.rs:340` (`RowStore::open(present::log_path(..))` in `AgentWorkflow::open`): call `SessionStore::migrate_legacy` before it (off the UI thread). If an actor already has the log open it may recreate `<project>/conversation.jsonl` after the migration removed it; migration would then keep that file as a conflict and rows diverge, so do not rely on an error to catch it.
3. `app/tests/agent_workflow/regressions.rs:242, 352, 870` read `<project>/conversation.jsonl` directly; they move with step 1.
4. `RowStore::start_new` (`log.rs`, used by "New chat") archives to `conversation-<stamp>.jsonl`; replace by `create_session` + `set_current` while the log is closed, so each chat becomes a session instead of an archive. Row ids are monotonic per project (`model.rs`) and `start_new` keeps the counter going; a fresh `RowStore::open` on a new session's empty log restarts at id 1. The wiring must seed the new session's id floor from the previous session, or treat ids as per session and key by (session, id) (including `transcript_boundary`).
5. `actor/session.rs` `restore_session` (about line 612), `dismiss_restoration` (about 707), the save at about 545, `actor/mod.rs:489`, `actor/finish.rs:233-248`: these already call `SessionStore::load/save/clear` and need no change for storage; restore of a chosen session should call `SessionStore::load_session` for rows plus manifest, and drop `session/update` replayed during `session/load`.
6. `agent_session_manifest()` stays only for the pre-migration case.

## Tests

- `studio-engine/tests/session_history.rs` (14): legacy API untouched before migration; full migration with rows and manifest (v1 -> v2, resumable, archive session, saves land in the session); rerun is a no-op; nothing-to-migrate; interrupted staging directory; interrupted after publish (pointer missing, legacy files back) finished by the rerun; a changed legacy file is never removed; unreadable manifest stays; v1 manifest resumable; list order, title fallback, agent, outcome, time; long title and repair prompts; `create_session` does not move current; load returns newest rows with `has_older` and the latest version of an updated row; list/load create nothing and reject bad ids.
- `studio-ui` lib (`agent::history`): scoring (every word, title above body, word-start bonus, tie order, empty query), list rows, empty state, row click and button events, typing filters and ranks a title above a prompt match, a prompt word finds its session, Return opens the best match, Escape clears, new entries keep the query, summary conversion, time and outcome labels.

## Limitations

- `list_sessions` reads every session log fully (bounded by the 16 MiB compaction limit per file) to get prompts, outcome and times; fine for dozens of sessions, a cached index would be needed for many large ones. The actor does not update `meta.json`, so nothing is cached.
- Sessions that never opened a wire session have no manifest: agent is `None`, not resumable.
- Before migration `list_sessions` does not show the legacy session (migrate first).
- A project that has only `archive-*` sessions keeps no `current` pointer (the API stays on the legacy path until a session is made current).
- Windows directory rename needs the target absent (checked first); a concurrent second process migrating the same project is not guarded.
- Resume, read-only fallback with a notice, replay filtering and the driver-capability choice are out of scope.

- Nothing in production calls `migrate_legacy` yet.
- Also verified: `cargo test -p fframes-studio --test provider_handoff` (21 passed), the heaviest existing caller of `SessionStore`.

## Review fixes

Every finding of `phase-05-review.md` is addressed in `studio-engine` (`session_store/layout.rs`, `session_store/history.rs`, `session_store.rs`, `app_paths.rs`, `log.rs`) and `studio-ui/src/agent/history/`.

1. Pointer: a project is migrated once `sessions/` holds a session directory. `current_session_id` repairs a missing, garbage or dangling `current` (most recently updated session, ties greatest id; rewritten, reported on stderr), so `load`, `save`, `clear` and `current_log_path` never fall back to legacy paths afterwards. `migrate_legacy` uses a non-repairing `pointer_target`; with legacy files present it points `current` at `legacy`, otherwise it repairs. A session created in a project without a pointer therefore becomes current (documented).
2. A legacy file that differs from its migrated copy is imported as a new session `legacy-<unix-seconds>` and then removed (`MigrationReport::imported`); `kept_legacy` now only means a removal failed.
3. `retry_denied` (6 attempts, 25 ms doubling, `PermissionDenied` only) wraps the staging rename and every removal. A failing item lands in `MigrationReport::failed` and does not stop the others. `remove_staging` handles files.
4. `RowStore::start_new` refuses a log whose directory sits in `sessions/`. `clear` removes only the current session's `manifest.json` (tested).
5. Unix only: `sessions/` is fsynced after publishing a session and after writing `current`, before any legacy file is deleted. No-op on Windows.
6. `SessionSummary::unreadable` and the title "Unreadable session"; `SessionEntry::unreadable` with a Danger "Unreadable" badge. `list_sessions` documents that it reads every log and belongs off the UI thread.
7. Row ids: `RowStore::open_with_floor`, `SessionMeta::first_row_id` (default 1), `SessionStore::row_id_floor`; `create_session_at` seeds the new session after the current one's rows. Flush the previous log first, only flushed rows count.
8. A corrupt or newer-schema legacy manifest is copied byte for byte, so `load` answers the same before and after migration.
9. Archive ids longer than 64 characters become `archive-<30 chars>-<16 hex FNV-1a>`. `AppPaths::agent_session_dir` validates ids (`is_valid_session_id` moved to `app_paths.rs`, re-exported from `layout`) and maps invalid ones to `sessions/.invalid`.
10. UI: rows open by session id (`open_id`, unknown ids ignored); Enter with a blank query does nothing; the outcome label is searched (weight 20).

Tests added: pointer missing/garbage/dangling each followed by load, save, current_log_path and migrate; differing legacy re-import; corrupt and newer manifest before/after; isolation of an item whose copy fails; retry helper (success after denials, bounded give-up, other errors not retried); long archive stamps; invalid ids; `start_new` refusal; `clear` scope; row id chain A -> B -> C; list order with an unreadable log; UI Enter on blank query, outcome search, `open_id`.

Accepted limitations: `list_sessions` reads every log with no caching beyond the unreadable marker; concurrent `migrate_legacy` calls are not guarded; `load_session` sees only flushed rows, so use it for non-current sessions; manifests are schema version 2 after migration, an older build cannot read them (downgrade is unsupported); Windows reserved device names and case-insensitive ids are harmless because ids are generated or `archive-`/`legacy-` prefixed. Nothing in production calls `migrate_legacy` yet.
Directory flushes (Unix) are best effort: a failure is reported on stderr and never fails a publish that already succeeded. If removing a legacy file fails and the session is then updated in place, a rerun imports the stale legacy file as an extra legacy-<unix> session; no data is lost.

Also left as is (finding 6, beyond the requested fixes): a session without a stored title is titled by its first surviving prompt, so compaction can change that title; the searchable prompt text keeps the first 8 KiB only. Both are cosmetic and are for the wiring to settle by writing a title into meta.json.
