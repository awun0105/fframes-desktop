use std::f32::consts::{PI, TAU};

use fframes::{
    Color,
    three_d::{Geometry, Material, Quat, Scene, Vec3},
};

use crate::{
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Enter3D, Object3D, Point3, darker, midpoint, place, rgb, surface::parametric, world};

/// The paints of a surface: a checkerboard of `fill` and a darker `fill`, or `fill` alone.
fn paints(fill: Color, checkerboard: bool) -> (Color, Option<Color>) {
    (fill, checkerboard.then(|| darker(fill)))
}

/// `geometry` built along the world's `y` (Manim's `z`) turned to point along Manim's
/// `direction`.
fn along(geometry: Geometry, direction: Point3) -> Geometry {
    let d = world(direction).normalize_or(Vec3::Y);
    geometry.rotate(Quat::from_rotation_arc(Vec3::Y, d))
}

macro_rules! object_defaults {
    ($ty:ident) => {
        impl Default for $ty {
            fn default() -> Self {
                Self::themed(&Theme::default())
            }
        }
    };
}

/// Manim's `Sphere`: radius 1 at `center`, a checkerboard of 24 by 12 pieces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub center: Point3,
    pub radius: f32,
    pub resolution: (usize, usize),
    pub fill_color: Color,
    pub checkerboard: bool,
    pub opacity: f32,
    pub enter: Option<Enter3D>,
}

impl Themed for Sphere {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.0, 0.0, 0.0),
            radius: 1.0,
            resolution: (24, 12),
            fill_color: t.colors.accent,
            checkerboard: true,
            opacity: 1.0,
            enter: None,
        }
    }
}
object_defaults!(Sphere);

impl Object3D for Sphere {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let r = self.radius;
        let (geometry, material) = parametric(
            |u, v| (r * u.cos() * v.sin(), r * u.sin() * v.sin(), -r * v.cos()),
            (0.0, TAU),
            (0.0, PI),
            self.resolution,
            paints(self.fill_color, self.checkerboard),
        );
        let material = material.opacity(self.opacity);
        place(
            ui,
            scene,
            geometry,
            material,
            self.center,
            self.enter.as_ref(),
        );
    }
}

/// Manim's `Torus`: a ring of radius `major_radius` (3) around the `z` axis, `minor_radius`
/// (1) thick, a checkerboard of 24 by 24 pieces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Torus {
    pub center: Point3,
    pub major_radius: f32,
    pub minor_radius: f32,
    pub resolution: (usize, usize),
    pub fill_color: Color,
    pub checkerboard: bool,
    pub opacity: f32,
    pub enter: Option<Enter3D>,
}

impl Themed for Torus {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.0, 0.0, 0.0),
            major_radius: 3.0,
            minor_radius: 1.0,
            resolution: (24, 24),
            fill_color: t.colors.accent,
            checkerboard: true,
            opacity: 1.0,
            enter: None,
        }
    }
}
object_defaults!(Torus);

impl Object3D for Torus {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let (big, small) = (self.major_radius, self.minor_radius);
        let (geometry, material) = parametric(
            |u, v| {
                let ring = big - small * v.cos();
                (ring * u.cos(), ring * u.sin(), -small * v.sin())
            },
            (0.0, TAU),
            (0.0, TAU),
            self.resolution,
            paints(self.fill_color, self.checkerboard),
        );
        let material = material.opacity(self.opacity);
        place(
            ui,
            scene,
            geometry,
            material,
            self.center,
            self.enter.as_ref(),
        );
    }
}

/// Manim's `Cylinder`: radius 1 and height 2 around `center` along `direction` (the `z`
/// axis), a checkerboard of 24 by 24 pieces, closed at both ends with `show_ends`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cylinder {
    pub center: Point3,
    pub radius: f32,
    pub height: f32,
    pub direction: Point3,
    pub show_ends: bool,
    pub resolution: (usize, usize),
    pub fill_color: Color,
    pub checkerboard: bool,
    pub opacity: f32,
    pub enter: Option<Enter3D>,
}

impl Themed for Cylinder {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.0, 0.0, 0.0),
            radius: 1.0,
            height: 2.0,
            direction: (0.0, 0.0, 1.0),
            show_ends: true,
            resolution: (24, 24),
            fill_color: t.colors.accent,
            checkerboard: true,
            opacity: 1.0,
            enter: None,
        }
    }
}
object_defaults!(Cylinder);

/// A disk of radius `r` at height `z` on the `z` axis, in one colour.
fn disk(r: f32, z: f32, segments: usize, color: Color) -> Geometry {
    parametric(
        |s, v| (s * r * v.cos(), s * r * v.sin(), z),
        (0.0, 1.0),
        (0.0, TAU),
        (1, segments),
        (color, None),
    )
    .0
}

impl Object3D for Cylinder {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let (r, h) = (self.radius, self.height / 2.0);
        let (side, material) = parametric(
            |u, v| (r * v.cos(), r * v.sin(), u),
            (-h, h),
            (0.0, TAU),
            self.resolution,
            paints(self.fill_color, self.checkerboard),
        );
        let geometry = if self.show_ends {
            let n = self.resolution.1;
            Geometry::merge([
                side,
                disk(r, -h, n, self.fill_color),
                disk(r, h, n, self.fill_color),
            ])
        } else {
            side
        };
        let geometry = along(geometry, self.direction);
        let material = material.opacity(self.opacity);
        place(
            ui,
            scene,
            geometry,
            material,
            self.center,
            self.enter.as_ref(),
        );
    }
}

/// Manim's `Cone`: its tip at `apex`, its base `height` (1) behind it along `direction`
/// (the `z` axis) with radius `base_radius` (1), in one colour; `show_base` closes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    pub apex: Point3,
    pub base_radius: f32,
    pub height: f32,
    pub direction: Point3,
    pub show_base: bool,
    pub resolution: (usize, usize),
    pub fill_color: Color,
    pub checkerboard: bool,
    pub opacity: f32,
    pub enter: Option<Enter3D>,
}

impl Themed for Cone {
    fn themed(t: &Theme) -> Self {
        Self {
            apex: (0.0, 0.0, 0.0),
            base_radius: 1.0,
            height: 1.0,
            direction: (0.0, 0.0, 1.0),
            show_base: false,
            resolution: (24, 24),
            fill_color: t.colors.accent,
            checkerboard: false,
            opacity: 1.0,
            enter: None,
        }
    }
}
object_defaults!(Cone);

impl Object3D for Cone {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let (r, h) = (self.base_radius, self.height);
        let (side, material) = parametric(
            // around the middle of the axis, so that it grows from there
            |t, v| (t * r * v.cos(), t * r * v.sin(), h / 2.0 - t * h),
            (0.0, 1.0),
            (0.0, TAU),
            self.resolution,
            paints(self.fill_color, self.checkerboard),
        );
        let geometry = if self.show_base {
            Geometry::merge([side, disk(r, -h / 2.0, self.resolution.1, self.fill_color)])
        } else {
            side
        };
        let geometry = along(geometry, self.direction);
        let material = material.opacity(self.opacity);
        let (d, a) = (self.direction, self.apex);
        let n = (d.0 * d.0 + d.1 * d.1 + d.2 * d.2).sqrt().max(1e-6);
        let k = h / 2.0 / n;
        let middle = (a.0 - d.0 * k, a.1 - d.1 * k, a.2 - d.2 * k);
        place(ui, scene, geometry, material, middle, self.enter.as_ref());
    }
}

/// Manim's `Prism`: a box of `dimensions` (3, 2, 1) around `center`, filled at opacity 0.75.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prism {
    pub center: Point3,
    pub dimensions: Point3,
    pub fill_color: Color,
    pub opacity: f32,
    pub enter: Option<Enter3D>,
}

impl Themed for Prism {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.0, 0.0, 0.0),
            dimensions: (3.0, 2.0, 1.0),
            fill_color: t.colors.accent,
            opacity: 0.75,
            enter: None,
        }
    }
}
object_defaults!(Prism);

impl Object3D for Prism {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let (x, y, z) = self.dimensions;
        let material = Material::standard(rgb(self.fill_color))
            .opacity(self.opacity)
            .flat();
        place(
            ui,
            scene,
            Geometry::cuboid(x, z, y),
            material,
            self.center,
            self.enter.as_ref(),
        );
    }
}

/// Manim's `Cube`: a [`Prism`] with every side `side_length` (2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cube {
    pub center: Point3,
    pub side_length: f32,
    pub fill_color: Color,
    pub opacity: f32,
    pub enter: Option<Enter3D>,
}

impl Themed for Cube {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.0, 0.0, 0.0),
            side_length: 2.0,
            fill_color: t.colors.accent,
            opacity: 0.75,
            enter: None,
        }
    }
}
object_defaults!(Cube);

impl Object3D for Cube {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let s = self.side_length;
        Prism {
            center: self.center,
            dimensions: (s, s, s),
            fill_color: self.fill_color,
            opacity: self.opacity,
            enter: self.enter,
        }
        .add_to(ui, scene);
    }
}

/// Manim's `Dot3D`: a small smooth sphere, radius 0.08.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dot3D {
    pub point: Point3,
    pub radius: f32,
    pub color: Color,
    pub opacity: f32,
    pub enter: Option<Enter3D>,
}

impl Themed for Dot3D {
    fn themed(t: &Theme) -> Self {
        Self {
            point: (0.0, 0.0, 0.0),
            radius: 0.08,
            color: t.colors.title,
            opacity: 1.0,
            enter: None,
        }
    }
}
object_defaults!(Dot3D);

impl Object3D for Dot3D {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let material = Material::standard(rgb(self.color)).opacity(self.opacity);
        let geometry = Geometry::sphere(self.radius, 16, 8);
        place(
            ui,
            scene,
            geometry,
            material,
            self.point,
            self.enter.as_ref(),
        );
    }
}

/// A rod from `a` to `b` (Manim's points) of radius `thickness`, centered on the middle.
fn rod(a: Point3, b: Point3, thickness: f32) -> (Geometry, Point3, Vec3) {
    let (wa, wb) = (world(a), world(b));
    let length = wa.distance(wb);
    let mid = midpoint(a, b);
    let dir = (wb - wa).normalize_or(Vec3::Y);
    let geometry = Geometry::cylinder(thickness, thickness, length.max(1e-4), 12)
        .rotate(Quat::from_rotation_arc(Vec3::Y, dir));
    (geometry, mid, dir)
}

/// Manim's `Line3D`: a thin rod from `start` to `end`, radius `thickness` (0.02).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line3D {
    pub start: Point3,
    pub end: Point3,
    pub thickness: f32,
    pub color: Color,
    pub opacity: f32,
    pub enter: Option<Enter3D>,
}

impl Themed for Line3D {
    fn themed(t: &Theme) -> Self {
        Self {
            start: (-1.0, 0.0, 0.0),
            end: (1.0, 0.0, 0.0),
            thickness: 0.02,
            color: t.colors.title,
            opacity: 1.0,
            enter: None,
        }
    }
}
object_defaults!(Line3D);

impl Object3D for Line3D {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let (geometry, mid, _) = rod(self.start, self.end, self.thickness);
        let material = Material::standard(rgb(self.color)).opacity(self.opacity);
        place(ui, scene, geometry, material, mid, self.enter.as_ref());
    }
}

/// Manim's `Arrow3D`: a [`Line3D`] with a cone tip `height` (0.3) long and `base_radius`
/// (0.08) wide ending at `end`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrow3D {
    pub start: Point3,
    pub end: Point3,
    pub thickness: f32,
    pub height: f32,
    pub base_radius: f32,
    pub color: Color,
    pub opacity: f32,
    pub enter: Option<Enter3D>,
}

impl Themed for Arrow3D {
    fn themed(t: &Theme) -> Self {
        Self {
            start: (-1.0, 0.0, 0.0),
            end: (1.0, 0.0, 0.0),
            thickness: 0.02,
            height: 0.3,
            base_radius: 0.08,
            color: t.colors.title,
            opacity: 1.0,
            enter: None,
        }
    }
}
object_defaults!(Arrow3D);

impl Object3D for Arrow3D {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let (wa, wb) = (world(self.start), world(self.end));
        let length = wa.distance(wb);
        let dir = (wb - wa).normalize_or(Vec3::Y);
        let tip = self.height.min(length);
        let shaft = (length - tip).max(1e-4);
        let turn = Quat::from_rotation_arc(Vec3::Y, dir);
        // both pieces around the middle of the whole arrow, the node's origin
        let half = length / 2.0;
        let line = Geometry::cylinder(self.thickness, self.thickness, shaft, 12).translate(
            0.0,
            shaft / 2.0 - half,
            0.0,
        );
        let cone = Geometry::cone(self.base_radius, tip.max(1e-4), 16).translate(
            0.0,
            half - tip / 2.0,
            0.0,
        );
        let geometry = Geometry::merge([line, cone]).rotate(turn);
        let (a, b) = (self.start, self.end);
        let mid = midpoint(a, b);
        let material = Material::standard(rgb(self.color)).opacity(self.opacity);
        place(ui, scene, geometry, material, mid, self.enter.as_ref());
    }
}
