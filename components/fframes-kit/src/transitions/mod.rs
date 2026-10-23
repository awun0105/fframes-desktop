//! Transitions, as in Remotion: [`TransitionSeries`] with the [`Fade`], [`Slide`] and
//! [`Wipe`] presentations and [`Timing`]s, [`SceneFade`] for the edges of a sequence or
//! scene, and [`scene_transition`] for core scenes with an `Overlap`.

mod fade;
mod presentation;
mod scene_fade;
mod series;
mod slide;
mod timing;
mod wipe;

pub use fade::Fade;
pub use presentation::{Direction, Presentation, Side};
pub use scene_fade::{SceneFade, scene_transition};
pub use series::{TransitionItem, TransitionSeries};
pub use slide::Slide;
pub use timing::Timing;
pub use wipe::Wipe;
