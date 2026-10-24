---
phase: 15
title: "Phase 15: Manim shapes II"
status: in-progress
priority: P1
effort: 5d
dependencies: [2]
---

# Phase 15: Manim shapes II

## Goal

Add every 2D Manim CE mobject in geometry that the kit still lacks, so that every Manim
geometry name maps to a kit construct.

## Context

- The list is in
  [manim-remotion-usage](../reports/research-261024-1748-manim-remotion-usage.md), section
  "Manim CE mobjects". It ranks by distinctive names:
  - `SurroundingRectangle` 7,216;
  - `CurvedArrow` 1,716;
  - `Underline` 1,448;
  - `DashedVMobject` 1,242;
  - `ArcBetweenPoints` 856;
  - `BackgroundRectangle` 700;
  - `BraceBetweenPoints` 479;
  - `RightAngle` 391;
  - `Elbow` 236;
  - `LabeledDot` 158.
- The existing shapes are in `components/fframes-kit/src/shapes/` (`basic.rs`, `arrows.rs`,
  `brace.rs`, `remotion.rs`). Path helpers are in `paths.rs`, and geometry in `geometry.rs`.
- The matchers need the bounds of what they surround. Shapes already know their geometry.
  For text and formulas, use the measured boxes the kit already computes (`text`,
  `latex::FormulaView`).

## Requirements

- **Matchers:**
  - `SurroundingRectangle`, `BackgroundRectangle`, `Cross` and `Underline`.
  - Each one takes a box (`Rect`) or anything that gives its bounds, with `buff` and
    corner radius as in Manim.
- **Arcs and curves:**
  - `ArcBetweenPoints`, `CurvedArrow`, `CurvedDoubleArrow`, `TangentialArc`, `CubicBezier`;
  - `AnnularSector`, `ArcPolygon`, `ArcPolygonFromArcs`.
- **Dots and labels:**
  - `AnnotationDot`, `LabeledDot`;
  - `LabeledLine`, `LabeledArrow`, `LabeledPolygram`.
  - Labels can be text, or a formula with the `latex` feature.
- **Angles:**
  - `Angle` (with `other_angle`, `quadrant`, `dot` and a label), `RightAngle`, `Elbow`;
  - `TangentLine` as a mapping row onto the kit's `Tangent`, or a 2D version for a path.
- **Polygons:**
  - `Polygram`, `RegularPolygram`, `ConvexHull`, `Cutout`;
  - `FullScreenRectangle`, `ScreenRectangle`.
- **Arrow tips:**
  - `ArrowTriangleTip`, `ArrowTriangleFilledTip`;
  - `ArrowCircleTip`, `ArrowCircleFilledTip`;
  - `ArrowSquareTip`, `ArrowSquareFilledTip`;
  - `StealthTip`.
  - The tip is a prop of every arrow (`tip_start`, `tip_end`).
- **Braces:** `ArcBrace`, `BraceBetweenPoints`, `BraceText`.
- **Dashes:**
  - `DashedVMobject`: dashes along any path, with `num_dashes` and `dashed_ratio`, by arc
    length.
  - `CurvesAsSubmobjects`: a path split into its curves, so each one can be styled or
    animated.
- **Boolean operations:**
  - `Union`, `Intersection`, `Difference`, `Exclusion` on `kurbo::BezPath`.
  - They use `i_overlay` (see step 1). It works on polygons, so curves are flattened at
    0.25 px first.
- **SVG import:**
  - `SvgShape`, Manim `SVGMobject`: an SVG file from media, drawn as `svgr` with its parts
    addressable by id or index.
  - `SvgPath`, `VMobjectFromSVGPath`: one path from a `d` string.
  - Both parse through `usvgr`, which fframes re-exports.
- **Images:** `ImageMobject` is a mapping row onto `<image>` together with the kit's
  `KenBurns` and `PhotoFrame`. Write a doc example instead of a new type.
- **Shared rules:**
  - Every shape has themed defaults and the `enter`, `exit` and `emphasis` props of phase 2.
  - Every shape works with `Create` and `Write`.
- Add every name to the Manim mapping table in `lib.rs`.

## Files

- Create:
  - `components/fframes-kit/src/shapes/{matchers.rs,arcs.rs,labeled.rs,angles.rs,polygram.rs,tips.rs,dashed.rs,boolean.rs,svg_import.rs}`;
  - `components/gallery/src/scenes/shapes_ii.rs`.
- Modify:
  - `components/fframes-kit/src/shapes/{mod.rs,arrows.rs,brace.rs}`;
  - `components/fframes-kit/Cargo.toml` (the boolean-ops crate);
  - `prelude.rs`, `lib.rs`;
  - `components/gallery/src/scenes/mod.rs`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/components.md` (the shapes section).

## Tasks & Steps

1. **Boolean-ops crate: `i_overlay` 9.1.** Chosen on 2026-10-24, and the probe passed:
   - Licence MIT or Apache-2.0. It builds for wasm32. The union of two overlapping 2×2
     squares gives the exact area 7.0.
   - API: `SingleFloatOverlay::overlay(&other, OverlayRule::Union, FillRule::EvenOdd)`
     returns a `Vec` of shapes. A shape is a `Vec` of contours: contour 0 is the outer ring,
     and the rest are holes.
   - Flatten curves at 0.25 px before the operation; the result is lines.
   - Fallback: `flo_curves` 0.8 (Apache-2.0, builds), which keeps Bézier curves. Use it only
     if flattened results look wrong under `Create`.
2. Matchers and dashes.
3. Arcs, curves, dots and labels.
4. Angles, polygrams and tips.
5. Braces, boolean operations, SVG import, and the image mapping row.
6. Gallery scenes and e2e frames (`Matchers@end`, `Angles@end`, `Boolean@end`,
   `SvgShape@end`).

## Verification

- `cargo test -p fframes-kit`:
  - matcher boxes with `buff`;
  - angle arcs in all four quadrants;
  - `DashedVMobject` gives equal dash lengths on a curve;
  - boolean results on two overlapping squares (known area);
  - SVG import of a sample file keeps the number of paths.
- `frame Matchers@end --ids` by eye.
- `just -f components/justfile e2e cpu`, `check-wasm`, `clippy` and `boundary`.

## Risks

- **Boolean operations on curves.** Flattening changes the shape slightly. Document the
  tolerance, and keep the result as lines.
- **SVG import scope.** Files with filters, CSS or text may draw differently. Support
  shapes, groups, fills, strokes and gradients, and let `inspect` warn about the rest.

## Todo

- [x] Boolean-ops crate chosen and recorded
- [x] Matchers and dashes
- [ ] Arcs, curves, dots and labels
- [ ] Angles, polygrams and tips
- [ ] Braces, boolean operations, SVG import, image mapping
- [ ] Gallery scenes, e2e frames, docs and mapping rows
