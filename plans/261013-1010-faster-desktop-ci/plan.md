---
title: "Faster desktop CI: caches, one hosted workflow, heavy tests on self-hosted Linux"
status: completed
---

# Faster desktop CI

## Outcome

Desktop checks on a PR finish in roughly 15-25 minutes instead of ~50, no test runs twice,
and nothing the old workflows proved is lost.

Baseline (PR #2, last full run): hosted Desktop CI ~50 min, hosted "Desktop feasibility
artifacts" ~30-40 min (mostly the same steps), self-hosted Windows ~33 min, self-hosted
Linux ~6.5 min (unit tests only).

## Decisions (user, 2026-10-13)

- Cache the build and the FFmpeg install; remove the duplicate workflow; move the heavy
  Linux steps to the self-hosted Linux runner.
- Hosted Ubuntu runs on every PR, but only what needs a clean sudo-capable VM: build (cached),
  SDK, package and the isolated-account presentation test.
- PRs from forks get hosted fmt/test/clippy; own PRs skip it (self-hosted covers them).
- Revised the same day: all heavy work runs self-hosted. Self-hosted Linux also packages and
  runs the presentation check as the runner user; the root-only isolated-account variant runs
  on hosted Ubuntu for pushes to `main` and manual runs only. Self-hosted test builds are
  uploaded as `studio-test-<target>` for download, never under the release `fframes-studio-*`
  name. Setup guide: `docs/desktop/self-hosted-runners.md`.
- Revised again: the runner user has passwordless sudo, and the user accepted root-for-pushers
  on that machine, so the isolated-account test runs on self-hosted Linux for every push and
  hosted Ubuntu runs only the tests for pull requests from other repositories.

## Constraints

- Public repo: self-hosted runners stay push-only, never `pull_request`.
- The isolated-account test needs root; the self-hosted Linux runner user has passwordless sudo
  (see the second revision above).
- A package built on the self-hosted VM must not be uploaded under a name the release
  workflow collects (`fframes-studio-*.zip` from any run id).
- FFmpeg bakes its install prefix into its pkg-config files: cache and restore it at the same
  path.
- The SDK assembly is a deliberately clean offline build; it is not cached.

## Changes

1. `.github/actions/desktop-linux-setup` (new composite): Python, Rust 1.98.1 with the build
   cache on `desktop/target`, apt prerequisites, FFmpeg restored from cache or built.
2. `desktop.yml`: only job `fork-tests` (tests and lint for pull requests from other
   repositories). Everything else moves to the self-hosted workflow.
3. `desktop-phase-zero.yml`: deleted; its one unique step (package and presentation check) runs
   in the `desktop-self-hosted.yml` Linux job.
4. `desktop-self-hosted.yml` Linux job: persistent build folder and FFmpeg in the runner's
   tool cache; worker, roundtrip, SDK, managed, portable, preview/timeline, audio parity,
   advisory M2 stress; prerequisite check names what is missing. It also packages the test build
   and runs the isolated-account presentation check with sudo (see the revisions above).
5. `test-qualification-m7.py`: the Windows FFmpeg pin check reads the workflows that pin it
   (it currently fails on `main` because two listed workflows never contained the pin).
6. `docs/desktop/phase-zero-feasibility.md`: workflow references.

## Validation

- YAML parses; `test-qualification-m7.py`, `test-packaging.py` pass locally.
- The PR's own run: every job green; compare times with the baseline. The first run only
  fills caches (caches made on a branch serve that branch; caches made on `main` serve all).

## Results (2026-10-13)

- Local: actionlint passes on both workflows; `test-packaging.py` (60 tests, 6 skipped) and
  `test-qualification-m7.py` (11 tests) pass.
- Self-hosted run 38364760112 (warm build folders, every job green): Windows 30 min, Linux
  38.5 min. The Linux job is about 21 min without the advisory M2 stress, which now runs its
  full 2001 seeks and 50 rebuilds (17.5 min) instead of stopping early. Hosted Desktop CI is
  skipped for own pull requests and runs only for pull requests from other repositories.
- Baseline per PR was hosted Desktop CI ~50 min plus ~30-40 min of duplicate feasibility jobs
  on hosted runners, next to self-hosted Windows ~33 min; that hosted work is gone.
- The 15-25 min target is not met. Without the M2 stress a push takes about 30 min, set by the
  Windows job's 18 min integration step; with it, 38.5 min. Whether the M2 stress stays on every
  push is an open user decision (see the M2 status below).

## Risks

- Self-hosted Linux may lack FFmpeg build tools, Xvfb or PulseAudio: the prerequisite step
  fails with the exact apt line for an admin; M2 stress stays advisory.
- Persistent build folder grows on the runner's disk; the job prints free space.

## M2 stress status (2026-10-13)

- Two driver bugs fixed: a seek fraction that rounded onto the ruler's right edge clicked outside
  its hit area (the run then timed out), and the 100 ms pointer settle sleep was counted inside
  every seek latency sample.
- An advisory failure now raises a warning annotation and a step summary line; the log prints a
  latency summary.
- On the self-hosted Ubuntu runner the full run (2001 seeks, 50 rebuilds) takes about 17.5 min
  and fails only the paused seek p95 budget: p50 342 ms, p95 465 ms, max 684 ms against 150 ms
  (reference run 2026-10-04: p95 96 ms on another machine). The preview pane is now 566x288 at
  scale 0.4; the reference pane was 718x118-132 at scale 0.16-0.18. Whether the gap is the
  machine or the app is open; the step stays advisory until the user decides.
