---
phase: 11
title: "Phase 11: Effects and more transitions"
status: in-progress
priority: P3
effort: 3d
dependencies: [3]
---

# Phase 11: Effects and more transitions

## Goal

Extend the basic transitions of phase 3 with more presentations, and ship the effects module
(`Trail`, `CameraMotionBlur`, `LightLeak`, `Glow`, `Blur`, `Noise`).

## Context

- Phase 3 already ships `TransitionSeries`, the `Presentation` trait, `Fade`, `Slide`, `Wipe`,
  `Timing::Linear`, `Timing::Spring` and `SceneFade`. This phase only adds presentations
  through that trait.
- Component model report sections 1 and 6 (`transitions`, `effects` rows). `Flip`,
  `ClockWipe` and `Iris` are unverified Remotion names: read the current Remotion docs before
  naming them.
- Overview map section 8 (`SlantWipe` from `examples/vinuni-future-rises/src/lib.rs:260`,
  `CircleWipe` from `examples/marketing/src/marketing.rs:403-410`) and section 6 (glow and
  shadow sources).
- Shader-based effects run only on Skia; this VM has no GPU, so they are checked with
  `fframes_skia_renderer::render::compile_shader` tests and the CPU fallback draws nothing.
- Runs in parallel with phases 9 and 10; owns `components/fframes-kit/src/effects/` and new files in
  `components/fframes-kit/src/transitions/`.

## Requirements

- Presentations: `Flip`, `ClockWipe`, `Iris` once their Remotion names are verified;
  `SlantWipe` and `CircleWipe` ported from the examples.
- Effects: `Trail { samples, spacing }` (renders the child at earlier local times with falling
  opacity), `CameraMotionBlur { shutter_angle, samples }`, `LightLeak { seed, hue_shift }`
  (SVG gradients, works on CPU), `Glow`, `Blur` (SVG filters), `Noise` (seeded; SkSL version
  behind `skia`).
- Every component: themed defaults, plain-data props, `key` where it creates SVG ids.

## Files

- Create: `components/fframes-kit/src/transitions/{flip.rs,clock_wipe.rs,iris.rs,slant_wipe.rs,circle_wipe.rs}`,
  `components/fframes-kit/src/effects/*.rs`, `components/gallery/src/scenes/effects.rs`,
  `components/fframes-kit/tests/shaders.rs` (compile tests, `skia` feature).
- Modify: `components/fframes-kit/Cargo.toml` (`skia` feature), `prelude.rs`,
  `components/gallery/src/scenes/transitions.rs` (new scenes), `components.md`
  (transitions and effects sections), `components/scripts/e2e.mjs`.

## Tasks & Steps

1. Verify the Remotion presentation names on remotion.dev; record them in the phase report.
2. New presentations through the phase 3 `Presentation` trait.
3. Effects; `Trail` and `CameraMotionBlur` render the child closure several times, so cap
   `samples` and document the cost.
4. Gallery scenes and e2e frames (`CircleWipe@50%,Trail@50%,LightLeak@50%`).

## Verification

- `cargo test -p fframes-kit --features skia` (shader compile tests) and default features.
- `strip CircleWipe -n 6`, `onion Trail -n 6`; `inspect --pops` clean.
- `just -f components/justfile e2e cpu`, `just -f components/justfile check-wasm`, `just -f components/justfile clippy`.

## Risks

- **Cost of multi-sample effects.** Measure `Trail` with 8 samples against the bench pair and
  document it in `components.md`.

## Todo

- [ ] Remotion names verified
- [ ] `Flip`, `ClockWipe`, `Iris`, `SlantWipe`, `CircleWipe`
- [ ] Effects, with shader compile tests behind `skia`
- [ ] Gallery scenes and e2e frames
