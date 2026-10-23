---
phase: 3
title: "Phase 3: Basic transitions"
status: todo
priority: P1
effort: 2d
dependencies: [1]
---

# Phase 3: Basic transitions

## Goal

Ship the basic Remotion-style transitions: `TransitionSeries` with `Fade`, `Slide` and `Wipe`
presentations, `Linear` and `Spring` timings, and `SceneFade` for the edges of a scene. The
advanced presentations and the effects module wait for phase 11.

## Context

- Component model report section 1 (Remotion's `TransitionSeries`, `fade()`, `slide()`
  (Remotion's default), `wipe()`, `linearTiming`, `springTiming`). These three names were
  verified on remotion.dev on 2026-10-23.
- Overview map section 8: `SceneFade` source `examples/vinuni-future-rises/src/lib.rs:87`;
  `CrossFade` and `Push` exist only as recipes in `skills/fframes-video/references/design.md`.
- Core scene cross-fades: `Overlap::{Previous, Next, PreviousAndNext}` and
  `ctx.get_scene_info(&scene)` for the resolved frame range.
- Time containers from phase 1 (`Sequence`, `Series`) provide the frame arithmetic.

## Requirements

- `TransitionSeries` with `TransitionSeries::sequence(duration, children)` and
  `TransitionSeries::transition(timing, presentation)` items. A transition overlaps the two
  neighbours by its duration, so the series is shorter by the sum of the transitions, as in
  Remotion. Two transitions in a row, or a transition first or last, render nothing for that
  transition and `inspect` warns.
- `Presentation` trait: `fn apply(&self, progress: f32, side: Side, content: Svgr, ui: &Ui) -> Svgr`
  where `Side` is `Entering` or `Exiting`, so users can write their own.
- Presentations: `Fade`, `Slide { direction }` (`FromLeft`, `FromRight`, `FromTop`,
  `FromBottom`), `Wipe { direction }` (a clip rectangle that grows; the clip id comes from the
  component key or the item index inside the series, never a global counter).
- Timings: `Timing::Linear { duration, easing }` and `Timing::Spring { mass, stiffness,
  damping, duration: Option<f32> }` (duration defaults to the spring's settle time, from
  `motion::spring`).
- `SceneFade { fade_in, fade_out, children }`: fades at the edges of the enclosing `Sequence`
  or scene, using `ui.duration`.
- `scene_transition(ui, presentation, timing, side)`: applies a presentation inside a core
  `Scene` that declares an `Overlap`, so scenes cross-fade or slide without a
  `TransitionSeries`.
- Every component: themed defaults (durations from `motion.duration.*`, easing from
  `motion.easing.*`), `key`.

## Files

- Create: `components/fframes-kit/src/transitions/{mod.rs,series.rs,presentation.rs,fade.rs,slide.rs,wipe.rs,timing.rs,scene_fade.rs}`,
  `components/gallery/src/scenes/transitions.rs`.
- Modify: `components/fframes-kit/src/prelude.rs`, `components/fframes-kit/src/lib.rs` (Remotion table rows),
  `docs/components/components.md` (transitions section, which replaces the cross-fade and
  push recipes of the upstream `design.md` for kit users; `design.md` itself is not edited),
  `components/scripts/e2e.mjs` (frames), `components/gallery/snapshots/`.

## Tasks & Steps

1. `TransitionSeries` frame arithmetic with unit tests: total length, local time of each item,
   both items visible during the overlap, transition progress 0 at start and 1 at end.
2. `Presentation` trait and `Fade`, `Slide`, `Wipe`.
3. `Timing::Linear` and `Timing::Spring`; test that a spring timing without a duration ends
   when the spring settles.
4. `SceneFade` (port from `vinuni-future-rises`) and `scene_transition` for core scenes.
5. Gallery scenes `Fade`, `Slide`, `Wipe`, `SceneFade`, and `SceneOverlap` (two core scenes
   with `Overlap::Previous` and a slide).
6. `components.md` entries with doctests; e2e frames
   `Fade@50%,Slide@50%,Wipe@50%,SceneOverlap@50%`.

## Verification

- `cargo test -p fframes-kit`.
- `cargo run --release -p components-gallery -- --renderer cpu strip Slide -n 8`;
  `inspect --pops` clean on the transition scenes (a transition must not pop at its edges).
- `just -f components/justfile e2e cpu`, `just -f components/justfile check-wasm`, `just -f components/justfile clippy`.

## Risks

- **Clip ids.** Two wipes on screen must not share a clip id; derive it from key or index and
  test two wipes in one frame.
- **Overlap length.** A transition longer than either neighbour is clamped to the shorter one,
  with an `inspect` warning.

## Todo

- [ ] `TransitionSeries` with frame arithmetic tests
- [ ] `Presentation` trait, `Fade`, `Slide`, `Wipe`
- [ ] `Timing::Linear` and `Timing::Spring`
- [ ] `SceneFade` and `scene_transition` for core scenes
- [ ] Gallery scenes, `components.md` transitions section, e2e frames
