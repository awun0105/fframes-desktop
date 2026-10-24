//! Manim's `SVGMobject` and `VMobjectFromSVGPath`: shapes read from SVG.

use std::{fmt, sync::Arc};

use fframes::{
    Color, Svgr,
    diagnostics::{Diagnostic, report},
    usvgr::{self, Node, fontdb},
};
use kurbo::{Affine, BezPath, Rect, Shape as _};

use super::{Paint, shape};
use crate::{
    animate::{Emphasis, EnterAnim, ExitAnim, draw_animated},
    geometry::{Geometry, Shape},
    paths,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// SVG that does not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgError(pub String);

impl fmt::Display for SvgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the SVG does not parse: {}", self.0)
    }
}

impl std::error::Error for SvgError {}

/// One drawn path of an SVG file, in the file's units.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgPart {
    /// The path's `id`, else the `id` of the nearest group around it, else empty.
    pub id: String,
    pub path: BezPath,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
}

/// What a file holds that is not drawn as it is, counted while it is read.
#[derive(Debug, Default)]
struct Unsupported {
    text: usize,
    images: usize,
    gradients: usize,
    patterns: usize,
}

impl Unsupported {
    fn messages(&self) -> Vec<String> {
        [
            (
                self.text,
                "text elements are not drawn (convert text to paths)",
            ),
            (self.images, "images are not drawn"),
            (
                self.gradients,
                "gradients are drawn in their first stop's colour",
            ),
            (self.patterns, "patterns are drawn black"),
        ]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, what)| format!("SvgShape: {n} {what}"))
        .collect()
    }
}

/// A solid paint as a color; a gradient takes the color of its first stop, a pattern is black.
fn color(paint: &usvgr::Paint, opacity: f32, seen: &mut Unsupported) -> Color {
    let first_stop = |stops: &[usvgr::Stop]| {
        stops.first().map_or((usvgr::Color::black(), 1.0), |s| {
            (s.color(), s.opacity().get())
        })
    };
    let (c, stop) = match paint {
        usvgr::Paint::Color(c) => (*c, 1.0),
        usvgr::Paint::LinearGradient(g) => {
            seen.gradients += 1;
            first_stop(g.stops())
        }
        usvgr::Paint::RadialGradient(g) => {
            seen.gradients += 1;
            first_stop(g.stops())
        }
        usvgr::Paint::Pattern(_) => {
            seen.patterns += 1;
            (usvgr::Color::black(), 1.0)
        }
    };
    let alpha = (opacity * stop).clamp(0.0, 1.0) * 255.0;
    Color::rgba(c.red, c.green, c.blue, alpha.round() as u8)
}

fn walk(group: &usvgr::Group, id: &str, out: &mut Vec<SvgPart>, seen: &mut Unsupported) {
    let id = if group.id().is_empty() {
        id
    } else {
        group.id()
    };
    for node in group.children() {
        match node {
            Node::Group(g) => walk(g, id, out, seen),
            Node::Path(p) => {
                let (path, at) = paths::from_usvgr(p);
                let scale = at.determinant().abs().sqrt() as f32;
                let fill = p.fill().map(|f| color(f.paint(), f.opacity().get(), seen));
                let stroke = p
                    .stroke()
                    .map(|s| color(s.paint(), s.opacity().get(), seen));
                out.push(SvgPart {
                    id: if p.id().is_empty() { id } else { p.id() }.to_owned(),
                    path,
                    fill,
                    stroke,
                    stroke_width: p.stroke().map_or(0.0, |s| s.width().get() * scale),
                });
            }
            // text and images are counted in the source, they leave no node behind
            _ => {}
        }
    }
}

/// The parts of `svg` and what in it is not drawn as it is.
fn read(svg: &str) -> Result<(Vec<SvgPart>, Unsupported), SvgError> {
    let tree = usvgr::Tree::from_str(svg, &usvgr::Options::default(), &fontdb::Database::new())
        .map_err(|e| SvgError(e.to_string()))?;
    let (mut parts, mut seen) = (Vec::new(), Unsupported::default());
    if let Ok(doc) = usvgr::roxmltree::Document::parse(svg) {
        for node in doc.descendants() {
            match node.tag_name().name() {
                "text" => seen.text += 1,
                "image" => seen.images += 1,
                _ => {}
            }
        }
    }
    walk(tree.root(), "", &mut parts, &mut seen);
    Ok((parts, seen))
}

/// Every path of `svg`, in document order, with its absolute position and paint.
///
/// # Errors
///
/// When `svg` is not SVG.
pub fn parse_svg(svg: &str) -> Result<Vec<SvgPart>, SvgError> {
    read(svg).map(|(parts, _)| parts)
}

/// Which parts of an [`SvgShape`]: those with an `id`, or the one at an index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartKey<'k> {
    Id(&'k str),
    Index(usize),
}

impl<'k> From<&'k str> for PartKey<'k> {
    fn from(id: &'k str) -> Self {
        Self::Id(id)
    }
}

impl From<usize> for PartKey<'_> {
    fn from(index: usize) -> Self {
        Self::Index(index)
    }
}

impl PartKey<'_> {
    fn matches(self, index: usize, part: &SvgPart) -> bool {
        match self {
            Self::Id(id) => part.id == id,
            Self::Index(i) => i == index,
        }
    }
}

/// An SVG drawing as a shape (Manim `SVGMobject`). Parse the file once with
/// [`SvgShape::parse`] and keep it; a clone shares the parts. The drawing is centered on
/// `(cx, cy)` and scaled to `height`. Every path keeps the file's paint unless `fill` or
/// `stroke` replace it; [`fill_part`](Self::fill_part), [`stroke_part`](Self::stroke_part)
/// and [`only`](Self::only) change single parts. Text and images in the file are not drawn
/// (convert text to paths before export) and a gradient is drawn in its first stop's colour;
/// `inspect` warns about them.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct SvgShape {
    pub parts: Arc<Vec<SvgPart>>,
    pub cx: f32,
    pub cy: f32,
    /// The drawn height; `None` keeps the file's size and position.
    pub height: Option<f32>,
    /// Replaces the fill of every filled part.
    pub fill: Option<Color>,
    /// Replaces the outline color of every outlined part.
    pub stroke: Option<Color>,
    pub opacity: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub emphasis: Option<Emphasis>,
    /// What the file holds that is not drawn as it is; `inspect` lists it.
    pub unsupported: Arc<Vec<String>>,
}

impl Themed for SvgShape {
    fn themed(_: &Theme) -> Self {
        Self {
            parts: Arc::default(),
            cx: 960.0,
            cy: 540.0,
            // Manim's two units
            height: Some(270.0),
            fill: None,
            stroke: None,
            opacity: 1.0,
            enter: None,
            exit: None,
            emphasis: None,
            unsupported: Arc::default(),
        }
    }
}

impl Default for SvgShape {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl SvgShape {
    /// The paths of `svg` with the default placement: 270 px tall in the middle of the canvas.
    ///
    /// # Errors
    ///
    /// When `svg` is not SVG.
    pub fn parse(svg: &str) -> Result<Self, SvgError> {
        let (parts, seen) = read(svg)?;
        Ok(Self {
            parts: Arc::new(parts),
            unsupported: Arc::new(seen.messages()),
            ..Self::default()
        })
    }

    /// The first part that `key` names.
    pub fn part<'k>(&self, key: impl Into<PartKey<'k>>) -> Option<&SvgPart> {
        let key = key.into();
        self.parts
            .iter()
            .enumerate()
            .find_map(|(i, p)| key.matches(i, p).then_some(p))
    }

    fn edit<'k>(mut self, key: impl Into<PartKey<'k>>, f: impl Fn(&mut SvgPart)) -> Self {
        let key = key.into();
        for (i, part) in Arc::make_mut(&mut self.parts).iter_mut().enumerate() {
            if key.matches(i, part) {
                f(part);
            }
        }
        self
    }

    /// Fills the parts that `key` names with `color`.
    #[must_use]
    pub fn fill_part<'k>(self, key: impl Into<PartKey<'k>>, color: Color) -> Self {
        self.edit(key, |p| p.fill = Some(color))
    }

    /// Outlines the parts that `key` names in `color`, keeping their width (2 px when they had
    /// no outline).
    #[must_use]
    pub fn stroke_part<'k>(self, key: impl Into<PartKey<'k>>, color: Color) -> Self {
        self.edit(key, |p| {
            p.stroke = Some(color);
            if p.stroke_width <= 0.0 {
                p.stroke_width = 2.0;
            }
        })
    }

    /// Keeps the parts for which `keep(index, part)` is true. Indexes stay those of the file
    /// for the closure, not for later calls.
    #[must_use]
    pub fn only(mut self, keep: impl Fn(usize, &SvgPart) -> bool) -> Self {
        let parts = self.parts.iter().enumerate().filter(|(i, p)| keep(*i, p));
        self.parts = Arc::new(parts.map(|(_, p)| p.clone()).collect());
        self
    }

    /// Where the file's units go on the canvas, and the scale.
    fn placement(&self) -> (Affine, f64) {
        let Some(height) = self.height else {
            return (Affine::IDENTITY, 1.0);
        };
        let bounds = self
            .parts
            .iter()
            .map(|p| p.path.bounding_box())
            .reduce(|a, b| a.union(b))
            .unwrap_or(Rect::ZERO);
        let scale = if bounds.height() > 0.0 {
            f64::from(height) / bounds.height()
        } else {
            1.0
        };
        let at = Affine::translate((f64::from(self.cx), f64::from(self.cy)))
            * Affine::scale(scale)
            * Affine::translate(-bounds.center().to_vec2());
        (at, scale)
    }

    /// Every part placed on the canvas with its paint.
    pub fn shapes(&self) -> Vec<Shape> {
        let (at, scale) = self.placement();
        self.parts
            .iter()
            .map(|p| Shape {
                path: at * p.path.clone(),
                fill: p.fill.and(self.fill).or(p.fill),
                stroke: p.stroke.and(self.stroke).or(p.stroke),
                stroke_width: p.stroke_width * scale as f32,
                dash: None,
                opacity: self.opacity,
            })
            .collect()
    }
}

impl Geometry for SvgShape {
    fn path(&self) -> BezPath {
        let mut path = BezPath::new();
        for shape in self.shapes() {
            path.extend(shape.path.iter());
        }
        path
    }
}

impl<'a> Component<'a> for SvgShape {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        for message in self.unsupported.iter() {
            report(Diagnostic::RendererWarning {
                message: message.clone(),
            });
        }
        let parts = self.shapes();
        draw_animated(
            ui,
            &parts,
            self.enter.as_ref(),
            self.exit.as_ref(),
            self.emphasis.as_ref(),
        )
    }
}

shape! {
    /// One path given as SVG path data (Manim `VMobjectFromSVGPath`), in canvas pixels as
    /// written: `SvgPath { path: paths::parse_path("M0 0 L100 0")?, ..ui.defaults() }`.
    SvgPath(t) {
        path: BezPath = BezPath::new(),
    }
    paint: Paint::outline(t);
    path(s) {
        s.path.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUSE: &str = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>\
        <g id='house' fill='#ff0000'>\
          <rect id='wall' x='20' y='50' width='60' height='50'/>\
          <path d='M10 50 L50 10 L90 50 Z'/>\
        </g>\
        <circle id='sun' cx='90' cy='10' r='10' fill='none' stroke='#0000ff' stroke-width='4' transform='scale(1)'/>\
        </svg>";

    #[test]
    fn parts_keep_their_id_paint_and_position() {
        let parts = parse_svg(HOUSE).unwrap();
        let ids: Vec<&str> = parts.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["wall", "house", "sun"]);
        assert_eq!(parts[0].fill, Some(Color::rgb(255, 0, 0)));
        assert_eq!(
            parts[0].path.bounding_box(),
            Rect::new(20.0, 50.0, 80.0, 100.0)
        );
        assert_eq!(parts[2].fill, None);
        assert_eq!(parts[2].stroke, Some(Color::rgb(0, 0, 255)));
        assert!((parts[2].stroke_width - 4.0).abs() < 1e-6);
        assert!(parse_svg("not svg").is_err());
    }

    #[test]
    fn the_drawing_is_centered_and_scaled_to_its_height() {
        let house = SvgShape::parse(HOUSE).unwrap();
        // the file spans 10..100 across (roof to sun) and 0..100 down
        let b = house.bbox();
        assert!((b.height() - 270.0).abs() < 1e-6, "{b:?}");
        assert!((b.center().x - 960.0).abs() < 1e-6 && (b.center().y - 540.0).abs() < 1e-6);
        // the outline scales with the drawing
        assert!((house.shapes()[2].stroke_width - 4.0 * 2.7).abs() < 1e-4);
        let native = SvgShape {
            height: None,
            ..house.clone()
        };
        assert_eq!(native.bbox(), Rect::new(10.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn parts_are_addressed_by_id_or_index() {
        let green = Color::rgb(0, 255, 0);
        let house = SvgShape::parse(HOUSE).unwrap();
        let painted = house
            .clone()
            .fill_part("house", green)
            .stroke_part(0, green);
        assert_eq!(
            painted.part("wall").unwrap().fill,
            Some(Color::rgb(255, 0, 0))
        );
        assert_eq!(painted.part(1).unwrap().fill, Some(green));
        assert_eq!(painted.part(0).unwrap().stroke, Some(green));
        // the original is untouched: parts are shared until changed
        assert_eq!(house.part(1).unwrap().fill, Some(Color::rgb(255, 0, 0)));
        let sun = house.only(|_, p| p.id == "sun");
        assert_eq!(sun.parts.len(), 1);
        // an override replaces the file's paint only where the part has one
        let blue = SvgShape {
            fill: Some(green),
            ..SvgShape::parse(HOUSE).unwrap()
        };
        let shapes = blue.shapes();
        assert_eq!((shapes[0].fill, shapes[2].fill), (Some(green), None));
    }

    #[test]
    fn svg_path_takes_path_data() {
        let path = paths::parse_path("M0 0 L100 0 L100 50 Z").unwrap();
        let s = SvgPath {
            path,
            ..SvgPath::default()
        };
        assert_eq!(s.bbox(), Rect::new(0.0, 0.0, 100.0, 50.0));
    }

    #[test]
    fn what_is_not_drawn_is_listed() {
        let svg = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>\
            <defs><linearGradient id='g'><stop offset='0' stop-color='#00ff00'/><stop offset='1' stop-color='#0000ff'/></linearGradient></defs>\
            <rect width='50' height='50' fill='url(#g)'/>\
            <text x='0' y='90'>hi</text>\
            <image width='10' height='10' href='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='/>\
            </svg>";
        let shape = SvgShape::parse(svg).unwrap();
        assert_eq!(shape.parts[0].fill, Some(Color::rgb(0, 255, 0)));
        assert_eq!(
            *shape.unsupported,
            [
                "SvgShape: 1 text elements are not drawn (convert text to paths)",
                "SvgShape: 1 images are not drawn",
                "SvgShape: 1 gradients are drawn in their first stop's colour",
            ]
        );
        assert!(SvgShape::parse(HOUSE).unwrap().unsupported.is_empty());
    }
}
