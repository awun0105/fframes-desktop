---
phase: 2
title: "Phase 2: Shapes, paths and enter animations"
status: in-progress
priority: P1
effort: 3d
dependencies: [1]
---

# Phase 2: Shapes, paths and enter animations

## Goal

Add the shapes every other layer draws with, path utilities over `kurbo::BezPath`, and the
Manim enter animations that need geometry (`Create`, `Write`, `GrowFromCenter`,
`DrawBorderThenFill`, `Transform`).

## Context

- Component model report section 6 (`shapes`, `paths` rows) and 3.5 (`Enter::apply` gets a
  `Geometry`). Manim research section 5 (animation and mobject tables, field names).
- Remotion shapes: `Arrow, Callout, Circle, Ellipse, Heart, Pie, Polygon, Rect, Spark, Star,
  Triangle`. Manim additions: `Square, Line, DashedLine, Dot, Arc, Annulus, RoundedRectangle,
  Vector, CurvedArrow, Brace`.
- Draw-on sources to absorb: `examples/conference-splash-screen/src/speaker_screen.rs:119`,
  `examples/marketing/src/marketing.rs:429`, `examples/fframes-studio-intro/src/scenes/install.rs:47`.
- `scale_about` sources: `examples/motion-graphics/src/quote_card.rs:157`.

## Requirements

- `Shape` value: a `kurbo::BezPath` plus fill, stroke, stroke width, dash, opacity. Every shape
  component builds one and draws `<path d=..>`. The `d` string comes from props, so it is
  built when the component is built and never rebuilt for animation; animation goes on the
  wrapping `<g transform opacity>`.
- `Geometry` trait: `path() -> BezPath`, `length()`, `bbox()`, implemented by every shape and
  by `Formula` later (phase 5) and by text outlines.
- `paths` module (Remotion names, snake_case): `get_length`, `get_point_at_length`,
  `get_tangent_at_length`, `evolve_path`, `interpolate_path`, `translate_path`, `scale_path`,
  `reverse_path`, `cut_path`, `warp_path`, `normalize_path`, `get_bounding_box`,
  `get_subpaths`, `extend_view_box`, `center_path`, `parse_path`, `serialize`, `reset_path`,
  `reduce_instructions`, plus `align_points(a, b, n)` for `Transform`.
- Animations with Manim fields (`run_time` 1.0, `rate_func` Smooth, `lag_ratio` where Manim
  has it): `Create` (dash offset over `length()`), `Uncreate`, `DrawBorderThenFill`,
  `GrowFromCenter`, `GrowFromPoint`, `GrowArrow`, `SpinInFromNothing`, `Write` (on text
  outlines: stagger per glyph), `Transform` and `ReplacementTransform` (point-aligned morph),
  `Indicate`, `Circumscribe`, `Flash`, `Wiggle`, `MoveAlongPath`.
- Group timing: `LaggedStart { lag_ratio, children }`, `AnimationGroup`, `Succession` as
  components that set each child's `delay`.
- Misuse that types can catch does not compile: `Create`/`Transform` require `Geometry`.

## Files

- Create: `components/fframes-kit/src/shapes/{mod.rs,basic.rs,arrows.rs,brace.rs,remotion.rs}`,
  `components/fframes-kit/src/paths.rs`, `components/fframes-kit/src/geometry.rs`,
  `components/fframes-kit/src/animate/{mod.rs,create.rs,write.rs,grow.rs,transform.rs,emphasis.rs,group.rs}`,
  `components/fframes-kit/src/text_outline.rs` (glyph outlines for `Write`, from font data, built once
  per string per video via `OnceLock` or the frame's text cache),
  `components/gallery/src/scenes/{shapes.rs,animate.rs}`.
- Modify: `components/fframes-kit/src/prelude.rs`, `components/fframes-kit/src/lib.rs` (Manim and Remotion tables),
  `docs/components/components.md` (shapes and animations tables),
  `components/scripts/e2e.mjs` (add frames), `components/gallery/snapshots/`.

## Tasks & Steps

1. `Geometry` and `Shape`; port draw-on math once.
2. Shapes in two batches: Manim set first (used by explain/math), Remotion set second. Each:
   struct with themed defaults, `Themed` impl, unit test of its path bbox.
3. `paths` functions over kurbo, each with a unit test (length of a unit circle is 2π within
   1e-3, `cut_path` halves, `interpolate_path` endpoints equal inputs).
4. Enter and emphasis animations. `Create` uses `stroke-dasharray`/`stroke-dashoffset`
   from `length()`. `Write` needs outlines: implement `text_outline` from
   `MediaProvider::get_all_font_data` + `ttf_parser`, cached once per (font, size, text); if
   fonts are not resolvable (editor system fonts) fall back to `FadeIn` per word.
5. `Transform`: resample both paths to the same point count with `align_points`, lerp per
   frame. `ReplacementTransform` is the same with the source removed at the end.
6. Group timing components.
7. Gallery scenes, one per component, named after it; the Manim classic "SquareToCircle" scene
   (`Create` a square, `Transform` into a circle, `FadeOut`).
8. Update `components.md` and the crate-doc tables; add the classic scenes to the e2e frame
   list.

## Verification

- `cargo test -p fframes-kit` (unit and doctests).
- `cargo run --release -p components-gallery -- --renderer cpu strip SquareToCircle -n 8`
  shows the expected motion; `inspect --pops` clean on `SquareToCircle`, `Write`, `Transform`.
- `just -f components/justfile e2e cpu`, `just -f components/justfile check-wasm`, `just -f components/justfile clippy`.
- Bench pair still within 10%.

## Risks

- **Per-frame `d` strings.** Morphing builds a new `d` each frame; that is unavoidable, but
  static shapes must not. Review that no static shape rebuilds strings per frame.
- **Text outlines in the editor.** Fonts may not resolve; the fallback must render something.

## Todo

- [x] `Geometry` and `Shape`
- [x] Manim shapes, then Remotion shapes
- [x] `paths` functions with tests
- [x] `text_outline` with editor fallback
- [x] `Create`, `Write`, grow, emphasis and `MoveAlongPath` animations
- [x] `Transform` and `ReplacementTransform`
- [x] `LaggedStart`, `AnimationGroup`, `Succession`
- [ ] Gallery scenes incl. `SquareToCircle`; docs tables; e2e frames
