//! The layers of a [`NeuralNetwork`](super::NeuralNetwork) and where their parts go.

use kurbo::{BezPath, Point, Rect};

/// The activation function of a feed-forward layer, drawn as a small plot above it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum ActivationFunction {
    Relu,
    Sigmoid,
    Tanh,
}

impl ActivationFunction {
    pub fn apply(self, x: f64) -> f64 {
        match self {
            Self::Relu => x.max(0.0),
            Self::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            Self::Tanh => x.tanh(),
        }
    }

    /// The name `ManimML` writes (`"ReLU"`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Relu => "ReLU",
            Self::Sigmoid => "Sigmoid",
            Self::Tanh => "Tanh",
        }
    }

    /// The curve in `bounds`, over an `x` range that shows its shape (-2 to 2 for `Relu`, -5 to 5
    /// for the sigmoid, -2.5 to 2.5 for tanh), with the function's range filling the height.
    pub(super) fn plot(self, bounds: Rect) -> BezPath {
        let (reach, lo, hi) = match self {
            Self::Relu => (2.0, 0.0, 2.0),
            Self::Sigmoid => (5.0, 0.0, 1.0),
            Self::Tanh => (2.5, -1.0, 1.0),
        };
        let mut path = BezPath::new();
        for i in 0..=32 {
            let u = f64::from(i) / 32.0;
            let y = (self.apply(reach * (2.0 * u - 1.0)) - lo) / (hi - lo);
            let p = Point::new(
                bounds.x0 + u * bounds.width(),
                bounds.y1 - y * bounds.height(),
            );
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        path
    }
}

/// The operation of a [`Layer::MathOperation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Operation {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "×",
            Self::Divide => "÷",
        }
    }
}

/// One layer of a [`NeuralNetwork`](super::NeuralNetwork), `ManimML`'s layer classes.
#[derive(Debug, Clone, PartialEq)]
pub enum Layer<'a> {
    /// A column of `nodes` neurons (`FeedForwardLayer`), with its activation function.
    FeedForward {
        nodes: usize,
        activation: Option<ActivationFunction>,
    },
    /// `maps` feature maps of `size` by `size` cells, made by a `filter` by `filter` kernel
    /// (`Convolutional2DLayer`).
    Conv2D {
        maps: usize,
        size: usize,
        filter: usize,
    },
    /// Max pooling with a `kernel` by `kernel` window: the maps of the layer before it, each
    /// `kernel` times smaller (`MaxPooling2DLayer`).
    MaxPool2D { kernel: usize },
    /// An image of the media folder, `size` cells square for the convolutions after it
    /// (`ImageLayer`).
    ImageInput { image: &'a str, size: usize },
    /// A 2D latent space with a cloud of `points` around the origin (`EmbeddingLayer`).
    Embedding { points: usize, seed: u64 },
    /// A column of numbers in brackets (`VectorLayer`).
    Vector { values: Vec<f64> },
    /// An operation on the layers before it (`MathOperationLayer`).
    MathOperation { operation: Operation },
    /// A query image above its positive and negative examples (`PairedQueryLayer`).
    PairedQuery {
        positive: &'a str,
        negative: &'a str,
    },
    /// An anchor image with its positive and negative examples (`TripletLayer`).
    Triplet {
        anchor: &'a str,
        positive: &'a str,
        negative: &'a str,
    },
}

impl Layer<'_> {
    /// A feed-forward layer of `nodes` neurons without an activation function.
    pub fn feed_forward(nodes: usize) -> Self {
        Self::FeedForward {
            nodes,
            activation: None,
        }
    }

    /// The short name drawn on layers that have no picture yet.
    pub(super) fn name(&self) -> &'static str {
        match self {
            Self::FeedForward { .. } => "dense",
            Self::Conv2D { .. } => "conv",
            Self::MaxPool2D { .. } => "pool",
            Self::ImageInput { .. } => "image",
            Self::Embedding { .. } => "embedding",
            Self::Vector { .. } => "vector",
            Self::MathOperation { .. } => "op",
            Self::PairedQuery { .. } => "query",
            Self::Triplet { .. } => "triplet",
        }
    }
}

/// The sizes a network's layout is built from, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Sizes {
    pub node_radius: f64,
    pub node_spacing: f64,
    pub box_width: f64,
    pub box_height: f64,
}

/// Where the parts of one layer are, in pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerGeometry {
    /// The middle of the layer.
    pub center: Point,
    /// The box around the layer.
    pub bounds: Rect,
    /// The neurons of a feed-forward layer, top to bottom.
    pub nodes: Vec<Point>,
}

impl LayerGeometry {
    /// The point connections leave from on the right.
    pub fn out_port(&self) -> Point {
        Point::new(self.bounds.x1, self.center.y)
    }

    /// The point connections arrive at on the left.
    pub fn in_port(&self) -> Point {
        Point::new(self.bounds.x0, self.center.y)
    }
}

/// The width and height of `layer`.
pub(super) fn extent(layer: &Layer<'_>, sizes: Sizes) -> (f64, f64) {
    match layer {
        Layer::FeedForward { nodes, .. } => {
            let n = (*nodes).max(1) as f64;
            (
                2.0 * sizes.node_radius,
                (n - 1.0) * sizes.node_spacing + 2.0 * sizes.node_radius,
            )
        }
        _ => (sizes.box_width, sizes.box_height),
    }
}

/// The left edges of boxes `widths` wide with `spacing` between neighbours, centred on 0.
pub(super) fn place(widths: &[f64], spacing: f64) -> Vec<f64> {
    let total: f64 = widths.iter().sum::<f64>() + spacing * widths.len().saturating_sub(1) as f64;
    let mut left = -total / 2.0;
    widths
        .iter()
        .map(|w| {
            let x = left;
            left += w + spacing;
            x
        })
        .collect()
}

/// The geometry of `layer` with its middle at `center`.
pub(super) fn geometry(layer: &Layer<'_>, center: Point, sizes: Sizes) -> LayerGeometry {
    let (w, h) = extent(layer, sizes);
    let bounds = Rect::from_center_size(center, (w, h));
    let nodes = match layer {
        Layer::FeedForward { nodes, .. } => (0..*nodes)
            .map(|i| {
                Point::new(
                    center.x,
                    bounds.y0 + sizes.node_radius + i as f64 * sizes.node_spacing,
                )
            })
            .collect(),
        _ => Vec::new(),
    };
    LayerGeometry {
        center,
        bounds,
        nodes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_sit_spacing_apart() {
        let left = place(&[20.0, 20.0, 40.0], 100.0);
        assert_eq!(left, vec![-140.0, -20.0, 100.0]);
        // the gap between neighbouring edges is the spacing
        assert_eq!(left[1] - (left[0] + 20.0), 100.0);
        assert_eq!(left[2] - (left[1] + 20.0), 100.0);
        assert_eq!(place(&[], 10.0), Vec::<f64>::new());
    }

    #[test]
    fn feed_forward_nodes_are_spaced_and_centred() {
        let sizes = Sizes {
            node_radius: 10.0,
            node_spacing: 40.0,
            box_width: 60.0,
            box_height: 120.0,
        };
        let g = geometry(&Layer::feed_forward(3), Point::new(100.0, 200.0), sizes);
        let ys: Vec<f64> = g.nodes.iter().map(|p| p.y).collect();
        assert_eq!(ys, vec![160.0, 200.0, 240.0]);
        assert_eq!(g.bounds.height(), 100.0);
        assert_eq!(g.out_port(), Point::new(110.0, 200.0));
    }

    #[test]
    fn activations() {
        assert_eq!(ActivationFunction::Relu.apply(-1.0), 0.0);
        assert_eq!(ActivationFunction::Relu.apply(1.5), 1.5);
        assert!((ActivationFunction::Sigmoid.apply(0.0) - 0.5).abs() < 1e-12);
        assert!((ActivationFunction::Tanh.apply(0.0)).abs() < 1e-12);
    }
}
