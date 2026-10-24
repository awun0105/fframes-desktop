---
phase: 13
title: "Phase 13: Remotion transitions II"
status: todo
priority: P2
effort: 3d
dependencies: [11]
---

# Phase 13: Remotion transitions II

## Goal

Add the Remotion presentations that SVG can draw. With phase 11 they cover every Remotion
presentation except the pixel and shader ones, which the user deferred.

## Context

- The research report
  [manim-remotion-usage](../reports/research-261024-1748-manim-remotion-usage.md) is the list
  this phase works from (table "Remotion transitions"). `@remotion/transitions` has 1.52M
  downloads a month.
- All presentations go through the phase 3 `Presentation` trait
  (`components/fframes-kit/src/transitions/presentation.rs`). Phase 11 added `Flip`,
  `ClockWipe`, `Iris`, `CircleWipe` and `SlantWipe` the same way.
- Blur and turbulence are SVG filters (`feGaussianBlur`, `feTurbulence`, `feComponentTransfer`).
  Both renderers draw them, so nothing here needs the `skia` feature.
- Deferred, do not implement: `crosswarp`, `ripple`, `bookFlip`, `cube`,
  `custom-html-in-canvas`.
- This phase owns new files in `components/fframes-kit/src/transitions/` and the transition
  scenes of the gallery.

## Requirements

- **Presentations**, each one a `Presentation` with themed defaults and plain-data props.
  Read the Remotion doc page of each one first and match its props:
  - `NoTransition`: Remotion `none`. The overlap is kept and the cut happens at its middle.
  - `PushCut`
  - `ZoomInOut`
  - `DreamyZoom`
  - `LinearBlur`: directional blur with an angle.
  - `BlurSlide`
  - `CrossZoom`: an SVG approximation, with the approximation documented.
  - `Dissolve`: a seeded `feTurbulence` threshold mask.
  - `FilmBurn`: a gradient and turbulence overlay.
  - `SwapCards`: a 2D approximation of Remotion's card `swap`. The name `Swap` is left for
    Manim's `Swap` animation (phase 16). The mapping row reads `swap()` → `SwapCards`.
- **Transition progress inside a scene**, Remotion's `useTransitionProgress`: a scene reads
  how far its entering or exiting transition has got, and whether it is entering or
  exiting.
- **Audio fade on transitions**: a helper that returns the fade-in and fade-out times of
  each scene in a `TransitionSeries`. The user applies them to their scene audio
  (`AudioTrack::fade_in` / `fade_out`). The kit cannot change `fn audio` itself.
- Every filter id that the kit creates comes from the component `key`, so two transitions
  in the same frame do not collide.
- Add the new rows to the Remotion mapping table in `components/fframes-kit/src/lib.rs`.
- The transitions section of `docs/components/components.md` lists the deferred
  presentations (`crosswarp`, `ripple`, `bookFlip`, `cube`, `custom-html-in-canvas`) as not
  yet available, with a link to the usage report.

## Files

- Create: `components/fframes-kit/src/transitions/{none.rs,push_cut.rs,zoom.rs,blur.rs,dissolve.rs,film_burn.rs,swap.rs,progress.rs}`.
- Modify:
  - `components/fframes-kit/src/transitions/mod.rs`, `prelude.rs`, `lib.rs` (the mapping table);
  - `components/gallery/src/scenes/transitions.rs`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/components.md` (the transitions section).

## Tasks & Steps

1. Read the Remotion docs for each presentation. Record the props and any approximation in
   the phase report.
2. Presentations, simplest first: `NoTransition`, `PushCut`, `ZoomInOut`, then the blur
   family, then `Dissolve`, `FilmBurn` and `SwapCards`.
3. Transition progress, and the audio fade helper.
4. Gallery scenes and e2e frames (`Dissolve@50%`, `DreamyZoom@50%`, `FilmBurn@50%`,
   `SwapCards@50%`).

## Verification

- `cargo test -p fframes-kit`: unit tests for progress at 0, 0.5 and 1. Also test that
  `NoTransition` shows only the exiting scene before the middle of the overlap and only the
  entering scene after it.
- `strip Dissolve -n 6` and `strip SwapCards -n 6` by eye. `inspect --pops` shows only the
  expected cut in `NoTransition`.
- `just -f components/justfile e2e cpu`, `check-wasm`, `clippy` and `boundary`.

## Risks

- **Blur cost on the CPU backend.** A large `stdDeviation` on a full 1080p frame is slow
  with tiny-skia. Measure `DreamyZoom` on the bench pair and document the cost.
- **Approximations.** `CrossZoom` and `SwapCards` are not exact copies of the WebGL originals.
  Say so in their docs.

## Todo

- [ ] Remotion props verified and recorded
- [ ] `NoTransition`, `PushCut`, `ZoomInOut`
- [ ] Blur family: `DreamyZoom`, `LinearBlur`, `BlurSlide`, `CrossZoom`
- [ ] `Dissolve`, `FilmBurn`, `SwapCards`
- [ ] Transition progress and the audio fade helper
- [ ] Gallery scenes, e2e frames, docs and mapping rows
