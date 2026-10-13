---
phase: 1
title: "Milestone merge gates"
status: pending
priority: P1
effort: "0.5d per milestone"
dependencies: []
---

# Phase 1: Milestone merge gates

## Goal

Every milestone lands on `main` only after the same gate passes, so `main` always builds, tests green and runs.

## Before M1

- [x] `ci/faster-desktop-ci` (PR #3) and `ci/fast-push-checks` (PR #4, `57829e4`) are merged into `main`.
- [ ] `feat/studio-rewrite` is branched from the updated `main`.

## Gate checklist (run for each milestone)

1. `just clippy` and `cargo fmt --all --check` pass in `desktop/` and the root workspace.
2. `cargo test` passes in `desktop/` on Linux, Windows and macOS CI (macOS job added in M1 phase 1, runs on every pull request). Pushes run only the fast checks since `bec9275`, so the gate is a green `full` run of `desktop-self-hosted.yml` (Run workflow, `full` ticked) on the milestone's merge commit, plus a green `desktop.yml` run.
3. The parity suite passes: `cargo test -p fframes-studio --test parity` until M2 phase 4 moves it, then `cargo test -p studio-ui --test parity`.
4. Until M2 phase 7 deletes it, the qualification CI still passes (advisory from M2 phase 2, which changes layout on purpose); from then on, the release workflow's replacement gate passes a dry run.
5. `cargo deny check sources advisories` passes (added in M1 phase 2), with Cap's GPUI fork resolved from the project's mirror.
6. Size rule: `git ls-files 'desktop/*.rs' | xargs wc -l | awk '$1>1500 && $2!="total"'` prints nothing, from M2 on.
7. The app opens on this VM and the milestone's own success criteria are checked by hand. If Cap's GPUI fork does not render on this GPU-less VM (M1 spike S1), the user runs these checks on their own machine and records the result in the merge log.
8. Docs touched by the milestone are updated (`docs/desktop/`, `AGENTS.md`).
9. The milestone's `plan.md` status is set to completed and the roadmap table updated.

## Merge log

| Milestone | Merged on | Commit | Notes |
|-----------|-----------|--------|-------|
| M1 | | | |
| M2 | | | |
| M3 | | | |
| M4 | | | |
| M5a | | | |
| M5 | | | |
| M6 | | | |

## Consistency sweep

Run on 2026-10-13 after applying the red-team findings.

- Decision deltas applied: containment kept as production code (M2 p1, p7; M3 p4); timing revision outside the build key (M4 p1, p3); SDK release and migration (M4 p2, new); multi-scene template (M4 p2); `SetTiming` as a capability, no contract bump (M1 S2, M4 p1); CLI timing opt-in by builder only (M4 p1); validated partial apply and per-path revert (M5 p4); harness removed in M2 p7 instead of M6; release gate replacement (M2 p7); NT handle-relative Windows calls, widened `FileId`, executor and recovery ports, `MUTATION_QUALIFIED` removal, alias defence (M3); Windows tool broker (M3 p5, new); playback keeps `PlaybackClock` epochs (M2 p3, M4 p6); port closure spike (M1 S4); oversize splits (M2 p8, new); ACP facts, plan and command events, multi-session storage, link and attachment safety (M5); fork mirrors and `cargo deny` (M1, gates).
- Stale-term search (protocol bump, qualification until M6, tool-group folding, image-prompt gap, `--no-timing`, old phase numbers): no stale claims left; remaining hits are the red-team table and the M1 phase 2 re-baseline, which still applies because qualification lives until M2 phase 7.
- Links: every relative link in the seven plans resolves. `ak plan validate` passes for all seven.
- Cross-milestone references checked; one fixed (M3 phase start conditions in the roadmap).
- Two journal format bumps are planned in sequence (M3 phase 1 `FileId`, M5 phase 4 per-path reversal); each must decode the previous format.
- Unresolved contradictions: none.
