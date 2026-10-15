---
title: "M3 Durable writes, containment and agent tools on Windows and macOS"
description: "Port no-clobber publication, the transaction executor and crash recovery to Windows and macOS, verify agent writer containment at runtime, run the agent tool broker on Windows, and open the mutation gates there."
status: in-progress
priority: P1
effort: "18-27d"
branch: feat/studio-rewrite
tags: [desktop, backend, security, critical]
blockedBy: [261013-0733-studio-m1-foundations]
blocks: [261013-0733-studio-m4-editing, 261013-0733-studio-m5-agent-ux]
created: 2026-10-13
---

# M3 Durable writes, containment and agent tools on Windows and macOS

## Overview

Part of [the studio rewrite roadmap](../261013-0733-studio-rewrite-roadmap/plan.md). On Windows and macOS today, four things block every write and every agent apply:

1. `studio-project/src/publish.rs` and the transaction executor `studio-engine/src/edit_transaction/linux.rs` (1,050 lines) are Linux-only; `edit_transaction.rs` compiles stand-ins elsewhere and `Controller::apply_gate()` (`controller/promotion.rs:128-140`) stays blocked.
2. `MUTATION_QUALIFIED` (`studio-engine/src/preset_state.rs:43`) is hard-wired to Linux x86_64 and blocks presets first.
3. Writer containment is "qualified" only through a ledger file and Unix process groups (`agent_task.rs:316`, `containment` after M2 phase 1).
4. The agent tool broker (`agent_tools/broker.rs`, `client.rs`) is Unix sockets only; Windows agents run without project tools.

Crash recovery suites (`studio-engine/tests/{edit_transaction,task_recovery,review_publication}.rs`) are Linux-only too.

## Phases

| # | Phase | Effort | Depends on | Status |
|---|-------|--------|-----------|--------|
| 1 | [Windows publication and executor](./phase-01-windows-publish.md) | 5-7d | M1 phase 1 | Completed |
| 2 | [macOS publication and executor](./phase-02-macos-publish.md) | 3-4d | M1 phase 1 | In progress (macOS CI run pending) |
| 3 | [Open the gates, path aliases and CI coverage](./phase-03-enable-mutations-and-ci.md) | 3-5d | 1, 2, 4, M2 phase 3 | Pending |
| 4 | [Runtime writer containment](./phase-04-runtime-writer-containment.md) | 4-6d | M2 phases 1 and 8 | Pending |
| 5 | [Agent tool broker on Windows](./phase-05-windows-tool-broker.md) | 3-5d | M2 phase 1 | Pending |

Phases 1, 2 and 5 run beside M2. Phase 3 waits for M2 phase 3 because it edits the controller and preset code M2 restructures, and for phase 4, which itself needs M2 phases 1 and 8.

## Success criteria

- [ ] The same publish, executor and recovery test lists pass on Windows, macOS and Linux CI.
- [ ] `edit_transaction.rs` has no Linux-only stand-ins; `MUTATION_QUALIFIED` is gone; the gates open on a successful runtime probe.
- [ ] Writer containment is verified at runtime on all three platforms, with no ledger file; `qualify-writer` and the ledger code are gone.
- [ ] A granted tool capability round-trips through the broker on Windows CI.
- [ ] Writes into protected trees through case, short-name or Unicode aliases are refused on Windows and macOS.
- [ ] Linux behavior and tests are unchanged.
