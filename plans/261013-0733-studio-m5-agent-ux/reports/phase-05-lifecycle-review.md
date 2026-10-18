# Phase 5 lifecycle review

Scope: session wiring in `desktop/crates/studio-engine/src/agent.rs`, `agent/actor/{mod,session}.rs`, `agent/session_store/{startup,layout,history}.rs`, `agent/log.rs`. Read-only review; nothing was compiled or run (Bash was unavailable).

Verdict: the core swap is sound. Flush, open, `set_current` and replace are ordered so a failure at any step leaves the old store and the old pointer in force. There is no actor-concurrent writer, because the actor is the only writer and is single-threaded. The Busy checks are atomic enough, because the actor re-checks authoritatively. The real defects are around migration partial failure, cross-session state that survives the swap, and the resume/failure path.

## Ranked findings

### 1. High: a partly failed migration makes the live conversation write into an unrelated archived session
`session_store/startup.rs:67-89`, `session_store/layout.rs:361-367`, `session_store.rs:175-191`.

`migrate_legacy` migrates each item independently. If the `legacy` item fails (log locked or copy error, disk full) but an `archive-*` item succeeds, no valid pointer exists, so `current_session_id` repairs the pointer to the most recently modified session, an archive.

`prepare` only falls back to the legacy log when no session directory exists (`current_session_id(..).is_none()`). Here it returns the archive's log path. The actor then:
- appends the current conversation's new rows to that archive's `log.jsonl`;
- saves and loads manifests through `manifest_path`, which follows the pointer, so it overwrites the archive's manifest.

The author's report lists this as a limitation, but it is silent corruption of a stored session, and it contradicts "the legacy log stays served". The notice says the items "stay where they were", which is false for the live conversation. On the next successful open the real legacy conversation is imported as a separate `legacy` session while `current` stays on the polluted archive.

Fix, either of:
- (a) In `prepare`, when `report.failed` names the legacy item, or legacy files remain without a `legacy` session dir, create a fresh empty session and `set_current` it. The legacy items are then imported on retry and nothing foreign is written to.
- (b) Make `migrate_legacy` refuse to set or repair the pointer to an archive while the legacy item is unmigrated.

Add a test with a failing legacy item plus a migratable archive, asserting that no row lands in the archive.

### 2. High: a completed provider switch's handoff context leaks into whatever session is opened next
`agent/actor/session.rs:805-815` (`replace_conversation`), `session.rs:449-450`, `turn.rs:193-240`.

`next_handoff_context` (the bounded transcript summary of the outgoing provider's conversation) and `next_draft_choice` (possibly `FromHandoff`) survive until the next task starts. `replace_conversation` clears the stream, tool rows, task and restore state, but not these two. The switch blocker only covers `switch_target` and `switch_pending`, which are already cleared once the switch completes.

Scenario: switch provider from A to B, then open session S instead of prompting. The first brief in S is sent with session-A's handoff envelope appended, and the draft is prepared from A's handoff revision. This is a cross-session data leak into the agent prompt, and it can also resume S's wire session with foreign context. It existed for New chat; `open_session` makes it reachable to a different stored conversation.

Fix: in `replace_conversation`, set `next_handoff_context = None` and `next_draft_choice = None`. Alternatively add "a handoff continuation is pending" to `switch_blocker`. Add a test.

### 3. Medium: resume failure permanently poisons the stored session, and it does not "fall back to a fresh session"
`session.rs:8-35` (`on_provider_failure`), `session.rs:903-940`.

Any `Phase::Session` failure or `RestoreUnsupported` marks the current session's manifest `Unsafe` and then `fail_with`s the task. The task ends `Failed`; no fresh session is started automatically, so the author's "falls back to a fresh session" claim is imprecise. The stored session then reads `resumable=false` forever, including after a transient timeout.

The capability decision that feeds this is weak:
- `self.capabilities` is whatever the last launched agent reported (`prompt.rs:79,452`). It is never reset in `settings_changed`, and it is not keyed to the manifest's provider.
- `capabilities.or(manifest.capability_snapshot)` prefers the global value over the session's own snapshot. After a provider change the snapshot taken from provider A can approve a resume for session of provider B, or the reverse (the reverse is safe).

Fix:
- Use `manifest.capability_snapshot` when `self.adapter.last_launch` or the last initialized provider is not `manifest.provider_id`.
- Do not mark the stored session `Unsafe` for `RestoreUnsupported`/session-phase failures that happen on a resume attempt.
- Better, retry once with `resume_session = None` inside the same task.

### 4. Medium: New chat on an empty conversation now creates an unbounded stack of empty sessions
`session.rs:829-860`, `log.rs:443-475`.

The old `start_new` on a log with no file was a no-op (`archived = None`). `begin_session` always calls `create_session`, so each click persists a new directory and lists a `rows == 0` session. This is also acknowledged as a limitation. It is cheap to fix: if the store has no resident rows and the file is empty or missing and there is no manifest, return without creating anything. Also remove the old empty session when leaving it.

### 5. Medium: every open of a stored session appends notice rows to it
`session.rs:903-940`, `session.rs:615-706` (`restore_session` always emits a notice).

Resuming pushes "Restoring session…", and read-only pushes the long "cannot be resumed" notice, both through `push_row` into the opened session's own log. Effects:
- Browsing sessions mutates them and re-sorts the list by recency.
- The stored transcript accumulates a duplicate notice per visit.
- An empty session becomes non-empty after one visit.
- The transcript is described as read-only.

Fix: show these as transient state (the `session_restore` view already carries the text) or use `warn_once` dedupe. Keep notices out of the log.

### 6. Medium: heavy synchronous disk work on the actor thread, with no off-thread guard
`session.rs:829-895`, `history.rs:94-117`, `layout.rs:256-265`.

`open_session` reads the whole target log three times: `load_session(..,1)` decodes every row just to get the manifest, then `RowStore::open` replays it again. `begin_session` runs `row_id_floor` (a full `read_rows`) on the old log. All of this runs on the actor, which the code elsewhere takes care to keep responsive for Stop and Close (`history_page` is off-thread, `needs_controller` defers).

A large log is a long actor stall. A log path that blocks (the FIFO case the existing history test covers) hangs the actor, and Close, forever.

Fix:
- Read the manifest alone (`layout::read_manifest`) instead of `load_session`.
- Compute the floor from the opened store's `sequence_boundary()` or from the meta first row id plus the in-memory boundary, not a second file scan.
- Ideally run the open on a worker and swap on completion.

`open_session` and `restore_session` also take the controller lock directly on the actor, but are not in `needs_controller`.

### 7. Medium: `open_session` can silently do nothing after a New chat
`agent.rs:960-980`, `session.rs:861-870`.

The "already current" no-op is decided on the caller thread from the pointer file. If the caller sends `new_session()` (still queued) and then `open_session(X)` where X was current, the check sees X as current and returns `Ok(())`. The actor then processes New chat, and the user ends up on the new empty session instead of X.

The actor re-check at `session.rs:867` is sound. The caller-side early return is not. Fix: drop the caller-side shortcut, since the actor already no-ops.

### 8. Low-medium: legacy files that cannot be deleted are re-imported as a new session on every open
`layout.rs:568-597` (`settle_item`).

If `remove_legacy` fails (file locked, which is common on Windows), the next open finds the same legacy log, and its copy no longer matches because the live session log has grown. It therefore imports the stale legacy log as another `legacy-<unix>` session and fails to delete it again. Each launch adds one duplicate listed session until the file is removable. Fix: remember a marker (for example record the imported hash in the session meta), or import once and write a tombstone.

### 9. Low-medium: row ids are unique only within one process lifetime
`startup.rs:86-90`, `layout.rs:287-303`.

The floor at startup is only the current session's own `first_row_id`. `create_session` sets `first_row_id` from the session that is current at the time. If A (ids 1-10) is open, B (11-30) was created earlier, and the user opens A and then New chat, C's meta floor is 11, below B's ids.

After a restart with C current and empty, C hands out 11.. and collides with B. In-process this is fine, because `open_session_log` uses the in-memory boundary. Legacy and archive sessions all get `first_row_id: 1` (`layout.rs:516`). This only matters if the UI or any consumer dedupes by id across sessions. Fix: have `prepare` and `create_session` use the max floor across all sessions.

### 10. Low: History pages are not tagged with the conversation
`mod.rs:736-753`, `agent.rs:509-520`.

`history_page` is captured against whichever store is current when the message is processed. A page request issued for the old session that is queued behind `OpenSession` returns the new session's older rows, with ids that do not relate to the requester's `before`. The caller must compare `chat` and discard stale results. The `done` callback gets no `chat` value to do it with.

Fix: pass the expected `chat` in `Msg::History`, and answer `Err` or an empty page on mismatch.

### 11. Low: `RestoreSession` is not gated by the read-only decision
`agent.rs:907-911`, `session.rs:615-706`.

After the read-only fallback the view says `is_resumable=false`, but the engine does not enforce it. A `restore_session()` call re-reads the manifest and sets `pending_resume_session` from the manifest's own `is_resumable()`, bypassing the capability and provider checks. The next prompt then resumes the old wire session. `spawn_writer` still re-checks provider, digest and revisions, but not capabilities, so it can hit finding 3. Fix: refuse `RestoreSession` in the actor when `session_restore.is_resumable` is false.

### 12. Low: a stale wire id can be marked resumable after a fresh start in a read-only session
`finish.rs:233-273`.

If the fresh session has no usable wire id (redacted), `on_session_ready` does not rewrite the manifest. `finalize` then loads the old manifest (provider matches), bumps `last_*_revision` to the current source, sets `Ready` and reports `is_resumable=true`. The old wire session never saw the new turns, but is now claimed resumable at the new revision. Fix: when the fresh start has no wire id, clear the manifest's `wire_session_id` (or set it non-resumable).

### 13. Low: Busy wording and escape hatch
`agent.rs:231-234,982-988`, `session.rs:724-748`.
- `AwaitingReview` and conflict states are reported by the caller-side check as "Stop the running task…", while the actor says "Apply or Discard…".
- `switch_blocker` includes unresolved recovery conflicts, so a project with an unresolved conflict cannot change session at all, not just not start a new chat.
- A session whose log cannot be flushed cannot be left, because flush failure aborts the switch (correct for not losing rows, but it is the only exit and gives no way out).

## Checked and found sound
- Flush, open, `set_current`, replace order (`session.rs:829-895`). A failure at any step keeps the old store and the old pointer. Crash after `create_session` leaves only an empty listed directory (clutter, covered by finding 4). A pointer write is atomic. `begin_session` removes its new dir on failure, and the pointer was not written in that case.
- Concurrent publish: the actor is the only writer and runs the swap on its own thread, so there is no interleaving. `sessions()` from another thread can only see a flushed prefix or briefly an unreadable file during compaction (shows "Unreadable session" transiently).
- Busy coverage: `active` covers turn, validate, apply (`promoting`), reject (`rejecting`) and AwaitingReview. Undo and revert-path are covered by `undo`. Queued briefs by `queue` and `retained_queue`. Submissions in the channel before `OpenSession` are processed first and then refused by `switch_blocker`. The `registry.busy()` pre-check includes `pending`, so a submit that has already incremented pending is refused.
- Late results after a switch (`Evidence`, `Probe`, `Committed`, `HandoffResolved`) either require an active task or update view state only, not rows.
- Row ids across New chat and `open_session` in a single process: the floor is `sequence_boundary()` of the store being left.
- The resume path selects the manifest through the pointer that was just set, so the wire id comes from the right stored session. `spawn_writer` rechecks provider, launch digest, draft path and both revisions. The read-only fallback sets no `pending_resume_session`, and `replace_conversation` clears any previous one.
- Nothing in this change runs on the UI thread except `open_session`'s `is_dir`/pointer reads and `sessions()`, which is documented as off-thread.

## Test gaps
- No fault injection for flush failure, `set_current` failure, or a log that cannot be opened mid-switch.
- No test of a partial migration with archives present (finding 1).
- No test that a switch is refused during validation, reject or apply, only during a hanging edit.
- No test for a handoff context pending across a switch (finding 2).
- No check that opening a session leaves its log unchanged (finding 5).

Status: DONE_WITH_CONCERNS
Summary: The swap sequence and Busy handling hold, but a partial migration can write live rows into an archived session, a pending handoff context leaks across sessions, and resume failure marks the session Unsafe. Several actor-thread blocking and notice-row side effects also need fixing.
Concerns/Blockers: Review was static only. Bash was unavailable, so no test, clippy or run was done, and line numbers outside `session.rs` and `startup.rs` are approximate.
