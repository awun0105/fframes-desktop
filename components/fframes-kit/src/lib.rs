//! Components for [fframes](https://github.com/dmtrKovalenko/fframes) videos, in the
//! style of Remotion with Manim's names for animations.
//!
//! ```no_run
//! use fframes_kit::prelude::*;
//! ```
//!
//! is the only import a video needs. A component is a props struct plus
//! `render(self, &mut Ui)`. You set the props you care about and fill the rest from the
//! theme with `..ui.defaults()`; `ui.show(component)` renders it at the current time and
//! returns an `Svgr` for any `{expr}` in `svgr!`. A component has no start time of its own:
//! its parent places it with a [`Sequence`](time::Sequence), and inside, time starts at 0.
//!
//! | Remotion | fframes-kit |
//! | --- | --- |
//! | `<Sequence from durationInFrames>` | [`Sequence`](time::Sequence) `{ from, duration, children }` (seconds) |
//! | `<Series>` | [`Series`](time::Series) |
//! | `<Loop>` | [`Loop`](time::Loop) |
//! | `<Freeze>` | [`Freeze`](time::Freeze) |
//! | `<AbsoluteFill>` | [`Fill`](time::Fill) |
//! | `useCurrentFrame()` | `ui.frame.index`, `ui.seconds()` |
//! | `useVideoConfig().durationInFrames` | `ui.duration` |
//! | `interpolate`, `spring` | [`motion::progress`], [`motion::lerp`], [`motion::spring`], `timeline!` through [`Anim`](anim::Anim) |
//! | `@remotion/shapes` (`Circle`, `Ellipse`, `Polygon`, `Star`, `Triangle`, `Rect`, `Arrow`, `Heart`, `Pie`, ...) | [`shapes`]: the same names, `Arrow` is [`BlockArrow`](shapes::BlockArrow) |
//! | `@remotion/paths` (`getLength`, `evolvePath`, `interpolatePath`, ...) | [`paths`]: the same names in snake_case over [`kurbo::BezPath`] |
//! | `<TransitionSeries>`, `.Sequence`, `.Transition` | [`TransitionSeries`](transitions::TransitionSeries), `TransitionSeries::sequence`, `TransitionSeries::transition` |
//! | `fade()`, `slide()`, `wipe()` | [`Fade`](transitions::Fade), [`Slide`](transitions::Slide), [`Wipe`](transitions::Wipe), or your own [`Presentation`](transitions::Presentation) |
//! | `none()`, `pushCut()`, `zoomInOut()` | [`NoTransition`](transitions::NoTransition), [`PushCut`](transitions::PushCut), [`ZoomInOut`](transitions::ZoomInOut) |
//! | `dreamyZoom()`, `linearBlur()`, `blurSlide()`, `crossZoom()` | [`DreamyZoom`](transitions::DreamyZoom), [`LinearBlur`](transitions::LinearBlur), [`BlurSlide`](transitions::BlurSlide), [`CrossZoom`](transitions::CrossZoom) |
//! | `dissolve()`, `filmBurn()` | [`Dissolve`](transitions::Dissolve), [`FilmBurn`](transitions::FilmBurn) |
//! | `swap()` | [`SwapCards`](transitions::SwapCards) (`Swap` is Manim's animation) |
//! | `useTransitionProgress()` | [`Ui::transition_progress`](ui::Ui::transition_progress) |
//! | audio fade on transitions | [`audio_fades`](transitions::audio_fades), [`TransitionSeries::audio_fades`](transitions::TransitionSeries::audio_fades), applied in `fn audio` |
//! | `crosswarp()`, `ripple()`, `bookFlip()`, `cube()`, `custom-html-in-canvas` | not yet available (pixel shaders) |
//! | `@remotion/effects` colour: brightness, contrast, saturation, hue, grayscale, invert, tint, duotone, levels, curves, exposure, colour correction, white balance, vibrance, shadows/highlights, colour key, thermal vision, LUT | [`effects`]: [`Brightness`](effects::Brightness), [`Contrast`](effects::Contrast), [`Saturation`](effects::Saturation), [`HueRotate`](effects::HueRotate), [`Grayscale`](effects::Grayscale), [`Invert`](effects::Invert), [`Tint`](effects::Tint), [`Duotone`](effects::Duotone), [`Levels`](effects::Levels), [`Curves`](effects::Curves), [`Exposure`](effects::Exposure), [`ColorCorrection`](effects::ColorCorrection), [`WhiteBalance`](effects::WhiteBalance), [`Vibrance`](effects::Vibrance), [`ShadowsHighlights`](effects::ShadowsHighlights), [`ColorKey`](effects::ColorKey), [`ThermalVision`](effects::ThermalVision), [`Lut`](effects::Lut); several matrices in one filter with [`ColorStack`](effects::ColorStack) |
//! | `@remotion/effects` edges and shape: `outline()`, `emboss()`, `bevelAlpha()`, `longShadows()`, `roughenEdges()`, `shrinkwrap()` | [`Outline`](effects::Outline), [`Emboss`](effects::Emboss), [`BevelAlpha`](effects::BevelAlpha), [`LongShadows`](effects::LongShadows), [`RoughenEdges`](effects::RoughenEdges), [`Shrinkwrap`](effects::Shrinkwrap) (without the displacement) |
//! | `whiteNoise()`, `fractalNoise()` | [`WhiteNoise`](effects::WhiteNoise), [`FractalNoise`](effects::FractalNoise) (`feTurbulence`: two fractal types) |
//! | `scanlines()`, `dotGrid()`, `checkerboard()`, `gridlines()`, `lines()`, `rings()`, `linearGradientTint()`, `linearGradient()`, `vignette()` | [`Scanlines`](effects::Scanlines), [`DotGrid`](effects::DotGrid), [`Checkerboard`](effects::Checkerboard), [`Gridlines`](effects::Gridlines) (flat: no `rotationX`, `rotationY`, `perspective`), [`Lines`](effects::Lines), [`Rings`](effects::Rings), [`LinearGradientTint`](effects::LinearGradientTint), [`LinearGradient`](effects::LinearGradient), [`Vignette`](effects::Vignette) |
//! | `pattern()`, `speckle()`, `flannel()`, `burlap()`, `paper()` | [`Pattern`](effects::Pattern), [`Speckle`](effects::Speckle), [`Flannel`](effects::Flannel), [`Burlap`](effects::Burlap), [`Paper`](effects::Paper) (approximations; `paper()`'s `drops` and `fade` are not available) |
//! | `halftone()`, `halftoneLinearGradient()`, `starburst()`, `lightTrail()` | [`Halftone`](effects::Halftone), [`HalftoneLinearGradient`](effects::HalftoneLinearGradient), [`Starburst`](effects::Starburst), [`LightTrail`](effects::LightTrail) (approximations; `halftone()`'s `sampling` is not available) |
//! | `mirror()`, `tile()`, `skew()`, `scale()`, `xyTranslate()`, `uvTranslate()`, `venetianBlinds()`, `evolve()`, `tvSignalOff()` | [`Mirror`](effects::Mirror), [`Tile`](effects::Tile) (the rectangle is given as `rect`), [`Skew`](effects::Skew), [`Scale`](effects::Scale), [`Translate`](effects::Translate) (`uv: true` for `uvTranslate()`), [`VenetianBlinds`](effects::VenetianBlinds), [`Evolve`](effects::Evolve), [`TvSignalOff`](effects::TvSignalOff) |
//! | `regionBlur()`, `linearProgressiveBlur()`, `radialProgressiveBlur()` | [`RegionBlur`](effects::RegionBlur), [`LinearProgressiveBlur`](effects::LinearProgressiveBlur), [`RadialProgressiveBlur`](effects::RadialProgressiveBlur) (blurred copies under graded masks, at most `MAX_BLUR_LEVELS`) |
//! | `noiseDisplacement()`, `turbulentDisplace()`, `wave()`, `waves()`, `zigzag()` | not yet available (`feDisplacementMap` draws wrongly on the CPU backend) |
//! | fisheye, barrel distortion, sphere, cylinder, pixelate, progressive pixelate, pixel dissolve, chromatic aberration, zoom blur, lens blur, page turn, shatter, tear, corner pin, contour lines, liquid contours, shine | not yet available (per-pixel code) |
//! | `@remotion/animation-utils`: `makeTransform([...])` | [`fframes::Transform`] `{ translate_x, translate_y, rotate, scale, skew_x, skew_y, ..Default::default() }` (written as translate, rotate, scale, skew X, skew Y) |
//! | `@remotion/animation-utils`: `interpolateStyles` | [`motion::progress`] over the input range, then [`motion::lerp`] for each value |
//! | `@remotion/noise`: `noise2D`, `noise3D`, `noise4D` | [`motion::noise_2d`], [`motion::noise_3d`], [`motion::noise_4d`] (seed: a number or a string) |
//! | `linearTiming`, `springTiming` | [`Timing::Linear`](transitions::Timing::Linear), [`Timing::Spring`](transitions::Timing::Spring) |
//! | JSX: `<LowerThird name="Ada" />`, children between tags | [`ui!`](ui!): the same tags inside `svgr!` markup, expanded to `ui.show(LowerThird { name: "Ada".into(), ..ui.defaults() })` |
//!
//! | Manim | fframes-kit |
//! | --- | --- |
//! | `FadeIn(m, shift=UP)` | [`FadeIn`](anim::FadeIn) `{ shift: (0.0, -40.0), .. }` (canvas units, `y` down) |
//! | `FadeOut(m)` | [`FadeOut`](anim::FadeOut) |
//! | `run_time`, `rate_func` | the same fields; [`RateFunc`](anim::RateFunc) has `smooth`, `linear`, `rush_into`, `rush_from`, `there_and_back`, `double_smooth` |
//! | `lag_ratio` | [`motion::stagger`], [`LaggedStart`](animate::LaggedStart), [`AnimationGroup`](animate::AnimationGroup), [`Succession`](animate::Succession) |
//! | `Circle`, `Square`, `Arrow`, `Brace`, ... (mobjects) | [`shapes`], placed by center in canvas pixels |
//! | `SurroundingRectangle`, `BackgroundRectangle`, `Cross`, `Underline` | [`SurroundingRectangle`](shapes::SurroundingRectangle), [`BackgroundRectangle`](shapes::BackgroundRectangle), [`Cross`](shapes::Cross), [`Underline`](shapes::Underline) around a `kurbo::Rect` (`shape.bbox()`, [`TextPath::bounds`](shapes::TextPath::bounds)) |
//! | `DashedVMobject`, `CurvesAsSubmobjects` | [`DashedVMobject`](shapes::DashedVMobject) ([`dash_path`](shapes::dash_path)), [`CurvesAsSubmobjects`](shapes::CurvesAsSubmobjects) ([`curves`](shapes::curves)) |
//! | `ArcBetweenPoints`, `CurvedArrow`, `CurvedDoubleArrow`, `TangentialArc`, `CubicBezier`, `AnnularSector`, `ArcPolygon`, `ArcPolygonFromArcs` | the same names in [`shapes`] |
//! | `AnnotationDot`, `LabeledDot`, `LabeledLine`, `LabeledArrow` | the same names in [`shapes`], the label a [`ShapeLabel`](shapes::ShapeLabel): text, or a formula with the `latex` feature |
//! | `Angle`, `RightAngle`, `Elbow` | [`Angle`](shapes::Angle), [`RightAngle`](shapes::RightAngle), [`Elbow`](shapes::Elbow), with `quadrant`, `other_angle`, `dot` and a label |
//! | `TangentLine` | [`TangentLine`](shapes::TangentLine) for any path; `kits::math::Tangent` (feature `math`) for a function graph on `Axes` |
//! | `Polygram`, `RegularPolygram`, `ConvexHull`, `Cutout`, `ScreenRectangle`, `FullScreenRectangle`, `LabeledPolygram` | the same names in [`shapes`] ([`convex_hull`](shapes::convex_hull), [`polylabel`](shapes::polylabel)) |
//! | `ArrowTriangleTip`, `ArrowTriangleFilledTip`, `ArrowCircleTip`, `ArrowCircleFilledTip`, `ArrowSquareTip`, `ArrowSquareFilledTip`, `StealthTip` | [`ArrowTip`](shapes::ArrowTip) `::Triangle`, `::TriangleFilled`, `::Circle`, `::CircleFilled`, `::Square`, `::SquareFilled`, `::Stealth` in an arrow's `tip_start` / `tip_end` |
//! | `BraceBetweenPoints`, `BraceText`, `ArcBrace` | [`Brace`](shapes::Brace) (between its two points; a negative `depth` turns it over), [`BraceLabel`](label::BraceLabel), [`ArcBrace`](shapes::ArcBrace) with [`ArcBrace::tip`](shapes::ArcBrace::tip) |
//! | `Union`, `Intersection`, `Difference`, `Exclusion` | the same names in [`shapes`]; [`boolean`](shapes::boolean) for one path (nonzero fill, curves flattened at 0.25 px) |
//! | `SVGMobject`, `VMobjectFromSVGPath` | [`SvgShape`](shapes::SvgShape) from [`SvgShape::parse`](shapes::SvgShape::parse), parts by id or index; [`SvgPath`](shapes::SvgPath) with [`paths::parse_path`] |
//! | `ImageMobject` | `<image href={ctx.get_image(..)?.href()} ..>`, framed by [`PhotoFrame`](decor::PhotoFrame), zoomed by [`KenBurns`](media::KenBurns) |
//! | `Text` as a `VMobject` | [`TextPath`](shapes::TextPath) (glyph outlines) |
//! | `Create`, `Uncreate`, `DrawBorderThenFill`, `Write` | [`animate`], in a shape's `enter` / `exit` prop |
//! | `Unwrite`, `RemoveTextLetterByLetter` | [`animate`], `exit` |
//! | `ShowPartial`, `ShowIncreasingSubsets`, `ShowSubmobjectsOneByOne`, `SpiralIn`, `AddTextLetterByLetter` | [`animate`], `enter`, on the glyphs of a [`TextPath`](shapes::TextPath) or the parts given to [`draw_animated`](animate::draw_animated) |
//! | `AddTextLetterByLetter`, `AddTextWordByWord` on `Text` | [`Typewriter`](text::Typewriter) `{ cursor: None, .. }`, [`TextReveal`](text::TextReveal) `{ by: RevealBy::Word, run_time: 0.0, .. }` |
//! | `TypeWithCursor`, `UntypeWithCursor` | [`Typewriter`](text::Typewriter), `erase: Some(seconds)` for the second |
//! | `GrowFromCenter`, `GrowFromPoint`, `GrowArrow`, `SpinInFromNothing` | [`animate`], `enter` |
//! | `Indicate`, `Circumscribe`, `Flash`, `Wiggle` | [`animate`], `emphasis` |
//! | `GrowFromEdge` | [`GrowFromEdge`](animate::GrowFromEdge) `{ edge: Anchor::Bottom, .. }`, `enter` |
//! | `Blink`, `FocusOn` | [`Blink`](animate::Blink), [`FocusOn`](animate::FocusOn), `emphasis` |
//! | `Rotate`, `Rotating` | [`Rotate`](animate::Rotate), [`Rotating`](animate::Rotating), `emphasis` (a positive angle turns counterclockwise) |
//! | `Homotopy`, `ComplexHomotopy`, `SmoothedVectorizedHomotopy`, `PhaseFlow` | the same names in [`animate`], components over a shape's points in `ui.space` units |
//! | `TracedPath` | [`TracedPath`](animate::TracedPath) `{ point: &\|t\| .., .. }`, sampled at most [`MAX_TRACE_SAMPLES`](animate::MAX_TRACE_SAMPLES) times |
//! | `ShowPassingFlash`, `ShowPassingFlashWithThinningStrokeWidth`, `Broadcast`, `AnimatedBoundary`, `ApplyWave` | the same names in [`animate`], components that take a [`Shape`](geometry::Shape) (`Broadcast` a point) |
//! | `Transform`, `ReplacementTransform` | [`animate::Transform`] (`Morph` in the prelude), [`ReplacementTransform`](animate::ReplacementTransform) |
//! | `MoveAlongPath` | [`MoveAlongPath`](animate::MoveAlongPath) |
//! | `TransformFromCopy`, `ClockwiseTransform`, `CounterclockwiseTransform` | the same names in [`animate`], components over two [`Shape`](geometry::Shape)s; `path_arc` sets the arc the points travel on |
//! | `CyclicReplace`, `Swap` | [`CyclicReplace`](animate::CyclicReplace), [`Swap`](animate::Swap): the shapes move on arcs to each other's places |
//! | `FadeTransform`, `FadeTransformPieces` | [`FadeTransform`](animate::FadeTransform), [`FadeTransformPieces`](animate::FadeTransformPieces): pieces pair by index, the pieces left over fade where they are instead of being split |
//! | `TransformMatchingShapes` | [`TransformMatchingShapes`](animate::TransformMatchingShapes) over two lists of pieces, matched by [`shape_key`](animate::shape_key); pieces of one key pair in order, not as a group |
//! | `TransformAnimations` | [`TransformAnimations`](animate::TransformAnimations): two `enter` animations, the shape and its look go from the first to the second |
//! | `FadeToColor`, `ScaleInPlace` | [`FadeToColor`](animate::FadeToColor), [`ScaleInPlace`](animate::ScaleInPlace), `emphasis` |
//! | `ShrinkToCenter` | [`ShrinkToCenter`](animate::ShrinkToCenter), `exit` |
//! | `ApplyMatrix`, `ApplyPointwiseFunction`, `ApplyComplexFunction`, `ApplyFunction`, `ApplyPointwiseFunctionToCenter` | the same names in [`animate`], components over a shape in `ui.space` units, `y` up |
//! | `LaggedStartMap` | [`LaggedStart::map`](animate::LaggedStart::map) |
//! | `ChangeSpeed` | [`ChangeSpeed`](animate::ChangeSpeed) `{ speed: vec![(fraction, speed), ..], .. }` around the children |
//! | `ChangeDecimalToValue`, `ChangingDecimal` | [`ChangeDecimalToValue`](animate::ChangeDecimalToValue), [`ChangingDecimal`](animate::ChangingDecimal) over a [`DecimalNumber`](text::DecimalNumber) |
//! | `Wait` | no call: what follows starts later, [`Sequence`](time::Sequence) `{ from, .. }` |
//! | `ApplyMethod`, `.animate`, `MoveToTarget` | [`animate::Transform`] (`Morph`) from the shape to a copy with the change made (the target) |
//! | `save_state`, `Restore` | a clone of the shape kept, [`animate::Transform`] from the changed shape back to it |
//! | `UpdateFromFunc`, updaters | the prop computed from `ui.seconds()` in `render`, which runs every frame |
//! | `UpdateFromAlphaFunc` | the same with [`RateFunc::apply`](anim::RateFunc::apply) over [`motion::progress`] |
//! | `MaintainPositionRelativeTo` | the follower placed from the leader's position at the same time, plus the offset |
//! | `MathTex`, `Tex`, `TexTemplate` (feature `latex`) | `latex::MathTex`, `latex::Tex`, `latex::TexTemplate`, typeset with a `latex::TexEngine` and drawn by `latex::FormulaView` |
//! | `TransformMatchingTex` (feature `latex`) | `latex::TransformMatchingTex` |
//! | the frame: 8 units tall, `y` up, origin in the middle | [`Space`](explain::Space) in `ui.space`, [`Canvas`](explain::Canvas) for another one |
//! | `MovingCameraScene`, `self.camera.frame`, `camera.auto_zoom` | [`MovingCamera`](explain::MovingCamera) `{ center, width, children }`, [`auto_zoom`](explain::auto_zoom) |
//! | `ZoomedScene` (`zoomed_camera`, `zoomed_display`, `get_zoomed_display_pop_out_animation`) | [`ZoomedInset`](explain::ZoomedInset) `{ frame, display, delay, run_time, children }` |
//! | `VectorScene` (`add_vector`, `get_basis_vectors`), `LinearTransformationScene` | a [`NumberPlane`](explain::NumberPlane) with [`Vector`](shapes::Vector)s, `kits::math::LinearTransform` (feature `math`) for the transformation |
//! | `ThreeDScene`, `set_camera_orientation`, `begin_ambient_camera_rotation`, `add_fixed_in_frame_mobjects` (feature `3d`) | `three::ThreeDView { phi, theta, focal_distance, zoom, ambient_rotation, objects, fixed_in_frame }`, `three::Projector` for 2D on 3D points |
//! | `Surface`, `checkerboard_colors` (feature `3d`) | `three::Surface { func, u_range, v_range, resolution, checkerboard_colors }` |
//! | `ThreeDAxes`, `c2p`, `get_axis_labels` (feature `3d`) | `three::ThreeDAxes` with `c2p` and `show_labels` |
//! | `Sphere`, `Cube`, `Prism`, `Cylinder`, `Cone`, `Torus`, `Dot3D`, `Line3D`, `Arrow3D` (feature `3d`) | the same names in `three`, Manim's sizes and resolutions |
//! | `Polyhedron`, `Tetrahedron`, `Octahedron`, `Icosahedron`, `Dodecahedron`, `ConvexHull3D` (feature `3d`) | the same names in `three`; `three::convex_hull_faces` for the hull's faces |
//! | `Create`, `FadeIn` on a 3D mobject (feature `3d`) | the object's `enter: Some(Create { .. }.into())`, a `three::Enter3D` |
//! | `NumberLine`, `Axes`, `NumberPlane` | [`explain`]: the same names, `x_range: Range::new(min, max, step)` |
//! | `LinearBase`, `LogBase` (`scaling=`) | [`Scaling`](explain::Scaling) in `Axes::x_scaling`, `y_scaling`, `NumberLine::scaling`; the range in exponents |
//! | `UnitInterval` | [`NumberLine::unit_interval`](explain::NumberLine::unit_interval) |
//! | `ComplexPlane`, `n2p`, `p2n`, `ComplexValueTracker` | [`ComplexPlane`](explain::ComplexPlane), `(re, im)` pairs, [`ComplexValueTracker`](anim::ComplexValueTracker) |
//! | `PolarPlane`, `pr2pt`, `pt2pr` | [`PolarPlane`](explain::PolarPlane) with [`AzimuthUnits`](explain::AzimuthUnits) |
//! | `SampleSpace`, `divide_vertically`, `divide_horizontally` | [`SampleSpace`](explain::SampleSpace) of [`SampleColumn`](explain::SampleColumn)s of [`SampleRow`](explain::SampleRow)s |
//! | `ImplicitFunction`, `axes.plot_implicit_curve` | [`ImplicitFunction`](explain::ImplicitFunction) (marching squares), `Axes::plot_implicit_curve` |
//! | `FunctionGraph` | [`FunctionGraph`](explain::FunctionGraph), from `Axes::plot` or with its own `x_range` |
//! | `VectorField`, `ArrowVectorField` | [`VectorField`](explain::VectorField) (the field and its colours), [`ArrowVectorField`](explain::ArrowVectorField) |
//! | `StreamLines`, `start_animation` | [`StreamLines`](explain::StreamLines), traced once and cached; `flow: Some(`[`Flow`](explain::Flow)`)` |
//! | `Graph`, `DiGraph` (`labels`, `vertex_config`, `edge_config`, `layout_scale`) | `explain::Graph` (not in the prelude, the CS kit's `Graph` is), [`DiGraph`](explain::DiGraph), [`VertexStyle`](explain::VertexStyle), [`EdgeStyle`](explain::EdgeStyle) |
//! | `layout="circular"`, `"kamada_kawai"`, `"planar"`, `"random"`, `"shell"`, `"spectral"`, `"spiral"`, `"spring"`, `"partite"`, `"tree"`, a layout callable | [`GraphLayout`](layout::GraphLayout) and [`LayoutFunction`](layout::LayoutFunction), also as [`graph_layout`](layout::graph_layout) |
//! | `axes.c2p`, `p2c`, `plot`, `i2gp`, `get_vertical_line`, `get_graph_label`, `get_riemann_rectangles`, `get_area` | `Axes::c2p`, `p2c`, `plot`, `FunctionGraph::point`, `Axes::vertical_line`, `graph_label`, `riemann_rectangles`, `area` |
//! | `m.next_to(point, UR)` | [`Label`](label::Label) `{ direction: Anchor::TopRight, .. }` |
//! | `ParametricFunction` | [`ParametricFunction`](explain::ParametricFunction) |
//! | `VGroup(..).arrange(RIGHT, buff)`, `arrange_in_grid` | [`VGroup`](explain::VGroup) `::arrange`, `::arrange_in_grid`; boxes alone with [`layout::arrange`], `row`, `column`, `grid` |
//! | `VDict` (a `VGroup` with keys) | a `BTreeMap<&str, Shape>` of your own: look a part up by its key, draw them all with `VGroup::new(parts.values().cloned().collect())` |
//! | `DecimalMatrix`, `IntegerMatrix`, `MobjectMatrix` | [`DecimalMatrix`](explain::DecimalMatrix), [`IntegerMatrix`](explain::IntegerMatrix), [`MobjectMatrix`](explain::MobjectMatrix) of [`Mobject`](explain::Mobject)s, each with a `matrix: Matrix` for the look |
//! | `DecimalTable`, `IntegerTable`, `MobjectTable`, `MathTable` (feature `latex`) | [`DecimalTable`](explain::DecimalTable), [`IntegerTable`](explain::IntegerTable), [`MobjectTable`](explain::MobjectTable), `explain::MathTable` of typeset formulas, each with a `table: Table` for the labels and look |
//! | `Matrix`, `Table`, `BraceLabel` | [`Matrix`](explain::Matrix), [`Table`](explain::Table), [`BraceLabel`](label::BraceLabel) |
//! | `ValueTracker` with `always_redraw` | [`ValueTracker`](anim::ValueTracker), a `timeline!` read with `ui.sample` inside `render` |
//! | `DecimalNumber`, `Integer`, `Variable` | [`DecimalNumber`](text::DecimalNumber) (digits in fixed slots), [`Integer`](text::Integer), [`Variable`](text::Variable), each following a `ValueTracker` |
//! | `BulletedList`, `Title`, `MarkupText` | [`BulletedList`](text::BulletedList), [`Title`](text::Title), [`MarkupText`](text::MarkupText) (Pango tags; [`parse_markup`](text::parse_markup)) |
//! | updaters that step a simulation (`dt`) | [`Sim`](sim::Sim) `::precompute` in the constructor, `sample(t)` per frame |
//! | `axes.plot_line_graph`, `BarChart`, histograms, heat maps | [`plot`]: `LinePlot`, `ScatterPlot`, `Histogram` with `bin`, `Heatmap`, `plot::bars` |
//! | the math scenes (feature `math`): dots and intervals on a `NumberLine`, tangents, secants, Riemann sums, the unit circle, `LinearTransformationScene`, vector sums, the central limit theorem, `FourierCirclesScene` | `kits::math`: `NumberPoint`, `Interval`, `FunctionPlot`, `Tangent`, `Secant`, `RiemannSum`, `AreaUnder` (and `integral`), `UnitCircle`, `LinearTransform`, `VectorSum`, `Pythagoras`, `FractionBar`, `FractionPie`, `MatrixView`, `NormalFit`, `Epicycles` |
//! | manim-physics `Charge` (feature `physics`) | `kits::physics::Charge` (the data) drawn by `kits::physics::ChargeGlow` |
//! | manim-physics `ElectricField` (feature `physics`) | `kits::physics::ElectricField { charges, x_range, y_range, lines_per_charge }`, the charges a prop computed per frame to move them |
//! | manim-physics `Wire`, `MagneticField` (feature `physics`) | `kits::physics::Wire::{Through, Segment}`, `kits::physics::MagneticField` in the page (arrows, ⊙ and ⊗), `kits::physics::magnetic_field` |
//! | manim-physics `Lens`, `Ray` (feature `physics`) | `kits::physics::Lens::{thin, thick}` and `kits::physics::Ray` drawn on a `kits::physics::OpticalBench` (paraxial tracing, focal points); `kits::physics::RayDiagram` for the three principal rays |
//! | manim-physics `Space`, `SpaceScene` (feature `physics`) | `kits::physics::RigidMotion::new(bodies, gravity, seconds)` (rapier, run once) with `kits::physics::Body::{circle, rect, polygon, wall}`, drawn by `kits::physics::RigidWorld` (a `draw` closure per body optional) |
//! | manim-physics `MultiPendulum` (feature `physics`) | `kits::physics::MultiPendulumMotion::new(lengths, masses, angles, seconds)` solved once, drawn by `kits::physics::MultiPendulum` with a trail of the last bob |
//! | manim-physics `LinearWave`, `StandingWave` (feature `physics`) | `kits::physics::Wave { front: false, .. }` (speed = wavelength / period) and `Wave { .. }.harmonic(n)`, a 2D curve; `WaveKind::{Longitudinal, Superposition}` as well |
//! | manim-physics `RadialWave` (feature `physics`) | `kits::physics::RadialWave { sources, .. }`: crest and trough rings in 2D, the surface in a `three::ThreeDView` with the `3d` feature |
//! | manim-circuit `Resistor`, `Capacitor`, `Inductor`, `VoltageSource`, `CurrentSource`, `Ground`, `Opamp` (feature `physics`) | `kits::physics::Symbol { kind: SymbolKind::Resistor, .. }` and the other kinds, with `Battery`, `Switch`, `Bulb` and `Diode`: a label, a rotation and named terminals (`Symbol::terminal`) |
//! | manim-circuit wires between symbols (feature `physics`) | `kits::physics::Schematic { symbols, wires: vec![Connection::new((0, "b"), (1, "a")).via([..])] }`: right angles through given corners, junction dots, the current as moving dots |
//! | manim-chemistry `MMoleculeObject`, `ThreeDMolecule` from a file (feature `chemistry`) | `kits::chemistry::MolFile::parse` (V2000 `.mol`, the first record of an `.sdf`), drawn from `structure()` by `kits::chemistry::Molecule2D`, from `structure_3d()` by `kits::chemistry::Molecule3D` with `3d` |

// The derives expand to `::fframes_kit::..` paths, also on the kit's own components.
extern crate self as fframes_kit;

pub mod anim;
pub mod animate;
pub mod audio;
pub mod data;
pub mod decor;
pub mod effects;
pub mod explain;
pub mod geometry;
pub mod kits;
pub mod label;
#[cfg(feature = "latex")]
pub mod latex;
pub mod layout;
pub mod media;
pub mod motion;
pub mod overlay;
pub mod paths;
pub mod plot;
pub mod prelude;
pub mod props;
pub mod shapes;
pub mod sim;
pub mod text;
pub mod text_outline;
pub mod theme;
#[cfg(feature = "3d")]
pub mod three;
pub mod time;
pub mod transitions;
pub mod ui;

pub use fframes_kit_macros::ui;
pub use kurbo;

#[cfg(doctest)]
#[doc = include_str!("../../../docs/components/components.md")]
struct ComponentsDocs;

#[cfg(all(doctest, feature = "math"))]
#[doc = include_str!("../../../docs/components/math.md")]
struct MathDocs;

#[cfg(all(doctest, feature = "3d"))]
#[doc = include_str!("../../../docs/components/three-d.md")]
struct ThreeDDocs;

#[cfg(all(doctest, feature = "physics"))]
#[doc = include_str!("../../../docs/components/physics.md")]
struct PhysicsDocs;

#[cfg(all(doctest, feature = "biology"))]
#[doc = include_str!("../../../docs/components/biology.md")]
struct BiologyDocs;

#[cfg(all(doctest, feature = "chemistry"))]
#[doc = include_str!("../../../docs/components/chemistry.md")]
struct ChemistryDocs;
#[cfg(all(doctest, feature = "cs"))]
#[doc = include_str!("../../../docs/components/cs.md")]
struct CsDocs;
