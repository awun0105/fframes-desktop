---
phase: 3
title: "Engine handle and typed events"
status: in-progress
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

- [ ] Engine work off the main thread; adapter probe cache with test

- [ ] Scout controller surface, poll reads and mutex holders
- [ ] Proto types with session ids
- [ ] `EngineHandle` and `ProjectSession`
- [ ] `AppState` with session filtering
- [ ] Agent and preview polls and UI mutex removed
- [ ] Playback tick and epoch path preserved

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
- **`AppState`** (studio-ui, one gpui entity per window) is fed by a `cx.spawn` task awaiting the event receiver. It holds the current `SessionId`, the project snapshot and the command in flight. It drops events from any other session and re-emits the rest to the shell.- **Session filtering.**
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
