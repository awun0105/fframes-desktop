---
title: "Faster desktop CI: caches, one hosted workflow, heavy tests on self-hosted Linux"
status: in-progress
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

## Constraints

- Public repo: self-hosted runners stay push-only, never `pull_request`.
- The isolated-account test needs sudo; the self-hosted Linux runner has none.
- A package built on the self-hosted VM must not be uploaded under a name the release
  workflow collects (`fframes-studio-*.zip` from any run id).
- FFmpeg bakes its install prefix into its pkg-config files: cache and restore it at the same
  path.
- The SDK assembly is a deliberately clean offline build; it is not cached.

## Changes

1. `.github/actions/desktop-linux-setup` (new composite): Python, Rust 1.98.1 with the build
   cache on `desktop/target`, apt prerequisites, FFmpeg restored from cache or built.
2. `desktop.yml`: job `fork-tests` (fork PRs only) and job `package` (SDK, package, isolated
   presentation, artifact). Heavy integration steps and the advisory M2 stress move out.
3. `desktop-phase-zero.yml`: deleted; its one unique step lives in `desktop.yml` `package`.
4. `desktop-self-hosted.yml` Linux job: persistent build folder and FFmpeg in the runner's
   tool cache; worker, roundtrip, SDK, managed, portable, preview/timeline, audio parity,
   advisory M2 stress; prerequisite check names what is missing. It does not package: the
   Linux package is built once, on hosted Ubuntu. The SDK is assembled on both, because the
   self-hosted integration tests need it.
5. `test-qualification-m7.py`: the Windows FFmpeg pin check reads the workflows that pin it
   (it currently fails on `main` because two listed workflows never contained the pin).
6. `docs/desktop/phase-zero-feasibility.md`: workflow references.

## Validation

- YAML parses; `test-qualification-m7.py`, `test-packaging.py` pass locally.
- The PR's own run: every job green; compare times with the baseline. The first run only
  fills caches (caches made on a branch serve that branch; caches made on `main` serve all).

## Risks

- Self-hosted Linux may lack FFmpeg build tools, Xvfb or PulseAudio: the prerequisite step
  fails with the exact apt line for an admin; M2 stress stays advisory.
- Persistent build folder grows on the runner's disk; the job prints free space.
