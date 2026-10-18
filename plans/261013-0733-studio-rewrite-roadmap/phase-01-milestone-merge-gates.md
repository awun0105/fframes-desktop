---
phase: 1
title: "Milestone merge gates"
status: completed
priority: P1
effort: "0.5d per milestone"
dependencies: []
---

# Phase 1: Milestone merge gates

## Goal

Every milestone lands on `main` only after the same gate passes, so `main` always builds, tests green and runs.

**Current decisions:** M6 uses the owner's Windows-local validation gate (2026-10-17), rather than requiring new Linux/macOS CI runs. The owner confirmed all Windows app hand checks passed on 2026-10-18 and selected a **Windows unsigned preview** for release preparation. The preview dry run therefore owns the remaining release check, and it passed on 2026-10-18 ([39110213274](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/39110213274)); consumer publication stays disabled, with its signing and provenance controls unchanged. See [platform checks](../261013-0733-studio-m6-cleanup/reports/platform-checks.md).

## Before M1

- [x] `ci/faster-desktop-ci` (PR #3) and `ci/fast-push-checks` (PR #4, `57829e4`) are merged into `main`.
- [x] `feat/studio-rewrite` is branched from the updated `main`.

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

Reconstructed from Git history on 2026-10-18. These are integration checkpoints on `feat/studio-rewrite`, not claims that every milestone was merged into `main`. M1 has the main-branch integration PR #5 (`06052b4`, 2026-10-14); the remaining milestones are on the rewrite branch. Dates below are the recorded checkpoint dates. A release tag and publication are separate operations.

| Milestone | Integration checkpoint | Commit | Notes |
|-----------|-----------|--------|-------|
| M1 | 2026-10-14 | `a85cd52` | Parity recorded green on all three platforms; foundations complete |
| M2 | 2026-10-15 | `b25c18d` | Architecture and X11 cleanup complete; later Windows caption acceptance recorded in M6 |
| M3 | 2026-10-15 | `533c00e` | Durable writes closed with hand-check evidence and accepted Linux exception |
| M4 | 2026-10-16 | `0834d05` | Editing integrated; export-comparison concern explicitly retained for follow-up, closed by `8fd8425` |
| M5a | 2026-10-16 | `deba715` | Native wires integrated; macOS coverage explicitly limited to CI |
| M5 | 2026-10-17 | `df53e56` | Agent UX complete and integrated; remaining limitations handed to M6 |
| M6 | 2026-10-18 | `8cd1a5a`, `5a3722e` | Windows fixes committed; preview dry run [39110213274](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/39110213274) passed |

## Consistency sweep

Run on 2026-10-13 after applying the red-team findings.

- Decision deltas applied: containment kept as production code (M2 p1, p7; M3 p4); timing revision outside the build key (M4 p1, p3); SDK release and migration (M4 p2, new); multi-scene template (M4 p2); `SetTiming` as a capability, no contract bump (M1 S2, M4 p1); CLI timing opt-in by builder only (M4 p1); validated partial apply and per-path revert (M5 p4); harness removed in M2 p7 instead of M6; release gate replacement (M2 p7); NT handle-relative Windows calls, widened `FileId`, executor and recovery ports, `MUTATION_QUALIFIED` removal, alias defence (M3); Windows tool broker (M3 p5, new); playback keeps `PlaybackClock` epochs (M2 p3, M4 p6); port closure spike (M1 S4); oversize splits (M2 p8, new); ACP facts, plan and command events, multi-session storage, link and attachment safety (M5); fork mirrors and `cargo deny` (M1, gates).
- Stale-term search (protocol bump, qualification until M6, tool-group folding, image-prompt gap, `--no-timing`, old phase numbers): no stale claims left; remaining hits are the red-team table and the M1 phase 2 re-baseline, which still applies because qualification lives until M2 phase 7.
- Links: every relative link in the seven plans resolves. `ak plan validate` passes for all seven.
- Cross-milestone references checked; one fixed (M3 phase start conditions in the roadmap).
- Two journal format bumps are planned in sequence (M3 phase 1 `FileId`, M5 phase 4 per-path reversal); each must decode the previous format.
- Unresolved contradictions: none.
