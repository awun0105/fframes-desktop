# Research: what Manim and Remotion users use, and what fframes-kit still lacks

Date: 2026-10-24. Scope: Manim Community and its plugins (manim-physics, manim-chemistry,
ManimML, manim-dsa, manim-circuit, manim-voiceover), and Remotion with its `@remotion/*`
packages. The goal is the list of components to add to fframes-kit and the order to add them in.

## Outcome

- **Manim.** The kit already covers the most-used Manim vocabulary: `Create`, `Write`,
  `FadeIn`, `Transform`, `VGroup`, `Axes`, `ValueTracker`, `Brace`, `Table`, `Matrix` and the
  basic shapes. The largest gaps are:
  - matchers (`SurroundingRectangle`, `BackgroundRectangle`, `Cross`, `Underline`);
  - number mobjects (`DecimalNumber`, `Integer`, `Variable`);
  - `Angle`/`RightAngle`;
  - 3D (`ThreeDScene`, `Surface`, `ThreeDAxes`, `Sphere`, `Cube`);
  - camera scenes (`MovingCameraScene`, `ZoomedScene`);
  - `TracedPath`, `Rotate`/`Rotating`, `TransformFromCopy`, `Restore`, `Swap`, `Unwrite`;
  - `SVGMobject` and `ImageMobject`.
- **Remotion.** The kit covers the core (`Sequence`, `Series`, `Loop`, `Freeze`, `Fill`,
  `spring`, `interpolate`) and 8 of the 24 presentations. The largest gaps are:
  - about 60 SVG-filter effects of `@remotion/effects`;
  - 10 transitions that SVG can draw;
  - TikTok-style word captions;
  - the layout utils `fitTextOnNLines` and `fillTextBox`;
  - the asset packages (`sfx`, `animated-emoji`, `mac-cursors`).
- **Deferred by the user (2026-10-24: "temporary not implement it").** Effects that need
  per-pixel code (CPU Rust or SkSL), and shader transitions. They are listed at the end so
  they can be planned later from one place.

## Method

1. Downloads of the last month, from the npm downloads API and pypistats. They are inflated
   by transitive installs (for example, `@remotion/zod-types` is pulled in by the CLI).
2. GitHub code search counts, made with `gh api search/code`.
   - Manim queries: `NAME "from manim import" language:Python`.
   - Remotion queries: `NAME remotion language:TSX`.
3. The Manim CE reference index, and the API pages of each plugin.
4. The source trees of the plugins (`gh api repos/../git/trees`), because their READMEs list
   only a few classes.
5. The Remotion docs directory (`remotion-dev/remotion`, `packages/docs/docs`).

**Code search counts are noisy for common words.** The search is case-insensitive and matches
any token:
- `Angle` (27,392) is above `Arc` (10,752);
- `Star` (25,408) is above `Circle` (15,136);
- `Title` (22,336) is above `Axes` (12,064).

None of these rankings is real. The order below uses only distinctive names. The counts are
read as "how many files", not as a strict ranking.

## Package downloads (last month)

| Remotion package | Downloads | Kit status |
| --- | --- | --- |
| `remotion` | 7.58M | core covered |
| `@remotion/captions` | 3.45M | sentence captions only, no word pages |
| `@remotion/paths` | 2.92M | covered (`paths`) |
| `@remotion/shapes` | 1.62M | covered |
| `@remotion/transitions` | 1.52M | 8 of 24 presentations |
| `@remotion/layout-utils` | 446K | `FitText` only |
| `@remotion/motion-blur` | 270K | covered (`Trail`, `CameraMotionBlur`) |
| `@remotion/noise` | 233K | `Noise` (grain), no `noise2D/3D/4D` functions |
| `@remotion/lottie` | 179K | fframes `lottie` feature |
| `@remotion/animation-utils` | 176K | `interpolateStyles` and `makeTransform` are not mapped |
| `@remotion/effects` | 151K | 4 effects (`Blur`, `Glow`, `Noise`, `LightLeak`) |
| `@remotion/sfx` | 66K | none |
| `@remotion/animated-emoji` | 62K | none |
| `@remotion/mac-cursors` | 6.3K | none |

| Manim package | Downloads | Kit status |
| --- | --- | --- |
| `manim` | 132,683 | see the gaps below |
| `manim-voiceover` | 5,706 | none |
| `manim-physics` | 386 | 15 figures, missing the items below |
| `manim-chemistry` | 381 | 15 figures, missing the items below |
| `manim-ml` | 195 | none |
| `manim-circuit` | 178 | one fixed `Circuit` figure, no schematic symbols |
| `manim-dsa` | 139 | most structures covered by `kits::cs` |
| `manim-data-structures` | 46 | `MArray`, `MVariable`: covered by `ArrayView` |

## Manim names by code search (distinctive names only)

| Count | Names | Kit |
| --- | --- | --- |
| over 5,000 | `MathTex` 20,032, `Transform` 18,816, `Axes` 12,064, `Code` 11,072, `LaggedStart` 8,336, `SurroundingRectangle` 7,216, `RoundedRectangle` 6,784, `Table` 6,400, `ReplacementTransform` 5,664, `ValueTracker` 5,664, `Indicate` 5,528, `GrowArrow` 5,120 | all covered except **`SurroundingRectangle`** |
| 2,000 to 5,000 | `add_updater` 4,888, `DashedLine` 4,880, `always_redraw` 4,664, `Matrix` 4,528, `Flash` 4,096, **`ThreeDScene` 3,688**, `GrowFromCenter` 3,212, **`ImageMobject` 3,200**, `NumberPlane` 3,160, **`Surface` 2,784**, **`Variable` 2,616**, **`ThreeDAxes` 2,308**, **`MovingCameraScene` 2,180**, **`DecimalNumber` 2,136**, `MoveAlongPath` 2,048, **`Restore` 2,040** | bold names are missing |
| 1,000 to 2,000 | **`Integer` 1,952**, `Uncreate` 1,792, `DrawBorderThenFill` 1,740, **`TransformFromCopy` 1,728**, **`CurvedArrow` 1,716**, **`Swap` 1,680**, **`SVGMobject` 1,568**, **`Sphere` 1,560**, `ParametricFunction` 1,556, `Circumscribe` 1,484, **`Underline` 1,448**, `TransformMatchingTex` 1,388, **`Cube` 1,372**, `Succession` 1,312, `NumberLine` 1,292, `Paragraph` 1,288, **`DashedVMobject` 1,242**, **`VoiceoverScene` 1,234**, **`MarkupText` 1,144**, `DoubleArrow` 1,104, **`Unwrite` 1,076** | bold names are missing |
| 300 to 1,000 | `get_area` 982, **`TracedPath` 970**, **`Rotating` 948**, **`FadeTransform` 932**, **`MoveToTarget` 902**, **`Cone` 902**, **`ShowPassingFlash` 878**, **`ArcBetweenPoints` 856**, `Wiggle` 810, **`TransformMatchingShapes` 764**, **`BackgroundRectangle` 700**, **`Cylinder` 599**, `FunctionGraph` 501, **`BraceBetweenPoints` 479**, `BarChart` 432, **`Prism` 420**, `get_riemann_rectangles` 407, **`AddTextLetterByLetter` 396**, **`RightAngle` 391**, **`BulletedList` 350**, **`Broadcast` 332**, **`Torus` 318** | bold names are missing |
| under 300 | **`ZoomedScene` 280**, **`ComplexPlane` 276**, **`ImplicitFunction` 264**, **`PolarPlane` 258**, **`Elbow` 236**, **`ApplyWave` 233**, **`StreamLines` 221**, `LinearTransformationScene` 212, **`TangentLine` 201** (kit `Tangent`), **`ShowIncreasingSubsets` 197**, **`ApplyMatrix` 188**, **`ArrowVectorField` 184**, `SpinInFromNothing` 183, **`FocusOn` 169**, **`DiGraph` 164**, **`MathTable` 160**, **`LabeledDot` 158**, **`VectorScene` 129**, **`ChangeDecimalToValue` 124**, `BraceLabel` 118, **`CyclicReplace` 67** | bold names are missing |

**Noisy counts:** `Rotate` 9,200, `Vector` 7,584, `Cross` 6,080, `Polygon` 4,976,
`Triangle` 4,016, `Brace` 3,184 and `Graph` 11,360. They are common words, so their order
means little, but all of them are core vocabulary.

## Remotion names by code search

The sweep counts how many TSX files mention each name together with "remotion".

**Core API.** The kit maps all of these.

| Name | Files |
| --- | --- |
| `AbsoluteFill` | 75,008 |
| `interpolate` | 69,872 |
| `useVideoConfig` | 57,088 |
| `spring` | 41,224 |
| `Img` | 28,800 |
| `Easing` | 27,631 |
| `Sequence` | 27,033 |
| `Audio` | 19,290 |
| `Loop` | 11,442 |
| `Series` | 7,627 |
| `OffthreadVideo` | 5,248 |
| `Freeze` | 2,640 |
| `interpolateColors` | 1,376 |

**Component-level names:**

| Name | Files | Kit status |
| --- | --- | --- |
| `loadFont` | 7,456 | n/a: fonts come from media |
| `Gif` | 4,258 | not covered |
| `TransitionSeries` | 3,376 | covered |
| `Trail` | 3,291 | covered |
| `linearTiming` | 2,488 | covered |
| `fitText` | 2,056 | covered |
| `iris` | 946 | covered |
| `ThreeCanvas` | 852 | covered by `fframes::three_d` |
| `noise2D` | 659 | **missing** |
| `measureText` | 632 | covered |
| `LightLeak` | 615 | covered |
| `springTiming` | 563 | covered |
| `Lottie` | 539 | covered by fframes |
| `CameraMotionBlur` | 435 | covered |
| `visualizeAudio` | 385 | covered |
| `evolvePath` | 380 | covered |
| `createTikTokStyleCaptions` | 275 | **missing** |
| `clockWipe` | 246 | covered |
| `makeTransform` | 148 | **missing** |
| `parseSrt` | 121 | **missing** |
| `noise3D` | 82 | **missing** |
| `AnimatedEmoji` | 73 | **missing** |
| `fillTextBox` | 19 | **missing** |

- **Noisy counts:** `Star` 64,640, `Rect` 57,767, `fade` 31,040, `slide` 17,995 and
  `random` 8,891 are common words.
- **What the counts show.** Among missing component names, the noise functions and the
  TikTok captions are used most. Phases 14 and 22 do them early inside their phases.
- **`Gif`** is used in 4,258 files. fframes has no GIF decoder, but the kit can decode the
  frames itself (the `image` crate) and draw them through `rendered_image`, with no core
  change. It is not planned until the user decides.
- The sweep script and its CSV are in the session scratchpad (`usage-sweep.ps1`,
  `usage.csv`).

## Gaps, by area

### Manim CE mobjects

- **Frame:**
  - `FullScreenRectangle`, `ScreenRectangle`.
- **Geometry:**
  - `AnnotationDot`, `AnnularSector`, `ArcBetweenPoints`, `ArcPolygon`, `ArcPolygonFromArcs`, `CubicBezier`,
    `CurvedArrow`, `CurvedDoubleArrow`, `LabeledDot`, `TangentialArc`.
  - `Polygram`, `RegularPolygram`, `ConvexHull`, `Cutout`.
  - Labeled shapes: `LabeledArrow`, `LabeledLine`, `LabeledPolygram`.
  - Angles: `Angle`, `RightAngle`, `Elbow`.
  - Arrow tips: `ArrowCircleTip`, `ArrowCircleFilledTip`, `ArrowSquareTip`,
    `ArrowSquareFilledTip`, `ArrowTriangleTip`, `ArrowTriangleFilledTip`, `StealthTip`.
- **Boolean operations:**
  - `Union`, `Intersection`, `Difference`, `Exclusion`.
- **Matchers:**
  - `SurroundingRectangle`, `BackgroundRectangle`, `Cross`, `Underline`.
- **Braces:**
  - `ArcBrace`, `BraceBetweenPoints`, `BraceText`.
- **Coordinate systems and functions:**
  - `ComplexPlane`, `PolarPlane`, `ThreeDAxes`, `ImplicitFunction`, `LogBase` scaling,
    `UnitInterval`, `SampleSpace`.
- **Matrices and tables:**
  - `DecimalMatrix`, `IntegerMatrix`, `MobjectMatrix`.
  - `DecimalTable`, `IntegerTable`, `MathTable`, `MobjectTable`.
- **Text:**
  - `DecimalNumber`, `Integer`, `Variable`, `BulletedList`, `Title`, `MarkupText`.
- **Graphs:**
  - `Graph` and `DiGraph` with the layouts `circular`, `kamada_kawai`, `planar`, `random`,
    `shell`, `spectral`, `spiral`, `spring`, `partite`, `tree` and `LayoutFunction`.
  - The kit's `GraphView` takes positions only.
- **Vector fields:**
  - `ArrowVectorField`, `StreamLines` (with `start_animation`), `VectorField`.
- **Media:**
  - `SVGMobject`, `VMobjectFromSVGPath`, `ImageMobject`.
- **Structure:**
  - `DashedVMobject`, `CurvesAsSubmobjects`, `VDict`, `ComplexValueTracker`.
- **3D**, through `fframes::three_d`:
  - `Surface`, `Sphere`, `Cube`, `Prism`, `Cone`, `Cylinder`, `Torus`, `Dot3D`, `Line3D`,
    `Arrow3D`.
  - The polyhedra: `Tetrahedron`, `Octahedron`, `Dodecahedron`, `Icosahedron`, `ConvexHull3D`.

### Manim CE animations

- **Creation:**
  - `Unwrite`, `AddTextLetterByLetter`, `AddTextWordByWord`, `RemoveTextLetterByLetter`.
  - `ShowIncreasingSubsets`, `ShowPartial`, `ShowSubmobjectsOneByOne`, `SpiralIn`.
  - `TypeWithCursor`, `UntypeWithCursor`.
- **Growth and indication:**
  - `GrowFromEdge`, `ApplyWave`, `Blink`, `FocusOn`, `ShowPassingFlash`,
    `ShowPassingFlashWithThinningStrokeWidth`, `Broadcast`.
- **Movement:**
  - `Rotate`, `Rotating`, `Homotopy`, `ComplexHomotopy`, `SmoothedVectorizedHomotopy`,
    `PhaseFlow`.
- **Transforms:**
  - `ClockwiseTransform`, `CounterclockwiseTransform`, `CyclicReplace`, `Swap`.
  - `FadeToColor`, `FadeTransform`, `FadeTransformPieces`, `ScaleInPlace`, `ShrinkToCenter`.
  - `TransformFromCopy`, `TransformMatchingShapes`, `TransformAnimations`.
  - `ApplyMatrix`, `ApplyFunction`, `ApplyComplexFunction`, `ApplyPointwiseFunction`,
    `ApplyPointwiseFunctionToCenter`.
- **Numbers:**
  - `ChangeDecimalToValue`, `ChangingDecimal`.
- **Time and composition:**
  - `Wait`, `ChangeSpeed`, `LaggedStartMap`, `AnimatedBoundary`, `TracedPath`.
- **Imperative names with no struct of their own:**
  - `ApplyMethod`, `MoveToTarget`, `Restore`, `UpdateFromFunc`, `UpdateFromAlphaFunc`,
    `MaintainPositionRelativeTo`.
  - In a declarative kit, these become mapping rows and doc examples on the existing
    `Transform`, `Sequence` and `ui.sample`.
- **Scenes:**
  - `MovingCameraScene`, `ZoomedScene`, `ThreeDScene`, `VectorScene`.
  - `LinearTransformationScene` is the kit's `LinearTransform`.

### manim-physics

- Missing:
  - electrostatics: `Charge`, `ElectricField`;
  - magnetostatics: `Wire`, `MagneticField`;
  - optics: `Lens` (thin and thick, with focal points) and `Ray` through lenses;
  - rigid bodies: `Space` and `SpaceScene` with pymunk boxes, circles, polygons and walls;
  - pendulums: `MultiPendulum` (chaos);
  - waves: `LinearWave`, `RadialWave` (surface), `StandingWave`.
- Already in the kit: `Projectile`, `Pendulum`, `EnergyBars`, `SpringMass`, `Wave`,
  `FreeBody`, `Orbit`, `FieldLines`, `Circuit`, `RayDiagram`, `Refraction`, `Collision1D`,
  `Doppler`, `GasBox`, `Sim`.
- `rapier2d` was already picked for rigid bodies (math and science report, section 4).

### manim-chemistry

- Missing:
  - `MMoleculeObject` from `.mol` / `.sdf` files;
  - `NamedMolecule` (a molecule looked up by name);
  - `GraphMolecule` with `GMAnimationBuilder` (bond rotation, colour change);
  - `MElementObject` (an element card with more fields);
  - `ThreeDMolecule` from files.
- Already in the kit: `BohrAtom`, `ElementTile`, `PeriodicTable` (with trends), `Molecule`
  (SMILES through chematic), `Molecule2D`, `Molecule3D`, `Orbital`, `ReactionEquation`,
  `BondFormation`, `EnergyProfile`, `TitrationCurve`, `PhScale`, `StatesOfMatter`,
  `Equilibrium`, `ChemEq`.
- `NamedMolecule` needs a name-to-structure table. Only a small embedded table is possible
  offline.

### ManimML

From the source tree of `helblazer811/ManimML`:

- `NeuralNetwork`, with these layers:
  - `FeedForwardLayer` (with activation functions `ReLU` and `Sigmoid`);
  - `Convolutional2DLayer`;
  - `MaxPooling2DLayer`;
  - `ImageLayer`;
  - `EmbeddingLayer`;
  - `VectorLayer`;
  - `MathOperationLayer`;
  - `PairedQueryLayer`;
  - `TripletLayer`.
- Each pair of layers has a connection class (for example `Convolutional2DToMaxPooling2D`).
- Network animations:
  - `make_forward_pass_animation`;
  - the dropout animation;
  - `insert_layer`, `remove_layer`, `replace_layer`.
- `VariationalAutoencoder`.
- `DecisionTreeDiagram` (from a fitted tree, unfolding level by level), and
  `DecisionTreeSurface`.
- MCMC: `MCMCAxes` with a Metropolis–Hastings chain.
- `GaussianDistribution`: covariance ellipses.

### manim-dsa

- `MArray` (indexes, `append`, `pop`, labels), `MStack`.
- `MGraph`: nodes, straight and curved edges, weights, highlight, backward edges, networkx
  layouts.
- `MTree`, `MVariable`.

The kit already has `ArrayView`, `Stack`, `Queue`, `LinkedList`, `TreeView` (BST, heap),
`GraphView` with BFS, DFS and Dijkstra traces, `HashTable` and `RecursionTree`. Still missing:

- curved and backward edges;
- edge weights drawn as labels on the edge;
- automatic graph layouts (shared with Manim `Graph`);
- array `append` and `pop` as animated steps;
- a variable box (`MVariable`).

### manim-circuit

`Resistor`, `Capacitor`, `Inductor`, `VoltageSource`, `CurrentSource`, `Ground`, `Opamp`.

- Each symbol has labels and `get_terminals`.
- A circuit is wired from terminal to terminal.

The kit's `Circuit` is one fixed figure (a battery, a switch, a bulb, a resistor).

### manim-voiceover

- Main API:
  - `VoiceoverScene` with `with self.voiceover(text) as tracker:`;
  - `tracker.duration`, `get_remaining_duration`, `time_until_bookmark`;
  - `wait_until_bookmark` with `<bookmark mark="A"/>` in the text;
  - `add_wrapped_subcaption`.
- Speech services: Azure, Coqui, gTTS, OpenAI, pyttsx3, ElevenLabs, Gemini, and a recorder.
  Word timings come from the service or from Whisper.
- In scope: timing only, with no TTS and no recording. The user supplies the audio and the
  timings.

### Remotion transitions

| Presentation | Kit | Plan |
| --- | --- | --- |
| `fade`, `slide`, `wipe`, `flip`, `clockWipe`, `iris` | covered (with `CircleWipe`, `SlantWipe`) | none |
| `none` | missing | SVG: a hard cut that still takes the overlap |
| `pushCut` | missing | SVG: transforms |
| `zoomInOut` | missing | SVG: transforms and opacity |
| `dreamyZoom` | missing | SVG: zoom plus a blur filter |
| `linearBlur` | missing | SVG: directional blur (`stdDeviation` x/y) |
| `blurSlide` | missing | SVG: slide plus blur |
| `crossZoom` | missing | SVG approximation: zoom plus blur plus opacity |
| `dissolve` | missing | SVG: a `feTurbulence` threshold mask |
| `filmBurn` | missing | SVG: a gradient and turbulence overlay |
| `swap` | missing | SVG: 2D approximation of the card swap |
| `useTransitionProgress` | missing | the progress value inside a scene |
| `crosswarp`, `ripple`, `bookFlip`, `cube` (paid), `custom-html-in-canvas` | missing | **deferred** (pixel or shader) |

Remotion also has audio transitions: a fade of the scene audio during the overlap.

### Remotion effects

**SVG-filter effects** (planned). They work on both renderers: Skia handles every filter
primitive, and the CPU backend renders through resvg.

- **Colour:** brightness, contrast, saturation, hue, grayscale, invert, tint, duotone,
  levels, curves, exposure, colour correction, white balance, vibrance, shadows/highlights
  (approximate), colour key, thermal vision, LUT (approximate, through
  `feComponentTransfer` tables).
- **Edges and shape:** outline, emboss, bevel alpha, long shadows, roughen edges, shrinkwrap.
- **Noise and distortion:**
  - noise, white noise, fractal noise;
  - noise displacement, turbulent displace;
  - wave, waves, zigzag (through `feDisplacementMap` with generated maps).
- **Patterns and overlays:**
  - scanlines, dot grid, checkerboard, gridlines, lines, rings;
  - pattern, paper, burlap, flannel, speckle;
  - linear gradient tint, linear gradient, vignette, light trail.
- **Halftone:** halftone, halftone linear gradient (a mask of dots).
- **Geometry:** mirror, tile, skew, scale, x/y translate, uv translate.
- **Blinds and reveals:** venetian blinds, evolve (a reveal by mask), TV signal off (scale
  and flash).
- **Blur:** region blur (blur inside a mask), linear and radial progressive blur (several
  blurred copies under graded masks).

**Already in the kit:** `Blur`, `Glow`, `DropShadow`, `Noise`, `LightLeak` and
`CameraMotionBlur`. Starburst is drawable as SVG and can join `LightLeak`.

### Remotion text, captions and utilities

- **`@remotion/captions`:**
  - `createTikTokStyleCaptions`: pages of words, with the current word highlighted.
  - `parseSrt`, `serializeSrt`.
  - The kit reads VTT through fframes.
- **`@remotion/layout-utils`:**
  - `fitTextOnNLines`, `fillTextBox` (`measureText` is `text_width`; `fitText` is
    `FitText`).
- **`@remotion/animation-utils`:**
  - `interpolateStyles`;
  - `makeTransform` (a transform builder; `fframes::Transform` covers it).
- **`@remotion/noise`:**
  - `noise2D`, `noise3D`, `noise4D` as plain functions, used to drive motion.

### Remotion assets

- **`@remotion/sfx`:** whoosh, ding, mouse-click, page-turn, record-scratch, shutter,
  ui-switch, vine-boom, whip and other meme sounds.
  - The licence differs per file, so ship only CC0 files.
- **`@remotion/animated-emoji`:** Google Noto animated emoji. They are Lottie files under
  CC BY 4.0.
  - The kit plays user-supplied files through the fframes `lottie` feature.
  - The kit ships names and attribution, not the files.
- **`@remotion/mac-cursors`:** Apple cursor artwork.
  - The kit draws its own cursors instead (no Apple artwork).

## Deferred (user, 2026-10-24: "temporary not implement it")

These need per-pixel code: CPU Rust over `ctx.render_svgr` and `fframes::svgr_image`, or SkSL
through `fframes::Shader` with an `.image` uniform.

- **Effects:** fisheye, barrel distortion, sphere, cylinder, pixelate, linear/radial
  progressive pixelate, pixel dissolve, chromatic aberration, zoom blur, lens blur, page
  turn, shatter, tear, corner pin, contour lines, liquid contours, shine, true halftone
  sampling, true LUT files.
- **Transitions:** `crosswarp`, `ripple`, `bookFlip`, `cube`, `custom-html-in-canvas`.

Feasibility was checked without a core change:
- `ctx.render_svgr(svgr, w, h)` rasterises a subtree with the backend's rasterizer
  (`fframes/src/fframes_context.rs:179`). It returns `None` in the editor.
- `rendered_image(width, height, rgba)` wraps pixels.
- Shader uniforms accept images.

## Excluded (user, 2026-10-24)

- OpenGL and interactive-only Manim features (`manimgl` interactivity, `InteractiveScene`).
- Typst mobjects.
- `ManimBanner`.

## Unresolved questions

- **SFX sources.** CC0 sources were found for most sounds:
  - Kenney "Interface Sounds" for click, switch and pop;
  - qubodup's bamboo swooshes on OpenGameArt for whoosh.

  Ding and shutter have no source yet.
- **`NamedMolecule`** will use a small embedded table of about 100 names, written from
  public facts.
- **Settled by probes on 2026-10-24:**
  - boolean operations use `i_overlay` 9.1;
  - rigid bodies use `rapier2d` 0.36;
  - both build for wasm32;
  - `feTurbulence` agrees across the two renderers.

  `feDisplacementMap` is still unchecked.
- **`<Gif>`** can be done in the kit without a core change: decode the frames with the
  `image` crate, then draw them through `rendered_image`. Whether to add it is the user's
  call.
