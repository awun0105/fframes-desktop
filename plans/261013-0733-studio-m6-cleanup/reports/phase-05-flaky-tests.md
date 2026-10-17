# Flaky tests and leftover limitations

All measurements and runs are local on Windows Server 2022. The owner set Windows-only verification for M6, so the Linux and macOS CI rows of the plan's validation were not run.

## A. Export-cancel parity hang

### Measurement

Temporary stderr timestamps went around each stage of `services::cancelling_an_export_leaves_no_file`, in the engine's export thread, in `ProcessTreeManager::shutdown`, and in the harness `settle` loop. Later probes covered gpui's test scheduler (each runnable's spawn site and run time), its `flush_effects`, and the window's dispatch tree. One representative run:

| Stage | Time |
| --- | --- |
| Displayed | 1.7 s |
| Export started | 2.4 s |
| Engine: compile, compiled, worker spawned | within 50 ms of the start |
| Test sees `rendering` | 34 s |
| Engine: cancel seen, child terminated, reader joined, shutdown done | within 6 ms of the cancel |
| `CancelExport` dispatch returns | 6 s later |
| `settle(500 ms)` | 30 to 84 s |

The engine side was instant in every run. All of the time was on the gpui test thread: `run_until_parked` returned in microseconds, while `advance_clock` took 1 to 9.5 s per 16 ms step. The scheduler probe named one task as the whole cost, the shell's preview wake loop (`shell/mod.rs`, the `preview_wakes` loop). Each of its polls ran for up to 27 s.

That loop drained about 13,000 coordinator wakes in a single test, each one quick (50 µs in `drain_preview`). 99.9% of them carried a thumbnail. Over the same run the shell rendered about 20,000 times and the cached timeline view rendered 4 times.

### Cause

The export panel replaces the editor column (`shell/layout.rs`), so while it is open the timeline is not in the window and its view has no dispatch node. `TimelineView::thumbnail` stored a delivered thumbnail in a single `pending_thumbnail` slot, and only `render` moved it into the cache. The timeline never rendered, so nothing reached the cache. `thumbnail_requests` therefore kept asking for every visible frame, the coordinator re-rendered them in turn, and each delivery notified the timeline, which made gpui redraw the whole window.

The result was an endless thumbnail and redraw loop for as long as the export panel or settings covered a displayed preview. This is a product bug, not a test artifact: the worker redraws the visible thumbnails without end in the app too, during a real export as well. In the test the loop ran inline on the test thread, which is why a passing run took 70 to 100 s. That is at or above the 90 s watchdog, which explains the CI "hang".

Hypotheses 1 (unbounded pipe-EOF joins), 2 (inline shutdown contention) and 4 (quit hook) were measured and contribute nothing: the export thread's terminate, join and remove, and every `shutdown`, finish within milliseconds. No deadline was added to the joins, because the plan allows fixing only what the measurement shows.

### Fix

`TimelineView::thumbnail` creates the image and inserts it into the cache on arrival. Images that leave the cache (evicted, or cleared on install and clear) wait in `retired` until the next render releases them through the window, as before. `pending_thumbnail` and `clear_cache` are gone.

`PumpMetrics.thumbnails` counts the thumbnails the coordinator delivered.

### Tests

- New: `services::thumbnails_arriving_behind_the_export_panel_are_kept`. A preview is installed behind the export panel. Its 12 visible thumbnails are cached, and over the next 500 ms no further thumbnail is delivered. Without the fix the count went from 43 to 486 in those 500 ms and the test failed.
- `services::cancelling_an_export_leaves_no_file`: 30 consecutive passes, 2.3 to 2.9 s each (was 70 to 100 s).
- The new test: 15 consecutive passes, at most 4 s.

## Intermittent tests

| Test | What was done | Local result |
| --- | --- | --- |
| `parity/agent.rs` `undo_in_the_prompt_undoes_the_typing_there` | `StudioShell::prompt_focused` and the harness `focus_prompt`, which dispatches `FocusPrompt` and waits until the prompt holds the focus. It replaces every `FocusPrompt` + `settle(50 ms)` pair (6 in `agent.rs`, 1 in `composer.rs`, 1 in `timeline.rs`). | 30 of 30 alone; improved, not proven fixed (see Unresolved) |
| `studio-engine/tests/build_sharing.rs` `cancelling_the_ui_compile_scope_leaves_a_tool_subscriber_running` | The fixed 150 ms sleep is replaced by a bounded poll for `Compiling { subscribers: 1 }`, then `killed == 0` is asserted. | 30 of 30 |
| `app/tests/agent_workflow/regressions.rs` `a_check_still_running_when_the_adapter_changes_is_redone_for_the_new_adapter` | `acp-agent.py` answers `initialize` only once `release-initialize` exists, when `hold-initialize` does. The test holds the check, changes the adapter, then releases the old check. A scratch run confirmed that the check stays `Checking` for 2 s under the hold. | 30 of 30, run again on the final code because the settings-change handler changed |
| `services::an_unmigrated_project_exports_only_the_default_whatever_was_saved` | Deleted in phase 4 with project migration. | N/A |
| `parity/agent.rs` `switching_projects_mid_turn_shows_nothing_of_the_old_turn` and its sibling | See below. The harness `open_path` waits for `busy == false`, dispatches `OpenProject` until the paths prompt appears (bounded), then chooses the path. Both switch tests and `checkpoints.rs` use it. | 30 of 30 each |

### The project-switch test

The plan's diagnosis was that the shell ignores Open while it is preparing. That does not match the macOS failure. An ignored Open would leave no paths prompt, and `choose_path` would then panic with "no pending paths prompt". The CI log shows a wait timeout instead. The test never failed locally, so the cause is unproven. `open_path` removes the one race that can be seen by reading, an Open dispatched while the shell is still busy. If macOS fails again, the timeout message now names the stage it waited for.

## Limitations

| Item | Result | Test |
| --- | --- | --- |
| Plan updates are not coalesced | Already done in the driver: `EventQueue` replaces an unread plan in place and keeps its sequence number. No code change. | `driver::events` `a_burst_of_plans_yields_one_unread_plan_holding_the_latest`: one plan, a delta and 99 more plans drain as 2 events, the plan holding step 99 under the first sequence number. A plan pushed after the drain is a new event. |
| Workspace `image` feature (jpeg, gif, webp) | Stays. `studio-engine` decodes JPEG, GIF and WebP attachments into PNG, so it is not used by `studio-harness` alone. Cargo unifies features across the binaries that share the workspace, so scoping it per crate would change nothing. | None |
| A plan or command update before the session id is stored is accepted | Owner decision, left documented. `on_event` stores such an update only in `composer.plan` and `composer.commands`, which are display-only; no session, log or manifest state is touched. | None |
| Adapter change clears the command list (unit test only) | The workflow harness now runs a second adapter. | `agent_workflow::composer` `the_commands_of_one_adapter_are_not_shown_for_another` |
| Startup offer skips the stored-capability gate | Fixed. `offer_saved_session` uses `resume_blocker`, as opening a session does. The notice names the reason (another provider, no wire session, or no resume support). The actor applies the settings before making the offer, so the configured provider is known. The offer depends on the provider now, so a settings change makes it again while the startup offer is still shown. | `agent_workflow::sessions` `the_session_offered_at_startup_is_resumable_only_when_its_agent_can_resume`: not resumable, with the reason, without `load-session`; resumable with it. `the_session_offered_at_startup_follows_an_adapter_change`: refused after a switch to another provider, offered again after the switch back. Both fail without their fix. |
| New chat while the legacy log is served | N/A: the legacy log and its migration were removed in phase 2. | None |
| Listing sessions reads every manifest | Measured first, with 500 sessions in a release build: 60 to 72 ms with 20 rows each, 164 to 243 ms with 200 rows each. Listing reads every log in full, not only manifests. That is over 50 ms, so `list_sessions` now keeps each summary in memory and reuses it while the size and modification time of the session's log, manifest and meta files are unchanged. The cache is keyed by stamp rather than cleared on create, rename and delete, because every appended row changes the summary too. A repeated listing takes 31 to 40 ms; the first one costs the same as before. A log that failed to open is not cached, so it is read again next time. The cache lives for the process: about 8 KiB of prompt text per session at most, about 4 MB for 500 sessions. A file rewritten to the same size within one modification-time tick would go unnoticed; NTFS has 100 ns resolution. | `session_history` `a_listing_reflects_every_change_since_the_previous_listing`: an appended row, a new session, a moved current pointer and a deleted session all show in the next listing. Fails when the cache ignores the stamp. |

## Composer base

The 2026-10-14 decision stands. The reasons that `ComposerInput` existed for are gone. The question comes back only when `@` mentions land, and only if the plan owner raises it.

## Leftover edits in the main checkout

`review_publication.rs` and `tests/support/tx_fixture.rs` add `for_each_boundary`. It replays each crash and I/O-failure boundary of a matrix on up to 8 scoped threads, pulling from an atomic counter.

Verdict: safe to adopt, not adopted here. Each replay builds its own temporary fixture and controller. Neither file has statics, environment changes or working-directory changes. The only child processes are `git` and `cmd`, each run in its own fixture. A panic in one replay stops that worker. The other workers finish, and the scope then fails the test. The edits are the owner's uncommitted work in the main checkout. M6 leaves them there to commit as a separate change; the speed-up was not measured.

## Checks

Run on 2026-10-17, Windows Server 2022, after the review fixes:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test -p studio-ui --lib` | 473 passed |
| `cargo test -p studio-engine` | 539 passed |
| `cargo test -p fframes-studio` | 278 passed |
| `cargo test -p studio-harness` | 185 passed |
| `cargo test -p studio-ui --test parity`, 3 consecutive runs | 79 passed each, 37 to 38 s |

The plan asks for 3 full-suite passes per intermittent test. Parity has 3 on the final code. The engine and app suites have 1 on the final code and 1 earlier on the code before the review fixes.

A code review of the diff found no blocking issue. Its four medium findings were fixed before these runs: the startup offer is made again after an adapter change, unreadable summaries are not cached, the last `FocusPrompt` + `settle` pair (in `timeline.rs`) uses `focus_prompt`, and this section.

## Unresolved

- The cause of the macOS project-switch failure is unproven (see above).
- Linux and macOS CI were not run, by the owner's decision for M6. The 3 consecutive CI runs in the plan's validation are still open.
- One of 14 full parity runs had a wait timeout in `undo_in_the_prompt_undoes_the_typing_there`, on the first run after a rebuild. The cause is unexplained; the other 13 runs, and the 30 runs of the test alone, passed.
