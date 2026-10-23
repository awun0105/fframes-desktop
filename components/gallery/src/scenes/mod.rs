use crate::GalleryScene;

pub mod animate;
pub mod core;
pub mod shapes;
pub mod text;
pub mod transitions;

/// Every gallery scene in play order.
pub fn all() -> Vec<GalleryScene> {
    let mut scenes = core::scenes();
    scenes.extend(shapes::scenes());
    scenes.extend(animate::scenes());
    scenes.extend(transitions::scenes());
    scenes.extend(text::scenes());
    scenes.extend(shapes::bench_scenes());
    scenes
}
