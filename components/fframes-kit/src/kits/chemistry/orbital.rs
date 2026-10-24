use std::f32::consts::FRAC_PI_2;

use fframes::{
    Color, Svgr,
    three_d::{Geometry, Material, Node, Quat, Vec2, Vec3},
};
use kurbo::Point;

use crate::{
    anim::{FadeIn, FadeOut, fade},
    layout::Anchor,
    motion::{cubic_in_out, progress},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::stage::{chem, fitting_camera, rgb, stage};

/// An atomic orbital: the s orbital, the three p orbitals and the five d orbitals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrbitalKind {
    #[default]
    S,
    Px,
    Py,
    Pz,
    Dxy,
    Dxz,
    Dyz,
    Dx2y2,
    Dz2,
}

impl OrbitalKind {
    /// The letter and its subscript (`("p", "z")`, `("d", "x²−y²")`).
    pub fn name(self) -> (&'static str, &'static str) {
        match self {
            Self::S => ("s", ""),
            Self::Px => ("p", "x"),
            Self::Py => ("p", "y"),
            Self::Pz => ("p", "z"),
            Self::Dxy => ("d", "xy"),
            Self::Dxz => ("d", "xz"),
            Self::Dyz => ("d", "yz"),
            Self::Dx2y2 => ("d", "x²−y²"),
            Self::Dz2 => ("d", "z²"),
        }
    }

    /// The lobes as unit directions in chemistry axes (z up) with the sign of the wave
    /// function in them. The s orbital has none, it is a sphere.
    pub fn lobes(self) -> Vec<([f32; 3], bool)> {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let axis = |d: [f32; 3]| vec![(d, true), ([-d[0], -d[1], -d[2]], false)];
        // the four lobes between two axes, positive where the product of the coordinates is
        let quadrants = |a: usize, b: usize| {
            [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
                .into_iter()
                .map(|(u, v): (f32, f32)| {
                    let mut d = [0.0; 3];
                    d[a] = u * h;
                    d[b] = v * h;
                    (d, u * v > 0.0)
                })
                .collect()
        };
        match self {
            Self::S => Vec::new(),
            Self::Px => axis([1.0, 0.0, 0.0]),
            Self::Py => axis([0.0, 1.0, 0.0]),
            Self::Pz => axis([0.0, 0.0, 1.0]),
            Self::Dxy => quadrants(0, 1),
            Self::Dxz => quadrants(0, 2),
            Self::Dyz => quadrants(1, 2),
            Self::Dx2y2 => vec![
                ([1.0, 0.0, 0.0], true),
                ([-1.0, 0.0, 0.0], true),
                ([0.0, 1.0, 0.0], false),
                ([0.0, -1.0, 0.0], false),
            ],
            Self::Dz2 => vec![([0.0, 0.0, 1.0], true), ([0.0, 0.0, -1.0], true)],
        }
    }
}

/// An atomic orbital in 3D, rendered into a `width` x `height` pixel image around `center`
/// (units of `ui.space`): lobes of `size` (scene units, the axes reach 1.3 times as far) in
/// `positive` and `negative` for the sign of the wave function, `d_z²` with its ring. The
/// lobes grow and fade in over `grow` seconds from `delay`, then the view turns `spin` degrees
/// a second seen from `elevation` degrees above. `axes` draws and labels x, y and z, and
/// `label` names the orbital under the image.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Orbital {
    pub kind: OrbitalKind,
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub size: f32,
    pub positive: Color,
    pub negative: Color,
    pub opacity: f32,
    pub delay: f32,
    pub grow: f32,
    pub spin: f32,
    pub azimuth: f32,
    pub elevation: f32,
    pub axes: bool,
    pub axis_color: Color,
    pub label: bool,
    pub label_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for Orbital {
    fn themed(t: &Theme) -> Self {
        Self {
            kind: OrbitalKind::default(),
            center: (0.0, 0.0),
            width: 420.0,
            height: 420.0,
            size: 1.0,
            positive: Color::hex("#4dabf7"),
            negative: Color::hex("#ff6b6b"),
            opacity: 0.85,
            delay: 0.0,
            grow: 1.0,
            spin: 20.0,
            azimuth: 30.0,
            elevation: 20.0,
            axes: true,
            axis_color: t.colors.muted,
            label: true,
            label_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Orbital {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// One lobe along +Y from the origin, the polar plot of `size · cos²θ`.
fn lobe(size: f32) -> Geometry {
    let points: Vec<Vec2> = (0..=24)
        .map(|i| {
            // from the side (θ = 90°) to the tip (θ = 0) so y rises
            let theta = FRAC_PI_2 * (1.0 - i as f32 / 24.0);
            let rho = size * theta.cos().powi(2);
            Vec2::new(rho * theta.sin(), rho * theta.cos())
        })
        .collect();
    Geometry::lathe(&points, 40)
}

impl<'a> Component<'a> for Orbital {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let now = ui.seconds();
        let grown = cubic_in_out(progress(now, self.delay, self.delay + self.grow.max(1e-3)));
        let mut scene = stage();
        let material = |positive: bool| {
            let color = if positive {
                self.positive
            } else {
                self.negative
            };
            Material::standard(rgb(color))
                .roughness(0.45)
                .opacity(self.opacity * grown)
        };
        if grown > 0.0 {
            let scale = Vec3::splat(0.4 + 0.6 * grown);
            let lobes = self.kind.lobes();
            if lobes.is_empty() {
                let mut sphere =
                    Node::mesh(Geometry::sphere(0.6 * self.size, 48, 24), material(true));
                sphere.scale = scale;
                scene.add(sphere);
            }
            let geometry = lobe(self.size);
            for (d, positive) in lobes {
                let mut node = Node::mesh(geometry.clone(), material(positive));
                node.rotation = Quat::from_rotation_arc(Vec3::Y, chem(d[0], d[1], d[2]));
                node.scale = scale;
                scene.add(node);
            }
            if self.kind == OrbitalKind::Dz2 {
                // the ring of opposite sign around the waist, in the xy plane
                let ring =
                    Geometry::torus(0.42 * self.size, 0.13 * self.size, 20, 64).rotate_x(FRAC_PI_2);
                let mut node = Node::mesh(ring, material(false));
                node.scale = scale;
                scene.add(node);
            }
        }
        let reach = 1.3 * self.size;
        if self.axes {
            let stick = Geometry::cylinder(0.012, 0.012, 2.0 * reach, 8);
            for d in [Vec3::X, Vec3::Y, Vec3::Z] {
                let mut axis = Node::mesh(
                    stick.clone(),
                    Material::standard(rgb(self.axis_color)).roughness(0.8),
                );
                axis.rotation = Quat::from_rotation_arc(Vec3::Y, chem(d.x, d.y, d.z));
                scene.add(axis);
            }
        }
        let azimuth = self.azimuth + self.spin * (now - self.delay).max(0.0);
        let camera = fitting_camera(
            Vec3::ZERO,
            reach + 0.15,
            self.width,
            self.height,
            azimuth,
            self.elevation,
        );
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (w, h) = (f64::from(self.width), f64::from(self.height));
        let corner = Point::new(middle.x - w / 2.0, middle.y - h / 2.0);
        let image = super::stage::picture(&scene, &camera, middle, self.width, self.height);
        let mut parts = vec![image];
        if self.axes {
            for (name, d) in [("x", Vec3::X), ("y", Vec3::Y), ("z", Vec3::Z)] {
                let tip = chem(d.x, d.y, d.z) * (reach + 0.1);
                if let Some(p) = camera.project(tip, self.width, self.height) {
                    let at = Point::new(corner.x + f64::from(p.x), corner.y + f64::from(p.y));
                    parts.push(super::super::label(
                        ui,
                        name.to_owned(),
                        at,
                        Anchor::Center,
                        self.axis_color,
                    ));
                }
            }
        }
        if self.label {
            let (letter, sub) = self.kind.name();
            let runs = [(letter.to_owned(), false), (sub.to_owned(), true)];
            let top = corner.y + h + f64::from(ui.theme.spacing.sm);
            parts.push(super::subscript_text(
                ui,
                &runs,
                middle.x,
                top,
                self.label_color,
            ));
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lobes_carry_the_sign_of_the_wave_function() {
        assert_eq!(OrbitalKind::S.lobes().len(), 0);
        assert_eq!(OrbitalKind::Pz.lobes().len(), 2);
        for kind in [
            OrbitalKind::Dxy,
            OrbitalKind::Dxz,
            OrbitalKind::Dyz,
            OrbitalKind::Dx2y2,
        ] {
            let lobes = kind.lobes();
            assert_eq!(lobes.len(), 4);
            // opposite lobes of a d orbital have the same sign, two of each
            assert_eq!(lobes.iter().filter(|(_, p)| *p).count(), 2);
        }
        // d_xy is positive where x and y agree in sign
        for (d, positive) in OrbitalKind::Dxy.lobes() {
            assert_eq!(positive, d[0] * d[1] > 0.0);
            assert!((d[0].hypot(d[1]) - 1.0).abs() < 1e-6);
        }
        assert!(OrbitalKind::Dz2.lobes().iter().all(|(_, p)| *p));
    }

    #[test]
    fn a_lobe_reaches_its_size_along_its_axis() {
        let top = lobe(1.5).bounds().max.y;
        assert!((top - 1.5).abs() < 1e-4);
    }
}
