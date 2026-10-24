//! Transitions, as in Remotion: [`TransitionSeries`] with the [`Fade`], [`Slide`], [`Wipe`],
//! [`Flip`], [`ClockWipe`] and [`Iris`] presentations, [`CircleWipe`] and [`SlantWipe`],
//! [`NoTransition`], [`PushCut`], [`ZoomInOut`], the blurred [`DreamyZoom`],
//! [`LinearBlur`], [`BlurSlide`] and [`CrossZoom`], [`Dissolve`], [`FilmBurn`] and
//! [`SwapCards`], and [`Timing`]s; [`TransitionProgress`] inside a sequence and
//! [`audio_fades`] for its sound; [`SceneFade`] for the edges of a sequence or scene, and
//! [`scene_transition`] for core scenes with an `Overlap`.

mod blur;
mod circle_wipe;
mod clock_wipe;
mod dissolve;
mod fade;
mod film_burn;
mod flip;
mod iris;
mod none;
mod presentation;
mod progress;
mod push_cut;
mod scene_fade;
mod series;
mod slant_wipe;
mod slide;
mod swap;
mod timing;
mod wipe;
mod zoom;

pub use blur::{BlurSlide, CrossZoom, DreamyZoom, LinearBlur};
pub use circle_wipe::{CircleStage, CircleWipe};
pub use clock_wipe::ClockWipe;
pub use dissolve::Dissolve;
pub use fade::Fade;
pub use film_burn::FilmBurn;
pub use flip::Flip;
pub use iris::Iris;
pub use none::NoTransition;
pub use presentation::{Direction, Presentation, Side};
pub(crate) use progress::TransitionState;
pub use progress::{AudioFade, SeriesLength, TransitionProgress, audio_fades};
pub use push_cut::PushCut;
pub use scene_fade::{SceneFade, scene_transition};
pub use series::{TransitionItem, TransitionSeries};
pub use slant_wipe::SlantWipe;
pub use slide::Slide;
pub use swap::SwapCards;
pub use timing::Timing;
pub use wipe::Wipe;
pub use zoom::ZoomInOut;
