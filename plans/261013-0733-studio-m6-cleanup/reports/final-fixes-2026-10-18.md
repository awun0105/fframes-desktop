# M6 final fixes, 2026-10-18

The overlapping-scene reorder, incomplete Claude ACP turn, native-provider defaults and the newly observed parity race are fixed. The release app, SDK and saved template preview have been rebuilt. The owner subsequently confirmed all Windows hand checks passed on 2026-10-18 ("test all okay by my hand"); [platform checks](./platform-checks.md) record this acceptance. Phase 6 remains open only for the owner release dry run. The automated evidence and installed-state snapshot below predate that manual acceptance.

## Changes

- **Fractional preview geometry.** JSON can shift a frame scale by a few floating-point units and cross an even-pixel rounding boundary. The client accepts only the dimensions produced within four ULPs of the requested scale, then restores that scale for presentation identity checks. Frame index, seek serial, payload size, stride and allocation limits remain checked. The viewport `19 / 1920` (18×10 versus 20×10) and fractional-scale sweeps pass; incorrect scales and geometry still stop the worker. Committed in `8cd1a5a`.
- **Overlapping scene reorder.** A last scene no longer receives an outgoing overlap. The video duration also includes the furthest scene end when an unusually long overlap reaches beyond subsequent clips. Both new [core regressions](../../../fframes/src/video_timing_tests.rs) fail without the fix. The actual 02 fixture now reorders to ranges `0..60`, `60..120`, `105..180`, with `total_frames: 180`; frame 179, its thumbnail and the reordered MP4 all pass.
- **Claude ACP completion.** The pinned 0.89.1 adapter ignored an actual `advisor_tool_result`, leaving its server call unfinished. The [installer correction](../../../desktop/crates/studio-harness/src/adapter_install/claude_patch.rs) closes that exact call, including an explicit failed status for advisor errors. It checks the original and corrected SHA-256 and uses a separate `advisor-result-v1` install directory. Missing results still block capture. The JavaScript regression fails against the original npm module and passes against the corrected one; the [live test](../../../desktop/crates/studio-harness/tests/claude_acp_real_cli.rs) verifies permission denial, approval, the exact title edit, authoritative completion and process cleanup.
- **Provider defaults.** New Claude and Codex profiles use their native protocols and detect their CLIs without Node or ACP. Saved protocol choices remain in effect. The existing saved Claude ACP profile was backed up and pointed at the corrected managed adapter; its other settings are preserved. The scripted handoff tests explicitly select the ACP protocol their fake peer implements.
- **Parity acknowledgement.** A full workspace run passed 1,703 other tests but failed two parity tests in their shared setup. The captured telemetry already said `preview.status: building` while `busy` was false. A fast engine response had cleared the command's transient busy flag before the helper read it. The helper now also recognizes active build/preparation, compiler start or an installed transport epoch; it never resubmits an accepted build. All 79 parity tests then passed three times, and each previously failing case passed 30 consecutive runs.

## Verification

**Later release decision:** the owner selected a Windows unsigned preview. The consumer preflight's refusal and the non-publishing preview workflow run are tracked in [platform checks](./platform-checks.md); this supersedes the earlier requirement that the owner run a consumer dry run personally.

| Check | Result |
| --- | --- |
| Desktop test sets, pinned Rust 1.98.1 | 1,703 non-parity tests passed in the workspace run; parity passed 79/79 in three runs after the setup fix. Workspace doc tests pass. This is combined validation, not a claim that the original workspace invocation was green. |
| Previously failing canvas/export parity cases | 30/30 each after the fix |
| Core library | 77 passed; core clippy passes on Rust 1.98.1 |
| Desktop clippy, workspace/all targets, warnings denied | Pass on Rust 1.98.1 |
| Rust formatting and Git whitespace checks | Pass in both worktrees |
| Packaging script tests | 22 run, 3 platform skips, no failures |
| Rebuilt SDK | Two offline worker builds, frame PNGs and protocol checks pass; isolated and normal-home transactional installs pass |
| Actual 01 saved template | Six clips, 314 frames; fractional-scale seeks, frame 313 and thumbnail pass. Built into the normal Studio cache; timeline preserved. |
| 02 scenes/audio | Baseline audio/rendering, split/trim, undo/redo, rejected-document recovery, baseline/edited/reordered exports and cancellation pass |
| 03 import | Additive import in a copy, all 300 frames, preview/export comparison and cancellation pass |
| 04 older Studio | Expected refusal; original SHA-256 remains `CBCDE65FDDC5329049CCDB2863E75816D3E2C99CC900229919B379A53A84CD06` |
| 05 playground | All 300 frames, preview/export/cancellation pass; actual signed-in ACP denied/approved edit passes |
| Main-checkout integration | Existing edits restored after fast-forward to `84ba69c`; the 14 publication tests pass with the owner's boundary helper. The saved real-worker tests compile against the current manifest API. |
| M4 assembled-SDK export parity (`8fd8425`) | H.264 at frames 0, 135, 269: preview and pre-encode CPU RGBA are byte-identical; decoded PNGs pass `odiff -t 0.4`; a magenta negative-control image is rejected. Test passes in 75.19 s; clippy and formatting pass. Initial attempts exposed a test-fixture error conversion, a broken npm executable shim, and one MPEG-4 pixel outside the strict threshold; the passing case is explicitly H.264. |
| Windows unsigned package | Production packager passes, including static CRT/PE-import checks and bundled SDK verification. The 502,676,251-byte ZIP passes full inventory hashing; its bundled CLI launches successfully. The installer/release-files dry run is tracked separately in platform checks. |

The initial shell used Rust 1.99 because `--manifest-path` alone does not select a different directory's toolchain. Desktop validation above was rerun explicitly with the repository's pinned 1.98.1. SDK workers were assembled with 1.98.1 throughout. Native dialogs, caption gestures, the real Ctrl+Enter chord, review layout and speaker output were subsequently accepted through the owner's hand checks.

## Installed state and evidence

The corrected SDK is active in `C:\Users\Administrator\.fframes\sdk\active`, with compatibility digest `d0801240e35bb6d7f644612cc7fb88d97757a4b1ebe0173e09bc77abc9ef2c56`. The previous SDK is retained by the installer. Its new checksum was deliberately adopted in the current-format manifests of projects 01, 02 and 05 after qualification; their original manifests are backed up and their timelines are preserved. Project 03 still has no `studio.json` in the original folder, and project 04 was not upgraded.

The launcher `C:\Users\Administrator\studio-hand-checks\start-studio.ps1` now uses the rebuilt release app and `desktop\target\m6-fixed-sdk` bundle in the M6 worktree. The corrected adapter is installed under the normal Studio `adapters` directory. Original provider settings and the launcher are backed up alongside the project manifests.

- [02 result](C:/Users/Administrator/fframes-desktop/output/hand-checks-02-05/run10/02-scenes-and-audio/result.json), [corrected raw reorder](C:/Users/Administrator/fframes-desktop/output/hand-checks-02-05/run10/02-scenes-and-audio/reorder-raw-timeline.json), [reordered MP4](C:/Users/Administrator/fframes-desktop/output/hand-checks-02-05/run10/02-scenes-and-audio/reordered.mp4)
- [03 result](C:/Users/Administrator/fframes-desktop/output/hand-checks-02-05/run9/03-import-me/result.json), [04 refusal](C:/Users/Administrator/fframes-desktop/output/hand-checks-02-05/run9/04-older-studio/result.json), [05 result](C:/Users/Administrator/fframes-desktop/output/hand-checks-02-05/run10/05-agent-playground/result.json)
- [Live ACP outcome](C:/Users/Administrator/fframes-desktop/output/hand-checks-02-05/run8/05-agent-playground/acp-result.json), [installed adapter/settings result](C:/Users/Administrator/fframes-desktop/output/m6-completion/managed-claude-install.json)
- [Saved template preview](C:/Users/Administrator/fframes-desktop/output/m6-completion/template-preview/result.json), [last-frame PNG](C:/Users/Administrator/fframes-desktop/output/m6-completion/template-preview/frame-0313.png), [SDK adoption and backups](C:/Users/Administrator/fframes-desktop/output/m6-completion/sdk-adoption.json)
- [Parity repeat results](C:/Users/Administrator/fframes-desktop/output/m6-completion/parity-rechecks/results.json), [workspace output including the original parity failures](C:/Users/Administrator/fframes-desktop/output/m6-completion/workspace.log)
- [M4 pixel parity result](C:/Users/Administrator/fframes-desktop/output/m6-release-closure/export-parity-h264/result.json), [package verification](C:/Users/Administrator/fframes-desktop/output/m6-release-closure/windows-package-verification.json), [Windows ZIP](C:/Users/Administrator/fframes-desktop/output/m6-release-closure/fframes-studio-x86_64-pc-windows-msvc.zip)

The exact disposable fixture runner is retained under `output/hand-checks-02-05`. Its temporary integration test and settings helper were removed from the M6 source tree after use. Source fixes and the durable regression tests remain reviewable in the working tree.
