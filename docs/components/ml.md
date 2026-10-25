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
  box above it.
- **Other kinds.** `Layer::Embedding { points, seed }` is a box with a cloud of `points`
  points drawn from a tilted 2D normal distribution (`Gaussian2D`), the same for the same
  `seed`. `Layer::Vector { values }` writes the numbers in brackets, with at most two
  decimals. `Layer::MathOperation { operation }` is a circle with the `Operation`'s symbol
  (`Add`, `Subtract`, `Multiply`, `Divide`). `Layer::PairedQuery { positive, negative }` and
  `Layer::Triplet { anchor, positive, negative }` stack images of the media folder, the
  positive outlined in `positive_color` and the negative in `negative_color`; a missing image
  leaves its square empty. A pooling after a layer without maps is a box named `pool`.
- **Feature maps.** `Layer::ImageInput { image, size }` draws an image of the media folder as
  a map `size` cells square; a missing image leaves the map empty. `Layer::Conv2D { maps,
  size, filter }` draws `maps` feature maps of `size` cells, `Layer::MaxPool2D { kernel }` the maps
  of the layer before it, `kernel` times smaller. Maps are squares `cell` pixels a cell, seen
  at an angle and standing one behind the other (at most 8 are drawn), with the lines between
  their cells up to 16 cells.
- **Connections.** Each pair of neighbouring layers is joined by the drawing of its kind:
  every neuron to every neuron between two feed-forward layers, four lines from the window a
  convolution or a pooling slides over the maps before it to the cell it makes, a fan from
  every neuron or every corner of a map to the next layer's neurons or side, and an arrow
  between two boxes.
- **Forward pass.** `forward_pass: Some(ForwardPass { start, duration })` sends pulses
  through the connections one after the other. Each connection gets an equal part of the
  time, so the pulses reach the last layer at `start + duration`. Between two layers with maps
  the window slides over the map instead, row by row from the top left, gliding from each
  position to the next: one cell along for a convolution, a whole kernel for a pooling. A layer lights up in
  `pulse_color` as they arrive and fades back while they travel on.
- **Dropout.** `dropout: Some(Dropout { rate, seed, start, duration })` crosses out a share
  `rate` of the neurons of every hidden feed-forward layer from `start` for `duration`
  seconds: they dim, do not light up, and their edges and pulses fade. The neurons are
  picked once, the same for the same `seed`; the input and output layers keep all of theirs
  and every layer keeps at least one.
- **Layer changes.** `changes: vec![TimedChange { change, start, duration }]` runs
  `LayerChange::Insert { index, layer }`, `Remove { index }` and `Replace { index, layer }`
  one after the other. The other layers slide to their new places, the layer that comes or
  goes fades, and so do the connections that are new or gone. An index counts the layers as
  the changes before it left them; one out of range changes nothing. Give the changes in the
  order of their starts: a change that starts before the previous one has finished cuts it
  short. A change should not overlap a forward pass or dropout.
- **Layout.** `layout(ui)` gives every layer's `LayerGeometry` in pixels (its middle, its box,
  its neurons, maps, points, rows and images), for a label or an arrow pointing at a layer.
  It is the layout of `layers` as given, before any change.

Gallery scenes: `ForwardPass`, `ConvNet` (an image through a convolution and a pooling),
`LayerKinds` (a triplet into an embedding, a paired query through a vector and two
operations), `Dropout` and `LayerChanges` (an insert, a replace and a remove).

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

## VariationalAutoencoder

`VariationalAutoencoder` draws ManimML's autoencoder as a `NeuralNetwork`: the image `input`,
the feed-forward layers of the `encoder` (neurons per layer, `[5, 3]` by default), the latent
space, the `decoder` (`[3, 5]`) and the image `output`, both `size` cells square. `layers()`
gives these layers and `latent_index()` the place of the latent space among them.

- **Latent space.** An embedding box with a cloud of `points` points for `seed` and the 1σ
  and 2σ rings of its Gaussian in `ring_color`.
- **Code.** The image's point in the latent space, drawn in `pulse_color`, is `μ + L·epsilon`
  (`L` the Cholesky factor of the covariance, the reparameterisation trick). `epsilon` is in
  units of the standard normal distribution, `(1.3, -0.7)` by default; the point stays inside
  the box.
- **Forward pass.** With `forward_pass` the pulses run from the input to the output, and the
  point appears as they reach the latent space. Without one it is always shown.

Gallery scene: `Vae`.

```rust
use fframes_kit::prelude::*;

fn autoencoder<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(VariationalAutoencoder {
        input: "digit.png",
        output: "digit.png",
        epsilon: (0.4, 1.1),
        forward_pass: Some(ForwardPass { start: 0.9, duration: 3.2 }),
        enter: Some(FadeIn::default()),
        ..ui.defaults()
    })
}
```
