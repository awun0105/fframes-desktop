//! The math kit (feature `math`): figures built from the [`explain`](crate::explain) layer.
//!
//! The overlays take the mapping of the line or axes they belong to and draw only
//! themselves, so the line or axes are shown once: [`NumberPoint`], [`Interval`],
//! [`Tangent`], [`Secant`], [`RiemannSum`], [`AreaUnder`]. [`FunctionPlot`] draws its axes,
//! graph, label and tracer together. [`UnitCircle`], [`LinearTransform`], [`VectorSum`] and
//! [`Pythagoras`] are whole figures placed in units of `ui.space`, and so is [`Epicycles`],
//! the chain of circles that draws a path from its [`fourier`] series. [`FractionBar`],
//! [`FractionPie`] and [`MatrixView`] are placed in pixels. [`NormalFit`] draws seeded
//! [`normal_samples`] on axes with the [`plot`](crate::plot) module. Animated values are
//! [`ValueTracker`](crate::anim::ValueTracker)s.
//! `docs/components/math.md` has an example of each with its gallery scene.

mod calculus;
mod epicycles;
mod fraction;
mod function_plot;
mod linear;
mod matrix_view;
mod normal;
mod number_line;
mod pythagoras;
mod unit_circle;

pub use calculus::{AreaUnder, RiemannSum, Secant, Tangent, integral};
pub use epicycles::{Epicycles, Term, fourier, tip};
pub use fraction::{FractionBar, FractionPie};
pub use function_plot::FunctionPlot;
pub use linear::{LinearTransform, Mat2, VectorSum, apply, towards};
pub use matrix_view::{MatrixView, product};
pub use normal::{NormalFit, mean_sd, normal_pdf, normal_samples};
pub use number_line::{Interval, NumberPoint};
pub use pythagoras::Pythagoras;
pub use unit_circle::UnitCircle;

pub(crate) use super::short;
