# Phase 04 engine review (per-file review, reject files, per-path revert)

Scope: the uncommitted diff of `feat/studio-m5-p4`, read from the saved diff file plus the current files. Nothing was run (no cargo). Line numbers refer to the current worktree files.

Verdict: no path was found where an unvalidated or partially validated tree is applied or becomes `CandidateReady`. `authorizes` in `controller/promotion.rs` is untouched. The defects below are one history-accounting bug, one case where the original candidate is lost, and several robustness gaps.

## What I verified as sound

- `authorizes` is byte-identical. The replacement keeps the task identity, source base and manifest rules, so the same function guards Apply.
- `agent_settle_rejection` (review.rs:168) checks the task, candidate, source base and manifest against the report. It calls `candidate_validated` only when `next_step` returns `Accept`.
  - `Accept` requires `acceptance_gap() == None`.
  - A `Source` failure maps to Repair or Retain, which gives `false` here without spending the repair budget.
- While the replacement is unsettled, the engine task is `Validating`, so Apply, Discard and a second Reject are refused by the actor's phase check. The engine also refuses Apply from `Validating`.
- Rejected and reverted paths are never joined to the filesystem.
  - Reject input is matched only against the `file_changes(base, candidate)` path set (review.rs:51 onward). `..`, absolute and unknown paths fail with "not a file this candidate changes".
  - Revert input must equal a path in `entry.changes` (state.rs:230).
- The replacement is rebuilt from the task base plus accepted deltas and then compared to the accepted path set (review.rs "formed_paths != expected"). A rejected file cannot be silently carried along or dropped.
- Stale After evidence for a replaced candidate is dropped (prompt.rs:371).
- Journal: format 2, 3 and 4 lines all pass `header.format` in `OLDEST..=TASK_FORMAT`, and newer than 4 is refused without a rewrite (journal.rs:446). `undoes_paths` is `#[serde(default, skip_serializing_if)]`.
- Partial revert only removes created directories that are empty again (executor.rs:610 uses `rmdir`), so passing the full `created_dirs` is safe.
- Diff reads happen on the caller thread of `candidate_file_diffs`. The controller lock is held only to copy the source descriptor, and file I/O is outside it. Reads are capped per file (512 KiB) and in total (8 MiB).

## Findings, ranked

### 1. MEDIUM-HIGH: Stop during replacement validation ends the whole task and discards the original candidate
- Where: `agent/actor/session.rs:96-103` (Stop in `Validating` cancels the gate), `agent/actor/apply.rs:829-870` (`on_rejected` -> `cancel_or_interrupt`), `agent/actor/apply.rs:812` (`resume_review` with `stop`). The test `stopping_while_the_remaining_files_validate_ends_the_task_and_applies_nothing` locks this in.
- Scenario: the user has a validated candidate in manual review, rejects one file, and the replacement validation takes minutes (compile and probe). The user presses Stop to abort the reject. The actor finalizes the task as Cancelled. The original validated candidate is dropped, the draft is kept, and the user must start a new agent task.
- Why this is inconsistent: Stop in `AwaitingReview` is deliberately a no-op that tells the user to use Discard (session.rs:125-131). The brief says "failure/Stop/error restores the original candidate". Only failure and error do. The Stop path is also reachable with a harmless race: Stop pressed just as the replacement settles passes the new candidate to `cancel_or_interrupt` as well.
- Fix: in the `Validating` branch, if the job is `Reject`, cancel only the job's gate and scopes and set no `stop_requested`. On `RejectEnd::Cancelled` call `resume_review` unconditionally, with a notice "Stopped; the candidate is unchanged". `closing` should still interrupt. Update the test to expect `AwaitingReview` and the original candidate.

### 2. MEDIUM: Partial-revert accounting breaks after a whole Undo plus a redo of the partial revert
- Where: state.rs:168-196 (`reversals`, `Reversal.whole`), state.rs:200-212 (`reverted_paths`), `fully_reversed`.
- Scenario: Apply A changes {x, y}. `revert_path(x)` records U0 with `undoes_paths=[x]`. A whole Undo of A records U1, which inverts only `y` (via `undoable_changes`) but has `undoes_paths = None`. Then the user undoes U0, which puts x back (`undo_target(Some(U0))` allows it; U0 is not fully reversed). `reversals` now skips U0 (targeted) and holds only `whole = true` from U1. Result: x is back in effect on disk, but history says A is entirely reversed. `reverted_paths(A)` reports x and y, `prepare_undo(A)` is refused ("already undone"), and `revert_target(A, x)` is refused ("already reverted"). The user has no history-driven way to revert x again.
- Why: a whole Undo records "everything" rather than the paths it actually covered, so later changes to which earlier partial Undos are effective are not reflected.
- Fix: record the covered paths on every Undo. For a whole Undo set `undoes_paths = Some(paths of changes)` in `form_undo`, and treat `None` (old records) as whole only for pre-format-4 lines. Alternatively compute `whole` chronologically as "all paths not already covered by an effective partial at that point". Add a test for this exact sequence; the existing tests never undo an Undo after a partial revert.
- Only reachable if the UI offers Undo for Undo entries. `undo_target(Some(id))` accepts any id, so the engine does not prevent it.

### 3. MEDIUM: `replace_candidate` mutates before checking its precondition
- Where: agent_task/mod.rs:971-977 (`task.candidate.replace(candidate), task.candidate_manifest.replace(manifest)` inside a tuple, with an `else { return Err(ReportMismatch) }` afterwards).
- Scenario: if either stored field were `None` (not reachable today because `record_candidate` always sets both in `Validating`, but nothing in the type system enforces it), the function returns `Err` after both fields already hold the unvalidated replacement. The task is still `CandidateReady`, so an unvalidated candidate would sit in `CandidateReady` and `agent_reject_files` would return the error without a rollback. This is exactly the invariant the phase is meant to protect.
- Fix: read with `task.candidate.take()` or use `as_ref()` to validate first, then assign; or use `let (Some(..), Some(..)) = (task.candidate.clone(), task.candidate_manifest.clone()) else { return Err(..) };` before mutating.

### 4. MEDIUM: errors from restoring the original candidate are swallowed, leaving the actor and the engine out of step
- Where: jobs.rs:728-731 (`let _ = env.controller.lock().agent_abandon_rejection(rejection)`), then `RejectEnd::{Invalid, Refused, Cancelled}` -> `resume_review` (apply.rs:812) sets the phase to `AwaitingReview` unconditionally.
- Scenario: `restore_candidate` fails (task state moved off `Validating`, identity check fails). The engine stays `Validating` with the replacement candidate recorded while the actor shows `AwaitingReview` for the original retained candidate. Apply is then refused on every attempt with a confusing engine error; Discard works. Nothing unsafe is applied, but the user is wedged and the cause is hidden.
- Fix: return the restore error in the `RejectEnd` (for example `RejectEnd::Refused` carrying both errors) and let `on_rejected` `fail_with` it instead of `resume_review`, or at least check the engine state before `resume_review`.

### 5. LOW-MEDIUM: a panic or crash in the reject job loses the original candidate and leaks scopes and the draft
- Where: jobs.rs:703 (`run_reject` owns `retained`), `on_job_aborted` `JobKind::Reject` (apply.rs:559) -> `fail_with`.
- Scenario: a panic in validation after `agent_reject_files` moved the task to `Validating` ends the task `Failed` (acceptable, but the doc comment "every exit settles it" and the brief's "error restores the candidate" do not hold for panics). `retained.scopes` are not shut down on unwind; only `Retained::drop` tears down the staged worker. The temp tree `drafts/reject-<uuid>` is removed only on the non-panicking path (review.rs: `fs::remove_dir_all(&draft)` after the closure), and not at all after a process crash.
- Fix: guard the draft with a drop-guard, shut `retained.scopes` down in a scope guard, and document that a panic ends the task.

### 6. LOW-MEDIUM: heavy I/O under the controller mutex, not cancellable
- Where: jobs.rs:714-717 calling `agent_reject_files` under `lock_unless_cancelled`; review.rs:51-150 loads two inventories, writes every accepted object into a draft and re-hashes the tree in `capture_replacement_candidate`, all while the mutex is held. `candidate_file_diffs` and `revision_file_diffs` (agent.rs:767-790) do a blocking `controller.lock()` on the calling thread.
- Impact: any other controller user (UI snapshot, timing edits, preview) blocks for the full capture of a large project, and Stop cannot interrupt it (the gate is checked only afterwards). A UI thread calling the diff functions blocks on the same lock.
- Fix: copy the base, candidate, scope and identity out under the lock, build and capture the draft outside it, and re-take the lock only for `replace_candidate` after re-checking the candidate. For the diff functions, document or enforce that they run on a worker thread, or make the source reads available through a cloned `Arc`.

### 7. LOW: diff reads are bounded only after the whole object is read
- Where: diff_text.rs:77-79 uses `Checkpoints::copy_object(sha256, size, &mut bytes)`. `copy_object` (studio-project/src/checkpoint.rs:122) streams the entire file into the writer and only afterwards checks `digest != hash || count != size`. `Vec::with_capacity(size)` does not cap growth.
- Scenario: a damaged or oversized object (store is app-owned, so low likelihood) can allocate an arbitrarily large `Vec` before being rejected as an integrity failure.
- Fix: wrap the output in a writer that errors past `size`, or read with `Read::take(size + 1)`.
- Also: the number of `FileDiff` entries is unbounded (one per changed path). A candidate with a very large change set builds a large vector even though the text is capped. Consider a path-count cap with an "N more files" marker.

### 8. LOW: candidate diffs carry no revision, so the viewer can show stale content
- Where: `CandidateDiffSource` (review.rs:206) and `FileDiff` (diff_text.rs).
- Scenario: the user opens file diffs for candidate C1, then a reject replaces it with C2. The UI still shows C1's `FileDiff` list (no candidate revision on it), and the next reject request may name a path that C2 no longer contains. The controller refuses it ("not a file this candidate changes"), so nothing unsafe happens, but the UI sees an error for a path it just displayed.
- Fix: return the candidate revision with the list (or add it to `FileDiff`), so the UI can discard or refresh results for another candidate.

### 9. LOW: other behaviour details
- apply.rs:372-376 / `await_review(None)` after `Replaced` clears a previous `apply_blocked` reason and pushes a second "waiting for your review" outcome row. The blocked-gate reason shown before the reject is lost until the next Apply fails again.
- apply.rs:737-749 discards the whole candidate when every listed path covers the (non-truncated) change set, even if the list also contains unknown paths. Harmless, but it skips the unknown-path refusal the controller would give.
- agent_task/mod.rs:150: the transition table now permits `CandidateReady -> Validating` for the public `agent_task_transition`, although `replace_candidate` sets the state directly and needs no table entry. A caller can now move a validated task to `Validating` with the same candidate and no replacement. Prefer leaving the public table unchanged.
- rows.rs:438 calls `history.reverted_paths(&r.id)` once per displayed entry, and each call recomputes `reversals()` over the whole history (quadratic per snapshot refresh). Compute `reversals()` once.
- Concurrency of reject and revert: they are mutually exclusive (reject needs an active task, revert needs none), and `undo_task_gated` rechecks the history fence, so I found no race beyond item 1.

## Concurrent-file edits (additive check)

- `agent.rs`, `actor/mod.rs`, `actor/finish.rs`, `agent/model.rs`: additive (new enum variants, new methods, new serde-defaulted fields, one match arm).
- `actor/prompt.rs`: not purely additive. It adds an early `return` in the existing `on_evidence` (the stale-After guard). It is small and correct, but a concurrent edit to that function will conflict.
- `agent/jobs.rs`: additive except the new `JobKind::Reject` variant. Any exhaustive `match JobKind` elsewhere (another session's code) will stop compiling; the only one I found is `on_job_aborted`, which is updated.
- `agent/actor/rows.rs` and `agent/actor/apply.rs` are also modified. They are not on the "additive only" list but are touched. The `HistoryEntry { .. }` literal in rows.rs gained two fields, so any other constructor of `HistoryEntry` in another session will fail to compile. I found no other constructor in this tree.
- `WorkflowError` gained `NoCandidate` and `Review`; I found no exhaustive match on it outside `agent.rs`.

## Test gaps worth adding
- Undo an Undo after a partial revert (item 2).
- Stop during replacement validation expecting the original candidate to survive (item 1).
- `replace_candidate` with an inconsistent task (item 3), asserting nothing changed on error.
- Restore failure in `agent_abandon_rejection` (item 4).

Status: DONE_WITH_CONCERNS
Summary: No way to apply or reach `CandidateReady` with an unvalidated tree was found, and `authorizes` is unchanged. The top issues are that Stop during replacement validation discards the original candidate (item 1), and that a partial revert, whole Undo and redo sequence corrupts per-path history accounting (item 2).
Concerns/Blockers: Bash was unusable, so everything was read from the saved diff file and the worktree files; nothing was compiled or run. I did not check the studio-ui callers of the new `AgentWorkflow` methods.
