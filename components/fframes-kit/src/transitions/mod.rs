//! Transitions, as in Remotion: [`TransitionSeries`] with the [`Fade`], [`Slide`], [`Wipe`],
//! [`Flip`], [`ClockWipe`] and [`Iris`] presentations, [`CircleWipe`] and [`SlantWipe`],
//! and [`Timing`]s, [`SceneFade`] for the edges of a sequence or
//! scene, and [`scene_transition`] for core scenes with an `Overlap`.

mod circle_wipe;
mod clock_wipe;
mod fade;
mod flip;
mod iris;
mod presentation;
mod scene_fade;
mod series;
mod slant_wipe;
mod slide;
mod timing;
mod wipe;

pub use circle_wipe::{CircleStage, CircleWipe};
pub use clock_wipe::ClockWipe;
pub use fade::Fade;
pub use flip::Flip;
pub use iris::Iris;
pub use presentation::{Direction, Presentation, Side};
pub use scene_fade::{SceneFade, scene_transition};
pub use series::{TransitionItem, TransitionSeries};
pub use slant_wipe::SlantWipe;
pub use slide::Slide;
pub use timing::Timing;
pub use wipe::Wipe;
