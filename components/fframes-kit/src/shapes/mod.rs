//! Shapes: Manim's mobjects and Remotion's `@remotion/shapes`, each a component with its
//! geometry props, paint, and the `enter`, `exit` and `emphasis` animation props.
//!
//! Positions are canvas pixels with `y` down, angles radians. Shapes are placed by their
//! center (`cx`, `cy`) unless they are given by points (`Line`, `Arrow`, `Polygon`).
//! Manim shapes are outlines in the accent colour by default, Remotion shapes are filled.
//!
//! ```ignore
//! ui.show(Circle {
//!     cx: 960.0,
//!     cy: 540.0,
//!     radius: 200.0,
//!     enter: Some(Create::default().into()),
//!     ..ui.defaults()
//! })
//! ```

mod arrows;
mod basic;
mod brace;
mod remotion;

pub use crate::text_outline::TextPath;
pub use arrows::{Arrow, CurvedArrow, DoubleArrow, Vector};
pub use basic::{
    Annulus, Arc, Circle, DashedLine, Dot, Ellipse, Line, Polygon, Rectangle, RegularPolygon,
    RoundedRectangle, Sector, Square, Star, Triangle,
};
pub use brace::Brace;
pub use remotion::{BlockArrow, Callout, Heart, Pie, Rect, Spark};

use fframes::Color;
use kurbo::Point;

use crate::theme::Theme;

/// Flattening tolerance for circles and arcs, in pixels.
pub(crate) const TOLERANCE: f64 = 0.05;

/// The default paint of a shape family.
pub(crate) struct Paint {
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
}

impl Paint {
    /// Manim: an accent outline.
    pub fn outline(t: &Theme) -> Self {
        Self {
            fill: None,
            stroke: Some(t.colors.accent),
            stroke_width: t.stroke.regular,
        }
    }

    /// Remotion, and Manim's filled mobjects (`Dot`, `Annulus`, `Sector`).
    pub fn solid(t: &Theme) -> Self {
        Self {
            fill: Some(t.colors.accent),
            stroke: None,
            stroke_width: t.stroke.regular,
        }
    }

    /// Arrows: the shaft is an outline, the tip is filled, both in the accent colour.
    pub fn arrow(t: &Theme) -> Self {
        Self {
            fill: Some(t.colors.accent),
            stroke: Some(t.colors.accent),
            stroke_width: t.stroke.regular,
        }
    }
}

pub(crate) fn pt(x: f32, y: f32) -> Point {
    Point::new(f64::from(x), f64::from(y))
}

/// A shape component: the geometry fields given here plus `fill`, `stroke`, `stroke_width`,
/// `dash`, `opacity`, `enter`, `exit` and `emphasis`, with `Themed`, `Default`, `Geometry`
/// and `Component` impls. `path(s)` builds the outline from the props `s`.
macro_rules! shape {
    (@dash) => {
        None
    };
    (@dash $dash:expr) => {
        Some($dash)
    };
    (
        $(#[$meta:meta])*
        $name:ident($t:ident) {
            $($(#[$fmeta:meta])* $field:ident: $ty:ty = $default:expr,)*
        }
        paint: $paint:expr;
        $(dash: $dash:expr;)?
        path($s:ident) $path:block
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            $($(#[$fmeta])* pub $field: $ty,)*
            /// `None` leaves the inside empty.
            pub fill: Option<fframes::Color>,
            /// `None` draws no outline.
            pub stroke: Option<fframes::Color>,
            pub stroke_width: f32,
            /// Dash and gap lengths of the outline.
            pub dash: Option<(f32, f32)>,
            pub opacity: f32,
            pub enter: Option<$crate::animate::EnterAnim>,
            pub exit: Option<$crate::animate::ExitAnim>,
            pub emphasis: Option<$crate::animate::Emphasis>,
        }

        impl $crate::ui::Themed for $name {
            #[allow(unused_variables)]
            fn themed($t: &$crate::theme::Theme) -> Self {
                let paint: $crate::shapes::Paint = $paint;
                Self {
                    $($field: $default,)*
                    fill: paint.fill,
                    stroke: paint.stroke,
                    stroke_width: paint.stroke_width,
                    dash: $crate::shapes::shape!(@dash $($dash)?),
                    opacity: 1.0,
                    enter: None,
                    exit: None,
                    emphasis: None,
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                <Self as $crate::ui::Themed>::themed(&$crate::theme::Theme::default())
            }
        }

        impl $name {
            /// The path and paint as a [`Shape`]($crate::geometry::Shape).
            pub fn shape(&self) -> $crate::geometry::Shape {
                $crate::geometry::Shape {
                    path: $crate::geometry::Geometry::path(self),
                    fill: self.fill,
                    stroke: self.stroke,
                    stroke_width: self.stroke_width,
                    dash: self.dash,
                    opacity: self.opacity,
                }
            }
        }

        impl $crate::geometry::Geometry for $name {
            fn path(&self) -> kurbo::BezPath {
                let $s = self;
                $path
            }
        }

        impl<'a> $crate::ui::Component<'a> for $name {
            fn render(self, ui: &mut $crate::ui::Ui<'a, '_, '_>) -> fframes::Svgr<'a> {
                $crate::animate::draw_animated(
                    ui,
                    &[self.shape()],
                    self.enter.as_ref(),
                    self.exit.as_ref(),
                    self.emphasis.as_ref(),
                )
            }
        }
    };
}
pub(crate) use shape;

#[cfg(test)]
pub(crate) fn assert_bbox(g: &impl crate::geometry::Geometry, x0: f64, y0: f64, x1: f64, y1: f64) {
    let b = g.bbox();
    let close = |a: f64, b: f64| (a - b).abs() < 0.5;
    assert!(
        close(b.x0, x0) && close(b.y0, y0) && close(b.x1, x1) && close(b.y1, y1),
        "bbox {b:?} is not ({x0}, {y0}, {x1}, {y1})"
    );
}
