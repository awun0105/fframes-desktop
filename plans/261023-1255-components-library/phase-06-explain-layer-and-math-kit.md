---
phase: 6
title: "Phase 6: Explain layer and math kit"
status: completed
priority: P1
effort: 5d
dependencies: [2]
---

# Phase 6: Explain layer and math kit

## Goal

Add Manim's coordinate space and mobjects (`Canvas`, `Axes`, `NumberPlane`, `NumberLine`,
`FunctionGraph`, `VGroup.arrange`, `Matrix`, `Table`, plots) and the 12 math figures, with the
`docs/components/math.md` skill file.

## Context

- Units: Manim research section 3 (8 units tall, origin centred, y up); component model report
  section 2 (`Canvas` maps Manim units to pixels for its children).
- Mobject mapping: Manim research section 5. Math figures: strategy section 7 (math table).
- Plots: math report section 4 (`LinePlot`, `BarChart`, `Histogram`, `ScatterPlot`, `Heatmap`
  on the library's `Axes`; no `plotters`).
- Data crates allowed in `math`: `glam`, `num-complex`, `statrs` (all wasm32-clean; check each
  with `just -f components/justfile check-wasm` when added).

## LaTeX labels

This phase depends on phase 2 only, so the math kit does not wait for a TeX install. Labels,
axis numbers and `Matrix`/`Table` cells use plain `<text>` by default; with the `latex`
feature (phase 5) the same props switch them to `MathTex`. If phase 5 is done before this
phase ends, verify both builds; otherwise phase 5 adds the `math,latex` gallery check.

## Requirements

- `Canvas { children, unit: f32 (px per Manim unit, default HEIGHT/8) }`; inside, `Ui` carries
  a `Space` so explain components take Manim coordinates.
- `space` module: `Space` (data to pixel), `Axes { x_range, y_range, x_length, y_length, tips,
  axis_config }`, `NumberPlane`, `NumberLine`, `Axes::c2p`/`p2c`, ticks and labels (labels via
  `MathTex` when `latex` is on, plain text otherwise).
- `FunctionGraph { f: fn(f32) -> f32, x_range }`, `ParametricFunction`, `axes.plot(..)`
  sampled once per component build (not per frame unless an `Anim` parameter changes).
- `VGroup` with `arrange(direction, buff)` and `arrange_in_grid`; `layout` module with `row`,
  `column`, `grid`, `Anchor` with safe area.
- `Matrix`, `Table`, `Brace` with label, `ValueTracker` expressed as `Anim<f32>`.
- `label` module: `Label`, `Leader`, `Callout` reuse (phase 4), simple overlap avoidance.
- `sim` module: `Sim::precompute(dt, steps, step_fn)` and `sample(t)` with interpolation
  (needed by `Epicycles` here and by physics later).
- Math figures (`math` feature), in this order: `NumberLine` point/interval, `FunctionPlot`
  with tracer, `Tangent`/`Secant`, `RiemannSum`/`AreaUnder`, `UnitCircle`, `LinearTransform`,
  `VectorSum`, `Pythagoras`, `FractionBar`/`FractionPie`, `MatrixView`, `Histogram` to
  `NormalCurve`, `Epicycles`.
- Plots: `LinePlot`, `BarChart` (reuse phase 4), `Histogram`, `ScatterPlot`, `Heatmap`.

## Files

- Create: `components/fframes-kit/src/explain/{canvas.rs,space.rs,axes.rs,graph.rs,vgroup.rs,matrix.rs,table.rs}`,
  `components/fframes-kit/src/{layout.rs,label.rs,sim.rs}`, `components/fframes-kit/src/kits/math/*.rs`,
  `components/fframes-kit/src/plot/*.rs`, `components/gallery/src/scenes/{explain.rs,math.rs}`,
  `docs/components/math.md` (doctested like `components.md`).
- Modify: `components/fframes-kit/Cargo.toml` (feature `math` with optional `glam`, `num-complex`,
  `statrs`), `prelude.rs`, `lib.rs` (Manim table extended), `docs/components/README.md`
  (one line for `math.md`), gallery features, `components/scripts/e2e.mjs` frames.

## Tasks & Steps

1. `Canvas` and `Space`, with tests for `c2p`/`p2c` round trips.
2. Axes family and `FunctionGraph`; port two Manim gallery examples from the Manim research
   (`SinAndCosFunctionPlot`, `GraphAreaPlot`) as gallery scenes.
3. `VGroup`, `layout`, `label`, `sim` with unit tests (layout positions, `sim` sampling
   interpolates and clamps).
4. Math figures in table order; each one: component, gallery scene, `math.md` entry (one
   sentence, one snippet, the gallery scene name).
5. Plots.
6. Add a representative subset to the e2e frame list (one frame per figure would be too many:
   pick `FunctionPlot@end,UnitCircle@50%,LinearTransform@end,RiemannSum@end,Epicycles@50%`).

## Verification

- `cargo test -p fframes-kit --features math` (unit and doctests of `math.md`), and
  `--features math,latex` once phase 5 is done.
- `strip <Figure> -n 8` reviewed for each figure; `inspect --pops` clean on the gallery.
- `just -f components/justfile e2e cpu`, `just -f components/justfile check-wasm` with `math`, `just -f components/justfile clippy`.
- Bench: `BenchKit` unaffected; one math scene timed and recorded.

## Risks

- **Too many figures.** Build in table order and stop at a useful set if the phase overruns;
  record what is left in the phase report rather than rushing quality.
- **Per-frame sampling.** `FunctionGraph` must cache its samples; review with the cost rule.

## Todo

- [x] `Canvas` and `Space`
- [x] `Axes`, `NumberPlane`, `NumberLine`, `FunctionGraph`
- [x] `VGroup`, `layout`, `label`, `sim`
- [x] `Matrix`, `Table`, `Brace`
- [x] The 12 math figures in table order
- [x] Plots
- [x] `math.md` doctested; e2e subset
