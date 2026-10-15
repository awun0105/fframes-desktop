---
phase: 7
title: "Remove the bounds-probe harness"
status: done
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
- [x] Parity coverage check (headless gaps filled; real-input checks lost, listed below)
- [x] Release gate replacement with refusal test
- [x] Harness code, tests, data, scripts and CI jobs deleted

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

### Step 3a done: the stress code and the qualify entries are gone

- `fframes-studio` loses `qualify-m2`, `qualify-m3` and `qualify-presentation`. The spike loses the 1,000-presentation stress (`presentation_stress.rs`, `stress_metrics.rs`, its button, the readiness file and the paint acknowledgement in `preview_element.rs`). The shell loses `start_qualification`; `shell/qualification.rs` is now `shell/observation.rs`.
- **Deviation, flag shape:** the observation is three hidden flags on `studio`: `--observe-project DIR --observe-telemetry FILE [--observe-test-writer-containment LABEL]`, not one flag that writes `telemetry()`. The X11 tests wait on keys only the agent writer produces (`agent_workflow_open`, lines 398, 683 and 1103 before the change), and they cannot run on this Windows host, so the existing `start_agent_telemetry` writer stays. The names are long because `launch.bat` and the `.deb` launcher forward user arguments after `studio`: a plain `--project` would turn a stray user argument into a usage error. Parse tests (mutation-checked: an unhidden flag, and each dropped `requires`, fail them) pin both.
- **Deviation, m5 test:** instead of deleting `m5_resource_qualification.rs`, its assertions moved to `crates/studio-engine/tests/editor_resource_limits.rs` without the timing loops, the evidence JSON and the milestone labels: no other test checks the source-index totals or the metadata byte budget at the object limit.
- `x11_shell.rs` launches `studio --observe-*` and drops the xwd/ffmpeg screenshots and their `M4_`/`M5_NATIVE_*` variables (opt-in qualification evidence); the canvas test now needs only Xvfb and xdotool.
- Deleted `m3_development_evidence.rs` and `app/examples/probe_m6_providers.rs` (`probe_adapter` keeps callers in `agent/jobs.rs` and the harness tests).
- **Coverage gaps left by `m3_development_evidence.rs`** (no surviving test asserts these): a provider event flood never overflows the queue; each accepted edit compiles exactly once per cycle; CLI and MCP parity for `project_context`, `timeline` and `inspect` (only `render_frame` is compared) and the CLI capability dying with its task; the resident-row limits equal their constants; adapter PIDs are not alive after close; the authentic adapter probe (needs a real adapter by design). Partially covered: tool-card row counts, leases released to zero (a support helper, not a dedicated test).
- **Bounds probes:** gpui's `VisualTestContext::debug_bounds` with `.debug_selector` could replace them in the headless parity tests, but the X11 tests read bounds from a real process's telemetry, so the probes stay.

### Step 3b: the X11 shell tests run in full CI

- The Linux full job runs `x11_shell` (`--ignored`, one test at a time) after the audio parity step, whose installed SDK (`$RUNNER_TEMP/m2-home/.fframes/sdk/active`) is the tests' `SDK_ACTIVE`; the canvas test installs its own from `SDK_BUNDLE`. The step exports `CARGO_HOME` and `RUSTUP_HOME` from the job's home, because the tests give the app process its own `HOME`.
- These tests had never run in CI before this step; its first full run is their first run on the runner.
- That first full run (38674459610, commit `037b391`) failed two of the three tests, on snapshots read mid-transition: AutoApply released the range candidate's evidence between two telemetry reads, and the staged-preview hand-off was read while a newer frame request was in flight (`seek_serial` 9, displayed `frame_serial` 8). `66afd89` makes the range wait accept the released candidate record (same frames, a revision other than the base); that test then passed. Its hand-off change was wrong: it waited for the `displayed` record's frame serial to equal the newest seek serial, but that record is the install snapshot and frames requested after the install are presented without changing it, so the second full run (38677705719) timed out with `presented_serial` 9 equal to `seek_serial` 9 and the record at 8. The original assertion had the same flaw. `dd15d25` makes the staged-preview and Undo hand-offs wait for the newest seek to be presented and assert that the installed frame answers the seek the install carried, a serial that did not go backwards. The file compiles only on Unix, so CI is its only check.
- Observation for the next session, not changed here: the staged preview installs at a stale scale (448 px wide on a 540 px canvas in the first run) and is re-rendered at the canvas scale right after the hand-off.

### Step 4 done: the scripts and ledgers are gone

- Deleted `desktop/qualification/` (247 files), `qualify-linux-native.py`, `qualify-m2-playback.py`, `qualify-m3-agent.py`, `qualify-m5-selection.py`, `qualify-m6-providers.py`, `qualify-windows-native.py`, `test-qualification-m4.py` to `m7.py` and `validate-qualification.py`. Nothing in packaging, CI or the Rust sources read the folder (the app reads only `<data>/qualification/`; `provider_profiles.rs` uses its own fixture copy).
- Docs: links to the ledgers point at commit `037b391`, the last commit with the files. `phase-zero-feasibility.md` replaces the harness sections (isolated Linux, Windows harness, qualification evidence, the M3 harness) with a "Qualification history" section that links to that commit and keeps what still holds; links to the old M3 anchor point at it. `architecture.md` names the installed ledger as the source of provider status.
- Success grep (`qualify_|qualification::|presentation_stress|stress_metrics|bounds_probe|start_agent_telemetry` in `desktop/**/*.rs`) matches only `qualify_writer` and its subcommand, `start_agent_telemetry` (the observation flags' writer), and the bounds probes (`kit/interact.rs` and its users, the shell and the conversation panel), all kept by the decisions above. The kept probes all still exist.
- For the user: the self-hosted Windows runner no longer needs a visible RDP session; nothing runs the native Windows harness any more.
- **Open: all CI workflows pass.** Every full self-hosted run now fails the X11 step on its last wait in `the_native_shell_runs_a_task_adopts_the_staged_preview_and_undoes_it`: the agent panel's `owned_processes` (the controller's process root, `root_scope.active_count()`) stays 2 for the whole 240 s after the task's Undo, where the test allows at most 1, the displayed preview's worker (run 38680984890 on `dd15d25`; the range-scope and starter-title tests pass). It is 0 when the workflow opens and already 2 after the first accept, before any Undo (run 38674459610), also after the range test's AutoApply accept. The launch path did not change (the old `qualify-m3` and the observation flags call the same `start_agent_telemetry`); the test last passed on 2026-10-05, run by hand. By the code, the adapter and the tool workers are stopped at task end (`agent/actor/finish.rs:181`, `:276`) and the displayed preview worker is kept by design; `on_undone` shuts only the Undo's compiler scope (`finish.rs:86`), leaving its worker scope to the adopted preview, but that cannot explain the 2 seen before any Undo. The second process is unidentified; listing the PIDs and command lines on the Linux runner during the wait is the first step of the fix. Whether the opening 0 already includes a displayed preview is not known, so the right bound may be 0. This is left as its own fix, outside this phase and phase 8: the wait is neither loosened nor ignored. Until it is fixed, no full run is green, so no tag passes the release gate.
- **Open success criteria.** A release dry run with `consumer_release_enabled` set has not run: the committed policy must stay disabled, so it is left to the user (dispatch Desktop Release with `dry_run` and a policy enabled only for that run). The release workflow has been shown to load since its fix, never to execute; `desktop.yml` runs only on a pull request or a dispatch, and its edits have not run either.

### Parity coverage check

The deleted harnesses were compared with the surviving tests. Play, pause, seek, step, mute, audio output, keyboard and build behaviour already had default-run parity or engine tests. The checks with no equivalent that run headlessly now have tests, each mutation-checked:

- A failed rebuild keeps the displayed revision with matching audio, and restoring the source then building again reconciles (`services::a_failed_rebuild_keeps_the_displayed_preview_until_a_build_succeeds`; caught a shell that drops the displayed preview on failure).
- Playing to the end stops on the last frame and play replays from the start (`transport::playing_to_the_end_stops_on_the_last_frame_and_play_replays`; caught removing the rewind at the end).
- Playback keeps image release failures at 0, resident and queued images at 2 or fewer, and the pump's queue high-water marks at 1 frame and 12 thumbnails (`transport::playback_keeps_images_and_requests_within_their_bounds`). The test project never asks for more than 12 thumbnails, so a raised cap passed it; the cap has its own unit test (`preview_coordinator::tests::at_most_twelve_thumbnails_wait_however_many_are_visible`; caught a cap of 40).
- The timeline's Fit button fits a zoomed timeline again (`timeline::the_fit_button_fits_a_zoomed_timeline_again`; caught a no-op click).

Lost, since they need a real window, real input or real timing: closing through `WM_DELETE_WINDOW` and reaping the workers, device-loss retry, seek p95 timing and the RSS slope, X11/Win32 event delivery and foreground focus, and screenshots. On Linux the X11 shell tests cover part of the event delivery. The 1,000-presentation stress was removed by decision.
