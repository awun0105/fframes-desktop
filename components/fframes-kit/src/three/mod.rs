//! Manim's 3D scene and mobjects (feature `3d`), drawn with `fframes::three_d`.
//!
//! Positions are Manim's: units of `ui.space`, `z` out of the screen. In the renderer's
//! world, where `y` is up, the point `(x, y, z)` is `(x, z, -y)`. A [`ThreeDView`] renders
//! its [`Object3D`]s into one image over the canvas, then draws their 2D overlays (labels,
//! fixed-in-frame text) on top.
//!
//! Every prop is plain data and the scene is built again for every frame, geometry
//! included. On the CPU rasterizer that is a small part of the frame; the wgpu renderer
//! caches meshes by their `Arc`, which a rebuilt mesh does not hit.

use fframes::{
    Color, Svgr,
    three_d::{Geometry, Material, Node, Scene, Vec3, vec3},
};
use kurbo::Point;

use crate::{
    anim::FadeIn,
    animate::{Create, enter_alpha, grow_scale},
    kits::stage::rgb,
    ui::Ui,
};

mod axes;
mod polyhedra;
mod solids;
mod surface;
mod view;

pub use axes::ThreeDAxes;
pub use polyhedra::{
    ConvexHull3D, Dodecahedron, Icosahedron, MAX_HULL_POINTS, Octahedron, Polyhedron, Tetrahedron,
    convex_hull_faces,
};
pub use solids::{Arrow3D, Cone, Cube, Cylinder, Dot3D, Line3D, Prism, Sphere, Torus};
pub use surface::Surface;
pub use view::{Projector, ThreeDView};

/// A point in Manim's units, `z` out of the screen.
pub type Point3 = (f32, f32, f32);

/// Manim's point `(x, y, z)` in the renderer's world, where `y` is up.
pub fn world(p: Point3) -> Vec3 {
    vec3(p.0, p.2, -p.1)
}

/// Something a [`ThreeDView`] draws: it adds its nodes to the scene, and may draw 2D on top
/// of the picture, placed with the view's [`Projector`].
pub trait Object3D {
    /// Adds the object to `scene` as it is at `ui`'s time.
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene);

    /// 2D drawn over the picture, in canvas pixels.
    fn overlay<'a>(&self, _ui: &mut Ui<'a, '_, '_>, _projector: &Projector) -> Svgr<'a> {
        Svgr::empty()
    }
}

/// The `enter` prop of 3D mobjects: the kit's [`Create`] grows the object from its own
/// center, [`FadeIn`] fades it in (its 2D `shift` does not apply).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Enter3D {
    Create(Create),
    FadeIn(FadeIn),
}

impl From<Create> for Enter3D {
    fn from(a: Create) -> Self {
        Enter3D::Create(a)
    }
}

impl From<FadeIn> for Enter3D {
    fn from(a: FadeIn) -> Self {
        Enter3D::FadeIn(a)
    }
}

impl Enter3D {
    /// The scale and the opacity factor at `ui`'s time.
    pub fn look(&self, ui: &Ui<'_, '_, '_>) -> (f32, f32) {
        match self {
            Enter3D::Create(c) => {
                let p = enter_alpha(ui, c.delay, c.run_time, c.rate_func);
                (grow_scale(p) as f32, 1.0)
            }
            Enter3D::FadeIn(f) => (1.0, enter_alpha(ui, f.delay, f.run_time, f.rate_func)),
        }
    }
}

/// Adds `geometry` at Manim's point `at` with the look of `enter`: scaled about `at` and
/// faded. Nothing is added while it is invisible.
pub(crate) fn place(
    ui: &Ui<'_, '_, '_>,
    scene: &mut Scene,
    geometry: Geometry,
    material: Material,
    at: Point3,
    enter: Option<&Enter3D>,
) {
    let (scale, fade) = enter.map_or((1.0, 1.0), |e| e.look(ui));
    let opacity = material.opacity * fade;
    if scale <= 0.0 || opacity <= 0.0 {
        return;
    }
    let node = Node::mesh(geometry, material.opacity(opacity))
        .with_position(world(at))
        .scaled(scale);
    scene.add(node);
}

/// The point halfway from `a` to `b`.
pub(crate) fn midpoint(a: Point3, b: Point3) -> Point3 {
    (a.0.midpoint(b.0), a.1.midpoint(b.1), a.2.midpoint(b.2))
}

/// `color` a third of the way to black, the second checkerboard colour.
pub(crate) fn darker(color: Color) -> Color {
    use fframes::animation::Animatable;
    color.apply_progress(&Color::BLACK, 0.35)
}

/// A pixel from a projected point.
pub(crate) fn pixel(p: Vec3) -> Point {
    Point::new(f64::from(p.x), f64::from(p.y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manim_z_is_the_world_up() {
        assert_eq!(world((1.0, 2.0, 3.0)), vec3(1.0, 3.0, -2.0));
    }
}
