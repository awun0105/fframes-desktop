---
title: "fframes-kit: component library"
description: "Build fframes-kit in a fork-owned components/ workspace: a Remotion-style component library with Manim's vocabulary, LaTeX formulas through a user-installed TeX, and science kits, simple pieces first, with no changes to upstream fframes code."
status: pending
priority: P1
effort: 47d
branch: main
tags: [feature, library, rust, components]
blockedBy: []
blocks: []
created: 2026-10-23
---

# fframes-kit: component library

## Overview

A new crate, `fframes-kit`, in a fork-owned nested Cargo workspace at `components/` (like
`desktop/`), gives every fframes video one import
(`use fframes_kit::prelude::*;`) and a catalog of components: time containers, shapes,
Manim-named animations, basic transitions, general motion-graphics pieces, LaTeX formulas, and
explainer kits for math, physics, chemistry, computer science and biology. A component is a
props struct plus `render(self, &mut Ui)`; time is local and set by `Sequence`, as in Remotion.

The order is simple first: the core, shapes and basic transitions, then the general
components, then LaTeX and the explainer kits, then the subject kits. The kit is independent of
fframes Studio: it uses only upstream fframes APIs and never touches `desktop/` or the Studio
crates.

## Which report is authoritative

Read in this order. Later items override earlier ones where they conflict. This plan overrides
all of them on formulas: **LaTeX through a user-installed TeX, no Typst.**

1. [Overview map](../reports/components-261023-0753-overview-map.md): source pointers for every
   candidate copied in the examples. Still valid.
2. [Strategy](../reports/components-261023-0804-library-strategy.md): use only sections 6
   and 7 (catalogs), 9 (quality gates) and 11 (risks). Its section 4 contract, `start`/`exit`
   props, section 8 spike and section 10 roadmap are **superseded**.
3. [Manim API research](../reports/research-261023-1404-manim-api-for-fframes.md): sections 3
   and 5 (units, mapping tables). Its Typst rendering (section 4) and the recorded `Script` are
   not in this plan.
4. [Component model](../reports/research-261023-1700-component-model-remotion-manim.md): **the
   contract** (section 3), the tag form as the kit's `ui!` macro (section 4), and **no changes
   to upstream code** (section 10). Its section 5 (Studio editing) and the `studio` feature are
   **not used**: the kit is independent of Studio.
5. [Math, LaTeX and science](../reports/research-261023-1718-math-latex-science-support.md):
   only the science crates of section 4 (`rapier2d`, `chematic`, `mendeleev`,
   `rust-sugiyama`) and the parts problem of section 3. Its Typst engine, MiTeX, vendored Typst
   packages and CeTZ are **dropped**.

## Decisions taken for this plan (2026-10-23)

| Decision | Choice | Effect |
| --- | --- | --- |
| Crate name | `fframes-kit` (crate `fframes_kit`) | `fframes-components` is a third-party crates.io crate (0.1.2) that two examples use; it stays untouched |
| Props API | struct update syntax with themed defaults: `LowerThird { name, ..ui.defaults() }` | no builder dependency; missing required data renders nothing and `inspect` warns |
| Formulas | full LaTeX through the user's TeX install (MiKTeX on Windows, Ubuntu, macOS; TeX Live works too), Manim's `latex` + `dvisvgm` pipeline, SVG cache | any package works (amsmath, mhchem, physics, siunitx, TikZ, chemfig, circuitikz); a committed cache renders without TeX; the web editor shows the TeX source as text |
| Typst | not supported | no Typst crates, packages or CeTZ; the `Formula` type does not depend on the engine, so a second engine can be added later without changing components |
| Transitions | basic set early (phase 3): `Fade`, `Slide`, `Wipe`, linear and spring timing, `SceneFade` | advanced presentations and effects wait for phase 11 |
| Studio props panel | **cancelled** (user, 2026-10-23: no changes to fframes core or fframes Studio; the kit must be independent) | phase 8 is not done; phases 9 to 11 no longer depend on it |
| Tag form | the kit's own proc macro `fframes_kit::ui!` wrapping `svgr!`, implicit variable named `ui` | `svgr-macro` is not changed; the macro error says how to create `ui` when it is missing |
| Core changes | **none** (user, 2026-10-23: "i do not want to modify the core of fframes since i will continue pull the latest changes"); this replaces the earlier "additive only" row | text measured through `ctx.font_source` and borrowed strings, tags through `ui!`, warnings through the public `fframes::diagnostics::report`; see "Upstream boundary" below |
| Layout | nested workspace `components/` (`fframes-kit`, `fframes-kit-macros`, `gallery`, own `justfile`, `scripts/e2e.mjs`, `Cargo.lock`); agent references in `docs/components/` | upstream merges never conflict; moving the kit to its own repository later is a folder move |
| Fork-only APIs | **not used**: no `EditorObjectKey`, `Svgr::with_editor_object`, `Scene::editor_instance_key` or `fframes::Styles`, no `studio` feature | the kit builds against plain upstream fframes; `Theme` is the kit's own type with its own defaults |
| Example migration | only the fork-only `fframes-studio-intro` (and later `vinuni-future-rises`) | `examples/fframes-intro` is upstream-owned and keeps its helpers |
| MiKTeX on this VM | approved (2026-10-23): phase 5 installs it without asking (Chocolatey; this VM has no winget) | real formulas render and the `tex-cache/` is generated here |
| Phase 8 | **the loop stops before phase 8** (user, 2026-10-23) | phase 8 is cancelled; phases 9 to 12 wait for a new decision to run them |
| Commits | one conventional commit per finished phase on `main`, scope `kit` (2026-10-23) | stage only the phase's own paths, never the unrelated dirty files already in the tree |

## Phases

| # | Phase | Depends on | Effort | Status |
|---|-------|-----------|--------|--------|
| 1 | [Core crate and contract](./phase-01-core-crate-and-contract.md) | none | 3d | Completed |
| 2 | [Shapes, paths and enter animations](./phase-02-shapes-paths-and-enter-animations.md) | 1 | 3d | Completed |
| 3 | [Basic transitions](./phase-03-basic-transitions.md) | 1 | 2d | Pending |
| 4 | [Text, general layer and example migration](./phase-04-text-general-and-example-migration.md) | 2 | 5d | Pending |
| 5 | [LaTeX formulas](./phase-05-latex-formulas.md) | 2 | 4d | Pending |
| 6 | [Explain layer and math kit](./phase-06-explain-layer-and-math-kit.md) | 2 (LaTeX labels when 5 is done) | 5d | Pending |
| 7 | [Component tags with the ui! macro](./phase-07-component-tags-in-svgr.md) | 2 | 3d | Pending |
| 8 | [Studio props panel](./phase-08-studio-props-panel.md) | n/a | 0d | Cancelled |
| 9 | [Physics and chemistry kits](./phase-09-physics-and-chemistry-kits.md) | 6, 8 | 6d | Pending |
| 10 | [CS and biology kits](./phase-10-cs-and-biology-kits.md) | 6, 8 | 6d | Pending |
| 11 | [Effects and more transitions](./phase-11-effects-and-more-transitions.md) | 3, 8 | 3d | Pending |
| 12 | [Getting started and packaging](./phase-12-scaffolding-and-studio-template.md) | 4 | 1d | Pending |

Phases 2 and 3 can run in parallel after phase 1; 4, 5, 6 and 7 can run in parallel after phase 2;
9, 10 and 11 can run in parallel after phase 6 (11 after phase 3). Each owns its own module, gallery file and skill
reference.

## Success criteria (whole plan)

All kit commands run from `components/` (its own workspace) or as `just -f components/justfile ..`.

- [ ] `cargo test -p fframes-kit --all-features` passes, including doctests of every
      `docs/components/` file the kit owns.
- [ ] `just -f components/justfile check-wasm` passes with the default and 2D kit features.
- [ ] `just -f components/justfile e2e cpu` passes with the gallery's snapshots (formulas from
      the committed TeX cache); Skia-only parts are covered by `compile_shader` tests.
- [ ] `just -f components/justfile boundary` prints nothing after every phase.
- [ ] `inspect` on the gallery reports no errors; `inspect --pops` reports no unintended pops.
- [ ] The kit-vs-hand-written bench scene stays within 10% (phase 1 baseline).
- [ ] The duplicated `beat.rs`/`ui.rs` helpers are gone from the fork-only
      `examples/fframes-studio-intro`, with unchanged key frames (`examples/fframes-intro` is
      upstream and untouched).
- [ ] `docs/latex-setup.md` explains installing MiKTeX on Windows, Ubuntu and macOS, and a
      missing TeX gives that guidance in the error.
- [ ] Every subject kit has a gallery scene per figure and a skill reference file.

## Verification environment

Windows Server VM, CPU only (no Vulkan): render and snapshot with `--renderer cpu` and
`just -f components/justfile e2e cpu`; Skia shader code is checked with `compile_shader` tests. Verify locally; do not
wait on CI. `node` is available for scripts, `python` is not. The `wasm32-unknown-unknown`
target is installed. No TeX is installed; phase 5 installs MiKTeX (approved 2026-10-23).

## Upstream boundary

Baseline commit: `cdf2f12a` (`main` before the kit work, 2026-10-23).

Upstream-owned paths (from `git ls-tree upstream/main`) are never modified: `fframes*/`,
`svgr-macro/`, `media-dir-macro/`, `webvtt-parser/`, `cargo-fframes/`, `examples/` (except
the fork-only `examples/fframes-studio-intro` and `examples/vinuni-future-rises`), `skills/`,
`e2e/`, `scripts/`, `justfile`, `render-bench/`, `package.json`, `.github/`. fframes Studio is off
limits too, although the fork owns it: `desktop/`, `fframes-studio-protocol/`,
`fframes-studio-runtime/`. The only allowed
edits in upstream-owned files are one word in the root `Cargo.toml` `exclude` line and one
pointer line in `AGENTS.md` (phase 1), plus the root `Cargo.lock` entries that phase 4 adds when the
fork-only `fframes-studio-intro` starts depending on the kit. Reading upstream code is fine.

`components/justfile` `boundary` recipe (phase 1 creates it; the kit's commits use the scope
`kit`, so merges from upstream and unrelated fork commits are not counted):

```sh
cd .. && git log --no-merges --grep='(kit)' --name-only --format= cdf2f12a..HEAD -- \
  fframes fframes-media fframes-editor fframes-editor-controller fframes-skia-renderer \
  fframes-three-d fframes-native-player fframes-test-utils fframes-lyon-renderer svgr-macro \
  media-dir-macro webvtt-parser cargo-fframes examples skills e2e scripts justfile render-bench \
  package.json .github desktop fframes-studio-protocol fframes-studio-runtime \
  ':!examples/fframes-studio-intro' ':!examples/vinuni-future-rises' | sort -u
cd .. && git log --no-merges --grep='(kit)' -p --format=%h cdf2f12a..HEAD -- Cargo.toml AGENTS.md
```

The first command must print nothing. The second prints the kit commits' own edits to
`Cargo.toml` and `AGENTS.md`; check by eye that they are only the `exclude` word and the one
pointer line.
Uncommitted changes that were in the tree before the kit work (`typos.toml`,
`fframes/src/renderer/tests/timing_tests.rs`, `.github/workflows/*`, `desktop/**`,
`examples/fframes-studio-intro/src/scenes/keynote/speed.rs` and others) belong to the user:
never stage, revert or format them. Stage kit work by explicit path.

## Implementation loop

Started with `/loop /ak:cook plans/261023-1255-components-library/plan.md --auto` (self-paced).
Status words: the table uses `Pending`, `In progress`, `Completed`, `Blocked`, `Cancelled`; phase files use
`todo`, `in-progress`, `completed`, `blocked`, `cancelled` for the same states. The picker skips
`Cancelled` phases. Each firing:

1. Read this file and the phase table. Pick the lowest-numbered phase whose status is not
   `Completed` and whose dependencies are `Completed`. A phase marked `In progress` is resumed
   from its Todo list (checked items are done; verify them quickly before continuing).
2. **Stop conditions**, checked before any work. End the loop (`ScheduleWakeup` with
   `stop: true`) and tell the user why when:
   - the next phase to run is 8 or higher (user decision: the loop stops before phase 8);
   - a phase is `Blocked` (its file records the blocker);
   - the same check failed in two consecutive firings after an advisor call;
   - the boundary check prints anything;
   - all phases are `Completed`.
3. Set the phase to `In progress` here and in its file (`status: in-progress`), then implement
   it with the cook workflow: tasks in order, the narrowest test first, then the phase's
   Verification list. Tick Todo items in the phase file as they finish, so a later firing can
   resume. Record measurements (bench, TeX timings) in
   `plans/reports/kit-phase-NN-{YYMMDD-HHMM}.md`.
4. Before declaring the phase done, run the boundary check and call the advisor to audit the
   phase against its requirements (user rule). Fix what it finds.
5. Commit: `git add` the phase's paths explicitly (`components/`, `docs/components/`,
   `docs/latex-setup.md`, this plan folder, the report, and the two pointer edits in phase 1;
   `examples/fframes-studio-intro/` in phase 4 only for files the phase changed), then
   `git commit -m "feat(kit): <what the phase delivered>"`. No AI references or attribution
   trailer in the message (user rule), no plan IDs or phase numbers in commit messages or code.
   Do not push.
6. Set the phase to `Completed` (table and file), then schedule the next firing
   (`ScheduleWakeup`, about 60 s, same prompt). If the phase is too large for one firing, commit
   the finished, green part as `feat(kit): ...`, leave the phase `In progress` with its Todo
   ticks, and schedule the next firing.
7. A failed check is never hidden or weakened. When a phase's own risk section says to stop
   (for example two `fframes` packages in phase 4, a failed MiKTeX install in phase 5), mark the
   phase `Blocked` with the reason and stop the loop.

## Recorded open questions (not blocking)

- Real LaTeX runs on Ubuntu and macOS cannot be checked on this VM; phase 5 tests discovery
  with fake executables, and a run on those systems is a manual check.
- Which parts-marking method works with dvisvgm (`\special` groups or unique colours); phase 5
  decides on its first step.
- Embedding the TeX cache in the web editor bridge, so formulas show there too, is left for
  later.
- Whether `chematic` exposes 2D coordinates (phase 9 checks first).
- Remotion transition names beyond `fade`, `slide`, `wipe` are unverified (phase 11 checks).

## Validation Log

Round 1, prompt mode, 4 questions (2026-10-23): Studio panel timing (after general and math),
CeTZ (vendor), props API (struct update with themed defaults), crate name (`fframes-kit`, after
finding that `fframes-components` 0.1.2 on crates.io belongs to a third party and is used by
`examples/fframes-studio-intro` and `examples/vinuni-future-rises`; `fframes-kit` is free).

Round 2 (2026-10-23, user direction): full LaTeX with a user-installed MiKTeX on Windows,
Ubuntu and macOS; no Typst (this voids the CeTZ answer of round 1); simple things first; basic
transitions. The formula spike and the Typst formula phase were replaced by phase 5, basic
transitions moved to phase 3, and component tags moved after the LaTeX work.

Verification against the code, tier Full (12 phases): text helper signatures and
`clone_with_scene_offset` in `fframes/src/frame.rs`; `EditorObjectKey` in `fframes/src/editor.rs`;
`duration_in_frames` in `fframes/src/fframes_context.rs`; 40 tokens in the built-in presets and
4 in the Studio template; `kurbo` 0.13 optional in `fframes/Cargo.toml`; `usvgr` re-exported at
`fframes/src/lib.rs:205`; the `svgr()` entry in `svgr-macro/src/lib.rs`; `check-wasm` and `e2e`
recipes; the `scripts/e2e.mjs` example list; the timing pipeline (`timing_file` at
`fframes/src/renderer/cli.rs:485`, `set_timing` at `fframes/src/renderer/preview.rs:365`,
`SetTiming` and `set_timing_v1` in the studio protocol and runtime); no `latex`, `dvisvgm` or
MiKTeX on this VM; every other file path named in the phases. Failed: none. Unverified:
external crate facts from the research reports (checked in phases 9, 10 and 11 before use).

Round 3 (2026-10-23, user direction): no changes to upstream fframes code. Phases 1, 3, 4, 5, 6,
7, 8, 9, 10 and 12 were revised: nested `components/` workspace, agent references in
`docs/components/`, `ui!` macro instead of a `svgr-macro` pre-pass, `ctx.font_source` for
per-frame strings, only fork-only examples migrate, phase 8 is a gate, phase 12 no longer edits
`cargo-fframes` or `scripts/publish.sh`. Same round: commits per phase on `main` (scope `kit`),
MiKTeX install approved, the loop stops before phase 8.

Round 4 (2026-10-23, user direction): "stop before phase 8, i do not want to touch the fframes
core. or fframes studio. It must be independence". Phase 8 cancelled; the `studio` feature,
editor keys and `Theme::from_styles` removed from phase 1; `Keyed`/`#[derive(Props)]` removed
from phases 2, 9, 10, 11; phase 12 reduced to the getting-started page and packaging (no
Studio template); `desktop/` and the Studio crates added to the boundary check.

Note: the plan directory is named `components-library`; the crate it builds is `fframes-kit`.
