use std::collections::HashSet;

use fframes::{Color, Svgr, animation::Animatable};
use kurbo::{BezPath, Circle, Point, Shape as _, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    label::Label,
    layout::Anchor,
    motion::{cubic_in_out, progress},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{
    atom_color,
    molecule::{BondKind, Structure},
};

/// Turns the part of the molecule on the `to` side of bond `bond` (an index into
/// `structure.bonds`) by `degrees` about the bond, as if the bond were a shaft, from `start`
/// over `duration` seconds. The bond `(a, b)` turns the atoms reached from `b`; give the
/// bond the other way round in the structure to turn the other side. A bond in a ring turns
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BondRotation {
    pub bond: usize,
    pub degrees: f32,
    pub start: f32,
    pub duration: f32,
}

/// An atom or a bond of a [`GraphMolecule`], by its index in the structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoleculePart {
    Atom(usize),
    Bond(usize),
}

/// Changes the colour of `parts` to `color` from `start` over `duration` seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct Recolor {
    pub parts: Vec<MoleculePart>,
    pub color: Color,
    pub start: f32,
    pub duration: f32,
}

/// Puts a soft band of `color` behind `atoms` and the bonds between them, fading in from
/// `start` over `duration` seconds. [`functional_groups`] finds the atoms of a group.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupHighlight {
    pub atoms: Vec<usize>,
    pub color: Color,
    pub start: f32,
    pub duration: f32,
}

/// A functional group that [`functional_groups`] can find.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionalGroup {
    /// An O-H on a carbon that is not a carbonyl carbon.
    Hydroxyl,
    /// A C=O, in any group.
    Carbonyl,
    /// A C(=O)-OH.
    Carboxyl,
    /// A C(=O)-O-C: the carbon and both oxygens.
    Ester,
    /// A C(=O)-N: the carbon, the oxygen and the nitrogen.
    Amide,
    /// A nitrogen with single bonds only, not on a carbonyl carbon.
    Amine,
    /// An oxygen between two carbons, neither a carbonyl carbon.
    Ether,
}

/// The atoms of every `group` in `s`, each list starting with the group's central atom.
/// It reads the bonds and the hydrogens of the structure, so a structure from SMILES or a
/// molfile works; hydrogens drawn as atoms count as well.
pub fn functional_groups(s: &Structure, group: FunctionalGroup) -> Vec<Vec<usize>> {
    let symbol = |i: usize| s.atoms[i].symbol.as_str();
    let bonds_of = |i: usize| {
        s.bonds
            .iter()
            .filter(move |b| b.a == i || b.b == i)
            .map(move |b| (if b.a == i { b.b } else { b.a }, b.kind))
    };
    let hydrogens = |i: usize| {
        usize::from(s.atoms[i].hydrogens) + bonds_of(i).filter(|&(n, _)| symbol(n) == "H").count()
    };
    let carbonyl_oxygen =
        |c: usize| bonds_of(c).find(|&(n, k)| symbol(n) == "O" && k == BondKind::Double);
    let single_heavy = |i: usize| -> Vec<usize> {
        bonds_of(i)
            .filter(|&(n, _)| symbol(n) != "H")
            .map(|(n, _)| n)
            .collect()
    };
    let only_single =
        |i: usize| bonds_of(i).all(|(_, k)| !matches!(k, BondKind::Double | BondKind::Triple));
    let mut found = Vec::new();
    for i in 0..s.atoms.len() {
        match (group, symbol(i)) {
            (FunctionalGroup::Hydroxyl, "O") => {
                let heavy = single_heavy(i);
                if hydrogens(i) >= 1
                    && heavy.len() == 1
                    && symbol(heavy[0]) == "C"
                    && carbonyl_oxygen(heavy[0]).is_none()
                {
                    found.push(vec![i]);
                }
            }
            (FunctionalGroup::Carbonyl, "C") => {
                if let Some((o, _)) = carbonyl_oxygen(i) {
                    found.push(vec![i, o]);
                }
            }
            (FunctionalGroup::Carboxyl | FunctionalGroup::Ester | FunctionalGroup::Amide, "C") => {
                let Some((o, _)) = carbonyl_oxygen(i) else {
                    continue;
                };
                for (n, k) in bonds_of(i) {
                    let fits = k != BondKind::Double
                        && match group {
                            FunctionalGroup::Carboxyl => symbol(n) == "O" && hydrogens(n) >= 1,
                            FunctionalGroup::Ester => {
                                symbol(n) == "O"
                                    && single_heavy(n).iter().any(|&m| m != i && symbol(m) == "C")
                            }
                            _ => symbol(n) == "N",
                        };
                    if fits {
                        found.push(vec![i, o, n]);
                        break;
                    }
                }
            }
            (FunctionalGroup::Amine, "N") => {
                if only_single(i)
                    && single_heavy(i)
                        .iter()
                        .all(|&n| carbonyl_oxygen(n).is_none())
                {
                    found.push(vec![i]);
                }
            }
            (FunctionalGroup::Ether, "O") => {
                let heavy = single_heavy(i);
                if heavy.len() == 2
                    && hydrogens(i) == 0
                    && heavy
                        .iter()
                        .all(|&n| symbol(n) == "C" && carbonyl_oxygen(n).is_none())
                {
                    found.push(vec![i]);
                }
            }
            _ => {}
        }
    }
    found
}

/// The atoms reached from `from` without crossing the bond to `other`, `None` when `other`
/// is reached anyway (the bond is in a ring).
fn substituent(s: &Structure, from: usize, other: usize) -> Option<HashSet<usize>> {
    let mut seen = HashSet::from([from]);
    let mut stack = vec![from];
    while let Some(i) = stack.pop() {
        for n in s.neighbors(i) {
            if i == from && n == other {
                continue;
            }
            if n == other {
                return None;
            }
            if seen.insert(n) {
                stack.push(n);
            }
        }
    }
    Some(seen)
}

/// `p` turned by `angle` radians about the axis through `a` along unit `axis` (Rodrigues).
fn turn(p: [f64; 3], a: [f64; 3], axis: [f64; 3], angle: f64) -> [f64; 3] {
    let v = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let (sin, cos) = angle.sin_cos();
    let dot = v[0] * axis[0] + v[1] * axis[1] + v[2] * axis[2];
    let cross = [
        axis[1] * v[2] - axis[2] * v[1],
        axis[2] * v[0] - axis[0] * v[2],
        axis[0] * v[1] - axis[1] * v[0],
    ];
    std::array::from_fn(|k| a[k] + v[k] * cos + cross[k] * sin + axis[k] * dot * (1.0 - cos))
}

/// The positions of the atoms of `s` in bond lengths (`y` up, `z` towards the viewer) after
/// the parts of `rotations` that have happened by `t`.
pub(crate) fn posed(s: &Structure, rotations: &[BondRotation], t: f32) -> Vec<[f64; 3]> {
    let mut at: Vec<[f64; 3]> = s.atoms.iter().map(|a| [a.x, a.y, 0.0]).collect();
    for r in rotations {
        let Some(bond) = s.bonds.get(r.bond) else {
            continue;
        };
        let amount = cubic_in_out(progress(t, r.start, r.start + r.duration.max(1e-3)));
        if amount <= 0.0 {
            continue;
        }
        let Some(moving) = substituent(s, bond.b, bond.a) else {
            continue;
        };
        let (a, b) = (at[bond.a], at[bond.b]);
        let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if length < 1e-9 {
            continue;
        }
        let axis = d.map(|v| v / length);
        let angle = f64::from(r.degrees * amount).to_radians();
        // the far atom of the bond is on the axis
        for i in moving.into_iter().filter(|&i| i != bond.b) {
            at[i] = turn(at[i], a, axis, angle);
        }
    }
    at
}

/// Manim-chemistry's `GraphMolecule` with the animations of its `GMAnimationBuilder`: every
/// atom of `structure` as a ring with its symbol, carbons too, and the bonds as lines
/// (doubles and triples as two and three), around `center` (units of `ui.space`) with
/// `bond_length` pixels between bonded atoms.
///
/// - `rotations` turn the part of the molecule beyond a bond about it, in 3D seen from
///   `elevation` degrees above the page (0 is straight on, where atoms in line hide each
///   other): atoms nearer the viewer are drawn larger and on top.
/// - `recolors` change the colour of atoms and bonds.
/// - `highlights` put a band of colour behind a group of atoms ([`functional_groups`] finds
///   hydroxyls, carbonyls, carboxyls, esters, amides, amines and ethers).
///
/// All three run on the figure's own seconds. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct GraphMolecule<'a> {
    pub structure: Option<&'a Structure>,
    pub center: (f64, f64),
    pub bond_length: f32,
    pub atom_radius: f32,
    pub elevation: f32,
    pub rotations: Vec<BondRotation>,
    pub recolors: Vec<Recolor>,
    pub highlights: Vec<GroupHighlight>,
    pub color_atoms: bool,
    pub bond_color: Color,
    pub atom_fill: Color,
    pub stroke_width: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for GraphMolecule<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            structure: None,
            center: (0.0, 0.0),
            bond_length: 110.0,
            atom_radius: 24.0,
            elevation: 15.0,
            rotations: Vec::new(),
            recolors: Vec::new(),
            highlights: Vec::new(),
            color_atoms: true,
            bond_color: t.colors.text,
            atom_fill: t.colors.surface,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
        }
    }
}

impl Default for GraphMolecule<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

fn segment(a: Point, b: Point) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(a);
    path.line_to(b);
    path
}

impl<'a> Component<'a> for GraphMolecule<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(s) = self.structure else {
            return Svgr::empty();
        };
        let now = ui.seconds();
        let at = posed(s, &self.rotations, now);
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let length = f64::from(self.bond_length);
        let (sin, cos) = f64::from(self.elevation).to_radians().sin_cos();
        let px =
            |p: [f64; 3]| middle + Vec2::new(p[0] * length, -(p[1] * cos - p[2] * sin) * length);
        // towards the viewer; nearer atoms are a little larger, so a turn reads as depth
        let depth = |p: [f64; 3]| p[2] * cos + p[1] * sin;
        let scale = |p: [f64; 3]| (1.0 + 0.12 * depth(p)).clamp(0.6, 1.4);
        let radius = f64::from(self.atom_radius);
        let color_of = |part: MoleculePart, base: Color| {
            self.recolors
                .iter()
                .filter(|r| r.parts.contains(&part))
                .fold(base, |color, r| {
                    let amount = progress(now, r.start, r.start + r.duration.max(1e-3));
                    color.apply_progress(&r.color, cubic_in_out(amount))
                })
        };

        let mut shapes = Vec::new();
        for h in &self.highlights {
            let amount = progress(now, h.start, h.start + h.duration.max(1e-3));
            if amount <= 0.0 {
                continue;
            }
            let opacity = 0.4 * cubic_in_out(amount);
            let width = (radius * 3.0) as f32;
            for b in s
                .bonds
                .iter()
                .filter(|b| h.atoms.contains(&b.a) && h.atoms.contains(&b.b))
            {
                shapes.push(Shape {
                    path: segment(px(at[b.a]), px(at[b.b])),
                    fill: None,
                    stroke: Some(h.color),
                    stroke_width: width,
                    dash: None,
                    opacity,
                });
            }
            for &i in h.atoms.iter().filter(|&&i| i < at.len()) {
                shapes.push(Shape {
                    path: Circle::new(px(at[i]), radius * 1.7 * scale(at[i])).to_path(0.1),
                    fill: Some(h.color),
                    stroke: None,
                    stroke_width: 0.0,
                    dash: None,
                    opacity,
                });
            }
        }

        for (k, b) in s.bonds.iter().enumerate() {
            if b.a >= at.len() || b.b >= at.len() {
                continue;
            }
            let (pa, pb) = (px(at[b.a]), px(at[b.b]));
            let span = (pb - pa).hypot();
            let (ra, rb) = (radius * scale(at[b.a]), radius * scale(at[b.b]));
            if span <= ra + rb {
                continue;
            }
            let unit = (pb - pa) / span;
            let (a, e) = (pa + unit * ra, pb - unit * rb);
            let normal = Vec2::new(-unit.y, unit.x) * (length * 0.09);
            let offsets: &[f64] = match b.kind {
                BondKind::Double => &[-0.5, 0.5],
                BondKind::Triple => &[-1.0, 0.0, 1.0],
                _ => &[0.0],
            };
            let color = color_of(MoleculePart::Bond(k), self.bond_color);
            for &o in offsets {
                shapes.push(Shape {
                    path: segment(a + normal * o, e + normal * o),
                    fill: None,
                    stroke: Some(color),
                    stroke_width: self.stroke_width,
                    dash: None,
                    opacity: 1.0,
                });
            }
        }

        let mut order: Vec<usize> = (0..s.atoms.len()).collect();
        order.sort_by(|&i, &j| depth(at[i]).total_cmp(&depth(at[j])));
        let style = ui.theme.typography.body;
        let mut atoms = Vec::new();
        for i in order {
            let atom = &s.atoms[i];
            let base = if self.color_atoms {
                atom_color(&atom.symbol, self.bond_color)
            } else {
                self.bond_color
            };
            let color = color_of(MoleculePart::Atom(i), base);
            let r = radius * scale(at[i]);
            let ring = Shape {
                path: Circle::new(px(at[i]), r).to_path(0.1),
                fill: Some(self.atom_fill),
                stroke: Some(color),
                stroke_width: self.stroke_width,
                dash: None,
                opacity: 1.0,
            };
            let size = (r * 0.95).max(6.0) as usize;
            let style = TextStyle { size, ..style };
            let p = px(at[i]);
            let symbol = ui.show(Label {
                text: atom.symbol.clone().into(),
                x: p.x as f32,
                y: p.y as f32,
                direction: Anchor::Center,
                buff: 0.0,
                style,
                fill: color,
                ..ui.defaults()
            });
            atoms.push(ring.draw());
            atoms.push(symbol);
        }
        let back: Vec<Svgr<'a>> = shapes.iter().map(Shape::draw).collect();
        fade(
            ui,
            fframes::svgr!(<g>{back}{atoms}</g>),
            self.enter.as_ref(),
            self.exit.as_ref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn butanol() -> Structure {
        Structure::from_smiles("CCCCO").unwrap()
    }

    #[test]
    fn a_half_turn_flips_the_far_side_across_the_bond() {
        let s = butanol();
        // the bond between the second and third carbons, as laid out
        let k = s
            .bonds
            .iter()
            .position(|b| s.neighbors(b.a).count() == 2 && s.neighbors(b.b).count() == 2)
            .unwrap();
        let bond = s.bonds[k];
        let turn = [BondRotation {
            bond: k,
            degrees: 180.0,
            start: 0.0,
            duration: 1.0,
        }];
        let before = posed(&s, &turn, 0.0);
        let after = posed(&s, &turn, 1.0);
        let moving = substituent(&s, bond.b, bond.a).unwrap();
        // the bond itself stays, the rest keeps its distances
        for i in [bond.a, bond.b] {
            assert_eq!(before[i], after[i]);
        }
        for i in 0..s.atoms.len() {
            if moving.contains(&i) || i == bond.a {
                continue;
            }
            assert_eq!(before[i], after[i], "{i} moved");
        }
        let side = |p: [f64; 3]| {
            let (a, b) = (before[bond.a], before[bond.b]);
            (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        };
        for &i in moving.iter().filter(|&&i| i != bond.b) {
            assert!(side(before[i]) * side(after[i]) < 0.0, "{i} did not flip");
            assert!(after[i][2].abs() < 1e-9);
        }
        // half way the far side stands out of the page by its distance from the axis
        let mid = posed(&s, &turn, 0.5);
        assert!(moving.iter().any(|&i| mid[i][2].abs() > 0.4));
    }

    #[test]
    fn a_ring_bond_turns_nothing() {
        let s = Structure::from_smiles("c1ccccc1C").unwrap();
        let ring = s
            .bonds
            .iter()
            .position(|b| s.atoms[b.a].hydrogens == 1 && s.atoms[b.b].hydrogens == 1)
            .unwrap();
        let turn = [BondRotation {
            bond: ring,
            degrees: 90.0,
            start: 0.0,
            duration: 1.0,
        }];
        assert_eq!(posed(&s, &turn, 0.0), posed(&s, &turn, 1.0));
    }

    #[test]
    fn groups_of_known_molecules() {
        use FunctionalGroup::*;
        let count =
            |smiles: &str, g| functional_groups(&Structure::from_smiles(smiles).unwrap(), g).len();
        // ethanol
        assert_eq!(count("CCO", Hydroxyl), 1);
        assert_eq!(count("CCO", Carbonyl), 0);
        // acetic acid: a carboxyl, no hydroxyl of its own
        assert_eq!(count("CC(=O)O", Carboxyl), 1);
        assert_eq!(count("CC(=O)O", Hydroxyl), 0);
        // aspirin: an ester and a carboxyl, two carbonyls
        let aspirin = "CC(=O)Oc1ccccc1C(=O)O";
        assert_eq!(count(aspirin, Ester), 1);
        assert_eq!(count(aspirin, Carboxyl), 1);
        assert_eq!(count(aspirin, Carbonyl), 2);
        // paracetamol: an amide, a phenol hydroxyl, no amine
        let paracetamol = "CC(=O)Nc1ccc(O)cc1";
        assert_eq!(count(paracetamol, Amide), 1);
        assert_eq!(count(paracetamol, Hydroxyl), 1);
        assert_eq!(count(paracetamol, Amine), 0);
        assert_eq!(count("CCN", Amine), 1);
        assert_eq!(count("CCOCC", Ether), 1);
        let groups = functional_groups(&Structure::from_smiles("CCC(=O)O").unwrap(), Carboxyl);
        assert_eq!(groups[0].len(), 3);
    }
}
