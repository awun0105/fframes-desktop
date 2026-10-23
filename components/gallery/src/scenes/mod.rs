use crate::GalleryScene;

pub mod core;

/// Every gallery scene in play order.
pub fn all() -> Vec<GalleryScene> {
    core::scenes()
}
