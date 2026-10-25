use fframes::{Color, Svgr};
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
    layers::{Layer, LayerGeometry, Sizes, extent, geometry, place},
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
        }
    }

    /// Where every layer goes with the middle of the network at `middle`, in pixels.
    pub fn layout_at(&self, middle: Point) -> Vec<LayerGeometry> {
        let sizes = self.sizes();
        let widths: Vec<f64> = self.layers.iter().map(|l| extent(l, sizes).0).collect();
        place(&widths, f64::from(self.spacing))
            .into_iter()
            .zip(&widths)
            .zip(&self.layers)
            .map(|((left, w), layer)| {
                let center = Point::new(middle.x + left + w / 2.0, middle.y);
                geometry(layer, center, sizes)
            })
            .collect()
    }

    /// Where every layer goes, in pixels.
    pub fn layout(&self, ui: &Ui<'_, '_, '_>) -> Vec<LayerGeometry> {
        self.layout_at(ui.space.to_px(self.center.0, self.center.1))
    }
}

impl<'a> Component<'a> for NeuralNetwork<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let layout = self.layout(ui);
        let now = ui.seconds();
        let r = f64::from(self.node_radius);
        let gaps = layout.len().saturating_sub(1);
        let connections: Vec<_> = layout
            .windows(2)
            .map(|w| connect(&w[0], &w[1], r))
            .collect();
        let mut shapes = Vec::new();

        let mut edges = BezPath::new();
        for c in &connections {
            for l in &c.lines {
                edges.move_to(l.p0);
                edges.line_to(l.p1);
            }
        }
        let edge_width = (self.stroke_width * 0.5).max(1.0);
        shapes.push(stroke(edges, self.edge_color, edge_width, 0.55));
        for c in connections.iter().filter(|c| c.arrow) {
            for l in &c.lines {
                let d = l.p1 - l.p0;
                if d.hypot() > 1e-9 {
                    shapes.push(tip(l.p1, d.normalize(), r * 0.8, self.edge_color, 0.9));
                }
            }
        }

        if let Some(pass) = self.forward_pass {
            let mut pulses = BezPath::new();
            let mut opacity = 1.0;
            for (i, c) in connections.iter().enumerate() {
                let Some(along) = pass.pulse(now, i, gaps) else {
                    continue;
                };
                // in and out at the ends, so a pulse never appears or vanishes at full size
                opacity = (along * 8.0).min((1.0 - along) * 8.0).min(1.0);
                for l in &c.lines {
                    let at = l.p0.lerp(l.p1, f64::from(along));
                    pulses.extend(Circle::new(at, r * 0.32).to_path(0.1));
                }
            }
            if !pulses.elements().is_empty() {
                shapes.push(filled(pulses, self.pulse_color, opacity));
            }
        }

        let style = ui.theme.typography.caption;
        let mut texts = Vec::new();
        for (j, (layer, g)) in self.layers.iter().zip(&layout).enumerate() {
            let glow = self
                .forward_pass
                .map_or(0.0, |pass| pass.glow(now, j, gaps));
            let ring = mix(self.node_color, self.pulse_color, glow);
            let fill = mix(self.node_fill, self.pulse_color, glow * 0.6);
            if let Layer::FeedForward { activation, .. } = layer {
                for &p in &g.nodes {
                    let circle = Circle::new(p, r).to_path(0.1);
                    shapes.push(filled(circle.clone(), fill, 1.0));
                    shapes.push(stroke(circle, ring, self.stroke_width, 1.0));
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
                    shapes.push(stroke(
                        RoundedRect::from_rect(frame, r * 0.3).to_path(0.1),
                        self.edge_color,
                        edge_width,
                        1.0,
                    ));
                    shapes.push(stroke(
                        f.plot(frame.inset(-r * 0.4)),
                        ring,
                        (self.stroke_width * 0.8).max(1.5),
                        1.0,
                    ));
                }
            } else {
                let frame = RoundedRect::from_rect(g.bounds, r * 0.4).to_path(0.1);
                shapes.push(filled(frame.clone(), fill, 1.0));
                shapes.push(stroke(frame, ring, self.stroke_width, 1.0));
                texts.push(line(
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
        let parts: Vec<Svgr<'a>> = shapes.iter().map(Shape::draw).chain(texts).collect();
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}
