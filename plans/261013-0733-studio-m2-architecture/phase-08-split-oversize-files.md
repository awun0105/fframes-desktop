---
phase: 8
title: "Split the remaining oversize files"
status: done
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

- [x] Re-measure
- [x] Source splits
- [x] Test splits
- [x] Size gate passes

## Success criteria

Merge gate 6 prints nothing; all tests pass with the same test count as before the splits.

## Progress (2026-10-14)

Re-measured: the table's three moved test files did not move in phase 1. `agent_workflow.rs` (2,969), `agent_tools.rs` (2,748) and `candidate_runner.rs` (1,606) are in `desktop/app/tests`, because they start the app's bins. `acp_v1.rs` had grown to 1,831 lines.

Done, one commit per file, each a pure move. A line-multiset diff against `HEAD` showed only module wiring, module doc headers and rustfmt reflows. The only edits were the visibility widening that a module boundary needs: `pub(super)`, and `pub(in crate::driver)` for `Shared`'s methods that `driver::transport` calls. The test-count sequence per binary was unchanged (822 passed, 14 ignored).

| File | Layout |
|------|--------|
| `studio-bootstrap/src/process.rs` | `process/{mod, unix, windows, tests, job_escape_tests}.rs` (Linux and macOS group scans share `unix.rs`; `windows.rs` is also compiled for tests) |
| `studio-harness/src/driver/runtime.rs` | `runtime/{mod, shared, connection, handle}.rs` along its own section markers |
| `studio-engine/src/candidate_validation.rs` | `candidate_validation/{mod, capture, report, probe, coverage, validation}.rs`; routing stays in `mod.rs` |
| `studio-engine/src/agent_task.rs` | `agent_task/{mod, draft, tests}.rs` |
| `studio-harness/tests/acp_v1.rs` | `tests/acp_v1/{main, negotiation, turns, services, scenarios}.rs` |
| `studio-engine/tests/candidate_validation.rs` | `tests/candidate_validation/{main, capture, coverage, failures, audio, controller}.rs` |
| `app/tests/candidate_runner.rs` | `tests/candidate_runner/{main, validation, tools, broker}.rs` |

A test's root moved to `tests/<name>/main.rs`, which Cargo still names `<name>`, so `--test <name>` in CI is unchanged. A root at `tests/<name>.rs` cannot hold child modules without `#[path]`. Verification: `studio-bootstrap` and `studio-harness` pass clippy with `-D warnings` for Windows, Linux and macOS targets. `studio-engine` and the app crate were checked on Windows only, because sqlite and ffmpeg block cross-checking. The Unix-only `candidate_runner` broker tests call only helpers in their root, never a sibling module's.

Left: the two Unix-gated app tests (after phase 3, which edits them), and the gate's other offenders, which phases 4, 5 and 6 split (`studio_shell.rs`, `actor.rs`, `conversation_panel.rs`).

## Progress (2026-10-15)

The two Unix-gated app tests are split, one commit each, as pure moves:

| File | Layout |
|------|--------|
| `app/tests/agent_tools.rs` | `tests/agent_tools/{main, contract, broker, slow_peers, client, mcp, subprocess}.rs`; the shared helpers `never`, `private_copy`, `rpc` and `initialize` are in `main.rs` |
| `app/tests/agent_workflow.rs` | `tests/agent_workflow/{main, readiness, happy_path, manual_review, queueing, undo, repair, stop, permissions, failures, close, recovery, preview_handoff, resource_bounds, cycles, regressions, real_sdk}.rs`; the `Sink` impl, `wait_ready` and `World::wait_phase_snapshot` are in `main.rs` |

Checked with a normalized line-multiset diff and the same `#[test]` and `#[ignore]` counts. Linux CI ran the same totals as before the split: `agent_tools` 39 passed; `agent_workflow` 67 passed, 2 ignored. Phases 4, 5 and 6 split the shell, the actor and the conversation panel. `git ls-files 'desktop/*.rs' | xargs wc -l | awk '$1>1500'` prints nothing.
