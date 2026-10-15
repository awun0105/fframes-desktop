---
phase: 7
title: "Remove the bounds-probe harness"
status: in-progress
priority: P1
effort: "2-3d"
dependencies: [4]
---

# Phase 7: Remove the bounds-probe harness

## Goal

Delete the bounds-probe qualification harness, its evidence, scripts and CI jobs now that parity tests cover the behavior, and replace the release workflow's qualification gate with an explicit one. Production safety checks stay.

## Keep (production code, not harness)

- `studio-engine::containment` (moved in phase 1), including ledger verification and `containment::qualify_writer` with the `qualify-writer` subcommand in `app/src/main.rs`. Until M3 phase 4 replaces the ledger with runtime checks, they are the only way a real adapter becomes `ProcessGroupContained`; deleting them here would block real agent apply on Linux. M3 phase 4 deletes them.
- Capability probes: `probe_no_clobber` and `probe_mechanism` (`studio-project/src/publish.rs`), `probe_apply_gate` (`edit_transaction.rs`), `probe_adapter` (`studio-harness` discovery), `probe_candidate` (`studio-bootstrap/src/process.rs`), `probe_bundled_compiler` (`studio-sdk/src/doctor.rs`), `CandidateProbe` (`candidate_validation.rs`).

## Delete (verify each by scouting)

- Code: the `qualify-m2`, `qualify-m3` (with its `--test-writer-containment` ownership injection at `main.rs:171`) and other qualification subcommands except `qualify-writer`; the qualification and telemetry parts of the shell, `timeline_view`, the conversation panel UI and `conversation_panel/qualification.rs` (minus what phase 1 moved); `presentation_stress.rs`, `stress_metrics.rs`, `evidence_preview.rs` (minus `decode_png`, moved in phase 1), the `ui/probe` component, `desktop/app/examples/probe_m6_providers.rs`.
- Tests: `m3_development_evidence.rs`, `m5_resource_qualification.rs`.
- Data: `desktop/qualification/` once the release gate below no longer reads it. The runtime ledger lives in `<data>/qualification/` on each machine, not in the repository, so this does not affect writer qualification.
- Scripts: `desktop/scripts/qualify-*.py`, `test-qualification-m4.py` to `m7.py`, `validate-qualification.py`. `package-phase-zero.py` reads `desktop/qualification/m0-results.json`; drop that input in the same commit.
- CI: qualification and M2 stress jobs in `.github/workflows/desktop.yml` and `desktop-self-hosted.yml`.

## Tasks

1. Scout every reference to the delete list across `desktop/`, `.github/`, `justfile` and `docs/`.
2. Confirm each parity item has a gpui test; add any missing one before deleting.
3. Release gate replacement in `.github/workflows/desktop-release.yml` (today at around `:187-188`, `:397-411`, `:453`): an `eligible_targets` list in the release policy file, a package verifier kept as a non-qualification script (`desktop/scripts/verify-package.py`, moved out of `test-qualification-m7.py`), and a check that the tag commit equals the commit of a green `full` run of `desktop-self-hosted.yml` and a green `desktop.yml` run on `main` (a fast-only push run does not count). Add tests that an ineligible target and a tag with only a fast run are refused.
4. Delete the list above.
5. Note for the user: the self-hosted Windows runner no longer needs a visible RDP session.

## Todo

- [x] Scout references
- [ ] Parity coverage check
- [x] Release gate replacement with refusal test
- [ ] Harness code, tests, data, scripts and CI jobs deleted

## Success criteria

`grep -rnE "qualify_|qualification::|presentation_stress|stress_metrics|bounds_probe|start_agent_telemetry" desktop --include=*.rs` matches only `studio-engine/src/containment/` and the `qualify-writer` subcommand (both deleted in M3 phase 4); the kept probes still exist; the containment ledger tests and `qualify_writer` tests pass; all CI workflows and a release dry run with `consumer_release_enabled` set pass.

## Scouting results and decisions (2026-10-15)

The scout found these against the lists above:

- **The X11 tests need `qualify-m3`.** `app/tests/x11_shell.rs` drives a real window (Xvfb and xdotool) in three tests. They launch `fframes-studio qualify-m3 --project ? --telemetry ? --test-writer-containment x11-scripted-peer`, read the telemetry file, and click at the button bounds the telemetry measures. All three are `#[ignore]` and no workflow runs them: only `qualify-m3-agent.py`, deleted here, or a person did. (The user was first told they ran in CI; that was wrong and the question was asked again.) The user decided to **keep a test-only mode and run the tests in the Linux full-run job**: `qualify-m2`, `qualify-presentation` and the stress code go, `qualify-m3` becomes one hidden observation flag on the studio command that writes `telemetry()` to a file and keeps the test-only containment injection, and the full Linux job gains a step that runs the X11 tests. M3 phase 4 deletes the mode with the ledger. The success grep therefore also matches this mode and the bounds probes it needs.
- **Telemetry is not harness.** `StudioShell::telemetry` (`shell/qualification.rs`) and the conversation panel's `telemetry` are what the parity tests read. They stay. Only `start_qualification` (the `qualify-m2` file writer) goes; `start_agent_telemetry` becomes the observation flag's writer. Once the qualification parts are gone, `shell/qualification.rs` is renamed after what it holds.
- **Bounds probes stay** (the parity harness's `report_button_bounds`, the settings parity test and the X11 test click by them), unless gpui offers a test bounds lookup that replaces them.
- **`evidence_preview.rs` stays.** It draws the agent panel's evidence images (`actor/mod.rs`, `actor/prompt.rs`); it is not harness.
- **`CandidateProbe` does not exist** under that name; the kept-probes check uses the names that exist.
- **`qualification.json` in the package stays as a file.** The finalize, installer and Windows signing scripts need it for the target triple and whether the SDK is included. `package-phase-zero.py` writes it from the build inputs instead of copying `desktop/qualification/m0-results.json`.
- **`test-packaging.py` loads `validate-qualification.py` and `qualify-m3-agent.py` and reads the m0/m2 ledgers**, and every workflow runs it. Those parts go before the scripts do.
- **No workflow runs the m7 tests**, although earlier plans say so. The new gate's tests run in the fast CI steps.
- **The release gate's runs.** `desktop.yml` has no push trigger, so "a green `desktop.yml` run on main" means a manually dispatched run on `main` at the tag commit. A self-hosted run counts as full when every job's full-only step "Real worker frame, anchor, crash and restart" succeeded; in a fast push run that step is `skipped` (checked against run 38614584722).
- **Lost with the native Windows harness.** `qualify-windows-native.py` (with `qualify-m2`) is the only tool for Windows real-input checks: the caption hit-test, a real title drag, Close's hover colour, and render counts under a real mouse. Nothing in the repository verifies these after this phase.

Order, one commit per step, each green in CI:

1. The release gate (`desktop/scripts/release-gate.py`), `desktop/scripts/verify-package.py` and their offline tests, a `dry_run` input, and `eligible_targets` in the release policy. The release workflow stops reading `desktop/qualification/` and the m7 script.
2. `test-packaging.py` and `package-phase-zero.py` stop reading the ledgers and the deleted scripts. The workflows drop the m0/m2 ledger validation, the M2 stress step and the isolated-account step.
3. Rust: the subcommands, the stress code, the probe example and the m3/m5 evidence tests.
4. The scripts, `desktop/qualification/` and their docs references.

### Step 1 done: the release gate

- `desktop/scripts/release-gate.py` refuses unless the policy enables consumer releases, lists `eligible_targets` that the release builds, and the tag commit has a successful full self-hosted run (every job's full-only step succeeded) and a successful `Desktop CI` run on `main`. The workflow fetches the runs with `gh run list --commit` and `gh run view --json`, and the script decides offline.
- `desktop/scripts/verify-package.py` replaces the m7 script's package check. The old check compared the archive's digest with the one the M7 ledger recorded for a qualified candidate; with the ledger gone, it checks the target is eligible and the archive holds exactly the files its signed-and-finalized `inventory.json` lists, with matching sizes and digests, all under one folder named like the archive (CI test builds are `studio-test-<target>`, release builds `fframes-studio-<target>`).
- The release policy gains `eligible_targets` (empty) and loses `qualification.trusted_evidence_keys`, which only the m7 script read. Consumer release stays disabled in the committed policy; a test asserts that.
- `dry_run` (with `consumer_release`) runs the gate and the verifier and skips publishing.
- `desktop/scripts/test-release-gate.py`: 23 offline tests, run in the fast steps of all three workflows. Refusals tested: disabled policy, no eligible target, an eligible target the release does not build, an ineligible package target, only a fast run, one job fast, a failed run, a run of another commit or workflow, a Desktop CI run off `main`, and a package with a changed, unlisted, missing or out-of-root file. Eight mutations of the checks each fail the tests. A test also ties the gate's workflow and step names to the workflow files, and that `inputs.full` gates the step in each job.
- Against real runs (read-only `gh`): the fast push run of `9e82d3c` is refused as not full; the full dispatched run of `5cdf529` passes the full-run check and is refused only for having no `Desktop CI` run on `main`. The Linux package of that run (366 MB, made by `package-phase-zero.py`) verifies, and a copy with one byte of `qualification.json` changed is refused.
- Not done here: a live dispatch of the release workflow with `dry_run`. It needs a tag and a policy with consumer release enabled, which must not be committed; it is left to the user.

### Step 2 done: packaging and CI stop reading the ledgers

- `package-phase-zero.py` writes `qualification.json` from `package_record(target, sdk_included, build_seconds)`: the timestamp, the target platform, the build time, and `build_evidence` with `native_build` and `sdk_included`. It no longer copies `desktop/qualification/m0-results.json` with its gates. The finalizer's tests now build their record with `package_record`, so they break if the two drift apart.
- `test-packaging.py` drops the tests of the scripts being deleted: the m0/m2 ledger tests in `ArtifactTests`, `M3LedgerTests` and `HarnessTests`. It no longer loads `validate-qualification.py` or `qualify-m3-agent.py`; 22 tests remain (3 skipped on Windows).
- CI: the m0/m2 ledger validation goes from all three workflows (the step is now "Packaging and release gate tests"); the Linux full job loses the M2 stress step, its advisory report and the isolated-account presentation step, with their evidence paths, PIL/PulseAudio/`unshare`/`setpriv` prerequisites and the passwordless-sudo check. Xvfb and xdotool stay for the X11 tests.
- `docs/desktop/self-hosted-runners.md` matches: the runner needs no sudo any more, and an existing `/etc/sudoers.d/github-runner` can be removed by its admin (not done here).
- `docs/desktop/phase-zero-feasibility.md` no longer lists the removed CI steps. Its "remain subject to the qualification gates below" clause (same paragraph) and the ledger sections go with the ledgers in the script removal step.
- The only readers of the package's `qualification.json` are `finalize-native-package.py` (reads `target_platform.triple` and `build_evidence.sdk_included`, both still written), and the installer and signing scripts, which only require the file to exist.
