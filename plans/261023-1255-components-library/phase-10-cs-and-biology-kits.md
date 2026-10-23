---
phase: 10
title: "Phase 10: CS and biology kits"
status: todo
priority: P2
effort: 6d
dependencies: [6]
---

# Phase 10: CS and biology kits

## Goal

Ship the `cs` and `biology` features: the figures from the strategy's tables, each with a
gallery scene and a skill reference.

## Context

- Figure tables: strategy section 7 (CS: 13, biology: 11).
- Crates: math report section 4. CS: `rust-sugiyama` 0.4 (layered DAG layout, deterministic),
  `petgraph` (algorithms). Biology: small library types, no `bio` dependency.
- `CodeBlock` colours code with a small keyword tokenizer (no Typst). Static diagrams (TikZ,
  for example) can be drawn with `Tex` from phase 5 when `latex` is on; the
  animated `GraphView` and `StateMachine` do not need it.
- Runs in parallel with phases 9 and 11; owns `components/fframes-kit/src/kits/{cs,biology}/`, their
  gallery scene files and skill files. Tidy tree layout goes into `components/fframes-kit/src/layout.rs`
  only if phase 6 did not add it; coordinate with phase 9 if both edit `layout.rs`.

## Requirements

- CS, in table order: `ArrayView` with pointers, `SortVisualizer` (bubble, insertion, merge,
  quick; steps computed once), `BinarySearch`, `LinkedList`, `Stack`/`Queue`, `TreeView` (BST,
  heap), `GraphView` (BFS, DFS, Dijkstra), `HashTable`, `StateMachine`, `CallStack` /
  `RecursionTree`, `ComplexityPlot`, `CodeBlock` with line highlight, `SequenceDiagram`.
- Biology, in table order: `Cell` (animal, plant) with labelled organelles, `DnaHelix` (2D; 3D
  behind `3d`), `Transcription`/`Translation`, `MitosisStages`, `Neuron` with
  `ActionPotential`, `PopulationCurve`, `PhyloTree`, `FoodWeb`, `Circulation`, `EnzymeFit`,
  `PunnettSquare`.
- Algorithm steps are recorded once (a `Vec<Step>` built in the component's build step) and
  sampled by time; nothing re-runs per frame.
- Every figure: themed defaults, plain-data props, seeded randomness.

## Files

- Create: `components/fframes-kit/src/kits/cs/*.rs`, `components/fframes-kit/src/kits/biology/*.rs`,
  `components/gallery/src/scenes/{cs.rs,biology.rs}`,
  `docs/components/{cs.md,biology.md}` (doctested).
- Modify: `components/fframes-kit/Cargo.toml`, `prelude.rs`, `components/fframes-kit/src/layout.rs` (tidy tree and
  layered layout if missing), `docs/components/README.md` (two lines), gallery features, `components/scripts/e2e.mjs`.

## Tasks & Steps

1. Layout: tidy tree (for `TreeView`, `RecursionTree`, `PhyloTree`) and layered layout through
   `rust-sugiyama` (for `GraphView`, `StateMachine`, `FoodWeb`), both computed once; unit tests
   for determinism.
2. CS figures in order, each with a gallery scene and a `cs.md` entry. Test sort step
   recorders against `slice::sort` results.
3. Biology figures in order, each with a gallery scene and a `biology.md` entry.
4. e2e subset: `SortVisualizer@50%,GraphView@end,CodeBlock@50%,DnaHelix@50%,PunnettSquare@end,PhyloTree@end`.

## Verification

- `cargo test -p fframes-kit --features cs,biology`.
- `strip <Figure> -n 8` per figure; `inspect --pops` clean.
- `just -f components/justfile e2e cpu`, `just -f components/justfile check-wasm` with these features, `just -f components/justfile clippy`.

## Risks

- **`rust-sugiyama` on wasm32 or its output stability.** If it fails either, implement a small
  longest-path layering with barycentric ordering in `layout.rs`.
- **Text-heavy figures.** `CodeBlock` and `SequenceDiagram` must truncate or wrap long lines
  with `text_fit`/`text_break_lines`; `inspect` must not report clipped text.

## Todo

- [ ] Tidy tree and layered layout
- [ ] CS figures in table order
- [ ] Biology figures in table order
- [ ] `cs.md` and `biology.md` doctested
- [ ] e2e subset
