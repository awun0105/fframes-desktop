//! Audio: the spectrum of the current frame as bars or as a smooth wave.

mod spectrum;
mod waveform;

pub use spectrum::{BarAlign, SpectrumBars, spectrum};
pub use waveform::{Waveform, wave_path};
