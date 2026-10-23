//! Everything a video needs: `use fframes_kit::prelude::*;`.

pub use fframes::{
    AudioMap, Color, Duration, FFramesContext, FontQuery, Frame, Overlap, Scene, Scenes, Svgr,
    Transform, Video, animation::Easing, svgr, timeline,
};

pub use crate::anim::{Anim, Enter, Exit, FadeIn, FadeOut, RateFunc};
pub use crate::animate::{
    AnimationGroup, Circumscribe, Create, DrawBorderThenFill, Emphasis, EnterAnim, ExitAnim, Flash,
    GrowArrow, GrowFromCenter, GrowFromPoint, Indicate, LaggedStart, MoveAlongPath,
    ReplacementTransform, SpinInFromNothing, Succession, Transform as Morph, Uncreate, Wiggle,
    Write,
};
pub use crate::geometry::{Geometry, Look, Shape};
pub use crate::motion;
pub use crate::paths;
pub use crate::shapes::{
    Annulus, Arc, Arrow, BlockArrow, Brace, Callout, Circle, CurvedArrow, DashedLine, Dot,
    DoubleArrow, Ellipse, Heart, Line, Pie, Polygon, Rect, Rectangle, RegularPolygon,
    RoundedRectangle, Sector, Spark, Square, Star, TextPath, Triangle, Vector,
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
