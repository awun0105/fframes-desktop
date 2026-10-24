---
phase: 16
title: "Phase 16: Manim animations II"
status: in-progress
priority: P1
effort: 5d
dependencies: [2, 17]
---

# Phase 16: Manim animations II

## Goal

Give every Manim CE animation name a kit construct. Some become new components. The
imperative names become mapping rows with doc examples onto the components that already
exist.

## Context

- The list is in
  [manim-remotion-usage](../reports/research-261024-1748-manim-remotion-usage.md), section
  "Manim CE animations". It ranks by distinctive names:
  - `Restore` 2,040;
  - `TransformFromCopy` 1,728;
  - `Swap` 1,680;
  - `Unwrite` 1,076;
  - `TracedPath` 970;
  - `Rotating` 948;
  - `FadeTransform` 932;
  - `MoveToTarget` 902;
  - `ShowPassingFlash` 878;
  - `TransformMatchingShapes` 764.
- The existing animations are in `components/fframes-kit/src/animate/` (`create.rs`,
  `write.rs`, `grow.rs`, `emphasis.rs`, `transform.rs`, `group.rs`). The phase 2 contract
  (`EnterAnim`, `ExitAnim`, `Emphasis`, `ShapeAnimation`) is where new animations plug in.
- The kit is declarative: `render` is a function of local time.
  - Manim's imperative calls (`ApplyMethod`, `MoveToTarget`, `Restore`, `UpdateFromFunc`,
    `UpdateFromAlphaFunc`, `MaintainPositionRelativeTo`, `Wait`) have no state to act on.
  - They ship as rows in the Manim mapping table, each with a doc example on `Transform`,
    `Sequence`, `Anim` or `ui.sample`.
  - Do not invent types that duplicate those.

## Requirements

- **Creation:**
  - `Unwrite` (the exit of `Write`);
  - `AddTextLetterByLetter`, `AddTextWordByWord`, `RemoveTextLetterByLetter`;
  - `ShowIncreasingSubsets`, `ShowPartial`, `ShowSubmobjectsOneByOne`, `SpiralIn`;
  - `TypeWithCursor`, `UntypeWithCursor`. The kit's `Typewriter` gains the Manim names and
    an erase mode.
- **Growth and indication:**
  - `GrowFromEdge`, `ApplyWave`, `Blink`, `FocusOn`;
  - `ShowPassingFlash`, `ShowPassingFlashWithThinningStrokeWidth`;
  - `Broadcast`, `AnimatedBoundary`.
- **Movement:**
  - `Rotate` (angle about a point), `Rotating` (continuous);
  - `Homotopy`, `ComplexHomotopy`, `SmoothedVectorizedHomotopy` over path points;
  - `PhaseFlow`.
  - `TracedPath`: the trail of a point over local time. It samples earlier times, so cap
    the samples, as `Trail` does.
- **Transforms:**
  - `TransformFromCopy`, `FadeTransform`, `FadeTransformPieces`;
  - `ClockwiseTransform`, `CounterclockwiseTransform` (a transform along arcs);
  - `CyclicReplace`, `Swap`;
  - `FadeToColor`, `ScaleInPlace`, `ShrinkToCenter`;
  - `TransformMatchingShapes`: matched by path equality after normalising position, the way
    Manim does it;
  - `TransformAnimations`;
  - `ApplyMatrix`, `ApplyFunction`, `ApplyComplexFunction`, `ApplyPointwiseFunction`,
    `ApplyPointwiseFunctionToCenter`, all on path points.
- **Numbers:**
  - `ChangeDecimalToValue`, `ChangingDecimal`, on phase 17's `DecimalNumber`. That is why
    this phase depends on 17.
- **Time:**
  - `ChangeSpeed`: remaps local time through a speed curve for its children;
  - `LaggedStartMap`: a helper over `LaggedStart`.
- **Mapping rows only**, each with a doc example: `Wait`, `ApplyMethod`, `MoveToTarget`,
  `Restore`, `UpdateFromFunc`, `UpdateFromAlphaFunc`, `MaintainPositionRelativeTo`.
- **Shared rules:**
  - Every new animation follows the phase 2 contract.
  - Each one uses `run_time` and `rate_func`, with the kit's `RateFunc` names.

## Files

- Create:
  - `components/fframes-kit/src/animate/{text_by.rs,subsets.rs,indicate_ii.rs,rotate.rs,homotopy.rs,traced_path.rs,transform_ii.rs,apply.rs,speed.rs}`;
  - `components/gallery/src/scenes/animate_ii.rs`.
- Modify:
  - `components/fframes-kit/src/animate/{mod.rs,write.rs,transform.rs,group.rs}`;
  - `components/fframes-kit/src/text/typewriter.rs`;
  - `prelude.rs`, `lib.rs` (the mapping table);
  - `components/gallery/src/scenes/mod.rs`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/components.md` (the animations section).

## Tasks & Steps

1. **Mapping rows and doc examples for the imperative names.** These are doctests, so they
   are checked.
2. Creation animations.
3. Growth and indication.
4. Movement and `TracedPath`.
5. Transforms, including the matching of shapes.
6. Numbers and time.
7. Gallery scenes and e2e frames (`Unwrite@50%`, `TracedPath@end`,
   `TransformFromCopy@50%`, `ApplyMatrix@50%`).

## Verification

- `cargo test -p fframes-kit` (doctests included):
  - `ChangeSpeed` keeps the start and end of the time it remaps;
  - `Rotate` by 2π comes back to the start;
  - `TransformMatchingShapes` pairs identical shapes.
- `onion TracedPath -n 6` and `strip Swap -n 6` by eye. `inspect --pops` reports nothing
  unintended.
- `just -f components/justfile e2e cpu`, `check-wasm`, `clippy` and `boundary`.

## Risks

- **`TracedPath` and homotopies are costly.** They sample many times or many points. Cap
  them and document the cost.
- **Name clashes.** `Rotate`, `Swap` and `Broadcast` are common words. Keep them in
  `animate::`, and export them in the prelude only if nothing else there has the same
  name.

## Todo

- [x] Mapping rows and doc examples for the imperative names
- [x] Creation animations
- [x] Growth and indication
- [x] Movement and `TracedPath`
- [x] Transforms and shape matching
- [x] Numbers and time
- [x] Gallery scenes, e2e frames, docs

## Notes

- `inspect --pops` on every new scene (2026-10-25). Expected pops, all by design:
  - steps that change in one frame: `ShowIncreasingSubsets`, `ShowSubmobjectsOneByOne`,
    `AddTextLetterByLetter`, `RemoveTextLetterByLetter`, the `TypeWithCursor` letters and
    cursor, the `ChangeDecimalToValue` and `ChangingDecimal` digits;
  - fast motion that crosses a cell in one frame: the `SpiralIn` dots in their first frames,
    the `ShowPassingFlash` head;
  - `AnimatedBoundary` switching colour, as in Manim.
  The transforms, `Apply*`, homotopies, `Blink`, `ChangeSpeed` and `LaggedStartMap` report none.
- `Rotate`, `Swap` and `Broadcast` are in the prelude: nothing else there has those names.
- `TransformMatchingShapes` matches keys within one step of the rounding: glyph outlines
  are f32 and land a hair differently at other positions.
