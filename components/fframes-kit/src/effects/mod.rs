//! Effects over or around other content: [`Trail`] and [`CameraMotionBlur`] (Remotion's
//! motion blur), [`LightLeak`], [`Blur`] and film grain with [`Noise`] (and [`ShaderNoise`]
//! on the GPU with the `skia` feature). The glow and drop shadow filters are
//! [`Glow`](crate::decor::Glow) and [`DropShadow`](crate::decor::DropShadow).

mod blur;
mod light_leak;
mod motion_blur;
mod noise;

pub use blur::Blur;
pub use light_leak::{Flare, LightLeak};
pub use motion_blur::{CameraMotionBlur, MAX_SAMPLES, Trail};
pub use noise::Noise;
#[cfg(feature = "skia")]
pub use noise::{GRAIN_SKSL, ShaderNoise, grain_shader};
