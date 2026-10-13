---
phase: 1
title: "Windows publication and executor"
status: pending
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

- [ ] Scout
- [ ] Module splits
- [ ] `FileId` widened, journal format bumped, old records decoded
- [ ] NT handle-relative publication
- [ ] Hard-link refusal on all OSes
- [ ] Cloud placeholder refusal
- [ ] Mode check defined
- [ ] Windows executor and recovery
- [ ] Suites green on this VM

## Success criteria

The four suites pass on this VM with the same test names as on Linux; Linux CI unchanged.

## Risks

- Antivirus or indexer handles cause sharing violations: bounded retry on `ERROR_SHARING_VIOLATION` only, then report.
