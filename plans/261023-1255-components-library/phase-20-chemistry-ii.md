---
phase: 20
title: "Phase 20: Chemistry kit II"
status: todo
priority: P3
effort: 3d
dependencies: [9]
---

# Phase 20: Chemistry kit II

## Goal

Cover the rest of manim-chemistry in the chemistry kit (feature `chemistry`):
- molecules from `.mol` and `.sdf` files;
- molecules looked up by name;
- graph molecules with bond and colour animations;
- the larger element card.

## Context

- The research report
  [manim-remotion-usage](../reports/research-261024-1748-manim-remotion-usage.md) lists what
  is missing, in the section "manim-chemistry". It is:
  - `MMoleculeObject`;
  - `NamedMolecule`;
  - `GraphMolecule` with `GMAnimationBuilder`;
  - `MElementObject`;
  - `ThreeDMolecule` from files.
- **The existing kit:** `components/fframes-kit/src/kits/chemistry/`.
  - `molecule.rs`: SMILES through `chematic`, 2D layout.
  - `molecule_3d.rs`: `3d` feature.
  - `periodic.rs`: `mendeleev`.
- `.mol` (V2000) and `.sdf` hold 2D or 3D coordinates and bonds. A small parser is enough;
  check first whether `chematic` or another maintained crate already reads them.

## Requirements

- **Molecule files:**
  - `MolFile`: parses V2000 `.mol` and the first record of `.sdf`.
  - Coordinates from the file are used as they are. Without coordinates, the existing
    layout places the atoms.
  - The molecule is drawn by `Molecule2D`, and by `Molecule3D` with the `3d` feature.
- **Named molecules:** `NamedMolecule`, looked up in a small embedded table of names and
  SMILES.
  - The table holds about 100 common molecules: water, methane, ethanol, benzene, glucose,
    caffeine, aspirin and others.
  - Unknown names render nothing, and `inspect` warns.
  - No network lookups.
- **Graph molecule animations**, manim-chemistry's `GMAnimationBuilder`:
  - rotate a bond's substituent about the bond axis;
  - change the colour of atoms or bonds;
  - highlight a functional group;
  - all as time-based props.
- **Element card:** `ElementCard`, manim-chemistry's `MElementObject`. It shows the atomic
  number, symbol, name, mass, electron configuration and electronegativity from
  `mendeleev`, with a layout prop.
- Every component is listed in `docs/components/chemistry.md`, and every manim-chemistry
  name has a row in the mapping table in `lib.rs`.

## Files

- Create:
  - `components/fframes-kit/src/kits/chemistry/{mol_file.rs,named.rs,graph_anim.rs,element_card.rs}`;
  - `components/fframes-kit/src/kits/chemistry/samples/` (two `.mol` test files, written
    for the tests);
  - `components/gallery/src/scenes/chemistry_ii.rs`.
- Modify:
  - `components/fframes-kit/src/kits/chemistry/{mod.rs,molecule.rs,molecule_3d.rs}`;
  - `prelude.rs`, `lib.rs`;
  - `components/gallery/src/scenes/mod.rs`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/chemistry.md`.

## Tasks & Steps

1. Check `chematic` and crates.io for `.mol` support. Record the choice in the phase report.
2. `MolFile` and its tests.
3. The name table and `NamedMolecule`.
4. Graph molecule animations.
5. `ElementCard`.
6. Gallery scenes and e2e frames (`MolFile@end`, `NamedMolecule@end`, `BondRotation@50%`,
   `ElementCard@end`).

## Verification

- `cargo test -p fframes-kit --features chemistry`:
  - parsing a test file gives the right atom and bond counts;
  - every SMILES in the name table parses;
  - the element card values match `mendeleev`.
- `just -f components/justfile e2e cpu`, `check-wasm` (with `chemistry`), `clippy` and
  `boundary`.

## Risks

- **Licensing of the name table.** Write the table from public-domain facts (names and
  SMILES). Do not copy a database file.

## Todo

- [ ] `.mol` support checked and recorded
- [ ] `MolFile`
- [ ] Name table and `NamedMolecule`
- [ ] Graph molecule animations
- [ ] `ElementCard`
- [ ] Gallery scenes, e2e frames, docs and mapping rows
