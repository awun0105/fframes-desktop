//! The biology kit (feature `biology`): cells, DNA and the figures of genetics, physiology
//! and ecology. Figures are placed in units of `ui.space` and sized in pixels; anything
//! random is seeded. `docs/components/biology.md` has an example of each with its gallery
//! scene.

mod callouts;
mod cell;
mod dna;
mod expression;
mod mitosis;
mod neuron;

pub use cell::{Cell, CellKind, CellPalette, Organelle};
pub use dna::{BaseColors, DnaHelix, complement};
pub use expression::{Transcription, Translation, amino_acid, reading_frame, transcribe};
pub use mitosis::{MitosisStage, MitosisStages};
pub use neuron::{ActionPotential, Neuron, membrane_potential};
