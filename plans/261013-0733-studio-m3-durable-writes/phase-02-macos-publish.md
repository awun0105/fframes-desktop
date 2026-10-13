---
phase: 2
title: "macOS publication and executor"
status: pending
priority: P1
effort: "3-4d"
dependencies: []
---

# Phase 2: macOS publication and executor

## Goal

Publication, the executor and crash recovery work on macOS (Apple Silicon included) with the same guarantees, tested on the `macos-15` CI job added in M1.

## Tasks

1. Implement `publish/macos.rs`: `openat(O_NOFOLLOW | O_DIRECTORY)` walk; `FileId` from `fstat`; no-replace rename with `renameatx_np(..., RENAME_EXCL)`; `unlinkat`. Share the POSIX code from `publish/unix.rs` (phase 1 split).
2. `probe_no_clobber` qualifies `RENAME_EXCL` on the project volume and refuses where it fails (SMB and similar).
3. Run the executor (`executor_unix.rs`) on macOS; macOS fault injection replaces `/dev/full`.
4. Run `tests/publish.rs` and the three engine recovery suites on the macOS CI job.

## Todo

- [ ] macOS publication
- [ ] Probe
- [ ] Executor and fault injection
- [ ] Suites green on macOS CI

## Success criteria

The four suites pass on `macos-15` with the same test names as on Linux.
