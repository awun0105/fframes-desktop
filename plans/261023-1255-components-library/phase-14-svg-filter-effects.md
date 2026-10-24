---
phase: 14
title: "Phase 14: SVG-filter effects"
status: in-progress
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
- **Also deferred: every `feDisplacementMap` effect** (wave, waves, zigzag, noise
  displacement, turbulent displace). Checked on 2026-10-24 with a scratch video:
  - A flat `#c0c0c0` map at scale 29 should shift the picture by about 7 px. Skia-CPU does
    that, as the SVG spec says. The CPU backend draws garbage: on the repro frame (scale
    29), 57.27% of the pixels differ from Skia-CPU.
  - The fault is in the `svgr` crate (0.50.2, pinned by the root `Cargo.toml`), the CPU
    backend's filter code, inherited from resvg (0.45.1 to 0.48.1 draw the same):
    `scale` is applied twice, and the map is read premultiplied. Upstream issue
    linebender/resvg#1160, fix linebender/resvg#1157 (open on 2026-10-24).
  - With both lines fixed in a local copy of svgr, 0.50% of the pixels differ from
    Skia-CPU, at the e2e limit. The rest are thin lines along the edges of displaced
    shapes.
  - The PR draft for svgr, with the repro and the patch, is in
    `plans/reports/svgr-displacement-261024/`.
  - e2e renders on `cpu`, so any displacement effect would snapshot as garbage.
  - Do not patch the dependency and do not displace pixels in the kit (that is a per-pixel
    effect, which the user deferred). The fix belongs upstream; the effects come back when
    a fixed `svgr` arrives through the regular pull. Even then, check the e2e difference
    per effect: 0.50% is too close to the limit to assume it passes.

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
     - not noise displacement, turbulent displace, wave, waves or zigzag: they need
       `feDisplacementMap` (see Context).
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
  effects (the report's "Deferred" list) and the displacement effects as not yet
  available. It says why the displacement effects wait.
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

1. **Renderer checks: done (2026-10-24).**
   - `feTurbulence` agrees. On the gallery's `Noise@50%`, 34 pixels (0.0016%) differ at
     the e2e threshold of 0.15, against a limit of 0.5%.
   - `feDisplacementMap` does not agree, so its effects are deferred (see Context).
   - Every other new primitive is compared when its group's gallery scene renders
     (Verification).
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

- **Renderer differences.** `feDisplacementMap` differs and is deferred; `feTurbulence`
  agrees. Another primitive may differ too: the per-scene comparison finds it, and the
  effect is then documented or deferred the same way.
- **Cost.** Progressive blur and long shadows draw many copies. Cap the counts and document
  the cost per effect.
- **Effects in the editor.** The editor draws through the browser's SVG. It supports these
  filters, but its look is not checked by e2e.

## Todo

- [x] Renderer checks: `feTurbulence` agrees, `feDisplacementMap` deferred
- [x] Colour group and matrix composition
- [x] Edges and shape
- [x] Noise, noise functions
- [x] Patterns and overlays, halftone
- [ ] Geometry and reveals
- [ ] Blur group with measured cost
- [ ] Gallery scenes, e2e frames, docs and mapping rows
