//! What the 3D figures share: the lit scene, the mapping from chemistry axes and the image
//! in the frame.

use fframes::{
    Color, Svgr,
    three_d::{Background, Camera, Environment, Light, Renderer, Rgb, Scene, Vec3, vec3},
};
use kurbo::Point;

/// A kit colour for the 3D renderer.
pub(super) fn rgb(color: Color) -> Rgb {
    Rgb::srgb8(color.r, color.g, color.b)
}

/// A point in chemistry axes (z up, x towards the viewer, y right) in the renderer's axes
/// (y up, z towards the viewer). The mapping is a rotation, so handedness is kept.
pub(super) fn chem(x: f32, y: f32, z: f32) -> Vec3 {
    vec3(x, z, -y)
}

/// An empty scene with a studio environment, a sun from the upper left and no background,
/// so the frame shows through.
pub(super) fn stage() -> Scene {
    let mut scene = Scene::new()
        .environment(Environment::studio())
        .background(Background::Transparent);
    scene.add(fframes::three_d::Node::light(Light::sun(
        Rgb::WHITE,
        2.5,
        vec3(-3.0, 5.0, 4.0),
    )));
    scene
}

/// A camera turned `azimuth` degrees around the Y axis and raised `elevation` degrees, far
/// enough to fit a sphere of `radius` around `target` into a `width` x `height` image.
pub(super) fn fitting_camera(
    target: Vec3,
    radius: f32,
    width: f32,
    height: f32,
    azimuth: f32,
    elevation: f32,
) -> Camera {
    const FOV: f32 = 30.0;
    let vertical = FOV.to_radians();
    let horizontal = 2.0 * ((vertical / 2.0).tan() * width / height.max(1.0)).atan();
    let half = vertical.min(horizontal) / 2.0;
    let distance = radius / half.sin().max(1e-3) * 1.05;
    Camera::perspective(FOV).orbit(target, distance, azimuth, elevation)
}

/// Renders `scene` and places the image `width` x `height` pixels around `middle`.
pub(super) fn picture<'a>(
    scene: &Scene,
    camera: &Camera,
    middle: Point,
    width: f32,
    height: f32,
) -> Svgr<'a> {
    let image = Renderer::new(
        width.round().max(1.0) as u32,
        height.round().max(1.0) as u32,
    )
    .render(scene, camera);
    let (x, y) = (
        middle.x as f32 - width / 2.0,
        middle.y as f32 - height / 2.0,
    );
    fframes::svgr!(<image href={image.href()} x={x} y={y} width={width} height={height} />)
}
