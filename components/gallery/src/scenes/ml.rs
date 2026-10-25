//! The machine learning kit: neural networks, their layer kinds, their forward pass,
//! dropout, layer changes, a variational autoencoder and decision trees.

use fframes_kit::prelude::*;

use crate::GalleryScene;

use super::core::title;

/// A small classifier: pulses run from the inputs to the outputs, lighting each layer.
fn forward_pass<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ForwardPass");
    let relu = Some(ActivationFunction::Relu);
    let network = ui.show(NeuralNetwork {
        layers: vec![
            Layer::feed_forward(4),
            Layer::FeedForward {
                nodes: 6,
                activation: relu,
            },
            Layer::FeedForward {
                nodes: 6,
                activation: relu,
            },
            Layer::FeedForward {
                nodes: 3,
                activation: Some(ActivationFunction::Sigmoid),
            },
        ],
        center: (0.0, -0.3),
        spacing: 240.0,
        node_radius: 20.0,
        node_spacing: 66.0,
        forward_pass: Some(ForwardPass {
            start: 0.5,
            duration: 3.0,
        }),
        enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    svgr!(<g>{head}{network}</g>)
}

/// An image through a convolution and a max pooling into two dense layers: the filters slide
/// over the maps, then the pulses run on to the outputs.
fn conv_net<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ConvNet");
    let network = ui.show(NeuralNetwork {
        layers: vec![
            Layer::ImageInput {
                image: "photo.png",
                size: 8,
            },
            Layer::Conv2D {
                maps: 3,
                size: 6,
                filter: 3,
            },
            Layer::MaxPool2D { kernel: 2 },
            Layer::feed_forward(5),
            Layer::FeedForward {
                nodes: 2,
                activation: Some(ActivationFunction::Sigmoid),
            },
        ],
        center: (0.0, -0.3),
        spacing: 150.0,
        cell: 26.0,
        node_radius: 18.0,
        node_spacing: 58.0,
        forward_pass: Some(ForwardPass {
            start: 1.0,
            duration: 3.6,
        }),
        enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    svgr!(<g>{head}{network}</g>)
}

/// The other layer kinds: a triplet into an embedding above a paired query through a vector
/// and two operations.
fn layer_kinds<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "LayerKinds");
    let pass = |start| {
        Some(ForwardPass {
            start,
            duration: 2.4,
        })
    };
    let metric = ui.show(NeuralNetwork {
        layers: vec![
            Layer::Triplet {
                anchor: "photo.png",
                positive: "photo.png",
                negative: "photo.png",
            },
            Layer::feed_forward(4),
            Layer::Embedding {
                points: 160,
                seed: 7,
            },
        ],
        center: (-3.4, -0.4),
        spacing: 170.0,
        node_radius: 18.0,
        node_spacing: 60.0,
        forward_pass: pass(0.5),
        enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    let arithmetic = ui.show(NeuralNetwork {
        layers: vec![
            Layer::PairedQuery {
                positive: "photo.png",
                negative: "photo.png",
            },
            Layer::Vector {
                values: vec![0.82, -0.4, 1.25],
            },
            Layer::MathOperation {
                operation: Operation::Multiply,
            },
            Layer::feed_forward(3),
            Layer::MathOperation {
                operation: Operation::Divide,
            },
        ],
        center: (3.1, -0.4),
        spacing: 100.0,
        node_radius: 18.0,
        node_spacing: 60.0,
        forward_pass: pass(1.0),
        enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    svgr!(<g>{head}{metric}{arithmetic}</g>)
}

/// A network grows a layer, swaps one for another and loses one again, its connections
/// following.
fn layer_changes<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "LayerChanges");
    let change = |change, start| TimedChange {
        change,
        start,
        duration: 0.9,
    };
    let network = ui.show(NeuralNetwork {
        layers: vec![
            Layer::feed_forward(3),
            Layer::feed_forward(5),
            Layer::feed_forward(3),
        ],
        changes: vec![
            change(
                LayerChange::Insert {
                    index: 2,
                    layer: Layer::FeedForward {
                        nodes: 4,
                        activation: Some(ActivationFunction::Relu),
                    },
                },
                0.6,
            ),
            change(
                LayerChange::Replace {
                    index: 1,
                    layer: Layer::FeedForward {
                        nodes: 6,
                        activation: Some(ActivationFunction::Tanh),
                    },
                },
                2.0,
            ),
            change(LayerChange::Remove { index: 2 }, 3.4),
        ],
        center: (0.0, -0.3),
        spacing: 220.0,
        node_radius: 20.0,
        node_spacing: 66.0,
        enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    svgr!(<g>{head}{network}</g>)
}

/// A forward pass with dropout: some hidden neurons are crossed out and carry nothing.
fn dropout<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Dropout");
    let network = ui.show(NeuralNetwork {
        layers: vec![
            Layer::feed_forward(4),
            Layer::feed_forward(7),
            Layer::feed_forward(7),
            Layer::feed_forward(3),
        ],
        center: (0.0, -0.3),
        spacing: 260.0,
        node_radius: 20.0,
        node_spacing: 62.0,
        forward_pass: Some(ForwardPass {
            start: 1.3,
            duration: 2.2,
        }),
        dropout: Some(Dropout {
            rate: 0.35,
            seed: 7,
            start: 0.9,
            duration: 2.9,
        }),
        enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    svgr!(<g>{head}{network}</g>)
}

/// A variational autoencoder: the image is encoded to a point of the latent Gaussian and
/// decoded back, the point appearing as the pulses reach the latent space.
fn vae<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Vae");
    let vae = ui.show(VariationalAutoencoder {
        input: "photo.png",
        output: "photo.png",
        center: (0.0, -0.3),
        spacing: 120.0,
        cell: 18.0,
        node_radius: 18.0,
        node_spacing: 56.0,
        forward_pass: Some(ForwardPass {
            start: 0.9,
            duration: 3.2,
        }),
        enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    svgr!(<g>{head}{vae}</g>)
}

/// The iris tree: petal length, then petal width.
fn iris() -> DecisionTree<'static> {
    DecisionTree {
        nodes: vec![
            DecisionNode::Split {
                feature: 0,
                threshold: 2.45,
                left: 1,
                right: 2,
            },
            DecisionNode::Leaf { class: 0 },
            DecisionNode::Split {
                feature: 1,
                threshold: 1.75,
                left: 3,
                right: 4,
            },
            DecisionNode::Leaf { class: 1 },
            DecisionNode::Leaf { class: 2 },
        ],
        features: vec!["petal length", "petal width"],
        classes: vec!["setosa", "versicolor", "virginica"],
    }
}

/// A decision tree grows node by node while the feature space beside it is cut into the
/// regions of its leaves.
fn decision_tree<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "DecisionTree");
    let reveal = Some(TreeReveal {
        order: RevealOrder::BreadthFirst,
        start: 0.9,
        step: 0.6,
    });
    let diagram = ui.show(DecisionTreeDiagram {
        tree: iris(),
        center: (-3.3, -0.3),
        level: 140.0,
        reveal,
        enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    let axes = Axes {
        x_range: Range::new(0.0, 7.0, 1.0),
        y_range: Range::new(0.0, 3.0, 0.5),
        x_length: 5.0,
        y_length: 4.2,
        center: (3.4, -0.4),
        tips: false,
        x_numbers: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        y_numbers: vec![1.0, 2.0, 3.0],
        x_label: Some("length".into()),
        y_label: Some("width".into()),
        labels_enter: Some(FadeIn::default()),
        ..ui.defaults()
    };
    let surface = ui.show(DecisionSurface {
        tree: iris(),
        axes: axes.map(ui),
        reveal,
        enter: Some(FadeIn::default()),
        ..ui.defaults()
    });
    let axes = ui.show(axes);
    svgr!(<g>{head}{diagram}{surface}{axes}</g>)
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("ForwardPass", 4.0, forward_pass),
        GalleryScene::new("ConvNet", 5.0, conv_net),
        GalleryScene::new("LayerKinds", 4.0, layer_kinds),
        GalleryScene::new("Dropout", 4.0, dropout),
        GalleryScene::new("LayerChanges", 5.0, layer_changes),
        GalleryScene::new("Vae", 5.0, vae),
        GalleryScene::new("DecisionTree", 5.0, decision_tree),
    ]
}
