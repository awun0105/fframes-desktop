# Research: a Manim-style library for fframes

Question: how could fframes get a library like Manim (Manim Community), so that videos and
agents can write `Create`, `Write`, `Transform`, `Axes`, `MathTex` and the rest on top of
fframes rendering?

Builds on: [library strategy](components-261023-0804-library-strategy.md) (this becomes its
"explain" layer and the base of the domain kits).

> **Partly superseded** by [the component model](research-261023-1700-component-model-remotion-manim.md):
> the recorded `Script` becomes optional sugar, milestones M0 to M6 become C0 to C7, and the
> cheat sheet and gallery move from `skills/` and `examples/` into the fork-owned
> `components/` workspace (its section 10). Formula rendering follows
> [math, LaTeX and science support](research-261023-1718-math-latex-science-support.md).
> Units, rendering notes and the mapping tables (sections 3 to 5) still stand.

## Summary

- **It fits fframes, with one translation.** Manim is imperative: `construct()` calls
  `self.play(...)` and time advances as it runs. fframes is a pure function of time:
  `render_frame` runs on several threads. The bridge is to record a Manim-style script once in
  the video's constructor into a timeline, then evaluate that timeline at each frame's time.
  ranim, an MIT-licensed Rust engine inspired by Manim, uses this record-then-sample model
  (`r.play(seq![...])`, then sampling a frame at a time).
- **Keep Manim's names, kwargs and units.** Manim's class names become Rust tuple structs, so
  `self.play(Transform(circle, square), run_time=2)` becomes
  `s.play(Transform(circle, square)).run_time(2.0)`. An agent, or a person, can port a Manim
  example almost line by line. This is the biggest win for agents.
- **Equations: use Typst.** Manim Community itself now ships `Typst` and `MathTypst`
  mobjects; they compile Typst markup to SVG with the `typst` package and import it as
  `SVGMobject`. ranim does the same in Rust with `typst` + `typst-svg` 0.15.1. fframes already
  parses SVG with `usvgr`, so the same path works here. This replaces the three-way spike in
  the strategy with a check of one candidate.
- **Build our own engine; borrow ideas from ranim.** ranim is the most active Rust port, but
  its API is marked unstable and it brings its own wgpu renderer and `bevy_ecs`. fframes
  already has rendering, encoding, audio, the editor and Studio. What it lacks is the
  mobject, animation and script layer, which is the part to write.
- **Scope.** About 200 Manim classes map onto 7 milestones (section 7). Milestones M0 to M3
  (shapes, script, animations, text, math, axes, graphs of functions) cover most educational
  videos.

## 1. Manim in one page

| Concept | Manim | Note for fframes |
| --- | --- | --- |
| Scene | `class S(Scene): def construct(self)` | runs once, imperatively |
| Mobject | `Mobject`, `VMobject` (cubic Bezier points + fill and stroke), `Group`, `VGroup` | `VMobject` maps to a `kurbo::BezPath` plus style |
| Animation | `Animation.interpolate(alpha)`, `run_time` (1 s default), `rate_func` (`smooth` default), `lag_ratio` | alpha in 0..1 after the rate function |
| Play | `self.play(a, b, run_time=2)` plays in parallel and advances time; `self.wait(1)` | the script cursor |
| Animate syntax | `self.play(sq.animate.shift(UP).scale(2))` | records a target state, then interpolates |
| State | `save_state`, `restore`, `generate_target` + `MoveToTarget`, `become`, `copy` | snapshots in the script |
| Updaters | `add_updater(f)`, `always_redraw(f)`, `ValueTracker` | must become pure functions of time |
| Positioning | `shift`, `move_to`, `next_to(m, RIGHT, buff=..)`, `to_edge`, `to_corner`, `align_to`, `arrange`, `arrange_in_grid` | layout at build time |
| Units | frame 8 units tall, origin at the center, y up; `UP`, `DOWN`, `LEFT`, `RIGHT`, `UL` ... directions | convert to pixels at render |
| Camera | `MovingCameraScene` (`self.camera.frame.animate`), `ZoomedScene`, `ThreeDScene` | a transform on the root group, or 3D |
| Media | `add_sound`, `add_subcaption`, `next_section` | map to `AudioMap`, captions, fframes `Scenes` |

Rate functions (49 in `manim.utils.rate_functions`): `linear`, `smooth`, `smoothstep`,
`smootherstep`, `smoothererstep`, `rush_into`, `rush_from`, `slow_into`, `double_smooth`,
`there_and_back`, `there_and_back_with_pause`, `running_start`, `not_quite_there`, `wiggle`,
`squish_rate_func`, `lingering`, `exponential_decay`, `unit_interval`, `zero`, and the
`ease_{in,out,in_out}_{sine,quad,cubic,quart,quint,expo,circ,back,elastic,bounce}` family.
All of them are plain `f32 -> f32` functions and port directly.

## 2. The translation: script, then timeline

```rust
use fframes_manim::prelude::*; // crate name is an open question

#[derive(Debug)]
pub struct SquareToCircle {
    script: Script,
}

impl SquareToCircle {
    pub fn new(media: &MyVideoMedia) -> Self {
        // Runs once, like Manim's construct(). Nothing here runs per frame.
        // Fonts come from the video's embedded media (MediaProvider::get_all_font_data).
        let script = Script::build(media, |s| {
            let circle = s.add(Circle::new().set_fill(PINK, 0.5));
            s.play(Create(circle));
            s.play(Transform(circle, Square::new().rotate(PI / 4.0)));
            s.play(circle.animate().shift(RIGHT * 2.0).scale(0.5)).run_time(2.0);
            let euler = s.add(MathTex::new("e^{i\\pi} + 1 = 0"));
            s.play((FadeOut(circle).shift(DOWN), Write(euler))); // a tuple plays in parallel
            s.wait(1.0);
        });
        Self { script }
    }
}

impl Video for SquareToCircle {
    const FPS: usize = 60;
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;

    fn duration(&self) -> Duration<'_> { self.script.duration() }
    fn audio(&self) -> AudioMap<'_> { self.script.audio() } // from add_sound

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        fframes::svgr!(<svg viewBox="0 0 1920 1080" width={Self::WIDTH} height={Self::HEIGHT}>
            {self.script.render(&frame, ctx)}
        </svg>)
    }
}
```

How it works:

1. **Handles.** `s.add(..)` returns a `Mob`, a `Copy` id. The script owns every mobject's
   state, so Rust borrowing never gets in the way, and `circle` can be used after `Transform`
   just like in Manim.
2. **Fonts.** `Text` outlines and Typst layout happen at build time, before any `ctx`
   exists, so `Script::build` takes the video's media. It reads the embedded fonts with
   `MediaProvider::get_all_font_data()` and adds Typst's bundled fonts (`typst-kit` feature
   `embedded-fonts`, which includes a math font; ranim does the same). System fonts are never
   scanned, so renders are reproducible. The editor runs the same constructor in WASM with the
   same embedded fonts, so it lays out identically.
3. **Recording.** Each `play` gets the current state of the mobjects it touches. It records
   one segment per mobject (start time, end time, animation, state before, state after) and
   moves the cursor by the longest `run_time`. `.animate()` collects method calls and applies
   them to a copy, which becomes the target state. This is what Manim does at runtime. `play`
   records immediately and returns a handle to that group: `.run_time(..)`, `.rate_func(..)`
   and `.lag_ratio(..)` rewrite the group's segments and move the cursor again, so the next
   `play` starts after the stretched group.
4. **Sampling.** `render(&frame, ctx)` takes the frame time. For each mobject on screen it
   finds the segment at that time, computes `alpha = rate_func((t - start) / run_time)` and
   asks the animation for the state at `alpha`. The result is turned into `svgr!` paths. The
   result depends only on time, so it is safe to run on several threads.
5. **Updaters become pure closures.** `s.always_redraw(|at| ...)` and `add_updater` take
   `Fn(&At) -> VMobject + Send + Sync`. `At` exposes the time, each `ValueTracker`'s value at
   that time (trackers are timelines too) and other mobjects' states. Manim's mutable updater
   (`mob.move_to(dot)` every frame) becomes "position of `dot` at `t`".
6. **It plugs into fframes.**
   - `Script` implements `fframes::Scene`, so scripts can be listed in `define_scenes` and
     cross-fade with `Overlap`.
   - `next_section` maps to Studio timing clips.
   - `add_sound` feeds `Video::audio`.
   - `add_subcaption` produces the captions track.

## 3. Units and defaults (Manim compatible)

- Coordinates are Manim units: the frame is 8 units tall, with the origin at the center and y
  pointing up. `render` scales by height using `Video::WIDTH` and `HEIGHT`, as Manim does, so
  the width follows the aspect ratio: 14.2 units at 16:9 but only 4.5 units in a 1080x1920
  portrait video. A 16:9 example drawn in portrait is cut off at the sides and needs its own
  layout.
- Constants are kept with Manim's names:
  - directions: `UP`, `DOWN`, `LEFT`, `RIGHT`, `ORIGIN`, `UL`, `UR`, `DL`, `DR`, `IN`, `OUT`;
  - buffers: `SMALL_BUFF`, `MED_SMALL_BUFF`, `MED_LARGE_BUFF`, `LARGE_BUFF`;
  - default font size, stroke width and `PI`/`TAU`;
  - the Manim color palette (`BLUE_A` ... `BLUE_E`, `PINK`, ...).

  The exact values are copied from the Manim source in M0 and checked by a test, not written
  from memory.
- The theme is optional. `Theme::from_styles` (the 40 Studio preset tokens) can replace the
  Manim palette and fonts, so an explainer matches a Studio project's look.

## 4. Rendering on fframes

| Need | How |
| --- | --- |
| `VMobject` to pixels | `kurbo::BezPath` per mobject, emitted as `<path d=.. fill stroke>` in `svgr!` (kurbo is already an optional dependency for `lottie`) |
| `Create`, `ShowPartial` | sub-path by arc-length fraction (kurbo segments) |
| `DrawBorderThenFill`, `Write` | stroke partial, then fill fades in; `Write` staggers glyph submobjects with `lag_ratio` |
| `Transform` | align both paths to the same number of curves (subdivide), then interpolate points and style; Manim's `align_points` |
| `Text` | glyph outlines from the font file (`ttf-parser` and shaping through `usvgr`'s font stack) at build time, so `Write` and `Transform` work on letters; simple static text can stay `<text>` |
| `Tex`, `MathTex`, `Typst`, `MathTypst` | Typst to SVG (`typst`, `typst-svg`), parsed with `usvgr` into submobjects at build time; labeled sub-expressions for `TransformMatchingTex` |
| `ImageMobject` | `<image>` through `ctx.get_image` |
| Boolean ops (`Union`, `Difference`, ...) | needs a path boolean library: Skia PathOps (Skia is already a dependency of `fframes-skia-renderer`), or a pure Rust one for the CPU and editor builds; decide in M4 |
| 3D (`ThreeDScene`, `Surface`, `Sphere`) | Manim-style vector 3D: project 3D points and sort faces back to front. For solid, lit 3D use `fframes::three_d` |

**Cost.** Static `svgr!` caching does not apply to paths that change every frame. Mobjects
that are not animating at time `t` reuse a path string built once per segment, so only moving
mobjects rebuild their paths. M0 measures a 200-mobject scene with `just bench` before the API
grows.

**LaTeX syntax.** Agents will write LaTeX in `MathTex("\\frac{a}{b}")`, while Typst math
syntax is different. Option: accept LaTeX in `MathTex` and `Tex` through a LaTeX-to-Typst
converter (the MiTeX project is one candidate, to be checked), and Typst in `MathTypst` and
`Typst`, as Manim does. This needs to be verified in M2.

## 5. Manim to fframes mapping

Status: **M0** to **M6** are milestones (section 7). **fframes** means it already exists in
fframes and only needs a wrapper. **skip** means not planned.

### Animations

| Manim module | Classes | Plan |
| --- | --- | --- |
| animation | `Animation`, `Wait`, `Add` | M0 |
| composition | `AnimationGroup`, `Succession`, `LaggedStart`, `LaggedStartMap` | M0 |
| creation | `Create`, `Uncreate`, `DrawBorderThenFill`, `Write`, `Unwrite`, `ShowPartial` | M0 |
| creation (text) | `AddTextLetterByLetter`, `AddTextWordByWord`, `RemoveTextLetterByLetter`, `TypeWithCursor`, `UntypeWithCursor`, `ShowIncreasingSubsets`, `ShowSubmobjectsOneByOne` | M2 |
| creation | `SpiralIn` | M4 |
| fading | `FadeIn`, `FadeOut` (`shift`, `scale`, `target_position`) | M0 |
| transform | `Transform`, `ReplacementTransform`, `TransformFromCopy`, `.animate` (`ApplyMethod`), `MoveToTarget`, `Restore`, `FadeToColor`, `ScaleInPlace`, `ShrinkToCenter` | M0 |
| transform | `ClockwiseTransform`, `CounterclockwiseTransform`, `Swap`, `CyclicReplace`, `FadeTransform`, `FadeTransformPieces`, `ApplyFunction`, `ApplyMatrix`, `ApplyPointwiseFunction`, `ApplyPointwiseFunctionToCenter` | M1 |
| transform | `ApplyComplexFunction` | M4 |
| transform | `TransformAnimations` | skip |
| transform_matching_parts | `TransformMatchingShapes`, `TransformMatchingTex` | M2 |
| rotation | `Rotate`, `Rotating` | M0 |
| growing | `GrowFromPoint`, `GrowFromCenter`, `GrowFromEdge`, `GrowArrow`, `SpinInFromNothing` | M1 |
| indication | `Indicate`, `Flash`, `Circumscribe`, `FocusOn`, `ShowPassingFlash`, `ShowPassingFlashWithThinningStrokeWidth`, `Wiggle`, `Blink`, `ApplyWave` | M1 |
| specialized | `Broadcast` | M1 |
| movement | `MoveAlongPath` | M1 |
| movement | `Homotopy`, `ComplexHomotopy`, `SmoothedVectorizedHomotopy`, `PhaseFlow` | M4 |
| numbers | `ChangingDecimal`, `ChangeDecimalToValue` | M2 |
| changing | `TracedPath`, `AnimatedBoundary` | M3 |
| speedmodifier | `ChangeSpeed` | M3 |
| updaters | `UpdateFromFunc`, `UpdateFromAlphaFunc`, `MaintainPositionRelativeTo`, `add_updater`, `always_redraw` | M3 (pure closures) |

### Mobjects

| Manim module | Classes | Plan |
| --- | --- | --- |
| mobject, types | `Mobject`, `Group`, `VMobject`, `VGroup`, `VectorizedPoint`, `DashedVMobject` | M0 |
| types | `VDict`, `CurvesAsSubmobjects` | M1 |
| types | `ImageMobject` | M1 (`ctx.get_image`) |
| types | `PMobject`, `Mobject1D`, `Mobject2D`, `PGroup`, `Point`, `PointCloudDot`, `ImageMobjectFromCamera` | skip (use dots) |
| geometry.arc | `Circle`, `Dot`, `Arc`, `ArcBetweenPoints`, `Ellipse`, `Sector`, `Annulus`, `AnnularSector`, `CubicBezier` | M0 |
| geometry.arc | `CurvedArrow`, `CurvedDoubleArrow`, `AnnotationDot`, `LabeledDot`, `ArcPolygon`, `ArcPolygonFromArcs`, `TangentialArc` | M1 |
| geometry.line | `Line`, `DashedLine`, `Arrow`, `Vector`, `DoubleArrow` | M0 |
| geometry.line | `TangentLine`, `Elbow`, `Angle`, `RightAngle` | M1 |
| geometry.polygram | `Polygon`, `Polygram`, `RegularPolygon`, `RegularPolygram`, `Triangle`, `Rectangle`, `Square`, `RoundedRectangle`, `Star` | M0 |
| geometry.polygram | `Cutout`, `ConvexHull` | M1 |
| geometry.tips | `ArrowTip`, `ArrowTriangleTip`, `ArrowTriangleFilledTip`, `ArrowCircleTip`, `ArrowCircleFilledTip`, `ArrowSquareTip`, `ArrowSquareFilledTip`, `StealthTip` | M1 |
| geometry.shape_matchers | `SurroundingRectangle`, `BackgroundRectangle`, `Cross`, `Underline` | M1 |
| geometry.labeled | `Label`, `LabeledLine`, `LabeledArrow`, `LabeledPolygram` | M2 |
| geometry.boolean_ops | `Union`, `Intersection`, `Difference`, `Exclusion` | M4 |
| frame | `ScreenRectangle`, `FullScreenRectangle` | M1 |
| text | `Text`, `Paragraph`, `MarkupText` | M2 |
| text | `Tex`, `MathTex`, `SingleStringMathTex`, `MathTexPart`, `BulletedList`, `Title`, `Typst`, `MathTypst` | M2 (Typst) |
| text.numbers | `DecimalNumber`, `Integer`, `Variable` | M2 |
| text.code_mobject | `Code` | M4 (needs a highlighter) |
| svg | `SVGMobject`, `VMobjectFromSVGPath` | M2 (`usvgr`) |
| svg.brace | `Brace`, `BraceBetweenPoints`, `BraceLabel`, `BraceText`, `ArcBrace` | M2 |
| graphing.number_line | `NumberLine`, `UnitInterval` | M3 |
| graphing.coordinate_systems | `Axes`, `NumberPlane`, `ComplexPlane`, `PolarPlane` (with `c2p`, `p2c`, `plot`, `plot_parametric_curve`, `get_graph_label`, `get_area`, `get_riemann_rectangles`, `get_secant_slope_group`, `get_vertical_line`, `get_T_label`, `i2gp`) | M3 |
| graphing.coordinate_systems | `ThreeDAxes` | M6 |
| graphing.functions | `FunctionGraph`, `ParametricFunction`, `ImplicitFunction` | M3 |
| graphing.probability | `BarChart`, `SampleSpace` | M3 |
| graphing.scale | `LinearBase`, `LogBase` | M3 |
| value_tracker | `ValueTracker`, `ComplexValueTracker` | M3 |
| graph | `Graph`, `DiGraph`, `GenericGraph`, `LayoutFunction` (layouts `circular`, `shell`, `spiral`, `partite`, `tree`, seeded `spring`) | M4 |
| graph | layouts `kamada_kawai`, `spectral`, `planar`, `random` | later |
| matrix | `Matrix`, `DecimalMatrix`, `IntegerMatrix`, `MobjectMatrix` | M4 |
| table | `Table`, `MathTable`, `MobjectTable`, `IntegerTable`, `DecimalTable` | M4 |
| vector_field | `VectorField`, `ArrowVectorField`, `StreamLines`, `StreamLine` | M4 |
| three_d | `ThreeDVMobject`, `Surface`, `Sphere`, `Dot3D`, `Cube`, `Prism`, `Cone`, `Arrow3D`, `Cylinder`, `Line3D`, `Torus` | M6 |
| three_d.polyhedra | `Polyhedron`, `Tetrahedron`, `Octahedron`, `Icosahedron`, `Dodecahedron`, `ConvexHull3D` | M6 |
| logo | `ManimBanner` | skip |

### Scenes, cameras, utilities

| Manim | Plan |
| --- | --- |
| `Scene`: `play`, `wait`, `pause`, `add`, `remove`, `clear`, `bring_to_front`, `bring_to_back`, `add_foreground_mobject(s)` | M0 (`Script`) |
| `wait_until` | M3 (needs trackers) |
| `add_sound` | M5 (feeds `Video::audio`, fframes mixer) |
| `add_subcaption` | M5 (captions track) |
| `next_section` | M5 (fframes `Scenes` / timing clips) |
| `MovingCameraScene`, `MovingCamera` (`camera.frame.animate`) | M5 (transform on the root group) |
| `ZoomedScene` | M5 |
| `LinearTransformationScene`, `VectorSpaceScene` | M4 |
| `ThreeDScene`, `ThreeDCamera` (`set_camera_orientation`, `move_camera`, `begin_ambient_camera_rotation`, `add_fixed_in_frame_mobjects`) | M6 |
| `MultiCamera`, `MappingCamera` | skip |
| config (frame size, fps, background) | from `Video` constants; nothing to configure |
| `rate_functions` (49), `color` (`ManimColor`, palette, interpolation), `constants`, `bezier`, `space_ops`, `paths` (`straight_path`, `path_along_arc`, `clockwise_path`) | M0 |
| CLI, `manim.utils.testing` | fframes CLI (`frame`, `strip`, `inspect`) and `e2e` snapshots |

## 6. What makes it easy for agents

1. **Same names, same arguments.** Class names stay CamelCase tuple structs (`Create(mob)`,
   `Transform(a, b)`). Methods keep Manim's snake_case names (`next_to`, `to_edge`,
   `arrange`). Each Python kwarg becomes a builder method of the same name: `run_time=2`
   becomes `.run_time(2.0)`, `buff=0.5` becomes `.buff(0.5)`.
2. **A "Manim to fframes" cheat sheet.** `skills/fframes-video/references/manim.md` lists the
   few rules that differ:
   - mobjects are added and referenced by handle;
   - updaters are closures of `At`;
   - `PI` is an `f32`;
   - LaTeX goes in `MathTex`, Typst in `MathTypst`.

   Every snippet in it is compiled as a doctest.
3. **A ported gallery.** Port Manim's example gallery (square to circle, `MovingAngle`,
   `SineCurveUnitCircle`, `GraphAreaPlot`, `MovingFrameBox`, ...) into
   `examples/manim-gallery`, one scene per example with the same name. An agent can run
   `frame SineCurveUnitCircle@50%` to see what a call produces.
4. **Script inspection.** `inspect` reports a `play` on a mobject that was never added,
   overlapping segments on one mobject, and text that leaves the frame. Studio receives the
   script's sections as clips.

## 7. Milestones

| Milestone | Scope | Done when |
| --- | --- | --- |
| M0 | `VMobject`, `Group`, `Script` (record and sample), handles, `.animate`, rate functions, colors, constants, positioning, basic geometry, `Create`, `Write` (shapes), `FadeIn`/`FadeOut`, `Transform` family, composition, `Rotate`; cost measurement | square-to-circle and 5 gallery ports render; doctests and wasm check pass; 200-mobject bench recorded |
| M1 | the rest of geometry, tips, shape matchers, growing, indication, `MoveAlongPath`, more transforms, `ImageMobject` | gallery ports for each |
| M2 | `Text` (glyph outlines), Typst `MathTex`/`MathTypst`, LaTeX syntax check, numbers, braces, `SVGMobject`, labeled geometry, `TransformMatchingTex`/`Shapes`, text creation animations | `Write(MathTex(..))` and `TransformMatchingTex` work on CPU and in the editor, or the editor fallback is documented |
| M3 | `NumberLine`, `Axes`, `NumberPlane`, `PolarPlane`, `ComplexPlane`, function graphs, `BarChart`, `ValueTracker`, updaters, `always_redraw`, `TracedPath`, `wait_until` | the gallery's graph and tracker examples render |
| M4 | `Graph`/`DiGraph` with layouts, `Matrix`, `Table`, `VectorField`/`StreamLines`, boolean ops, homotopies, `Code`, linear transformation scene | as above |
| M5 | moving and zoomed camera, `add_sound`, `add_subcaption`, `next_section` with Studio clips | a video with sound, captions and sections renders through `fframes::cli` |
| M6 | 3D: `ThreeDAxes`, `Surface`, solids, polyhedra, `ThreeDScene` camera (vector 3D), with `fframes::three_d` for lit solids | 3D gallery ports render |

After M3, the domain kits from the strategy (math, physics, CS, biology, chemistry) are built
from these mobjects instead of from their own foundation. `Pendulum`, `SortVisualizer` and the
rest become small `Script` helpers.

## 8. Build or adopt

| Project | State | Use |
| --- | --- | --- |
| [ranim](https://lib.rs/crates/ranim) | MIT, 0.2.1 (May 2026), active, API marked unstable; own wgpu renderer, `bevy_ecs`; Typst through `typst` + `typst-svg` 0.15.1 | Read for design: `seq!`, morph alignment, Typst world setup in `packages/ranim-items/src/vitem/typst.rs`. Code may be borrowed with attribution (MIT). Do not depend on it. |
| [aniy](https://docs.rs/aniy) | personal project, SVG based, "not for production" | none |
| [mathlikeanim-rs](https://github.com/MathItYT/mathlikeanim-rs) | WebAssembly, work in progress, last release March 2024 | none |
| [Manimx](https://cnb.cool/Manim-X/manimx) | one commit | none |

## Unresolved questions

1. Crate name: `fframes-manim` says what it is but uses Manim's name;
   `fframes-explain` or `fframes-mobject` avoid that. Default: `fframes-explain`, with the
   Manim mapping in its docs, the way `fframes_three_d` maps three.js.
2. Should the Manim names be exact (`Create(circle)`, `MathTex`), even where they read
   unusually in Rust? Default: yes, for agent recall.
3. Should `MathTex` accept LaTeX syntax through a converter, or only Typst? Default: try to
   support LaTeX in M2; if the converter fails, `MathTex` takes Typst syntax and the cheat
   sheet says so.
4. How does this relate to the general components crate (lower thirds, captions)? Default:
   two crates. The general components can use `fframes-explain` mobjects where it helps, but
   they do not require it.

Sources:
[Manim reference index](https://docs.manim.community/en/stable/reference.html),
[animations](https://docs.manim.community/en/stable/reference_index/animations.html),
[mobjects](https://docs.manim.community/en/stable/reference_index/mobjects.html),
[rate functions](https://docs.manim.community/en/stable/reference/manim.utils.rate_functions.html),
[Mobject](https://docs.manim.community/en/stable/reference/manim.mobject.mobject.Mobject.html),
[Scene](https://docs.manim.community/en/stable/reference/manim.scene.scene.Scene.html),
[Typst mobject](https://docs.manim.community/en/stable/reference/manim.mobject.text.typst_mobject.html),
[ranim](https://github.com/AzurIce/ranim) (`examples/hello_ranim/lib.rs`,
`packages/ranim-items/Cargo.toml`, `packages/ranim-items/src/vitem/typst.rs`).
