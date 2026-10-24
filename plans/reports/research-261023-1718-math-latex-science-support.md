# Research: first-class math, LaTeX and science support for the fframes component library

Date: 2026-10-23. Scope: how the component library (model in
`research-261023-1700-component-model-remotion-manim.md`) gets strong support for math
formulas written in LaTeX or Typst, for chemistry, physics and biology figures, and for the
animations Manim users expect on them (`Write`, `TransformMatchingTex`, `Indicate` on a part).
Constraints: pure Rust, offline, builds on Windows and for the wasm32 editor bridge, pure
`render_frame`.

Facts were checked on 2026-10-23 against the sources at the end. Items marked *unconfirmed* or
*to measure* are settled by the spike in section 7.

## Summary

- **One formula engine: Typst, in process.** The `typst` crate (0.15.1, Apache-2.0) compiles a
  snippet to a laid-out `Frame`. The library walks the frame instead of parsing SVG: each
  `TextItem` gives glyph ids and positions, `FontInstance::ttf()` gives the `ttf_parser::Face`,
  so every glyph becomes one `<path d>` element in `svgr!`. Fraction bars and radicals arrive as
  `Shape` items. No SVG string has to enter `Svgr`: the native `compile-time-svgtree` build has
  no route for one, and the same glyph route then works in both builds.
- **LaTeX stays the input agents write.** `MathTex { tex: r"\frac{a}{b}" }` converts LaTeX to
  Typst math with MiTeX (`mitex` 0.2.4, Apache-2.0, `convert_math`), the way Manim's new
  `MathTypst` and the Typst `mitex` package do. Typst syntax is accepted directly in
  `MathTypst` and `Typst`. This is the same split Manim Community uses on its main branch
  (`Typst`, `MathTypst` in `manim/mobject/text/typst_mobject.py`).
- **Parts of a formula are addressable**, so `TransformMatchingTex`, `Indicate` and per-part
  colours work. Manim's `{{ }}` shorthand marks a part. Two ways to find its glyphs: labelled
  elements produce `Tag` items in the frame (the preferred route, syntax to be fixed by the
  spike), or Manim's own method, one extra compile per part with that part hidden and a diff of
  the glyph lists (proven, costs n+1 compiles at build time). The spike decides.
- **Science is two layers.** Data and simulation come from pure-Rust crates that build for
  wasm32 (`chematic` for SMILES to 2D structure, `mendeleev` for element data, `rapier2d` with
  `enhanced-determinism` for physics, `rust-sugiyama` for layered graph layout, `bio` for
  sequences). Notation and typeset diagrams come from vendored Typst packages compiled by the
  same engine (`whalogen` for `\ce` chemical equations, `alchemist` or `typsium` for skeletal
  formulas, `physica` for physics notation; `fletcher` and `circuiteria` for static diagrams
  are optional because they depend on CeTZ, whose LGPL licence is an open question).
  Animated figures (axes, graphs, orbits, pendulums, circuits that light up) are library
  components drawn with `kurbo` paths, because Typst output is static.
- **No upstream code changes.** Everything here lives in the library's `components/`
  workspace (component model report, section 10). The formula pipeline uses only upstream
  APIs (`MediaProvider::get_all_font_data`, the `ttf_parser` re-export, `svgr!` with `{expr}`
  path data, `ctx.get_image` for the editor fallback); `typst`, `mitex`, `kurbo` and the
  science crates are dependencies of the library, not of fframes.
- **Milestones move.** The formula module becomes part of C1 (right after the core), and the
  math slice (C4 in the component model report) splits into "formulas and axes" and "science
  kits". Section 8 gives the new order.

## 1. Choosing the formula engine

| Candidate | What it is | Status (checked 2026-10-23) | Verdict |
| --- | --- | --- | --- |
| **Typst** (`typst`, `typst-library`, `typst-svg`, `typst-kit` 0.15.1, 2026-07-17, Apache-2.0) | full typesetter with a math mode; `World` trait with `library`, `book`, `main`, `source`, `file`, `font`, `today`; `typst-kit` feature `embedded-fonts` bundles `typst-assets` | maintained; compiles for wasm32 (typst.ts and the `typst-pack` crate build it without features; *size to measure*) | **use**: one engine for math, text, chemistry, diagrams; gives frames with glyph ids, fonts, shapes and labels |
| **MiTeX** (`mitex` 0.2.4, 2024-06-17, Apache-2.0) | LaTeX to Typst converter; `convert_math`, `convert_text`; "correctly converted 32.5k equations from OI Wiki" | package switching (amsmath, mhchem) "still unimplemented"; last release 2024 | **use** for `MathTex`; `\ce` is routed to `whalogen` by the library, not to MiTeX |
| ReX (grafeia/ReX fork, MIT/Apache-2.0) | pure Rust TeX math typesetter to SVG | "heavy development"; original repo gone; font and wasm status *unconfirmed* | skip: unmaintained, math only |
| RaTeX (erweixin/RaTeX) | KaTeX-compatible pure Rust, SVG and wasm targets, self-reported 99% coverage | maintenance *unconfirmed*; no chemistry, no diagrams | fallback only if Typst is too heavy for wasm32 |
| latex-rust 1.0.2 | pure Rust LaTeX math to SVG/PNG | maintenance *unconfirmed*; math only | skip |
| tectonic | full TeX engine | needs TeX bundles; 2.7 MiB native package plus bundles | skip: not offline, not wasm |
| KaTeX (`katex-rs`, `katex`) | HTML/MathML output or a JS backend | no SVG, or needs JS | skip |

Typst wins because the science layer needs more than math: chemical equations, skeletal
formulas, circuit and arrow diagrams all exist as Typst packages, and the same engine typesets
them with the same fonts. The cost is a large dependency; the spike measures it.

## 2. From a formula to `svgr!` paths

What fframes offers today (`fframes/src`): text is measured from `ttf-parser` advances only
(`text.rs:339`, `renderer_font_source.rs:27`); there is no 2D glyph outline API, and `.svg` is
not an image format (`media-dir-macro/src/lib.rs:287-289`). An SVG string can be rendered only
in the wasm32 or no-`compile-time-svgtree` build, where `From<String>` passes raw markup through
(`svgr.rs:150-157`); in the native build it makes a text node (`svgr.rs:161`). So a string
route is not portable, and the glyph route below is used in both builds. `ttf_parser` and
`usvgr` are re-exported (`lib.rs:204-205`), `kurbo` is an optional dependency, and
`fframes-three-d` already has an outline collector over `ttf_parser::OutlineBuilder`
(`geometry.rs:707`, `text_shapes`) to copy from. Font bytes come from
`MediaProvider::get_all_font_data` (`media_provider.rs:43`, implemented by the media macro at
`media-dir-macro/src/lib.rs:251`). A dynamic `<path d={string}>` is parsed and rasterised by
usvgr on every frame on the CPU backend; the Skia backend caches the path conversion by a
fingerprint of the string. `fframes/src/lottie.rs:349` already emits path strings per frame
this way.

The pipeline, run once per formula at build time (constructor or `OnceLock`), never per frame:

1. **World.** A `World` implementation with: the standard library; a `FontBook` built from
   the `typst-assets` crate's embedded fonts (31 files: New Computer Modern and NewCM Math,
   Libertinus Serif, DejaVu Sans Mono, Foxit Type 1; the library embeds NewCM and NewCM Math by
   default and the rest behind a feature) plus the video's own fonts from
   `MediaProvider::get_all_font_data`, so a formula can use the video's text font; `file`
   serving vendored package files from memory; `today` fixed. No file system, no network, so it
   behaves the same on wasm32. (`typst-kit`'s package downloader and local package directory
   are CLI behaviour and are not used.)
2. **Source.** `MathTex` wraps the converted math in `$ .. $` with a `#set text(size: ..)` and
   `#set page(width: auto, height: auto, margin: 0pt)`; `Typst` passes markup through.
3. **Compile** to a `PagedDocument`, take the first page's `Frame`.
4. **Walk** `Frame::items()` (`(Point, FrameItem)` pairs): `Group` recurses with its transform;
   `Text(TextItem { font, size, fill, glyphs })` turns each `Glyph { id, x_advance, x_offset,
   y_advance, y_offset }` into an outline through `font.ttf().outline_glyph(GlyphId(id), ..)`
   scaled by `size / units_per_em` (for a variable font the `FontInstance` variation
   coordinates must be applied to the face first; NewCM is not variable, a video's own font
   may be); `Shape` becomes a path with its fill and stroke; `Tag::Start(content, ..)` and
   `Tag::End(location, ..)` open and close a range, and the label is read from
   `content.label()`; `Image` and `Link` are ignored.
5. **Emit** a `Formula` value: a list of `Glyph { path: kurbo::BezPath, d: String, bbox,
   parts: Vec<PartId> }` in drawing order, plus `width`, `height`, `baseline`. The component
   renders `<g transform=..><path d={glyph.d} fill=.. /> ..</g>`, one `<path>` per glyph, with
   the `d` strings built once. Nothing is re-typeset per frame. The CPU backend still parses
   and rasterises those strings every frame (about 12k path segments for 20 formulas of 15
   glyphs, well under a millisecond to parse and far less than rasterising them); if the spike
   shows parse cost, the native build can pass usvgr `PathData` segments instead of a string,
   since `svgr!` emits `SvgAttributeValue::from(expr)` for an expression attribute and usvgr
   converts from `Vec<PathSegment>`.

Why frames and not `typst-svg`: the SVG output would have to be parsed back (usvgr can parse it,
but the native `Svgr` cannot take a parsed tree), and glyph identity would come from heuristics
on `<use>` elements. The frame has the glyph ids, the font and the tags as data. Tags are
emitted for locatable elements and for any element that carries a label (typst-realize on the
main branch; the 0.15.1 tag is to be checked in the spike), which is the one fact that makes
the label route possible.

## 3. The formula components

Formulas are typeset once, in the video's constructor, and borrowed by the component as a
prop; the struct form below works from C1, the tag form arrives with C2.

```rust
// constructor: LaTeX in, Typst underneath. Parts in {{ }} are selectable, like Manim.
let euler = MathTex::typeset(&fonts, r"{{e^{i\pi}}} + 1 = {{0}}")?;
let pyth = MathTex::typeset(&fonts, r"{{a^2}} + {{b^2}} = {{c^2}}")?;
// Typst math directly (Manim's MathTypst); a part is marked with the library's `part(name, body)` function.
let sum = MathTypst::typeset(&fonts, "sum_(k=1)^n k = #part(<rhs>, frac(n(n+1), 2))")?;

// render_frame: components borrow the typeset formulas and add props.
ui.show(Formula { of: &self.euler, size: 72.0, enter: Some(Write::default()), ..ui.defaults() });
ui.show(Formula { of: &self.pyth, color_map: &[(Part::Index(0), BLUE), (Part::Index(1), GREEN)],
                  emphasis: Some(Indicate { part: Part::Index(2), ..ui.defaults() }), ..ui.defaults() });
ui.show(TransformMatchingTex { from: &self.euler, to: &self.sum, ..ui.defaults() });
```

- `MathTex`, `Tex` (text mode through `convert_text`), `MathTypst`, `Typst`: the same four
  names as Manim Community. `MathTex` turns each `{{ .. }}` group into a part before
  conversion; `Part::Index(n)` and `Part::Label("rhs")` resolve to glyph ranges. Props stay
  plain data as the component model requires: a formula is a borrowed value from the video,
  an emphasis is a prop on the same component, not a handle to a sibling.
- The `{{ }}` to part step is inference, not Manim's method: Manim's `MathTypst` rewrites each
  group to a `manimgrp` call and re-compiles once per group with that group hidden, then diffs
  the SVG leaves (`typst_mobject.py`, `_manimgrp_preamble`). Converting a LaTeX group on its own
  can change its context (`{{a}}^2`, `\frac{{{a}}}{b}`, `\left ... \right` split across groups),
  so the spike tests those cases and falls back to Manim's hide-probe method on frames if the
  label route fails.
- `Write` staggers glyphs in drawing order with `lag_ratio`; `Create` draws the outlines;
  `DrawBorderThenFill` strokes then fills. These are the generic enter animations of the
  component model applied to a `Formula`'s glyphs.
- `TransformMatchingTex` pairs parts with the same label or the same glyph id sequence and
  moves them, fades out unmatched parts and fades in new ones; `Transform` falls back to
  point-aligned path morphing from the Manim research.
- Size is in pixels (`size` is the Typst font size in px); inside a `Canvas` (Manim units) the
  default is Manim's 48 pt equivalent. Colours default to the theme's text colour; `fill`
  from Typst (`text(fill: red)`) is kept when set.
- A number that changes per frame inside a formula (a counter) is out of C1: `Formula` is
  typeset once. The pattern is a static formula with a `Counter` component placed at a part's
  bounding box, which the library exposes (`formula.part_bbox(..)`).
- `ChemEq { ce: "2H_2 + O_2 -> 2H_2O" }` compiles through `whalogen`'s `ce` function.
  `whalogen` takes one string and documents no structured output, so per-species parts (to
  animate species across the arrow) need the library to split the mhchem string on `+` and
  the arrow and call `ce` per species; the spike checks that the pieces typeset identically.
- Code listings fall out of the same engine: Typst `raw` blocks are highlighted by syntect, so
  a `Code` component with per-token colours costs nothing extra.

## 4. Science support

| Domain | Data and simulation (pure Rust, wasm32) | Typeset notation and diagrams (Typst packages, vendored) | Animated components (library, kurbo paths) |
| --- | --- | --- | --- |
| Math | `glam` 0.34 (vectors), `num-complex` 0.4, `statrs` 0.19 (distributions) | Typst math, `physica` for vector/derivative notation | `Axes`, `NumberPlane`, `NumberLine`, `FunctionGraph`, `ParametricCurve`, `Vector`, `Matrix`, `Riemann` sums, `Brace`, the 12 math figures in the strategy |
| Physics | `rapier2d` 0.36 with `enhanced-determinism` (same result on every platform and wasm; not with `simd`); `nalgebra` 0.35 | `physica` 0.9.8 (notation), `circuiteria` 0.2.1 (block circuit diagrams, on CeTZ) | `Pendulum`, `Spring`, `Projectile`, `Orbit`, `Wave`, `FieldLines`, `Circuit` (animated current), `RayDiagram`, the 12 physics figures |
| Chemistry | `chematic` 1.2.0 (2026-10-20, MIT/Apache-2.0, no C deps, wasm32): SMILES/SMARTS/MOL parsing, 2D depiction `depict::depict_svg` with stereo; `mendeleev` 0.8.4 (element data) | `whalogen` 0.3.0 (mhchem `ce` port), `alchemist` 0.2.0 (chemfig-like skeletal formulas, on CeTZ 0.5), `typsium` 0.3.2 (molecules, equations, isotopes) | `Molecule` (from SMILES through chematic's layout, drawn as library paths so bonds and atoms can animate), `Reaction`, `PeriodicTable`, `Orbital`, `Titration` curve, the 11 chemistry figures |
| Biology | small library types for sequences, alleles and trees (no crate needed; `bio` 4.2.2 is heavy and its wasm32 build is unverified, so it is not a default dependency); no crate for tree or cell drawing found | Typst text and tables for Punnett squares and alignments; no biology package found | `DnaHelix`, `Sequence` alignment, `PunnettSquare`, `PhyloTree` (layout by the library), `Cell` diagram, `PopulationCurve`, the 11 biology figures |
| Computer science | `rust-sugiyama` 0.4 (layered DAG layout, deterministic); `petgraph` (algorithms) | `fletcher` 0.5.8 (node and arrow diagrams, on CeTZ); Typst `raw` for code | `Graph`, `Tree`, `Array`, `LinkedList`, `Stack`, `Queue`, `StateMachine`, `Code` with highlights, the 13 CS figures |
| Units | none needed | a Typst units package (`unify` or `metro`, licence to check) for siunitx-style `\qty{9.81}{m/s^2}`, since MiTeX has no package switching | used inside `MathTex` through a `units` option |
| Plots | the library's own `Axes` and `Plot` components (animatable); `plotters` 0.3.7 is stable but produces static SVG, so it is not used | none: the library's `Axes` covers static plots too, so `cetz-plot` and `lilaq` are not vendored | `LinePlot`, `BarChart`, `Histogram`, `ScatterPlot`, `Heatmap` |

Rules that make this work:

- **Static versus animated.** A Typst package draws a finished picture (a skeletal formula, a
  circuit schematic). It is right when the figure appears, is pointed at, or is written on
  (`Write` still works, glyph by glyph or shape by shape). When parts must move independently
  (atoms rearranging, current flowing), the library component draws the figure itself from the
  data crate's coordinates. Both routes return the same `Formula`-like value (paths with
  labels), so the animations do not care which one produced it.
- **Chemistry depiction.** `chematic` produces an SVG string; the library uses its 2D layout
  (atom positions and bond list, from the same depiction call) and draws atoms and bonds as
  components. If `chematic` exposes only the SVG (its 2D coordinate API is *unconfirmed*), the
  spike parses that SVG once with usvgr at build time into paths, which is acceptable for a
  build-time step but not the preferred route.
- **Vendored packages.** A custom `World` resolves `@preview/name:version` through its own
  `file` method, so the library serves the package files from memory (embedded with
  `include_dir`-style includes); no network, no cache directory, the same on every platform.
  (The CLI's local package directory and `TYPST_PACKAGE_PATH` are `typst-kit` behaviour and do
  not apply.) The set is small and pinned: `whalogen` 0.3.0 (Apache-2.0) for `\ce`, `physica`
  0.9.8 (MIT) for notation, and one skeletal-formula package, `alchemist` 0.2.0 (MIT, on CeTZ)
  or `typsium` 0.3.2 (MIT, CeTZ dependency not stated), chosen by the spike. `fletcher` 0.5.8
  (MIT, on CeTZ) and `circuiteria` 0.2.1 (Apache-2.0, on CeTZ) are optional features for static
  diagrams; the animated `Graph` and `Circuit` components do not need them. Only `cetz` and
  `whalogen` licences were read from their Universe pages; the others are *to check*.
- **CeTZ ships a WebAssembly plugin** (`cetz_core.wasm`, reported for 0.4.0 and later in the
  Typst forum, *unconfirmed* on the Universe page). Typst runs plugins in its own interpreter,
  so this should work in process natively; whether it works when the host itself is wasm32
  (the editor bridge) is *to measure*. If it does not, CeTZ-based packages are native-only and
  the editor shows the pre-rendered fallback image (section 7).
- **Licences, an open question.** `cetz` 0.5.3 is LGPL-3.0-or-later. Vendoring it means
  embedding its source and its compiled plugin into the crate and every video binary, and
  evaluating it at the video's runtime; whether that is "linking" under the LGPL is unsettled,
  the LGPL's replaceability condition is not met by an in-memory store, and the crate's
  licence field would have to add it, which downstream `cargo-deny` policies flag. Two clean
  options: load the CeTZ family from a directory the user can replace at runtime (native
  only), or leave CeTZ out of the first milestones and draw diagrams with the library's own
  components, which the animated route does anyway. The embedded fonts also need their
  notices shipped: New Computer Modern (GUST Font License), Libertinus (OFL 1.1), DejaVu
  (Bitstream Vera licence).

## 5. What agents write

The subject kits keep Manim's names and add science vocabulary that Typst packages already
define, so prior knowledge applies:

| Need | Component | Input syntax |
| --- | --- | --- |
| an equation | `MathTex` | LaTeX (amsmath subset MiTeX supports); `{{ }}` for parts |
| Typst-native math | `MathTypst` | Typst math; `part(<name>, body)` for parts |
| a chemical equation | `ChemEq` | mhchem `\ce` syntax through `whalogen` |
| a molecule | `Molecule` | SMILES (`"CC(=O)O"`) or a skeletal formula in `alchemist` syntax |
| a physics quantity | `MathTex` with `physica` on | `\va{F} = m \dv{v}{t}` |
| a quantity with units | `MathTex` with `units` on | `\qty{9.81}{m/s^2}` |
| a DNA sequence, a Punnett square, a tree of species | `Sequence`, `PunnettSquare`, `PhyloTree` | strings and small structs (`"ATGC.."`, allele pairs, a nested tree) |
| a circuit | `Circuit` (animated) or `CircuitDiagram` (static, `circuiteria`) | a component list or `circuiteria` markup |
| a graph or automaton | `Graph`, `StateMachine` | nodes and edges; layout by `rust-sugiyama` |
| a plot of data | `Axes` + `LinePlot` | `Vec<(f32, f32)>` |

`references/components.md` gets a "math and science" section with these rows, the LaTeX
subset MiTeX accepts, and the one rule that matters: parts you want to animate must be
wrapped in `{{ }}` or marked with `part(..)`. Every data crate and vendored package sits
behind a feature (`formula`, `chemistry`, `physics`, `cs`, `biology`, `diagrams`), so a video
pays only for what it uses.

## 6. Cost and caching

- A formula is typeset once per video (constructor or `OnceLock`), not per frame; the
  component model's rule 3.6 applies. Per frame, the component emits one `<g>` with the
  prebuilt `d` strings; `Write` and `Transform` rebuild only the glyphs that are moving.
- Typst compile time per small formula and the binary size of `typst` plus embedded fonts are
  *to measure*. The embedded fonts alone are 31 files; the library should embed only NewCM
  Math, NewCM and the video's fonts by default and make the rest a feature.
- A `formula!` proc macro (compile-time typesetting, paths as literals that `svgr!` caches
  statically) is possible later and would use the same pipeline; it is not needed for the
  first milestones because the build-time route already removes per-frame cost.

## 7. The spike (before C1 starts)

One crate, `fframes-components` with feature `formula`, no other components yet.

1. Implement the `World` with embedded fonts and an in-memory package store; compile
   `e^{i\pi} + 1 = 0`, a fraction, a 2x2 matrix, `\sum_{k=1}^n`, and the `whalogen` equation
   `2H_2 + O_2 -> 2H_2O`, also split per species.
2. Walk the frames into `Formula` values; render them in a gallery scene with `Write`;
   compare against Typst's own PDF output by eye and by snapshot.
3. Parts: find a label syntax that works in math mode (Typst labels attach only in markup
   mode, so the candidate is a prelude function `part(name, body)` that wraps the body in a
   labelled box; `#box[..] <l>` in math is probably invalid), confirm that labelled elements
   produce `Tag` items in typst 0.15.1, and check `{{a}}^2`, `\frac{{{a}}}{b}` and
   `\left({{x}}\right)` through MiTeX. If any of this fails, implement Manim's hide-probe
   method on frames (one extra compile per part, diff the glyph lists).
4. MiTeX coverage: run the 20 most common LaTeX constructs from Manim's examples through
   `convert_math` and list what fails.
5. Build for wasm32 with the editor bridge; measure the `.wasm` size with and without the
   `formula` feature, and CeTZ plugin behaviour on wasm32.
6. Measure: typesetting time per formula (debug and release), binary size delta, and the
   per-frame cost of a scene with 20 formulas versus the same scene with static `<text>`
   (which usvgr also outlines every frame unless the subtree is static).

Pass criteria: all five snippets render identically to Typst's PDF; parts resolve; wasm32
builds; a formula typesets in well under a second in release; the 20-formula scene renders
within 10% of the `<text>` scene. If wasm32 size is unacceptable, the formula feature becomes
native-only and the editor shows a fallback image rendered ahead of time on native (by the
video's own `frame` command or a build step), shipped in `media/` and loaded with
`ctx.get_image`, the same contract as `editor_fallback_image` for video sources.
`ctx.render_svgr` cannot serve as the fallback because it returns `None` on wasm32
(`fframes/src/fframes_context.rs:185`).

## 8. Changes to the component model milestones

| Milestone | Before (component model report, section 8) | Now |
| --- | --- | --- |
| C0 core | unchanged | unchanged, plus the formula spike runs in parallel |
| C1 shapes and enters | 22 shapes, paths, `Create`, `Write` on text, `GrowFromCenter`, `DrawBorderThenFill`, `Transform` | plus `formula`: `MathTex`, `MathTypst`, `Tex`, `Typst`, parts, `Write` on formulas, `TransformMatchingTex`, `ChemEq` |
| C2 component tags | unchanged | unchanged |
| C3 text and general | unchanged | unchanged |
| C4 explain and math | `Canvas`, `Axes`, `NumberPlane`, `FunctionGraph`, `MathTex` via Typst, `VGroup.arrange`, `LaggedStart`, 12 math figures | the same minus `MathTex` (it exists since C1), plus plots |
| C5 Studio panel | unchanged | unchanged |
| C6 other subjects and transitions | physics, CS, biology, chemistry figures; transitions; effects | split: C6a physics and chemistry (`rapier2d`, `chematic`, `mendeleev`, vendored `physica`, `whalogen`, a skeletal-formula package), C6b CS and biology (`rust-sugiyama`, library data types), C7 transitions and effects |

Corrections to the earlier reports:

- Manim research, section 4: "Typst to SVG, parsed with usvgr" becomes "Typst frames walked
  directly"; the `typst-svg` crate is not needed. Its note that Manim Community has `Typst` and
  `MathTypst` mobjects is confirmed for the main branch (`manim/mobject/text/typst_mobject.py`;
  they compile through the `typst` Python package; `Typst` selects parts by label, `MathTypst`
  by hide-probe re-renders).
- Strategy, section 8 (the formula spike): RaTeX and latex-rust are dropped from the spike;
  the spike is section 7 here.
- Component model report, unresolved question 4 (Typst only or LaTeX too): answered. LaTeX
  through MiTeX in `MathTex`, Typst directly in `MathTypst`.

## Unresolved questions

1. CeTZ and the LGPL (section 4): vendor it, load it from a replaceable directory, or leave it
   out of the first milestones? If it is out, `fletcher`, `alchemist` and `circuiteria` are
   out with it, and skeletal formulas come from `typsium` (if it has no CeTZ dependency) or
   from `chematic` only.
2. Does `chematic` expose its 2D atom coordinates, or only the SVG? (Decides whether
   `Molecule` can animate bonds without parsing SVG.)
3. Typst on wasm32: binary size and the CeTZ plugin. If too large, formulas are native-only
   with a raster fallback in the editor.
4. MiTeX is at 0.2.4 (release date *to verify*; crates.io and docs.rs disagree). If its
   coverage gaps matter, the alternatives are to write Typst math directly (agents learn
   `frac(a, b)`), or to maintain a small LaTeX macro table in the library for the missing
   commands.
5. Should `Tex` (text mode LaTeX) exist at all, or should prose use the library's `Text` and
   `Typst` components? Manim keeps both.

## Sources

- Typst: https://docs.rs/typst/latest/typst/trait.World.html,
  https://docs.rs/typst/0.15.1/typst/layout/enum.FrameItem.html,
  https://docs.rs/typst-library/latest/typst_library/layout/struct.Frame.html,
  https://docs.rs/typst-library/latest/typst_library/text/struct.TextItem.html,
  https://docs.rs/typst-library/latest/typst_library/text/struct.Glyph.html,
  https://docs.rs/typst-library/latest/typst_library/text/struct.FontInstance.html,
  https://docs.rs/crate/typst-kit/latest/features, https://github.com/typst/typst-assets/tree/main/files/fonts,
  https://docs.rs/crate/typst-pack/0.5.0, https://man.archlinux.org/man/typst-compile.1.en
- MiTeX: https://crates.io/crates/mitex, https://docs.rs/mitex/latest/mitex/,
  https://typst.app/universe/package/mitex
- Manim Community Typst mobjects: https://github.com/ManimCommunity/manim/blob/main/manim/mobject/text/typst_mobject.py
- Other renderers: https://github.com/grafeia/ReX, https://docs.rs/crate/latex-rust/1.0.2,
  https://lib.rs/crates/katex-rs
- Typst packages: https://typst.app/universe/package/{alchemist,whalogen,typsium,physica,circuiteria,cetz,cetz-plot,fletcher,lilaq}
- Rust crates: https://crates.io/crates/{chematic,mendeleev,rapier2d,nalgebra,glam,num-complex,statrs,bio,rust-sugiyama,layout-rs,plotters},
  https://docs.rs/chematic/latest/chematic/, https://rapier.rs/docs/user_guides/rust/getting_started/
- fframes: `fframes/src/{text.rs,svgr.rs,font_data.rs,media_provider.rs,lib.rs}`,
  `fframes/src/renderer/renderer_font_source.rs`, `fframes-three-d/src/geometry.rs`,
  `media-dir-macro/src/lib.rs`, `svgr-macro/src/nodes_to_svgtree.rs`
