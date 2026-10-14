---
phase: 3
title: "Engine handle and typed events"
status: done
priority: P1
effort: "4-5d"
dependencies: [1]
---

# Phase 3: Engine handle and typed events

## Goal

The UI talks to the engine only through `studio-proto` commands and events. One `AppState` entity applies events on the main thread; agent and preview polling and the shared controller mutex leave the UI.

## Design source

Zeron's `crates/ui/src/state.rs` (`AppState` 751, `EngineHandle` 275, `spawn_watch` 3173) is a model, not a port: its in-process engine still runs an RPC service and JSON subscriptions. Write `EngineHandle` and typed channels new, following the same shape. Runtime setup copies Zeron's `gpui_tokio::init_from_handle` (`crates/ui/src/lib.rs:171`).

## Tasks

1. Scout: every `Controller` field the shell reads, every mutation it calls, what `poll_agent`, `poll_preview` and `poll_audio` read (around `studio_shell.rs:1604,1807,2191`), and every holder of the `Arc<Mutex<Controller>>` (`studio_shell.rs:66,120`, `agent_workflow.rs:290,334,903`, `agent_workflow/jobs.rs:143,262,521`, `conversation_panel/host.rs:612`, `preset_panel.rs:409`).
2. Define in `studio-proto`: `EngineCommand` (one variant per mutation found), `EngineEvent`, and snapshots (`ProjectSnapshot`, `PreviewSnapshot`, `AudioSnapshot`, `BuildSnapshot`, `WorkflowSnapshot`, and `SetupSnapshot` for the setup and settings screens: adapter detection from the cache in pattern 2, provider profiles and app settings such as theme and preview quality; named 2026-10-14 after spike S4), every event tagged with the project session id.
3. In `studio-engine`, add `EngineHandle` and `ProjectSession` with the lifetimes in [the plan](./plan.md#state-lifetimes). The agent actor's job threads keep their existing locked access to the controller inside the engine; the UI gets commands only.
4. In `studio-ui`, add `AppState` fed by watch tasks; it ignores events whose session id is not current and resets on project switch.
5. Replace `poll_agent`, `poll_preview` and the UI's controller mutex with `AppState` reads and commands.
6. Keep the 16 ms playback tick and its audio path intact: the tick still reads the audio output snapshot and calls `PlaybackClock::accept_output` with its epoch check (`studio-engine/src/playback_clock.rs`). Only move the call behind the engine handle; M4 phase 6 replaces the tick.

## Reference patterns (Cap, Zeron)

From [the Zeron report](../reports/research-261013-1749-zeron-agent-control.md) and [the Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md); see the roadmap's pattern map.

1. Engine work runs on the tokio runtime (`gpui_tokio`) or the background executor, and results reach views only through `AppState`. Nothing that enumerates audio devices, probes adapters or reads project files runs inside a render or on the main thread; Cap measured 180 ms of blank window from inline device enumeration.
2. Scout how often the setup screen probes adapters today. Cache adapter detection in the engine, keyed by canonical path, modification time and size, failures included, with a timeout (Zeron uses 2 s, `executable.rs:146-289`). Snapshots read the cache; publishing a snapshot never re-probes. Zeron's `ListHarnesses` re-runs detection on every call although a comment says it does no subprocess work; that is the bug to avoid.
3. Test: two consecutive setup snapshots run the probe once; changing the adapter file re-probes.

## Todo

- [x] Engine work off the main thread; adapter probe cache with test
- [x] Scout controller surface, poll reads and mutex holders
- [x] Proto types with session ids (ids only; payload types live in the engine, see the design decisions)
- [x] `EngineHandle` (no separate `ProjectSession` type, see the outcome)
- [x] `AppState` with session filtering
- [x] Agent and preview polls and UI mutex removed
- [x] Playback tick and epoch path preserved

## Success criteria

Parity tests pass, including a new test that switches projects mid-turn and asserts no event from the old session reaches the new `AppState`; `grep -rn "Mutex<Controller>\|poll_agent\|poll_preview" desktop/crates/studio-ui/src desktop/app/src` is empty.

## Risks

- Hidden ordering between polls (agent apply triggering a rebuild): discrete events travel on one ordered channel; assert the order in a test.

## Design decisions (2026-10-14, after scouting)

- **Where the types live.** Commands and events carry engine-owned payloads: `StagedPreview` (owns a worker), `ReadyPreview`, frames, `Promotion`, `host::Opened`, and the agent workflow handle. `studio-proto` cannot hold them without depending on `studio-engine`, which would be a cycle. So `EngineCommand`, `EngineEvent`, `EngineHandle` and the snapshots live in `studio_engine::engine`. `studio-proto` holds the plain ids `SessionId` and `CommandId`.
- **Moved out of the app as gpui-free logic.** Each move is a plain relocation with no re-export layer. Call sites and tests import from the engine.
  - `Backend` becomes `engine::backend`.
  - From `setup_view`: the SDK install functions and manifest resolution become `engine::sdk_setup`. The app version is passed in, because `env!("CARGO_PKG_VERSION")` differs between the app (0.1.2) and the engine (0.1.0).
  - The preset logic in `preset_panel.rs` (lines 1-520) becomes `engine::presets`.
  - `ProjectPresentation` moves into the engine.
  - `install_fence`, `presentation_verdict` and `PresentationVerdict` become `engine::fence`. Because `install_fence(&Mutex<Controller>)` is engine-internal locking, its signature stays the same for `promotion_handoff`.
- **Lifetimes.** `EngineHandle` (`Clone`) owns the backend, the shared build service, the event sender and one monitor thread. The engine opens `ProjectSession`, which holds the controller, its `SessionId` and the agent workflow. The engine also closes the workflow before the controller it shares, which `take_workflow_for` and `start_agent_open` used to do in the shell.
- **Channels.** Discrete events (command finished, presentation refreshed, agent opened, promotion handoff, source changed, teardown problem) travel on one ordered unbounded `async_channel`. Frames and thumbnails stay in the preview coordinator's latest-value mailbox. Snapshot changes do too: the workflow snapshot is read on wake. A wake is coalesced through an `AtomicBool`, so at most one is queued.
- **Polling that moves to the engine, where it is allowed.** One monitor thread does three things. Every 750 ms it checks `changed_hint` and sends `SourceChanged`; the UI still skips the refresh while a command is in flight. Every 500 ms it retries `set_build` on the workflow. It also drains teardown reports. Agent snapshot changes come from a thread that blocks on `AgentWorkflow::wait_changed`. Preview mailbox changes come from a listener the coordinator calls.
- **Synchronous by design.** The preview install commit in `render()` keeps a synchronous, nonblocking source fence: `EngineHandle::with_install_fence(|state| ...)` does `try_lock` on the backend and the controller and refuses while `changed_hint` is set. An event round trip would break the guarantee that "validate, coordinator commit, preview-state install" happens atomically.
- **Unchanged until later phases.** The 16 ms playback tick stays (M4 phase 6). The panel keeps calling `AgentWorkflow` methods directly: they are actor messages, not the controller mutex, and phase 6 rewrites the panel against snapshots. The panel no longer polls; it refreshes on workflow wakes and on page-load completion.
- **Adapter probe cache.** Zeron caches a `--version` run per executable identity (`executable.rs:146-289`). fframes' detection (`detect_agents`, `scan_agents`) only checks that files exist and never spawns. So the one per-executable process probe is the ACP readiness probe (`run_probe`).
  - Today it runs only on an explicit "Check adapter", on a turn-on save, and on the actor's re-check after settings changed mid-probe. Opening a project never launches an agent, and no snapshot probes.
  - `AdapterProbeCache` wraps `run_probe`. Its key is the canonical resolved executable, modification time, size, arguments and auth method, and failures are cached too.
  - Automatic paths read through the cache: the stale-generation re-check and the setup snapshot, which only peeks and never probes.
  - An explicit check always probes and replaces the entry, because sign-in state is not in the key.
  - The 30 s init timeout stays, because real adapters start through `npx`.
- **`AppState`** (studio-ui, one gpui entity per window) is fed by a `cx.spawn` task awaiting the event receiver. It holds the current `SessionId`, the project snapshot and the command in flight. It drops events from any other session and re-emits the rest to the shell.
- **Session filtering.**
  - Every session-scoped event is stamped where it is created, not where it is forwarded. That covers handoffs, `Opened`, workflow wakes, preview wakes and teardown reports. `HostInbox` entries carry the session of the workflow that queued them.
  - `CommandFinished` is never filtered, because it is what moves `AppState` to a new session.
  - At dispatch of a command that replaces the project, the UI marks the outgoing session as leaving and detaches the panel synchronously, as `take_workflow_for` does today. From then on, events from that session are dropped, even before the command completes.
  - Reopening the open folder only reconciles and keeps its `SessionId`.
- **Also moved.**
  - Export: `freeze_export_source` and the export thread go behind `EngineHandle::start_export`, and progress arrives on an awaited channel instead of the per-tick `try_recv`.
  - Thumbnail requests are sent when the timeline's visible set or the play state changes. An empty request while playing still cancels the pending ones.
  - A wake that arrives while a command or the preview gate is in flight is remembered. The mailbox is drained again when either clears.
- **Thread hygiene.**
  - The workflow waiter exits once the snapshot reports closed.
  - The monitor holds a `Weak` to the engine, uses `try_lock` on the backend, and exits when the handle is gone.
  - The coordinator calls its listener after releasing the mailbox lock.
- **Spike.** Under `TestAppContext`, with the parity harness's `advance_clock` plus `run_until_parked` loop, a `std::thread` sending on an `async_channel` wakes an entity's `cx.spawn` task, and the values arrive in order. So `AppState` can be fed by awaiting the channel.

## Outcome (2026-10-14)

Done on Windows; Linux and macOS run in CI on the next push.

- **What the window does now.**
  - `studio_engine::engine::handle::EngineHandle` owns the backend on its own `studio-engine` thread. Commands, presentation refreshes, preview news and agent-workflow opens run there one at a time, in the order sent. Each answer is one `EngineEvent` on one ordered `async_channel`.
  - `studio_ui::app_state::AppState` (one gpui entity per window) awaits that channel, tracks the current `SessionId` (taken from each `CommandFinished`, which carries the session even when its presentation failed) and the command in flight, drops session-scoped events of any other session or of the session being left, and passes the rest to the shell. `CommandFinished` and `CleanupProblem` are never filtered.
  - The shell no longer holds the backend, a controller, or an epoch counter. `dispatch` sends and returns; `CommandFinished` replaces the epoch check. `Presentation.controller` is gone.
  - The install commit in `render()` goes through `EngineHandle::with_install_fence`, which never blocks.
  - The export runs behind `EngineHandle::start_export`. Its progress arrives on an awaited channel; `poll_export` is gone. The project is held only while the source is frozen.
  - Agent-workflow opens go through `EngineHandle::open_workflow`. The engine opens only for the session still open and replays the log on its own thread.
  - `grep -rn "Mutex<Controller>\|poll_agent\|poll_preview" desktop/crates/studio-ui/src desktop/app/src` is empty.
- **Wakes instead of ticks.**
  - Preview: `PreviewCoordinator::set_listener` is called once per drain when news (compiled, ready, frame, thumbnail, error) waits. The shell drains on that wake. News is held back while a command, the preview gate or a native dialog is in flight, so the shell drains again when any of them finishes; a dialog closed without a command used to strand it. The gate itself (`complete`/`reconcile` on the controller) runs on the engine thread (`EngineHandle::preview_news` answered by `PreviewGated`).
  - Agent: `HostInbox::set_listener` hears the first wake after each `take_wake`. The workflow already calls the inbox's `notify` on every snapshot publish and evidence image, so no separate `wait_changed` thread was needed. A finished history page wakes the panel through `ConversationPanel::set_wake`, and scrolling asks for the next page from the list's scroll handler.
  - Source changes and cleanup problems: a `studio-engine-monitor` thread (`Weak`, `try_lock` only) sends `SourceChanged` every 750 ms while `changed_hint` is set; the shell dispatches `Refresh` when not busy. It also sends the newest teardown problem.
  - A refused `set_build` is retried on the next agent wake (a task ending publishes a snapshot) instead of every 500 ms.
  - Still on the 16 ms tick, by design: playback, the playhead forwarded to the workflow, thumbnail requests (sent only when they changed, and re-sent after a thumbnail arrives, a coordinator is replaced or an install commits), and the 8 s stranded-acceptance safety net.
- **Where this differs from the design above.**
  - There is no `ProjectSession` type and no snapshot structs. The session is a `SessionId` on the backend, `Presentation` remains the project snapshot, and the existing `Command` is the command type. The shell still holds the agent workflow handle for the panel until phase 6; the engine closes it, before the command that leaves its session runs.
  - `HostInbox` entries are not stamped with a `SessionId`. They keep the existing per-open serial check (`agent.attached`). They never travel on the engine channel.
  - No `gpui_tokio` runtime: engine work runs on plain threads, and results reach the UI over `async_channel`.
  - The audio output read in the tick (`accept_output`) did not move behind the handle. The audio service is owned by the window, and M4 phase 6 replaces the tick.
- **Tests.** 13 new tests:
  - six `AppState` tests (stale session dropped, a finished command never dropped and moving the window, one without a presentation still moving it, the session being left says nothing more, a refused switch keeps the session, ordered delivery);
  - four engine-handle tests (preview news answered with the open session, an external source change reported, a workflow opened only for the session still open, an export of a session no longer open failing with a reason);
  - the coordinator listener test;
  - the inbox listener test;
  - the parity test `frames_keep_arriving_after_a_cancelled_dialog`, which failed before the drain after a dialog closes.
  The parity test `switching_projects_mid_turn_shows_nothing_of_the_old_turn` stays green.
- **Verification.**
  - `cargo test --workspace`: 841 passed, 0 failed, 14 ignored, up from 828.
  - `cargo clippy --workspace --tests -D warnings` is clean on Windows. Cross-target clippy cannot build ring/sqlite here.
  - The Unix-gated app tests type-check with their gates stripped, with only Unix-API errors (`gated_check.sh`). They do not run here; the Linux CI run is the first execution of the rewired handoff, gate and preset paths in those suites.
  - `candidates_above_the_preview_cap_validate_and_stage_at_the_effective_scale` failed once under full-suite load with a fake-worker audio read error ("failed to fill whole buffer"), then passed 3/3 alone. That code is untouched here.
