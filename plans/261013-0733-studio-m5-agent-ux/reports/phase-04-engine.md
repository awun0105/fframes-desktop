# Phase 4 engine report: per-file review, reject before apply, revert one file after apply

Worktree: `C:/Users/Administrator/fframes-m5-p4` (workspace `desktop/`, branch feat/studio-m5-p4). Nothing committed or staged.

## API the UI will call

Types (re-exported from `studio_engine`): 
- `FileDiff { path: String, change, before: FileText, after: FileText }`
- `FileText::{Absent, Text(String), NotShown(NotShown)}`; `NotShown::{Binary, TooLarge, Budget, Unreadable}`. Limits: `MAX_TEXT_BYTES` 512 KiB per side, `MAX_TOTAL_TEXT_BYTES` 8 MiB per read.
- `CandidateDiffSource`, `RevisionDiffSource`, each with `read(&self) -> Result<Vec<FileDiff>, EngineError>` (reads files; call off the UI thread).
- `Rejection` (`captured()`, `rejected()`).

`AgentWorkflow` (studio_engine::agent):
- `candidate_file_diffs(&self) -> Result<Vec<FileDiff>, WorkflowError>` (NoCandidate unless a candidate is waiting for review)
- `revision_file_diffs(&self, revision: &str) -> Result<Vec<FileDiff>, WorkflowError>`
- `reject_files(&self, paths: &[String]) -> Result<(), WorkflowError>` (empty list -> `Review`; async, result arrives through the snapshot)
- `revert_path(&self, revision: &str, path: &str) -> Result<(), WorkflowError>` (async like Undo)
- `WorkflowError::NoCandidate`, `WorkflowError::Review(String)` added.
- `HistoryEntry` gained `undoes_paths: Option<Vec<String>>` and `reverted_paths: Vec<String>` (both serde default).

Controller (`controller/review.rs`):
- `agent_reject_files(&mut self, &CapturedCandidate, &[String]) -> Result<Rejection, EngineError>`
- `agent_settle_rejection(&mut self, &Rejection, &ValidationReport) -> Result<bool, EngineError>`
- `agent_abandon_rejection(&mut self, Rejection) -> Result<(), EngineError>`
- `candidate_diff_source(&self) -> Option<CandidateDiffSource>`, `revision_diff_source(&self, &str) -> Option<RevisionDiffSource>`
- `prepare_revert_path(target, path)` in controller/promotion.rs; `UndoPreparation` gained `undoes_paths` and `paths()`.
- `TaskHistory::reverted_paths(id) -> Vec<String>`, `undoable_changes(&entry)`, `revert_target(id, path)` in state.rs.
- `authorizes(report, captured)` is UNCHANGED.

## Reject state-machine decision

Same-task replacement. `CandidateReady -> Validating` is now an allowed transition. `agent_reject_files` builds a subset tree (task source base plus accepted deltas), captures it as a replacement candidate (same task identity, source base and scope) and moves the task to `Validating`. The replacement goes through full candidate validation. It never spends the repair attempt.
- Passing report -> `candidate_validated`, task `CandidateReady` with the replacement; Apply and `authorizes` work unchanged.
- Failing report, Stop, or error -> `agent_abandon_rejection` restores the original candidate (`CandidateReady`). A subset is never `CandidateReady` unless validated, so an unvalidated subset cannot be applied.
- Reject-all is refused by the engine (the actor routes it to discard); unknown path, empty list, or a split that cannot be separated (changed-path set differs from accepted set) are refused.

## Journal format 4

`TASK_FORMAT` = 4. `TaskRevisionRecord.undoes_paths: Option<Vec<String>>` (omitted when None). A per-file revert is an Undo record with `undoes` = target and `undoes_paths` = the reverted paths; a whole-revision undo has None. Formats 2, 3 and 4 decode, and can be mixed in one journal (fixture test). `OLDEST_TASK_FORMAT` stays 2; appends use 4; anything newer than 4 is refused as `NewerFormat`; older builds refuse format 4. A revision counts as reversed when every one of its changed paths is reverted (per-path or whole); Undo of a partly reverted revision inverts only what remains.

## Files touched

New: `studio-engine/src/controller/review.rs`, `src/candidate_validation/diff_text.rs`, `tests/file_review.rs`.
Engine (mine): `edit_transaction/mod.rs`, `state.rs`, `journal.rs` (comments and tests), `controller.rs` (mod + re-export), `controller/promotion.rs`, `controller/preset.rs` and `controller/migration.rs` (one-line `undoes_paths: None`), `candidate_validation/{capture.rs,mod.rs}`, `agent_task/{mod.rs,tests.rs}`, `agent/actor/apply.rs`, `lib.rs` re-exports, tests `review_history.rs` and `task_recovery.rs` (future-format fixtures moved to format 5).

Additive hooks in concurrent-list files:
- `agent.rs`: `WorkflowError::{NoCandidate, Review}`; `AgentWorkflow::{candidate_file_diffs, revision_file_diffs, reject_files, revert_path}`.
- `agent/actor/mod.rs`: imports; `Msg::{RejectFiles, RevertPath, Rejected}`; three dispatch arms.
- `agent/actor/finish.rs`: one drain-loop arm `Msg::Rejected`.
- `agent/jobs.rs`: `JobKind::Reject`; appended `RejectInput`, `RejectEnd`, `run_reject`, `shut_down`, `run_revert_path`.
- `agent/model.rs`: two `HistoryEntry` fields.
- `agent/actor/rows.rs`: `refresh_idle` fills them.

## Tests (all pass)

- `cargo test -p studio-engine --no-fail-fast`: all binaries green (lib 201; file_review 7; journal mixed-format and round-trip tests).
- `cargo test -p fframes-studio --test agent_workflow --test agent_workflow_ui --test agent_workflow_apply_gate`: 61 / 3 / 13 passed.
- `cargo test -p studio-ui --test parity`: 63 passed.
- clippy 1.98.1 `--all-targets -D warnings` for studio-engine and studio-ui: clean. `cargo fmt --all --check`: clean.

file_review covers: rejecting a dependent file fails validation and restores the original; valid subset applies exactly the accepted files; reject-all/unchanged/empty refused; revert A then reopen then Undo inverts only B; reverting every file reverses the revision; revert after outside edit refused with a conflict naming the file; per-file text for added/deleted/modified/binary and for an accepted revision.

## Actor-level tests (`app/tests/agent_workflow_review.rs`, 4 pass)

Real scripted agent writing three files (`src/lib.rs`, worker config, `notes.txt`) under ManualReview:
- reject `notes.txt` from AwaitingReview -> replacement validated, back in AwaitingReview/CandidateReady, Apply publishes `src/lib.rs` only, `notes.txt` absent, `assert_clean`.
- reject then Stop while the replacement validates (compiler blocked) -> the TASK ends Cancelled (the actor treats Stop as ending the task; the engine restores the original candidate before that), nothing applied. The earlier wording "Stop brings back the original candidate" refers to the engine step only.
- reject every file -> routed to discard, task Cancelled, source unchanged.
- apply, then `revert_path(rev, "src/lib.rs")` -> file restored, `notes.txt` untouched, `HistoryEntry.reverted_paths` contains the path.

## Closed review items

- Stale evidence: an After-evidence job for a candidate replaced by a rejection could still land (reject is allowed while the first After evidence is pending). Fixed with an additive guard in `on_evidence` (`agent/actor/prompt.rs`): After evidence whose revision differs from the retained candidate is ignored. Every After request uses the candidate revision, so the normal path is unaffected.
- Persistence: the agent task manager is in memory only (`Controller::open` builds a fresh `AgentTaskManager`; `close`/`drop` interrupt the active task). A crash during the replacement's Validating therefore cannot reopen as CandidateReady with an unvalidated subset; the task is gone and its draft is recovered as a restart leftover. The invariant holds.

## Remaining notes

- `start_revert_path` / `run_revert_path` duplicate `start_undo` / `run_undo` to honor the no-restructure rule on concurrent files.
- Mode-only changes are no-ops on Windows; engine tests filter `run.sh`.
- The engine-level reject-validation-failure test supplies a failing `BuildOutcome` (no real compiler in engine tests); the actor tests use the scripted compiler.

## Review fixes

Supersedes the Stop wording above (the earlier "task ends Cancelled" behaviour is gone).

1. Stop during the replacement validation: Stop cancels only the reject job (new `Active.rejecting` flag; early branch in `stop` in `agent/actor/session.rs`), the engine restores the original candidate, and the task returns to `AwaitingReview` with the original (notice "Stopped; the candidate under review is unchanged."). `resume_review` and `on_rejected` no longer treat a cancelled gate as a stop; closing and a pending provider switch still end the task. `agent_workflow_review::stopping_while_the_remaining_files_validate_returns_to_review_of_the_original` asserts AwaitingReview, CandidateReady with the same candidate, `notes.txt` still listed, then Apply publishes every file.
2. Per-path accounting: every Undo now records the paths it actually inverted (`form_undo`: `undoes_paths = Some(changes' paths)`, format 4 only); `None` on older records still means all paths of the target. New tests in `tests/file_review.rs`: apply A{notes, added}, revert notes, whole Undo of A (records only `added.txt`), undo of the revert -> notes back in effect, `reverted_paths(A)` = [added], notes can be reverted again; and undoing a per-file revert alone. `HistoryEntry.reverted_paths` is now computed in one pass (`TaskHistory::all_reverted_paths`).
3. `replace_candidate` reads the previous candidate and manifest first (clone) and only then assigns; unit test `a_refused_replacement_changes_nothing` (task with a missing manifest stays CandidateReady with its candidate) plus a replace/restore round trip.
4. Restore failure: `agent_abandon_rejection` errors are no longer swallowed. New `RejectEnd::Stranded`; the actor drops the original and fails the task (`reject_restore_failed`), so actor and engine agree (task ended). Engine test `a_rejection_cannot_be_abandoned_once_the_task_has_ended`. The actor path itself is not reachable through the scripted agent, so only the engine half is tested.
5. Panic in the reject job: `run_reject` wraps the body in `catch_unwind`, shuts down the replacement's and the original candidate's scopes, then resumes the unwind (the actor ends the task Failed through `on_job_aborted`). The `drafts/reject-<uuid>` tree is removed by a drop guard (`RemoveOnDrop` in `controller/review.rs`). A process crash leaves the tree to the existing draft-store recovery. No panic-injection test.
6. Cheap items: diff reads go through a capped writer (`Capped`) so a damaged object cannot grow past its recorded size; results hold at most `MAX_DIFF_FILES` (2000) entries plus one trailing marker entry (empty `path`, both sides `NotShown::More { files }`), unit tests for both; the earlier `apply_blocked` reason is kept across a reject and no second "waiting for review" outcome row is pushed (a notice reports the result instead); `CandidateReady -> Validating` removed from the public transition table (`replace_candidate`/`restore_candidate` set the state directly; table test now asserts it is illegal).

Caller responsibilities (not changed): `agent_reject_files` captures the replacement while the controller lock is held (job thread, cancel checked only before and after), and `candidate_file_diffs` / `revision_file_diffs` read files on the calling thread after taking the controller lock only to copy the source descriptor. Call all of these off the UI thread. Candidate diffs carry no revision (review item 8, not requested): the UI should refresh its list after every reject and treat a "not a file this candidate changes" refusal as stale.

Additional hooks in concurrent files since the first report: `agent/actor/mod.rs` (`Active.rejecting` field), `agent/actor/turn.rs` (its initializer), `agent/actor/session.rs` (early branch in `stop`), `agent/actor/rows.rs` (one-pass reverted paths).

Verification (all in `desktop/`, toolchain 1.98.1): `cargo fmt --all --check` clean; `cargo clippy -p studio-engine -p fframes-studio -p studio-ui --all-targets -- -D warnings` clean; `cargo test -p studio-engine --no-fail-fast` all green (lib 205, file_review 10); `cargo test -p fframes-studio --test agent_workflow --test agent_workflow_ui --test agent_workflow_apply_gate --test agent_workflow_review` 61 / 3 / 13 / 4 pass; `cargo test -p studio-ui --test parity` 63 pass.

Status: DONE
Summary: Review findings 1 to 6 are fixed and tested (Stop returns to review of the original; every Undo records its covered paths; replace is check-then-assign; restore failure fails the task; panic cleanup; bounded diff reads); all acceptance commands pass.
Concerns/Blockers: none blocking. The Stranded actor path and panic cleanup have no actor-level test (not reachable through the scripted agent without fault injection).
