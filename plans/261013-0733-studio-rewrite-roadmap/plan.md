---
title: "fframes Studio rewrite on Zeron and Cap foundations (roadmap)"
description: "Rebuild the desktop app on Cap's GPUI fork with Cap's components, editor layout and settings screens, Zeron-style crate layering, Cap-style timing edits and playback, and an agent panel with Zeron's internals in Cap chrome, in six mergeable milestones."
status: pending
priority: P1
effort: "110-156 engineer-days (sum of milestones)"
branch: feat/studio-rewrite
tags: [desktop, refactor, gpui, feature, critical]
blockedBy: [261013-1010-faster-desktop-ci]
blocks: []
created: 2026-10-13
---

# fframes Studio rewrite (roadmap)

## Overview

This roadmap indexes six milestone plans. Each milestone ends with the app building, the parity tests green and the branch mergeable to `main`. The confirmed contract is repeated below so executors do not need the planning conversation.

## Milestones

| # | Milestone | Blocked by | Effort | Status |
|---|-----------|-----------|--------|--------|
| M1 | [Foundations: spikes, Cap GPUI switch, parity tests](../261013-0733-studio-m1-foundations/plan.md) | CI branch merged | 12-17d | Pending |
| M2 | [Architecture: crates, engine events, Cap kit and layout, harness removal, splits](../261013-0733-studio-m2-architecture/plan.md) | M1 | 26-35d | Pending |
| M3 | [Durable writes, containment and tools on Windows and macOS](../261013-0733-studio-m3-durable-writes/plan.md) | M1 phase 1; phases 3-5 also need M2 phases 1, 3, 8 | 18-27d | Pending |
| M4 | [Editing: timing doc, SDK release, timeline, playback, export](../261013-0733-studio-m4-editing/plan.md) | M2, M3 | 24-34d | Pending |
| M5a | [Native agent wires: Claude stream-json, Codex app-server](../261013-0733-studio-m5a-native-wires/plan.md) | M2, M3 | 10-14d | Pending |
| M5 | [Agent UX: markdown, composer, todo, diffs, history](../261013-0733-studio-m5-agent-ux/plan.md) | M2, M3, M5a | 17-25d (re-estimate after spike S4) | Pending |
| M6 | [Cleanup: spike names, docs, platform checks](../261013-0733-studio-m6-cleanup/plan.md) | M4, M5 | 3-4d | Pending |

M3 phases 1 and 2 run beside M2 from the start; phase 5 starts after M2 phase 1, phase 4 after M2 phases 1 and 8, and phase 3 after M3 phases 1, 2 and 4 and M2 phase 3 (so it also waits for M2 phase 8). Writer qualification by ledger stays usable until M3 phase 4 replaces it, so there is no window in which a real agent cannot apply on Linux. One executor (confirmed 2026-10-13): run M1, M2, M3, M4, M5a, M5, M6 in that order, doing M3 phases 1, 2 and 5 between M2 phases where their dependencies allow. The overlaps described above say what may interleave, not that work runs at once.

## Phases

| # | Phase | Status |
|---|-------|--------|
| 1 | [Milestone merge gates](./phase-01-milestone-merge-gates.md) | Pending |

## Confirmed contract

- **GPUI base:** Cap's fork: `wingleeio/zed` at `5d1f83d9f27a19bec1fb241dc33b42238af9cf8d` with Cap's three patches (`apps/desktop-gpui/patches/zed-{gpui,windows,linux}.patch`) applied, consumed through the mirror `khanhthanhdev/zed-cap` (one commit per patch on a pinned branch). zui and `gpui-base` are not used.
- **Crates:** `studio-proto`, `studio-theme`, `studio-markdown`, `studio-engine`, `studio-harness` (renamed `studio-agent-spike`), `studio-ui`, and a thin `app` binary. The engine stays in-process. `EngineHandle` is new code modelled on Zeron's, not a port of Zeron's RPC-based engine.
- **Look and UI:** Cap first. Cap's theme model (`src/theme.rs`) with Cap's System, Light or Dark setting (`ThemePreference`, default System) with the tokens from [the Cap spec](../reports/xia-261011-0016-cap-design-system-spec.md), Cap's component library (`src/ui/`), Cap's editor window layout, panels and sidebar, Cap's settings pages for setup and providers, and Cap's Windows caption. No glass, frost or material tokens.
- **Timing:** `timeline.json` in the project holds clip order, trims and splits; code defines scene content. It has its own timing revision, outside the build key, so edits never trigger a Rust rebuild. Preview, render and export read it; export binds to the pair (source revision, timing revision).
- **SDK:** the timing hook and the extended export contract ship in a new bundled SDK (the vendored crate set the app ships; no crates.io publish); existing projects get a durable, undoable migration and keep building on the old vendored versions until migrated.
- **Agent wires:** native drivers for Claude Code (stream-json control protocol) and Codex (`app-server` JSON-RPC), ported from Zeron's harness; ACP for every other agent; all behind one driver trait, and all started in the task's draft directory like ACP today. No driver answers a permission or approval request on the user's behalf; every request becomes a row the user answers. What a CLI runs without asking is its own policy: Claude Code in default mode runs read-only tools without a request, and Codex with `on-request` asks only for actions outside its sandbox. Writes are always contained, captured from the draft and validated before apply.
- **Agent panel:** a tab in the editor sidebar next to Presets and Scenes; chrome (frame, tabs, controls, menus, inputs) from Cap's kit; Zeron supplies the internals: streaming markdown with code highlighting, todo panel, composer (slash commands, attachments, steer/stop), diff review where rejecting files forms a new candidate that must pass validation before apply, per-file revert after apply (also validated), and multi-session history.
- **Safety kept:** writer containment is verified at runtime by the engine (Unix process groups, Windows Job Objects), not by a qualification ledger. The agent tool broker runs on Windows over named pipes.
- **Tests:** `#[gpui::test]` replaces the bounds-probe harness. Parity tests land in M1 and stay green; the harness is deleted in M2.
- **Platforms:** Windows and macOS ship and are checked by hand; Linux compiles and passes CI.

## Done means

- `desktop/` builds on Cap's GPUI fork and draws with Cap's components; `just clippy` and `cargo test` pass; no `.rs` file under `desktop/` exceeds 1,500 lines.
- Trim, split or reorder a scene in a project made from the studio template: preview updates without a Rust rebuild, `timeline.json` is written, Ctrl+Z restores it, and an export matches the preview.
- An agent turn renders markdown, a todo list and a diff with per-file accept (validated) and revert, on Windows and macOS, with project tools available on both, over native wires for Claude Code and Codex and ACP for other agents (Codex is checked on Windows where it is installed and signed in).
- Durable writes, crash recovery and writer containment run on Windows and macOS, proven by tests in CI.
- No bounds-probe, telemetry or qualification harness code remains; the release workflow gates on an explicit eligibility list, package verification and a tag-to-CI-run check instead; no `*_spike` names remain.
- `docs/desktop/architecture.md` describes the real crate layout.

## Non-goals

fframes framework changes beyond the opt-in timing hook and the export options; timing undo that survives an app restart (history is per session, like Cap); an engine daemon or RPC; imported media clips or recording; glass, frost, Zeron's visual style or theme registry; Linux as a first-class shipping target; implicit global `PATH` or login-shell lookups for agents.

## Cap and Zeron reference patterns

Sources: [Zeron agent control](../reports/research-261013-1749-zeron-agent-control.md) (read at Zeron `v0.2.112`; the pin `09beace0` is two commits later, so its citations apply) and [Cap GPUI desktop UI](../reports/research-261013-1801-cap-gpui-desktop-ui.md) (`a32e4806b`). Each phase that uses a pattern has a "Reference patterns" section naming the source location and the fframes target.

**Adopted**

| Pattern | Source | Phase |
|---------|--------|-------|
| App bootstrap order: embedded assets and fonts, tokio bridge, key bindings, globals, window, then native work from a task | Cap `main.rs:242-560` | M2 p4 |
| Native window calls only from spawned tasks, never inside a gpui update | Cap `platform*.rs` | M2 p4 |
| Cap's whole component library and theme model, ported with only import changes | Cap `src/ui/`, `src/theme.rs` | M2 p2 |
| Cap's editor window layout (header, toolbar, sidebar, panels, section entities), settings pages and Windows caption | Cap `editor_window.rs`, `editor_panels.rs`, `editor_sidebar.rs`, `settings_window.rs`, `settings_pages.rs`, `ui/windows_caption.rs` | M2 p4, p6 |
| Key contexts: a focused text field owns editing chords before editor shortcuts | Cap `ui/text_input.rs` | M2 p4, M4 p5, M5 p2 |
| Engine work off the main thread; adapter probes cached by (path, mtime, size) and never re-run per snapshot | Cap `main.rs`; Zeron `executable.rs:146-289` and its `ListHarnesses` flaw | M2 p3 |
| High-rate preview as its own entity, other sections `.cached()`, no per-frame `window.refresh()`, `drop_image` for replaced frames | Cap `editor_window.rs:563, 608, 6354` | M2 p4, M4 p4, M4 p6 |
| Foreign-thread frame callback that never blocks (adapted: the producer overwrites one slot, so the newest frame wins, where Cap's `try_send` keeps the oldest); hot pixel loops optimised in dev builds | Cap `editor_window.rs:11779`, `kernels/` | M4 p6 |
| Drags measured in prepaint with a full-window drag layer | Cap editor drag layer (`editor_clips.rs`, `editor_panels.rs`) | M4 p5 |
| Windows launch suspended in a kill-on-close job (exists today), graceful cancel before job termination, `CLAUDECODE*` stripped, adapter files limited to `.exe/.cmd/.bat/.com` | Zeron `process/windows/mod.rs:127-152`, `lib.rs:601-605`, `process/owned.rs:6-17`, `executable.rs:405-468` | M3 p4 |
| Steering through `_session/steering` when advertised, else cancel and re-prompt with no tool open; one source for the steering mode | Zeron `acp/mod.rs`; `registry.rs` inconsistency | M5 p2 |
| Permission requests whose options carry no kind shown as questions | Zeron `acp/normalize.rs:488-503` | M5 p2 |
| Updates replayed during `session/load` dropped when rows come from the stored log | Zeron `acp/mod.rs` session setup | M5 p5 |
| Native Claude Code (stream-json) and Codex (`app-server`) drivers behind one trait, with user-answered approvals | Zeron `crates/harness/src/{lib,claude,codex}` | M5a |

**Not adopted**

- Zeron's native wires for OpenCode (HTTP + SSE), Pi (JSONL) and the Cursor SDK shim: only Claude Code and Codex get native drivers (M5a).
- Zeron's auto-approval of every permission request: fframes shows permission requests to the user (parity item 9) and validates candidates before apply.
- Zeron's discovery (process `PATH`, login-shell snapshot, version-manager directories, newest `--version` wins): a roadmap non-goal. fframes keeps explicit adapter files and `detect.rs`, which already resolves npm `.cmd` shims to their script instead of running `cmd.exe`.
- Zeron's model catalogs, harness preferences and relay to other devices: out of scope.
- Cap's native glass and vibrancy, panel levels, capture exclusion, tray and global hotkeys: a non-goal (no glass) or unrelated to the studio.
- Cap's `paint_surface_fitted` zero-copy path: the fork provides it, but it paints CVPixelBuffer/IOSurface frames, while fframes preview frames arrive from the worker process as RGBA; preview uses the `RenderImage` path on every OS.
- Cap's environment-variable harness (`CAP_GPUI_AUTO_*`) and binary hot swap: the roadmap replaces harness-driven checks with `#[gpui::test]`.
- Cap's separate gpui workspace: already the case (`desktop/Cargo.toml` is its own workspace).
- Cap's dev profile with dependencies at `opt-level = 2`: `desktop/Cargo.toml` sets no profile, but CI build time was just tuned; the targeted per-crate override in M4 p6 comes first, and this is revisited only if debug playback is still too slow.
- Cap's window registry of typed handles (`AppWindows`): fframes has one main window; adopt it only if a later phase opens a second window.

## Porting rules (all milestones)

- **Licence:** the user reports permission from the owners of Cap and Zeron to port code directly. This supersedes the "facts only" licensing line in [261011-0033-cap-style-studio-ui](../261011-0033-cap-style-studio-ui/plan.md).
- **Source pins:** Cap `a32e4806b3004071effe00243e1dabd52bc7068c` (`apps/desktop-gpui`, on the same patched fork as the app, so Cap ports need type mapping but no GPUI API changes); Zeron `09beace00fbd4d521bf1e7fd4f591e5ab13162fb` (on zui, so every Zeron port also adapts GPUI calls to Cap's fork).
- **Cap first for UI.** Every window, panel, editor surface, settings page and component starts from Cap's code. Zeron supplies only agent internals (markdown parsing and rendering, composer logic, transcript list, diff viewer internals, todo data, search scoring), drawn with Cap's kit.
- **Zeron UI files are not drop-in.** They depend on `zeron_proto`, `zeron_doc`, `zeron_rpc`, `zeron_syntax` and many `crate::*` modules (settings, popover, frost, dictation). Spike S4 measures the closure; each port task names the fframes type it maps to and leaves frost, dictation and Zeron settings out.
- **Split on port.** Files over 1,500 lines are split in the porting commit; each task names its module layout.
- **Keep the ACP driver** (`acp_v1` tests) for every agent except Claude Code and Codex, which get native drivers ported from Zeron's `crates/harness/src/{claude,codex}` (M5a). Discovery stays explicit for all three wires: never `PATH` or login-shell resolution. Zeron's approval answers are never ported.

## Execution

Work on `feat/studio-rewrite` from `main` (`ci/faster-desktop-ci` and `ci/fast-push-checks` are merged, PRs #3 and #4). Each milestone merges when its success criteria and the [merge gates](./phase-01-milestone-merge-gates.md) pass. Deep mode: each phase starts with a scout pass of the files it names; line numbers cited in phases were taken on 2026-10-13 and drift.

## Red Team Review

### Session — 2026-10-13

**Findings:** 15 after deduplicating 38 (15 accepted, 0 rejected; 3 sub-points rejected: removing snapping, dropping syntax highlighting, and the per-frame export cost model, the first two because the user confirmed them).
**Severity breakdown:** 6 Critical, 9 High.

| # | Finding | Severity | Disposition | Applied to |
|---|---------|----------|-------------|------------|
| 1 | Qualification includes the runtime writer-containment gate | Critical | Accept; user chose runtime containment | M2 p1, p7; M3 p4; M6 |
| 2 | `timeline.json` write invalidates the build and wipes undo | Critical | Accept | Roadmap; M4 p1, p3 |
| 3 | SDK pins and project-owned worker block the hook | Critical | Accept | M4 p1, p2 (new) |
| 4 | Template has no scenes; split keys collide | Critical | Accept | M1 p1; M4 p1, p2 |
| 5 | Case-insensitive `.git` alias bypass on NTFS/APFS | Critical | Accept | M3 p3 |
| 6 | Per-file accept publishes an unvalidated subset | Critical | Accept; user chose re-validation | M5 p4 |
| 7 | Harness kept and relocated until M6; release gate deleted without replacement | High | Accept | M2 p7 (new); M6 |
| 8 | Rename, service move and parity location under-specified | High | Accept | M1 p3; M2 p1 |
| 9 | M3 misses `MUTATION_QUALIFIED`, recovery suites, executor, `FileId` width, NT APIs | High | Accept | M1 p1; M3 p1-p3 |
| 10 | Tool broker is Unix-only | High | Accept; user chose named pipes | M3 p5 (new) |
| 11 | Playhead port drops epoch fencing | High | Accept | M2 p3; M4 p6 |
| 12 | Zeron ports not drop-in; state lifetimes undefined | High | Accept | M1 p1 (S4); M2 p3; M5 |
| 13 | 1,500-line rule fails on unassigned files | High | Accept | M2 p8 (new); gates |
| 14 | M5 ACP facts wrong; history store is single-session | High | Accept | M5 |
| 15 | Link opening, PATH detection, unbounded timing input, attachments, fork supply chain | High | Accept | M1 p1; M4 p1; M5 p1, p2; gates |

Export presets: user chose to extend the export contract (M4 p1, p2, p7).

### Whole-Plan Consistency Sweep

Applied after the edits above; results recorded in [the merge gates phase](./phase-01-milestone-merge-gates.md#consistency-sweep).

## Validation Log

### Session — 2026-10-13

Questions resolved from repository evidence; the open ones are recorded as assumptions for the executor to confirm at the named point.

| # | Question | Resolution | Source | Applied to |
|---|----------|-----------|--------|-----------|
| 1 | How does a new SDK reach projects: crates.io or the app bundle? | Bundled. Projects build offline against `framework/vendor` from the shipped SDK. The earlier "publish to crates.io" step was wrong and is replaced; enforcement accepts the legacy and the new pin set so unmigrated projects still build. | `build_materialization.rs` crates-io `replace-with = "studio-vendor"`; `desktop/scripts/assemble-phase-zero-sdk.py` (`cargo vendor`) | Roadmap contract; M4 p2 |
| 2 | Which GitHub organisation holds the zui and gpui-base mirrors? | **Assumption:** the owner of `origin` (`khanhthanhdev`). Confirm before creating the mirrors in M1 spike S1. | `git remote -v` | M1 p1 (S1) |
| 3 | Who runs the macOS manual checks? | **Assumption:** the user on their own Mac; this VM is Windows and CPU-only. macOS CI covers the automated part. Confirm when M1 S1 needs the first manual run. | memory note on the VM; `desktop-release.yml` macOS job | M1 p1; M6 p2 |
| 4 | Do M4 and M5 run in parallel or in sequence? | Parallel, as planned; they own disjoint `studio-ui` modules and coordinate only on `studio-proto`. With a single executor, run M4 first because it carries the SDK migration. | Roadmap milestone table | Roadmap |

No contradictions with confirmed user decisions. `ak plan validate` re-run after these edits.

### Session 2 — 2026-10-13 (after the plan review)

Review: [review-261013-1617-studio-rewrite-roadmap](../reports/review-261013-1617-studio-rewrite-roadmap.md). All 23 findings applied.

| # | Question | Resolution | Applied to |
|---|----------|-----------|-----------|
| 1 | Do the CI branches land first? | Yes: `ci/faster-desktop-ci` (#3) and `ci/fast-push-checks` (#4) are on `main`. Merge gates and the release gate require a green `full` self-hosted run, because pushes run the fast checks only. | Gates; M2 p7 |
| 2 | Linux apply gap between M2 p7 and M3 p4? | Closed; confirmed by the user on 2026-10-13: `qualify_writer` and ledger reading are kept (moved to `studio-engine::containment`) until M3 p4 deletes them together with the ledger; M3 p4 adds a production-policy test with no ledger. The alternative, running M3 p4 before M2 p7, was not chosen because it would make M2's merge wait on an M3 phase. | M2 p1, p7; M3 p4 |
| 3 | Timing undo lost on restart? | Accepted; recorded as a non-goal. | Roadmap; M4 p3 |
| 4 | Gate 7 if the GPUI fork does not render on this VM? | The user runs the manual checks on their own machine. | Gates; M1 p1 |

### Session 3 — 2026-10-13 (Cap and Zeron research)

The two research reports were mapped onto the phases (see [Cap and Zeron reference patterns](#cap-and-zeron-reference-patterns)). Patterns that conflict with the contract (native agent wires, auto-approval, `PATH` discovery) are listed as not adopted. Spike S5 (M1) measures the ACP turn-end problem that made Zeron leave ACP, with a real adapter on this VM. Repository check: the Windows Job Object launch that M3 phase 4 planned to add already exists in `studio-bootstrap/src/process.rs`; the phase now reuses it.

### Session 4 — 2026-10-13 (Cap instead of zui)

| # | Decision | Applied to |
|---|----------|-----------|
| 1 | GPUI base is Cap's fork (`wingleeio/zed` `5d1f83d` plus Cap's three patches) through the mirror `khanhthanhdev/zed-cap`; zui and `gpui-base` are dropped. | Contract; M1 p1 (S1), p2; gates |
| 2 | UI comes from Cap: component kit and theme, editor window layout, panels and sidebar, timeline, clips and export, settings pages for setup and providers, and the agent panel's chrome. Zeron supplies agent internals only, adapted from zui to the fork. | M2 p2, p4, p6; M4 p4, p7; M5 |
| 3 | M2 is no longer free of visible change: the app takes Cap's components and editor layout; parity tests still assert behavior. M2 p4 grows to 5-7d. | M2 |

### Session 5 — 2026-10-13 (clarifying questions)

| # | Question | Answer | Applied to |
|---|----------|--------|-----------|
| 1 | ACP turn-end risk for Claude and Codex? | Native wires for Claude Code and Codex in a new milestone M5a after M3 (run after M4 with one executor); ACP stays for other agents. Spike S5 now proves both native protocols on real CLIs. | Contract; M1 S5; M5a; M5 |
| 2 | Approvals on native wires? | Always asked of the user (Claude `--permission-prompt-tool stdio`, Codex `on-request`); never auto-approved. | Contract; M5a |
| 3 | Where does the agent panel live? | A tab in the editor sidebar next to Presets and Scenes. | Contract; M2 p4, p6 |
| 4 | Theme choice? | Cap's System, Light or Dark setting, default System. | Contract; M2 p2 |
| 5 | Mirror owner? | The `khanhthanhdev` account (session 1 assumption confirmed). | M1 S1, p2 |
| 6 | `qualify-writer` until M3 p4? | Confirmed. | Session 2 row 2 |
| 7 | How many executors? | One: milestones run in sequence M1, M2, M3, M4, M5a, M5, M6. | Roadmap order |
