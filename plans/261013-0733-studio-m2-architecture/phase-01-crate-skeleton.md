---
phase: 1
title: "Crate skeleton and service move"
status: completed
priority: P1
effort: "4-5d"
dependencies: []
---

# Phase 1: Crate skeleton and service move

## Goal

Create the target crates, move the GPUI-free services and their tests into the engine, and rename the harness crate, with no logic changes.

## Known UI edges to cut first (verified 2026-10-13; re-scout)

- `worker_client.rs:691` calls `crate::frame_image::generate_reference_frame` (`frame_image.rs` imports gpui).
- `agent_workflow/actor.rs:325-333` uses `conversation_panel::host::AdapterFile` and `qualification::launch_identity`; `:386` uses `evidence_preview::decode_png` (imports gpui); `:2886-2936` uses `host::load_registry`/`save_registry` and `provider_profiles::ProviderRegistry`.
- `agent_workflow.rs:146,255,315,403` uses `qualification::OwnershipPolicy` and `evidence_preview::MAX_ENCODED_BYTES`.
- `thumbnail_cache.rs` and `agent_workflow.rs` each import gpui once.
- `agent_tools/backend.rs:929,954` read design tokens.

## Tasks

1. Scout: turn the list above into a complete edge list (`use crate::` from every service to every gpui module).
2. Move first, before the services: `conversation_panel/{host,provider_profiles}` (non-UI parts) to `studio-engine::providers`; ownership resolution, ledger verification and launch identity from `conversation_panel/qualification.rs` to `studio-engine::containment`, and `qualify_writer.rs` (no gpui use, verified 2026-10-13; it backs the `qualify-writer` subcommand, the only way a real adapter becomes contained until M3 phase 4) to `studio-engine::containment::qualify_writer`; split `generate_reference_frame` and `decode_png` into a plain RGBA part (engine) and the gpui conversion (UI).
3. Rename `desktop/crates/studio-agent-spike` to `studio-harness`. Consumers to update (about 40 files): `desktop/Cargo.toml` (member and dependency), `desktop/app/Cargo.toml`, the crate's own `Cargo.toml`; every `studio_agent_spike::` import in `app/src` and `app/tests`; `-p studio-agent-spike` in `.github/workflows/desktop.yml:50-51`, `desktop-self-hosted.yml:94,96,256,258`, `desktop-release.yml:183-184`; `desktop/scripts/qualify-m3-agent.py:656` and `qualify-m6-providers.py:60` (until phase 7 deletes them); `docs/desktop/architecture.md`. Do not edit `desktop/qualification/evidence/*`. Check with `git grep -n studio.agent.spike -- ':!plans' ':!desktop/qualification/evidence'` returning nothing.
4. Create empty `studio-proto`, `studio-theme` and `studio-ui` crates as workspace members.
5. Move into `studio-engine/src/services/`: `build_service`, `preview_coordinator`, `preview_worker_client`, `worker_client`, `worker_project`, `audio_service` (with `audio_service/`), `export_service`, `candidate_runner`, `thumbnail_cache` (key and storage only). Move `agent_workflow/*` to `studio-engine/src/agent/` and `agent_tools/*` to `studio-engine/src/agent_tools/`.
6. Move the engine-level integration tests from `desktop/app/tests` to `desktop/crates/studio-engine/tests`: `agent_tools.rs`, `agent_workflow.rs`, `audio_preview.rs`, `build_sharing.rs`, `candidate_runner.rs`, `managed_worker.rs`, `native-export.rs`, `portable_managed_worker.rs`, `preview_coordination.rs`, `promotion_handoff.rs`, `provider_handoff.rs`, `real_sdk_candidates.rs`, `real_sdk_selection.rs`, `selection_tools.rs`, `timeline_controls.rs`, `agent_workflow_apply_gate.rs`, `provider_profiles.rs` (copy the `m6-results.json` it embeds with `include_str!` into its own test fixtures, so it no longer reads `desktop/qualification/`), `m3_development_evidence.rs` (deleted in phase 7 instead if it only tests the harness), `support/{build_fixture,workflow_world}.rs` and the `fixtures/` they use (including `pre_m4_project`). No re-export layer in the app crate.
7. Split `agent_tools/backend.rs` (1,835 lines) along its sections into `agent_tools/backend/` modules.
8. Point the `studio-tools` and `studio-mcp` bins at the moved modules.
9. Rename kebab-case Rust files while moving (`session-store.rs`, `provider-profiles.rs`, and `studio-engine/src/export-source.rs`) and drop their `#[path]` attributes.

## Todo

- [x] Complete edge list
- [x] Providers, containment and image helpers moved first
- [x] Harness rename with every consumer, grep clean
- [x] Empty crates
- [x] Services and agent modules moved
- [x] Engine tests moved
- [x] `backend.rs` split
- [x] Bins rewired
- [x] File renames

## Success criteria

Parity tests, `cargo test`, `just clippy` and all CI workflows pass; `cargo tree -p studio-engine -e normal` and `-p studio-harness` show no `gpui`; the harness-rename grep is clean.

## Outcome (2026-10-14)

Done in seven commits on `feat/studio-rewrite` (`8058fbf`..`ccc62b6`). Verified on Windows: `cargo test --workspace` 822 passed, 0 failed, 14 ignored before and after (835 tests listed per package before and after, now split engine 336 / app 244); `cargo clippy --workspace --all-targets -D warnings` clean; `cargo tree -e normal` shows no `gpui` for `studio-engine`, `studio-harness`, `studio-proto`; the rename grep is clean; `test-packaging.py`, `test-qualification-m4/m5/m6/m7.py` pass. Not verified here: the CI workflows on Linux and macOS (no push yet). Ten app test files are gated to Unix or Linux as a whole (`agent_tools`, `agent_workflow`, `agent_workflow_ui`, `m3_development_evidence`, `preset_workflow`, `promotion_handoff`, `provider_handoff`, `real_sdk_promotion`, `scoped_editing`, `x11_shell`) and were never compiled on this Windows VM; a textual sweep for stale module paths, run regardless of `cfg`, found and fixed one (`agent_workflow_ui.rs`) and finds nothing else.

Module layout in `studio-engine`: `services::{audio_service, build_service, candidate_runner, export_service, preview_coordinator, preview_worker_client, teardown, thumbnail_cache, worker_client, worker_project}`, `agent` (with `agent::evidence_preview`), `agent_tools` (with `backend::{artifacts, project, writer_gate}`), `providers::{host, provider_profiles}`, `containment` (with `containment::qualify_writer`). No re-export layer in the app.

Deviations from the task list, with the reason:

- **GPUI edges cut by type, not by splitting functions.** `decode_png` returns the decoded RGBA thumbnail; the agent panel wraps it in a `RenderImage` once per new artifact. `ThumbnailCache<I>` is generic over the image type, so the whole cache moved; the timeline uses `ThumbnailCache<gpui::RenderImage>`. `generate_reference_frame` moved whole into `services::worker_client` (it has no gpui use).
- **`teardown` moved too** (`preview_coordinator` depends on it; not in the list).
- **Tests that stay in the app:** `agent_tools`, `agent_workflow`, `agent_workflow_apply_gate`, `candidate_runner`, `m3_development_evidence`, `provider_handoff` start the `studio-tools`/`studio-mcp` bins (`CARGO_BIN_EXE_*` exists only in the bins' own package, and the bins stay in the app per the target layout); `audio_preview` and `timeline_controls` use `frame_image` (gpui); `promotion_handoff` uses `studio_shell`. They import from `studio_engine` directly. Moved: `build_sharing`, `managed_worker`, `native-export`, `portable_managed_worker`, `preview_coordination`, `real_sdk_candidates`, `real_sdk_selection`, `selection_tools`, `provider_profiles`.
- **`provider_profiles.rs` split:** the four `provider_setup` (gpui) tests went to `app/tests/provider_setup.rs`; the rest moved with its own `tests/fixtures/m6-results.json`.
- **Shared fixtures** (`support/`, `stalled-worker.py`, `malformed-preview-worker.py`) live in `studio-engine/tests`; app tests include them by relative path and find them through `studio_test_support::ENGINE_TESTS_DIR`. `fixtures/pre_m4_project` stays in the app (only `real_sdk_presets` uses it).
- **`backend.rs` split along its own section markers** (`writer_gate`, `artifacts`, `project`); the `ToolBackend` impl stays with `ProjectToolBackend` in `project.rs` (1,313 lines) because separating them would widen the visibility of every private worker and revision type.
- **Kebab renames:** `session-store.rs` and `provider-profiles.rs` were renamed in the move; `export-source.rs` in its own commit. `studio-sdk/src/release-manifest.rs` was not in the list and is left for M2 phase 8 or M6.
- Updated outside Rust: CI `-p` lists (new crates, `-p studio-engine --test` for moved tests), `test-packaging.py`, `package-phase-zero.py` comment, `qualify-m3-agent.py` and `qualify-m5-selection.py` (until phase 7 deletes them), `docs/desktop/architecture.md` paths.

CI (2026-10-14): the self-hosted validation run 38515074950 on `5cdf529` (phase 1 plus the stale-path fix) passed on both Linux and Windows. The Linux job compiles and runs the Unix-gated app tests. No macOS job has run on a phase 1 head: `Desktop CI` last ran on `2f527d3`, before phase 1.
