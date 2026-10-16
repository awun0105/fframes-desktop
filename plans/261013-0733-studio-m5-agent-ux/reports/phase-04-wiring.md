# Review viewer wired to the engine (per-file review)

Date 2026-10-16. Working copy `fframes-m5`, nothing committed.

## What is wired

- Entry: a "Review changes" button on the change summary of the candidate that awaits review
  (only the current one, not older summaries) and on each accepted ("Apply") history entry in
  the agent sidebar. The panel emits `ReviewRequest(ReviewTarget)`; `PanelEvent` is untouched.
- Window: `StudioShell` swaps the preview for a `ReviewHost` (Changes view plus a status strip)
  when a request arrives. Close returns to the preview. Playback pauses on open. The host is
  closed when the workflow is detached, and for a candidate window when the task ends
  (applied, discarded, failed).
- Mapping (`changes/source.rs::map_diffs`): engine `FileDiff` to `DiffFile`. Every `NotShown`
  variant maps (`Binary`, `TooLarge`, `Budget`, `Unreadable`; viewer `NotShown` gained `Budget`
  and `Unreadable`). The trailing `More { files }` entry (empty path) becomes a count shown as
  "N more files not shown"; it never becomes a row or toggle and an empty path is also filtered
  out of Reject in both the viewer and the host.
- Threading: diff reads, `reject_files`, `revert_path`, `discard` go through the
  `ReviewBackend` trait (implemented for `AgentWorkflow`) and run on the background executor.
- Candidate freshness: candidate diffs have no revision, so the host keys them by
  `task id / review.candidate`. It reads when the candidate is AwaitingReview and the key changed
  or a reject settled; never while Validating. A result whose request serial is not the newest,
  or whose key is no longer the current one, is dropped.
- Reject: `Reject(paths)` calls `reject_files`. Until a snapshot answers (Validating then
  AwaitingReview, new candidate key, a newer error/warning row, or task end) the status says
  "Validating the remaining files..." and toggles, Reject, Revert are disabled. A refusal row
  text is shown as the status and the candidate stays. Panel Apply/Discard live in the banner
  only while the phase is AwaitingReview, so they are already absent while Validating.
- Reject everything: the button turns into "Discard all..." (only when no unlisted files exist),
  a confirmation strip asks, and confirming emits `ChangesEvent::Discard`, which the host sends
  to `workflow.discard()` (the engine treats reject-all as discard, so the same path).
- Revert: `RevertFile(path)` on an applied revision calls `revert_path`. A refusal (newest
  error row, e.g. "Undo conflicts with later changes to src/lib.rs") is shown in the status
  strip. `HistoryEntry.reverted_paths` show a "Reverted" label and no Revert button. Revert is
  disabled while a task is active or an undo is running.

## Files

New (all under `desktop/crates/studio-ui/src/`): `agent/changes/source.rs` (mapping, backend
trait, `Facts`), `agent/changes/host.rs`, `agent/changes/host_tests.rs`,
`agent/changes/strips.rs` (reject button, discard confirmation, more footer),
`agent/review_entry.rs` (panel buttons), `shell/review.rs`.

Edited, additive: `agent/changes/mod.rs`, `model.rs`, `file_list.rs`, `tests.rs`;
`agent/mod.rs` (+1 line), `agent/cards.rs` (call site, button), `agent/project_tab.rs`
(history button); `shell/mod.rs` (+4 lines), `shell/layout.rs`, `shell/engine_events.rs`
(close at the three detach sites, forward snapshots in `agent_refreshed`).
`desktop/app/tests/agent_workflow_review.rs` (+2 tests). `controls.rs` and `commands.rs` are
untouched. No file in `agent/changes` exceeds 617 lines.

## Tests

- studio-ui lib: 451 pass (new: mapping of every NotShown variant on either side, More marker
  is a count and never a file, Reject routes the toggled paths and refuses "", reject-all asks
  first and then discards, declining changes nothing, stale and gone-candidate results dropped,
  reread after validation, refused reject shows why, window closes when the candidate is gone,
  revert calls the engine and shows its refusal, reverted label without button, revert disabled
  during a task, a stopped reject says the candidate is unchanged, viewer-level
  notice/footer/busy/confirm/reverted tests).
- Parity: one new case in `crates/studio-ui/tests/parity/agent.rs` (candidate card, Review
  changes click, window open with files, panel Apply closes it). It passes (the agent parity group, 8 tests, passes). Telemetry gains a `review` key only while the window is open, and
  the `agent-review-candidate` button joins the measured buttons.
- `fframes-studio --test agent_workflow_review`: 6 pass, including two engine-backed tests: the
  candidate key changes after a real reject and the mapped listing drops the file; a real
  refused revert (file edited by hand) is found by `facts` with the reason.
- `studio-engine --test file_review`: 10 pass. fmt check clean; clippy `-D warnings` clean for
  studio-ui, studio-engine, fframes-studio with all targets. Parity: only the agent group was run, not the whole binary.

## Visual check

A throwaway example window (deleted afterwards, process stopped) drove the real `ReviewHost`
with a scripted stand-in backend; real clicks were posted to the window and captured with
PrintWindow (the session is disconnected, so CopyFromScreen fails with "handle is invalid";
`render_to_image` is macOS only). Looked at all six:
`reports/phase-04-visual-candidate.png` (all not-shown reasons, "37 more files not shown"),
`-validating.png`, `-after-reject.png`, `-discard-confirm.png`, `-revision.png`,
`-revert-refused.png` (refusal strip plus a Reverted file). A real candidate in the full app was
not looked at by eye: the scripted fake agent exists only in the parity harness, which the lead
runs; the real app needs a configured agent SDK and provider. The parity case above is the
executable check instead.

## Checked in source

- Replacing the preview with the window does not stall publication: the preview install and
  commit run in the shell's own render (`shell/layout.rs`), not in the player view, and the
  settings and export panels already remove the player from the tree the same way.
- Reject cannot settle early: on the success path `reject_files` (engine `actor/apply.rs`) sets
  Validating before any row is published; rows only come from its outcome.

## Parity failure and its cause

The first parity run timed out waiting for the button. Two causes, both in the test, not the
product:

- The helper read `t["buttons"]`, which is the timeline's map; the panel's measured buttons are
  under `t["panel"]["buttons"]`.
- At the harness window size the transcript is about 290 px tall and sits under the Apply banner.
  Pinned to the newest rows, it shows the validation and outcome cards and the candidate card
  above them is out of view, so the button is not painted and not measured. This is ordinary
  scrolling, not a missing button: with the transcript scrolled until the card was inside the
  list, the click opened the window. The test now makes the window tall enough that the whole
  transcript is on screen, which is what it asserts on.

## Diff work off the UI thread, Unreadable reason kept

- The background read now also builds the diffs and syntax highlights (`Prepared::compute`,
  carried in `source::Read`), and the viewer takes them with `Changes::set_prepared`. Only
  installing them runs on the UI thread. `Changes::new` and `set_files` still compute inline
  for callers that hold plain files.
- `DiffFile.detail` carries the reader's reason for an unreadable file; the notice reads "File
  could not be read, contents not shown: <reason>". Covered by a mapping test in `source.rs`.

## Limitations

- A `ChangeKind::Mode` change shows "No changes".
- The status line is cleared only by the next operation.
- The review window is checked end to end in one parity case; a real agent in the running app
  was not driven by hand.