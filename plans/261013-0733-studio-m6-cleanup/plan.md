---
title: "M6 Cleanup: no compatibility code, spike removal, flaky tests, docs, platform checks"
description: "Remove the spike and every compatibility path for old code and old on-disk formats (old data is refused, never repaired), fix the intermittent tests and handed-over limitations, update the docs, and hand-check Windows; macOS is covered by CI only."
status: in-progress
priority: P2
effort: "8-11d"
branch: feat/studio-m6
tags: [desktop, refactor, breaking-change, docs, tech-debt, tests]
blockedBy: []
blocks: []
created: 2026-10-13
---

# M6 Cleanup

## Overview

Part of [the studio rewrite roadmap](../261013-0733-studio-rewrite-roadmap/plan.md). M4, M5a and M5 are merged on this branch.

**Decision 2026-10-17 (plan owner): breaking changes, no compatibility with old code or old formats.** This reverses the migration criteria of M4 (pre-M4 projects) and M5 (the single-session layout). The completed M4 and M5 plans are left as they were.

Every removal follows one rule. **Old data is refused, never repaired or destroyed.** A file in an old format is left on disk byte for byte. The app gives a clear "made by an older Studio" refusal, and no recreate, quarantine or repair path runs on it. The exception is an old layout that is simply ignored, which is acceptable when nothing is deleted: the old session files, and the old `agent-adapter.json`, after which Setup is shown again.

Scope comes from [the compatibility inventory](./reports/compat-inventory.md) and [the readiness report](./reports/m6-readiness.md).

Out of scope:
- the `gray_*_legacy` theme tokens, which are only names;
- renaming the `phase-zero` script files, which push CI cannot exercise; only the bundle id and the docstrings change;
- the two uncommitted test edits left in the main checkout by a finished session, which stay for the user.

## Phases

| # | Phase | Effort | Depends on | Status |
|---|-------|--------|-----------|--------|
| 1 | [Remove the spike](./phase-01-remove-spikes-and-old-modules.md): spike files, `--spike-ui`, the harness's handwritten ACP session, bundle id `studio.fframes.spike`, unused app dependencies; `worker_roundtrip` rewritten as a real test on `WorkerClient` | 1-1.5d | none | Pending |
| 2 | Agent data without compatibility: session layout migration, manifest v1, `agent-adapter.json` and registry v1 ([inventory](./reports/compat-inventory.md#agent-sessions-phase-2)) | 1.5d | 1 | Pending |
| 3 | Durable data without compatibility: journal formats 2 and 3, `FileId` aliases, old-record defaults, source inventory v0 and the Timing hash identity, SQLite v1 ([inventory](./reports/compat-inventory.md#durable-data-phase-3)) | 1.5-2d | 2 | Pending |
| 4 | Project migration removed: pre-M4 projects, `LegacyFramework`, optional SDK manifest fields, the migration UI ([inventory](./reports/compat-inventory.md#project-migration-and-the-old-sdk-framework-phase-4)) | 2d | 3 | Pending |
| 5 | [Flaky tests and leftover limitations](./phase-05-flaky-tests-and-leftover-limitations.md), on the code phase 4 leaves; the export-cancel hang is measured first | 2d | 4 | Pending |
| 6 | [Docs, roadmap and Windows hand checks](./phase-06-docs-and-platform-checks.md) | 1.5d | 5 | Pending |

The phases run in sequence in one worktree, each by one agent. Each phase passes fmt, clippy, the affected test sets and full parity, and is then pushed to `feat/studio-m6`.

Phases 2 to 4 change how durable data is handled. A reviewer checks each diff, focusing on the refusal paths, before it is committed.

`feat/studio-rewrite` is fast-forwarded only when every phase is green on all-platform CI.

Rules for phases 2 to 4:
- A `#[serde(default)]` becomes required only where the current writer always emits that field. A round-trip test of a current record with every optional field empty proves which fields stay optional.
- Making SDK manifest fields required is done in the same commit that re-assembles `packaging/sdk/phase-zero-sdk.json` and passes `scripts/test-packaging.py`.
- Hashing Timing as Timing is done in the same commit that bumps the inventory version.

## Real tests

Every deleted compatibility test is replaced by a refusal test. It writes the old format, opens it through the real code path, asserts the specific message, and checks that the file is byte-identical afterwards.

The spike-based worker test becomes a real test on the live worker transport.

Phase 6 runs the hand checks in the real app on Windows. That needs the user's RDP session to stay visible, so the user is asked when phase 6 is ready.

## Success criteria

- [ ] No `spike` text remains under `desktop/` or `.github` (plans excluded). The app crate holds only `main.rs`, `assets.rs`, an assets-only `lib.rs` and the bins. The CI step "Real worker frame, anchor, crash and restart" runs a real test.
- [ ] `legacy|migrat|OLDEST_|format 2|dev:ino` in `desktop/**/src` only hits an allow-list kept in phase 6: `NewerFormat` refusals, crash recovery, ACP v1 negotiation, and words used in another sense.
- [ ] Each old format in the inventory has a refusal test that leaves the file byte-identical.
- [ ] `services::cancelling_an_export_leaves_no_file` has a proven cause and passes in seconds, 30 consecutive runs on Windows and 3 on self-hosted Linux CI. The other intermittent tests that still exist pass 30 consecutive runs each.
- [ ] Every M5 and M5a handed-over limitation still relevant after phase 2 is fixed with a test, or recorded as an owner decision.
- [ ] `docs/desktop` and `AGENTS.md` describe the real system, and their links resolve.
- [ ] Every roadmap Done-means item is hand-checked on Windows in `reports/platform-checks.md`. macOS is marked not fully tested: green CI only (decided 2026-10-16).
