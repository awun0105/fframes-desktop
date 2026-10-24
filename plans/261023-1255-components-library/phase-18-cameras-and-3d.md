---
phase: 18
title: "Phase 18: Cameras and 3D"
status: in-progress
priority: P2
effort: 5d
dependencies: [6]
---

# Phase 18: Cameras and 3D

## Goal

Add Manim's camera scenes and its 3D mobjects:
- `MovingCameraScene`, `ZoomedScene`, `ThreeDScene`;
- the 3D shapes, built on `fframes::three_d` behind the kit's `3d` feature.

## Context

- The list is in
  [manim-remotion-usage](../reports/research-261024-1748-manim-remotion-usage.md). It
  ranks:
  - `ThreeDScene` 3,688;
  - `Surface` 2,784;
  - `ThreeDAxes` 2,308;
  - `MovingCameraScene` 2,180;
  - `Sphere` 1,560;
  - `Cube` 1,372;
  - `Cone` 902;
  - `Cylinder` 599;
  - `Prism` 420;
  - `Torus` 318;
  - `ZoomedScene` 280.
- 2D cameras are group transforms: a `<g transform>` around the children.
  - `MovingCamera` frames a box over time.
  - `ZoomedInset` draws the children twice, the second time inside a clipped, scaled frame,
    with the frame outline and the inset border.
- 3D uses `fframes::three_d` (AGENTS.md section 7d). The kit's `3d` feature already forwards
  `fframes/3d`, and `Molecule3D` already uses it.
  - The CPU rasterizer works on this VM and in the editor (at half size there).
  - Build geometry once, in the constructor, and clone the scene per frame.
- Manim's 3D camera uses `phi` (polar) and `theta` (azimuth). Map them onto
  `Camera::perspective(..).orbit(..)`, and document the conversion.

## Requirements

- **2D cameras:**
  - `MovingCamera`: a frame box animated through `Anim`, with Manim's `auto_zoom` onto a
    list of boxes.
  - `ZoomedInset`: Manim's `ZoomedScene`, with the zoom box and display box as props and an
    animated pop-out.
- **3D scene:** `ThreeDView`, Manim's `ThreeDScene`.
  - Camera props: `phi`, `theta`, `distance`, `focal_distance`.
  - `begin_ambient_camera_rotation` as a rotation rate.
  - Fixed-in-frame 2D overlays, positioned with `camera.project`.
- **3D mobjects:**
  - `Surface` (a parametric `uv` function with a checkerboard fill);
  - `ThreeDAxes` (with axis labels as fixed-in-frame text);
  - `Sphere`, `Cube`, `Prism`, `Cone`, `Cylinder`, `Torus`;
  - `Dot3D`, `Line3D`, `Arrow3D`;
  - `Tetrahedron`, `Octahedron`, `Dodecahedron`, `Icosahedron`, `ConvexHull3D`.
- **3D animation:** `Create` and `FadeIn` on 3D mobjects, through opacity and scale, the way
  the kit's 2D animations work.
- **`VectorScene`** is a mapping row: a `NumberPlane` with vectors and the kit's
  `LinearTransform`.
- Every name goes into the Manim mapping table in `lib.rs`.

## Files

- Create:
  - `components/fframes-kit/src/explain/camera.rs` (2D cameras);
  - `components/fframes-kit/src/three/{mod.rs,view.rs,axes.rs,solids.rs,surface.rs,polyhedra.rs}`;
  - `components/gallery/src/scenes/three.rs`.
- Modify:
  - `components/fframes-kit/src/{lib.rs,prelude.rs}` (the `three` module behind `3d`);
  - `components/gallery/Cargo.toml` (the `3d` feature for these scenes);
  - `components/gallery/src/scenes/mod.rs`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/components.md` (cameras and 3D section).

## Tasks & Steps

1. 2D cameras.
2. **The 3D view.**
   - Map `phi`/`theta` to the camera.
   - Add fixed-in-frame overlays.
   - Measure the CPU frame cost of one sphere at 1080p and record it in the phase report.
3. Solids, polyhedra, `Surface`, `ThreeDAxes`.
4. 3D enter animations.
5. Gallery scenes and e2e frames (`MovingCamera@50%`, `ZoomedInset@end`, `ThreeDAxes@end`,
   `Surface@50%`). The 3D frames render on the CPU rasterizer.

## Verification

- `cargo test -p fframes-kit --features 3d`:
  - the `phi`/`theta` mapping puts the camera at the expected position;
  - polyhedra have the right vertex and face counts.
- `cargo test -p fframes-kit` (default features) still builds, with `three` absent.
- `just -f components/justfile e2e cpu`. `check-wasm` with and without `3d` (the editor
  uses the CPU rasterizer).
- `clippy` and `boundary`.

## Risks

- **Cost.** A CPU 3D frame is 60 to 150 ms on 28 threads (AGENTS.md). The e2e frames use
  small scenes, and the gallery documents the cost.
- **Snapshots across machines.** Small rasterizer differences can exceed the e2e threshold.
  If they do, give the 3D frames their own threshold in `components/scripts/e2e.mjs`, and
  record why.

## Todo

- [x] 2D cameras
- [x] 3D view, camera mapping and overlays, measured cost
- [x] Solids, polyhedra, surfaces and 3D axes
- [x] 3D enter animations
- [x] Gallery scenes, e2e frames, docs and mapping rows
