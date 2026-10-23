//! Plots of data on [`Axes`](crate::explain::Axes): [`LinePlot`], [`ScatterPlot`],
//! [`Histogram`], [`Heatmap`] and [`bars`], which places a [`BarChart`](crate::data::BarChart)
//! on the axes. Every plot takes the axes' mapping (`axes.map(ui)`) and draws only itself, so
//! the axes are shown once.
//!
//! ```
//! use fframes_kit::prelude::*;
//! use fframes_kit::plot::bin;
//!
//! fn plot<'a>(ui: &mut Ui<'a, '_, '_>, samples: &[f64]) -> Svgr<'a> {
//!     let axes = Axes {
//!         x_range: Range::new(-3.0, 3.0, 1.0),
//!         y_range: Range::new(0.0, 0.5, 0.1),
//!         ..ui.defaults()
//!     };
//!     let histogram = ui.show(Histogram {
//!         axes: axes.map(ui),
//!         values: bin(samples, (-3.0, 3.0), 24, true),
//!         range: (-3.0, 3.0),
//!         ..ui.defaults()
//!     });
//!     let axes = ui.show(axes);
//!     svgr!(<g>{axes}{histogram}</g>)
//! }
//! ```

mod bars;
mod heatmap;
mod histogram;
mod line;
mod scatter;

pub use bars::bars;
pub use heatmap::Heatmap;
pub use histogram::{Histogram, bin};
pub use line::LinePlot;
pub use scatter::ScatterPlot;
