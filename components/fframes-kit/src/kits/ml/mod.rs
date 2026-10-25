//! The machine learning kit (feature `ml`): `ManimML`'s neural networks with their layers,
//! connections and forward pass. Weights and activations are data you pass; the kit runs no
//! model.

mod changes;
mod connections;
mod decision_tree;
mod forward;
mod layers;
mod network;
mod probability;
mod vae;

pub use changes::{LayerChange, TimedChange};
pub use decision_tree::{
    DecisionNode, DecisionSurface, DecisionTree, DecisionTreeDiagram, Partition, Reached,
    RevealOrder, TreeReveal,
};
pub use forward::{Dropout, ForwardPass};
pub use layers::{ActivationFunction, Layer, LayerGeometry, MapShape, Operation};
pub use network::NeuralNetwork;
pub use probability::{ChainRun, Gaussian2D, GaussianEllipse, McmcChain, McmcStep};
pub use vae::VariationalAutoencoder;
