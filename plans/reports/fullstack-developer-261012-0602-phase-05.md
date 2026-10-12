# Phase 5 report: player card

Outcome: the player area is now a card with a 44 px toolbar, the stage and a 48 px transport; all listed ids and telemetry stay rendered by default.

Files (desktop/app/src): `studio_shell.rs` (`render_player_area` split into `render_player_toolbar`, `render_stage`, `render_transport`, overflow menu; pure `transport_mode`, `show_total_time`, `format_clock`, `overflow_actions`; 3 tests), `canvas_view.rs` (`StageLayout`, `stage_layout`, 3 tests), `preview_element.rs` (corner radius 6 clip and line ring). Phase file status updated. Nothing under root `examples/`; no commit; no background processes.

Design:
- `preview-surface` keeps id, focus, all keys and mouse handling, and still carries the measuring canvas, so `canvas_origin`, `canvas_extent`, `preview_extent` and click mapping have the same meaning. The 16 px stage padding sits outside it. Focus ring is now a shadow ring (no border).
- Frame geometry comes from `StageLayout::from_viewport` (zoom and pan included): shadow, image (radius 6), selection (accent border plus selection fill) and scope (warning border) overlays clipped to the frame in frame coordinates, then a `line` ring. `stage_layout(bounds, size)` is the fit-zoom version; `preview_scale` untouched.
- Toolbar: Badge (Info while building/preparing, Warning on failure), Info icon with the hint as tooltip, audio label, then Next output / Retry audio / No audio as Editor buttons with probes. Transport: time readout (`m:ss.cc / total`, tooltip keeps revision and frame text), Previous, Play/Pause (32), Next, Mute, Build (Primary, Hammer), Cancel (always rendered, disabled without a build), overflow menu. Compact under 760 px: Build and Cancel become icon buttons (same ids), readout 12 px; total hidden under 620 px. Player width is measured by a canvas on the card.
- Overflow `Menu` (keyboard-driven) holds Refresh, Save checkpoint, Restore as copy…, Reveal retained draft, Reveal checkpoint. Items that were disabled are omitted (Menu has no disabled rows); conditions equal the old buttons'. Probe ids and enable conditions of the ten telemetry buttons are unchanged.

Validation: `cargo fmt --check` ok; clippy `--all-targets -D warnings -A deprecated` clean; `canvas_selection` (4) and `frame_image` (4) integration tests pass; lib 216 pass, 2 fail (known python `SpawnFailed`). Not run: x11_shell, qualify-*.py (no X/GPU), on-screen rendering.

Integration failures (`cargo test --no-fail-fast`, none touch the player; the code paths use no files I changed):
- lib `worker_client::stalled_control_request...`, `preview_worker_client::malformed_destinations...`: `Process(SpawnFailed{command:"python", NotFound})` (python spawn).
- `agent_workflow_apply_gate` (2 tests): "timed out waiting for candidate review / phase Editing" in `workflow_world.rs`, which launches a scripted agent with `python`. Attributed to missing python by source; not proven by rerun.
- `build_sharing` (1): "worker crash or unexpected exit: Exited before frame connection"; the fake preview worker is `fake-preview-worker.py` started through a launcher using `python`. Same attribution.
- `candidate_runner` (19 of 25): same "Exited before frame connection", or the BuildFailed/Worker-stage assertions that follow from it. Same attribution. Flag: none shown a different root cause, but only the two lib tests literally report `python`; the rest are inferred from the fake-worker launcher. I did not rerun with a working python.

Deviations: Tooltips on icon buttons and the hint show text only; `IconButton` has no key slot, so Space/M `Kbd` chips are not shown (Mute already says "(M)"). `Menu` rows cannot be disabled, so unavailable actions are omitted. Time readout is not tabular-figure; it uses a fixed min width. The spike's `preview_element.rs` now reads the `line` token for its ring. Shadow is drawn only in the shell, not in the spike element.

Status: DONE_WITH_CONCERNS
Summary: Player card, stage geometry, transport (with compact mode) and overflow menu are implemented with tests; fmt and clippy are clean and focused tests pass.
Concerns/Blockers: Layout and native click qualification are unverified here; 22 integration/lib failures all trace to the python fake worker or agent by source but only 2 report it literally. Kbd chips in tooltips are missing.
