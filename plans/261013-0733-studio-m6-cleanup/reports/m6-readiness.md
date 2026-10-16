# M6 readiness (2026-10-17)

Checked against `df53e56` on `feat/studio-m6`. The plan is ready to execute; one item (the export-cancel hang) needs measurement before any fix.

## What changed in the plan

- Phase 1 rewritten from a scout task into a file-by-file removal list with line counts, a note of what is already gone, and the production probes that stay. Its old grep criterion was wrong, because `start_agent_telemetry` and `bounds_probe` are kept on purpose; the criterion now lists the allowed sites.
- Phase 2 now lists the exact stale docs with line numbers. `AGENTS.md` needs only a `/desktop` entry and a pointer, since the M4 timing text is already there.
- Phase 3 added for everything M5 and M5a handed over.
- `plan.md`: branch `feat/studio-m6`, effort 4-5d (was 3-4d), `blockedBy` emptied because M4 and M5 are merged, three phases, new success criteria.

## Scouting counts

- 82 case-insensitive "spike" hits in 19 files under `desktop/` (plans, target, node_modules excluded); none in `.github`.
- Five files delete outright: `app.rs` 662, `agent_spike.rs` 372, `selection_spike.rs` 405, `preview_element.rs` 108, `ui/mod.rs` 4 (1,551 lines).
- `qualify_`, `presentation_stress`, `stress_metrics`, `qualification::`: zero hits (already gone).
- `worker_roundtrip.rs` (83 lines) depends on `selection_spike` and feeds a CI step the release gate keys on, so it is ported, not deleted.
- Five intermittent tests with a shared cause: a wait on a signal that does not imply the precondition.
- Seven handed-over limitations, one composer-base question.

## Leftover edits in the main checkout

Read from `git diff` only; nothing touched. They add `for_each_boundary` to `tests/support/tx_fixture.rs` (a scoped thread pool of up to 8 workers) and use it in `review_publication.rs` to run the crash-at-boundary and I/O-failure-at-boundary replays in parallel instead of one by one, because those matrices were the slowest tests. They are coherent and apparently complete for that one file (both loops converted, the helper is shared through the existing fixture include). They are unverified: nothing shows they compile or pass, and the other every-boundary matrices (`task_recovery.rs`, `preset_state.rs`, `project_migration.rs`) are still sequential. Verdict: adopt, in phase 3 as its own commit, only after `cargo test -p studio-engine --test review_publication` passes 10 times in a row and `just clippy` is clean (the closure must be `Sync`, so the replays must share no state). Drop them if replays share a process-wide resource and flake. They are a speed change, not a fix, so they stay out of the flaky-test commits. The plan owner keeps the final call.

## Decisions for the plan owner

1. `worker_roundtrip.rs` port: keep one source-anchor check through `request_elements` and hit-test, or accept losing the `code_snippet` coverage the spike had.
2. Which limitations to fix versus leave documented. Suggested: fix the startup capability gate, the legacy-migration notice and plan coalescing; leave the session-listing index until measured; record the plan-before-session-id window as display-only.
3. Composer base: no work unless you say so (2026-10-14 decision stands; revisit with `@` mentions).
4. Bundle identifier `studio.fframes.spike` in the packager: rename, or keep if a shipped release depends on it.
5. Whether the user runs the release dry run with `consumer_release_enabled`, and the Windows hand checks (they need the RDP session).
6. Adopt or drop the leftover edits (verdict above).

## Unresolved questions

- Where the 60 to 90 s of a passing cancel test goes. Needs the timing run in phase 3 step 1; no cause is claimed until then.
- Whether the hang reproduces locally or only on CI.
- Whether the Bash tool failure (`ENAMETOOLONG`) that blocked git in this session also affects execution sessions.
- Whether `app/Cargo.toml` dependencies are removable beyond `async-channel`, `smallvec` and `image`; needs a build with `--all-targets`.
- Any open items in M5a phase 4 beyond the macOS real-CLI checks; its report was not read in full.
