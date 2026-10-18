//! The timeline, after Cap's editor timeline: the engine's transform with a ruler, the
//! clip track from the timing document, overlay and audio rows, the mix waveform and a
//! minimap. The shell mounts the view cached and paints the playhead over it.
mod edit;
mod minimap;
mod model;
mod playhead;
mod ruler;
mod segment;
mod view;
mod waveform;

pub use playhead::{PlayheadGeometry, playhead_overlay};
pub use view::{KEY_CONTEXT, TimelineEvent, TimelineView};
