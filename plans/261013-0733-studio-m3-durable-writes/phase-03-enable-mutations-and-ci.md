---
phase: 3
title: "Open the gates, path aliases and CI coverage"
status: pending
priority: P1
effort: "3-5d"
dependencies: [1, 2, 4]
---

# Phase 3: Open the gates, path aliases and CI coverage

## Goal

Edit transactions, preset apply and agent apply run on Windows and macOS, protected trees cannot be reached through filesystem aliases, and CI proves it on each platform. Starts after M2 phase 3.

## Tasks

1. Scout `edit_transaction.rs` (24 `cfg(target_os = "linux")` sites on 2026-10-13), `controller/promotion.rs` (`apply_gate`), `controller/preset.rs` (`preset_mutation`), `preset_state.rs` (`MUTATION_QUALIFIED`) and their UI consumers after M2.
2. Protected paths before opening any gate: compare the excluded trees (`studio-project/src/revision.rs`, `excluded()`: `.git`, `target`, `.fframes/context`, `.fframes/cache`) case-insensitively on Unicode-normalised names; reject 8.3 short-name forms (`~N`) in `ProjectPath`; after opening each component by handle, compare its real name (`GetFinalPathNameByHandle`, `F_GETPATH`) with the requested one and refuse any delta that resolves into an excluded tree. Tests on Windows and macOS: `.Git/hooks/x`, `GIT~1/hooks/x`, `TARGET/x`, NFD variants are refused.
3. Remove the non-Linux stand-ins in `edit_transaction.rs` and call the per-OS modules from phases 1 and 2.
4. Remove `MUTATION_QUALIFIED`; `preset_mutation` and `apply_gate()` both open when `probe_no_clobber` succeeds on the project root.
5. Rewrite `studio-engine/tests/agent_workflow_apply_gate.rs` (moved in M2 phase 1; asserts apply is blocked off Linux) into a test that apply works on every OS and stays blocked when the probe fails; flip the gated parity assertions from M1 phase 3.
6. Make `preset_workflow.rs`, `scoped_editing.rs`, `promotion_handoff.rs`, `provider_handoff.rs` and the agent workflow suites run on Windows and macOS CI (they are `cfg(target_os = "linux")` or `cfg(unix)` today), porting their fixtures where needed.
7. Update the `docs/desktop/architecture.md` sentence saying mutation is enabled on Linux x86_64 only.

## Todo

- [ ] Scout
- [ ] Protected-path alias defence with tests
- [ ] Stand-ins removed
- [ ] `MUTATION_QUALIFIED` removed; gates open on probe
- [ ] Apply-gate test rewritten; parity assertions flipped
- [ ] Workflow suites on three platforms
- [ ] Docs sentence updated

## Success criteria

`grep -c 'target_os = "linux"' desktop/crates/studio-engine/src/edit_transaction/mod.rs` is 0 (the per-OS executor modules may keep theirs) and `grep -rn MUTATION_QUALIFIED desktop` is empty; preset apply and agent apply work by hand on this VM; the alias tests and the listed suites pass on Windows, macOS and Linux CI.
