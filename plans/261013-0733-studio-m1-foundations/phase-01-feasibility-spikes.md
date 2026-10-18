---
phase: 1
title: "Feasibility spikes"
status: complete
priority: P1
effort: "6-8d"
dependencies: []
---

# Phase 1: Feasibility spikes

## Goal

Prove or disprove the five assumptions the roadmap rests on, each with a pass/fail exit, before any production code changes.

## Context links

- Roadmap: [plan](../261013-0733-studio-rewrite-roadmap/plan.md)
- Cap GPUI pins: `~/code/Cap/apps/desktop-gpui/Cargo.toml:29-39` (`wingleeio/zed` `5d1f83d`); patches `apps/desktop-gpui/patches/`; `scripts/prepare-gpui-dependency.sh`
- Scene API: `fframes/src/scenes.rs:47-111`; `Frame::clone_with_scene_offset` (`fframes/src/frame.rs:76`, already public); scene lookup by pointer: `fframes/src/fframes_context.rs:110-142`
- Preview worker: `fframes-studio-runtime/src/preview_worker.rs:280,394`; protocol `fframes-studio-protocol/src/preview.rs` (Hello capabilities around `:40-44`, `PreviewRequest` around `:580`)
- Studio template: `desktop/crates/studio-project/templates/` (`src/lib.rs` has no scenes and `Duration::Seconds(5.0)`; `Cargo.toml` pins `fframes =1.2.0`, runtime and protocol `=0.1.0`)
- Linux publish primitives: `desktop/crates/studio-project/src/publish.rs`; tests `desktop/crates/studio-project/tests/publish.rs`
- CI: macOS appears only in `.github/workflows/desktop-release.yml:57` (`macos-15`)

## Key insights

- Live timing reload means timing is applied by the previewer and renderer on top of code scenes, not inside `Video::define_scenes`.
- Every studio project is created from a template with no scenes, so the spike must create scenes there, not only in `examples/fframes-studio-intro`.
- This VM has no GPU; the fork's DirectX renderer must run on the software adapter or manual checks move to the user's machine.

## Spike S1: Cap's GPUI fork on Windows and macOS

1. Create the mirror `khanhthanhdev/zed-cap`: fork `wingleeio/zed` at `5d1f83d9f27a19bec1fb241dc33b42238af9cf8d` with Cap's three patches (`apps/desktop-gpui/patches/zed-{gpui,windows,linux}.patch`) applied as one commit per patch on a branch (what Cap's `scripts/prepare-gpui-dependency.sh` does into `../zed-cap`), and pin that commit. Review the three patches (4,004 lines: transparent-window alpha, `paint_surface_fitted`, custom Windows frame insets, `start_window_move`, taskbar-aware fitting, DirectX atlas and shader fixes, Linux fixes) and the fork's diff against upstream Zed; note anything unsafe in the report.
2. Create `desktop/spikes/cap-gpui-hello/` as its own Cargo workspace pinned to the mirror, with Cap's `gpui_platform` features (`font-kit`, `runtime_shaders`; `wayland` and `x11` on Linux; Cap `Cargo.toml:29-39`). Run `cargo tree` there and list every git source the fork pulls besides itself (font and platform crates); each is mirrored or allow-listed in `deny.toml` in phase 2.
3. Open a window with a rounded card and text on this VM over RDP (DirectX on the software adapter).
4. Add one `#[gpui::test]` with `TestAppContext` that builds a view and dispatches an action; run it here and on Linux and macOS CI. Cap never builds gpui's `test-support` feature, and its patches change `crates/gpui/src/platform.rs`, `app.rs` and `window.rs` without touching the test platform (checked 2026-10-13), so new platform-trait methods may leave `TestAppContext` uncompiled. Fallback: add a mirror commit that implements them for the test platform.
5. Copy Cap's `src/theme.rs` and `src/ui/button.rs` into the spike and compile them with only import changes, which confirms Cap components port without GPUI API work. Then copy `desktop/app/src/ui/button.rs` with its token dependencies and record every API change fframes code needs between upstream Zed `1a84d5d` and the fork (this sizes phase 2).
6. Add a job `gpui-spike-macos` on `macos-15` in `.github/workflows/desktop.yml`, triggered by `pull_request` (same path filter as the workflow) and `workflow_dispatch`, that builds and tests the spike.
7. Confirm on this VM that Cap's Windows caption (`src/ui/windows_caption.rs`, using the patched custom-frame insets and `start_window_move`) draws and drags the window, and that the APIs later phases rely on (`.cached(StyleRefinement)`, `window.drop_image`, `observe_window_appearance`, `EntityInputHandler`, `canvas` prepaint bounds, `with_animation`) exist; Cap's code uses all of them ([Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md)).

**Exit:** window opens here (or, if the fork cannot render without a GPU, on the user's machine, which then takes over manual checks and merge gate 7), the gpui test passes here and on Linux and macOS CI on the patched fork (with the test-platform commit if needed), the patched mirror exists, and `reports/spike-cap-gpui-api-delta.md` lists the API changes and the patch review. **On failure:** stop and ask (fallback: stay on upstream Zed `1a84d5d` and adapt Cap's code to it).

## Spike S2: timing hook

1. Data shape: `TimingDoc { version, clips: Vec<TimingClip> }`, `TimingClip { key, trim_start_frames, trim_end_frames }`; play order is `clips` order; a split is two clips with the same `key`. Strict serde (`deny_unknown_fields`, known `version` only), bounds (clip count, total frames, `trim_start + trim_end` below the scene length), checked arithmetic, and whole-document rejection on any error.
2. Key rule: `editor_instance_key()` if present, else `name()`, plus `#n` for repeated keys in code order. Editor object keys for clips are derived as `key#clip-index`, so canvas selection stays unique after a split. Orphan clips are reported and skipped; code scenes missing from the doc are appended.
3. Wrapper scene delegating to a borrowed `&dyn Scene` using `Frame::clone_with_scene_offset`. Define how `get_scene_info` answers for a wrapped or split scene (proposed: the range of the first clip of that scene) and test it.
4. Apply the doc in `Previewer` and the render path, opt-in (default none); existing examples and the WASM bridge compile unchanged.
5. Add `SetTiming` as a capability offered in the Hello handshake (no contract version bump); the worker applies it and the next `Timeline` response reflects it without restarting. Old workers keep previewing without the capability.
6. Check `Overlap` and scene audio under trim, split and reorder; write down the rule (proposed: audio follows its clip and is trimmed with it).
7. Run all of the above on a project created from the studio template after giving it three scenes with `Duration::Auto`, and on `examples/fframes-studio-intro`.

**Exit:** on the template-based project, reorder, trim and split change preview frames through `SetTiming` with no rebuild, and a render given the same doc matches preview frames at three times. Results in `reports/spike-timing-hook.md`. **On failure:** stop and ask (fallback: reorder and trim only).

## Spike S3: no-clobber publish on Windows and macOS

1. List the guarantees `publish.rs` gives on Linux: operations relative to a held directory handle, no link following, atomic no-replace rename, root pathname binding, detection of a replaced source.
2. Windows: handle-relative opens with `NtCreateFile` (`OBJECT_ATTRIBUTES.RootDirectory`, `FILE_OPEN_REPARSE_POINT`) and handle-relative no-replace rename with `NtSetInformationFile(FileRenameInformationEx)` with `RootDirectory` set and no replace flag. No path-based `MoveFileExW` for live names. Read the 128-bit `FILE_ID_128` and volume serial.
3. macOS: `openat(O_NOFOLLOW)` and `renameatx_np(RENAME_EXCL)`.
4. Port to each prototype the no-replace, editor-save race, link, swapped-directory and root-binding tests from `tests/publish.rs`. Run Windows here and macOS on the S1 job.
5. Decide how cloud placeholder reparse points (OneDrive) are treated (proposed: refuse mutation with a clear reason).

**Exit:** both prototypes pass all five test kinds; `reports/spike-publish-primitives.md` written, including a re-estimate of M3 phase 1 from the Windows prototype. **On failure:** stop and ask.

## Spike S4: port closure for Zeron and Cap UI code

1. For all of Cap's `src/ui/` and `src/theme.rs` with the assets they use (`assets.rs` icons and fonts; `ui/` reaches `crate::feeds` and `crate::editor_sidebar` once each), and for `composer.rs`, `transcript.rs`, `changes.rs`, `todo_panel.rs` (Zeron), `editor_timeline.rs`, `editor_edits.rs`, the `editor_window.rs` frame path (`make_frame_callback` `:11779`, `frame_image` `:563`, `PreviewFrameView` `:608`) and `src/ui/{select,menu,tooltip,kbd,slider}.rs` (Cap), list every `crate::*` module and external crate each depends on, transitively, and mark each as port, replace with a fframes type, or drop (frost, dictation, Zeron settings, voice).
2. Compile `todo_panel.rs` and the composer input (`ComposerInput`) against fframes stand-in types on Cap's fork to measure real effort, including the zui-to-fork GPUI changes. Also list, for Cap's `editor_window.rs` (layout parts), `editor_panels.rs`, `editor_sidebar.rs`, `settings_window.rs` and `settings_pages.rs`, which Cap types (`cap-project`, `cap-editor`, recording state) each depends on and the `studio-proto` snapshot that replaces it.
3. Re-estimate M2 phases 2, 4 and 6, M4 phases 4 to 7 and every M5 phase from the closure.

**Exit:** `reports/spike-port-closure.md` with the closure tables and updated estimates written into M2 and M5.

## Spike S5: native Claude and Codex wires on real CLIs

M5a replaces ACP with native protocols for Claude Code and Codex ([Zeron report](../reports/research-261013-1749-zeron-agent-control.md), section 3). Prove both on real CLIs before that milestone is built.

1. Claude Code (2.1.296 on this VM): from a small probe, launch `claude --print --input-format stream-json --output-format stream-json --verbose --include-partial-messages --permission-prompt-tool stdio --permission-mode default` in a scratch project; send the `initialize` control request and a prompt that edits a file. Confirm a `can_use_tool` request arrives, `deny` blocks the edit and `allow` lets it through; `interrupt` stops a running turn and the process stays up; `--resume` continues the session.
2. Ask for work that starts a background command and finishes its reply first. Record when the `result` frame arrives and when the process tree is quiet.
3. Codex (not installed here; on the user's machine or wherever it is installed and signed in): `codex app-server`, `initialize`, `thread/start`, `turn/start` with `approvalPolicy: "on-request"`; confirm approval requests arrive, decline blocks the command, `turn/interrupt` ends the turn with `turn/aborted`. Same background-command check.
4. Record which requests each CLI actually sends under this machine's settings: for Claude, which tools produce `can_use_tool` with the user's settings loaded and with `--setting-sources` limited (and `--strict-mcp-config`, `--bare`), and which hooks, rules and MCP servers load in each case; for Codex, which approval policy (`on-request` or `untrusted`) asks before every command.
5. Record each CLI's version and a short frame transcript for the M5a fake CLIs.

**Exit:** `reports/spike-native-wires.md` with versions, transcripts and both checks. **On failure** (for example `--permission-prompt-tool stdio` missing): stop and ask; the fallback is ACP for that agent.

## Files

- Create: `desktop/spikes/cap-gpui-hello/**` (deleted in phase 2); `plans/261013-0733-studio-m1-foundations/reports/spike-*.md`
- Modify on a throwaway spike branch (rewritten as production code in M3 and M4): `fframes/src/scenes.rs`, `fframes-studio-protocol/src/preview.rs`, `fframes-studio-runtime/src/preview_worker.rs`, the template copy used for S2
- Modify and keep: `.github/workflows/desktop.yml` (macOS job)

## Todo

- [x] S1: mirrors and fork review
- [x] S1: window and gpui test on this VM
- [x] S1: button port and API-change list
- [x] S1: macOS CI job green
- [x] S2: strict, bounded data shape and key rule
- [x] S2: wrapper scene, derived keys, `get_scene_info` rule
- [x] S2: `SetTiming` as a Hello capability
- [x] S2: overlap and audio rule
- [x] S2: template-based project and render parity
- [x] S3: Windows NT handle-relative prototype and five test kinds
- [x] S3: macOS prototype and five test kinds
- [x] S3: cloud placeholder decision
- [x] S4: closure tables and re-estimates
- [x] S5: native Claude and Codex wires on real CLIs
- [x] Five spike reports written; user told of any failure

## Results (2026-10-14)

All five spikes pass; no fallback was needed.

- S1: [spike-cap-gpui-api-delta](./reports/spike-cap-gpui-api-delta.md).
- S2: [spike-timing-hook](./reports/spike-timing-hook.md); code on the throwaway branch `spike/m1-timing`. Timing lives on the resolved timeline entries rather than in wrapper scenes, which keeps `get_scene_info`'s pointer lookup working.
- S3: [spike-publish-primitives](./reports/spike-publish-primitives.md); prototypes on `spike/m1-publish`, green on macOS and Windows CI. Cloud placeholders and SMB shares refuse mutation.
- S4: [spike-port-closure](./reports/spike-port-closure.md); M2, M4 and M5 estimates updated, and the roadmap total is now 110-157d.
- S5: [spike-native-wires](./reports/spike-native-wires.md) (Claude Code 2.1.296, Codex 0.162.1). Both wires work. M5a phases 2-4 must absorb the deviations listed in its Result section: neither CLI's turn-end frame means the work is done, Codex has no `turn/aborted`, only `untrusted` asks before every command, and session stop needs a containment kill.

## Success criteria

Each spike exits with a pass and a written report, or the user has picked its fallback.

## Risks

- The fork does not render on a GPU-less VM: manual checks move to the user's machine; CI stays the gate.
- Some Windows filesystems (FAT, network shares) lack handle-relative no-replace rename: refuse mutation there, as Linux does when `renameat2` is missing.

## Next

Phase 2 uses S1. M3 builds on S3. M4 phases 1 and 2 turn S2 into production code. M2 and M5 use S4's estimates.
