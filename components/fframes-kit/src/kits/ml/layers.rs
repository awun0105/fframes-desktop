//! The layers of a [`NeuralNetwork`](super::NeuralNetwork) and where their parts go.

use kurbo::{BezPath, Point, Rect};

use super::probability::Gaussian2D;

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

    /// The short name drawn on a layer with nothing else to show: a pooling after a layer
    /// without maps.
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
    /// One cell of a feature map.
    pub cell: f64,
}

/// The feature maps of a layer: how many, and how many cells on a side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapShape {
    pub maps: usize,
    pub size: usize,
}

/// The feature maps of every layer: an image is one map, a convolution has its own, a
/// max pooling has the maps of the layer before it, `kernel` times smaller. Other layers, and
/// a pooling after a layer without maps, have none.
pub(super) fn map_shapes(layers: &[&Layer<'_>]) -> Vec<Option<MapShape>> {
    let mut shapes: Vec<Option<MapShape>> = Vec::with_capacity(layers.len());
    for layer in layers {
        let shape = match *layer {
            Layer::ImageInput { size, .. } => Some(MapShape {
                maps: 1,
                size: (*size).max(1),
            }),
            Layer::Conv2D { maps, size, .. } => Some(MapShape {
                maps: (*maps).max(1),
                size: (*size).max(1),
            }),
            Layer::MaxPool2D { kernel } => shapes.last().copied().flatten().map(|s| MapShape {
                maps: s.maps,
                size: (s.size / (*kernel).max(1)).max(1),
            }),
            _ => None,
        };
        shapes.push(shape);
    }
    shapes
}

/// At most this many maps of a layer are drawn.
const DRAWN_MAPS: usize = 8;

/// The panels of `shape`: each map a square `side` pixels high seen at an angle, so its
/// right edge is higher than its left, each map in front of the one before.
fn panel_sizes(shape: MapShape, sizes: Sizes) -> (f64, f64, f64, f64) {
    let side =
        (shape.size as f64 * sizes.cell).clamp(sizes.node_radius * 3.0, sizes.box_height * 1.6);
    let width = side * 0.5;
    let rise = side * 0.25;
    let step = width * 0.35;
    (side, width, rise, step)
}

/// Where the parts of one layer are, in pixels.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayerGeometry {
    /// The middle of the layer.
    pub center: Point,
    /// The box around the layer.
    pub bounds: Rect,
    /// The neurons of a feed-forward layer, top to bottom.
    pub nodes: Vec<Point>,
    /// The drawn feature maps, back to front, each as its corners: top left, top right,
    /// bottom right, bottom left.
    pub maps: Vec<[Point; 4]>,
    /// The cells on a side of each map.
    pub cells: usize,
    /// The window this layer slides over the maps of the layer before it, in cells: the
    /// filter of a convolution, the kernel of a pooling, 1 for the others.
    pub window: usize,
    /// How many cells the window moves at a time: 1 for a convolution, the kernel for a
    /// pooling, whose windows do not overlap.
    pub stride: usize,
    /// The point cloud of an embedding.
    pub points: Vec<Point>,
    /// The middle of each number of a vector, top to bottom.
    pub rows: Vec<Point>,
    /// The images of a paired query or a triplet, top to bottom.
    pub images: Vec<Rect>,
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

    /// The point at `(u, v)` of the map `map` (0 to 1 from its top left corner).
    pub fn map_point(&self, map: usize, u: f64, v: f64) -> Option<Point> {
        let [tl, tr, _, bl] = *self.maps.get(map)?;
        Some(tl + (tr - tl) * u + (bl - tl) * v)
    }
}

/// The width and height of `layer` with the feature maps `shape`.
pub(super) fn extent(layer: &Layer<'_>, shape: Option<MapShape>, sizes: Sizes) -> (f64, f64) {
    if let Some(shape) = shape {
        let (side, width, rise, step) = panel_sizes(shape, sizes);
        let drawn = shape.maps.min(DRAWN_MAPS) as f64;
        return (width + (drawn - 1.0) * step, side + rise);
    }
    match layer {
        Layer::FeedForward { nodes, .. } => {
            let n = (*nodes).max(1) as f64;
            (
                2.0 * sizes.node_radius,
                (n - 1.0) * sizes.node_spacing + 2.0 * sizes.node_radius,
            )
        }
        Layer::Embedding { .. } => (sizes.box_height * 0.8, sizes.box_height * 0.8),
        Layer::Vector { values } => (
            sizes.box_width,
            values.len().max(1) as f64 * vector_row(sizes) + sizes.node_radius,
        ),
        Layer::MathOperation { .. } => (sizes.node_radius * 2.8, sizes.node_radius * 2.8),
        Layer::PairedQuery { .. } => image_stack(2, sizes),
        Layer::Triplet { .. } => image_stack(3, sizes),
        _ => (sizes.box_width, sizes.box_height),
    }
}

/// The height of one number of a vector.
fn vector_row(sizes: Sizes) -> f64 {
    sizes.node_radius * 1.7
}

/// The side of the images of a paired query or a triplet and the gap between them.
fn image_side(sizes: Sizes) -> (f64, f64) {
    (sizes.box_width * 1.1, sizes.node_radius * 0.6)
}

/// The size of `n` images above each other.
fn image_stack(n: usize, sizes: Sizes) -> (f64, f64) {
    let (side, gap) = image_side(sizes);
    (side, n as f64 * side + (n - 1) as f64 * gap)
}

/// The covariance of an embedding's cloud, in units of its box's half width: tilted so the
/// cloud reads as data, not noise.
pub(super) const EMBEDDING: Gaussian2D = Gaussian2D {
    mean: (0.0, 0.0),
    covariance: [[0.1, 0.05], [0.05, 0.06]],
};

/// The pixel of the embedding box `bounds` at `(x, y)` in units of its half width, `y` up.
pub(super) fn embedding_px(bounds: Rect, (x, y): (f64, f64)) -> Point {
    let half = bounds.width() / 2.0;
    Point::new(bounds.center().x + x * half, bounds.center().y - y * half)
}

/// `points` points of the embedding cloud for `seed` in `bounds`, those that would fall
/// outside it pulled in to its edge.
fn embedding_points(points: usize, seed: u64, bounds: Rect, margin: f64) -> Vec<Point> {
    let inner = bounds.inset(-margin);
    EMBEDDING
        .samples(seed, points)
        .into_iter()
        .map(|sample| {
            // y grows downwards on screen, so the cloud rises to the right
            let p = embedding_px(bounds, sample);
            Point::new(p.x.clamp(inner.x0, inner.x1), p.y.clamp(inner.y0, inner.y1))
        })
        .collect()
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

/// The geometry of `layer`, with the feature maps `shape`, with its middle at `center`.
pub(super) fn geometry(
    layer: &Layer<'_>,
    shape: Option<MapShape>,
    center: Point,
    sizes: Sizes,
) -> LayerGeometry {
    let (w, h) = extent(layer, shape, sizes);
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
    let maps = shape.map_or_else(Vec::new, |shape| {
        let (side, width, rise, step) = panel_sizes(shape, sizes);
        (0..shape.maps.min(DRAWN_MAPS))
            .map(|k| {
                let left = bounds.x0 + k as f64 * step;
                let top = bounds.y0;
                [
                    Point::new(left, top + rise),
                    Point::new(left + width, top),
                    Point::new(left + width, top + side),
                    Point::new(left, top + rise + side),
                ]
            })
            .collect()
    });
    let window = match layer {
        Layer::Conv2D { filter, .. } => (*filter).max(1),
        Layer::MaxPool2D { kernel } => (*kernel).max(1),
        _ => 1,
    };
    let stride = match layer {
        Layer::MaxPool2D { kernel } => (*kernel).max(1),
        _ => 1,
    };
    let points = match layer {
        Layer::Embedding { points, seed } => {
            embedding_points(*points, *seed, bounds, sizes.node_radius * 0.5)
        }
        _ => Vec::new(),
    };
    let rows = match layer {
        Layer::Vector { values } => {
            let row = vector_row(sizes);
            let top = bounds.y0 + sizes.node_radius / 2.0 + row / 2.0;
            (0..values.len())
                .map(|i| Point::new(center.x, top + i as f64 * row))
                .collect()
        }
        _ => Vec::new(),
    };
    let images = match layer {
        Layer::PairedQuery { .. } | Layer::Triplet { .. } => {
            let (side, gap) = image_side(sizes);
            let n = if matches!(layer, Layer::Triplet { .. }) {
                3
            } else {
                2
            };
            (0..n)
                .map(|i| {
                    let top = bounds.y0 + f64::from(i) * (side + gap);
                    Rect::new(bounds.x0, top, bounds.x0 + side, top + side)
                })
                .collect()
        }
        _ => Vec::new(),
    };
    LayerGeometry {
        center,
        bounds,
        nodes,
        maps,
        cells: shape.map_or(0, |s| s.size),
        window,
        stride,
        points,
        rows,
        images,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZES: Sizes = Sizes {
        node_radius: 10.0,
        node_spacing: 40.0,
        box_width: 60.0,
        box_height: 120.0,
        cell: 8.0,
    };

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
        let g = geometry(
            &Layer::feed_forward(3),
            None,
            Point::new(100.0, 200.0),
            SIZES,
        );
        let ys: Vec<f64> = g.nodes.iter().map(|p| p.y).collect();
        assert_eq!(ys, vec![160.0, 200.0, 240.0]);
        assert_eq!(g.bounds.height(), 100.0);
        assert_eq!(g.out_port(), Point::new(110.0, 200.0));
    }

    #[test]
    fn pooling_shrinks_the_maps_before_it() {
        let layers = [
            Layer::ImageInput {
                image: "digit.png",
                size: 28,
            },
            Layer::Conv2D {
                maps: 4,
                size: 24,
                filter: 5,
            },
            Layer::MaxPool2D { kernel: 2 },
            Layer::feed_forward(3),
            Layer::MaxPool2D { kernel: 2 },
        ];
        let shapes = map_shapes(&layers.iter().collect::<Vec<_>>());
        assert_eq!(shapes[0], Some(MapShape { maps: 1, size: 28 }));
        assert_eq!(shapes[2], Some(MapShape { maps: 4, size: 12 }));
        assert_eq!(shapes[3], None);
        // a pooling after neurons has nothing to pool
        assert_eq!(shapes[4], None);
    }

    #[test]
    fn maps_stand_behind_each_other_inside_the_box() {
        let shape = MapShape { maps: 3, size: 10 };
        let layer = Layer::Conv2D {
            maps: 3,
            size: 10,
            filter: 3,
        };
        let g = geometry(&layer, Some(shape), Point::new(0.0, 0.0), SIZES);
        assert_eq!(g.maps.len(), 3);
        assert_eq!((g.cells, g.window), (10, 3));
        // 80 pixels high, 40 wide, rising 20, each map 14 to the right of the one before
        assert_eq!(g.bounds.size(), (68.0, 100.0).into());
        assert_eq!(g.maps[0][0], Point::new(-34.0, -30.0));
        assert_eq!(g.maps[2][1], Point::new(34.0, -50.0));
        assert_eq!(g.map_point(0, 0.0, 1.0), Some(Point::new(-34.0, 50.0)));
        let b = g.bounds;
        assert!(
            g.maps
                .iter()
                .flatten()
                .all(|p| (b.x0..=b.x1).contains(&p.x) && (b.y0..=b.y1).contains(&p.y))
        );
    }

    #[test]
    fn the_other_kinds_place_their_parts_inside_the_box() {
        let at = Point::new(0.0, 0.0);
        let embedding = Layer::Embedding {
            points: 200,
            seed: 3,
        };
        let g = geometry(&embedding, None, at, SIZES);
        assert_eq!(g.points.len(), 200);
        assert_eq!(g.points, geometry(&embedding, None, at, SIZES).points);
        assert!(g.points.iter().all(|p| g.bounds.contains(*p)));
        // the cloud rises to the right
        let right: Vec<&Point> = g.points.iter().filter(|p| p.x > 20.0).collect();
        let left: Vec<&Point> = g.points.iter().filter(|p| p.x < -20.0).collect();
        let mean_y = |ps: &[&Point]| ps.iter().map(|p| p.y).sum::<f64>() / ps.len() as f64;
        assert!(mean_y(&right) < mean_y(&left));

        let vector = Layer::Vector {
            values: vec![0.5, -1.0, 2.0],
        };
        let g = geometry(&vector, None, at, SIZES);
        // three rows of 17 and half a radius above and below
        assert_eq!(g.bounds.height(), 61.0);
        let ys: Vec<f64> = g.rows.iter().map(|p| p.y).collect();
        assert_eq!(ys, vec![-17.0, 0.0, 17.0]);

        let triplet = Layer::Triplet {
            anchor: "a.png",
            positive: "p.png",
            negative: "n.png",
        };
        let g = geometry(&triplet, None, at, SIZES);
        assert_eq!(g.images.len(), 3);
        // 66 pixel squares 6 apart
        assert_eq!(g.bounds.size(), (66.0, 210.0).into());
        assert_eq!(g.images[1].y0 - g.images[0].y1, 6.0);
        assert_eq!(g.images[2].y1, g.bounds.y1);
        let paired = Layer::PairedQuery {
            positive: "p.png",
            negative: "n.png",
        };
        assert_eq!(geometry(&paired, None, at, SIZES).images.len(), 2);
    }

    #[test]
    fn activations() {
        assert_eq!(ActivationFunction::Relu.apply(-1.0), 0.0);
        assert_eq!(ActivationFunction::Relu.apply(1.5), 1.5);
        assert!((ActivationFunction::Sigmoid.apply(0.0) - 0.5).abs() < 1e-12);
        assert!((ActivationFunction::Tanh.apply(0.0)).abs() < 1e-12);
    }
}
