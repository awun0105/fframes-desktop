//! Manim's marked points and labelled shapes: `AnnotationDot`, `LabeledDot`, `LabeledLine`
//! and `LabeledArrow`. A label is text in a font from the video's media or, with the `latex`
//! feature, a typeset formula; it is drawn as outlines, so `Create` and `Write` draw it with
//! the shape.

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Rect, Shape as _};

use super::{
    Paint, SMALL_BUFF, TOLERANCE,
    arrows::{ArrowTip, arrow_parts, tipped},
    polygram::{polylabel, ring_path, rings},
    pt, shape,
};
use crate::{
    animate::{Emphasis, EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    text_outline::TextPath,
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// What a labelled shape shows.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ShapeLabel<'a> {
    #[default]
    None,
    /// Text in `label_style`, as glyph outlines.
    Text(&'a str),
    /// A typeset formula, `label_style.size` pixels for 10 pt.
    #[cfg(feature = "latex")]
    Formula(&'a crate::latex::Formula),
}

impl<'a> From<&'a str> for ShapeLabel<'a> {
    fn from(text: &'a str) -> Self {
        Self::Text(text)
    }
}

#[cfg(feature = "latex")]
impl<'a> From<&'a crate::latex::Formula> for ShapeLabel<'a> {
    fn from(formula: &'a crate::latex::Formula) -> Self {
        Self::Formula(formula)
    }
}

/// The animation props a labelled shape hands to its label.
#[derive(Clone, Copy)]
pub(super) struct Anims<'p> {
    pub enter: Option<&'p EnterAnim>,
    pub exit: Option<&'p ExitAnim>,
    pub emphasis: Option<&'p Emphasis>,
}

/// A label centered on a point: its outlines and box, or, when the font or formula has no
/// outlines, the element that draws it instead.
pub(super) struct Placed<'a> {
    parts: Vec<Shape>,
    bounds: Option<Rect>,
    fallback: Option<Svgr<'a>>,
}

pub(super) fn place<'a>(
    ui: &mut Ui<'a, '_, '_>,
    label: ShapeLabel<'a>,
    style: TextStyle,
    color: Color,
    at: Point,
    anims: Anims<'_>,
) -> Placed<'a> {
    match label {
        ShapeLabel::None => Placed {
            parts: Vec::new(),
            bounds: None,
            fallback: None,
        },
        ShapeLabel::Text(text) => {
            let probe = TextPath {
                text,
                style,
                fill: Some(color),
                stroke: None,
                enter: anims.enter.copied(),
                exit: anims.exit.copied(),
                emphasis: anims.emphasis.copied(),
                ..ui.defaults()
            };
            let b = probe.bounds(ui.ctx);
            let word = TextPath {
                x: (at.x - b.center().x) as f32,
                y: (at.y - b.center().y) as f32,
                ..probe
            };
            let bounds = Some(b + (at - b.center()));
            match word.parts(ui.ctx) {
                Some(parts) => Placed {
                    parts,
                    bounds,
                    fallback: None,
                },
                None => Placed {
                    parts: Vec::new(),
                    bounds,
                    fallback: Some(ui.show(word)),
                },
            }
        }
        #[cfg(feature = "latex")]
        ShapeLabel::Formula(formula) => {
            let view = crate::latex::FormulaView {
                size: style.size as f32,
                x: at.x as f32,
                y: at.y as f32,
                anchor: crate::layout::Anchor::Center,
                color,
                enter: anims.enter.copied(),
                exit: anims.exit.copied(),
                emphasis: anims.emphasis.copied(),
                ..crate::latex::FormulaView::new(formula)
            };
            let parts = view.shapes();
            let bounds = Some(view.bounds());
            if parts.is_empty() {
                Placed {
                    parts,
                    bounds,
                    fallback: Some(ui.show(view)),
                }
            } else {
                Placed {
                    parts,
                    bounds,
                    fallback: None,
                }
            }
        }
    }
}

/// `shapes`, then a box behind the label when `boxed`, then the label, animated as one.
pub(super) fn draw_labeled<'a>(
    ui: &mut Ui<'a, '_, '_>,
    shapes: Vec<Shape>,
    label: Placed<'a>,
    boxed: bool,
    anims: Anims<'_>,
) -> Svgr<'a> {
    let opacity = shapes.first().map_or(1.0, |s| s.opacity);
    let mut parts = shapes;
    if let (true, Some(b)) = (boxed, label.bounds) {
        let pad = f64::from(SMALL_BUFF) / 2.0;
        parts.push(Shape {
            path: b.inflate(pad, pad).to_path(TOLERANCE),
            fill: Some(ui.theme.colors.background),
            stroke: None,
            stroke_width: 0.0,
            dash: None,
            opacity,
        });
    }
    parts.extend(label.parts);
    let drawn = draw_animated(ui, &parts, anims.enter, anims.exit, anims.emphasis);
    match label.fallback {
        Some(fallback) => fframes::svgr!(<g>{drawn}{fallback}</g>),
        None => drawn,
    }
}

shape! {
    /// A dot that marks a point for an annotation (Manim `AnnotationDot`): a third larger than
    /// a `Dot`, filled in the accent colour with a light outline.
    AnnotationDot(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        radius: f32 = 10.4,
    }
    paint: Paint { fill: Some(t.colors.accent), stroke: Some(t.colors.title), ..Paint::outline(t) };
    path(s) {
        kurbo::Circle::new(pt(s.cx, s.cy), f64::from(s.radius)).to_path(TOLERANCE)
    }
}

/// A dot with a label inside (Manim `LabeledDot`). Without a `radius` it is
/// [`SMALL_BUFF`] larger than half the label's longer side, as in Manim.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LabeledDot<'a> {
    pub label: ShapeLabel<'a>,
    pub cx: f32,
    pub cy: f32,
    pub radius: Option<f32>,
    pub label_style: TextStyle,
    pub label_color: Color,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
    pub opacity: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub emphasis: Option<Emphasis>,
}

impl Themed for LabeledDot<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            label: ShapeLabel::None,
            cx: 0.0,
            cy: 0.0,
            radius: None,
            label_style: t.typography.body,
            label_color: t.colors.title,
            fill: Some(t.colors.accent),
            stroke: None,
            stroke_width: t.stroke.regular,
            opacity: 1.0,
            enter: None,
            exit: None,
            emphasis: None,
        }
    }
}

impl Default for LabeledDot<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for LabeledDot<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let anims = Anims {
            enter: self.enter.as_ref(),
            exit: self.exit.as_ref(),
            emphasis: self.emphasis.as_ref(),
        };
        let at = pt(self.cx, self.cy);
        let label = place(
            ui,
            self.label,
            self.label_style,
            self.label_color,
            at,
            anims,
        );
        let fit = label
            .bounds
            .map_or(0.0, |b| b.width().max(b.height()) / 2.0);
        let radius = self.radius.map_or(fit + f64::from(SMALL_BUFF), f64::from);
        let dot = Shape {
            path: kurbo::Circle::new(at, radius).to_path(TOLERANCE),
            fill: self.fill,
            stroke: self.stroke,
            stroke_width: self.stroke_width,
            dash: None,
            opacity: self.opacity,
        };
        draw_labeled(ui, vec![dot], label, false, anims)
    }
}

/// The props `LabeledLine` and `LabeledArrow` share, with their defaults.
macro_rules! labeled_line {
    (
        $(#[$meta:meta])*
        $name:ident($t:ident) {
            $($(#[$fmeta:meta])* $field:ident: $ty:ty = $default:expr,)*
        }
        paint: $paint:expr;
        path($s:ident) $path:block
        parts($ps:ident, $shape:ident) $parts:block
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        #[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
        pub struct $name<'a> {
            pub label: ShapeLabel<'a>,
            pub x1: f32,
            pub y1: f32,
            pub x2: f32,
            pub y2: f32,
            $($(#[$fmeta])* pub $field: $ty,)*
            /// Where along the line the label sits, 0 at the start to 1 at the end.
            pub label_position: f32,
            pub label_style: TextStyle,
            pub label_color: Color,
            /// A box in the background colour behind the label, so the line does not cross it.
            pub label_box: bool,
            pub fill: Option<Color>,
            pub stroke: Option<Color>,
            pub stroke_width: f32,
            pub opacity: f32,
            pub enter: Option<EnterAnim>,
            pub exit: Option<ExitAnim>,
            pub emphasis: Option<Emphasis>,
        }

        impl Themed for $name<'_> {
            fn themed($t: &Theme) -> Self {
                let paint: Paint = $paint;
                Self {
                    label: ShapeLabel::None,
                    x1: 0.0,
                    y1: 0.0,
                    x2: 200.0,
                    y2: 0.0,
                    $($field: $default,)*
                    label_position: 0.5,
                    label_style: $t.typography.body,
                    label_color: $t.colors.title,
                    label_box: true,
                    fill: paint.fill,
                    stroke: paint.stroke,
                    stroke_width: paint.stroke_width,
                    opacity: 1.0,
                    enter: None,
                    exit: None,
                    emphasis: None,
                }
            }
        }

        impl Default for $name<'_> {
            fn default() -> Self {
                Self::themed(&Theme::default())
            }
        }

        impl $name<'_> {
            /// The line or arrow without its label.
            pub fn path(&self) -> BezPath {
                let $s = self;
                $path
            }
        }

        impl<'a> Component<'a> for $name<'a> {
            fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
                let anims = Anims {
                    enter: self.enter.as_ref(),
                    exit: self.exit.as_ref(),
                    emphasis: self.emphasis.as_ref(),
                };
                let (from, to) = (pt(self.x1, self.y1), pt(self.x2, self.y2));
                let at = from.lerp(to, f64::from(self.label_position));
                let label = place(ui, self.label, self.label_style, self.label_color, at, anims);
                let line = Shape {
                    path: self.path(),
                    fill: self.fill,
                    stroke: self.stroke,
                    stroke_width: self.stroke_width,
                    dash: None,
                    opacity: self.opacity,
                };
                let parts = {
                    let ($ps, $shape) = (&self, line);
                    $parts
                };
                draw_labeled(ui, parts, label, self.label_box, anims)
            }
        }
    };
}

labeled_line! {
    /// A line with a label on it (Manim `LabeledLine`).
    LabeledLine(t) {}
    paint: Paint::outline(t);
    path(s) {
        let mut path = BezPath::new();
        path.move_to(pt(s.x1, s.y1));
        path.line_to(pt(s.x2, s.y2));
        path
    }
    parts(_s, line) {
        vec![line]
    }
}

labeled_line! {
    /// An arrow with a label on it (Manim `LabeledArrow`).
    LabeledArrow(t) {
        tip_length: f32 = 24.0,
        tip_width: f32 = 20.0,
        tip_start: Option<ArrowTip> = None,
        tip_end: Option<ArrowTip> = Some(ArrowTip::TriangleFilled),
    }
    paint: Paint::arrow(t);
    path(s) {
        tipped(
            pt(s.x1, s.y1),
            pt(s.x2, s.y2),
            f64::from(s.tip_length),
            f64::from(s.tip_width),
            s.tip_start,
            s.tip_end,
        )
    }
    parts(s, arrow) {
        arrow_parts(arrow, s.tip_start, s.tip_end, true)
    }
}

/// Closed polygons with a label where it fits best (Manim `LabeledPolygram`): at the point
/// inside furthest from every edge, found to within `precision` pixels.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LabeledPolygram<'a> {
    pub vertex_groups: Vec<Vec<(f32, f32)>>,
    pub label: ShapeLabel<'a>,
    pub precision: f32,
    pub label_style: TextStyle,
    pub label_color: Color,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
    pub opacity: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub emphasis: Option<Emphasis>,
}

impl Themed for LabeledPolygram<'_> {
    fn themed(t: &Theme) -> Self {
        let paint = Paint::outline(t);
        Self {
            vertex_groups: Vec::new(),
            label: ShapeLabel::None,
            precision: 1.0,
            label_style: t.typography.body,
            label_color: t.colors.title,
            fill: paint.fill,
            stroke: paint.stroke,
            stroke_width: paint.stroke_width,
            opacity: 1.0,
            enter: None,
            exit: None,
            emphasis: None,
        }
    }
}

impl Default for LabeledPolygram<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl LabeledPolygram<'_> {
    /// Where the label goes: the inside point furthest from the edges.
    pub fn label_point(&self) -> Option<(f32, f32)> {
        let rings = rings(&self.vertex_groups);
        polylabel(&rings, f64::from(self.precision)).map(|p| (p.x as f32, p.y as f32))
    }
}

impl<'a> Component<'a> for LabeledPolygram<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let anims = Anims {
            enter: self.enter.as_ref(),
            exit: self.exit.as_ref(),
            emphasis: self.emphasis.as_ref(),
        };
        let at = self.label_point().map_or(Point::ZERO, |(x, y)| pt(x, y));
        let label = place(
            ui,
            self.label,
            self.label_style,
            self.label_color,
            at,
            anims,
        );
        let shape = Shape {
            path: ring_path(&rings(&self.vertex_groups)),
            fill: self.fill,
            stroke: self.stroke,
            stroke_width: self.stroke_width,
            dash: None,
            opacity: self.opacity,
        };
        draw_labeled(ui, vec![shape], label, false, anims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::assert_bbox;

    #[test]
    fn annotation_dot_is_a_third_larger_than_a_dot() {
        let d = AnnotationDot::default();
        assert_bbox(&d, -10.4, -10.4, 10.4, 10.4);
        assert!(d.fill.is_some() && d.stroke.is_some());
    }

    #[test]
    fn labeled_lines_keep_their_path() {
        let line = LabeledLine {
            label: "x".into(),
            ..LabeledLine::default()
        };
        assert_eq!(line.path().elements().len(), 2);
        let arrow = LabeledArrow::default();
        // shaft and tip
        assert_eq!(crate::paths::get_subpaths(&arrow.path()).len(), 2);
    }

    #[test]
    fn labeled_polygram_puts_the_label_inside() {
        let square = LabeledPolygram {
            vertex_groups: vec![vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]],
            ..LabeledPolygram::default()
        };
        let (x, y) = square.label_point().unwrap();
        assert!((x - 50.0).abs() < 1.0 && (y - 50.0).abs() < 1.0, "{x} {y}");
    }
}
