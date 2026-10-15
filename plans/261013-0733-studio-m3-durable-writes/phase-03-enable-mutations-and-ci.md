---
phase: 3
title: "Open the gates, path aliases and CI coverage"
status: in-progress
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
5. Rewrite `app/tests/agent_workflow_apply_gate.rs` (asserts apply is blocked off Linux; it now fails on Windows because publication landed there). Keep its ungated `cli_project_tools_remain_available_without_the_mcp_route`, which covers the CLI-only tool route on every platform, into a test that apply works on every OS and stays blocked when the probe fails; flip the gated parity assertions from M1 phase 3.
6. Make `preset_workflow.rs`, `scoped_editing.rs`, `promotion_handoff.rs`, `provider_handoff.rs` and the agent workflow suites run on Windows and macOS CI (they are `cfg(target_os = "linux")` or `cfg(unix)` today), porting their fixtures where needed.
7. Update the `docs/desktop/architecture.md` sentence saying mutation is enabled on Linux x86_64 only.

## Todo

- [x] Scout
- [x] Protected-path alias defence with tests
- [x] Stand-ins removed
- [x] `MUTATION_QUALIFIED` removed; gates open on probe
- [x] Apply-gate test rewritten; parity assertions flipped
- [ ] Workflow suites on three platforms (green on this Windows VM; Linux and macOS CI pending)
- [x] Docs sentence updated

## Success criteria

`grep -c 'target_os = "linux"' desktop/crates/studio-engine/src/edit_transaction/mod.rs` is 0 (the per-OS executor modules may keep theirs) and `grep -rn MUTATION_QUALIFIED desktop` is empty; preset apply and agent apply work by hand on this VM; the alias tests and the listed suites pass on Windows, macOS and Linux CI.

## Progress (2026-10-15)

### Protected-path alias defence (done before phase 4, so no gate opens without it)

- `studio_project::paths::PROTECTED_TREES` is the one list; `revision::excluded()` reads it and stays an **exact** comparison. **Deviation:** folding `excluded()` itself would change which files an inventory holds, for example drop a `Target/` folder on a case-sensitive Linux volume, and make older checkpoints with such paths fail `SourceInventory::verify`.
- `paths::protected_tree(path)` folds each component: it drops git's HFS+-ignorable code points, applies NFKC (`icu_normalizer` 2.3, already in the lock; default features off so no new crates) and maps `to_uppercase().to_lowercase()`, so `ı` and `ſ` reach `i` and `s`. `edit_transaction::plan` refuses any delta whose path folds into a protected tree, right after `deltas_between` and before any probe, open or reservation, as a `PlanError::Conflict` naming the tree. Preset apply and Undo plan through the same function. Studio itself never runs `git` in a draft or candidate, so `plan` is the one choke point. **Behaviour change:** this applies on every OS, as Git's checks do, so a delta under `Target/` or `.Git/` is refused on a case-sensitive Linux volume too.
- **Deviation:** `ProjectPath` does not reject `~N` components; that would make scans fail on ordinary names like `notes~1.txt`. Instead `publish::Dir::open_child` compares the name a directory is stored under with the requested one and refuses a mismatch with `InvalidInput`, never `NotFound`/`AlreadyExists`, which callers treat as ordinary states:
  - Windows uses `GetFinalPathNameByHandleW(FILE_NAME_NORMALIZED | VOLUME_NAME_NONE)`.
  - Unix reads the parent through a fresh `openat(".")` + `fdopendir` and requires an entry with exactly the requested name. It compares by name only, because `d_ino` differs from `st_ino` on overlayfs.
  - `open_root` and the root binding walk are not checked; `canonicalize` already normalises the root.
  - This catches 8.3 short names (including hashed forms), case and normalisation variants that folding cannot see.
- Tests:
  - `studio-project/tests/protected_paths.rs`: folding cases (`.Git`, `TARGET`, `.gıt`, `.fframeſ/cache`, ZWNJ, BOM, fullwidth); `open_child(".Git")` gives `InvalidInput` on a case-insensitive volume and `NotFound` elsewhere; the volume's real 8.3 name of `.git`, read with `GetShortPathNameW`, is refused (8.3 generation is enabled on this VM's C:); the inventory still excludes exactly the four trees.
  - `studio-engine/tests/edit_transaction.rs::aliases_of_protected_trees_are_refused_at_planning`: `.Git/hooks/pre-commit`, `TARGET/x.txt`, `.fframeſ/cache/x.txt` and the real short name. Each is refused with an unchanged tree (`.git` included), no transaction files and no task revision. The task ends `Conflict` with its candidate preserved, as for any unplannable delta.
- Cost: `review_publication` 111.9 s and `task_recovery` 63.6 s, unchanged from before the check.

### Gates opened (2026-10-15)

- `edit_transaction/mod.rs` has no platform gate left. The stand-ins for platforms other than Linux, macOS and Windows are gone; `studio_project::publish` has no implementation for any other platform, so they were never reachable.
- `MUTATION_QUALIFIED` is gone. `preset_mutation` and `apply_gate()` both open when `probe_no_clobber` succeeds on the project root; `apply_gate()` already did.
- `app/tests/agent_workflow_apply_gate.rs` runs on every platform: Apply publishes where the probe succeeds, and with only the link-based pair (`force_publication_mechanism(Link)`, refused by the real gate) the candidate stays reviewable and exportable with nothing written. The parity harness no longer has `mutation_open`; its preset and agent Apply cases assert the real outcome everywhere.
- Ungated suites: `agent_workflow`, `agent_workflow_ui`, `preset_workflow`, `promotion_handoff`, `provider_handoff`, `scoped_editing` (app) and `preset_state`, `promotion_install`, `review_history` (engine). Only one test needed a change for Windows: a working-copy path compared as a string with `/`. `agent_task.rs` keeps its Linux-only containment cases, which phase 4 covers.
- This VM: 1188 tests in the desktop packages, all green after that fix; clippy and fmt clean. The CI workflows already run `cargo test` for these packages on Linux, Windows and macOS, so no workflow change was needed.
- Docs: `architecture.md` (preset mutation) and the README Apply paragraph now name the per-platform rename primitives and the probe.
