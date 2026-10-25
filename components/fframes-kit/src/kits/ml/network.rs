use fframes::{Color, Svgr, Transform};
use kurbo::{BezPath, Circle, Point, Rect, RoundedRect, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    kits::{
        draw::{mix, stroke, tip},
        short,
    },
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{
    changes::{TimedChange, apply, lerp_geometry, running},
    connections::{Connection, connect},
    forward::{Dropout, ForwardPass},
    layers::{Layer, LayerGeometry, Sizes, extent, geometry, map_shapes, place},
};

fn filled(path: BezPath, color: Color, opacity: f32) -> Shape {
    Shape {
        path,
        fill: Some(color),
        stroke: None,
        stroke_width: 0.0,
        dash: None,
        opacity,
    }
}

/// A neural network as `ManimML` draws it: `layers` from left to right around `center` (units of
/// `ui.space`), `spacing` pixels between neighbouring layers, each pair joined by the
/// connection of its kind. Feed-forward layers are columns of neurons `node_radius` pixels
/// round and `node_spacing` apart, with their activation function plotted above them. With
/// `forward_pass` pulses run through the connections layer by layer and light the neurons
/// as they arrive. `dropout` crosses out some hidden neurons for a while, and `changes`
/// insert, remove and replace layers one after the other; a forward pass and a change should
/// not run at the same time.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct NeuralNetwork<'a> {
    pub layers: Vec<Layer<'a>>,
    pub center: (f64, f64),
    pub spacing: f32,
    pub node_radius: f32,
    pub node_spacing: f32,
    pub cell: f32,
    pub forward_pass: Option<ForwardPass>,
    pub dropout: Option<Dropout>,
    pub changes: Vec<TimedChange<'a>>,
    pub node_color: Color,
    pub node_fill: Color,
    pub edge_color: Color,
    pub pulse_color: Color,
    /// The outline of a positive example of a paired query or a triplet.
    pub positive_color: Color,
    /// The outline of a negative example.
    pub negative_color: Color,
    pub stroke_width: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for NeuralNetwork<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            layers: vec![
                Layer::feed_forward(3),
                Layer::feed_forward(5),
                Layer::feed_forward(3),
            ],
            center: (0.0, 0.0),
            spacing: 160.0,
            node_radius: 16.0,
            node_spacing: 52.0,
            cell: 14.0,
            forward_pass: None,
            dropout: None,
            changes: Vec::new(),
            node_color: t.colors.text,
            node_fill: t.colors.surface,
            edge_color: t.colors.muted,
            pulse_color: t.colors.highlight,
            positive_color: Color::hex("#69db7c"),
            negative_color: Color::hex("#ff6b6b"),
            stroke_width: 2.5,
            enter: None,
            exit: None,
        }
    }
}

impl Default for NeuralNetwork<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl NeuralNetwork<'_> {
    fn sizes(&self) -> Sizes {
        let r = f64::from(self.node_radius);
        Sizes {
            node_radius: r,
            node_spacing: f64::from(self.node_spacing),
            box_width: r * 5.0,
            box_height: r * 10.0,
            cell: f64::from(self.cell),
        }
    }

    /// Where every layer of `layers` goes with the middle of the network at `middle`.
    fn layout_of(&self, layers: &[&Layer<'_>], middle: Point) -> Vec<LayerGeometry> {
        let sizes = self.sizes();
        let shapes = map_shapes(layers);
        let widths: Vec<f64> = layers
            .iter()
            .zip(&shapes)
            .map(|(l, s)| extent(l, *s, sizes).0)
            .collect();
        place(&widths, f64::from(self.spacing))
            .into_iter()
            .zip(&widths)
            .zip(layers.iter().zip(shapes))
            .map(|((left, w), (layer, shape))| {
                let center = Point::new(middle.x + left + w / 2.0, middle.y);
                geometry(layer, shape, center, sizes)
            })
            .collect()
    }

    /// Where every layer goes with the middle of the network at `middle`, in pixels: the
    /// layers as given, before any of `changes`.
    pub fn layout_at(&self, middle: Point) -> Vec<LayerGeometry> {
        let layers: Vec<&Layer<'_>> = self.layers.iter().collect();
        self.layout_of(&layers, middle)
    }

    /// Where every layer goes, in pixels, before any of `changes`.
    pub fn layout(&self, ui: &Ui<'_, '_, '_>) -> Vec<LayerGeometry> {
        self.layout_at(ui.space.to_px(self.center.0, self.center.1))
    }
}

impl<'a> NeuralNetwork<'a> {
    /// A layer without neurons or maps, its outline `ring` and its fill `fill`: an embedding's
    /// point cloud, a vector's numbers in brackets, an operation's symbol in a circle, the
    /// images of a paired query or a triplet, or a box with the layer's name.
    fn other_layer(
        &self,
        ui: &Ui<'a, '_, '_>,
        layer: &Layer<'a>,
        g: &LayerGeometry,
        ring: Color,
        fill: Color,
    ) -> Vec<Svgr<'a>> {
        let r = f64::from(self.node_radius);
        let style = ui.theme.typography.caption;
        let text = |x: f64, y: f64, size: f64, s: String| {
            let style = TextStyle {
                size: size.round().max(1.0) as usize,
                ..style
            };
            line(
                x as f32,
                (y + size * 0.35) as f32,
                style,
                self.node_color,
                TextAnchor::Middle,
                0.0,
                1.0,
                s.into(),
            )
        };
        let mut parts = Vec::new();
        match layer {
            Layer::Embedding { .. } => {
                let frame = RoundedRect::from_rect(g.bounds, r * 0.4).to_path(0.1);
                parts.push(filled(frame.clone(), fill, 1.0).draw());
                parts.push(stroke(frame, ring, self.stroke_width, 1.0).draw());
                let mut dots = BezPath::new();
                for &p in &g.points {
                    dots.extend(Circle::new(p, r * 0.13).to_path(0.1));
                }
                parts.push(filled(dots, ring, 0.85).draw());
            }
            Layer::Vector { values } => {
                let b = g.bounds;
                let arm = r * 0.5;
                let mut brackets = BezPath::new();
                for (edge, inward) in [(b.x0, arm), (b.x1, -arm)] {
                    brackets.move_to((edge + inward, b.y0));
                    brackets.line_to((edge, b.y0));
                    brackets.line_to((edge, b.y1));
                    brackets.line_to((edge + inward, b.y1));
                }
                parts.push(stroke(brackets, ring, self.stroke_width, 1.0).draw());
                let size = r * 1.1;
                for (&p, &v) in g.rows.iter().zip(values) {
                    parts.push(text(p.x, p.y, size, short(v, 2)));
                }
            }
            Layer::MathOperation { operation } => {
                let circle = Circle::new(g.center, r * 1.4).to_path(0.1);
                parts.push(filled(circle.clone(), fill, 1.0).draw());
                parts.push(stroke(circle, ring, self.stroke_width, 1.0).draw());
                parts.push(text(
                    g.center.x,
                    g.center.y,
                    r * 2.4,
                    operation.symbol().into(),
                ));
            }
            Layer::PairedQuery { positive, negative } => {
                let images = [
                    (*positive, self.positive_color),
                    (*negative, self.negative_color),
                ];
                parts.extend(self.images(ui, g, &images, fill));
            }
            Layer::Triplet {
                anchor,
                positive,
                negative,
            } => {
                let images = [
                    (*anchor, ring),
                    (*positive, self.positive_color),
                    (*negative, self.negative_color),
                ];
                parts.extend(self.images(ui, g, &images, fill));
            }
            _ => {
                let frame = RoundedRect::from_rect(g.bounds, r * 0.4).to_path(0.1);
                parts.push(filled(frame.clone(), fill, 1.0).draw());
                parts.push(stroke(frame, ring, self.stroke_width, 1.0).draw());
                parts.push(text(
                    g.center.x,
                    g.center.y,
                    style.size as f64,
                    layer.name().into(),
                ));
            }
        }
        parts
    }

    /// `images` (a media name and its outline colour) in the squares of `g`.
    fn images(
        &self,
        ui: &Ui<'a, '_, '_>,
        g: &LayerGeometry,
        images: &[(&str, Color)],
        fill: Color,
    ) -> Vec<Svgr<'a>> {
        let mut parts = Vec::new();
        for (rect, &(name, outline)) in g.images.iter().zip(images) {
            let corners = [
                Point::new(rect.x0, rect.y0),
                Point::new(rect.x1, rect.y0),
                Point::new(rect.x1, rect.y1),
                Point::new(rect.x0, rect.y1),
            ];
            parts.push(filled(quad(&corners), fill, 1.0).draw());
            parts.push(picture(ui, name, &corners));
            parts.push(stroke(quad(&corners), outline, self.stroke_width, 1.0).draw());
        }
        parts
    }
}

/// The corners as a closed path.
fn quad(corners: &[Point; 4]) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(corners[0]);
    for &p in &corners[1..] {
        path.line_to(p);
    }
    path.close_path();
    path
}

/// The lines between the cells of a map `cells` wide.
fn grid(g: &LayerGeometry, map: usize) -> BezPath {
    let mut path = BezPath::new();
    let n = g.cells.max(1);
    for i in 1..n {
        let t = i as f64 / n as f64;
        for (a, b) in [((t, 0.0), (t, 1.0)), ((0.0, t), (1.0, t))] {
            if let (Some(p), Some(q)) = (g.map_point(map, a.0, a.1), g.map_point(map, b.0, b.1)) {
                path.move_to(p);
                path.line_to(q);
            }
        }
    }
    path
}

/// Maps with more cells than this are drawn without the lines between cells.
const GRID_CELLS: usize = 16;

/// A layer as drawn this frame.
struct Shown<'l, 'a> {
    layer: &'l Layer<'a>,
    geometry: LayerGeometry,
    opacity: f32,
    /// Its place among the layers once the running change is done; none while it fades out.
    index: Option<usize>,
}

/// A connection as drawn this frame.
struct Joined {
    connection: Connection,
    opacity: f32,
    /// The place of the first layer it joins (see [`Shown::index`]), which is also its place
    /// in the forward pass; the second layer is the next place. None for a connection that
    /// fades out.
    from: Option<usize>,
}

/// What a frame draws: its layers and connections, the number of connections the forward
/// pass runs through, and the neurons dropout crosses out.
struct Resolved<'l, 'a> {
    shown: Vec<Shown<'l, 'a>>,
    joined: Vec<Joined>,
    gaps: usize,
    dropped: Vec<Vec<bool>>,
}

impl Resolved<'_, '_> {
    /// Whether neuron `node` of the layer at place `index` drops out.
    fn is_dropped(&self, index: Option<usize>, node: Option<usize>) -> bool {
        index
            .zip(node)
            .and_then(|(i, n)| self.dropped.get(i)?.get(n).copied())
            .unwrap_or(false)
    }

    /// Whether line `k` of `j` starts or ends at a neuron that drops out.
    fn line_dropped(&self, j: &Joined, k: usize) -> bool {
        let (s, t) = j.connection.ends.get(k).copied().unwrap_or((None, None));
        self.is_dropped(j.from, s) || self.is_dropped(j.from.map(|i| i + 1), t)
    }
}

/// `svg` at `opacity`, as one layer so overlapping parts do not show through each other.
fn dim(svg: Svgr<'_>, opacity: f32) -> Svgr<'_> {
    if opacity >= 1.0 {
        svg
    } else {
        fframes::svgr!(<g opacity={opacity.max(0.0)}>{svg}</g>)
    }
}

impl<'a> NeuralNetwork<'a> {
    /// The layers and connections of the frame at `now`. While a change runs, the layers in
    /// the network before and after it move from their old place to the new one, the others
    /// fade in or out, and so do the connections between neighbours that are new or gone.
    fn resolve<'l>(&'l self, ui: &Ui<'a, '_, '_>, now: f32) -> Resolved<'l, 'a> {
        let r = f64::from(self.node_radius);
        let middle = ui.space.to_px(self.center.0, self.center.1);
        // before the first change both sides are the layers as given
        let (before, after, p) = if let Some((k, p)) = running(&self.changes, now) {
            (
                apply(&self.layers, &self.changes[..k]),
                apply(&self.layers, &self.changes[..=k]),
                p,
            )
        } else {
            let base = apply(&self.layers, &[]);
            (base.clone(), base, 1.0)
        };
        let layers = |list: &[(usize, &'l Layer<'a>)]| -> Vec<&'l Layer<'a>> {
            list.iter().map(|(_, l)| *l).collect()
        };
        let old = self.layout_of(&layers(&before), middle);
        let new = self.layout_of(&layers(&after), middle);
        let fading = p as f32;

        let mut shown: Vec<(usize, Shown<'l, 'a>)> = Vec::new();
        for (i, (&(id, layer), g)) in after.iter().zip(&new).enumerate() {
            let (geometry, opacity) = match before.iter().position(|(b, _)| *b == id) {
                Some(j) => (lerp_geometry(&old[j], g, p), 1.0),
                None => (g.clone(), fading),
            };
            let index = Some(i);
            shown.push((
                id,
                Shown {
                    layer,
                    geometry,
                    opacity,
                    index,
                },
            ));
        }
        for (&(id, layer), g) in before.iter().zip(&old) {
            if !after.iter().any(|(a, _)| *a == id) {
                let geometry = g.clone();
                let opacity = 1.0 - fading;
                shown.push((
                    id,
                    Shown {
                        layer,
                        geometry,
                        opacity,
                        index: None,
                    },
                ));
            }
        }

        let gaps = after.len().saturating_sub(1);
        let geometry_of = |id: usize| {
            shown
                .iter()
                .find(|(s, _)| *s == id)
                .map(|(_, s)| &s.geometry)
        };
        let pairs = |list: &[(usize, &Layer<'a>)]| -> Vec<(usize, usize)> {
            list.windows(2).map(|w| (w[0].0, w[1].0)).collect()
        };
        let (old_pairs, new_pairs) = (pairs(&before), pairs(&after));
        let mut joined = Vec::new();
        for (i, &(x, y)) in new_pairs.iter().enumerate() {
            let (Some(a), Some(b)) = (geometry_of(x), geometry_of(y)) else {
                continue;
            };
            let sweep = self
                .forward_pass
                .map_or(0.0, |f| (f.front(now, gaps) - i as f32).clamp(0.0, 1.0));
            let opacity = if old_pairs.contains(&(x, y)) {
                1.0
            } else {
                fading
            };
            let connection = connect(a, b, r, f64::from(sweep));
            joined.push(Joined {
                connection,
                opacity,
                from: Some(i),
            });
        }
        for &(x, y) in old_pairs.iter().filter(|q| !new_pairs.contains(q)) {
            let (Some(a), Some(b)) = (geometry_of(x), geometry_of(y)) else {
                continue;
            };
            let connection = connect(a, b, r, 0.0);
            joined.push(Joined {
                connection,
                opacity: 1.0 - fading,
                from: None,
            });
        }
        joined.retain(|j| j.opacity > 0.0);

        let dropped = self
            .dropout
            .map_or_else(Vec::new, |d| d.mask(&layers(&after)));
        Resolved {
            shown: shown
                .into_iter()
                .map(|(_, s)| s)
                .filter(|s| s.opacity > 0.0)
                .collect(),
            joined,
            gaps,
            dropped,
        }
    }

    /// The parts of one layer lit `glow` (0 to 1) by the forward pass, its neurons `dropped`
    /// by dropout gone by `dropout` (0 to 1).
    fn draw_layer(
        &self,
        ui: &Ui<'a, '_, '_>,
        layer: &Layer<'a>,
        g: &LayerGeometry,
        glow: f32,
        dropped: &[bool],
        dropout: f32,
    ) -> Vec<Svgr<'a>> {
        let r = f64::from(self.node_radius);
        let edge_width = (self.stroke_width * 0.5).max(1.0);
        let ring = mix(self.node_color, self.pulse_color, glow);
        let fill = mix(self.node_fill, self.pulse_color, glow * 0.6);
        let mut parts = Vec::new();
        if !g.maps.is_empty() {
            let front = g.maps.len() - 1;
            for (k, corners) in g.maps.iter().enumerate() {
                let outline = quad(corners);
                parts.push(filled(outline.clone(), fill, 1.0).draw());
                if k == front {
                    if let Layer::ImageInput { image, .. } = layer {
                        parts.push(picture(ui, image, corners));
                    } else if g.cells <= GRID_CELLS {
                        parts.push(stroke(grid(g, k), self.edge_color, 1.0, 0.6).draw());
                    }
                }
                parts.push(stroke(outline, ring, self.stroke_width * 0.8, 1.0).draw());
            }
        } else if let Layer::FeedForward { activation, .. } = layer {
            for (k, &p) in g.nodes.iter().enumerate() {
                let circle = Circle::new(p, r).to_path(0.1);
                let gone = if dropped.get(k) == Some(&true) {
                    dropout
                } else {
                    0.0
                };
                if gone <= 0.0 {
                    parts.push(filled(circle.clone(), fill, 1.0).draw());
                    parts.push(stroke(circle, ring, self.stroke_width, 1.0).draw());
                    continue;
                }
                // a dropped neuron dims, does not light up, and is crossed out
                let lit = glow * (1.0 - gone);
                let node = vec![
                    filled(
                        circle.clone(),
                        mix(self.node_fill, self.pulse_color, lit * 0.6),
                        1.0,
                    )
                    .draw(),
                    stroke(
                        circle,
                        mix(self.node_color, self.pulse_color, lit),
                        self.stroke_width,
                        1.0,
                    )
                    .draw(),
                ];
                parts.push(dim(node.into(), 1.0 - 0.75 * gone));
                let arm = r * 0.75;
                let mut cross = BezPath::new();
                for (dx, dy) in [(arm, arm), (arm, -arm)] {
                    cross.move_to((p.x - dx, p.y - dy));
                    cross.line_to((p.x + dx, p.y + dy));
                }
                parts.push(stroke(cross, self.negative_color, self.stroke_width, gone).draw());
            }
            if let Some(f) = activation {
                let w = r * 3.4;
                let h = r * 2.4;
                let bottom = g.bounds.y0 - r * 0.7;
                let frame = Rect::new(
                    g.center.x - w / 2.0,
                    bottom - h,
                    g.center.x + w / 2.0,
                    bottom,
                );
                parts.push(
                    stroke(
                        RoundedRect::from_rect(frame, r * 0.3).to_path(0.1),
                        self.edge_color,
                        edge_width,
                        1.0,
                    )
                    .draw(),
                );
                parts.push(
                    stroke(
                        f.plot(frame.inset(-r * 0.4)),
                        ring,
                        (self.stroke_width * 0.8).max(1.5),
                        1.0,
                    )
                    .draw(),
                );
            }
        } else {
            parts.extend(self.other_layer(ui, layer, g, ring, fill));
        }
        parts
    }
}

/// The lines of `c` as one path.
fn lines(c: &Connection) -> BezPath {
    let mut path = BezPath::new();
    for l in &c.lines {
        path.move_to(l.p0);
        path.line_to(l.p1);
    }
    path
}

impl<'a> Component<'a> for NeuralNetwork<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let now = ui.seconds();
        let r = f64::from(self.node_radius);
        let pass = self.forward_pass;
        let frame = self.resolve(ui, now);
        let (shown, joined, gaps) = (&frame.shown, &frame.joined, frame.gaps);
        let dropout = self.dropout.map_or(0.0, |d| d.amount(now));
        // how far line `k` of `j` is gone with the neurons dropout crosses out
        let gone = |j: &Joined, k: usize| {
            if dropout > 0.0 && frame.line_dropped(j, k) {
                dropout
            } else {
                0.0
            }
        };
        // how strongly each connection carries the pass now, 0 to 1, in and out at its ends
        let carrying: Vec<f32> = joined
            .iter()
            .map(|j| {
                j.from
                    .and_then(|i| pass.and_then(|p| p.pulse(now, i, gaps)))
                    .map_or(0.0, |along| (along * 8.0).min((1.0 - along) * 8.0).min(1.0))
            })
            .collect();
        let edge_width = (self.stroke_width * 0.5).max(1.0);
        let mut parts: Vec<Svgr<'a>> = Vec::new();

        // the edges that are fully there as one path, the fading ones each on their own, the
        // edges of dropped neurons as one path fading with them
        let mut edges = BezPath::new();
        let mut dropped_edges = BezPath::new();
        for j in joined.iter().filter(|j| j.connection.filter.is_none()) {
            if j.opacity >= 1.0 {
                for (k, l) in j.connection.lines.iter().enumerate() {
                    let path = if gone(j, k) > 0.0 {
                        &mut dropped_edges
                    } else {
                        &mut edges
                    };
                    path.move_to(l.p0);
                    path.line_to(l.p1);
                }
            } else {
                let path = lines(&j.connection);
                parts.push(dim(
                    stroke(path, self.edge_color, edge_width, 0.55).draw(),
                    j.opacity,
                ));
            }
        }
        parts.insert(0, stroke(edges, self.edge_color, edge_width, 0.55).draw());
        if !dropped_edges.elements().is_empty() {
            let opacity = 0.55 * (1.0 - dropout);
            parts.push(stroke(dropped_edges, self.edge_color, edge_width, opacity).draw());
        }
        for j in joined.iter().filter(|j| j.connection.arrow) {
            for l in &j.connection.lines {
                let d = l.p1 - l.p0;
                if d.hypot() > 1e-9 {
                    let head = tip(l.p1, d.normalize(), r * 0.8, self.edge_color, 0.9).draw();
                    parts.push(dim(head, j.opacity));
                }
            }
        }

        for s in shown {
            let glow = s.index.zip(pass).map_or(0.0, |(j, p)| p.glow(now, j, gaps));
            let dropped = s
                .index
                .and_then(|i| frame.dropped.get(i))
                .map_or(&[][..], Vec::as_slice);
            let layer = self.draw_layer(ui, s.layer, &s.geometry, glow, dropped, dropout);
            if s.opacity >= 1.0 {
                parts.extend(layer);
            } else {
                parts.push(dim(layer.into(), s.opacity));
            }
        }

        // the filters on top of the maps they slide over
        for (j, &k) in joined.iter().zip(&carrying) {
            let Some(f) = j.connection.filter else {
                continue;
            };
            let color = mix(self.edge_color, self.pulse_color, k);
            let mut filter =
                vec![stroke(lines(&j.connection), color, edge_width, 0.55 + 0.45 * k).draw()];
            for corners in [f.source, f.target] {
                filter.push(filled(quad(&corners), self.pulse_color, 0.15 + 0.35 * k).draw());
                filter.push(stroke(quad(&corners), color, edge_width * 1.5, 1.0).draw());
            }
            if j.opacity >= 1.0 {
                parts.extend(filter);
            } else {
                parts.push(dim(filter.into(), j.opacity));
            }
        }

        // pulses on the edges of dropped neurons fade with them
        let mut pulses = BezPath::new();
        let mut dropped_pulses = BezPath::new();
        let mut opacity = 0.0_f32;
        for (j, &k) in joined.iter().zip(&carrying) {
            let Some(along) = j
                .from
                .and_then(|i| pass.and_then(|p| p.pulse(now, i, gaps)))
            else {
                continue;
            };
            if j.connection.filter.is_some() {
                continue;
            }
            opacity = opacity.max(k);
            for (n, l) in j.connection.lines.iter().enumerate() {
                let at = l.p0.lerp(l.p1, f64::from(along));
                let path = if gone(j, n) > 0.0 {
                    &mut dropped_pulses
                } else {
                    &mut pulses
                };
                path.extend(Circle::new(at, r * 0.32).to_path(0.1));
            }
        }
        if !pulses.elements().is_empty() {
            parts.push(filled(pulses, self.pulse_color, opacity).draw());
        }
        if !dropped_pulses.elements().is_empty() {
            let faded = opacity * (1.0 - dropout);
            parts.push(filled(dropped_pulses, self.pulse_color, faded).draw());
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}
/// The image `name` drawn on the map `corners`, sheared like the map; a missing image (a
/// typo, or dynamic media in the editor) leaves the map's fill.
fn picture<'a>(ui: &Ui<'a, '_, '_>, name: &str, corners: &[Point; 4]) -> Svgr<'a> {
    let Some(image) = ui.ctx.get_image(name) else {
        return Svgr::empty();
    };
    let [tl, tr, _, bl] = *corners;
    let width = tr.x - tl.x;
    let height = bl.y - tl.y;
    if width <= 0.0 || height <= 0.0 {
        return Svgr::empty();
    }
    let transform = Transform {
        translate_x: tl.x,
        translate_y: tl.y,
        skew_y: ((tr.y - tl.y) / width).atan().to_degrees(),
        ..Transform::default()
    };
    fframes::svgr!(
        <image href={image.href()} x="0" y="0" width={width} height={height}
               preserveAspectRatio="none" transform={transform} />
    )
}
