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
//! | `linearTiming`, `springTiming` | [`Timing::Linear`](transitions::Timing::Linear), [`Timing::Spring`](transitions::Timing::Spring) |
//!
//! | Manim | fframes-kit |
//! | --- | --- |
//! | `FadeIn(m, shift=UP)` | [`FadeIn`](anim::FadeIn) `{ shift: (0.0, -40.0), .. }` (canvas units, `y` down) |
//! | `FadeOut(m)` | [`FadeOut`](anim::FadeOut) |
//! | `run_time`, `rate_func` | the same fields; [`RateFunc`](anim::RateFunc) has `smooth`, `linear`, `rush_into`, `rush_from`, `there_and_back`, `double_smooth` |
//! | `lag_ratio` | [`motion::stagger`], [`LaggedStart`](animate::LaggedStart), [`AnimationGroup`](animate::AnimationGroup), [`Succession`](animate::Succession) |
//! | `Circle`, `Square`, `Arrow`, `Brace`, ... (mobjects) | [`shapes`], placed by center in canvas pixels |
//! | `Text` as a `VMobject` | [`TextPath`](shapes::TextPath) (glyph outlines) |
//! | `Create`, `Uncreate`, `DrawBorderThenFill`, `Write` | [`animate`], in a shape's `enter` / `exit` prop |
//! | `GrowFromCenter`, `GrowFromPoint`, `GrowArrow`, `SpinInFromNothing` | [`animate`], `enter` |
//! | `Indicate`, `Circumscribe`, `Flash`, `Wiggle` | [`animate`], `emphasis` |
//! | `Transform`, `ReplacementTransform` | [`animate::Transform`] (`Morph` in the prelude), [`ReplacementTransform`](animate::ReplacementTransform) |
//! | `MoveAlongPath` | [`MoveAlongPath`](animate::MoveAlongPath) |

pub mod anim;
pub mod animate;
pub mod geometry;
pub mod motion;
pub mod paths;
pub mod prelude;
pub mod shapes;
pub mod text_outline;
pub mod theme;
pub mod time;
pub mod transitions;
pub mod ui;

pub use kurbo;

#[cfg(doctest)]
#[doc = include_str!("../../../docs/components/components.md")]
struct ComponentsDocs;
