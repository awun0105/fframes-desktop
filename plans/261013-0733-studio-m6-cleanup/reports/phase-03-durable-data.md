# Durable data without compatibility

Old data is refused, never repaired or destroyed. Each refusal names the file, says it was made by an older Studio, and runs before any recreate, quarantine, backup or repair path.

## Removed

- Task journal: `OLDEST_TASK_FORMAT` and the format 2/3 decoding. Only `TASK_FORMAT` (4) is read.
- `FileId`: the `dev` and `ino` serde aliases. The u64-or-string encoding of `file` stays.
- Source inventory: version 0, the unversioned hash forms, the `Timing`-hashed-as-`Other` mapping, and the `include_executable` hash switch. `matches_revision` now compares one hash.
- History database: the old-version migration, its online backup (`backup_database`), `CREATE TABLE IF NOT EXISTS`, and the self-heal of the projection table.
- `#[serde(default)]` on fields the current writer always emits, and the tests that only read old records (format 2/3 mixed journals, legacy `dev/ino`, v1 migration, unversioned checkpoints, WAL backup).
- Stale wording: legacy, older format, format 2 and earlier-build comments in the touched code. `docs/` is untouched.

## Serde defaults

Required now: `created_dirs`, `remove_dirs`, `defer_stage`, `vacates` (intent and record), `SourceFile.executable`, `SourceInventory.version`, `ProjectState.review_policy`, `ProjectState.saved_generation`. `task_history` is `#[serde(skip)]`: the journal is its only source.

Kept, because current writers omit them when empty (`skip_serializing_if`) or they are `Option`s that read as `None` when absent: `before_mode`, `after_mode`, `undoes_paths`, `preset`, `migration`, `slot_dir`, `build_source`.

The round-trip test `a_record_with_every_optional_field_empty_round_trips` asserts the skipped fields are not written, that the record reads back equal, and that the always-written fields are required. A `ProjectState` test asserts the same for `review_policy` and `saved_generation`.

## Refusal behaviour

| Format | Where it is judged | Result |
| --- | --- | --- |
| `tasks.jsonl` format 2 or 3 | the first line header in `scan_task_journal`, before the torn-tail path; `Controller::open` calls `TaskJournal::inspect` first | `EngineError::OlderFormat`, naming `tasks.jsonl`. A torn tail on an older file is not quarantined. |
| `tasks.jsonl` newer format | unchanged | `NewerFormat` |
| Inventory or checkpoint manifest version 0 or 1 | `read_manifest` reads `version` first, before verify, load or draft. `Controller` also calls `Checkpoints::check_format` on the last record's accepted revision at open, before recovery or comparison. | `ProjectError` naming the manifest path: "made by an older Studio". A newer version says "newer Studio". |
| History DB `user_version` 0 or 1, with tables | `Store::open`, `Store::check_format`, `Store::ensure_current` | `EngineError::OlderFormat`, naming the file. |
| History DB newer | unchanged | `NewerFormat` |
| Missing or empty history DB | `Store::open` | Created fresh in one `BEGIN IMMEDIATE` transaction. A file with tables but no version counts as v0 and is refused. |

`Store::open_read_only` is unchanged and still opens any database for diagnostics. The agent surface maps `OlderFormat` to an `older_format` structured error (`agent/present.rs`).

## Revision hash

`CURRENT_INVENTORY_VERSION` is 2, with prefix `studio-source-v2`. `Timing` is hashed as kind 7. New expected hashes in `tests/revision.rs` were computed independently of the code under test.

## `NoClobber::Link`

It is not a real fallback today. `probe_no_clobber` returns `Renameat2` or an error that keeps Apply blocked. `plan` refuses a forced `Link`. The live-name move in `executor.rs` is hardcoded to `Renameat2`. `Link` remains only as a recorded variant and in tests that force it, and `executor.rs` now says the apply gate never selects it. Removing the variant would change the public enum, so I left it.

## Tests

- `journal.rs`: `a_task_journal_of_an_older_format_is_refused_and_left_as_it_is` (formats 2 and 3, with and without a torn tail, through `open` and `inspect`; bytes identical and one file in the directory). `a_record_with_every_optional_field_empty_round_trips`.
- `store.rs`: `a_missing_database_is_created_at_the_current_version`, `a_database_of_an_older_version_is_refused_and_left_byte_identical` (versions 0 and 1; no backup; diagnostic open still works), newer-version test kept.
- `tests/task_recovery.rs`, through the real `Controller::open`:
  - journal formats 2 and 3 are refused;
  - an old accepted checkpoint, both unversioned and v1, is refused;
  - a v1 history DB is refused.

  Each asserts the message, the byte-identical data tree, the unchanged source tree, and no new files.
- `studio-project/tests/revision.rs`: old checkpoints (unversioned x2, v1) are refused through `load`, `verified_inventory` and `draft`, with a byte-identical manifest and an identical directory listing. A newer version is refused. The Timing hash and the revision hash are checked against independent values.
- `tests/relocation.rs`: the unversioned-history test is replaced by a current-history reopen, relocate and export test.
- `FileId`: `dev`/`ino` is rejected.

## Checks (Windows, local)

- `cargo fmt --all -- --check`: clean (`cargo fmt --all` was run).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p studio-project -p studio-engine -p fframes-studio -p studio-harness --no-fail-fast`: all pass.
- `cargo test -p studio-ui --lib`: 473 passed.
- `cargo test -p studio-ui --test parity --no-fail-fast -- --skip services::cancelling_an_export_leaves_no_file`: 77 passed, 0 failed. No flake.

I did not run `fmt --check` again after the final comment-only edit in `publish/mod.rs`.
`plans/261013-0733-studio-m6-cleanup/plan.md` was not touched. Nothing is committed.

## Review

A reviewer traced every open path: project open, controller recovery, `Store::open`, diagnostics, checkpoint load, draft and verify, and undo. Old data is refused before anything writes, quarantines, backs up or recreates. There were no blocking findings. Three minor findings were fixed before the commit:

- **Torn first line.** A task journal whose only line is a torn line of format 2 or 3 was quarantined and truncated. A torn first line is now judged by the `format` it starts with, so it is refused untouched. The refusal test covers it.
- **Required fields.** The round-trip tests now also assert that `defer_stage` and `vacates` are required on `FileOp`.
- **Checkpoint error.** A checkpoint inventory of another format reached the agent as `source_unavailable`, "Fix the project folder". It is now `history_format`, with a test.

All checks below were run again on Windows after these fixes.