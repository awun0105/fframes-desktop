use fframes::{Color, Svgr, Transform};
use kurbo::{BezPath, Circle, Point, Rect, RoundedRect, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    kits::draw::{mix, stroke, tip},
    text::{TextAnchor, line},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{
    connections::connect,
    forward::ForwardPass,
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
/// as they arrive.
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
    pub node_color: Color,
    pub node_fill: Color,
    pub edge_color: Color,
    pub pulse_color: Color,
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
            node_color: t.colors.text,
            node_fill: t.colors.surface,
            edge_color: t.colors.muted,
            pulse_color: t.colors.highlight,
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

    /// Where every layer goes with the middle of the network at `middle`, in pixels.
    pub fn layout_at(&self, middle: Point) -> Vec<LayerGeometry> {
        let sizes = self.sizes();
        let shapes = map_shapes(&self.layers);
        let widths: Vec<f64> = self
            .layers
            .iter()
            .zip(&shapes)
            .map(|(l, s)| extent(l, *s, sizes).0)
            .collect();
        place(&widths, f64::from(self.spacing))
            .into_iter()
            .zip(&widths)
            .zip(self.layers.iter().zip(shapes))
            .map(|((left, w), (layer, shape))| {
                let center = Point::new(middle.x + left + w / 2.0, middle.y);
                geometry(layer, shape, center, sizes)
            })
            .collect()
    }

    /// Where every layer goes, in pixels.
    pub fn layout(&self, ui: &Ui<'_, '_, '_>) -> Vec<LayerGeometry> {
        self.layout_at(ui.space.to_px(self.center.0, self.center.1))
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

impl<'a> Component<'a> for NeuralNetwork<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let layout = self.layout(ui);
        let now = ui.seconds();
        let r = f64::from(self.node_radius);
        let gaps = layout.len().saturating_sub(1);
        let pass = self.forward_pass;
        let connections: Vec<_> = layout
            .windows(2)
            .enumerate()
            .map(|(i, w)| {
                let sweep = pass.map_or(0.0, |p| (p.front(now, gaps) - i as f32).clamp(0.0, 1.0));
                connect(&w[0], &w[1], r, f64::from(sweep))
            })
            .collect();
        // how strongly each connection carries the pass now, 0 to 1, in and out at its ends
        let carrying: Vec<f32> = (0..connections.len())
            .map(|i| {
                pass.and_then(|p| p.pulse(now, i, gaps))
                    .map_or(0.0, |along| (along * 8.0).min((1.0 - along) * 8.0).min(1.0))
            })
            .collect();
        let edge_width = (self.stroke_width * 0.5).max(1.0);
        let mut parts: Vec<Svgr<'a>> = Vec::new();

        let mut edges = BezPath::new();
        for c in connections.iter().filter(|c| c.filter.is_none()) {
            for l in &c.lines {
                edges.move_to(l.p0);
                edges.line_to(l.p1);
            }
        }
        parts.push(stroke(edges, self.edge_color, edge_width, 0.55).draw());
        for c in connections.iter().filter(|c| c.arrow) {
            for l in &c.lines {
                let d = l.p1 - l.p0;
                if d.hypot() > 1e-9 {
                    parts.push(tip(l.p1, d.normalize(), r * 0.8, self.edge_color, 0.9).draw());
                }
            }
        }

        let style = ui.theme.typography.caption;
        for (j, (layer, g)) in self.layers.iter().zip(&layout).enumerate() {
            let glow = pass.map_or(0.0, |p| p.glow(now, j, gaps));
            let ring = mix(self.node_color, self.pulse_color, glow);
            let fill = mix(self.node_fill, self.pulse_color, glow * 0.6);
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
                for &p in &g.nodes {
                    let circle = Circle::new(p, r).to_path(0.1);
                    parts.push(filled(circle.clone(), fill, 1.0).draw());
                    parts.push(stroke(circle, ring, self.stroke_width, 1.0).draw());
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
                let frame = RoundedRect::from_rect(g.bounds, r * 0.4).to_path(0.1);
                parts.push(filled(frame.clone(), fill, 1.0).draw());
                parts.push(stroke(frame, ring, self.stroke_width, 1.0).draw());
                parts.push(line(
                    g.center.x as f32,
                    g.center.y as f32 + style.size as f32 * 0.35,
                    style,
                    self.node_color,
                    TextAnchor::Middle,
                    0.0,
                    1.0,
                    layer.name().into(),
                ));
            }
        }

        // the filters on top of the maps they slide over
        for (c, &k) in connections.iter().zip(&carrying) {
            let Some(f) = c.filter else {
                continue;
            };
            let color = mix(self.edge_color, self.pulse_color, k);
            let mut lines = BezPath::new();
            for l in &c.lines {
                lines.move_to(l.p0);
                lines.line_to(l.p1);
            }
            parts.push(stroke(lines, color, edge_width, 0.55 + 0.45 * k).draw());
            for corners in [f.source, f.target] {
                parts.push(filled(quad(&corners), self.pulse_color, 0.15 + 0.35 * k).draw());
                parts.push(stroke(quad(&corners), color, edge_width * 1.5, 1.0).draw());
            }
        }

        let mut pulses = BezPath::new();
        let mut opacity = 0.0_f32;
        for ((i, c), &k) in connections.iter().enumerate().zip(&carrying) {
            let Some(along) = pass.and_then(|p| p.pulse(now, i, gaps)) else {
                continue;
            };
            if c.filter.is_some() {
                continue;
            }
            opacity = opacity.max(k);
            for l in &c.lines {
                let at = l.p0.lerp(l.p1, f64::from(along));
                pulses.extend(Circle::new(at, r * 0.32).to_path(0.1));
            }
        }
        if !pulses.elements().is_empty() {
            parts.push(filled(pulses, self.pulse_color, opacity).draw());
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
