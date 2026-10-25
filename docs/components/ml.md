# fframes-kit: machine learning kit

The machine learning kit is the `ml` feature of `fframes-kit`
(`fframes-kit = { .., features = ["ml"] }`) and needs no other crate. It builds for wasm32,
so every figure here also runs in the editor. It draws ManimML's figures; it runs no model.
Weights, activations and samples are data you pass. Every entry names its scene in the
components gallery (`cargo run --release -p components-gallery -- strip <Scene> -n 8`).

- **Placement.** A figure is placed in units of `ui.space` (Manim's frame: 8 units tall, `y`
  up, the origin in the middle). Its sizes are in pixels.
- **Time.** Animations are props with a start and a duration in seconds, sampled at the
  frame's time, so any frame can be rendered on its own.

## NeuralNetwork

`NeuralNetwork` draws `layers` from left to right around `center`, `spacing` pixels between
neighbouring layers.

- **Layers.** `Layer::FeedForward { nodes, activation }` is a column of neurons
  `node_radius` pixels round and `node_spacing` apart (`Layer::feed_forward(n)` has no
  activation). Its `ActivationFunction` (`Relu`, `Sigmoid` or `Tanh`) is plotted in a small
  box above it. Layer kinds without a picture of their own are drawn as a named box.
- **Connections.** Each pair of neighbouring layers is joined by the drawing of its kind:
  every neuron to every neuron between two feed-forward layers, a fan from every neuron to
  the side of a box, and an arrow between two boxes.
- **Forward pass.** `forward_pass: Some(ForwardPass { start, duration })` sends pulses
  through the connections one after the other. Each connection gets an equal part of the
  time, so the pulses reach the last layer at `start + duration`. A layer lights up in
  `pulse_color` as they arrive and fades back while they travel on.
- **Layout.** `layout(ui)` gives every layer's `LayerGeometry` (its middle, its box and its
  neurons in pixels), for a label or an arrow pointing at a layer.

Gallery scene: `ForwardPass`.

```rust
use fframes_kit::prelude::*;

fn network<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let relu = Some(ActivationFunction::Relu);
    ui.show(NeuralNetwork {
        layers: vec![
            Layer::feed_forward(4),
            Layer::FeedForward { nodes: 6, activation: relu },
            Layer::FeedForward { nodes: 3, activation: Some(ActivationFunction::Sigmoid) },
        ],
        forward_pass: Some(ForwardPass { start: 0.5, duration: 3.0 }),
        ..ui.defaults()
    })
}
```
