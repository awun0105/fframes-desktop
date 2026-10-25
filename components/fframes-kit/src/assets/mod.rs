//! Asset packs for screen recordings and social clips: drawn [`Cursor`]s with
//! [`CursorPath`], [`Click`], [`Drag`] and [`TypeAt`]; Noto animated emoji in [`emoji`]
//! (`AnimatedEmoji` with the `lottie` feature); the CC0 sound effects of [`sfx`]; and
//! `Gif` with the `gif` feature.

mod cursor;
pub mod emoji;
#[cfg(feature = "gif")]
mod gif;
pub mod sfx;

pub use cursor::{
    Click, ClickLook, Cursor, CursorMotion, CursorPath, CursorPoint, CursorShape, CursorStyle,
    Drag, TypeAt,
};
#[cfg(feature = "lottie")]
pub use emoji::AnimatedEmoji;
pub use emoji::SkinTone;
#[cfg(feature = "gif")]
pub use gif::{Gif, GifData, GifError, GifFit, frame_index};
pub use sfx::Sfx;
