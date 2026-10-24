# fframes components library: strategy

Goal: a component library that a coding agent can use correctly on the first try, that drops
into any fframes video with one dependency, and that grows from general motion-graphics pieces
into explainer kits for math, physics, computer science, biology and chemistry.

Input: [components overview map](components-261023-0753-overview-map.md) (what the examples
already build and how often it is copied).

> **Partly superseded** by [the Manim research](research-261023-1404-manim-api-for-fframes.md).
> The explainer layer becomes its own crate:
> - it is built on a recorded `Script` (record once, sample per frame), not on the props and
>   `render` contract of section 4;
> - it uses Manim's own names (`Transform`, not `Morph`; `MathTex`/`MathTypst`, not
>   `Formula`);
> - it renders formulas with Typst first instead of comparing three renderers (section 8).
>
> The general components (sections 3, 4 and 6, the gallery, quality gates) still stand. After
> M3 of the Manim plan, the domain kits in section 7 are built from its mobjects.

> **Superseded again** by [the component model](research-261023-1700-component-model-remotion-manim.md)
> and [math, LaTeX and science support](research-261023-1718-math-latex-science-support.md).
> The contract (section 4) and the roadmap (section 10) are replaced. Locations change too: the
> crate is not a root workspace member, the gallery is not under `examples/`, and the skill
> reference is not under `skills/`. Everything lives in a fork-owned `components/` workspace so
> no upstream path is edited (component model report, section 10).

## 1. Assumed defaults (still open, see the end)

- **Separate crate, written so it could go upstream.** Only new files plus one workspace member
  line, so upstream merges into this fork stay clean.
- **Offered through the skill docs first.** `cargo fframes new --components` and the Studio
  project template follow once the general layer is stable.

## 2. Layers

```text
fframes (core)            Svgr, svgr!, timeline!, Frame, text_fit, Styles, Shader, three_d
  └─ fframes-components
       ├─ foundation      motion, geom, space, layout, label, sim, formula, theme
       ├─ general         text, overlays, data, audio, decoration, backgrounds, transitions,
       │                  media, mockups (the map's 25 high-reuse items)
       ├─ explain         Manim-style actions: Create, Write, FadeIn, Transform, Indicate, ...
       └─ kits            math | physics | cs | biology | chemistry   (one feature each)
```

Each layer only uses the layers above it in this diagram. The domain kits are built from the
foundation, so the foundation comes first; otherwise every kit would rebuild axes, arrows and
layouts.

## 3. Crate and integration

- **Crate:** `fframes-components/` at the repo root, a workspace member, depending on `fframes`
  and `kurbo` (already in the tree as an optional dependency of the `lottie` feature).
- **No re-export from `fframes`.** The components depend on `fframes` (`Frame`, `Svgr`), so
  `fframes` cannot depend back on them. A video adds one line:

  ```toml
  fframes-components = { workspace = true, features = ["math", "physics"] }
  ```

- **One import:** `use fframes_components::prelude::*;` brings the general layer, the actions,
  and every enabled kit.
- **Features:**

  | Feature | What it adds |
  | --- | --- |
  | (default) | foundation, general, explain |
  | `math`, `physics`, `cs`, `biology`, `chemistry` | the domain kits |
  | `formula` | equations (see section 8) |
  | `styles` | `from_styles` constructors (turns on `fframes/styles`) |
  | `skia` | shader backgrounds |
  | `3d` | molecules, orbitals, DNA in 3D (turns on `fframes/3d`) |
  | `lottie` | `LottieBox` |

  The default set, and every kit without `3d` or `skia`, must pass `just check-wasm`.
- **Scaffolding (later):** `cargo fframes new --components math,physics` adds the dependency and
  a sample scene. The Studio project template does the same once the general layer is stable.

## 4. The component contract

Every component has the same shape, so an agent that has used one can use all of them.

```rust
use fframes_components::prelude::*;

// In the constructor: build once, store on self. Defaults come from the theme.
let theme = Theme::from_styles(&styles);          // or Theme::default()
let title = LowerThird::new(LowerThirdProps {
    name: "Ada Lovelace",
    role: "Mathematician",
    start: 1.0,                                    // seconds, video or scene time
    ..LowerThirdProps::themed(&theme)
});

// In render_frame (bind `mut frame`): draw it.
fframes::svgr!(<svg ...>{title.render(&mut frame, ctx)}</svg>)
```

The exact signature every component implements:

```rust
pub fn render<'a>(&'a self, frame: &mut Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a>
```

Actions (section 5) are the second shape. They wrap content instead of drawing their own:
`FadeIn { start, duration, shift }.apply(&frame, content) -> Svgr`. `Create` and `Morph` take a
`Shape` instead of any `Svgr`, so misuse does not compile.

Rules for every component:

1. **`Props` struct with `Default` and `themed(&Theme)`.** Only the content fields are
   required. Field names are shared across the library:
   - `start`, `duration`, `exit` for timing (`start` matches `LottieOptions::start` in core);
   - `x`, `y`, `width`, `height`, `anchor` for placement;
   - `color`, `accent`, `font` for style.
2. **One `render` signature (above).** It takes `&mut Frame` everywhere because the text helpers
   (`frame.text_width` and friends) need it, so agents never have to guess.
3. **Time-pure.** Output depends only on the props and the frame time. Animation is closed-form
   (motion helpers) or a `timeline!` built in `new`. Nothing is allocated per frame beyond the
   `Svgr`.
4. **Heavy work in `new`.** Simulations, layouts and formula paths are computed in the
   constructor (or a `OnceLock` once fonts resolve) and sampled by time. `render_frame` runs on
   several threads.
5. **Static geometry stays static.** Decorative markup has no `{}`, so `svgr!` caches it.
   Animated values go on a wrapping `<g transform opacity>`.
6. **No panics.** When a font, image or file is missing, the component draws a sensible fallback
   (raw text, an empty group) and leaves the reporting to `inspect`.
7. **Studio keys, not invented ids.** Core already has `fframes::EditorObjectKey`
   (`fframes/src/editor.rs:19`): scene instance, component, object and repeat keys, a source
   anchor and the style tokens the object reads. The Studio template builds one by hand
   (`desktop/crates/studio-project/templates/src/lib.rs:35`). Components use it as follows:
   - each component takes an optional `key` prop (scene key and object key);
   - it fills `component_key` with its own kebab-case name (`lower-third`);
   - it lists the theme tokens it reads in `style_tokens`;
   - it attaches the result with `Svgr::with_editor_object`.

   Studio then knows which component an object is and which preset tokens change it. Keys are
   stable because they come from props, not from a counter: inserting a component earlier in
   the video does not renumber the ones after it.
8. **Seeded randomness.** Randomness comes from `motion::hash` and `motion::noise` with a
   `seed` prop, never from `rand`. Renders are reproducible.

## 5. Making it easy for agents

Agents already have strong priors from popular libraries, so the names follow them. This is what
`fframes_three_d` does with three.js: its crate docs map three.js classes to fframes names.

1. **Manim vocabulary for the explainer layer.**

   | Manim | fframes-components | Works on |
   | --- | --- | --- |
   | `Create` | `Create` (stroke draw-on) | `Shape` |
   | `Write` | `Write` (glyph stagger) | `Text`, `Formula` |
   | `FadeIn` / `FadeOut` (with `shift`) | `FadeIn` / `FadeOut` | any `Svgr` |
   | `GrowFromCenter`, `GrowArrow` | `GrowFromCenter`, `GrowArrow` | any `Svgr`, `Arrow` |
   | `Transform`, `ReplacementTransform` | `Morph` | `Shape` to `Shape` (kurbo resampling) |
   | `Indicate`, `Circumscribe`, `Flash` | `Indicate`, `Circumscribe`, `Flash` | any `Svgr` |
   | `MoveAlongPath` | `MoveAlongPath` | any `Svgr` on a `Shape` |
   | `LaggedStart`, `Succession` | `lagged(step)`, `succession()` | lists of actions |
   | `Axes`, `NumberPlane`, `NumberLine` | same names | `space` |
   | `MathTex` | `Formula` / `formula!` | `formula` |
   | `Arrow`, `Dot`, `Line`, `Brace` | same names | `geom` |

   The crate docs carry this table, like `fframes-three-d/src/lib.rs` does for three.js.

2. **The gallery is the catalog.** `examples/components-gallery` has one scene per component,
   named after it. An agent sees a component without reading code:
   - `cargo run -p components-gallery -- frame LowerThird@50%`
   - `... strip Pendulum -n 8`
   - `... timeline` lists every scene, so it is the index.
3. **Skill references per domain.** Add `skills/fframes-video/references/components.md`
   (general layer and actions), plus `math.md`, `physics.md`, `cs.md`, `biology.md` and
   `chemistry.md`. Each entry has one sentence, one copy-paste snippet and the gallery scene
   name. `SKILL.md` gets one line per file, so an agent loads only the domain it needs. The
   prose recipes in `design.md` (lower third, counter, draw-on, transitions) point to the
   components.
4. **The skill docs are the tests.** The crate compiles each reference file as doctests, so
   there is one copy and `cargo test -p fframes-components` fails when an API changes:

   ```rust
   #[cfg(doctest)]
   #[doc = include_str!("../../skills/fframes-video/references/components.md")]
   struct ComponentsDocs;
   ```

   `cfg(doctest)` keeps the path outside the package from breaking `cargo package`. Snippets
   that need fonts or media are marked `no_run`.
5. **Errors that teach.** Doc comments state units (seconds, pixels, data units), the anchor
   origin and what happens before `start` and after `exit`. A misuse that can be caught by type
   (for example `Morph` between non-`Shape` values) does not compile.
6. **Studio (later).** The agent context in Studio includes the index of `components.md`, so the
   agent knows what exists before it writes code.

## 6. Foundation

Every kit in section 7 is built from these.

| Module | Contents | Source to absorb |
| --- | --- | --- |
| `motion` | `progress`, `clamp01`, `lerp`, easing curves, closed-form `spring`, named springs from `motion.easing.*`, `stagger`, beat grid, `hash`, `noise` | 6 examples, see map section 1 |
| `geom` | `Shape` (kurbo `BezPath`): `Line`, `Arrow`, `Dot`, `Circle`, `Arc`, `Polygon`, `Brace`; path length (draw-on), resampling (morph), arrowheads, dashes, `scale_about` | `examples/arm`, draw-on uses |
| `space` | `Space` (data units to pixels), `Axes`, `NumberPlane`, `NumberLine`, ticks, `plot(f)`, `parametric(f)` | new |
| `layout` | `row`, `column`, `grid`, `Anchor` with a safe area, tidy tree layout, layered and circular graph layout (seeded, computed in `new`) | new |
| `label` | `Label`, `Leader` (label with a line to a point), `Callout`, simple overlap avoidance | `examples/fframes-intro/src/ui.rs` |
| `sim` | `Sim::precompute(dt, steps, step_fn)` into a table, `sample(t)` with interpolation | new |
| `formula` | `Formula::new(tex)`, `formula!(tex)` macro, one group per glyph so `Write` can stagger | spike, section 8 |
| `theme` | `Theme { colors, type, spacing, radius, stroke, shadow, motion }`, `Theme::from_styles` (the 40 preset tokens) | `fframes::Styles` |

## 7. Domain kits

For each figure: which foundation pieces it uses, and what moves. A figure is one component
with props; for example `Pendulum { length, angle0, damping, start, .. }`.

### Math (`math`)

| Figure | Builds on | What animates |
| --- | --- | --- |
| `NumberLine` with point and interval | space, geom | point slides, interval grows |
| `FunctionPlot` | space, geom, formula | curve draws on, tracer dot, label follows |
| `Tangent` / `Secant` | space, geom | secant slides into the tangent |
| `RiemannSum` / `AreaUnder` | space | rectangles refine as `n` grows, area fills |
| `UnitCircle` | space, geom, formula | angle sweeps, sin and cos projections draw |
| `LinearTransform` | space (`NumberPlane`) | grid warps by a 2x2 matrix, basis vectors follow |
| `VectorSum` | geom | head-to-tail placement, resultant grows |
| `Pythagoras` | geom, layout | squares on the sides, pieces rearrange (`Morph`) |
| `FractionBar` / `FractionPie` | geom | parts fill, equal fractions align |
| `MatrixView` | formula, layout | rows and columns highlight in a product |
| `Histogram` to `NormalCurve` | space, motion (seeded) | samples stack up, curve fits |
| `Epicycles` (Fourier) | geom, sim | rotating circles trace a shape |

### Physics (`physics`)

| Figure | Builds on | What animates |
| --- | --- | --- |
| `Projectile` | sim, space, geom | ball with velocity vectors, trace, peak marker |
| `Pendulum` with `EnergyBars` | sim | swing, kinetic and potential energy exchange |
| `SpringMass` with x(t) plot | sim, space | oscillation synced with its graph |
| `Wave` (transverse, longitudinal, standing, superposition) | space, motion | waves travel, add, form nodes |
| `FreeBody` | geom, label | force arrows grow, net force appears |
| `Orbit` (Kepler) | sim, geom | ellipse orbit, equal-area sweeps |
| `FieldLines` (point charges) | geom, sim | lines draw on, test charge follows |
| `Circuit` (battery, resistor, bulb, switch, wire) | geom, layout | current dots flow, switch toggles, bulb glows |
| `RayDiagram` (lens, mirror, refraction) | geom, label | rays draw, image forms |
| `Collision1D` | sim | carts collide, momentum bars balance |
| `Doppler` | geom, motion | wavefronts compress ahead of the source |
| `GasBox` | sim (seeded) | particles bounce, temperature changes speed |

### Computer science (`cs`)

| Figure | Builds on | What animates |
| --- | --- | --- |
| `ArrayView` with pointers | layout, label | cells highlight, pointers move, swaps |
| `SortVisualizer` (bubble, insertion, merge, quick) | layout, motion; steps computed in `new` | bars compare and swap |
| `BinarySearch` | layout | range narrows, middle index marked |
| `LinkedList` | layout, geom | node insert and delete, arrows re-route |
| `Stack` / `Queue` | layout | push, pop, enqueue, dequeue |
| `TreeView` (BST, heap) | layout (tidy tree) | insert path, traversal order highlight |
| `GraphView` (BFS, DFS, Dijkstra) | layout (graph), label | frontier spreads, distances update |
| `HashTable` | layout | key hashes to a bucket, collision chain |
| `StateMachine` | layout, geom | input consumed, active state moves |
| `CallStack` / `RecursionTree` | layout | frames push and pop, tree expands |
| `ComplexityPlot` (O(n), O(n log n), O(n^2)) | space, formula | curves race |
| `CodeBlock` with line highlight | text | lines highlight in sync with another figure |
| `SequenceDiagram` (client, server, packets) | layout, geom | messages travel between lifelines |

`CodeBlock` needs syntax coloring. Start with a small keyword tokenizer and decide on a real
highlighter later.

### Biology (`biology`)

| Figure | Builds on | What animates |
| --- | --- | --- |
| `Cell` (animal, plant) with labeled organelles | geom, label | organelles appear, leaders draw |
| `DnaHelix` (2D, 3D with `3d`) | geom, motion | strands twist, base pairs light up, unzip for replication |
| `Transcription` / `Translation` | layout, geom | ribosome moves along mRNA, chain grows |
| `MitosisStages` | geom, `Morph` | chromosomes condense, align, separate |
| `Neuron` with `ActionPotential` | geom, space | pulse travels along the axon, voltage plot synced |
| `PopulationCurve` (exponential, logistic, predator-prey) | sim, space | curves grow, carrying capacity line |
| `PhyloTree` | layout (tidy tree) | branches grow from the root |
| `FoodWeb` | layout (graph) | energy flows along edges |
| `Circulation` | geom (`MoveAlongPath`) | blood cells travel the loop |
| `EnzymeFit` | geom, `Morph` | substrate docks, product releases |
| `PunnettSquare` | layout, label | alleles combine, ratios count up |

### Chemistry (`chemistry`)

| Figure | Builds on | What animates |
| --- | --- | --- |
| `BohrAtom` | geom, motion | shells fill, electrons orbit |
| `ElementTile` and `PeriodicTable` | layout, theme | groups highlight, trends sweep |
| `Molecule2D` (skeletal, from atom and bond lists) | geom, label | bonds draw on, atoms label |
| `Molecule3D` (ball and stick, XYZ input) | `3d` | molecule turns, bonds highlight |
| `Orbital` (s, p, d) | `3d` | lobes fade in, axes rotate |
| `ReactionEquation` with balancing | formula | coefficients change, sides balance |
| `BondFormation` (ionic, covalent) | geom, `MoveAlongPath` | electrons transfer or share |
| `EnergyProfile` | space, label | path draws, activation energy and catalyst path |
| `TitrationCurve` / `PhScale` | space | curve draws, equivalence point marked |
| `StatesOfMatter` | sim (seeded) | particles order and disorder with temperature |
| `Equilibrium` bars | motion | concentrations shift and settle |

`Molecule2D` takes coordinates. Generating them from SMILES needs a layout algorithm and is out
of scope until a video needs it.

## 8. Formula rendering spike

Math, physics and chemistry all need equations, so this decides whether those three kits work.

**What we know.** `Svgr` has no public way to embed a raw SVG string (`fframes/src/svgr.rs`):
with `compile-time-svgtree` it holds prebuilt nodes. A formula must therefore become
`<path d={..}>` elements inside `svgr!`, ideally one `<g>` per glyph so `Write` can stagger
them.

**Candidates (pure Rust):**
- **RaTeX:** KaTeX-compatible, split into parser, layout and SVG crates; used by the konoma
  file browser.
- **latex-rust:** one crate, a TeX-style box model, self-contained SVG output, MIT/Apache-2.0.
- **Typst** (`typst` + `typst-svg`): full typesetting, a much heavier dependency.

**The spike.** For each candidate:
1. Render `e^{i\pi} + 1 = 0`, a fraction, a matrix and `\ce`-style chemistry (`H_2O`,
   `\rightleftharpoons`).
2. Extract glyph paths with positions.
3. Check that it builds offline on Windows and for `wasm32`.
4. Measure compile time and binary size.

**Output.** Recommend one renderer, and decide between:
- a compile-time `formula!` macro: static paths, cached by `svgr!`, nothing at runtime;
- a runtime `Formula::new`: formulas built from data;
- both, with the macro calling the same code.

## 9. Quality gates

- `cargo test -p fframes-components`: unit tests for the math helpers, plus doctests for every
  snippet in the skill references.
- `components-gallery` key frames in `e2e/snapshots`, rendered with `just e2e cpu`. The
  qualification machine has no GPU, so Skia-only components are covered by a
  `compile_shader` test instead.
- `just check-wasm components-gallery` for the default set and the 2D kits.
- `just clippy` and `cargo fmt --all`.
- `inspect` on the gallery reports no errors. `inspect --pops` reports no unintended pops.
- Cost budget: `just bench components-gallery` stays near the cost of the same markup written
  by hand. A component that allocates per frame or breaks static caching fails review.

## 10. Roadmap

| Phase | Scope | Done when |
| --- | --- | --- |
| 0 | Crate, prelude, `Theme`, the contract (with editor keys), `motion`, `geom`, the gallery with its `editor/` bridge (which `just check-wasm` needs), skill `components.md` stub | gallery renders, doctests pass, wasm check passes |
| 1 | General layer, tier 1 from the map (FitText, TextReveal, Typewriter, Counter, Appear, LowerThird, Captions, SpectrumBars, DrawOn, CornerMarks, CrossFade, Push, SceneFade, ProgressBar) | each has a gallery scene and a snapshot |
| 2 | `explain` actions, `space`, `layout`, `label`, `sim`; formula spike | Manim table implemented, spike recommendation written |
| 3 | `math` and `physics` kits | all figures in section 7 for both, plus `math.md` and `physics.md` |
| 4 | `cs` kit | as above |
| 5 | `chemistry` and `biology` kits (3D parts behind `3d`) | as above |
| 6 | Migrate `fframes-intro` and `fframes-studio-intro` to the crate; `cargo fframes new --components`; Studio template and agent context | the duplicated `beat.rs` and `ui.rs` are deleted, renders match snapshots |
| 7 | General layer tier 2 and 3 from the map | as needed by videos |

Phases 3 to 5 depend on phase 2. Phases 3, 4 and 5 can run in parallel because each kit owns
its own module.

## 11. Risks

- **Too much scope.** About 75 domain figures are listed. Build each kit's figures in the order
  of the tables and stop when a kit is useful. Phase 1 and the foundation carry most of the
  value.
- **The API sets in early.** Agents copy whatever the docs show. Freeze the props names
  (section 4) in phase 0 and keep the crate at `0.x` until two real videos use it.
- **Per-frame cost.** Kits with simulations or layouts must precompute. Review every component
  against rule 4 and the cost budget.
- **Formula dependency weight.** If no candidate builds for `wasm32`, `formula` becomes a
  native-only feature and the editor shows the plain TeX text.

## Unresolved questions

1. Fork-only, or offered upstream? (Default: written so it could go upstream.)
2. Studio integration timing? (Default: skill docs first, templates in phase 6.)
3. Which domain kit first after the foundation? (Default: math and physics, since they share
   `space`, `sim` and `formula`.)
4. Should the explainer layer copy Manim's API names exactly, or only where they read naturally
   in Rust? (Default: exactly, for agent recall.)

Sources for section 8: [latex-rust](https://docs.rs/crate/latex-rust/1.0.2),
[RaTeX via konoma](https://docs.rs/crate/konoma/0.23.1/source/src/preview/math.rs),
[Typst math to SVG (forum)](https://forum.typst.app/t/how-to-pre-render-maths-to-images/6441).
