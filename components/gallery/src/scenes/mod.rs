use crate::GalleryScene;

pub mod animate;
pub mod core;
pub mod shapes;

/// Every gallery scene in play order.
pub fn all() -> Vec<GalleryScene> {
    let mut scenes = core::scenes();
    scenes.extend(shapes::scenes());
    scenes.extend(animate::scenes());
    scenes
}
