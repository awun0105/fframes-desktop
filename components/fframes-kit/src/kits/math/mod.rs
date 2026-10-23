//! The math kit (feature `math`): figures built from the [`explain`](crate::explain) layer.
//!
//! The overlays take the mapping of the line or axes they belong to and draw only
//! themselves, so the line or axes are shown once: [`NumberPoint`], [`Interval`],
//! [`Tangent`], [`Secant`], [`RiemannSum`], [`AreaUnder`]. [`FunctionPlot`] draws its axes,
//! graph, label and tracer together. Animated values are [`ValueTracker`](crate::anim::ValueTracker)s.
//! `docs/components/math.md` has an example of each with its gallery scene.

mod calculus;
mod function_plot;
mod number_line;

pub use calculus::{AreaUnder, RiemannSum, Secant, Tangent, integral};
pub use function_plot::FunctionPlot;
pub use number_line::{Interval, NumberPoint};
