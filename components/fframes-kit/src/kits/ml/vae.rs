use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Point, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    kits::draw::stroke,
    motion::cubic_in_out,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{
    forward::ForwardPass,
    layers::{EMBEDDING, Layer, LayerGeometry, embedding_px},
    network::NeuralNetwork,
};

/// A variational autoencoder as `ManimML` draws it (`VariationalAutoencoder`): the image
/// `input`, the feed-forward layers of the `encoder` (neurons per layer), the latent space
/// with its Gaussian, the `decoder` and the image `output`, `size` cells square. The latent
/// box shows a cloud of `points` points for `seed`, the Gaussian's 1σ and 2σ rings in
/// `ring_color`, and the code of this image in `pulse_color`: the point `μ + L·epsilon` of the Gaussian,
/// `epsilon` in units of the standard normal distribution. With `forward_pass` the pulses
/// run from the input to the output and the point appears when they reach the latent space.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct VariationalAutoencoder<'a> {
    pub input: &'a str,
    pub output: &'a str,
    pub size: usize,
    pub encoder: Vec<usize>,
    pub decoder: Vec<usize>,
    pub points: usize,
    pub seed: u64,
    pub epsilon: (f64, f64),
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
    pub ring_color: Color,
    pub stroke_width: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for VariationalAutoencoder<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            input: "",
            output: "",
            size: 8,
            encoder: vec![5, 3],
            decoder: vec![3, 5],
            points: 120,
            seed: 1,
            epsilon: (1.3, -0.7),
            center: (0.0, 0.0),
            spacing: 130.0,
            node_radius: 16.0,
            node_spacing: 46.0,
            cell: 14.0,
            forward_pass: None,
            node_color: t.colors.text,
            node_fill: t.colors.surface,
            edge_color: t.colors.muted,
            pulse_color: t.colors.highlight,
            ring_color: t.colors.accent,
            stroke_width: 2.5,
            enter: None,
            exit: None,
        }
    }
}

impl Default for VariationalAutoencoder<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// How long the code of the image takes to appear, in seconds.
const APPEAR: f32 = 0.3;

impl<'a> VariationalAutoencoder<'a> {
    /// The layers from the input image to the output image.
    pub fn layers(&self) -> Vec<Layer<'a>> {
        let image = |image| Layer::ImageInput {
            image,
            size: self.size,
        };
        let dense = |&nodes: &usize| Layer::feed_forward(nodes);
        let mut layers = vec![image(self.input)];
        layers.extend(self.encoder.iter().map(dense));
        layers.push(Layer::Embedding {
            points: self.points,
            seed: self.seed,
        });
        layers.extend(self.decoder.iter().map(dense));
        layers.push(image(self.output));
        layers
    }

    /// The place of the latent space among [`layers`](Self::layers).
    pub fn latent_index(&self) -> usize {
        1 + self.encoder.len()
    }

    /// The network that draws the layers, its look taken from `base`.
    fn network(&self, base: NeuralNetwork<'a>) -> NeuralNetwork<'a> {
        NeuralNetwork {
            layers: self.layers(),
            center: self.center,
            spacing: self.spacing,
            node_radius: self.node_radius,
            node_spacing: self.node_spacing,
            cell: self.cell,
            forward_pass: self.forward_pass,
            node_color: self.node_color,
            node_fill: self.node_fill,
            edge_color: self.edge_color,
            pulse_color: self.pulse_color,
            stroke_width: self.stroke_width,
            enter: None,
            exit: None,
            ..base
        }
    }

    /// Where the code of the image sits in the latent box `latent`, kept inside the box.
    fn code(&self, latent: &LayerGeometry) -> Option<Point> {
        let p = embedding_px(latent.bounds, EMBEDDING.point(self.epsilon)?);
        let inner = latent.bounds.inset(-f64::from(self.node_radius) * 0.5);
        Some(Point::new(
            p.x.clamp(inner.x0, inner.x1),
            p.y.clamp(inner.y0, inner.y1),
        ))
    }
}

impl<'a> Component<'a> for VariationalAutoencoder<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let now = ui.seconds();
        let network = self.network(ui.defaults());
        let layout = network.layout(ui);
        let mut parts = vec![ui.show(network)];
        let Some(latent) = layout.get(self.latent_index()) else {
            return fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref());
        };

        for (sigmas, opacity) in [(1.0, 0.9), (2.0, 0.45)] {
            let mut ring = BezPath::new();
            for (k, p) in EMBEDDING.ring(sigmas).into_iter().enumerate() {
                let p = embedding_px(latent.bounds, p);
                if k == 0 {
                    ring.move_to(p);
                } else {
                    ring.line_to(p);
                }
            }
            ring.close_path();
            parts.push(stroke(ring, self.ring_color, self.stroke_width * 0.6, opacity).draw());
        }

        // the code appears as the pulses reach the latent space
        let gaps = layout.len().saturating_sub(1);
        let shown = self.forward_pass.map_or(1.0, |p| {
            let arrival = p.arrival(self.latent_index(), gaps);
            cubic_in_out(((now - arrival + APPEAR / 2.0) / APPEAR).clamp(0.0, 1.0))
        });
        if shown > 0.0
            && let Some(code) = self.code(latent)
        {
            let r = f64::from(self.node_radius) * 0.4;
            let dot = Circle::new(code, r).to_path(0.1);
            parts.push(
                Shape {
                    path: dot.clone(),
                    fill: Some(self.pulse_color),
                    stroke: None,
                    stroke_width: 0.0,
                    dash: None,
                    opacity: shown,
                }
                .draw(),
            );
            parts.push(stroke(dot, self.node_color, self.stroke_width * 0.6, shown).draw());
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layers_run_from_image_to_image_through_the_latent_space() {
        let vae = VariationalAutoencoder {
            input: "in.png",
            output: "out.png",
            encoder: vec![5, 4, 3],
            decoder: vec![3, 5],
            ..Default::default()
        };
        let layers = vae.layers();
        assert_eq!(layers.len(), 8);
        assert!(matches!(
            layers[0],
            Layer::ImageInput {
                image: "in.png",
                size: 8
            }
        ));
        assert_eq!(layers[3], Layer::feed_forward(3));
        assert_eq!(vae.latent_index(), 4);
        assert!(matches!(
            layers[4],
            Layer::Embedding {
                points: 120,
                seed: 1
            }
        ));
        assert_eq!(layers[5], Layer::feed_forward(3));
        assert!(matches!(
            layers[7],
            Layer::ImageInput {
                image: "out.png",
                ..
            }
        ));
    }

    #[test]
    fn the_code_sits_in_the_latent_box() {
        let vae = VariationalAutoencoder::default();
        let layout = vae
            .network(NeuralNetwork::default())
            .layout_at(Point::new(960.0, 540.0));
        let latent = &layout[vae.latent_index()];
        // epsilon 0 is the mean of the Gaussian, wherever it sits in the box
        let mean = VariationalAutoencoder {
            epsilon: (0.0, 0.0),
            ..vae.clone()
        };
        let mean_px = embedding_px(latent.bounds, EMBEDDING.mean);
        assert!(mean.code(latent).unwrap().distance(mean_px) < 1e-9);
        let code = vae.code(latent).unwrap();
        assert!(latent.bounds.contains(code));
        // epsilon (1.3, -0.7): right of the middle
        assert!(code.x > latent.center.x);
        // far out it stays inside the box
        let far = VariationalAutoencoder {
            epsilon: (40.0, 40.0),
            ..vae.clone()
        };
        assert!(latent.bounds.contains(far.code(latent).unwrap()));
    }
}
