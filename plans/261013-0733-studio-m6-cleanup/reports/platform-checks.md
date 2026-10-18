# Platform checks

**Priority decision 2026-10-18:** finish the visible Windows app checks first, fixing failures as they appear. The release dry run follows; additional macOS and Linux testing is outside this immediate work. Existing automated Windows results remain the baseline.

**Windows hand checks passed, owner-reported 2026-10-18.** After the rebuilt release app was launched in the owner's active RDP session, the owner confirmed: "test all okay by my hand". This closes the app hand checks below, including the prepared projects 01–05. These results are attributed to the owner's manual testing; no release dry run was reported. The tested launcher uses `C:\Users\Administrator\fframes-m6\desktop\target\release\fframes-studio.exe`, built from `84ba69c` plus the 2026-10-18 fixes, and the matching `m6-fixed-sdk` bundle.

The [hand-check guide](./hand-check-guide.md) describes the checks. Automated baseline checks were run on 2026-10-17 on Windows Server 2022 after phase 5 ([`8b1fcbf`](https://github.com/khanhthanhdev/fframes-desktop/commit/8b1fcbf)); the updated 2026-10-18 automated results are recorded below.

## Roadmap Done-means items on Windows

| # | Done-means item ([roadmap](../../261013-0733-studio-rewrite-roadmap/plan.md#done-means)) | How checked | Result |
| --- | --- | --- | --- |
| 1 | `desktop/` builds on Cap's GPUI fork with Cap's components; `just clippy` and `cargo test` pass; no `.rs` file under `desktop/` exceeds 1,500 lines | `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` in `desktop/` (1,774 passed, 0 failed, 25 ignored real-CLI and platform tests, 104 test binaries, 650 s); 10 parity runs next to the `studio-engine` suite, all passing ([parity under load](./phase-05-flaky-tests.md#parity-under-load-found-during-phase-6)); a line count of every `.rs` file | Pass. `studio-ui/src/timeline/view.rs` had 1,863 lines; its tests moved to `timeline/view/tests.rs`, leaving 1,476. No other file is over 1,450. |
| 2 | Trim, split or reorder a scene in a studio-template project: preview updates without a Rust rebuild, `timeline.json` is written, Ctrl+Z restores it, an export matches the preview | Hand check in the real app | Pass, owner-reported 2026-10-18 |
| 3 | An agent turn renders markdown, a todo list and a diff with per-file accept and revert, with project tools, over native wires for Claude Code and Codex and ACP for other agents | Hand check in the real app with the installed Claude and Codex CLIs | Pass, owner-reported 2026-10-18 |
| 4 | Durable writes, crash recovery and writer containment proven by tests | `cargo test --workspace`: the crash-boundary matrices and recovery tests are in `studio-engine/tests`; the Job Object containment and escape tests are in `studio-bootstrap` (`src/process/tests.rs`, `src/process/job_escape_tests.rs`) | Pass, by local tests (CI is not a gate for M6) |
| 5 | No bounds-probe, telemetry or qualification harness code remains; release gates on an eligibility list, package verification and a tag-to-CI-run check; no `*_spike` names | Text search for `spike` under `desktop/` and `.github` (0 files). The release gates are in `.github/workflows/desktop-release.yml`; 24 gate/workflow tests pass. | Names and gate tests pass. Owner-selected Windows unsigned-preview dry run in progress; consumer publication stays disabled. |
| 6 | `docs/desktop/architecture.md` describes the real crate layout | Its crate table compared with the `members` of `desktop/Cargo.toml` | Pass (12 of 12) |

## Other Windows hand checks

| Check | Result |
| --- | --- |
| Windows title bar (open from M2) | Pass, owner-reported 2026-10-18 |
| Caption hit-test, title drag, Close hover colour (no automated test since the native harness was removed) | Pass, owner-reported 2026-10-18 |
| Native file dialogs (open project, choose attachment, export destination) | Pass, owner-reported 2026-10-18 |
| Ctrl+Enter in the composer | Pass, owner-reported 2026-10-18 |
| Review-banner layout in the real app | Pass, owner-reported 2026-10-18 |
| Audible playback and mute in the visible session | Pass, owner-reported 2026-10-18 |

## macOS

Not fully tested: CI only, as decided on 2026-10-16. The last recorded green Desktop CI run with the macOS job is [38965115683](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/38965115683) on `feat/studio-rewrite` (2026-10-16), which predates M6. M6 was not run on CI, by the owner's decision. Hand checks and the real-CLI tests (`cargo test -p studio-harness --test claude_real_cli --test codex_real_cli -- --ignored`) were not run.

## Compatibility-word allow-list

The plan's success criterion: `legacy|migrat|OLDEST_|format 2|dev:ino` (case-insensitive) in `desktop/**/src` hits only this list. Searched on 2026-10-17:

| Hit | Why it stays |
| --- | --- |
| `studio-theme` `gray_*_legacy` tokens and their doc comments | Token names, out of M6 scope |
| `answer_oldest_permission` (`studio-ui` `agent/commands.rs`, `shell/keymap.rs`) | Another sense: the oldest open permission row |
| `studio-sdk/src/manifest.rs` test with a `legacy` field | Refusal test: an unknown `legacy` field is rejected |
| `studio-engine/src/diagnostics.rs`, `store.rs` ("no migration") | Refusal wording: an old or newer file is opened read-only and never migrated |
| `studio-engine/src/edit_transaction/mod.rs` `dev:ino` | Crash recovery: the stage file's identity in the journal |
| `studio-engine/src/controller.rs`, `task_scope.rs` "legacy whole-project" task path | Another sense: a brief submitted with no compiled timeline scope, which the UI still sends (`studio-ui/src/agent/controls.rs`). It is a current entry point, not compatibility code. |

The one stale hit, a `studio-harness/src/driver.rs` comment about the removed handwritten ACP session, was corrected.

## Findings

1. **Fixed 2026-10-18: new Claude and Codex profiles use native wires.** New defaults are `ClaudeStreamJson` with `claude` and `CodexAppServer` with `codex`; Pi and Antigravity stay on ACP. Native detection does not need Node or an ACP package. Saved protocol choices remain in effect. The existing saved Claude ACP launch was deliberately rebound to the corrected managed adapter described in finding 5, with its other settings preserved. Provider-profile and handoff tests pass.

2. **Fixed during the hand checks: an older installed SDK blocked setup and builds.** With the v2 SDK from 2026-10-12 in `~\.fframes\sdk\active`, SDK setup failed and **Build preview** reported "missing field `runtime_version` at line 70 column 1". The reuse check parsed the installed manifest and treated a manifest this Studio cannot read as an error. It is now simply not reusable, so setup installs the current SDK and keeps the older one, unchanged, as the previous SDK; the Build error names the file. Test: `an_installed_sdk_from_an_older_release_is_not_reused_and_is_left_as_it_is` in `studio-sdk`. Checked by installing v4 into a copy of the real SDK home: `active` became v4, and `previous` held the v2 manifest byte for byte.

3. **Fixed during the hand checks: one refused preview request stopped the preview.** On `01-template`, after a few splits and trims, every timeline edit and play/pause failed with "io error: The pipe is being closed. (os error 232)". The preview client killed the worker on any error, including a well-formed refusal such as `INVALID_TIMING` or `AUDIO_CACHE_FULL`, so every later request wrote to a closed pipe. A refusal now leaves the worker serving the previous layout. A worker stopped for a real protocol fault reports that fault on every later request. A retime that fails after preparing its audio releases that audio, so the worker's two-mix cache can no longer fill up from a failed retime. A build whose timing document is refused now starts in code order, and the retime after it reports the reason. Reproduced with the real worker in a scratch replay of the session's documents: a rejected document followed by a frame request gave os error 232 before the fix and a frame after it. The committed `real_worker_plays_timeline_edits_without_a_rebuild` now requests that frame and was run after the fix only. Which refusal hit the hand-check session is not confirmed; a full audio cache after a failed retime is the likely one, and the banner now shows the worker's reason if it recurs. Unit test: `a_refused_request_keeps_the_worker_and_a_broken_reply_stops_it_with_its_reason`. The message "a split must fall inside the clip (1..29 frames)" is the expected refusal of a split on a clip edge.


4. **Fixed 2026-10-18: an overlapping scene reordered last no longer extends past the reported video.** The old 02 response placed Overlay at `105..195` but reported only 165 frames. The fix removes an outgoing overlap without a successor and includes the furthest scene end in the video duration. The actual worker now reports `0..60`, `60..120`, `105..180` with 180 total frames. Seek/frame 179, thumbnail and reordered export pass. Both new core regressions fail without the fix.

5. **Fixed 2026-10-18: Claude ACP processes actual advisor results.** The pinned adapter ignored `advisor_tool_result` and consequently reported a finished approved edit incomplete. A hash-checked install correction closes only a call with a real result; missing results still block capture. The conversion regression fails on the original npm artifact and passes after correction. Live denied/approved title edits end authoritatively, change exactly the requested source and clean up their processes. The selected saved ACP profile now uses `0.89.1-advisor-result-v1`, with its original settings backed up.

6. **Fixed 2026-10-18: parity no longer mistakes a fast build acknowledgement for a dropped command.** Two setup failures in the full workspace run already showed `preview.status: building` with `busy: false`. The helper had checked only a transient busy flag. It now also recognizes build/preparation, compiler start and an installed transport epoch, without resending an accepted build. Full parity passes 79/79 three times; each previously failing case passes 30/30.

**Updated automated evidence.** Projects 02–05 pass with the rebuilt SDK. The 01 saved template was rebuilt into the normal Studio cache: six clips, 314 frames, fractional seeks, frame 313 and thumbnail all pass. Current-format projects 01, 02 and 05 explicitly adopted the new SDK checksum after their manifests were backed up; their timelines were preserved. At the end of automated qualification, original 03 still awaited Import Rust and original 04 retained its prepared SHA-256. Desktop validation comprises 1,703 other workspace tests plus the fixed 79-test parity suite, with the repeated runs above; doc tests, formatting, clippy and local packaging checks pass. See [the final fixes report](./final-fixes-2026-10-18.md) for exact commands, artifacts, initial failures and installed state. The subsequent manual results above come from the owner's confirmation.

## Unresolved

- The owner selected a Windows unsigned preview on 2026-10-18. Its non-publishing [release dry run](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/39110213274) is in progress on `5a3722e` (`preview_release=true`, `dry_run=true`, `windows_only=true`, `consumer_release=false`).
- Consumer publication is outside that selected path. A local gate attempt on `8cd1a5a`, including a disposable policy with `consumer_release_enabled=true`, refused because trusted signing keys are unconfigured. The actual policy is unchanged, there are no configured repository signing secrets, and no qualifying consumer CI runs exist for that commit. Raw evidence: `output/m6-release-closure/release-preflight.json`.
