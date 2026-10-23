//! Everything a video needs: `use fframes_kit::prelude::*;`.

pub use fframes::{
    AudioMap, Color, Duration, FFramesContext, FontQuery, Frame, Overlap, Scene, Scenes, Svgr,
    TextAlign, Transform, Video, animation::Easing, svgr, timeline,
};

pub use crate::anim::{Anim, Enter, Exit, FadeIn, FadeOut, RateFunc, ValueTracker};
pub use crate::animate::{
    AnimationGroup, Circumscribe, Create, DrawBorderThenFill, Emphasis, EnterAnim, ExitAnim, Flash,
    GrowArrow, GrowFromCenter, GrowFromPoint, Indicate, LaggedStart, MoveAlongPath,
    ReplacementTransform, SpinInFromNothing, Succession, Transform as Morph, Uncreate, Wiggle,
    Write,
};
pub use crate::audio::{self, BarAlign, SpectrumBars, Waveform};
pub use crate::data::BarChart;
pub use crate::decor::{CornerMarks, DrawOn, DropShadow, Glow, PhotoFrame};
pub use crate::explain::{
    Area, Axes, AxesMap, Canvas, Curve, Func, FunctionGraph, Matrix, NumberLine, NumberPlane,
    ParametricFunction, Range, RiemannRectangles, SampleAt, Space, Table, VGroup,
};
pub use crate::geometry::{Geometry, Look, Shape};
#[cfg(feature = "math")]
pub use crate::kits::math::{
    AreaUnder, FunctionPlot, Interval, NumberPoint, RiemannSum, Secant, Tangent,
};
pub use crate::label::{BraceLabel, Label, Leader};
#[cfg(feature = "latex")]
pub use crate::latex::{
    ChemEq, Formula, FormulaView, MathTex, Part, Tex, TexEngine, TexError, TexTemplate,
    TransformMatchingTex,
};
pub use crate::layout::Anchor;
pub use crate::media::KenBurns;
pub use crate::motion::{self, Beat};
pub use crate::overlay::{
    Annotation, Appear, Caption, CaptionStack, Captions, ChapterList, Check, LowerThird,
    ProgressBar,
};
pub use crate::paths;
pub use crate::shapes::{
    Annulus, Arc, Arrow, BlockArrow, Brace, Callout, Circle, CurvedArrow, DashedLine, Dot,
    DoubleArrow, Ellipse, Heart, Line, Pie, Polygon, Rect, Rectangle, RegularPolygon,
    RoundedRectangle, Sector, Spark, Square, Star, TextPath, Triangle, Vector,
};
pub use crate::sim::{Lerp, Sim};
pub use crate::text::{
    Counter, FitText, HudLabel, LetterLook, Letters, Paragraph, RevealBy, TextAnchor, TextReveal,
    Typewriter,
};
pub use crate::theme::{
    Colors, Durations, Easings, Motion, Radius, Shadow, Shadows, Spacing, Stagger, Stroke,
    TextStyle, Theme, Typography,
};
pub use crate::time::{Fill, Freeze, Loop, Sequence, Series};
pub use crate::transitions::{
    Direction, Fade, Presentation, SceneFade, Side, Slide, Timing, TransitionItem,
    TransitionSeries, Wipe, scene_transition,
};
pub use crate::ui::{BoxedComponent, Children, Component, DynComponent, Themed, Ui, children};
pub use kurbo::{BezPath, Point};
