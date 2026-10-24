---
phase: 9
title: "Phase 9: Physics and chemistry kits"
status: in-progress
priority: P2
effort: 6d
dependencies: [6]
---

# Phase 9: Physics and chemistry kits

## Goal

Ship the `physics` and `chemistry` features: the figures from the strategy's tables, built on
`sim`, `space`, `geom`, `label` and the LaTeX formulas, each with a gallery scene and a skill reference.

## Context

- Figure tables: strategy section 7 (physics: 12, chemistry: 11).
- Crates: math report section 4 (its Typst packages are dropped). Physics: `rapier2d` 0.36
  with `enhanced-determinism` (not `simd`), `nalgebra`; notation through the LaTeX `physics`
  and `siunitx` packages (`TexTemplate::physics()`, phase 5). Chemistry: `chematic` 1.2.0
  (SMILES to 2D), `mendeleev` 0.8.4, `ChemEq` with mhchem (phase 5); static skeletal formulas
  through `chemfig` with `Tex` and `TexTemplate::chemistry()`, animated ones drawn by
  `Molecule` from `chematic` coordinates.
- 3D parts (`Molecule3D`, `Orbital`) go behind `3d` (turns on `fframes/3d`); render them on CPU
  here (`Renderer::cpu()` path).
- Runs in parallel with phases 10 and 11; owns `components/fframes-kit/src/kits/{physics,chemistry}/`,
  `components/gallery/src/scenes/{physics,chemistry}.rs` and the two skill files.

## Requirements

- Physics, in table order: `Projectile`, `Pendulum` with `EnergyBars`, `SpringMass` with x(t)
  plot, `Wave` (transverse, longitudinal, standing, superposition), `FreeBody`, `Orbit`
  (Kepler), `FieldLines`, `Circuit` (animated current), `RayDiagram`, `Collision1D`, `Doppler`,
  `GasBox`.
- Chemistry, in table order: `BohrAtom`, `ElementTile` and `PeriodicTable`, `Molecule2D` /
  `Molecule` (from SMILES), `Molecule3D`, `Orbital`, `ReactionEquation` (on `ChemEq`),
  `BondFormation`, `EnergyProfile`, `TitrationCurve` / `PhScale`, `StatesOfMatter`,
  `Equilibrium`.
- Simulations precompute in the component's build step through `Sim::precompute` (or
  `rapier2d` stepped once into a table), cached per video with `OnceLock`, never per frame.
  `rapier2d` is used only where a closed form does not exist (`Collision1D` stays closed form;
  `GasBox` may use it).
- Every figure: themed defaults, plain-data props, seeded randomness.
- All 2D figures build for wasm32 (`just -f components/justfile check-wasm` with `physics,chemistry`).

## Files

- Create: `components/fframes-kit/src/kits/physics/*.rs`, `components/fframes-kit/src/kits/chemistry/*.rs`,
  `components/gallery/src/scenes/{physics.rs,chemistry.rs}`,
  `docs/components/{physics.md,chemistry.md}` (doctested).
- Modify: `components/fframes-kit/Cargo.toml` (features and optional deps), `prelude.rs`,
  `docs/components/README.md` (two lines), gallery features, `components/scripts/e2e.mjs` (one frame
  for a subset of figures).

## Tasks & Steps

1. First, check whether `chematic` exposes 2D atom coordinates (open question 2 of the math
   report). If not, parse its SVG once at build time with usvgr into atoms and bonds, and note
   it in the phase report.
2. Check `rapier2d` with `enhanced-determinism` for wasm32 before writing figures on it.
3. Physics figures in order; each with a gallery scene and a `physics.md` entry.
4. Chemistry figures in order; each with a gallery scene and a `chemistry.md` entry.
5. e2e subset: `Pendulum@50%,Projectile@end,Circuit@50%,BohrAtom@50%,Molecule@end,EnergyProfile@end`.

## Verification

- `cargo test -p fframes-kit --features physics,chemistry,latex` (unit tests of each
  simulation's invariants, for example pendulum energy conserved within tolerance without
  damping, plus doctests).
- `strip <Figure> -n 8` for each figure; `inspect --pops` clean.
- `just -f components/justfile e2e cpu`, `just -f components/justfile check-wasm` with the 2D features, `just -f components/justfile clippy`.

## Risks

- **Determinism.** Every simulation must give the same frames on every run and platform; test
  by rendering a frame twice and comparing bytes.
- **Dependency weight.** Keep `rapier2d` optional within `physics` (sub-feature
  `physics-rapier`) if it adds a large build cost.

## Todo

- [x] `chematic` coordinates and `rapier2d` wasm32 checks
- [ ] Physics figures in table order
- [ ] Chemistry figures in table order
- [ ] `physics.md` and `chemistry.md` doctested
- [ ] Determinism tests; e2e subset
