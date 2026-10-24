---
phase: 17
title: "Phase 17: Coordinate systems, fields, graphs, tables and text mobjects"
status: todo
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

1. **Numbers and text.** Done first, because phase 16 uses `DecimalNumber`.
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

- [ ] Numbers and text mobjects
- [ ] Coordinate systems and `LogBase`
- [ ] Implicit functions, vector fields, stream lines
- [ ] Graph layouts, `Graph` and `DiGraph`
- [ ] Typed matrices and tables
- [ ] Gallery scenes, e2e frames, docs and mapping rows
