use crate::GalleryScene;

pub mod animate;
pub mod chemistry;
pub mod core;
pub mod cs;
pub mod explain;
pub mod general;
pub mod latex;
pub mod math;
pub mod overlay;
pub mod physics;
pub mod shapes;
pub mod tags;
pub mod text;
pub mod transitions;

/// Every gallery scene in play order.
pub fn all() -> Vec<GalleryScene> {
    let mut scenes = core::scenes();
    scenes.extend(shapes::scenes());
    scenes.extend(animate::scenes());
    scenes.extend(transitions::scenes());
    scenes.extend(text::scenes());
    scenes.extend(overlay::scenes());
    scenes.extend(general::scenes());
    scenes.extend(latex::scenes());
    scenes.extend(explain::scenes());
    scenes.extend(math::scenes());
    scenes.extend(physics::scenes());
    scenes.extend(chemistry::scenes());
    scenes.extend(cs::scenes());
    scenes.extend(tags::scenes());
    scenes.extend(shapes::bench_scenes());
    scenes.extend(latex::bench_scenes());
    scenes
}
