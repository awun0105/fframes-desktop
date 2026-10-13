# Phase 4: Windows publish backend

## Context

All OS work of Apply/Undo lives in `desktop/crates/studio-project/src/publish.rs`
(`Dir`, `FileId`, `EntryStat`, `probe_no_clobber`, about 35 libc call sites). The
transaction machine (`studio-engine/src/edit_transaction/linux.rs`) and planner make no OS
calls and are gated by `cfg(target_os = "linux")` only.

## Requirements

- `Dir` on Windows holds a directory handle (`FILE_FLAG_BACKUP_SEMANTICS`); every open is
  relative to it (`NtCreateFile` with `RootDirectory`) with `FILE_OPEN_REPARSE_POINT`, so
  links and junctions are never followed; a reparse point is reported as a link.
- `FileId` = volume serial + 128-bit file id (`FILE_ID_INFO`).
- `rename_noreplace`: `SetFileInformationByHandle(FileRenameInfo)` with
  `ReplaceIfExists = FALSE` and `RootDirectory` = target dir handle: atomic, fails if the
  name exists.
- `unlink`/`rmdir`: `FileDispositionInfoEx` (POSIX semantics where supported).
- `sync`: `FlushFileBuffers` on files; directory sync is a no-op on NTFS (metadata is
  journaled) and is documented as such.
- `open_root`/`check_root_binding`: walk the path component by component without
  following reparse points; re-walk to detect a replaced root.
- Modes: Windows has no executable bit. Rule: a file's mode is recorded as written by
  the plan; `chmod` only toggles read-only for the write bit; exec bits are preserved in
  the record and not applied. Stated in docs.
- `probe_no_clobber`: prove the rename refuses an existing name on the project volume
  (NTFS/ReFS); FAT/exFAT or network shares that cannot prove it stay Blocked.
- Lift `cfg(target_os = "linux")` on the transaction machine and Apply gate to
  `any(linux, windows)`.

## Also required (review findings)

- Transient `ERROR_SHARING_VIOLATION` (Defender, indexer, open editors): bounded retry,
  then halt as a conflict that retains every variant, never mid-step.
- Root binding compares case-insensitively and normalizes 8.3, `\\?\` and UNC forms;
  network shares are blocked.
- Modes: find every capture (incl. `source_index`) and exact-mode verification and define
  the Windows rule there, so a 0o755 candidate file can verify.
- `link_noreplace`: implement with `FileLinkInformation` (replace FALSE) or block it, after
  checking whether the machine uses it for private staged names.
- No module rename; only widen the `cfg`.

## Files

- `desktop/crates/studio-project/src/publish.rs` (split into `publish/unix.rs`,
  `publish/windows.rs` behind the existing API).
- `desktop/crates/studio-engine/src/edit_transaction.rs` and `edit_transaction/linux.rs`
  (cfg widening; rename module to `fs_machine.rs` if it stays shared).
- `docs/desktop/README.md` (Apply on Windows, limits).

## Validation

- Run `studio-project` publish tests and `studio-engine` transaction tests on Windows,
  including the editor-race and rollback cases.
- `desktop/app/tests/agent_workflow_apply_gate.rs` on Windows.

## Risk / rollback

Highest-risk phase (writes user source). Keep Apply Blocked on Windows until every
transaction test passes on this host; revert by restoring the non-Linux `probe_apply_gate`.
