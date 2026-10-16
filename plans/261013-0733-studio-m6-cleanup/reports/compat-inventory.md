# Compatibility-only code inventory (2026-10-17)

Read-only scout of `desktop/` at `7ebba47`. Line numbers are approximate; re-check before editing. Paths are relative to `desktop/`.

## Agent sessions (phase 2)

- `crates/studio-engine/src/agent/session_store/layout.rs` (717 lines): migration of the single-session layout (`conversation.jsonl`, `agent/session-manifest.json`, `conversation-<stamp>.jsonl` archives) into `sessions/<id>/`.
  - Removable, about 330 lines: the module doc (1-33), `LEGACY_SESSION_ID` (52), `SessionMeta.source_hash` (79-82), `MigrationReport` (85-100), `legacy_files_unmigrated` (185-190), the `current_session_id` branch for unmigrated files (221-225), the `current_log_path` fallback (254-261), `migrate_legacy` (390-456), and its helpers (~459-719).
  - Check before deleting: `publish_dir` and staging are also used by `create_session_from`.
  - Keep: `create_session`, `set_current`, the row floors, pointer repair, `retry_denied` and `write_meta`.
- `session_store/startup.rs`: `prepare` calls `migrate_legacy` and shows the "earlier conversations" notices. After the change it only creates the first session.
- `session_store.rs`: manifest schema v1 is still accepted (lines 17-19, 53, 202; `layout.rs:631-635`). Change these to an exact match. `manifest_path` falls back to `paths.agent_session_manifest` (175-180).
- `app_paths.rs`: `agent_session_manifest` (63-65) and `legacy_conversation_log` (83-86). `agent.rs` 363, 380-381: migration comments.
- `agent/log.rs` 441-507: `RowStore::start_new` and `archive_to_unused_name` write the archive format; check callers (tests only seen at 794-840).
- Tests:
  - `crates/studio-engine/tests/session_history.rs`: migration tests at ~142-470 and ~737-877. The shared `legacy_project()` fixture is also used by current list and load tests, so give those a current-layout fixture.
  - `app/tests/agent_workflow/sessions.rs` and `crates/studio-engine/tests/agent_task_scope.rs` mention the old layout.

## Agent adapter settings (phase 2)

- `crates/studio-engine/src/providers/host.rs`: the old `agent-adapter.json` and its migration into the provider registry.
  - Lines 28-32, 268, 297-314, 320, 345-346, 369-372, 385-398 (`load_settings` fallback), 409-421 (`save_settings` writes the old file), 457-465 and 647.
  - Tests at ~697-725 and ~876-1000.
  - Keep the registry rollback backup on save (336-339).
- `providers/provider_profiles.rs`: `migrate_from_adapter_file` (327-363), and the registry v1 upgrade (280-289) of `REGISTRY_SCHEMA_VERSION=2`.
- Callers that still use the old file through `save_settings`/`load_settings`/`AdapterFile` and must move to the registry API:
  - `app/tests/agent_workflow_ui.rs:147, 276`
  - `app/tests/x11_shell.rs:440, 725`
  - `crates/studio-ui/tests/parity/settings.rs:124`
  - `crates/studio-ui/tests/parity/harness.rs:146, 180`
  - `crates/studio-ui/src/agent/commands.rs:233-234`
- `crates/studio-engine/tests/provider_profiles.rs` has about 24 references to the old file.

## Durable data (phase 3)

- **Task journal formats 2 and 3.**
  - `edit_transaction/mod.rs`: `TASK_FORMAT=4` and `OLDEST_TASK_FORMAT=2` (63-72); "legacy records" docs (114-121, 226, 294-298).
  - `#[serde(default)]` fields at 303, 311, 315, 354, 358, 362 and 421. Make one required only if the current writer always emits it.
  - `journal.rs`: header comment (242-247), the format check (454-456), and the format-2/mixed-format tests (622-775).
  - `edit_transaction/mod.rs` tests at ~1305-1390.
  - Keep: the newer-format refusal, the lifecycle `NewerFormat`, torn-tail quarantine and crash recovery.
- **`FileId` aliases.** `crates/studio-project/src/publish/mod.rs` 78-89 (`alias = "dev"`, `alias = "ino"`).
  - Keep the u64-or-string encoding (111-137); it is needed for Windows u128.
  - `edit_transaction/executor.rs` 10-13, 124-132, 540-541: wording only. Check whether `NoClobber::Link` is a real fallback.
- **Snapshot defaults.** `state.rs` 375-400 (`build_source`, `review_policy`, `task_history` skip-serialising, `saved_generation`), with tests at ~801-845. Some may be current behaviour; verify each.
- **Source inventory version 0 and the Timing-hashed-as-Other identity.** `crates/studio-project/src/revision.rs`: 54-56, 67, 70-77, 139-170, 173, 245-255.
  - Tests: `crates/studio-project/tests/revision.rs` (72, 90-150) and `checkpoint.rs`.
  - Changing the hash needs an inventory version bump in the same commit.
- **SQLite history v1.** `store.rs` 19, 27-61 (migration and backup, the self-heal `CREATE TABLE IF NOT EXISTS`), with tests at ~186-300.
  - Keep the `NewerFormat` refusal and the read-only diagnostics open.

## Project migration and the old SDK framework (phase 4)

- `crates/studio-engine/src/project_migration.rs` (393 lines).
- `controller/migration.rs` (425 lines).
- `controller.rs`: 34, 39, 98-99, 214, 380 and 508.
- `controller/timing.rs`: ~29-61 and ~201.
- `journal.rs`: ~296-310 (`committed_migrations`).
- `edit_transaction/mod.rs`: `TaskRevisionRecord.migration` and `TransactionKind::Migration`.
- `build_materialization.rs`: 43-79 (`framework_for`, the `legacy` FrameworkSet).
- `engine/backend.rs`: 10, 81, 268-273, 398, 499, 510-520, 551-557.
- `services/export_service.rs`: 374-392 and 444-447 (`CapabilitiesError::Legacy`).
- `timing/doc.rs`: 102.
- `project_presentation.rs`: 26, 30, 84-86.
- `lib.rs`: 22 and 42-44.
- `crates/studio-project/src/manifest.rs`: `CURRENT_INSTRUCTION_VERSION` and `generated_instruction_version` (9-11, 118, 124, 212-213); `lifecycle.rs`: 9, 264, 363.
- `crates/studio-sdk/src/manifest.rs`:
  - `LegacyFramework` (77-87), `CompatibilityManifest.legacy` (113-114), validation (185-205), and a test (369-398).
  - Optional `runtime_version` with `FIRST_RUNTIME_VERSION` and `studio_runtime_version()` (10, 100-104, 125-128).
  - Optional `preview_contract_versions` and `preview_capabilities` (108-112).
  - Scripts that touch them: `scripts/assemble-phase-zero-sdk.py:280` and `scripts/test-packaging.py:40`.
- UI:
  - `shell/status_strip.rs` 4, 112-139 (migration notice and button);
  - `shell/export_panel.rs` 16, 111, 146 (`ExportSupport::Unmigrated`);
  - `shell/overflow.rs` 32, 161-196, 297-300 (`UndoMigration`);
  - `timeline/view.rs` 16, 585-586, 1848;
  - `timeline/model.rs` 97, 161.
- Fixtures and tests:
  - `app/tests/fixtures/pre_m4_project/`
  - `crates/studio-engine/tests/project_migration.rs`
  - `tests/native-export.rs` (~534-537)
  - `tests/materialization.rs`
  - `app/tests/real_sdk_presets.rs` (~395-460)
  - parity `services.rs` `make_legacy` (174-198)
  - `promotion_install.rs`, `timing.rs` and `studio-project/tests/manifest.rs`

## Spike (phase 1)

- `app/src/`: `app.rs` (662 lines), `agent_spike.rs` (372), `selection_spike.rs` (405), `preview_element.rs` (108), `ui/mod.rs` (4) and the `lib.rs` re-exports.
- `main.rs`: `Command::SpikeUi`, `run_spike_ui`, and the test at 266-282. Keep `assets.rs`.
- Harness code that only the spike uses:
  - `crates/studio-harness/src/session.rs` (853 lines). Move `AdapterConfig` (24-32) out first.
  - The `Acp*` structs in `driver.rs` (157-215) and their `lib.rs` re-exports (15-20).
  - The legacy modes of `tests/acp-peer.py`.
  - Check `supervisor.rs` (`AgentSupervisor`) for other users.
  - Keep `redact_sensitive_string` and `McpStdioServer`.
- `app/tests/worker_roundtrip.rs` (83 lines, ignored, uses `selection_spike::DisplayedSourceFrame`): rewrite against `WorkerClient`. Keep the CI step name "Real worker frame, anchor, crash and restart".
- `scripts/package-phase-zero.py`: docstring (2) and bundle id `studio.fframes.spike` (196). Keep the script file names.
- Stale wording:
  - `crates/studio-ui/src/setup_view.rs:73`
  - `crates/studio-engine/src/sdk_setup.rs:89`
  - `engine/backend.rs:272` ("M2 preview SDK")
  - `services/preview_worker_client.rs:1, 19` ("legacy client", M2)
- Docs (phase 6): `docs/desktop/README.md:59`, `phase-zero-feasibility.md:15-18` and `design-system.md:3`.

## Must stay (required by current behaviour)

- ACP v1 negotiation.
- Every `NewerFormat` refusal.
- Crash recovery and torn-tail quarantine.
- The `FileId` u64-or-string encoding.
- Row-log `FORMAT_VERSION=1`.
- The exact-match schema checks (`studio.json`, SDK, release, install receipt, source index, timing doc).
- `undoes_paths: Option` and the mode options.
- `CONTEXT_SCHEMA_VERSION`.
- The registry rollback backup.
- `AdapterConfig`, `redact_sensitive_string` and `McpStdioServer`.
- `WorkerClient`, the live transport under `PreviewWorkerClient`; fix its wording only.
- The `gray_*_legacy` theme tokens; they are names only and out of scope.
