# Phase 1: Windows escape scan

## Context

`TrackedChild::escaped_descendants` (`desktop/crates/studio-bootstrap/src/process.rs`) is
Linux-only and returns an empty list elsewhere. It feeds the live driver
(`studio-agent-spike/src/driver/runtime.rs` 649, 1306, 1412), so this changes production
behaviour on Windows, not only evidence.

## Requirements

- Member history: each scan queries `JobObjectBasicProcessIdList` and records every job
  member seen (pid + creation time) in the tracker. Walking down from the leader alone
  misses orphans whose parent already exited, which is exactly the teardown case.
- A process is escaped when it is alive, not in the job (`IsProcessInJob`), and its
  parent pid is a recorded member whose creation time is at or before the process's own
  (rules out pid reuse).
- `OpenProcess` failing because the process is gone means not escaped; only access denied
  on a live candidate counts as escaped (unverifiable never passes). Otherwise ordinary
  short-lived children would demote real tasks to Detached.
- Detection is a pure function over (snapshot, member history), like Linux
  `escaped_in_scan`, unit-tested with synthetic data.
- Bounded: one snapshot per call, no waits.

## Limits to state (docs + record)

A job without breakaway contains every process the adapter creates directly. Work handed
to a pre-existing service (COM out-of-process servers, Task Scheduler, WMI) is not a
descendant and not observable; the scan cannot prove its absence.

## Files

- `desktop/crates/studio-bootstrap/src/process.rs`; `Cargo.toml` windows-sys features.

## Validation

- Synthetic tests: orphan of a dead member detected; pid-reuse (child older than parent)
  ignored; exited process ignored; in-job process ignored.
- Live Windows test: test-only spawn option with `JOB_OBJECT_LIMIT_BREAKAWAY_OK`; child
  starts a grandchild with `CREATE_BREAKAWAY_FROM_JOB`; scan reports it.
- Existing process tests pass on Windows.

## Risk / rollback

Read-only scan. Revert the function to the empty list.
