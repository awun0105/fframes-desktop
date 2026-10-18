---
phase: 3
title: "Parity gpui tests"
status: complete
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

- [x] Scout triggers and observables
- [x] Harness
- [x] presets, setup, canvas
- [x] services, checkpoints, audio
- [x] timeline, transport, agent
- [x] Mutation check, one per module (results below)
- [x] Green on three CI platforms: self-hosted Windows and Linux (run 38503744645) and macos-15 (run 38503812288, with the dependency check), all at `2f527d3`

## Success criteria

`cargo test -p fframes-studio --test parity` passes on Windows, Linux and macOS; deleting the body of any parity behavior makes at least one test fail (checked once per parity module, nine in all).

## Risks

- Audio output switching needs a device: test the selection logic against the audio service's device list abstraction, and leave real playback to manual checks.
- Test-only code creeping into views: limited to new actions, which are product features.

## Progress

**State (2026-10-14):** 30 tests in `desktop/app/tests/parity/` pass on Windows (this VM, about 20 s). The rest of `cargo test -p fframes-studio`, clippy with `-D warnings` and `cargo fmt --check` pass too. CI passed on self-hosted Windows and Linux x86-64 and on macos-15 at `2f527d3`. On Linux x86-64 the preset and agent Apply tests take the branch where mutation is allowed.

### How the tests reach the app

- **Environment seam:** `StudioShell::with_environment(StudioEnvironment { app_data, sdk_home, builds, ownership })`. `StudioShell::new` passes `StudioEnvironment::system()`, which keeps production behavior unchanged. The parity harness injects:
  - a temporary app-data folder and SDK home;
  - a `BuildService` backed by the fake compiler;
  - `OwnershipPolicy::TestInjected(ProcessGroupContained { qualification: "scripted-fixture" })`, the same fixture the workflow tests use.

  Production code hard-codes no ownership.
- **Telemetry:** `StudioShell::telemetry(cx)` adds the following to the qualification snapshot: open project, SDK status, preview, canvas, timeline, the agent panel (including Detect agents state, transcript row kinds and sizes but never their text, and the reason Apply is blocked), export state, output devices and presets.
- **Actions:** `src/actions.rs` defines every command the tests drive, and the shell handles all of them at its root. Buttons that had private click handlers now call the same methods: preset apply, reset and override; checkpoint save and restore; SDK select and import; Detect agents and Select; prompt focus and submit; permission answers; Apply; review policy.
- **Fakes:**
  - The fake SDK passes real verification: compatibility manifest, FFmpeg folders, and a `rustc` built from `tests/support/fake-rustc.rs`.
  - `fake-preview-worker.py` gained export (`--export-v1`, with hang and failure modes) and editor-object metadata.
  - The scripted ACP agent is `tests/support/acp-agent.py`.
- **Gates:** `Studio::mutation_open(Mutation::Preset | Mutation::AgentApply)` is the one place M3 flips:
  - presets follow `preset_state::MUTATION_QUALIFIED`;
  - agent Apply follows `edit_transaction::probe_apply_gate`.

### Found while writing the tests

- **Export bug, fixed (`0f86b42`):** the export thread held the project controller for the whole render, so every other command waited for the export to finish. `services::commands_run_while_an_export_renders` pins the fix.
- **Checkpoints are not gated:** neither create nor restore writes project source. `plan.md` is corrected.
- **Agent Apply has its own gate:** `probe_apply_gate`, which blocks on every non-Linux OS. It is separate from the preset gate.
- **Export completion is not covered here:** the app verifies a finished MP4 by decoding it, which the fake worker cannot produce. `tests/native-export.rs` covers completion.
- **Source-click mapping:** the click-to-source JSON exists only in the spike UI (`preview_element.rs`). The production mapping is covered through the canvas element scope.
- **`deny.toml`:** gpui's `test-support` pulls `proptest` from git. It is allowed as a dev-only source.
- **Detection test assumption:** `an_agent_that_was_not_detected_is_not_turned_on` assumes `antigravity-acp` is not on the host's `PATH`.

### Mutation check (one emptied body per module, run on Windows)

| Module | Emptied | Failing tests |
|---|---|---|
| agent | `ConversationPanel::answer_permission` | 2 of 2 |
| audio | `OutputDevice::next` (returns `self`) | 1 of 3 |
| canvas | `StudioShell::select_canvas` | 3 of 3 |
| checkpoints | `Controller::checkpoint` (returns `Ok`) | 2 of 2 |
| presets | `StudioShell::run_preset` | 1 of 1 (blocked branch) |
| services | `StudioShell::cancel_export` | 1 of 7 |
| setup | `ConversationPanel::use_detected` | 2 of 5 |
| timeline | `TimelineView::cycle_scene` | 1 of 4 |
| transport | `StudioShell::step_preview` | 1 of 3 |
