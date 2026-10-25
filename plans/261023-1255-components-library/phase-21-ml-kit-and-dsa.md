---
phase: 21
title: "Phase 21: ML kit and data-structure additions"
status: in-progress
priority: P2
effort: 5d
dependencies: [10, 17]
---

# Phase 21: ML kit and data-structure additions

## Goal

- Add a machine-learning kit (feature `ml`) that covers ManimML.
- Fill the gaps between the CS kit and manim-dsa / manim-data-structures.

## Context

- **ManimML.** The research report
  [manim-remotion-usage](../reports/research-261024-1748-manim-remotion-usage.md), section
  "ManimML", lists everything, from the source tree of `helblazer811/ManimML`:
  - `NeuralNetwork`, with the layers feed-forward, convolutional 2D, max pooling 2D, image,
    embedding, vector, math operation, paired query and triplet;
  - activation functions;
  - forward-pass and dropout animations;
  - inserting, removing and replacing layers;
  - `VariationalAutoencoder`;
  - `DecisionTreeDiagram` and its decision surface;
  - MCMC chains on axes;
  - Gaussian covariance ellipses.
- **manim-dsa.**
  - Covered by the CS kit (`components/fframes-kit/src/kits/cs/`): `ArrayView`, `Stack`,
    `Queue`, `LinkedList`, `TreeView`, `GraphView` with traces, `HashTable`,
    `RecursionTree`.
  - Missing: curved and backward edges, weight labels on edges, automatic layouts, array
    `append`/`pop` as animated steps, and a variable box (`MVariable`).
- Phase 17 adds the graph layouts in `layout/graph_layouts.rs`. This phase uses them for
  `GraphView` and `TreeView`, and does not write its own.
- The CS kit's trace pattern (`*Trace`, `*Step`, `*Recorder`, `steps`, `at`) is how the
  animated steps are shown. New steps follow it.
- No ML runtime. Weights, activations, trees and samples are data the user passes, or
  values computed by a small seeded routine in the kit (the MCMC chain).

## Requirements

- **Neural networks** (feature `ml`):
  - `NeuralNetwork { layers, spacing, .. }`.
  - Layers: `FeedForward { nodes, activation }`, `Conv2D { maps, size, filter }`,
    `MaxPool2D { kernel }`, `ImageInput { image }`, `Embedding`, `VectorLayer`,
    `MathOperation`, `PairedQuery`, `Triplet`.
  - Every pair of neighbouring layers gets its own connection drawing, as in ManimML.
  - Animations, all as time-based props:
    - `forward_pass`: pulses travel along the edges, layer by layer, and the convolution
      filters move over the feature maps;
    - `dropout` (seeded);
    - layer insert, remove and replace (a layout change).
  - `ActivationFunction`: `Relu`, `Sigmoid`, `Tanh`, drawn as a small plot on the layer.
- **`VariationalAutoencoder`:** an encoder, a latent Gaussian, and a decoder, with the
  image forward pass.
- **Decision trees:**
  - `DecisionTreeDiagram`, from a tree given as nodes (split feature, threshold, leaf class),
    unfolding level by level or breadth-first;
  - `DecisionSurface`: the partition of a 2D feature space, drawn as rectangles on
    `explain::Axes`.
- **Probability:**
  - `GaussianEllipse`: mean and covariance, with 1σ, 2σ and 3σ rings;
  - `McmcChain`: a seeded Metropolis–Hastings chain on axes, with accepted and rejected
    proposals.
- **Data structures**, added to `kits::cs`:
  - `GraphView`: curved edges, backward edges, weight labels on the edge, and the phase 17
    layouts.
  - `TreeView`: the `tree` layout.
  - `ArrayView`: `append` and `pop` steps.
  - `VariableBox`: manim-dsa's `MVariable` (a name, a box and a value that changes over
    steps).
  - `ArrayView` and `Stack` get labels and index rows as in manim-dsa (`add_indexes`,
    `add_label`).
- Every ML component is listed in a new `docs/components/ml.md`, and the CS additions in
  `docs/components/cs.md`. Every ManimML and manim-dsa name has a row in the mapping table
  in `lib.rs`.

## Files

- Create:
  - `components/fframes-kit/src/kits/ml/{mod.rs,network.rs,layers.rs,connections.rs,forward.rs,vae.rs,decision_tree.rs,probability.rs}`;
  - `components/fframes-kit/src/kits/cs/variable.rs`;
  - `components/gallery/src/scenes/ml.rs`;
  - `docs/components/ml.md`.
- Modify:
  - `components/fframes-kit/Cargo.toml` (feature `ml`);
  - `components/fframes-kit/src/kits/{mod.rs}`;
  - `components/fframes-kit/src/kits/cs/{mod.rs,graph.rs,tree.rs,array.rs,list.rs}`;
  - `prelude.rs`, `lib.rs`;
  - `components/gallery/{Cargo.toml,src/scenes/mod.rs,src/scenes/cs.rs}`;
  - `components/scripts/e2e.mjs`;
  - `docs/components/cs.md`;
  - `docs/components/README.md` (link to `ml.md`).

## Tasks & Steps

1. **Network layout and the feed-forward layers.** Then the forward pass.
2. Convolutional, pooling and image layers with their connections. Then embedding, vector,
   math operation, paired query and triplet.
3. Dropout and layer changes.
4. `VariationalAutoencoder`.
5. Decision trees and the decision surface.
6. Gaussian ellipses and the MCMC chain.
7. The CS additions.
8. Gallery scenes and e2e frames (`ForwardPass@50%`, `ConvNet@50%`, `DecisionTree@end`,
   `McmcChain@end`, `WeightedGraph@end`).

## Verification

- `cargo test -p fframes-kit --features ml,cs`:
  - layer positions with the given spacing;
  - the forward pass reaches the last layer at the end of its duration;
  - dropout is the same for a seed;
  - covariance ellipse axes from the eigenvalues;
  - the MCMC chain is the same for a seed;
  - weight labels sit on the curve midpoint.
- `just -f components/justfile e2e cpu`, `check-wasm` (with `ml`), `clippy` and
  `boundary`.

## Risks

- **Size.** ManimML has many layer pairs. Do the common pairs first (feed-forward,
  convolution, pooling, image). If time runs short, mark the rest in the phase file and
  finish them in the next firing. Do not drop them.

## Todo

- [x] Network layout, feed-forward layers, forward pass
- [x] Convolution, pooling and image layers
- [ ] Embedding, vector, math operation, paired query, triplet layers
- [ ] Dropout and layer changes
- [ ] Variational autoencoder
- [ ] Decision trees and surface
- [ ] Gaussian ellipses and MCMC chain
- [ ] CS additions (edges, weights, layouts, steps, variable box)
- [ ] Gallery scenes, e2e frames, docs and mapping rows
