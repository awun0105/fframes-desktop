//! The machine learning kit: neural networks and their forward pass.

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

pub fn scenes() -> Vec<GalleryScene> {
    vec![GalleryScene::new("ForwardPass", 4.0, forward_pass)]
}
