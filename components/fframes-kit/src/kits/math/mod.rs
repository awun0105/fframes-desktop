//! The math kit (feature `math`): figures built from the [`explain`](crate::explain) layer.
//!
//! The overlays take the mapping of the line or axes they belong to and draw only
//! themselves, so the line or axes are shown once: [`NumberPoint`], [`Interval`],
//! [`Tangent`], [`Secant`], [`RiemannSum`], [`AreaUnder`]. [`FunctionPlot`] draws its axes,
//! graph, label and tracer together. [`UnitCircle`], [`LinearTransform`], [`VectorSum`] and
//! [`Pythagoras`] are whole figures placed in units of `ui.space`. [`FractionBar`],
//! [`FractionPie`] and [`MatrixView`] are placed in pixels. Animated values are
//! [`ValueTracker`](crate::anim::ValueTracker)s.
//! `docs/components/math.md` has an example of each with its gallery scene.

mod calculus;
mod fraction;
mod function_plot;
mod linear;
mod matrix_view;
mod number_line;
mod pythagoras;
mod unit_circle;

pub use calculus::{AreaUnder, RiemannSum, Secant, Tangent, integral};
pub use fraction::{FractionBar, FractionPie};
pub use function_plot::FunctionPlot;
pub use linear::{LinearTransform, Mat2, VectorSum, apply, towards};
pub use matrix_view::{MatrixView, product};
pub use number_line::{Interval, NumberPoint};
pub use pythagoras::Pythagoras;
pub use unit_circle::UnitCircle;

/// `v` with at most `max_decimals` decimals and no trailing zeros (`0.5`, `2`, `-1.25`).
pub(crate) fn short(v: f64, max_decimals: usize) -> String {
    let s = format!("{v:.max_decimals$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        &s
    };
    if s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn short_numbers_drop_trailing_zeros() {
        assert_eq!(super::short(0.5, 2), "0.5");
        assert_eq!(super::short(2.0, 2), "2");
        assert_eq!(super::short(-0.001, 2), "0");
        assert_eq!(super::short(60.0, 0), "60");
        assert_eq!(super::short(-1.257, 2), "-1.26");
    }
}
