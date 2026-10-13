---
phase: 1
title: "Crate skeleton and service move"
status: pending
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

- [ ] Complete edge list
- [ ] Providers, containment and image helpers moved first
- [ ] Harness rename with every consumer, grep clean
- [ ] Empty crates
- [ ] Services and agent modules moved
- [ ] Engine tests moved
- [ ] `backend.rs` split
- [ ] Bins rewired
- [ ] File renames

## Success criteria

Parity tests, `cargo test`, `just clippy` and all CI workflows pass; `cargo tree -p studio-engine -e normal` and `-p studio-harness` show no `gpui`; the harness-rename grep is clean.
