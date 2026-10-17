# Project migration and the old SDK framework removed

Old data is refused, never repaired or destroyed. A project or history that only an older Studio could open is refused before anything writes, names the file, and says it was made by an older Studio.

## Removed

- `project_migration.rs` and `controller/migration.rs`: planning, running, undoing and healing a migration; `ProjectMigration`, `MigrationOutcome`, `ManifestSwap`, `MIGRATE_TO_EDIT_TIMING`; `Controller::timing_editing` (the migration gate of timing edits) and `add_recovery_notice` (only migrations used it).
- `TransactionKind::Migration`, `TaskRevisionRecord.migration`, `RecordSeed.migration`, `TaskReplay::committed_migrations`.
- `framework_for` keeps only the current framework of the SDK the project is pinned to. A project pinned to another SDK is not built (the existing "SDK pin mismatch").
- Backend: `Command::Migrate`, `Command::UndoMigration`, the "SDK predates Studio's timeline" create check, and the migration status in the presentation. `ProjectPresentation` loses `migration`, `timing` and `migration_undo`.
- Export: `CapabilitiesError` is gone. A worker that cannot report its capabilities shows the reason in the export panel (`ExportSupport::Unavailable`) instead of an "unmigrated project" notice. The panel still offers the default export, which runs without asking for capabilities; settings other than the default, or a `timeline.json`, are refused with the reason.
- UI: the status-strip migration notice and button, More actions > Undo migration, `ExportSupport::Unmigrated` and its panel notice.
- SDK manifest: `LegacyFramework`, `CompatibilityManifest.legacy`, `FIRST_RUNTIME_VERSION`, `studio_runtime_version()`. `runtime_version` is a required `String` and is validated as semver.
- `scripts/assemble-phase-zero-sdk.py`: the legacy framework export, the legacy worker build, `LEGACY_*`, `export_commit`. `SDK_RELEASE` is `v4`; `offline_worker_builds` is 2. `compatibility.schema.json` drops `legacy` and requires `runtime_version`.
- `app/tests/fixtures/pre_m4_project/`, `tests/project_migration.rs`, the legacy halves of `native-export.rs`, `materialization.rs` and `real_sdk_presets.rs` (and the matching checks in `scripts/test-preset-relocation.py`), parity `make_legacy` and its test, the stale "legacy framework" comments in two workflows.

## Refusal behaviour

| Old data | Where it is judged | Result |
| --- | --- | --- |
| `studio.json` with `generated_instruction_version` 1 (pre-M4) | `Manifest::parse`, before the controller creates or opens anything app-local | `ProjectError` on `studio.json` / `generated_instruction_version`: "made by an older Studio (version 1, this Studio reads 2)"; "The project is left as it is and cannot be opened". A higher version says "newer Studio" / "Update Studio". |
| `studio.json` of another `schema_version` | same | same wording; the old "Use a Studio version with an explicit migration" is gone. |
| SDK `compatibility.json` with an unknown field (such as the v3 SDK's `legacy`) | `CompatibilityManifest` deserialisation (`deny_unknown_fields`) | serde's `unknown field` error. |

### Decision 2026-10-17 (owner): no compatibility handling for M4/M5 data

The studio is a rewrite, so data written by the M4/M5 Studio gets no special case. A journal that holds a migration record fails as a malformed journal. An installed v3 SDK fails with its parse error until it is removed. A project pinned to a v3 SDK reports "SDK pin mismatch" and is recreated. An earlier version of this phase added a targeted journal refusal and a named refusal for an installed v3 SDK; both were removed.

### Decision: the SDK preview fields stay optional

`preview_contract_versions` and `preview_capabilities` describe whether the SDK's worker serves previews. They are not compatibility switches: the bundled Linux manifest has none and is still used to build and export. Making them required would mean editing `phase-zero-sdk.json` by hand and claiming a preview worker for an SDK that cannot be re-assembled on Windows (it needs a Linux sysroot and FFmpeg). Only the "legacy worker contract" doc comment changed. `runtime_version` was already present in the bundled manifest at the same position, so its digest (`69844c9c…`, checked by `test-packaging.py` and the `studio-sdk` test) is unchanged and nothing was re-assembled.

## Tests

- `studio-project/tests/manifest.rs`: `a_project_of_other_instructions_is_refused_with_the_studio_that_made_it` (1 = older, 3 = newer); the schema-version test asserts older/newer wording.
- `tests/task_recovery.rs`: `a_project_generated_by_an_older_studio_is_refused_and_left_as_it_is`, through the real `Controller::open`. It asserts the message, a byte-identical app data tree and source tree, and no new files.
- Parity `checkpoints::a_project_from_an_older_studio_is_refused_and_left_as_it_is`: the UI's Open shows the refusal, the project stays closed, `studio.json` is unchanged.
- `studio-sdk`: `runtime_version_is_required_and_unknown_fields_are_refused`; the bundled digest test is unchanged.
- `materialization.rs`: `projects_bind_to_the_framework_of_the_sdk_they_are_pinned_to` (current binding, mixed releases and missing versions refused, another pin refused).
- Parity `the_export_panel_opens_and_settles_on_the_projects_options` now expects `unavailable` (the fake worker does not answer the capabilities flag) instead of `unmigrated`.

## Not verified here

- `real_sdk_presets.rs` and `native-export.rs`'s Linux export test are `#[ignore]` and need an assembled SDK; they compile, and only their legacy halves were removed.
- `assemble-phase-zero-sdk.py` was not run (no SDK inputs on this machine); it parses and `test-packaging.py`, which imports it, passes.
- An installed v3 SDK must be removed by hand before a v4 SDK installs. Phase 6's Windows hand checks need a v4 SDK assembled from this branch.

## Review

A reviewer traced every open path: `Controller::open` and its read-only preflight, backend Open, Locate, Independent and Create, diagnostics, and relocation. Nothing writes before either refusal. A torn last line holding a migration would be quarantined, but no old Studio could leave one: a torn migrate intent means `studio.json` is still at version 1 and is refused first, and an undo intent always follows complete interior migration lines. Both decisions were judged sound.

One blocking finding: an installed v3 SDK stops setup with a bare parse error. It was first fixed with a named refusal, then dropped under the owner decision above. Minor findings fixed: the schema refuses unknown fields and declares the preview fields; the export wording in this report; an identity closure, a redundant `map_err`, a one-element loop, and the timing message.

Left for phase 6: `docs/desktop/README.md` and `phase-zero-feasibility.md` still describe migration. The workflows keep `fetch-depth: 0`; only the stale comment went.

## Checks (Windows, local)

All run after the review fixes; after the owner decision, fmt, clippy, `studio-sdk`, `studio-project`, the engine library, `task_recovery` and `agent_task` were run again and pass:
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p studio-engine -p fframes-studio -p studio-project -p studio-harness --no-fail-fast`: all pass.
- `cargo test -p studio-ui --lib`: 473 passed.
- `cargo test -p studio-ui --test parity --no-fail-fast -- --skip services::cancelling_an_export_leaves_no_file`: 77 passed, 0 failed. `agent::undo_in_the_prompt_undoes_the_typing_there` passed in both runs.
- `python desktop/scripts/test-packaging.py` (through `uv`): 22 run, OK, 3 Linux-only skipped.

One run failed to link with `LNK1285 corrupt PDB`: two test PDBs were left half-written when an earlier session ended mid-build. Deleting those PDBs fixed it; it was not a code problem.