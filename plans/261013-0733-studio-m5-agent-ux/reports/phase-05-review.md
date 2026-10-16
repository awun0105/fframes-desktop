# Phase 5 review: multi-session storage and history list

Reviewed from the diff and the current files (read-only; cargo and git were not run). Paths are relative to `desktop/crates/` unless noted. Line numbers refer to the current files.

Verdict: no way found to lose data on a clean run, and no path escape. There are two real ways to reach silent loss or invisibility after migration (findings 1 and 2), and Windows directory rename is fragile (finding 3). The `current` pointer is the weak point of the design.

## Findings, worst first

### 1. HIGH: a missing, corrupt or dangling `current` pointer after migration reverts every caller to the legacy paths, and a rerun then hides the new data
`studio-engine/src/agent/session_store/layout.rs:96-101`, `:114-119`, `:210-212`, `:214-227`; `studio-engine/src/agent/session_store.rs:174-178`

`current_session_id` returns `None` for a missing file, garbage, an invalid id or a deleted session directory. `manifest_path` and `current_log_path` treat `None` as "not migrated", so they point back at the legacy files, which migration deleted.

Failure scenario:
1. A project is migrated and `current` = `legacy`. The user, an antivirus, a cleanup tool or a half-restored backup removes `sessions/current`, or the id in it names a deleted session directory.
2. `SessionStore::load` returns `None` (the session is not resumable). `current_log_path` returns `<project>/conversation.jsonl`, which does not exist, so `RowStore::open` creates an empty log. The user sees an empty conversation. The real one sits in `sessions/legacy/`. Every `save` writes `agent/session-manifest.json` again.
3. The next `migrate_legacy` run sees a legacy log and manifest again. `sessions/legacy` already exists, so it is not re-imported. The new log differs from the copy, so `settle` puts it in `kept_legacy` forever. `legacy_present && current is None` then calls `set_current(legacy)`, which switches the app back to the old session. The rows and manifest written in step 2 are on disk but no code path ever shows them. For the user, that work is lost.

Fix:
- Separate "never migrated" from "pointer invalid". Treat the layout as migrated when `sessions/` contains any valid session directory.
- When the pointer is invalid, repair it: choose the most recently updated valid session, or the `legacy` session, and write the pointer. Do this in `current_session_id`'s caller, in `migrate_legacy`, or in an explicit `ensure_current` step. Never fall back to the legacy file paths once `sessions/` exists.
- Add tests for pointer deleted, pointer garbage, and pointer naming a removed directory, each followed by `load`, `save`, `current_log_path` and `migrate_legacy`.

### 2. HIGH: a legacy file that differs from its migrated copy is never reconciled
`layout.rs:203-212`, `:214-227`, `:371-377`

Migration only imports an item when `sessions/<id>` does not exist. After that it can only delete an identical legacy file or report it as kept. Cases that end with data stuck in a legacy file that nothing reads:
- The finding-1 scenario above.
- A crash after publish and before cleanup, followed by another run of the same or an older build that appends to the legacy log before the rerun. The session copy is frozen at the earlier state, `current` points at it, and the extra rows stay in the legacy file (`kept_legacy`).
- On non-Windows, `migrate_legacy` run while an actor still has the log open. The copy is torn or short, it is published as complete, and the actor keeps appending to the legacy file.

Fix: when the legacy file exists, `sessions/<id>` exists and the bytes differ, import the legacy file as a new session (for example `legacy-<unix>`) instead of keeping it forever. Alternatively treat a legacy log that is a byte-prefix-extension of the copy as "newer" and re-import it. Surface `kept_legacy` to the caller as a warning that is shown to the user. Do not leave it as a silent report field.

### 3. MEDIUM: publishing by directory rename fails routinely on Windows, aborts the whole migration, and is not retried
`layout.rs:240-262`, `:263-277`

On Windows, `std::fs::rename` of a directory fails with "Access is denied" if any file inside it is open without delete sharing. That includes a scanner (Defender, Search indexer) opening the freshly written `conversation.jsonl`, `manifest.json` and `meta.json` in staging. This is the most common cause of directory-rename failures on that platform. `publish_dir` returns the error, `migrate_legacy` propagates it with `?`, and the items after it are not attempted. Nothing is lost, because the staging directory is deleted and legacy files are untouched. But history never appears, and the user hits the same failure every start while the scanner is quick. `remove_dir_all(staging)` has the same exposure.

Fix:
- Retry the rename a few times with a short back-off on `PermissionDenied`.
- Or avoid the rename: write into the final directory and make `meta.json` (or a `.complete` file) the last write and the "complete" marker. Treat a directory without the marker as an incomplete leftover to be cleared.
- Do not let one failed item stop the others. Collect errors into the report.

### 4. MEDIUM: wiring steps 1-2 alone make "New chat" hide the archived chat inside the session directory
`studio-engine/src/agent/actor/session.rs:743-770` (existing), `agent/log.rs:426-479` (existing), `layout.rs:11-23` (docs)

`new_chat` calls `SessionStore::clear` and then `RowStore::start_new`. `start_new` archives with `self.path.with_file_name("conversation-<stamp>.jsonl")`. If the actor opens `current_log_path` (wiring step 2) before `start_new` is replaced (step 4), the archive is created inside `sessions/<id>/`. That file is not a session and is not listed. The session `<id>` is left with an empty log and no manifest, because `clear` deleted the current session's manifest. The chat has vanished from the list, though not from disk. The module docs call the three steps "in order" but nothing enforces them as one change.

Fix: land steps 2 and 4 together. Or make `start_new` refuse (return an error) when the path is under `sessions/`.

### 5. MEDIUM: no directory fsync before legacy files are deleted (Unix) or after `current` is written
`layout.rs:240-262`, `:214-227`; `session_store.rs:223-256` (`write_atomic`)

File contents are synced, but the new directory entry (`sessions/<id>` after the rename) and the `current` rename are not. On a power loss, ext4 and APFS can persist the later `remove_file` of the legacy log without the earlier directory rename. The session is lost in that case. Not applicable to Windows, which is the user's host, but this is a cross-platform product.

Fix: after `rename` in `publish_dir`, `File::open(sessions)?.sync_all()` under `#[cfg(unix)]`, as `compact` already does. Do the same after the `current` write. Do it before any `settle` deletion.

### 6. MEDIUM: `list_sessions` cost, and the title and search text shift after compaction
`history.rs:61-84`, `:115-135`; `log.rs:83-92`

- Cost: every call fully reads and JSON-decodes every session log (up to 16 MiB each, held in a `BTreeMap` of all rows), sequentially and synchronously. With 100 sessions that is up to about 1.6 GB of reads and a large allocation spike each time the panel opens or refreshes. If this runs on the UI thread it freezes it. The author's note accepts "dozens"; there is no cap or cache.
- Title and `created_unix`: for sessions without a stored title (every migrated session, plus any session started through `create_session(None)`), the title is the first surviving user prompt. Compaction drops the oldest rows, so the title changes later, and a long conversation ends up titled by a late follow-up. `created_unix` falls back to the first surviving row for archives with no manifest.
- `prompt_text` is truncated to the first 8 KiB (`truncate_utf8` on the joined text). In a long session the later prompts are not searchable.
- `summarize` swallows a read error (`unwrap_or_default`), so a log that is briefly unreadable (compaction rename, antivirus lock) turns into "Untitled session", 0 rows, and `updated_unix` 0, sorting to the bottom for that refresh.

Fix: run it off the UI thread. Cache a summary per session keyed by (file length, mtime). Write the derived title into `meta.json` the first time it is computed, so it is stable. Keep the tail of the prompt text, or index per-prompt. Report a per-session read error in the summary instead of defaulting.

### 7. MEDIUM (verify before wiring): row ids restart at 1 in every new session
`log.rs:230-234` and `:424-426` (`start_new` docs: "no id of the old conversation is reused"), `layout.rs:20-22`; consumers in `studio-ui/src/agent/transcript/rows.rs`, `transcript/mod.rs:70`, `actor/rows.rs`, `actor/finish.rs`

The present design deliberately keeps ids unique across "New chat". `create_session` + `set_current` gives each new session an empty log, so `RowStore::open` gives `next = 1`. Any state keyed by `RowId` outside the log is then ambiguous across sessions: the transcript view's row and scroll caches, `tool_rows`/`permission_rows` (cleared in `new_chat`, so safe if the actor is rebuilt), an `Outcome` card's undo target, and `Row.task` references. I did not find a read of `transcript_boundary` in the engine (it is only written, at `session.rs:556` and `finish.rs:246`), so the manifest boundary is safe when it is kept per session. The risk is in the UI and any cross-session cache.

Fix: simplest is a floor. Add `RowStore::open_with_floor(path, limits, min_next)` and pass `max(previous session's next)` (or `now_unix` millis) when creating a session. Or require a full transcript and actor reset on a session switch, with a test that opens A, switches to B and checks no stale row from A is reused.

### 8. MEDIUM-LOW: `SessionStore::load` behaviour changes for a corrupt manifest after migration
`session_store.rs:192-209`, `layout.rs:210-212`, `:350-354`, `:311-330`

Before migration, `load` returns `Err("cannot parse session manifest")` for a corrupt manifest, which the actor can show. Migration deliberately leaves an unparsable (or newer-schema) legacy manifest at the old path and creates the session without a manifest. After that, `load` reads `sessions/<id>/manifest.json`, which is absent, and returns `Ok(None)`. The corrupt file is ignored without a message and is never cleaned up, and `kept_legacy` is not shown to the user. A newer-schema legacy manifest is the same, with the downgrade case being the likely one.

Fix: surface `kept_legacy` as a notice, or copy the unreadable manifest into the session directory as `manifest.json.unreadable` so it is neither lost nor orphaned. Add a test that compares `load` before and after migration for a corrupt manifest.

### 9. LOW: concurrency and other edges
- `layout.rs:263-277`: `remove_staging` runs at the start of every `migrate_legacy`, so it deletes another migrate's staging directory. Two overlapping calls for one project (two windows, or a retry racing a slow call) fail with an error. `rename` onto an existing non-empty directory fails, so data is not corrupted. The report's "not guarded" limitation is correct. A per-project lock file would fix it.
- `layout.rs:283-308`: an archive id is `archive-<stamp>` and the stamp is taken from the file name. A file name that makes the id longer than 64 characters creates a directory that `is_valid_session_id` then rejects, so the copy is invisible and the legacy file is deleted. Only crafted names do this (`start_new` stamps are short), but check the length before migrating.
- `session_store.rs:52`: `is_resumable` accepting `1..=2` is correct for reading. Downgrade note: a build that predates this change writes and reads version 1 only. Once this build saves a v2 manifest in the legacy path (pre-migration saves do that), the older build treats the file as newer and returns `None`, discarding the resumable session without a message. Accepted if downgrades are unsupported; say so.
- `history.rs:89-110`: `load_session` on the current session reads only flushed rows. The actor's unflushed `dirty` rows are missing, so the list and the transcript would disagree for the live session. Use it for non-current sessions only.
- `layout.rs:365` (`is_valid_session_id`) allows Windows device names (`con`, `nul`, `aux`) and is case-sensitive while NTFS is not (`Legacy` matches `legacy` on disk but `is_current` compares exactly). Both are harmless in practice: `is_dir` is false for device names.
- `AppPaths::agent_session_dir` (`app_paths.rs`) is a public function that joins an unvalidated `&str`. All current callers validate first, but the raw join is one new caller away from `..`. Validate inside it, or return a typed `SessionId`.

### 10. LOW: UI search and list behaviour
- `studio-ui/src/agent/history/mod.rs:228`: the click handler captures the positional index `ix` and resolves it against `self.shown` at click time. If `set_entries` or the query changes between paint and click (a refresh reorders the list), a different session is opened. Capture the session id in the closure instead.
- `studio-ui/src/agent/history/mod.rs:136`: `Confirmed` calls `open(0)`. In an empty search box, Return opens the newest session, which may not be the one the user wants and may be the current one. Either do nothing on an empty query or open only when a query is present.
- `search.rs:40-55`, `:57-70`: scoring is correct. Every term must match a field, the case-fold uses the same string on both sides so byte offsets are safe, and ordering is deterministic. It lowercases and allocates the title, agent and up to 8 KiB of prompt for every entry on every keystroke, which is fine for hundreds of sessions. The outcome label is not searchable ("failed" finds nothing unless the title says so).
- `mod.rs:192` `refilter` keeps the query across `set_entries`, as tested.

## Checked and found sound

- Path safety: every id that comes from the UI, the pointer file or a directory name goes through `is_valid_session_id` (letters, digits and `-`, 1 to 64 characters) in `current_session_id`, `set_current`, `session_log_path`, `load_session` and `list_sessions`. `..`, separators and absolute paths cannot pass. The test `loading_and_listing_never_create_anything` covers `../x`, `""` and `a/b`.
- Crash at each migration step: staging is deleted on the next run and a published directory is complete (log fsynced, then manifest and meta via `write_atomic`). `current` is written after publish. Legacy files are deleted last and only on a byte-identical copy (manifest identical after the v1 to v2 rewrite), so no step I traced loses a legacy file. A rerun skips published directories and finishes pointer and cleanup. Windows `rename` of a directory onto an existing target is guarded by the `exists()` check. A concurrent run is the only unguarded case (finding 9).
- Before migration, `save`, `load`, `clear` and `current_log_path` use the legacy paths unchanged (`the_single_session_api_is_untouched_before_migration`). A missing pointer is only a problem after migration (finding 1).
- Reading: `read_rows` is read-only, skips torn and unparsable lines, keeps the latest version per id, and cannot corrupt a log another process owns. It reads through `File::open`, which on Windows allows a concurrent rename-over by the compactor.
- Version bump: no other code compares `schema_version` against the constant (searched `desktop/`); `provider_handoff.rs` uses the constant in struct literals. `load` still rejects `> 2` and `read_manifest` agrees.

## Test gaps to close

Pointer missing, garbage or dangling after migration (finding 1); a legacy file that differs from an existing copy being re-imported (finding 2); a failing rename retried (finding 3); `load` before and after migration with a corrupt manifest (finding 8); a session switch that reuses row ids (finding 7); list ordering with an unreadable log.

Status: DONE_WITH_CONCERNS
Summary: The migration is crash-safe for the clean paths I traced, and session ids cannot escape `sessions/`. The `current` pointer is the weak point: if it goes missing, the app falls back to legacy paths and a rerun hides the newer data. Directory rename on Windows, row-id restart and `list_sessions` cost also need fixes or confirmation before the wiring lands.
Concerns/Blockers: No code was run (cargo and git were unavailable), so the Windows rename behaviour in finding 3 and the UI row-id use in finding 7 are from reading the code, not from a reproduction.
