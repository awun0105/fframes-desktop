# Phase 2: agent data without compatibility

Branch `feat/studio-m6`, based on `bd4f3ac`. Committed locally, not pushed. Rule applied: old data is refused or
ignored, never repaired or destroyed.

## Removed

Session store (`crates/studio-engine/src/agent/`):
- `SessionStore::migrate_legacy` and every helper behind it (item discovery, archive ids,
  fingerprints, staging, settle), `LEGACY_SESSION_ID`, `MigrationReport`, `source_hash`
  (field of `SessionMeta`), the legacy fallbacks of `current_session_id` and `current_log_path`,
  and the "earlier conversations" notices (`Startup.notices`).
- `AppPaths::agent_session_manifest`, `AppPaths::legacy_conversation_log`, and the manifest-path
  fallback in `session_store.rs`.
- `RowStore::start_new` and `archive_to_unused_name` (`log.rs`): no caller outside tests.
- The `startup_warnings` parameter of `Actor::new`.

Provider settings (`crates/studio-engine/src/providers/`):
- `agent-adapter.json` handling: `SETTINGS_FILE`, `settings_path`, the load fallback, the
  legacy writes, the second removal in `clear_settings`, `remove_if_present`.
- `migrate_from_adapter_file` and the registry v1 upgrade (including CLI-name filling).
- `AdapterFile::parse` is kept as a plain parser (used by the Setup form and tests).

## Behaviour now

- A session manifest is accepted only at `SESSION_MANIFEST_SCHEMA_VERSION` (2). An older one
  gives "...made by an older Studio (manifest format N, this Studio reads format 2). It is left
  as it is and cannot be restored."; a newer one gives the "newer Studio" equivalent.
  `SessionStore::load` returns `Err` with that text; `save` and `clear` refuse to touch the
  file; `list_sessions`, `load_session` and `session_manifest` ignore it (rows still open, not
  resumable). The agent actor shows the refusal as a warning row once (`offer_saved_session`).
  A manifest that is not JSON is not an "old format": it can still be replaced or cleared.
- `SessionStore::prepare` returns `Result<Startup, String>`: removes staging leftovers, creates
  and selects the first session only when none exists, returns its log and the row floor.
  It fails the workflow open when no session can be created (no legacy log to fall back to).
- Files of the single-session layout (`conversation.jsonl` in the project directory,
  `agent/session-manifest.json`, `conversation-<stamp>.jsonl`) are never read or touched: the
  project starts with an empty history and one new session.
- `current_log_path` returns `Option<PathBuf>` (`None` while no session exists). `save` with no
  session errors ("the project has no current session").
- Registry: `provider_profiles::check_registry_version` refuses any `schema_version` other than
  2 and any valid JSON that is not the current format (messages in Review fixes). `load_registry`, `load_settings`,
  `parse`, `save_*` and `clear_*` all refuse and leave the file byte-identical. A missing
  registry means Setup is needed; an `agent-adapter.json` is never read, written or removed.
- Rollback backup on save is kept; its suffix is now `.bak` (was `.migration-bak`).
- Callers moved to the registry API: `app/tests/x11_shell.rs` (two fixtures now call
  `host::save_settings`), `app/tests/agent_workflow_ui.rs` (damaged description is
  `provider-registry.json`, message "Invalid provider registry JSON").

## Tests

- `tests/session_history.rs`: migration tests and the legacy fixture deleted; new
  `current_project()` fixture. Added: old files ignored and byte-identical, v1 and v99 manifests
  refused (load/save/clear, byte-identical, session still lists and opens), non-JSON manifest
  replaceable, no-session load/save, prepare creating the first session once, serving the
  existing one, removing staging, leaving a v1 manifest alone, failing when sessions cannot be
  written (old log untouched).
- `tests/provider_profiles.rs`: migration tests replaced by: adapter file ignored (Setup
  needed, byte-identical), adapter file survives every registry write, v1 registry refused by
  load/parse/save/clear with the file byte-identical and no `.bak`, rollback copy kept on
  replace, parse of another schema version refused.
- `app/tests/agent_workflow/sessions.rs`: migration test replaced by old-layout-ignored test;
  `composer.rs` and `regressions.rs` adjusted to the new API.

## Check results (all on `feat/studio-m6`, CI idle before the first build)

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo check --workspace --all-targets`: pass.
- `cargo test -p studio-engine -p fframes-studio -p studio-harness --no-fail-fast`: pass
  (72 result blocks, 0 failed).
- `cargo test -p studio-ui --lib`: 473 passed.
- `cargo test -p studio-ui --test parity --no-fail-fast`: the run ended while `services::cancelling_an_export_leaves_no_file`, `a_failed_export_is_shown` and `commands_run_while_an_export_renders` were still running (cargo reported the target failed; no assertion failure was printed). Rerun with `-- --skip services::cancelling_an_export_leaves_no_file`: 77 passed, 0 failed, 1 filtered out, so the other two pass and the cancelling test is the only one that does not finish (it predates this work).

Environment note: a first full run failed ~170 app tests because `studio-test-support`'s baked
`CARGO_MANIFEST_DIR` pointed at the removed `fframes-m6-base` worktree (stale cached build in
the shared target dir). Touching `crates/studio-test-support/src/lib.rs` rebuilt it; no source
change was involved.

## Not compiled on this host

- `app/tests/x11_shell.rs` is `#![cfg(unix)]`. Its `save_scripted_peer` helper and the two call sites were not compiled here (the Linux target check needs a Linux C++ cross toolchain). First compile is Linux CI.
- A repo-wide search outside `docs/` and `plans/` for the removed names found no other caller, script or workflow. Remaining hits are the intentional "not read" docs and the old-file names in the new tests; `source_hash` hits elsewhere are unrelated anchor code.

## Concerns

- `SessionMeta` stays strict (see Review fixes): an M5-era `meta.json` with `source_hash` is not
  used and never rewritten.
- `current_log_path` is now `Option`; `prepare` can fail the workflow open.
- `.migration-bak` backup suffix became `.bak`; existing `.migration-bak` files are untouched.
- Newer-version manifests now return `Err` from `load` (was `Ok(None)`). Unchanged by the review.

## Review fixes

1. `SessionMeta` is strict again: `deny_unknown_fields` is back and `read_meta` requires
   `schema_version == SESSION_META_SCHEMA_VERSION`. A meta that does not read is skipped by every
   reader, so `record_last_row_id` (reached from a session switch) writes nothing and the file
   stays byte-identical. The session is still listed and opens read-only. Tests:
   `a_meta_with_an_unknown_field_is_never_rewritten_and_its_session_still_opens`,
   `a_meta_of_another_schema_version_is_not_used_and_not_rewritten`.
2. Refusals reach the user and name the file. `SessionStore::manifest_status` returns
   `ManifestStatus::{Missing, Ok, Refused(reason)}`; `open_session` and `restore_session` use it,
   so switching to or restoring a session with a refused manifest shows the "older/newer Studio"
   reason (read-only notice, restore notice) instead of "no saved session". Session manifest
   and provider registry messages include the file path. Actor test:
   `switching_to_and_restoring_a_session_with_a_refused_manifest_shows_the_reason`.
3. UI keeps the registry refusal: `settings_error` is now set from the save result in
   `commands.rs` (`apply_saved`) and `provider_setup.rs` (turn-on save): cleared only when the
   save succeeds, kept (set to the error) otherwise. There is no panel-level test harness that
   can attach a workflow here, so the covered contract is the engine side the panel reads:
   `a_registry_of_another_version_is_reported_on_open_and_again_by_every_save` in
   `app/tests/agent_workflow_ui.rs` (open reports it, every save path returns it with the file
   named and the file unchanged).
4. Valid JSON of the wrong shape is refused, not replaced: `refuse_foreign_manifest` and
   `check_registry_version` refuse any valid JSON that does not parse as the current format
   ("...is not in the format this Studio reads (...). It is left as it is..."). Only non-JSON text
   or a failed read stays replaceable. `ProviderRegistry::validate` requires
   `schema_version == REGISTRY_SCHEMA_VERSION`. `ProviderRegistry::parse` no longer runs the
   path-naming check itself (it has no path); `host::load_registry` runs it first.
5. The repeated "manifest could not be saved" warning goes through `warn_once`.
6. Tests: file name asserted in the manifest and registry refusals; wrong-shape manifest and
   registry refused byte-identical; `manifest_status` cases; `prepare`, pointer repair, new
   sessions and `record_last_row_id` leave a refused manifest byte-identical.
7. `docs/desktop/README.md:36` still describes the removed behaviour as current: the settings
   page "describes one ACP v1 adapter in the app-local file `<app data>/agent-adapter.json`" and
   Select "migrat[es] a legacy `agent-adapter.json` first, which is left intact". For the docs
   phase; `docs/` was not edited.

Checks after the review fixes are recorded in the final status of the hand-back.
