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
mod camera;
mod canvas;
mod complex_plane;
mod graph;
mod graph_mobject;
mod implicit;
mod matrix;
mod polar_plane;
mod sample_space;
mod space;
mod stream_lines;
mod table;
mod typed_matrix;
mod typed_table;
mod vector_field;
mod vgroup;

pub use axes::{Axes, AxesMap, NumberLine, NumberPlane, Range, Scaling};
pub use camera::{MovingCamera, ZoomedInset, auto_zoom, full_frame_width};
pub use canvas::Canvas;
pub use complex_plane::{ComplexPlane, format_complex};
pub use graph::{
    Area, Curve, Func, FunctionGraph, ParametricFunction, RiemannRectangles, SampleAt, sample,
    sample_curve, sample_scaled,
};
pub(crate) use graph::{graph_px, smooth};
pub use graph_mobject::{DiGraph, EdgeStyle, Graph, VertexStyle};
pub use implicit::{Implicit, ImplicitFunction, march};
pub use matrix::Matrix;
pub use polar_plane::{AzimuthDirection, AzimuthUnits, PolarPlane};
pub use sample_space::{SampleColumn, SampleRow, SampleSpace};
pub use space::Space;
pub use stream_lines::{Flow, StreamLines};
pub use table::Table;
pub use typed_matrix::{DecimalMatrix, IntegerMatrix, Mobject, MobjectMatrix};
#[cfg(feature = "latex")]
pub use typed_table::MathTable;
pub use typed_table::{DecimalTable, IntegerTable, MobjectTable};
pub use vector_field::{ArrowVectorField, Field, VectorField, sigmoid_length};
pub use vgroup::VGroup;
