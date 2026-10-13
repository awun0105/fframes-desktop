---
phase: 3
title: "Parity gpui tests"
status: pending
priority: P1
effort: "3-5d"
dependencies: [2]
---

# Phase 3: Parity gpui tests

## Goal

Every item on the [parity list](./plan.md#parity-list) has a `#[gpui::test]` that drives the real views through actions and public methods, not pixel bounds, so M2 to M5 can move code without losing behavior.

## Context links

- Existing test support: `desktop/app/tests/support/` (`workflow_world.rs`, `acp-agent.py`), `desktop/app/tests/malformed-preview-worker.py`, `desktop/app/tests/stalled-worker.py`, `desktop/crates/studio-test-support`
- Fixture project: `desktop/app/tests/fixtures/`
- Zeron's test style: `~/code/zeron/crates/ui/src/*_tests.rs` (for example `composer_dictation_tests.rs`)

## Tasks

1. Scout how each parity behavior is triggered today (action, click handler, method on `StudioShell`, `TimelineView`, `ConversationPanel`, `PresetPanel`, `SetupView`) and where its result is observable (entity state, emitted event, file on disk).
2. Add `desktop/app/tests/parity/main.rs` (one `[[test]] name = "parity"`) with a shared harness: open the fixture project in a `TestAppContext` window, real engine, fake preview worker, fake ACP agent. The suite moves to `desktop/crates/studio-ui/tests/parity/` in M2 phase 4, with the views it drives.
3. One module per parity item (`presets.rs`, `setup.rs`, `canvas.rs`, `services.rs`, `checkpoints.rs`, `audio.rs`, `timeline.rs`, `transport.rs`, `agent.rs`). Assert on state and events, never on element bounds. Tests drive the app through actions dispatched on the window and assert on controller and engine state, emitted events and files on disk (on `AppState` after M2 phase 3). No test names a view type (`StudioShell`, `TimelineView`, `ConversationPanel`, `PresetPanel`, `SetupView`), because M2 replaces those views with Cap's layout and the tests must not be rewritten with the code they guard; only the harness that opens the window knows the root view.
4. Gated items assert the real outcome on Linux and "blocked, with reason" on Windows and macOS, using one helper so M3 phase 3 flips them in one place.
5. The agent item needs writer ownership: use the same fixture path the existing tests use (`tests/support/workflow_world.rs`), not a hard-coded `ProcessGroupContained` in production code.
6. Where a behavior is only reachable through a private click handler, expose it as a gpui action on the view (an action is also what M2 keybindings need). Do not add test-only methods.
7. Run on Windows, Linux and macOS CI. Unix-only fixtures (process groups, `libc::kill`) get Windows equivalents or are scoped to the Linux leg with a stated reason.

## Todo

- [ ] Scout triggers and observables
- [ ] Harness
- [ ] presets, setup, canvas
- [ ] services, checkpoints, audio
- [ ] timeline, transport, agent
- [ ] Green on three CI platforms

## Success criteria

`cargo test -p fframes-studio --test parity` passes on Windows, Linux and macOS; deleting the body of any parity behavior makes at least one test fail (checked once per parity module, nine in all).

## Risks

- Audio output switching needs a device: test the selection logic against the audio service's device list abstraction, and leave real playback to manual checks.
- Test-only code creeping into views: limited to new actions, which are product features.
