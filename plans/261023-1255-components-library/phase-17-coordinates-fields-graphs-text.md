---
phase: 17
title: "Phase 17: Coordinate systems, fields, graphs, tables and text mobjects"
status: in-progress
priority: P1
effort: 6d
dependencies: [6]
---

# Phase 17: Coordinate systems, fields, graphs, tables and text mobjects

## Goal

Add the remaining 2D mobjects of the Manim CE reference:
- the coordinate systems and implicit plots;
- vector fields and stream lines;
- `Graph`/`DiGraph` with automatic layouts;
- the typed matrix and table variants;
- the number and text mobjects.

## Context

- The list is in
  [manim-remotion-usage](../reports/research-261024-1748-manim-remotion-usage.md), section
  "Manim CE mobjects". It ranks:
  - `Variable` 2,616;
  - `DecimalNumber` 2,136;
  - `Integer` 1,952;
  - `MarkupText` 1,144;
  - `BulletedList` 350;
  - `ComplexPlane` 276;
  - `ImplicitFunction` 264;
  - `PolarPlane` 258;
  - `StreamLines` 221;
  - `ArrowVectorField` 184;
  - `DiGraph` 164;
  - `MathTable` 160.

  `Graph` and `Title` are common words; their counts are noise.
- Existing modules:
  - `components/fframes-kit/src/explain/`: `axes.rs`, `space.rs`, `graph.rs`, `matrix.rs`,
    `table.rs`, `vgroup.rs`;
  - `components/fframes-kit/src/text/`.
  - `explain::Axes` already has `c2p`, `p2c`, `plot`, the Riemann sums and areas.
- Graph layouts are shared with phase 21 (`MGraph`, `MTree`) and with the CS kit's
  `GraphView`. They live in `components/fframes-kit/src/layout/`, next to the existing
  layered layout of `rust-sugiyama`.
- If this phase grows past 6 days, split it into "planes and fields" and "graphs, tables
  and text". Each half owns its own files, as listed below.

## Requirements

- **Coordinate systems:**
  - `ComplexPlane` (`n2p`, `p2n`, labels in `a+bi` form);
  - `PolarPlane` (`pr2pt`, `pt2pr`, azimuth units in degrees, radians or π);
  - `LogBase` scaling on `Axes` and `NumberLine`;
  - `UnitInterval`;
  - `SampleSpace` (divisions with braces and labels).
- **Functions:**
  - `ImplicitFunction`: marching squares on a grid, with the resolution as a prop;
  - `FunctionGraph` as a mapping row.
- **Vector fields:**
  - `ArrowVectorField` and `VectorField` (with colour by magnitude);
  - `StreamLines`, with Manim's `start_animation` as a flowing dash along each line.
  - The lines are traced once, in the constructor or through `Sim::precompute`, and never
    per frame.
- **Graphs:**
  - `Graph` and `DiGraph` from vertices and edges.
  - Layouts: `circular`, `kamada_kawai`, `planar` (or a documented fallback), `random`
    (seeded), `shell`, `spectral`, `spiral`, `spring`, `partite`, `tree`, and a
    `LayoutFunction` closure.
  - Per-vertex and per-edge styles and labels.
  - Edges can be straight or curved, with tips for `DiGraph`.
- **Matrices:** `DecimalMatrix`, `IntegerMatrix`, `MobjectMatrix` (cells are `Svgr`).
- **Tables:** `DecimalTable`, `IntegerTable`, `MathTable` (feature `latex`), `MobjectTable`.
- **Numbers and text:**
  - `DecimalNumber`, with `num_decimal_places`, `include_sign`, `group_with_commas`,
    `unit`, and a monospaced layout so the digits do not jitter;
  - `Integer`;
  - `Variable` (a label, `=`, and a value read from a `ValueTracker` or a closure);
  - `BulletedList`;
  - `Title` (with an underline);
  - `MarkupText`, a Pango-like markup subset: bold, italic, colour, size, underline.
- **Trackers:** `ComplexValueTracker`, and `VDict` as a mapping row onto a keyed `VGroup`.
- Every name goes into the Manim mapping table in `lib.rs`.

## Files

- Create:
  - `components/fframes-kit/src/explain/{complex_plane.rs,polar_plane.rs,sample_space.rs,implicit.rs,vector_field.rs,stream_lines.rs,typed_matrix.rs,typed_table.rs}`;
  - `components/fframes-kit/src/layout/graph_layouts.rs`;
  - `components/fframes-kit/src/explain/graph_mobject.rs`;
  - `components/fframes-kit/src/text/{decimal.rs,variable.rs,bulleted_list.rs,title.rs,markup.rs}`;
  - `components/gallery/src/scenes/explain_ii.rs`.
- Modify:
  - `components/fframes-kit/src/explain/{mod.rs,axes.rs,graph.rs,matrix.rs,table.rs}`;
  - `components/fframes-kit/src/text/mod.rs`;
  - `components/fframes-kit/src/layout.rs` moves to `layout/mod.rs`, keeping
    `layout::layered` and the other public paths;
  - `prelude.rs`, `lib.rs`;
  - `components/gallery/src/scenes/mod.rs`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/components.md` and `docs/components/math.md`.

## Tasks & Steps

1. **Numbers and text.** Phase 16's number animations build on `DecimalNumber`.
2. Coordinate systems and `LogBase`.
3. `ImplicitFunction`, vector fields and stream lines.
4. Graph layouts, then `Graph` and `DiGraph`.
5. Typed matrices and tables.
6. Gallery scenes and e2e frames (`ComplexPlane@end`, `StreamLines@50%`, `GraphLayouts@end`,
   `DecimalNumber@50%`).

## Verification

- `cargo test -p fframes-kit --all-features`:
  - `n2p`/`p2n` and `pr2pt`/`pt2pr` round trips;
  - marching squares on a circle gives points at the radius within the grid step;
  - every layout is deterministic, and keeps vertices apart by a minimum distance;
  - `DecimalNumber` formatting with sign, commas and units.
- `inspect --pops` on `DecimalNumber` shows no digit jumps beyond the counter.
- `just -f components/justfile e2e cpu`, `check-wasm`, `clippy` and `boundary`.

## Risks

- **Planar layout.** A planarity test and an embedding are not trivial. If no small crate
  fits, fall back to `spring` and document it.
- **`MarkupText` scope.** Support only the listed subset, and let `inspect` warn about
  unknown tags.

## Todo

- [x] Numbers and text mobjects
- [x] Coordinate systems and `LogBase`
- [x] Implicit functions, vector fields, stream lines
- [x] Graph layouts, `Graph` and `DiGraph`
- [x] Typed matrices and tables
- [ ] Gallery scenes, e2e frames, docs and mapping rows

## Notes

- `DecimalNumber`, `Integer` and `Variable` follow a `ValueTracker` (`f32`), so an existing
  tracker drives them; a closure of time is `f(ui.seconds()).into()`. `Counter` shares their
  formatting.
- No jitter is checked in `gallery/tests/decimal_number.rs`: the point and the commas keep their x
  while the digits change, also across a carry with `anchor: End`.
- `inspect --pops` on `DecimalNumber` reports the leading digits and the sign flip as pops "like
  a counter": the number counting, not a layout jump.
- `MarkupText` writes each run as a sibling `<tspan>` with its whole style. Underline draws; bold
  and italic need the family's bold and italic faces in the media (the gallery has only DM Sans
  Medium, so they look regular there).
- `BulletedList` draws its bullets as circles, independent of the font.
- `Scaling::{Linear, LogBase}` is on `Axes` (`x_scaling`, `y_scaling`), `NumberLine` (`scaling`)
  and `AxesMap`, which gained the two fields and `a2p` (axis units to pixels): a change to the
  shape of a public struct. Ranges are in axis units (exponents), as in Manim; points, numbers
  and graph ranges take values. Graphs on a log x-axis sample evenly by exponent
  (`sample_scaled`). Every existing frame stayed at 0.000% on CPU; skia-cpu shows only its
  drifts from before the change.
- `UnitInterval` is the constructor `NumberLine::unit_interval(theme)`.
- `ComplexValueTracker` (`Anim<(f32, f32)>`) came with the complex plane instead of with the
  trackers, and `ComplexPlane::label` writes `a+bi` with `format_complex`.
- `SampleSpace` scales probabilities that do not add up to 1 and divides one level (columns,
  then rows), which covers Manim's conditional-probability figures.
- DM Sans has `π` but not `τ`; the gallery uses `PiRadians` and the docs name the font need.
- Extra e2e frames: `PolarPlane@end`, `LogBase@end`, `SampleSpace@end`.
- `ImplicitFunction`, `VectorField` and `StreamLines` take plain `fn`s, so what they trace is
  cached by the function's address and the props (the graph sample cache, keyed by kind):
  traced on first use in a render, never per frame, without `Sim::precompute`.
- `VectorField` is not drawn by itself, as in Manim it is the base: it holds the function, the
  grid and the colour by magnitude that `ArrowVectorField` and `StreamLines` share. `Field` is
  not in the prelude because the physics kit exports a `Field`; it is `explain::Field`.
- `StreamLines` colours a whole line by the magnitude at its middle, where Manim colours
  along the line. `start_animation` is the prop `flow`: a piece of each line slides along it
  from a seeded phase (`slice_path`), rather than Manim's dash and fade. Starts where the field
  is zero draw nothing.
- Extra e2e frames: `ImplicitFunction@end`, `ArrowVectorField@end`.
- The layouts are `layout::GraphLayout` and `layout::graph_layout` in `layout/graph_layouts.rs`,
  written in the crate without a new dependency: Fruchterman-Reingold for `spring`, stress
  majorization for `kamada_kawai`, Jacobi rotations for `spectral` (`spring` above 400
  vertices), `tidy_tree` for `tree`. `planar` is `spring` with seed 0 (the documented
  fallback). Every layout is rescaled like networkx and then pushed `0.4 * scale / √n` apart.
  `LayoutFunction` is a plain `fn(count, edges, scale)`, so the layout cache keys it by address.
- Manim's `Graph` is `explain::Graph`, outside the prelude, because the CS kit's `Graph` is in
  it; `DiGraph { graph }` wraps it like `ComplexPlane { plane }`. Labelled vertices grow by the
  length of their name (no font measurement), and the label colour is one prop, `label_fill`.
  The kit's shared drawing helpers (`kits/draw.rs`) are now compiled without the CS and biology
  features.
- Extra e2e frame: `DiGraph@end`.
- The typed matrices and tables wrap `Matrix` and `Table` (`matrix: Matrix`, `table: Table`, like
  `DiGraph { graph }`); the numbers are written by `DecimalNumber`'s formatting, with commas.
  `MobjectMatrix` and `MobjectTable` take `Mobject { svgr, width, height }` cells, drawn around
  the origin (`Mobject::from_shapes` centers and measures shapes); they hold `Svgr`, which has
  no `PartialEq`, so they derive no studio props. `MathTable` (feature `latex`) takes formulas
  typeset beforehand and draws them through `MobjectTable`; the gallery has no TeX engine, so
  it is covered by the type check and the docs, not by a frame.
- `Matrix` and `Table` draw through a shared `draw(ui, grid, body)` now; their frames stayed at
  0.000%.
- Extra e2e frame: `TypedTables@end`.
