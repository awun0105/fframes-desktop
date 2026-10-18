---
phase: 2
title: "macOS publication and executor"
status: completed
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

- [x] macOS publication
- [x] Probe
- [x] Executor and fault injection
- [x] Suites green on macOS CI (Desktop self-hosted validation run 38753289498 and Desktop CI run 38753289654 on commit 34a342f, 2026-10-15)

## Success criteria

The four suites pass on `macos-15` with the same test names as on Linux.

## Outcome (2026-10-15)

- `publish/macos.rs` holds only `rename_exclusive` (`renameatx_np(RENAME_EXCL)`); the `openat(O_NOFOLLOW)` walk, `fstat` identities and `unlinkat` come from `publish/unix.rs`. `NoClobber::Renameat2` is the "atomic rename" mechanism on every OS and labels itself `renameatx_np(RENAME_EXCL)` on macOS.
- Probe: `is_unsupported` also counts `ENOTSUP`. On macOS that is 45 while `EOPNOTSUPP` is 102, and `RENAME_EXCL` on SMB or FUSE volumes answers `ENOTSUP`. Such a volume reports the rename as unsupported and the Apply gate stays `Blocked`.
- Durability: `File::sync_all` issues `F_FULLFSYNC` on Apple targets in std, for files and directory descriptors alike, so no macOS-specific sync code is needed.
- The shared executor and the three engine suites plus `tests/publish.rs` are gated `any(linux, macos, windows)`. Fault injection on macOS uses the read-only journal handle (`EBADF`), as on Windows. The link-refusal text no longer names `renameat2`.
- Verified here: `studio-project` (including the `publish.rs` test target) passes clippy `-D warnings` for `aarch64-apple-darwin`. `studio-engine` cannot be built for macOS on this VM (ring's C build), so the `desktop-macos` job (`.github/workflows/desktop.yml`, already running `cargo test -p studio-project -p studio-engine`) is the first full compile and run.
- Watch on that run: temp directories live under `/var`, a symlink to `/private/var`. `Dir::open_root` canonicalises before its no-follow walk, but any test that compares the fixture's un-canonicalised path with a path the engine reports would differ.
