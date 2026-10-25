//! The machine learning kit (feature `ml`): `ManimML`'s neural networks with their layers,
//! connections and forward pass. Weights and activations are data you pass; the kit runs no
//! model.

mod connections;
mod forward;
mod layers;
mod network;

pub use forward::ForwardPass;
pub use layers::{ActivationFunction, Layer, LayerGeometry, Operation};
pub use network::NeuralNetwork;
