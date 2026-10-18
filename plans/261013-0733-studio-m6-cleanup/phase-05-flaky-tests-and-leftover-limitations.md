---
phase: 5
title: "Flaky tests and leftover limitations"
status: done
priority: P1
effort: "2d"
dependencies: []
---

# Phase 3: Flaky tests and leftover limitations

## Context

Everything M5 and M5a handed over lands here. Independent of phase 1; touches test files and a few engine files that phases 1 and 2 do not. Run it first or in parallel with phase 1 if two people work.

Parity harness facts (`desktop/crates/studio-ui/tests/parity/harness.rs`): `wait` advances the clock, runs until parked and sleeps 5 ms; `settle` is 15 steps of 16 ms each followed by 2 ms; a `Watchdog` exits the process with 101 after `WAIT + 30 s`, where `WAIT` is 60 s, so after 90 s. In gpui tests background tasks run inline on the test thread, so a blocking background task blocks the frame loop.

### A. Export-cancel parity hang

Symptom: `services::cancelling_an_export_leaves_no_file` hung in 1 of 4 runs without the review engine and 2 of 4 with it, and failed the self-hosted Linux job of CI run 38917158146 once. Alone it passes in about 91 s (`studio-m5-agent-ux/reports/merge-verification.md`), which is itself at the watchdog threshold, so a passing run already spends 60 to 90 s somewhere. The cause is not proven; the plan owner agreed it is fixed in M6, but only after it is proven.

Path under test: `cancel_export` (`studio-ui/src/shell/engine_events.rs:885`) calls `request_cancel`, spawns `processes.shutdown(Duration::ZERO)` on the background executor and sets `Cancelled` at once. The engine's `studio-mp4-export` thread (`engine/handle.rs:245`, `services/export_service.rs:119`) polls every 25 ms, calls `terminate_verified(500 ms)`, then `join_progress_reader`, then removes the temporary file. The fake worker (`studio-engine/tests/support/fake-preview-worker.py`, `--export-v1`) prints one progress line then loops on `sleep(0.05)` while `export_hang` is set.

Ranked hypotheses:

1. Unbounded pipe-EOF waits: `join_progress_reader` (`export_service.rs:544-556`) spins until the stderr reader ends and `export_capabilities` (`:396-455`) joins the stdout reader with no deadline. A surviving descendant, or a handle inherited by a concurrently spawned child on Windows, delays EOF. Most likely: explains slow passes and rare hangs.
2. The inline background `shutdown` blocks the gpui test thread while the export thread contends for the child lock in `terminate_verified`; `shutdown_verified` (`studio-bootstrap/src/process/mod.rs`) is bounded (2 s verify, 10 ms poll) and no lock inversion was found by reading, so this explains slowness more than a hang.
3. A slow inline task in `settle` or `run_until_parked`.
4. The quit hook (`studio-ui/src/shell/mod.rs:319`: workflow close, processes shutdown, builds close, audio join, preview close, owner shutdown) running inline during drop; relevant to the unmigrated test below.

## Requirements

- Prove the cause of A with a measurement before changing behaviour; the fix addresses only what the measurement shows.
- After the fix, the cancel test passes in seconds and 30 consecutive runs pass on Windows and 3 consecutive self-hosted Linux CI runs pass.
- Each intermittent test below waits on a state that implies its precondition. Rule: fakes hold the state under test until the test releases it; positive preconditions wait on a stable post-condition with a bounded poll; a fixed sleep is allowed only for a negative assertion after a proven sync point.
- Each limitation below is fixed with a test, or recorded as an owner decision in the readiness report and left documented. Nothing here changes a user decision.
- The composer-base question is recorded, no work: the 2026-10-14 decision stands, the `ComposerInput` reasons are gone, revisit when `@` mentions land, and only if the plan owner says so.

## Files

Hang: `studio-ui/tests/parity/services.rs`, `studio-ui/tests/parity/harness.rs`, `studio-engine/src/services/export_service.rs`, `studio-ui/src/shell/engine_events.rs`, `studio-bootstrap/src/process/mod.rs` (only if the proof points there).

Intermittent tests:

| Test | Cause | Fix |
|------|-------|-----|
| `studio-ui/tests/parity/agent.rs:81` `undo_in_the_prompt_undoes_the_typing_there` | `settle(50ms)` after `FocusPrompt` without checking the prompt is focused; telemetry has no focus key | expose a prompt-focused key in shell telemetry and wait on it (or reuse the `click_id` two-measurement rule from `parity/composer.rs:73`) |
| `studio-ui/tests/parity/agent.rs:256` `switching_projects_mid_turn_shows_nothing_of_the_old_turn` (failed on macOS CI) | waits on `busy == false` after `send_brief`, a coarse flag; the shell ignores Open while preparing | wait for the turn to be running and the shell not preparing, then open |
| `studio-engine/tests/build_sharing.rs:639` `cancelling_the_ui_compile_scope_leaves_a_tool_subscriber_running` (macOS) | fixed `sleep(150ms)` after `cancel_build`, which fences and detaches asynchronously | poll, bounded, until the subscriber count is 1, then assert `killed == 0` |
| `app/tests/agent_workflow/regressions.rs:493` `a_check_still_running_when_the_adapter_changes_is_redone_for_the_new_adapter` (macOS) | waits for the transient `AdapterReadiness::Checking`, which can pass before it is sampled | make the scripted peer hold the check until released, as done for `codex_wire.rs` `capture_waits_for_a_command_that_outlives_its_turn` |
| `studio-ui/tests/parity/services.rs:194` `an_unmigrated_project_exports_only_the_default_whatever_was_saved` | starts an export and relies on drop to reap it; failed twice in runs that also stalled on exports | cancel and await the export explicitly before the test ends |

Limitations (driver report `studio-m5-agent-ux/reports/phase-02-03-driver-engine.md`, session report `phase-05-lifecycle.md`):

| Item | Step |
|------|------|
| Plan updates are not coalesced | coalesce in the driver event path so a burst yields one row update; test with a burst |
| Workspace `image` feature enabled for all of `desktop/` (jpeg, gif, webp) | scope it to `studio-harness` if the other crates do not decode those formats; check each user first; otherwise record why it stays |
| A plan or command update before the session id is stored is accepted | display-only today; add a test pinning that no state besides display is touched, or drop the update; owner decision |
| Adapter change clears the command list, covered by a unit test only | add an integration case once the workflow harness has a second adapter; until then keep the unit test |
| Startup offer of the saved session skips the stored-capability gate (read-only fallback still applies) | apply the gate to the offer; test with a session whose capability is gone |
| New chat while the legacy log is served hides the legacy conversation if its migration keeps failing | show a notice and keep the legacy log reachable; test with a failing migration |
| Listing sessions reads every manifest | add a small index or cache invalidated on create, rename and delete; measure with 500 sessions first and skip if listing is under 50 ms |

Leftover edits (main checkout, not committed): `studio-engine/tests/review_publication.rs` and `tests/support/tx_fixture.rs` run each crash or I/O-failure boundary replay in parallel through a new `for_each_boundary`. See the readiness report for the verdict.

## Steps

1. Hang proof: add temporary stderr timestamps around each stage of the cancel test (start export, first progress, `cancel_export`, status Cancelled, terminal event, temp file removed, teardown). Run alone, then 20 times in a loop, on Windows. Record per-stage times in the report. The 60 to 90 s that a passing run spends is the first thing to locate.
2. Decide by the evidence: unbounded EOF wait gives a deadline on both joins (and a recorded error, not a hang); inline-teardown contention gives teardown on a real thread (not the test-inline executor); a slow inline task gives a bounded step. Remove the temporary markers.
3. Fix the five intermittent tests with the rule in Requirements; run each 30 times alone and in the full suite.
4. Work the limitation table row by row, one commit per row.
5. Decide on the leftover edits (verdict in the report). If adopted, commit them separately from the flaky fixes.

## Validation

- Cancel test: 30 consecutive passes on Windows, each under 15 s; 3 consecutive green self-hosted Linux runs; the watchdog never fires.
- Each intermittent test: 30 consecutive passes alone and 3 full-suite passes; macOS CI green on 3 consecutive runs for the two macOS tests.
- `cargo test -p studio-ui --test parity`, `cargo test -p studio-engine`, `cargo test -p fframes-studio`, `just clippy`.
- A new test per fixed limitation, named for its behaviour.

## Risk

- High: the hang cannot be reproduced locally (1 in 4 to 1 in 2 in CI only). Mitigation: loop on CI via a temporary workflow dispatch with the markers; if still unproven after two rounds, record it as unresolved and keep the watchdog.
- Medium: bounding a join leaves a reader thread behind. Mitigation: detach it only after the child is verified dead, and log it.
- Low: parallel boundary replays (leftover edits) share a process-wide resource and flake. Mitigation: the verdict conditions.

## Rollback

One commit per item; revert the item. The watchdog stays in all cases.
