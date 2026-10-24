//! The biology kit (feature `biology`): cells, DNA and the figures of genetics, physiology
//! and ecology. Figures are placed in units of `ui.space` and sized in pixels; anything
//! random is seeded. `docs/components/biology.md` has an example of each with its gallery
//! scene.

mod callouts;
mod cell;
mod circulation;
mod dna;
mod enzyme;
mod expression;
mod food_web;
mod mitosis;
mod neuron;
mod phylo;
mod population;
mod punnett;

pub use cell::{Cell, CellKind, CellPalette, Organelle};
pub use circulation::Circulation;
pub use dna::{BaseColors, DnaHelix, complement};
pub use enzyme::EnzymeFit;
pub use expression::{Transcription, Translation, amino_acid, reading_frame, transcribe};
pub use food_web::FoodWeb;
pub use mitosis::{MitosisStage, MitosisStages};
pub use neuron::{ActionPotential, Neuron, membrane_potential};
pub use phylo::{Newick, NewickError, NewickNode, PhyloTree};
pub use population::{PopulationCurve, PopulationModel};
pub use punnett::{Punnett, PunnettSquare, gametes};
