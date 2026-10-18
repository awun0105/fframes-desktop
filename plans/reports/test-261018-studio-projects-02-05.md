# Studio projects 02–05: automated qualification, 2026-10-18

**Resolved later on 2026-10-18:** both findings are fixed and the rebuilt-SDK replays pass. See [final fixes and validation](../261013-0733-studio-m6-cleanup/reports/final-fixes-2026-10-18.md). The original failing qualification is retained below.

The original run found two failures: an overlapping scene reported beyond the video after a reorder in **02**, and an incomplete approved edit from the pinned live Claude ACP adapter in **05**. No production fix was made during this test run.

The tests used `C:\Users\Administrator\fframes-m6` at `84ba69c` with the preview scale/geometry correction from the preceding task, and the installed `studio-sdk-x86_64-pc-windows-msvc-v4`. Project mutations happened in temporary copies. Exports, sampled PNGs and JSON evidence are under `output/hand-checks-02-05/` in this checkout. The original projects were not imported, edited or reset.

| Project | Result | Coverage and evidence |
| --- | --- | --- |
| 02-scenes-and-audio | **Fail: reorder** | Inspected every one of 165 frames. Checked scene ranges `(0,60)`, `(45,135)`, `(105,165)`, boundary seeks, fractional scales, thumbnails, sequential frame requests, stereo PCM, silence and both cue positions. Baseline and split/trim MP4 exports fully decode and match their preview first frames within the lossy comparison threshold. Export cancellation cleans up its output. Split, trim, undo/redo, rejected-document recovery and Undo after the failed reorder pass. [Result](../../output/hand-checks-02-05/run6/02-scenes-and-audio/result.json), [raw reorder response](../../output/hand-checks-02-05/run6/02-scenes-and-audio/reorder-raw-timeline.json), [baseline MP4](../../output/hand-checks-02-05/run6/02-scenes-and-audio/baseline.mp4), [edited MP4](../../output/hand-checks-02-05/run6/02-scenes-and-audio/edited.mp4). |
| 03-import-me | **Pass** | Import adds only `studio.json` in the copy; all existing bytes stay identical. Build, inspection of all 300 frames, boundary seeks, fractional scales, thumbnails, sequential frame requests, silent PCM, export/preview comparison, full MP4 decode and cancellation pass. [Result](../../output/hand-checks-02-05/run3/03-import-me/result.json), [MP4](../../output/hand-checks-02-05/run3/03-import-me/baseline.mp4). |
| 04-older-studio | **Pass: expected refusal** | The older project is refused and every file remains identical. Its original `studio.json` SHA-256 still matches the prepared checksum: `CBCDE65FDDC5329049CCDB2863E75816D3E2C99CC900229919B379A53A84CD06`. [Result](../../output/hand-checks-02-05/run3/04-older-studio/result.json). |
| 05-agent-playground | **Partial: live ACP failure** | The actual project's build, inspection of all 300 frames, seeks, fractional scales, thumbnails, sequential requests, silent PCM, export/preview comparison, full MP4 decode and cancellation pass. Native provider and workflow checks pass as detailed below. The live ACP approved-edit result fails the clean-turn requirement. [Preview/export result](../../output/hand-checks-02-05/run3/05-agent-playground/result.json), [MP4](../../output/hand-checks-02-05/run3/05-agent-playground/baseline.mp4), [ACP result](../../output/hand-checks-02-05/run7/05-agent-playground/acp-result.json). |

## Finding: reorder puts Overlay past the video end

Reproduction on a copy of the untouched **02** project:

1. Build the preview and learn its three clips.
2. Move clip index `2` to index `0`.
3. Apply the resulting timing document to the same worker.

The worker acknowledges the document, but reports this layout:

| Clip key | Reported frame range |
| --- | --- |
| `studio_video::Repeated#2` | `0..60` |
| `studio_video::Repeated` | `60..120` |
| `studio_video::Overlay` | `105..195` |

The same response says `total_frames: 165`. The client rejects it with `Invalid compiled scene identity/range`. This is separate from the earlier fractional-scale error: the payload itself contains an out-of-range scene. The raw response and exact timing JSON are saved in the linked evidence. Undo restores a valid, seekable preview.

The other timing operations were tested independently before this reorder: splitting the first clip at 20 frames, trimming the second clip's end by five frames, refusing a split at zero without writing, undoing and redoing both edits, recovering from a deliberately rejected timing document, and exporting the edited layout.

The stereo audio check accounts for unequal source channels. The source left/right RMS ratio is `2.000324`; the first, left-panned cue produces `2.828886`, and the second, right-panned cue produces `1.414443`. These match the framework's stereo balance attenuation. The initial test incorrectly required the right-panned cue's right channel to be louder absolutely; that assertion was corrected, without changing project or mixer code.

## Finding: approved live Claude ACP edit ends incomplete

The live check installed the repository-pinned `@agentclientprotocol/claude-agent-acp` **0.89.1**, with its locked Claude Agent SDK **0.3.293**, in an isolated test folder. Studio's saved adapter configuration was not changed.

The adapter authenticated and opened a session on a temporary copy of **05**. The prompt requested only a title replacement in `src/lib.rs`, with no shell commands or other file edits. The test first declined permission, verified the source unchanged, then submitted the edit again and approved it.

On the final recorded run:

- The declined turn ended normally with one permission request and unchanged source.
- The approved turn also requested permission once and streamed a reply claiming completion.
- Its result was `stop_reason: EndTurn`, `authoritative: false`, `reported_incomplete: true`, with no unresolved permissions and no driver-wide failure.
- `eligible_for_quiescence()` was false. The edit therefore cannot be treated as a clean candidate for capture/Apply. The test did not accept or publish it.

An earlier live trial also returned an incomplete result on the declined turn. The finding above is based on the subsequent **approved** turn, not on requiring a refused edit to be eligible for Apply. [Declined-turn evidence](../../output/hand-checks-02-05/run7/05-agent-playground/acp-denied-turn.json), [approved-turn evidence](../../output/hand-checks-02-05/run7/05-agent-playground/acp-allowed-turn.json).

## Supporting suites

| Suite | Result |
| --- | --- |
| `studio-project`: import, version refusal, assets, source integrity, publication and source index | **49 passed** |
| `studio-harness`: ACP and native-driver protocol, discovery, permissions, lifecycle, redaction and cross-driver behavior | **185 passed**; live-provider tests were also run separately |
| Actual signed-in Claude native CLI | **4 passed**: permission denial/approval, interrupt, steering, resume, background work and questions |
| Actual signed-in Codex native CLI | **2 passed**: permission denial/approval, interrupt, steering, resume, background-command tracking and cleanup |
| Agent workflow, project tools, apply gate, review/per-file revert, UI model and writer containment | **135 passed**; two SDK-dependent tests run separately |
| Real-SDK agent edit/Apply/preview handoff/Undo and selected-title edit/reopen | **2 passed**; these use a scripted development agent, not a live provider |
| GPUI parity, including timeline, transport, agent/composer/review and setup controls | **79/79 passed** |

These automated suites do not establish that native Windows pickers, caption gestures or audible speaker output work in the owner's RDP session. Those hand checks remain pending.

## Reproduction

The exact qualification source is retained at [hand_check_projects.rs](../../output/hand-checks-02-05/hand_check_projects.rs). The runner stages it as an ignored integration test in the M6 checkout and uses fresh temporary source/history folders. Use a fresh run number so existing export artifacts are not overwritten:

```powershell
& .\output\hand-checks-02-05\run_fixture_checks.ps1 -RunName run8 -Filter project_02
& .\output\hand-checks-02-05\run_fixture_checks.ps1 -RunName run9 -Filter project_05_claude_acp_live
```

The ACP reproduction uses the isolated npm installation already prepared by this run and makes live model calls. The original `03-import-me` still has no `studio.json`; the import was exercised only in its test copy.
