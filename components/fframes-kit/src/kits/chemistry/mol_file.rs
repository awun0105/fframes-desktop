use std::{error::Error, fmt};

use chematic::core::{
    Atom, AtomIdx, BondIdx, BondOrder, Element, Molecule, MoleculeBuilder, implicit_hcount,
};

use super::molecule::{BondKind, ChemAtom, ChemBond, Structure, kekulized};

/// An atom of a [`MolFile`]: its symbol, its position as written (ångström for 3D files) and
/// its charge.
#[derive(Debug, Clone, PartialEq)]
pub struct MolAtom {
    pub symbol: String,
    pub position: [f64; 3],
    pub charge: i8,
}

/// A bond of a [`MolFile`] between the atoms at indexes `a` and `b`. `order` is the file's
/// bond type: 1 to 3, 4 for aromatic. `stereo` is 1 for a wedge and 6 for a hash on a single
/// bond, narrow at `a`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MolBond {
    pub a: usize,
    pub b: usize,
    pub order: u8,
    pub stereo: u8,
}

/// Why a `.mol` or `.sdf` file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MolError {
    /// Fewer than the four header lines.
    Header,
    /// A V3000 file; only V2000 is read.
    V3000,
    /// A line (1-based in the file) that does not hold what V2000 puts there.
    Line(usize),
}

impl fmt::Display for MolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header => write!(f, "a MOL file starts with three header lines and counts"),
            Self::V3000 => write!(f, "V3000 MOL files are not read, save it as V2000"),
            Self::Line(line) => write!(f, "line {line} is not a V2000 atom, bond or property"),
        }
    }
}

impl Error for MolError {}

/// A molecule read from a V2000 `.mol` file, or from the first record of an `.sdf` file
/// (manim-chemistry's `MMoleculeObject` reads the same files). [`MolFile::structure`] gives
/// the skeletal formula for [`Molecule2D`](super::Molecule2D), and `structure_3d` (feature
/// `3d`) the atoms for [`Molecule3D`](super::Molecule3D).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MolFile {
    pub title: String,
    pub atoms: Vec<MolAtom>,
    pub bonds: Vec<MolBond>,
}

/// Columns `a..b` of `line`, trimmed; empty past its end.
fn field(line: &str, a: usize, b: usize) -> &str {
    line.get(a.min(line.len())..b.min(line.len()))
        .unwrap_or("")
        .trim()
}

/// The charge of the atom block's `ccc` field: 1 to 3 are +3 to +1, 5 to 7 are -1 to -3.
fn block_charge(code: u8) -> i8 {
    match code {
        1..=3 | 5..=7 => 4 - code as i8,
        _ => 0,
    }
}

impl MolFile {
    /// Reads a V2000 molfile by its fixed columns: three header lines (the first is the
    /// title), the counts line, the atom and bond blocks and the `M  CHG` charges up to
    /// `M  END`. An `.sdf` file is read up to its first `$$$$`. When a file has `M  CHG`
    /// lines they replace the charges of the atom block, as V2000 says.
    pub fn parse(text: &str) -> Result<Self, MolError> {
        let lines: Vec<&str> = text
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .take_while(|l| !l.starts_with("$$$$"))
            .collect();
        if lines.len() < 4 {
            return Err(MolError::Header);
        }
        let counts = lines[3];
        if counts.contains("V3000") {
            return Err(MolError::V3000);
        }
        let number = |a, b, line: usize| -> Result<usize, MolError> {
            field(lines[line], a, b)
                .parse()
                .map_err(|_| MolError::Line(line + 1))
        };
        let (atom_count, bond_count) = (number(0, 3, 3)?, number(3, 6, 3)?);
        if lines.len() < 4 + atom_count + bond_count {
            return Err(MolError::Line(lines.len() + 1));
        }
        let coordinate = |l: usize, a| -> Result<f64, MolError> {
            field(lines[l], a, a + 10)
                .parse()
                .map_err(|_| MolError::Line(l + 1))
        };
        let mut atoms = Vec::with_capacity(atom_count);
        for (l, line) in lines.iter().enumerate().skip(4).take(atom_count) {
            let symbol = field(line, 31, 34);
            if symbol.is_empty() {
                return Err(MolError::Line(l + 1));
            }
            let code = field(line, 36, 39).parse().unwrap_or(0);
            atoms.push(MolAtom {
                symbol: symbol.to_owned(),
                position: [coordinate(l, 0)?, coordinate(l, 10)?, coordinate(l, 20)?],
                charge: block_charge(code),
            });
        }
        let mut bonds = Vec::with_capacity(bond_count);
        for (l, line) in lines
            .iter()
            .enumerate()
            .skip(4 + atom_count)
            .take(bond_count)
        {
            let (a, b) = (number(0, 3, l)?, number(3, 6, l)?);
            if a == 0 || b == 0 || a > atom_count || b > atom_count || a == b {
                return Err(MolError::Line(l + 1));
            }
            bonds.push(MolBond {
                a: a - 1,
                b: b - 1,
                order: number(6, 9, l)?.try_into().unwrap_or(1),
                stereo: field(line, 9, 12).parse().unwrap_or(0),
            });
        }
        let mut charged = false;
        for (l, line) in lines.iter().enumerate().skip(4 + atom_count + bond_count) {
            if line.starts_with("M  END") {
                break;
            }
            if !line.starts_with("M  CHG") {
                continue;
            }
            if !charged {
                charged = true;
                for atom in &mut atoms {
                    atom.charge = 0;
                }
            }
            let values: Vec<i64> = line[6..]
                .split_whitespace()
                .map(|v| v.parse().map_err(|_| MolError::Line(l + 1)))
                .collect::<Result<_, _>>()?;
            let Some((&count, pairs)) = values.split_first() else {
                return Err(MolError::Line(l + 1));
            };
            if pairs.len() != 2 * count.max(0) as usize {
                return Err(MolError::Line(l + 1));
            }
            for pair in pairs.chunks(2) {
                let atom = usize::try_from(pair[0] - 1)
                    .ok()
                    .and_then(|i| atoms.get_mut(i))
                    .ok_or(MolError::Line(l + 1))?;
                atom.charge = pair[1].clamp(-15, 15) as i8;
            }
        }
        Ok(Self {
            title: lines[0].trim().to_owned(),
            atoms,
            bonds,
        })
    }

    /// Whether the file gives positions: some coordinate is not 0.
    pub fn has_coordinates(&self) -> bool {
        self.atoms
            .iter()
            .any(|a| a.position.iter().any(|v| v.abs() > 1e-6))
    }

    /// Whether the positions are 2D: every `z` is 0.
    pub fn is_flat(&self) -> bool {
        self.atoms.iter().all(|a| a.position[2].abs() <= 1e-6)
    }

    /// Whether atom `i` is a hydrogen bonded to a heavier atom, which a skeletal formula
    /// counts on that atom instead of drawing.
    fn folded(&self, i: usize) -> bool {
        self.atoms[i].symbol == "H"
            && self.bonds.iter().any(|b| {
                (b.a == i && self.atoms[b.b].symbol != "H")
                    || (b.b == i && self.atoms[b.a].symbol != "H")
            })
    }

    /// The atoms other than folded hydrogens as a `chematic` molecule (hydrogens from the
    /// usual valences), aromatic bonds as alternating single and double bonds. Returns the
    /// molecule, the file index of each of its atoms and, for each file bond, its index in the
    /// molecule. `None` when an atom is not an element.
    fn graph(&self) -> Option<(Molecule, Vec<usize>, Vec<Option<BondIdx>>)> {
        let kept: Vec<usize> = (0..self.atoms.len()).filter(|&i| !self.folded(i)).collect();
        let mut index = vec![None; self.atoms.len()];
        let mut builder = MoleculeBuilder::with_capacity(kept.len(), self.bonds.len());
        for (k, &i) in kept.iter().enumerate() {
            let atom = &self.atoms[i];
            let mut chem = Atom::new(Element::from_symbol(&atom.symbol)?);
            chem.charge = atom.charge;
            chem.aromatic = self
                .bonds
                .iter()
                .any(|b| b.order == 4 && (b.a == i || b.b == i));
            builder.add_atom(chem);
            index[i] = Some(AtomIdx(k as u32));
        }
        let mut bond_index = vec![None; self.bonds.len()];
        for (j, bond) in self.bonds.iter().enumerate() {
            let (Some(a), Some(b)) = (index[bond.a], index[bond.b]) else {
                continue;
            };
            let order = match bond.order {
                2 => BondOrder::Double,
                3 => BondOrder::Triple,
                4 => BondOrder::Aromatic,
                _ => BondOrder::Single,
            };
            bond_index[j] = builder.add_bond(a, b, order).ok();
        }
        Some((kekulized(builder.build()), kept, bond_index))
    }

    /// The skeletal formula of the molecule, hydrogens on carbons left out and those on
    /// other atoms in their labels. A 2D file keeps its positions, scaled to one bond length
    /// between bonded atoms (the median), wedges and hashes included. A file without
    /// positions, or with 3D ones, is laid out like a SMILES string.
    pub fn structure(&self) -> Structure {
        let Some((mol, kept, bond_index)) = self.graph() else {
            return Structure::default();
        };
        if !(self.has_coordinates() && self.is_flat()) {
            return Structure::laid_out(&mol);
        }
        let mut lengths: Vec<f64> = self
            .bonds
            .iter()
            .zip(&bond_index)
            .filter(|(_, k)| k.is_some())
            .map(|(b, _)| {
                let (p, q) = (self.atoms[b.a].position, self.atoms[b.b].position);
                (p[0] - q[0]).hypot(p[1] - q[1])
            })
            .filter(|&d| d > 1e-6)
            .collect();
        lengths.sort_by(f64::total_cmp);
        let scale = lengths.get(lengths.len() / 2).map_or(1.0, |&d| 1.0 / d);
        let atoms = kept
            .iter()
            .enumerate()
            .map(|(k, &i)| {
                let atom = &self.atoms[i];
                let mut chem = ChemAtom::new(
                    &atom.symbol,
                    atom.position[0] * scale,
                    atom.position[1] * scale,
                )
                .hydrogens(implicit_hcount(&mol, AtomIdx(k as u32)))
                .charge(atom.charge);
                chem.label |= atom.charge != 0;
                chem
            })
            .collect();
        let bonds = self
            .bonds
            .iter()
            .zip(&bond_index)
            .filter_map(|(b, k)| {
                let entry = mol.bond((*k)?);
                let kind = match (entry.order, b.stereo) {
                    (BondOrder::Double, _) => BondKind::Double,
                    (BondOrder::Triple, _) => BondKind::Triple,
                    (_, 1) => BondKind::Wedge,
                    (_, 6) => BondKind::Dash,
                    _ => BondKind::Single,
                };
                Some(ChemBond::new(
                    entry.atom1.0 as usize,
                    entry.atom2.0 as usize,
                    kind,
                ))
            })
            .collect();
        let mut structure = Structure::new(atoms, bonds);
        structure.center();
        structure
    }

    /// Every atom (hydrogens too) at its position in the file, for
    /// [`Molecule3D`](super::Molecule3D); aromatic bonds become alternating single and
    /// double bonds. `None` when the file gives no positions.
    #[cfg(feature = "3d")]
    pub fn structure_3d(&self) -> Option<super::Structure3D> {
        use super::Atom3D;

        if !self.has_coordinates() {
            return None;
        }
        let graph = self.graph();
        let atoms = self
            .atoms
            .iter()
            .map(|a| Atom3D {
                element: a.symbol.clone(),
                position: a.position.map(|v| v as f32),
            })
            .collect();
        let bonds = self
            .bonds
            .iter()
            .enumerate()
            .map(|(j, b)| {
                let kekule = graph.as_ref().and_then(|(mol, _, index)| {
                    Some(match mol.bond(index[j]?).order {
                        BondOrder::Double => 2,
                        BondOrder::Triple => 3,
                        _ => 1,
                    })
                });
                (b.a, b.b, kekule.unwrap_or(b.order.clamp(1, 3)))
            })
            .collect();
        Some(super::Structure3D::new(atoms, bonds))
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    const ETHANOL: &str = include_str!("samples/ethanol.mol");
    const BENZENE: &str = include_str!("samples/benzene.mol");

    fn atom_line(symbol: &str, x: f64, charge: u8) -> String {
        format!(
            "{x:>10.4}{:>10.4}{:>10.4} {symbol:<3} 0{charge:>3}  0  0",
            0.0, 0.0
        )
    }

    #[test]
    fn ethanol_folds_its_hydrogens() {
        let file = MolFile::parse(ETHANOL).unwrap();
        assert_eq!(file.title, "ethanol");
        assert_eq!((file.atoms.len(), file.bonds.len()), (9, 8));
        assert!(file.has_coordinates() && file.is_flat());
        let s = file.structure();
        assert_eq!(s.atoms.len(), 3);
        assert_eq!(s.bonds.len(), 2);
        assert!(s.atoms.iter().all(|a| a.symbol != "H"));
        let oxygen = s.atoms.iter().find(|a| a.symbol == "O").unwrap();
        assert_eq!(oxygen.hydrogens, 1);
        assert!(oxygen.label);
        // the file's 1.5 Å bonds are one bond length
        for b in &s.bonds {
            let (p, q) = (&s.atoms[b.a], &s.atoms[b.b]);
            assert!(((p.x - q.x).hypot(p.y - q.y) - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn aromatic_bonds_alternate_inside_the_ring() {
        let s = MolFile::parse(BENZENE).unwrap().structure();
        assert_eq!(s.atoms.len(), 6);
        let doubles: Vec<_> = s
            .bonds
            .iter()
            .filter(|b| b.kind == BondKind::Double)
            .collect();
        assert_eq!(doubles.len(), 3);
        assert_eq!(s.bonds.len(), 6);
        assert!(doubles.iter().all(|b| s.side(b) != 0.0));
        assert!(s.atoms.iter().all(|a| a.hydrogens == 1));
        // the file's positions, not a new layout: atom 1 is still at the top
        assert!(s.atoms[0].x.abs() < 1e-9 && s.atoms[0].y > 0.9);
    }

    #[test]
    fn the_first_sdf_record_without_positions_is_laid_out() {
        let sdf = format!(
            "water\n\n\n  1  0  0  0  0  0  0  0  0  0999 V2000\n{}\nM  END\n> <name>\nwater\n\n$$$$\nnext\n\n\n  2  0  0  0  0  0  0  0  0  0999 V2000\n",
            atom_line("O", 0.0, 0)
        );
        let file = MolFile::parse(&sdf).unwrap();
        assert_eq!(file.atoms.len(), 1);
        assert!(!file.has_coordinates());
        let s = file.structure();
        assert_eq!((s.atoms.len(), s.atoms[0].hydrogens), (1, 2));
    }

    #[test]
    fn charges_from_the_block_or_from_m_chg() {
        let block = format!(
            "ions\n\n\n  2  0  0  0  0  0  0  0  0  0999 V2000\n{}\n{}\nM  END\n",
            atom_line("Na", 0.0, 3),
            atom_line("Cl", 2.0, 5)
        );
        let file = MolFile::parse(&block).unwrap();
        assert_eq!((file.atoms[0].charge, file.atoms[1].charge), (1, -1));
        // M  CHG replaces every charge of the block
        let replaced = block.replace("M  END", "M  CHG  1   2  -2\nM  END");
        let file = MolFile::parse(&replaced).unwrap();
        assert_eq!((file.atoms[0].charge, file.atoms[1].charge), (0, -2));
    }

    #[test]
    fn columns_without_separators() {
        let n = 101;
        let mut text = format!(
            "chain\n\n\n{n:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
            n - 1
        );
        for i in 0..n {
            text += &atom_line("C", 1.5 * f64::from(i), 0);
            text += "\n";
        }
        for i in 1..n {
            writeln!(text, "{i:>3}{:>3}  1  0", i + 1).unwrap();
        }
        text += "M  END\n";
        let file = MolFile::parse(&text).unwrap();
        assert_eq!((file.atoms.len(), file.bonds.len()), (101, 100));
        assert!(text.contains("100101  1  0"));
        assert_eq!((file.bonds[99].a, file.bonds[99].b), (99, 100));
        assert_eq!(file.structure().bonds.len(), 100);
    }

    #[test]
    fn errors() {
        assert_eq!(MolFile::parse("only\ntwo\n"), Err(MolError::Header));
        let v3000 = "x\n\n\n  0  0  0     0  0            999 V3000\n";
        assert_eq!(MolFile::parse(v3000), Err(MolError::V3000));
        let short = "x\n\n\n  2  0  0  0  0  0  0  0  0  0999 V2000\n";
        assert!(matches!(MolFile::parse(short), Err(MolError::Line(_))));
        let bad_bond = BENZENE.replace("  6  1  4  0", "  6  9  4  0");
        assert_eq!(MolFile::parse(&bad_bond), Err(MolError::Line(16)));
    }

    #[cfg(feature = "3d")]
    #[test]
    fn every_atom_goes_to_3d() {
        let s = MolFile::parse(ETHANOL).unwrap().structure_3d().unwrap();
        assert_eq!((s.atoms.len(), s.bonds.len()), (9, 8));
        let s = MolFile::parse(BENZENE).unwrap().structure_3d().unwrap();
        assert_eq!(s.bonds.iter().filter(|b| b.2 == 2).count(), 3);
    }
}
