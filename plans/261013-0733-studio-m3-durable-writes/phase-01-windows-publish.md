---
phase: 1
title: "Windows publication and executor"
status: completed
priority: P1
effort: "5-7d"
dependencies: []
---

# Phase 1: Windows publication and executor

## Goal

`studio-project::publish` and the transaction executor work on Windows with the guarantees the Linux module doc states, and crash recovery is tested there.

## Context

- Spike findings: `../261013-0733-studio-m1-foundations/reports/spike-publish-primitives.md`
- Linux: `studio-project/src/publish.rs` (626 lines), `studio-engine/src/edit_transaction/linux.rs` (1,050 lines; `encode_id`/`decode_id` as `"dev:ino"` around `:122-133`; mode-bit equality around `:94-99`)
- Journal: `TASK_FORMAT` in `edit_transaction.rs` (2 on 2026-10-13); `FileId { dev: u64, ino: u64 }` persisted in `FileOp.before_id` and `DirIdentity`

## Tasks

1. Scout `publish.rs`, `edit_transaction.rs` and `edit_transaction/linux.rs`: which parts are POSIX, which Linux-only, which OS-neutral.
2. Split into `publish/{mod,unix,linux,macos,windows}.rs` and `edit_transaction/{mod,executor_unix,executor_windows}.rs`.
3. Widen `FileId` to a volume id plus a 128-bit file id, bump the journal format, and decode the old `"dev:ino"` records (test: a format-2 journal replays after the bump). Document that builds from before the bump cannot read new journals.
4. Windows publication with NT handle-relative calls: `NtCreateFile` with `RootDirectory` and `FILE_OPEN_REPARSE_POINT` for every open, refusing reparse points; `NtSetInformationFile(FileRenameInformationEx)` with `RootDirectory` and no replace flag; `FileDispositionInformationEx` for unlink; `FILE_ID_128` from `FileIdInformation`. No path-based `MoveFileExW` for live names. Use `windows-sys`.
5. Hard links: refuse to displace or publish over a file whose link count is above 1 (`EntryStat.nlink` is collected today and never checked); apply the same check on Linux and macOS.
6. Cloud placeholder reparse points: refuse with a clear reason (decision from spike S3).
7. Mode bits: define the Windows equivalent of the executor's mode check (read-only attribute) or skip it with a documented reason.
8. Windows executor including recovery, driven by the existing `Boundary` crash-injection hooks; Windows fault injection replaces `/dev/full`.
9. Make `tests/publish.rs`, `studio-engine/tests/{edit_transaction,task_recovery,review_publication}.rs` and the journal disk-full test compile and pass on Windows.

## Todo

- [x] Scout
- [x] Module splits
- [x] `FileId` widened, journal format bumped, old records decoded
- [x] NT handle-relative publication
- [x] Hard-link refusal on all OSes
- [x] Cloud placeholder refusal
- [x] Mode check defined
- [x] Windows executor and recovery
- [x] Suites green on this VM

## Success criteria

The four suites pass on this VM with the same test names as on Linux; Linux CI unchanged.

## Risks

- Antivirus or indexer handles cause sharing violations: bounded retry on `ERROR_SHARING_VIOLATION` only, then report.

## Outcome (2026-10-15)

- **Layout.** `studio-project/src/publish/{mod,unix,linux,macos,windows}.rs`: `mod.rs` holds the OS-neutral types, the probe and the `Dir` operations built on the primitives; `unix.rs` the POSIX primitives; `linux.rs` (`renameat2`) and `macos.rs` (`renameatx_np(RENAME_EXCL)`) only the no-clobber rename; `windows.rs` the NT handle-relative implementation. **Deviation:** the executor is one shared `edit_transaction/executor.rs`, not `executor_unix`/`executor_windows`. Every OS difference lives below `publish::Dir`, so per-OS copies of the state machine would only duplicate it. It is gated `any(linux, windows)` until phase 2 adds macOS.
- **`FileId`** is `{ volume: u64, file: u128 }`. It is written as `"volume:file"` in decimal, so format-2 `"dev:ino"` records still parse; serde aliases `dev`/`ino` keep old JSON readable. `TASK_FORMAT` is 3 and `OLDEST_TASK_FORMAT` is 2. A format-2 journal with a pending transaction replays and takes format-3 appends (unit test in `journal.rs`). **Builds from before the bump refuse a format-3 journal as newer** (`NewerFormat`), so going back to an older Studio after a write on this version needs the journal preserved and repaired by hand.
- **Windows publication:** `NtCreateFile` with `RootDirectory` and `FILE_OPEN_REPARSE_POINT`, refusing any reparse point; `FileRenameInformationEx` without replace; POSIX-semantics disposition for unlink and rmdir (an rmdir of a non-empty directory fails); `FileIdInformation` for the 128-bit id. Only `ERROR_SHARING_VIOLATION` is retried (20 x 25 ms). UNC roots are refused. `NoClobber::Link` reports `Unsupported` on Windows, so the probe selects the rename mechanism.
- **Hard links:** `hard_link_refusal` (nlink > 1) runs at planning and again in `displace` after the live verification, on every OS.
- **Cloud placeholders:** `Dir::open_root` opts the process into `PHCM_EXPOSE_PLACEHOLDERS`; cloud reparse tags classify as `EntryKind::CloudPlaceholder` and are refused with "move the project to a local folder". `tests/cloud_placeholder.rs` registers a real sync root through the Cloud Files API and unregisters it on drop. The test uses `open_root` rather than an unbound open, because placeholders are only visible once the process has opted in, and `open_root` is where the opt-in happens. Directory placeholders could not be created on this VM (0x8007017c), so the "project below a sync root" case uses a plain folder inside it.
- **Mode check:** Windows synthesises the mode from attributes (directory 0o755, read-only 0o444, otherwise 0o644); `chmod` is a no-op and `create_new` sets `FILE_ATTRIBUTE_READONLY` when no write bit is requested. The executor's mode-equality check therefore compares the read-only bit. A mode-only change of the executable bit is no change on Windows, and the suites branch their expectations accordingly.
- **Fault injection:** `Journal::simulate_full_disk` uses `/dev/full` on Linux and a read-only handle of the journal elsewhere.
- **POSIX-only tests:** `a_crash_inside_the_link_pair_leaves_both_names_and_never_loses_bytes` (Windows has no link pair). The fifo variant of `links_and_special_files_at_a_destination_are_refused_at_planning` becomes a junction variant on Windows. Every other test runs under the same name on both OSes, with per-OS branches inside.
- **Verification on this VM:** `cargo test -p studio-project -p studio-engine` all green (publish 13, cloud_placeholder 1, edit_transaction 25, task_recovery 11 + 1 ignored child, review_publication 14, engine unit 176). `cargo clippy --workspace --tests -D warnings` is clean. `studio-project` clippy is clean for `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`. `studio-engine` cannot be cross-checked here (C build scripts of alsa-sys and ring need cross toolchains), so Linux CI is the check for its `cfg(unix)` branches.
- `review_history.rs`, `preset_state.rs` and `promotion_install.rs` are still `cfg(target_os = "linux")`; phase 3 opens them with the gates.
