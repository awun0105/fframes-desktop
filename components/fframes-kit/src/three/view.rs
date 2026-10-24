use std::f32::consts::FRAC_PI_2;

use fframes::{
    Svgr,
    three_d::{Camera, Renderer},
};
use kurbo::Point;

use crate::{
    explain::Space,
    kits::stage::stage,
    theme::Theme,
    ui::{Children, Component, Themed, Ui},
};

use super::{Object3D, Point3, pixel, world};

/// Where Manim's points land on the picture of a [`ThreeDView`].
#[derive(Debug, Clone, Copy)]
pub struct Projector {
    pub camera: Camera,
    /// The picture's size in pixels; it covers the canvas from its top left corner.
    pub width: f32,
    pub height: f32,
}

impl Projector {
    /// The pixel of Manim's point `p`, `None` behind the camera.
    pub fn project(&self, p: Point3) -> Option<Point> {
        self.camera
            .project(world(p), self.width, self.height)
            .map(pixel)
    }
}

/// Manim's `ThreeDScene`: 3D objects seen by a camera on a sphere around `frame_center`, in
/// one image over the canvas of `ui.space`, with 2D drawn on top.
///
/// The camera is Manim's: `phi` is the angle from the `z` axis, `theta` the angle around it
/// from the `x` axis (radians), so the default `phi` 0, `theta` -90° looks down on the `xy`
/// plane with `x` to the right. Its field of view shows the canvas's height in units at
/// `focal_distance` (times `zoom`), so while `distance` equals `focal_distance` the plane
/// through `frame_center` facing the camera lines up with `ui.space`, as in Manim, and
/// points in front of it grow by `f / (f - z)`. Manim's camera has no `distance`; here it
/// moves the camera in and out without changing the lens. In the renderer's terms the
/// camera is `Camera::orbit(world(frame_center), distance, theta + 90°, 90° - phi)`, with
/// the up direction that keeps `theta` turning the picture at `phi` 0.
///
/// `ambient_rotation` turns `theta` by that many radians per second, Manim's
/// `begin_ambient_camera_rotation(rate)` (Manim's default rate is 0.02).
/// `fixed_in_frame` is drawn over the picture in canvas pixels, Manim's
/// `add_fixed_in_frame_mobjects`; place things on 3D points with [`ThreeDView::projector`].
pub struct ThreeDView<'a> {
    pub phi: f32,
    pub theta: f32,
    pub distance: f32,
    pub focal_distance: f32,
    pub zoom: f32,
    pub frame_center: Point3,
    pub ambient_rotation: f32,
    pub objects: Vec<Box<dyn Object3D + 'a>>,
    pub fixed_in_frame: Option<Children<'a>>,
}

impl std::fmt::Debug for ThreeDView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreeDView")
            .field("phi", &self.phi)
            .field("theta", &self.theta)
            .field("distance", &self.distance)
            .field("focal_distance", &self.focal_distance)
            .field("zoom", &self.zoom)
            .field("frame_center", &self.frame_center)
            .field("ambient_rotation", &self.ambient_rotation)
            .field("objects", &self.objects.len())
            .finish_non_exhaustive()
    }
}

impl Themed for ThreeDView<'_> {
    /// Manim's camera: `phi` 0, `theta` -90°, focal distance 20; nothing in it yet.
    fn themed(_t: &Theme) -> Self {
        Self {
            phi: 0.0,
            theta: -FRAC_PI_2,
            distance: 20.0,
            focal_distance: 20.0,
            zoom: 1.0,
            frame_center: (0.0, 0.0, 0.0),
            ambient_rotation: 0.0,
            objects: Vec::new(),
            fixed_in_frame: None,
        }
    }
}

impl ThreeDView<'_> {
    /// The picture's size: the canvas of `space`, whose origin is its middle.
    fn picture_size(space: Space) -> (f32, f32) {
        (2.0 * space.origin.0, 2.0 * space.origin.1)
    }

    /// The camera `seconds` into the view.
    pub fn camera(&self, space: Space, seconds: f32) -> Camera {
        let (phi, theta) = (self.phi, self.theta + self.ambient_rotation * seconds);
        let (sp, cp) = phi.sin_cos();
        let (st, ct) = theta.sin_cos();
        let c = self.frame_center;
        let d = self.distance;
        let eye = (c.0 + d * sp * ct, c.1 + d * sp * st, c.2 + d * cp);
        let up = (-cp * ct, -cp * st, sp);
        let (_, height) = Self::picture_size(space);
        let units_tall = if space.unit > 0.0 {
            height / space.unit
        } else {
            8.0
        };
        let reach = (self.focal_distance * self.zoom).max(1e-3);
        let fov = 2.0 * (units_tall / 2.0 / reach).atan();
        let mut camera = Camera::perspective(fov.to_degrees())
            .position(world(eye))
            .look_at(world(c))
            .clip(0.05, (d + 100.0).max(500.0));
        camera.up = world(up);
        camera
    }

    /// The projector of the picture at `ui`'s time.
    pub fn projector(&self, ui: &Ui<'_, '_, '_>) -> Projector {
        let (width, height) = Self::picture_size(ui.space);
        Projector {
            camera: self.camera(ui.space, ui.seconds()),
            width,
            height,
        }
    }
}

impl<'a> Component<'a> for ThreeDView<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let projector = self.projector(ui);
        let mut scene = stage();
        for object in &self.objects {
            object.add_to(ui, &mut scene);
        }
        let (w, h) = (projector.width, projector.height);
        let image = Renderer::new(w.round().max(1.0) as u32, h.round().max(1.0) as u32)
            .render(&scene, &projector.camera);
        let overlays: Vec<Svgr<'a>> = self
            .objects
            .iter()
            .map(|o| o.overlay(ui, &projector))
            .collect();
        let fixed = self.fixed_in_frame.map_or_else(Svgr::empty, |c| c(ui));
        fframes::svgr!(
            <g>
                <image href={image.href()} x="0" y="0" width={w} height={h} />
                {overlays}
                {fixed}
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_camera_lines_up_with_the_space() {
        let space = Space::FULL_HD;
        let view = ThreeDView::themed(&Theme::default());
        let p = view.projector_at(space, 0.0);
        let at = p.project((1.0, 1.0, 0.0)).unwrap();
        assert!((at - Point::new(1095.0, 405.0)).hypot() < 0.05, "{at:?}");
        // in front of the plane a point is larger by f / (f - z)
        let near = p.project((1.0, 0.0, 10.0)).unwrap();
        assert!((near.x - (960.0 + 135.0 * 2.0)).abs() < 0.05, "{near:?}");
    }

    #[test]
    fn theta_turns_the_picture_even_from_above() {
        // theta 0: Manim's camera turns so that +y points right
        let space = Space::FULL_HD;
        let view = ThreeDView {
            theta: 0.0,
            ..ThreeDView::themed(&Theme::default())
        };
        let at = view
            .projector_at(space, 0.0)
            .project((0.0, 1.0, 0.0))
            .unwrap();
        assert!((at - Point::new(1095.0, 540.0)).hypot() < 0.05, "{at:?}");
        // and phi 90 degrees looks along the plane: z is up on screen
        let side = ThreeDView {
            phi: FRAC_PI_2,
            ..ThreeDView::themed(&Theme::default())
        };
        let up = side
            .projector_at(space, 0.0)
            .project((0.0, 0.0, 1.0))
            .unwrap();
        assert!((up - Point::new(960.0, 405.0)).hypot() < 0.05, "{up:?}");
    }

    #[test]
    fn ambient_rotation_turns_theta() {
        let space = Space::FULL_HD;
        let view = ThreeDView {
            ambient_rotation: FRAC_PI_2,
            ..ThreeDView::themed(&Theme::default())
        };
        let turned = view
            .projector_at(space, 1.0)
            .project((0.0, 1.0, 0.0))
            .unwrap();
        assert!(
            (turned - Point::new(1095.0, 540.0)).hypot() < 0.05,
            "{turned:?}"
        );
    }

    #[test]
    #[ignore = "measures the CPU cost of a frame; run with --ignored --nocapture"]
    fn cost_of_one_sphere_at_1080p() {
        use std::f32::consts::{PI, TAU};

        use fframes::three_d::Node;

        use crate::three::{darker, surface::parametric};

        let space = Space::FULL_HD;
        let view = ThreeDView {
            phi: 1.2,
            theta: 0.5,
            ..ThreeDView::themed(&Theme::default())
        };
        let camera = view.camera(space, 0.0);
        let accent = Theme::default().colors.accent;
        let frame = || {
            let mut scene = stage();
            let (g, m) = parametric(
                |u, v| (u.cos() * v.sin(), u.sin() * v.sin(), -v.cos()),
                (0.0, TAU),
                (0.0, PI),
                (24, 12),
                (accent, Some(darker(accent))),
            );
            scene.add(Node::mesh(g, m));
            Renderer::new(1920, 1080).render(&scene, &camera)
        };
        drop(frame());
        let runs = 10;
        let start = std::time::Instant::now();
        for _ in 0..runs {
            drop(frame());
        }
        let mean = start.elapsed().as_secs_f64() * 1000.0 / f64::from(runs);
        println!(
            "one sphere, 1920x1080, CPU: {mean:.1} ms per frame (scene and renderer built each frame)"
        );
    }
    impl ThreeDView<'_> {
        fn projector_at(&self, space: Space, seconds: f32) -> Projector {
            let (width, height) = Self::picture_size(space);
            Projector {
                camera: self.camera(space, seconds),
                width,
                height,
            }
        }
    }
}
