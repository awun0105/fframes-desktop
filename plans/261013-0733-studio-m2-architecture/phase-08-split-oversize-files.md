---
phase: 8
title: "Split the remaining oversize files"
status: pending
priority: P2
effort: "3-4d"
dependencies: [1]
---

# Phase 8: Split the remaining oversize files

## Goal

Every `.rs` file under `desktop/` is at most 1,500 lines, through pure module splits with no logic change.

## Files (line counts on 2026-10-13; re-measure)

| File | Lines | Proposed layout |
|------|-------|-----------------|
| `crates/studio-bootstrap/src/process.rs` | 3,334 | `process/{mod,spawn,group_linux,group_macos,tree,probe}.rs` (Windows containment lands here in M3 phase 4) |
| `crates/studio-harness/src/driver/runtime.rs` | 2,980 | `driver/runtime/{mod,session,prompt,updates,permissions}.rs` |
| `crates/studio-engine/src/candidate_validation.rs` | 2,174 | `candidate_validation/{mod,capture,stages,report}.rs` |
| `crates/studio-engine/src/agent_task.rs` | 2,024 | `agent_task/{mod,quiescence,capture,records}.rs` |
| `crates/studio-engine/tests/agent_workflow.rs` (moved) | 2,966 | split by scenario into a `tests/agent_workflow/` module tree |
| `crates/studio-engine/tests/agent_tools.rs` (moved) | 2,748 | same |
| `crates/studio-harness/tests/acp_v1.rs` | 1,814 | same |
| `crates/studio-engine/tests/candidate_validation.rs` | 1,776 | same |
| `crates/studio-engine/tests/candidate_runner.rs` (moved) | 1,604 | same |

The ACP driver split is a pure module split; "keep the existing ACP driver" means keep its behavior and tests, not its file layout.

## Tasks

1. Re-run the size check and add any new offender to the table.
2. Split each file along its existing sections; one commit per file; move code without editing it.
3. Run the affected crate's tests after each split.

## Todo

- [ ] Re-measure
- [ ] Source splits
- [ ] Test splits
- [ ] Size gate passes

## Success criteria

Merge gate 6 prints nothing; all tests pass with the same test count as before the splits.
