//! Manim's `Graph` and `DiGraph`: vertices and edges placed by a [`GraphLayout`].

use std::{borrow::Cow, sync::Arc};

use fframes::{Color, Svgr};
use kurbo::{Circle, CubicBez, ParamCurve, ParamCurveArclen, Point, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    kits::draw::{label_point, link, stroke, tip},
    layout::{GraphLayout, graph_layout},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{Space, graph::cached};

/// The look of one vertex of a [`Graph`]; `None` keeps the graph's.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VertexStyle<'a> {
    pub vertex: usize,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    /// In units.
    pub radius: Option<f64>,
    /// Written inside the vertex instead of its name.
    pub label: Option<Cow<'a, str>>,
}

/// The look of the edges from `edge.0` to `edge.1` of a [`Graph`] (either way round when
/// undirected); `None` keeps the graph's.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EdgeStyle<'a> {
    pub edge: (usize, usize),
    pub color: Option<Color>,
    pub width: Option<f32>,
    /// Pixels to the left of the way at the middle; 0 is straight.
    pub bend: Option<f64>,
    /// Written beside the middle of the edge.
    pub label: Option<Cow<'a, str>>,
}

/// Vertices joined by edges (Manim `Graph`), placed by `layout` within `layout_scale` units
/// of `center` (the layout is computed once and cached). Vertices are dots `radius` units
/// wide; with `labels` they carry their names and grow to hold them. Edges are straight, or
/// bent by `edge_bend` pixels. `vertex_styles` and `edge_styles` change single vertices and
/// edges. [`DiGraph`] draws the same with tips on the edges.
///
/// Vertices and edges appear with `enter` (`Create`, `GrowFromCenter`, ...), the names and
/// edge labels with `labels_enter`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Graph<'a> {
    pub vertices: Vec<Cow<'a, str>>,
    pub edges: Vec<(usize, usize)>,
    pub layout: GraphLayout,
    pub layout_scale: f64,
    pub center: (f64, f64),
    pub labels: bool,
    pub radius: f64,
    pub vertex_fill: Color,
    pub vertex_stroke: Option<Color>,
    pub label_style: TextStyle,
    pub label_fill: Color,
    pub edge_color: Color,
    pub edge_width: f32,
    pub edge_bend: f64,
    pub tip_length: f64,
    pub vertex_styles: Vec<VertexStyle<'a>>,
    pub edge_styles: Vec<EdgeStyle<'a>>,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub labels_enter: Option<FadeIn>,
    pub labels_exit: Option<FadeOut>,
}

impl Themed for Graph<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            vertices: Vec::new(),
            edges: Vec::new(),
            layout: GraphLayout::default(),
            layout_scale: 2.0,
            center: (0.0, 0.0),
            labels: false,
            radius: 0.1,
            vertex_fill: t.colors.text,
            vertex_stroke: None,
            label_style: t.typography.body,
            label_fill: t.colors.background,
            edge_color: t.colors.text,
            edge_width: t.stroke.regular,
            edge_bend: 0.0,
            tip_length: 16.0,
            vertex_styles: Vec::new(),
            edge_styles: Vec::new(),
            enter: None,
            exit: None,
            labels_enter: None,
            labels_exit: None,
        }
    }
}

impl Default for Graph<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Bits that tell layouts apart in the cache key.
fn layout_key(layout: &GraphLayout) -> Vec<u64> {
    let lists = |tag: u64, lists: &[Vec<usize>]| {
        let mut key = vec![tag];
        for list in lists {
            key.push(u64::MAX);
            key.extend(list.iter().map(|&v| v as u64));
        }
        key
    };
    match layout {
        GraphLayout::Circular => vec![0],
        GraphLayout::KamadaKawai => vec![1],
        GraphLayout::Planar => vec![2],
        GraphLayout::Random { seed } => vec![3, *seed],
        GraphLayout::Shell { shells } => lists(4, shells),
        GraphLayout::Spectral => vec![5],
        GraphLayout::Spiral => vec![6],
        GraphLayout::Spring { seed, iterations } => vec![7, *seed, *iterations as u64],
        GraphLayout::Partite { parts } => lists(8, parts),
        GraphLayout::Tree { root } => vec![9, *root as u64],
        GraphLayout::Function(f) => vec![10, *f as usize as u64],
    }
}

impl<'a> Graph<'a> {
    /// A graph of these vertices and edges with the theme's look and the spring layout.
    pub fn new(ui: &Ui<'_, '_, '_>, vertices: &[&'a str], edges: &[(usize, usize)]) -> Self {
        Self {
            vertices: vertices.iter().map(|&v| Cow::Borrowed(v)).collect(),
            edges: edges.to_vec(),
            ..ui.defaults()
        }
    }

    /// The layout's positions in units around the origin, computed once per layout, vertex
    /// count, edges and scale.
    pub fn layout_positions(&self) -> Arc<[(f64, f64)]> {
        let mut params = layout_key(&self.layout);
        params.push(u64::MAX);
        params.push(self.vertices.len() as u64);
        params.push(self.layout_scale.to_bits());
        params.extend(self.edges.iter().flat_map(|&(a, b)| [a as u64, b as u64]));
        let address = match self.layout {
            GraphLayout::Function(f) => f as usize,
            _ => 0,
        };
        cached((5, address, params), || {
            graph_layout(
                &self.layout,
                self.vertices.len(),
                &self.edges,
                self.layout_scale,
            )
            .into()
        })
    }

    /// Where vertex `v` is, in pixels of `space` (Manim `graph[v].get_center()`).
    pub fn vertex_point(&self, space: Space, v: usize) -> Option<Point> {
        let (x, y) = *self.layout_positions().get(v)?;
        Some(space.to_px(self.center.0 + x, self.center.1 + y))
    }

    fn vertex_style(&self, v: usize) -> Option<&VertexStyle<'a>> {
        self.vertex_styles.iter().find(|s| s.vertex == v)
    }

    fn edge_style(&self, (a, b): (usize, usize), directed: bool) -> Option<&EdgeStyle<'a>> {
        self.edge_styles
            .iter()
            .find(|s| s.edge == (a, b) || (!directed && s.edge == (b, a)))
    }

    /// The name written in vertex `v`, if any.
    fn label(&self, v: usize) -> Option<&str> {
        match self.vertex_style(v).and_then(|s| s.label.as_deref()) {
            Some(label) => Some(label),
            None if self.labels => self.vertices.get(v).map(AsRef::as_ref),
            None => None,
        }
    }

    /// The radius of vertex `v` in pixels.
    fn radius_px(&self, space: Space, v: usize) -> f64 {
        let r = self
            .vertex_style(v)
            .and_then(|s| s.radius)
            .unwrap_or(self.radius);
        let r = space.px(r);
        match self.label(v) {
            // wide enough for a short name in the label style
            Some(label) => {
                let chars = label.chars().count().max(1) as f64;
                let size = self.label_style.size as f64;
                r.max(size * (0.4 + 0.3 * chars))
            }
            None => r,
        }
    }

    fn render_as(self, ui: &mut Ui<'a, '_, '_>, directed: bool) -> Svgr<'a> {
        let n = self.vertices.len();
        let points: Vec<Point> = (0..n)
            .filter_map(|v| self.vertex_point(ui.space, v))
            .collect();
        let radii: Vec<f64> = (0..n).map(|v| self.radius_px(ui.space, v)).collect();
        let mut shapes = Vec::new();
        let mut texts: Vec<Svgr<'a>> = Vec::new();
        let caption = ui.theme.typography.caption;
        for &(a, b) in &self.edges {
            if a == b || a >= n || b >= n {
                continue;
            }
            let style = self.edge_style((a, b), directed);
            // the two edges of a pair that point both ways bend apart
            let paired = directed && self.edges.contains(&(b, a));
            let bend = style
                .and_then(|s| s.bend)
                .unwrap_or(self.edge_bend + if paired { 20.0 } else { 0.0 });
            let curve = link(points[a], points[b], radii[a], radii[b], bend);
            let color = style.and_then(|s| s.color).unwrap_or(self.edge_color);
            let width = style.and_then(|s| s.width).unwrap_or(self.edge_width);
            shapes.extend(edge(curve, directed, color, width, self.tip_length));
            if let Some(label) = style.and_then(|s| s.label.clone()) {
                // beside the middle, clear of the edge by half the text's width
                let size = caption.size as f64;
                let at = label_point(&curve, size * (0.5 + 0.3 * label.chars().count() as f64));
                texts.push(line(
                    at.x as f32,
                    at.y as f32 + caption.size as f32 * 0.35,
                    caption,
                    color,
                    TextAnchor::Middle,
                    0.0,
                    1.0,
                    label,
                ));
            }
        }
        for (v, &p) in points.iter().enumerate() {
            let style = self.vertex_style(v);
            let fill = style.and_then(|s| s.fill).unwrap_or(self.vertex_fill);
            let outline = style.and_then(|s| s.stroke).or(self.vertex_stroke);
            shapes.push(Shape {
                path: Circle::new(p, radii[v]).to_path(0.1),
                fill: Some(fill),
                stroke: outline,
                stroke_width: if outline.is_some() { 2.0 } else { 0.0 },
                dash: None,
                opacity: 1.0,
            });
            if let Some(label) = self.label(v) {
                texts.push(line(
                    p.x as f32,
                    p.y as f32 + self.label_style.size as f32 * 0.35,
                    self.label_style,
                    self.label_fill,
                    TextAnchor::Middle,
                    0.0,
                    1.0,
                    Cow::Owned(label.to_owned()),
                ));
            }
        }
        let drawn = draw_animated(ui, &shapes, self.enter.as_ref(), self.exit.as_ref(), None);
        let texts = fade(
            ui,
            texts.into_iter().collect(),
            self.labels_enter.as_ref(),
            self.labels_exit.as_ref(),
        );
        fframes::svgr!(<g>{drawn}{texts}</g>)
    }
}

/// An edge as shapes: the line, and a tip at its end when `head`.
fn edge(curve: CubicBez, head: bool, color: Color, width: f32, tip_length: f64) -> Vec<Shape> {
    let length = curve.arclen(0.5);
    if !head || length <= tip_length * 1.5 {
        return vec![stroke(curve.into_path(0.1), color, width, 1.0)];
    }
    let shaft = curve.subsegment(0.0..1.0 - tip_length / length);
    let dir = (curve.p3 - shaft.p3).normalize();
    vec![
        stroke(shaft.into_path(0.1), color, width, 1.0),
        tip(curve.p3, dir, tip_length, color, 1.0),
    ]
}

impl<'a> Component<'a> for Graph<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.render_as(ui, false)
    }
}

/// A [`Graph`] whose edges point from their first vertex to their second (Manim `DiGraph`):
/// each edge ends in a tip at the edge of its target, and the two edges of a pair that
/// point both ways bend apart.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct DiGraph<'a> {
    pub graph: Graph<'a>,
}

impl Themed for DiGraph<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            graph: Graph::themed(t),
        }
    }
}

impl Default for DiGraph<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> DiGraph<'a> {
    /// A directed graph of these vertices and edges with the theme's look.
    pub fn new(ui: &Ui<'_, '_, '_>, vertices: &[&'a str], edges: &[(usize, usize)]) -> Self {
        Self {
            graph: Graph::new(ui, vertices, edges),
        }
    }
}

impl<'a> Component<'a> for DiGraph<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.graph.render_as(ui, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> Graph<'static> {
        Graph {
            vertices: ["a", "b", "c", "d"].map(Cow::Borrowed).to_vec(),
            edges: vec![(0, 1), (1, 2), (2, 3), (3, 0)],
            layout: GraphLayout::Circular,
            center: (1.0, 0.5),
            ..Graph::default()
        }
    }

    #[test]
    fn vertices_sit_where_the_layout_puts_them_around_the_center() {
        let g = graph();
        let space = Space::FULL_HD;
        // circular from the right, counterclockwise, `layout_scale` units out
        assert_eq!(g.vertex_point(space, 0), Some(space.to_px(3.0, 0.5)));
        let up = g.vertex_point(space, 1).unwrap();
        let want = space.to_px(1.0, 2.5);
        assert!((up - want).hypot() < 1e-6, "{up:?}");
        assert_eq!(g.vertex_point(space, 4), None);
        assert!(Arc::ptr_eq(
            &g.layout_positions(),
            &graph().layout_positions()
        ));
    }

    #[test]
    fn labels_widen_vertices_and_styles_pick_their_edges() {
        let mut g = graph();
        let small = g.radius_px(Space::FULL_HD, 0);
        g.labels = true;
        assert!(g.radius_px(Space::FULL_HD, 0) > small);
        g.vertex_styles.push(VertexStyle {
            vertex: 2,
            label: Some("start".into()),
            ..VertexStyle::default()
        });
        assert_eq!(g.label(2), Some("start"));
        assert_eq!(g.label(1), Some("b"));
        g.edge_styles.push(EdgeStyle {
            edge: (2, 1),
            width: Some(9.0),
            ..EdgeStyle::default()
        });
        assert!(g.edge_style((1, 2), false).is_some());
        assert!(g.edge_style((1, 2), true).is_none());
        // an arrow ends in a tip, a plain edge does not
        let curve = link(Point::ZERO, Point::new(300.0, 0.0), 10.0, 10.0, 0.0);
        assert_eq!(edge(curve, true, Color::WHITE, 3.0, 16.0).len(), 2);
        assert_eq!(edge(curve, false, Color::WHITE, 3.0, 16.0).len(), 1);
    }
}
