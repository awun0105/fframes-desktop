---
phase: 14
title: "Phase 14: SVG-filter effects"
status: todo
priority: P2
effort: 5d
dependencies: [11]
---

# Phase 14: SVG-filter effects

## Goal

Add the `@remotion/effects` effects that SVG filters, masks and patterns can draw, as
wrappers around children.

## Context

- The list to work from is in
  [manim-remotion-usage](../reports/research-261024-1748-manim-remotion-usage.md), section
  "Remotion effects", "SVG-filter effects".
- Both renderers implement the full SVG filter set:
  - Skia handles the filter primitives in `fframes-skia-renderer`;
  - the CPU backend renders them through resvg.
- Phase 11 added `Blur`, `Glow`, `DropShadow`, `Noise`, `LightLeak` and `CameraMotionBlur`.
  Follow their file layout (`components/fframes-kit/src/effects/`,
  `components/fframes-kit/src/decor/filters.rs`).
- Glow and DropShadow already blur in sRGB (commit `b892569f`). New filters use the same
  `color-interpolation-filters` rule, so the CPU and Skia renders match.
- Deferred, do not implement: the per-pixel effects of the report's "Deferred" list
  (fisheye, pixelate, chromatic aberration, zoom blur, page turn, shatter and others).

## Requirements

- **Common shape.**
  - Every effect is a wrapper: `Effect { children, ..ui.defaults() }`.
  - Its parameters are plain data and can be animated through `Anim` or a closure of
    local time.
  - Every effect has a `key` for its filter id.
- **Groups**, by the report's sections:
  1. **Colour:**
     - brightness, contrast, saturation, hue, grayscale, invert, tint, duotone;
     - levels, curves, exposure, colour correction, white balance, vibrance;
     - shadows/highlights, colour key, thermal vision;
     - LUT. LUT and shadows/highlights are approximations through `feComponentTransfer`
       tables; their docs say so.
  2. **Edges and shape:** outline, emboss, bevel alpha, long shadows, roughen edges,
     shrinkwrap.
  3. **Noise and distortion:**
     - white noise and fractal noise, next to the existing `Noise`;
     - noise displacement, turbulent displace;
     - wave, waves, zigzag, through `feDisplacementMap` with generated maps.
  4. **Patterns and overlays:**
     - scanlines, dot grid, checkerboard, gridlines, lines, rings;
     - pattern, paper, burlap, flannel, speckle;
     - linear gradient tint, linear gradient, vignette, light trail;
     - starburst, which joins `LightLeak`.
  5. **Halftone:** dots through a pattern mask.
  6. **Geometry:** mirror, tile, skew, scale, x/y translate, uv translate.
  7. **Reveals:** venetian blinds, evolve, TV signal off.
  8. **Blur:** region blur (blur inside a mask), linear and radial progressive blur (blurred
     copies under graded masks; cap the copies and document the cost).
- **Effect stacks.** Effects nest. A helper composes several colour effects into one
  `feColorMatrix` where the maths allows it, so a stack costs one filter.
- **`noise2D`, `noise3D`, `noise4D`** as plain functions (Remotion `@remotion/noise`), for
  driving motion. Each one is seeded and deterministic.
- The effects section of `docs/components/components.md` lists the deferred per-pixel
  effects (the report's "Deferred" list) as not yet available.
- Add the new rows to the Remotion mapping table in `lib.rs`. Include the
  `@remotion/animation-utils` names as rows with doc examples:
  - `makeTransform` maps onto `fframes::Transform`;
  - `interpolateStyles` maps onto `motion::lerp` over the props.

## Files

- Create:
  - `components/fframes-kit/src/effects/{color.rs,edges.rs,distort.rs,patterns.rs,halftone.rs,geometry.rs,reveal.rs,progressive_blur.rs}`;
  - `components/fframes-kit/src/motion/noise.rs`;
  - `components/gallery/src/scenes/effects_ii.rs`.
- Modify:
  - `components/fframes-kit/src/effects/mod.rs`, `prelude.rs`, `lib.rs`;
  - `components/gallery/src/scenes/mod.rs`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/components.md` (the effects section).

## Tasks & Steps

1. **Check `feDisplacementMap` across renderers.** Write the first displacement effect and
   compare its CPU and Skia-CPU renders with the e2e difference (`--renderer skia-cpu` needs
   no GPU). Record the result in the phase report.
   - `feTurbulence` is already checked (2026-10-24). On the gallery's `Noise@50%`, 34 pixels
     (0.0016%) differ at the e2e threshold of 0.15, against a limit of 0.5%.
2. The colour group and the colour-matrix composition helper.
3. Edges and shape, noise and distortion, then the noise functions.
4. Patterns and overlays, halftone, geometry, reveals.
5. The blur group, with the cost measured on the bench pair.
6. Gallery: one scene per group showing every effect of the group on the same picture.
   e2e frames: one per group.

## Verification

- `cargo test -p fframes-kit`:
  - the colour matrices on known colours (grayscale of pure red, invert of white);
  - the noise functions are deterministic for a seed;
  - filter ids are unique per key.
- The CPU and Skia-CPU renders of each gallery scene agree within the e2e threshold, or
  the difference is documented on the effect.
- `just -f components/justfile e2e cpu`, `check-wasm`, `clippy` and `boundary`.

## Risks

- **Renderer differences.** resvg and Skia may differ in `feDisplacementMap`. Step 1
  measures this before the bulk of the work. `feTurbulence` already agrees.
- **Cost.** Progressive blur and long shadows draw many copies. Cap the counts and document
  the cost per effect.
- **Effects in the editor.** The editor draws through the browser's SVG. It supports these
  filters, but its look is not checked by e2e.

## Todo

- [ ] `feDisplacementMap` renderer agreement checked (`feTurbulence` done)
- [ ] Colour group and matrix composition
- [ ] Edges and shape
- [ ] Noise and distortion, noise functions
- [ ] Patterns and overlays, halftone
- [ ] Geometry and reveals
- [ ] Blur group with measured cost
- [ ] Gallery scenes, e2e frames, docs and mapping rows
