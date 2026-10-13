---
phase: 3
title: "Engine handle and typed events"
status: pending
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
2. Define in `studio-proto`: `EngineCommand` (one variant per mutation found), `EngineEvent`, and snapshots (`ProjectSnapshot`, `PreviewSnapshot`, `AudioSnapshot`, `BuildSnapshot`, `WorkflowSnapshot`), every event tagged with the project session id.
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
