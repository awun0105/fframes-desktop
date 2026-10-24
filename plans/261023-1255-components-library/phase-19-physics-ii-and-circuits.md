---
phase: 19
title: "Phase 19: Physics kit II and circuit symbols"
status: in-progress
priority: P2
effort: 5d
dependencies: [9]
---

# Phase 19: Physics kit II and circuit symbols

## Goal

Cover the rest of manim-physics, and the schematic symbols of manim-circuit, in the physics
kit (feature `physics`).

## Context

- The research report
  [manim-remotion-usage](../reports/research-261024-1748-manim-remotion-usage.md) lists what
  is missing, in the sections "manim-physics" and "manim-circuit".
- **manim-physics:**
  - electrostatics: `Charge`, `ElectricField`;
  - magnetostatics: `Wire`, `MagneticField`;
  - optics: `Lens`, `Ray`;
  - rigid bodies: `Space`, `SpaceScene`;
  - `MultiPendulum`;
  - waves: `LinearWave`, `RadialWave`, `StandingWave`.
- **manim-circuit:**
  - `Resistor`, `Capacitor`, `Inductor`, `VoltageSource`, `CurrentSource`, `Ground`,
    `Opamp`;
  - each symbol has labels and terminals.
- **The existing kit:** `components/fframes-kit/src/kits/physics/`.
  - `field_lines.rs` and `optics.rs` (`RayDiagram`, `Refraction`) have code to reuse.
  - `circuit.rs` is one fixed figure, and stays as it is.
  - `Sim::precompute` (`sim.rs`) is the pattern for anything stepped in time.
- **Rigid bodies.** `rapier2d` was picked in the math and science report (section 4). It is
  pure Rust; check its wasm32 build first. The simulation runs once, in the constructor.

## Requirements

- **Electrostatics:**
  - `Charge` (sign and magnitude, drawn with a glow);
  - `ElectricField`: arrows on a grid, coloured by magnitude, with field lines through the
    existing `FieldLines` code;
  - charges can move over time, and the field follows them.
- **Magnetostatics:** `Wire` (into or out of the page, or a segment), and `MagneticField`
  from the Biot–Savart law on a grid.
- **Optics:**
  - `Lens`: thin or thick, from focal length or from radii and index;
  - `Ray`: traced through a list of lenses, with focal points shown;
  - this extends `RayDiagram` instead of copying it.
- **Rigid bodies:**
  - `RigidWorld`, Manim's `Space`/`SpaceScene`: boxes, circles, polygons, walls, gravity
    and restitution, simulated with `rapier2d` through `Sim`;
  - each body is drawn by a closure, or by a default shape.
- **Pendulums:** `MultiPendulum` (n links, solved with RK4 through `Sim`), with a
  `TracedPath` of the last bob once phase 16 is done.
- **Waves:**
  - `LinearWave` and `StandingWave` (2D curves);
  - `RadialWave` (rings in 2D; a surface with the `3d` feature once phase 18 is done).
- **Circuit symbols:**
  - `Resistor`, `Capacitor`, `Inductor`, `VoltageSource`, `CurrentSource`, `Ground`,
    `Opamp`, plus the kit's own `Battery`, `Switch`, `Bulb` and `Diode`.
  - Every symbol has a label, a rotation and named terminals.
  - A `Schematic` connects terminals with wires routed at right angles. It also has the
    flowing-current dots of the existing `Circuit`.
- Every component is listed in `docs/components/physics.md`. Every manim-physics name has a
  row in the mapping table in `lib.rs`.

## Files

- Create:
  - `components/fframes-kit/src/kits/physics/{electro.rs,magneto.rs,lens.rs,rigid.rs,multi_pendulum.rs,waves_ii.rs,schematic.rs}`;
  - `components/gallery/src/scenes/physics_ii.rs`.
- Modify:
  - `components/fframes-kit/src/kits/physics/{mod.rs,field_lines.rs,optics.rs}`;
  - `components/fframes-kit/Cargo.toml` (`rapier2d` under `physics`);
  - `prelude.rs`, `lib.rs`;
  - `components/gallery/src/scenes/mod.rs`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/physics.md`.

## Tasks & Steps

1. **`rapier2d` 0.36 with `features = ["enhanced-determinism"]`.** Chosen on 2026-10-24,
   and the probe passed:
   - Licence Apache-2.0. It builds for wasm32.
   - 20 bouncing balls over 600 steps give identical positions in two runs. That proves
     same-machine repeatability. `enhanced-determinism` is what rapier offers for results
     that match across machines.
   - The 0.36 API differs from older docs: `step` takes `Vector::new(..)` by value, uses
     `BroadPhaseBvh`, and needs a `SoftBodySet` argument. Pin `rapier2d = "0.36"`.
   - Size: `rapier2d` and `i_overlay` together add 2.07 MB of unstripped wasm. Document
     this; it does not block anything.
2. Electrostatics and magnetostatics.
3. Lenses and rays.
4. Rigid bodies and `MultiPendulum`.
5. Waves.
6. Circuit symbols and `Schematic`.
7. Gallery scenes and e2e frames (`ElectricField@end`, `Lens@end`, `RigidWorld@50%`,
   `Schematic@end`).

## Verification

- `cargo test -p fframes-kit --features physics`:
  - the field of a point charge falls off as 1/r²;
  - the field of a straight wire falls off as 1/r;
  - a thin lens images at the distance of the lens equation;
  - a double pendulum conserves energy within a tolerance over 10 s;
  - rigid-body runs are identical twice.
- `just -f components/justfile e2e cpu`, `check-wasm` (with `physics`), `clippy` and
  `boundary`.

## Risks

- **`rapier2d` size.** It builds for wasm32, but it adds to the editor bundle. If the size
  matters, put `RigidWorld` behind a feature that the editor leaves off.
- **Wire routing.** Keep it simple: right angles with the path given as corners. No
  automatic router.

## Todo

- [x] `rapier2d` checked and recorded
- [x] Electrostatics and magnetostatics
- [x] Lenses and rays
- [ ] Rigid bodies and multi-pendulum
- [ ] Waves
- [ ] Circuit symbols and schematic
- [ ] Gallery scenes, e2e frames, docs and mapping rows
