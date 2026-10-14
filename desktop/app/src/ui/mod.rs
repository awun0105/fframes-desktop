//! The bounds probe that qualification telemetry reads. Every control comes
//! from `studio_ui::kit`.

pub mod probe;

pub use probe::{BoundsProbe, bounds_to_record};
