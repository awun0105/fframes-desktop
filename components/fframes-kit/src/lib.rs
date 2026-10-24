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
//! | `ArcBetweenPoints`, `CurvedDoubleArrow`, `TangentialArc`, `CubicBezier`, `AnnularSector`, `ArcPolygon`, `ArcPolygonFromArcs` | the same names in [`shapes`] |
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
//! | `GrowFromCenter`, `GrowFromPoint`, `GrowArrow`, `SpinInFromNothing` | [`animate`], `enter` |
//! | `Indicate`, `Circumscribe`, `Flash`, `Wiggle` | [`animate`], `emphasis` |
//! | `Transform`, `ReplacementTransform` | [`animate::Transform`] (`Morph` in the prelude), [`ReplacementTransform`](animate::ReplacementTransform) |
//! | `MoveAlongPath` | [`MoveAlongPath`](animate::MoveAlongPath) |
//! | `MathTex`, `Tex`, `TexTemplate` (feature `latex`) | `latex::MathTex`, `latex::Tex`, `latex::TexTemplate`, typeset with a `latex::TexEngine` and drawn by `latex::FormulaView` |
//! | `TransformMatchingTex` (feature `latex`) | `latex::TransformMatchingTex` |
//! | the frame: 8 units tall, `y` up, origin in the middle | [`Space`](explain::Space) in `ui.space`, [`Canvas`](explain::Canvas) for another one |
//! | `NumberLine`, `Axes`, `NumberPlane` | [`explain`]: the same names, `x_range: Range::new(min, max, step)` |
//! | `axes.c2p`, `p2c`, `plot`, `i2gp`, `get_vertical_line`, `get_graph_label`, `get_riemann_rectangles`, `get_area` | `Axes::c2p`, `p2c`, `plot`, `FunctionGraph::point`, `Axes::vertical_line`, `graph_label`, `riemann_rectangles`, `area` |
//! | `m.next_to(point, UR)` | [`Label`](label::Label) `{ direction: Anchor::TopRight, .. }` |
//! | `ParametricFunction` | [`ParametricFunction`](explain::ParametricFunction) |
//! | `VGroup(..).arrange(RIGHT, buff)`, `arrange_in_grid` | [`VGroup`](explain::VGroup) `::arrange`, `::arrange_in_grid`; boxes alone with [`layout::arrange`], `row`, `column`, `grid` |
//! | `Matrix`, `Table`, `BraceLabel` | [`Matrix`](explain::Matrix), [`Table`](explain::Table), [`BraceLabel`](label::BraceLabel) |
//! | `ValueTracker` with `always_redraw` | [`ValueTracker`](anim::ValueTracker), a `timeline!` read with `ui.sample` inside `render` |
//! | updaters that step a simulation (`dt`) | [`Sim`](sim::Sim) `::precompute` in the constructor, `sample(t)` per frame |
//! | `axes.plot_line_graph`, `BarChart`, histograms, heat maps | [`plot`]: `LinePlot`, `ScatterPlot`, `Histogram` with `bin`, `Heatmap`, `plot::bars` |
//! | the math scenes (feature `math`): dots and intervals on a `NumberLine`, tangents, secants, Riemann sums, the unit circle, `LinearTransformationScene`, vector sums, the central limit theorem, `FourierCirclesScene` | `kits::math`: `NumberPoint`, `Interval`, `FunctionPlot`, `Tangent`, `Secant`, `RiemannSum`, `AreaUnder` (and `integral`), `UnitCircle`, `LinearTransform`, `VectorSum`, `Pythagoras`, `FractionBar`, `FractionPie`, `MatrixView`, `NormalFit`, `Epicycles` |

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
