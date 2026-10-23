//! Everything a video needs: `use fframes_kit::prelude::*;`.

pub use fframes::{
    AudioMap, Color, Duration, FFramesContext, FontQuery, Frame, Scene, Scenes, Svgr, Transform,
    Video, animation::Easing, svgr, timeline,
};

pub use crate::anim::{Anim, Enter, Exit, FadeIn, FadeOut, RateFunc};
pub use crate::motion;
pub use crate::theme::{
    Colors, Durations, Easings, Motion, Radius, Shadow, Shadows, Spacing, Stagger, Stroke,
    TextStyle, Theme, Typography,
};
pub use crate::time::{Fill, Freeze, Loop, Sequence, Series};
pub use crate::ui::{BoxedComponent, Children, Component, DynComponent, Themed, Ui, children};
