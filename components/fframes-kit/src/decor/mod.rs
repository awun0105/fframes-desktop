//! Decoration: strokes that draw on, crop marks, shadow and glow filters, framed photos.

mod corners;
mod draw_on;
mod filters;
mod photo_frame;

pub use corners::CornerMarks;
pub use draw_on::DrawOn;
pub use filters::{DropShadow, Glow};
pub use photo_frame::PhotoFrame;
