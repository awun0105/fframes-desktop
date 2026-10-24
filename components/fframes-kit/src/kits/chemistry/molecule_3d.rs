use std::{error::Error, fmt};

use fframes::{
    Color, Svgr,
    three_d::{Geometry, Material, Node, Quat, Rgb, Vec3},
};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    motion::progress,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::stage::{chem, fitting_camera, picture, rgb, stage};

/// An atom of a [`Structure3D`]: its element symbol and its position in ångström.
#[derive(Debug, Clone, PartialEq)]
pub struct Atom3D {
    pub element: String,
    pub position: [f32; 3],
}

/// Atoms in 3D and the bonds between them (`(from, to, order)`, order 1 to 3).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Structure3D {
    pub atoms: Vec<Atom3D>,
    pub bonds: Vec<(usize, usize, u8)>,
}

/// Why an XYZ file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XyzError {
    /// The first line is not the number of atoms.
    Count,
    /// Fewer atom lines than the count.
    Missing { expected: usize, found: usize },
    /// An atom line (1-based in the file) that is not a symbol and three numbers.
    Line(usize),
}

impl fmt::Display for XyzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Count => write!(f, "the first line of an XYZ file is the number of atoms"),
            Self::Missing { expected, found } => {
                write!(f, "the XYZ file lists {found} of its {expected} atoms")
            }
            Self::Line(line) => write!(f, "line {line} is not a symbol and three coordinates"),
        }
    }
}

impl Error for XyzError {}

/// The covalent radius in ångström, for finding bonds.
fn covalent_radius(element: &str) -> f32 {
    match element {
        "H" => 0.31,
        "B" => 0.84,
        "C" => 0.76,
        "N" => 0.71,
        "O" => 0.66,
        "F" => 0.57,
        "Si" => 1.11,
        "P" => 1.07,
        "S" => 1.05,
        "Cl" => 1.02,
        "Br" => 1.20,
        "I" => 1.39,
        _ => 0.8,
    }
}

/// The CPK colour of an element, as in Jmol.
fn cpk(element: &str) -> &'static str {
    match element {
        "H" => "#f2f2f2",
        "C" => "#909090",
        "N" => "#3050f8",
        "O" => "#ff0d0d",
        "F" => "#90e050",
        "P" => "#ff8000",
        "S" => "#ffff30",
        "Cl" => "#1ff01f",
        "Br" => "#a62929",
        "I" => "#940094",
        _ => "#ff1493",
    }
}

impl Structure3D {
    pub fn new(atoms: Vec<Atom3D>, bonds: Vec<(usize, usize, u8)>) -> Self {
        Self { atoms, bonds }
    }

    /// Reads an XYZ file (the number of atoms, a comment line, then one `symbol x y z` line
    /// per atom in ångström) and finds single bonds between atoms closer than 1.2 times the
    /// sum of their covalent radii. Set `bonds` yourself for double and triple bonds.
    pub fn from_xyz(text: &str) -> Result<Self, XyzError> {
        let mut lines = text.lines();
        let count: usize = lines
            .next()
            .and_then(|l| l.trim().parse().ok())
            .ok_or(XyzError::Count)?;
        let atoms = lines
            .skip(1)
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
            .take(count)
            .map(|(i, line)| {
                let mut parts = line.split_whitespace();
                let element = parts.next().ok_or(XyzError::Line(i + 3))?;
                let mut xyz = [0.0_f32; 3];
                for v in &mut xyz {
                    *v = parts
                        .next()
                        .and_then(|p| p.parse().ok())
                        .ok_or(XyzError::Line(i + 3))?;
                }
                Ok(Atom3D {
                    element: element.to_owned(),
                    position: xyz,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if atoms.len() < count {
            return Err(XyzError::Missing {
                expected: count,
                found: atoms.len(),
            });
        }
        let mut structure = Self::new(atoms, Vec::new());
        structure.bonds = structure.inferred_bonds();
        Ok(structure)
    }

    /// Single bonds between atoms closer than 1.2 times the sum of their covalent radii.
    pub fn inferred_bonds(&self) -> Vec<(usize, usize, u8)> {
        let mut bonds = Vec::new();
        for (i, a) in self.atoms.iter().enumerate() {
            for (j, b) in self.atoms.iter().enumerate().skip(i + 1) {
                let d = Vec3::from(a.position).distance(Vec3::from(b.position));
                let limit = 1.2 * (covalent_radius(&a.element) + covalent_radius(&b.element));
                if d > 0.1 && d < limit {
                    bonds.push((i, j, 1));
                }
            }
        }
        bonds
    }

    /// The middle of the atoms' bounding box, in ångström.
    pub fn center(&self) -> [f32; 3] {
        let mut low = Vec3::splat(f32::INFINITY);
        let mut high = Vec3::splat(f32::NEG_INFINITY);
        for a in &self.atoms {
            low = low.min(Vec3::from(a.position));
            high = high.max(Vec3::from(a.position));
        }
        if self.atoms.is_empty() {
            [0.0; 3]
        } else {
            ((low + high) / 2.0).into()
        }
    }

    /// The largest distance of an atom from [`Structure3D::center`].
    pub fn radius(&self) -> f32 {
        let c = Vec3::from(self.center());
        self.atoms
            .iter()
            .map(|a| Vec3::from(a.position).distance(c))
            .fold(0.0, f32::max)
    }
}

/// A [`Structure3D`] as balls and sticks in CPK colours, rendered in 3D into a `width` x
/// `height` pixel image around `center` (units of `ui.space`). The molecule turns `spin`
/// degrees a second around the vertical axis, seen from `elevation` degrees above. The bonds
/// listed in `highlight` (indices into `bonds`) turn `highlight_color` and glow over half a
/// second from `highlight_at`. The image fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Molecule3D<'a> {
    pub structure: Option<&'a Structure3D>,
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub spin: f32,
    pub azimuth: f32,
    pub elevation: f32,
    pub highlight: Vec<usize>,
    pub highlight_at: f32,
    pub bond_color: Color,
    pub highlight_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for Molecule3D<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            structure: None,
            center: (0.0, 0.0),
            width: 900.0,
            height: 640.0,
            spin: 30.0,
            azimuth: 0.0,
            elevation: 20.0,
            highlight: Vec::new(),
            highlight_at: 0.0,
            bond_color: Color::hex("#bfbfbf"),
            highlight_color: t.colors.highlight,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Molecule3D<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The ball of an atom: a third of its covalent radius over a base size.
fn ball_radius(element: &str) -> f32 {
    0.2 + 0.28 * covalent_radius(element)
}

impl<'a> Component<'a> for Molecule3D<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(structure) = self.structure else {
            return Svgr::empty();
        };
        let now = ui.seconds();
        let at = |p: [f32; 3]| chem(p[0], p[1], p[2]);
        let mut scene = stage();
        for atom in &structure.atoms {
            let p = at(atom.position);
            let ball = Geometry::sphere(ball_radius(&atom.element), 32, 16);
            let material = Material::standard(cpk(&atom.element))
                .roughness(0.35)
                .clearcoat(0.3);
            scene.add(Node::mesh(ball, material).at(p.x, p.y, p.z));
        }
        let glow = progress(now, self.highlight_at, self.highlight_at + 0.5);
        for (index, &(i, j, order)) in structure.bonds.iter().enumerate() {
            let (Some(a), Some(b)) = (structure.atoms.get(i), structure.atoms.get(j)) else {
                continue;
            };
            let (a, b) = (at(a.position), at(b.position));
            let length = a.distance(b);
            if length <= 1e-4 {
                continue;
            }
            let direction = (b - a) / length;
            let lit = if self.highlight.contains(&index) {
                glow
            } else {
                0.0
            };
            let color: Rgb = rgb(self.bond_color).lerp(rgb(self.highlight_color), lit);
            let material = Material::standard(color)
                .roughness(0.4)
                .emissive(rgb(self.highlight_color), 1.5 * lit);
            // more lines side by side for a double or triple bond
            let order = order.clamp(1, 3);
            let across = direction
                .cross(if direction.y.abs() > 0.9 {
                    Vec3::X
                } else {
                    Vec3::Y
                })
                .normalize();
            let radius = if order == 1 { 0.1 } else { 0.065 };
            for k in 0..order {
                let offset = (f32::from(k) - f32::from(order - 1) / 2.0) * 0.17;
                let middle = (a + b) / 2.0 + across * offset;
                let mut stick = Node::mesh(
                    Geometry::cylinder(radius, radius, length, 16),
                    material.clone(),
                )
                .at(middle.x, middle.y, middle.z);
                stick.rotation = Quat::from_rotation_arc(Vec3::Y, direction);
                scene.add(stick);
            }
        }
        let c = structure.center();
        let reach = structure.radius() + 0.6;
        let azimuth = self.azimuth + self.spin * now;
        let camera = fitting_camera(
            at(c),
            reach,
            self.width,
            self.height,
            azimuth,
            self.elevation,
        );
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let image = picture(&scene, &camera, middle, self.width, self.height);
        fade(ui, image, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATER: &str =
        "3\nwater\nO 0.000 0.000 0.117\nH 0.000 0.757 -0.469\nH 0.000 -0.757 -0.469\n";

    #[test]
    fn reads_xyz_and_finds_bonds() {
        let water = Structure3D::from_xyz(WATER).unwrap();
        assert_eq!(water.atoms.len(), 3);
        assert_eq!(water.atoms[1].element, "H");
        // two O-H bonds, no H-H bond
        assert_eq!(water.bonds, vec![(0, 1, 1), (0, 2, 1)]);
        assert!((water.center()[1]).abs() < 1e-6);
        assert!(water.radius() > 0.7 && water.radius() < 0.9);
    }

    #[test]
    fn rejects_broken_xyz() {
        assert_eq!(Structure3D::from_xyz("x\n"), Err(XyzError::Count));
        assert_eq!(
            Structure3D::from_xyz("2\n\nO 0 0 0\n"),
            Err(XyzError::Missing {
                expected: 2,
                found: 1
            })
        );
        assert_eq!(
            Structure3D::from_xyz("1\n\nO 0 zero 0\n"),
            Err(XyzError::Line(3))
        );
    }

    #[test]
    fn chemistry_axes_map_to_a_rotation() {
        // z up becomes y up; the basis stays right-handed
        assert_eq!(chem(0.0, 0.0, 1.0), Vec3::Y);
        let (x, y, z) = (
            chem(1.0, 0.0, 0.0),
            chem(0.0, 1.0, 0.0),
            chem(0.0, 0.0, 1.0),
        );
        assert!((x.cross(y) - z).length() < 1e-6);
    }
}
