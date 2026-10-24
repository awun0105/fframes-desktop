use std::{
    borrow::Cow,
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex, OnceLock},
};

use chematic::{
    core::{apply_kekule, implicit_hcount, kekulize},
    depict::{DepictBondKind, compute_depict_data},
};
use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    label::{Label, text_size},
    layout::Anchor,
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::atom_color;

/// How two atoms are bonded, as drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BondKind {
    Single,
    Double,
    Triple,
    /// A solid wedge, wide at the second atom: the bond comes out of the page.
    Wedge,
    /// A hashed wedge, wide at the second atom: the bond goes behind the page.
    Dash,
}

/// An atom of a [`Structure`]: its symbol, its position in bond lengths (`y` up), its
/// hydrogens and charge. `label` is `false` for the carbons a skeletal formula leaves out.
#[derive(Debug, Clone, PartialEq)]
pub struct ChemAtom {
    pub symbol: String,
    pub x: f64,
    pub y: f64,
    pub hydrogens: u8,
    pub charge: i8,
    pub label: bool,
}

impl ChemAtom {
    /// An atom without hydrogens or charge, labelled unless it is a carbon.
    pub fn new(symbol: &str, x: f64, y: f64) -> Self {
        Self {
            symbol: symbol.to_owned(),
            x,
            y,
            hydrogens: 0,
            charge: 0,
            label: symbol != "C",
        }
    }

    pub fn hydrogens(self, hydrogens: u8) -> Self {
        Self { hydrogens, ..self }
    }

    pub fn charge(self, charge: i8) -> Self {
        Self { charge, ..self }
    }
}

/// A bond of a [`Structure`] between the atoms at indexes `a` and `b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChemBond {
    pub a: usize,
    pub b: usize,
    pub kind: BondKind,
}

impl ChemBond {
    pub fn new(a: usize, b: usize, kind: BondKind) -> Self {
        Self { a, b, kind }
    }
}

/// A SMILES string that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmilesError(pub String);

impl fmt::Display for SmilesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid SMILES: {}", self.0)
    }
}

impl std::error::Error for SmilesError {}

/// The atoms and bonds of a skeletal formula, with 2D positions in bond lengths. Build one
/// by hand or with [`Structure::from_smiles`], which lays the molecule out.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Structure {
    pub atoms: Vec<ChemAtom>,
    pub bonds: Vec<ChemBond>,
}

/// `mol` with its aromatic rings as alternating single and double bonds, or as it is when
/// they cannot be.
pub(crate) fn kekulized(mol: chematic::core::Molecule) -> chematic::core::Molecule {
    match kekulize(&mol) {
        Ok(kekule) => apply_kekule(&mol, &kekule),
        Err(_) => mol,
    }
}

/// The layout units of `chematic` per bond length.
const DEPICT_BOND: f64 = 40.0;

impl Structure {
    pub fn new(atoms: Vec<ChemAtom>, bonds: Vec<ChemBond>) -> Self {
        Self { atoms, bonds }
    }

    /// Reads `smiles` and lays it out in 2D with `chematic`, aromatic rings as alternating
    /// single and double bonds, centred on the origin. Hydrogens on carbons are left out;
    /// those on other atoms go into their labels.
    pub fn from_smiles(smiles: &str) -> Result<Self, SmilesError> {
        let parsed =
            chematic::smiles::parse(smiles).map_err(|e| SmilesError(format!("{smiles}: {e}")))?;
        Ok(Self::laid_out(&kekulized(parsed)))
    }

    /// Lays out a `chematic` molecule in 2D, centred on the origin.
    pub(crate) fn laid_out(mol: &chematic::core::Molecule) -> Self {
        let data = compute_depict_data(mol);
        let index: HashMap<u32, usize> = data
            .atoms
            .iter()
            .enumerate()
            .map(|(i, a)| (a.idx.0, i))
            .collect();
        let atoms: Vec<ChemAtom> = data
            .atoms
            .iter()
            .map(|a| ChemAtom {
                symbol: a.element.symbol().to_owned(),
                x: a.pos.x / DEPICT_BOND,
                y: -a.pos.y / DEPICT_BOND,
                hydrogens: implicit_hcount(mol, a.idx),
                charge: a.charge,
                label: a.label.is_some(),
            })
            .collect();
        let bonds = data
            .bonds
            .iter()
            .filter_map(|b| {
                let kind = match b.kind {
                    DepictBondKind::Double => BondKind::Double,
                    DepictBondKind::Triple => BondKind::Triple,
                    DepictBondKind::Up => BondKind::Wedge,
                    DepictBondKind::Down => BondKind::Dash,
                    DepictBondKind::Single | DepictBondKind::Aromatic => BondKind::Single,
                };
                Some(ChemBond::new(
                    *index.get(&b.atom1.0)?,
                    *index.get(&b.atom2.0)?,
                    kind,
                ))
            })
            .collect();
        let mut structure = Self { atoms, bonds };
        structure.center();
        structure
    }

    /// The smallest box around the atoms: `(min_x, min_y, max_x, max_y)` in bond lengths.
    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        self.atoms.iter().fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(x0, y0, x1, y1), a| (x0.min(a.x), y0.min(a.y), x1.max(a.x), y1.max(a.y)),
        )
    }

    /// Moves the atoms so the middle of their box is at the origin.
    pub fn center(&mut self) {
        if self.atoms.is_empty() {
            return;
        }
        let (x0, y0, x1, y1) = self.bounds();
        let (cx, cy) = (f64::midpoint(x0, x1), f64::midpoint(y0, y1));
        for a in &mut self.atoms {
            a.x -= cx;
            a.y -= cy;
        }
    }

    /// The other atoms bonded to atom `i`.
    fn neighbors(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        self.bonds.iter().filter_map(move |b| {
            if b.a == i {
                Some(b.b)
            } else if b.b == i {
                Some(b.a)
            } else {
                None
            }
        })
    }

    /// The side of bond `bond` its neighbours lie on: positive to the left of `a` to `b` (in
    /// bond lengths, `y` up), 0 when they balance.
    pub(crate) fn side(&self, bond: &ChemBond) -> f64 {
        let (a, b) = (&self.atoms[bond.a], &self.atoms[bond.b]);
        let along = Vec2::new(b.x - a.x, b.y - a.y);
        let normal = Vec2::new(-along.y, along.x);
        let mid = Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y));
        let mut sum = 0.0;
        for (end, other) in [(bond.a, bond.b), (bond.b, bond.a)] {
            for n in self.neighbors(end).filter(|&n| n != other) {
                let p = &self.atoms[n];
                sum += (Point::new(p.x, p.y) - mid).dot(normal).signum();
            }
        }
        sum
    }
}

/// A skeletal formula: the bonds of `structure` `bond_length` pixels long around `center`
/// (units of `ui.space`), double bonds in rings drawn inside the ring, and the labels of
/// the atoms that are not carbons, with their hydrogens and charges (`show_hydrogens`).
/// The bonds draw on in order with `enter` (`Create` draws them one after the other), the
/// labels with `parts_enter`. With `color_atoms` oxygen, nitrogen, sulfur, phosphorus and
/// the halogens get their own colours. Without `structure` nothing is drawn.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Molecule2D<'a> {
    pub structure: Option<&'a Structure>,
    pub center: (f64, f64),
    pub bond_length: f32,
    pub show_hydrogens: bool,
    pub color_atoms: bool,
    pub bond_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Molecule2D<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            structure: None,
            center: (0.0, 0.0),
            bond_length: 70.0,
            show_hydrogens: true,
            color_atoms: true,
            bond_color: t.colors.text,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Molecule2D<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Molecule2D<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        match self.structure {
            Some(structure) => draw(ui, structure, &self),
            None => Svgr::empty(),
        }
    }
}

/// A skeletal formula from a SMILES string (`"CC(=O)O"`), laid out by `chematic` and drawn
/// like [`Molecule2D`]. The layout is computed once per string and kept for the rest of the
/// process; an invalid string draws nothing (check it with [`Structure::from_smiles`]).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Molecule<'a> {
    pub smiles: Cow<'a, str>,
    pub center: (f64, f64),
    pub bond_length: f32,
    pub show_hydrogens: bool,
    pub color_atoms: bool,
    pub bond_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Molecule<'_> {
    fn themed(t: &Theme) -> Self {
        let m = Molecule2D::themed(t);
        Self {
            smiles: Cow::Borrowed("c1ccccc1"),
            center: m.center,
            bond_length: m.bond_length,
            show_hydrogens: m.show_hydrogens,
            color_atoms: m.color_atoms,
            bond_color: m.bond_color,
            stroke_width: m.stroke_width,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Molecule<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The layout of `smiles`, read once.
fn laid_out(smiles: &str) -> Option<Arc<Structure>> {
    static LAYOUTS: OnceLock<Mutex<HashMap<String, Option<Arc<Structure>>>>> = OnceLock::new();
    let layouts = LAYOUTS.get_or_init(Mutex::default);
    let mut layouts = layouts
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    layouts
        .entry(smiles.to_owned())
        .or_insert_with(|| Structure::from_smiles(smiles).ok().map(Arc::new))
        .clone()
}

impl<'a> Component<'a> for Molecule<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(structure) = laid_out(&self.smiles) else {
            return Svgr::empty();
        };
        let look = Molecule2D {
            structure: None,
            center: self.center,
            bond_length: self.bond_length,
            show_hydrogens: self.show_hydrogens,
            color_atoms: self.color_atoms,
            bond_color: self.bond_color,
            stroke_width: self.stroke_width,
            enter: self.enter,
            exit: self.exit,
            parts_enter: self.parts_enter,
            parts_exit: self.parts_exit,
        };
        draw(ui, &structure, &look)
    }
}

fn segment(a: Point, b: Point) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(a);
    path.line_to(b);
    path
}

/// The bonds of `s` in pixels, each as one or more paths (lines, or a filled wedge).
fn bond_paths(
    s: &Structure,
    bond: &ChemBond,
    px: impl Fn(usize) -> Point,
    length: f64,
) -> Vec<(BezPath, bool)> {
    let (pa, pb) = (px(bond.a), px(bond.b));
    let along = pb - pa;
    let span = along.hypot();
    if span < 1e-9 {
        return Vec::new();
    }
    let unit = along / span;
    // labelled atoms keep a gap around their text
    let gap = length * 0.3;
    let trim_a = if s.atoms[bond.a].label { gap } else { 0.0 };
    let trim_b = if s.atoms[bond.b].label { gap } else { 0.0 };
    if trim_a + trim_b >= span {
        return Vec::new();
    }
    let (a, b) = (pa + unit * trim_a, pb - unit * trim_b);
    // screen normal, to the left of a to b with y up
    let normal = Vec2::new(unit.y, -unit.x);
    match bond.kind {
        BondKind::Single => vec![(segment(a, b), false)],
        BondKind::Double => {
            let side = s.side(bond);
            if side == 0.0 {
                let o = normal * (length * 0.08);
                vec![
                    (segment(a + o, b + o), false),
                    (segment(a - o, b - o), false),
                ]
            } else {
                // the second line inside the ring (or toward the substituents), shorter
                let o = normal * (length * 0.17 * side.signum());
                let inset = unit * (length * 0.14);
                let (ia, ib) = (
                    a + o + if trim_a > 0.0 { Vec2::ZERO } else { inset },
                    b + o - if trim_b > 0.0 { Vec2::ZERO } else { inset },
                );
                vec![(segment(a, b), false), (segment(ia, ib), false)]
            }
        }
        BondKind::Triple => {
            let o = normal * (length * 0.13);
            vec![
                (segment(a, b), false),
                (segment(a + o, b + o), false),
                (segment(a - o, b - o), false),
            ]
        }
        BondKind::Wedge => {
            let w = normal * (length * 0.11);
            let mut path = BezPath::new();
            path.move_to(a);
            path.line_to(b + w);
            path.line_to(b - w);
            path.close_path();
            vec![(path, true)]
        }
        BondKind::Dash => {
            let count = 7;
            (0..count)
                .map(|i| {
                    let f = (f64::from(i) + 0.5) / f64::from(count);
                    let at = a + (b - a) * f;
                    let half = normal * (length * 0.11 * f + 1.0);
                    (segment(at - half, at + half), false)
                })
                .collect()
        }
    }
}

fn text_style(base: TextStyle, size: f32) -> TextStyle {
    TextStyle {
        size: size.round().max(6.0) as usize,
        ..base
    }
}

/// The label of atom `i`: its symbol centred on the atom, the hydrogens beside it (on the
/// side away from its bonds) with their count lowered, and the charge raised after it.
fn atom_label<'a>(
    ui: &mut Ui<'a, '_, '_>,
    s: &Structure,
    i: usize,
    at: Point,
    size: f32,
    color: Color,
    hydrogens: bool,
) -> Svgr<'a> {
    let atom = &s.atoms[i];
    let base = ui.theme.typography.body;
    let main = text_style(base, size);
    let small = text_style(base, size * 0.68);
    let (w_symbol, cap) = text_size(ui, main, &atom.symbol);
    let mut runs: Vec<(String, f64, f64, TextStyle)> = Vec::new();
    let (x, y) = (at.x, at.y);
    runs.push((atom.symbol.clone(), x, y, main));
    let half = f64::from(w_symbol) / 2.0;
    let cap = f64::from(cap);
    let mut right_end = x + half;
    if hydrogens && atom.hydrogens > 0 {
        let (w_h, _) = text_size(ui, main, "H");
        let w_h = f64::from(w_h);
        let count = (atom.hydrogens > 1).then(|| atom.hydrogens.to_string());
        let w_count = count
            .as_ref()
            .map_or(0.0, |c| f64::from(text_size(ui, small, c).0));
        // hydrogens go left when the bonds leave to the right
        let lean: f64 = s.neighbors(i).map(|n| s.atoms[n].x - atom.x).sum();
        let low = y + cap * 0.45;
        if lean > 0.1 {
            let mut left = x - half - 1.0;
            if let Some(c) = count {
                runs.push((c, left - w_count / 2.0, low, small));
                left -= w_count;
            }
            runs.push(("H".to_owned(), left - w_h / 2.0, y, main));
        } else {
            let start = x + half + 1.0;
            runs.push(("H".to_owned(), start + w_h / 2.0, y, main));
            right_end = start + w_h;
            if let Some(c) = count {
                runs.push((c, right_end + w_count / 2.0, low, small));
                right_end += w_count;
            }
        }
    }
    if atom.charge != 0 {
        let sign = if atom.charge > 0 { "+" } else { "−" };
        let text = match atom.charge.unsigned_abs() {
            1 => sign.to_owned(),
            n => format!("{n}{sign}"),
        };
        let (w, _) = text_size(ui, small, &text);
        runs.push((
            text,
            right_end + 1.0 + f64::from(w) / 2.0,
            y - cap * 0.75,
            small,
        ));
    }
    let drawn: Vec<Svgr<'a>> = runs
        .into_iter()
        .map(|(text, x, y, style)| {
            ui.show(Label {
                text: text.into(),
                x: x as f32,
                y: y as f32,
                direction: Anchor::Center,
                buff: 0.0,
                style,
                fill: color,
                ..ui.defaults()
            })
        })
        .collect();
    fframes::svgr!(<g>{drawn}</g>)
}

fn draw<'a>(ui: &mut Ui<'a, '_, '_>, s: &Structure, look: &Molecule2D<'_>) -> Svgr<'a> {
    let middle = ui.space.to_px(look.center.0, look.center.1);
    let length = f64::from(look.bond_length);
    let px = |i: usize| {
        let a = &s.atoms[i];
        middle + Vec2::new(a.x * length, -a.y * length)
    };
    let mut shapes = Vec::new();
    for bond in &s.bonds {
        if bond.a >= s.atoms.len() || bond.b >= s.atoms.len() {
            continue;
        }
        for (path, filled) in bond_paths(s, bond, px, length) {
            shapes.push(Shape {
                path,
                fill: filled.then_some(look.bond_color),
                stroke: Some(look.bond_color),
                stroke_width: look.stroke_width,
                dash: None,
                opacity: 1.0,
            });
        }
    }
    let bonds = draw_animated(ui, &shapes, look.enter.as_ref(), look.exit.as_ref(), None);
    let size = (look.bond_length * 0.42).max(8.0);
    let labels: Vec<Svgr<'a>> = (0..s.atoms.len())
        .filter(|&i| s.atoms[i].label)
        .map(|i| {
            let color = if look.color_atoms {
                atom_color(&s.atoms[i].symbol, look.bond_color)
            } else {
                look.bond_color
            };
            atom_label(ui, s, i, px(i), size, color, look.show_hydrogens)
        })
        .collect();
    let labels = fade(
        ui,
        fframes::svgr!(<g>{labels}</g>),
        look.parts_enter.as_ref(),
        look.parts_exit.as_ref(),
    );
    fframes::svgr!(<g>{bonds}{labels}</g>)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(s: &Structure, kind: BondKind) -> usize {
        s.bonds.iter().filter(|b| b.kind == kind).count()
    }

    #[test]
    fn benzene_is_a_ring_of_alternating_bonds() {
        let s = Structure::from_smiles("c1ccccc1").unwrap();
        assert_eq!(s.atoms.len(), 6);
        assert_eq!(count(&s, BondKind::Double), 3);
        assert_eq!(count(&s, BondKind::Single), 3);
        assert!(s.atoms.iter().all(|a| !a.label && a.hydrogens == 1));
        // one bond length between bonded atoms, centred on the origin
        for b in &s.bonds {
            let (a, c) = (&s.atoms[b.a], &s.atoms[b.b]);
            let d = (a.x - c.x).hypot(a.y - c.y);
            assert!((d - 1.0).abs() < 0.05, "{d}");
        }
        let (x0, y0, x1, y1) = s.bounds();
        assert!((x0 + x1).abs() < 1e-9 && (y0 + y1).abs() < 1e-9);
        // every double bond lies inside the ring
        for b in s.bonds.iter().filter(|b| b.kind == BondKind::Double) {
            assert_ne!(s.side(b), 0.0);
        }
    }

    #[test]
    fn acetic_acid_has_labels_hydrogens_and_a_balanced_carbonyl() {
        let s = Structure::from_smiles("CC(=O)O").unwrap();
        let oxygens: Vec<_> = s.atoms.iter().filter(|a| a.symbol == "O").collect();
        assert_eq!(oxygens.len(), 2);
        assert!(oxygens.iter().all(|a| a.label));
        let mut hydrogens: Vec<u8> = oxygens.iter().map(|a| a.hydrogens).collect();
        hydrogens.sort_unstable();
        assert_eq!(hydrogens, [0, 1]);
        let double = s.bonds.iter().find(|b| b.kind == BondKind::Double).unwrap();
        assert_eq!(s.side(double), 0.0);
    }

    #[test]
    fn charges_and_bad_smiles() {
        let s = Structure::from_smiles("[NH4+]").unwrap();
        assert_eq!((s.atoms[0].charge, s.atoms[0].hydrogens), (1, 4));
        assert!(Structure::from_smiles("C1CC(").is_err());
        assert!(laid_out("C1CC(").is_none());
        assert!(laid_out("CCO").is_some());
    }
}
