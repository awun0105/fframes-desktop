---
phase: 3
title: "Phase 3: Engine backend wiring"
status: in-progress
priority: P1
effort: 1.5d
dependencies: [2]
---

# Phase 3: Engine backend wiring

## Goal

Studio builds the Skia worker, passes it a renderer preference, accepts every backend the
runtime can report, and recovers from a GPU that hangs. Projects pinned to an older SDK are not
migrated (user decision, validation session 2).

## Context

**SDK pin.** A project pins `SdkPin { release: sdk_id, compatibility_sha256: digest }`
(`desktop/crates/studio-engine/src/build_materialization.rs:26-31`). Materialization and `select_sdk`
reject any other SDK with "SDK pin mismatch" (`build_materialization.rs:45-54`, `:116-118`;
`controller.rs:501-506`; `engine/backend.rs:467-469`). Phase 2 changes both the release and the
digest, and the codebase has no repin path. Without one, every existing project stops building
when v5 is installed.

**Lockfile.** This is not a problem. Studio deletes the copied lock in its private staging tree
and generates a new one (`build_materialization.rs:230-233`, `services/worker_project.rs:198`), so
the user's `Cargo.lock` is never used or changed.

**The single string `"cpu"` has three meanings:**

| Meaning | Site | After |
|---|---|---|
| Build key: what was compiled | `services/build_service.rs:111` (`BuildKey::worker`) | `skia` |
| | `services/worker_project.rs:96-112` (`CargoCompiler::accepts`) | accepts `skia` |
| | `candidate_validation/validation.rs:133` (`VALIDATED_BACKEND`), used at `report.rs:319` | `skia` |
| Runtime backend: what started | `services/preview_worker_client.rs:338` (Hello must say `cpu`) | accepts `cpu`, `skia-cpu`, `skia-gpu`; exposes `backend()` and `backend_detail()` |
| | `candidate_validation/validation.rs:122-129` (`is_unsupported_shader` matches the text "cpu") and the warning text at `:347-349` | the warning follows the declared gap; the text names the running backend |
| Pixel cache key | `services/thumbnail_cache.rs:31` (`ThumbnailKey.backend`) | the runtime backend from Hello, so `cpu` and `skia-*` thumbnails never collide |

The test fixtures that change with these sites:

- `desktop/app/tests/timeline_controls.rs:118`
- `desktop/app/tests/candidate_runner/validation.rs:382-387`
- `desktop/crates/studio-engine/tests/candidate_validation/main.rs:315`
- `desktop/crates/studio-engine/tests/support/passing_probe.rs:134`
- `desktop/crates/studio-engine/tests/support/fake-preview-worker.py:196`
- `desktop/crates/studio-engine/tests/malformed-preview-worker.py:28`
- `services/preview_worker_client.rs:684`

`release-manifest.rs:504` is a fixture in a negative test and stays as it is.

**Worker launch.** The preview worker is launched in `launch_preview_worker_checked`
(`services/worker_project.rs:259-295`). Agent tools launch their own preview workers
(`agent_tools/backend/project.rs:809`), and exports launch theirs from the export service.

## Files to create / modify

- Modify: the sites and fixtures listed above.
- Modify: `desktop/crates/studio-engine/src/services/worker_project.rs` and the export spawn in
  `services/export_service.rs`. Pass `--renderer <preference>` to every worker launch.
- Create: `desktop/crates/studio-engine/src/services/renderer_preference.rs`. The session's
  preference. It comes from the Settings choice from Phase 4 (`auto` by default), and Studio's own `FFRAMES_STUDIO_RENDERER` (read once in
  the Studio process; it never reaches the child environment) overrides it for testing. It switches to `cpu` (tiny-skia) for the
  rest of the session after a GPU worker hangs.
- Modify: `desktop/crates/studio-engine/src/services/preview_coordinator.rs`. Store `backend` and
  `backend_detail` in the snapshot. A frame response that carries a new backend (the worker's
  mid-session fallback) is handled like a relaunch: the new backend replaces the old one, thumbnails
  are keyed by it from then on, and the shader gap is re-evaluated. When a `skia-gpu` worker misses its request deadline or exits
  during a frame, relaunch it once with `cpu` and record the reason. A change of the Settings choice relaunches the preview worker with the new `--renderer`; no rebuild is needed. <!-- Updated: Validation Session 1 - fallback is tiny-skia; preference comes from Settings -->

## Tasks & steps

- [x] **Pin spike (do first).** Create a project with the v4 SDK, install v5, and open the project.
  - Confirm the "SDK pin mismatch" failure.
  - Read how a durable project write is done today (checkpoint, journal), so the repin follows the same path.
  - After pointing the pin at v5 by hand, confirm `bind` accepts the template's `=0.2.0` framework
    versions against the v5 framework (`build_materialization.rs:323-346`). This holds only if Phase 2
    kept the versions at 0.2.0.
  - Find the UI surface that shows the mismatch error.
  - Write down the outcome, and stop to re-plan if the pin works differently.
- [x] ~~Implement the repin.~~ Dropped in validation session 2: no migration from older SDKs.
- [x] Split the three meanings of `"cpu"` as in the table, and update the fixtures.
- [x] Add `renderer_preference.rs` and pass `--renderer` to the preview, agent-tool and export workers.
      Engine test: spawn a real worker with `--renderer cpu`, and Hello reports `cpu` with the gap.
- [ ] Hang recovery in the coordinator (demotion done; the automatic relaunch and the Settings relaunch move to Phase 4, see notes). Fault test: a fake worker that reports `skia-gpu`, then stops
      answering a frame. The coordinator kills it at the deadline, relaunches with `cpu`, and
      the next frame arrives. The session preference is now `cpu`. A second test: changing the preference relaunches the worker, and Hello reports the new backend.
- [x] Shader warning: no warning for `skia-*` workers, and the warning still appears for a `cpu`
      worker. Update `desktop/app/tests/candidate_runner/validation.rs:382-387` for both.

## Verification

- From `desktop/`: `cargo test -p studio-engine -p studio-sdk -p studio-project`
- `cargo test -p fframes-studio --test candidate_runner --test timeline_controls`
- `desktop/crates/studio-engine/tests/timing_retime.rs` passes with the Skia worker.
- Manual: create a project on SDK v5, then build and preview on this VM (`cpu` by default, `skia-cpu` with `FFRAMES_STUDIO_RENDERER=skia-cpu`).

## Risks

- A Windows GPU driver reset (TDR) caused by a project shader can also reset Studio's own GPUI
  device. The engine can only recover the worker. Phase 5 tests the GPUI side on the GPU host, and
  docs §2 records the limit.
- The preview worker, an agent-tool worker and an export can each hold a GPU context at the same
  time. This is accepted, and the GPU host run in Phase 5 records VRAM use.

## Implementation notes

**Pin spike**
- v4 and v5 ship the same framework versions (fframes 1.3.0, runtime 0.2.0), so `bind` accepts a
  v4 project against the v5 framework.
- The template depends on `fframes-studio-runtime = "=0.2.0"` with default features, and the
  runtime's default is `skia`, so a repinned project builds the Skia worker without a source edit.
- The mismatch reaches the user only as `Backend::sdk` (the status text shown by the shell), set in
  `engine/backend.rs` `check_sdk`. The shell files that show it belong to layout-a.

**Repin**
- Implemented, released in `v0.1.2`, then removed (validation session 2). A project pinned to
  an older SDK gets the existing "SDK pin mismatch" refusal.

**Backend split and renderer**
- Build key: `WORKER_BUILD_BACKEND = "skia"`.
- Hello accepts `cpu`, `skia-cpu` and `skia-gpu`. The shader warning names the running backend and
  appears only with the `shader_preview` gap.
- `renderer_preference` drives every launch through `launch_preview_worker_checked` and the export
  spawn. Test: `every_preview_launch_asks_for_the_sessions_renderer_and_negotiates_what_started`.
- The thumbnail key backend stays `"cpu"`. The UI builds the key from the report and only reaches
  the backend through layout-a files, so this moves to Phase 4.

**Hang recovery**
- A `skia-gpu` worker that is killed at a frame deadline moves the session to `cpu`, and the
  displayed error says so. The next launch gets `--renderer cpu`.
- Test: `a_gpu_worker_that_stops_answering_a_frame_is_killed_and_the_next_launch_uses_the_cpu`,
  using the fake worker with `hang_frame`.
- Not done here, moved to Phase 4: the automatic relaunch, and relaunching on a Settings change.
  The install path lives in `player.rs` (layout-a), and the PCM artifacts are nonce-keyed, so
  swapping the worker in place would break audio.

**Deviation**
- `real_sdk_candidate_validation_scenarios` sets the session renderer to `cpu`. It compares the
  project CLI frame (always tiny-skia) byte for byte with the validator frame.

**Verification** (Windows VM, no GPU)
- `cargo test -p studio-engine -p studio-sdk -p studio-project`: 641 passed.
- The app `candidate_runner`, `timeline_controls` and `project_foundation` suites passed.
- `cargo clippy -p studio-engine -p studio-sdk -p fframes-studio --tests -D warnings` is clean.
- Real SDK v5, `SDK_BUNDLE`:
  - with the default renderer: `managed_worker` and `real_sdk_candidates` (4) passed;
  - with `FFRAMES_STUDIO_RENDERER=skia-cpu`: `preview_coordination` and all of `timing_retime`
    except export parity passed. The worker accepted `skia-cpu`, so the Studio-built worker has Skia.
  - `real_worker_builds_split_template_at_fractional_scales` hit the five-minute install build
    deadline only while three other cargo jobs ran; alone it passed.
  - `edited_sdk_export_matches_preview_and_preencode_at_three_times` (`ODIFF_BIN` set) passes on
    the default renderer. Under the `skia-cpu` override it differs by design: its reference frames
    come from `--qualify-preencode`, which renders with tiny-skia.
