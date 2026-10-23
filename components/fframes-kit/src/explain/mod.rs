//! The explain layer: Manim's coordinate system and its graphing mobjects.
//!
//! Explain components measure in units of [`Space`] with `y` up, like Manim: every `Ui`
//! starts in [`Space::FULL_HD`] (8 units tall, the origin in the middle of a 1920x1080
//! canvas) and a [`Canvas`] puts its children into another one. They draw in pixels, as
//! shapes, so `Create` and the other shape animations work on them.
//!
//! ```
//! use fframes_kit::prelude::*;
//!
//! fn graph<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
//!     let axes = Axes {
//!         x_range: Range::new(-4.0, 4.0, 1.0),
//!         y_range: Range::new(-1.5, 1.5, 0.5),
//!         x_length: 10.0,
//!         y_length: 4.0,
//!         enter: Some(Create::default().into()),
//!         ..ui.defaults()
//!     };
//!     let sine = FunctionGraph {
//!         enter: Some(Create { delay: 1.0, ..ui.defaults() }.into()),
//!         ..axes.plot(ui, f64::sin)
//!     };
//!     let label = axes.graph_label(ui, &sine, "sin x", 4.0, Anchor::Right);
//!     let axes = ui.show(axes);
//!     let sine = ui.show(sine);
//!     let label = ui.show(label);
//!     svgr!(<g>{axes}{sine}{label}</g>)
//! }
//! ```

mod axes;
mod canvas;
mod graph;
mod matrix;
mod space;
mod table;
mod vgroup;

pub use axes::{Axes, AxesMap, NumberLine, NumberPlane, Range};
pub use canvas::Canvas;
pub use graph::{
    Area, Curve, Func, FunctionGraph, ParametricFunction, RiemannRectangles, SampleAt, sample,
    sample_curve,
};
pub(crate) use graph::{graph_px, smooth};
pub use matrix::Matrix;
pub use space::Space;
pub use table::Table;
pub use vgroup::VGroup;
