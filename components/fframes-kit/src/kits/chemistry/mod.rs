//! The chemistry kit (feature `chemistry`): atoms, the periodic table and molecules, with the
//! element data of [`mendeleev`] and the 2D depictions of [`chematic`].

mod bohr;
mod bond_formation;
mod energy_profile;
mod equilibrium;
mod molecule;
mod periodic;
mod reaction;
mod states;
mod titration;

pub use bohr::BohrAtom;
pub use bond_formation::{BondFormation, BondType, bond_electrons, missing, valence};
pub use energy_profile::EnergyProfile;
pub use equilibrium::{Equilibrium, EquilibriumModel};
pub use molecule::{BondKind, ChemAtom, ChemBond, Molecule, Molecule2D, SmilesError, Structure};
pub use periodic::{ElementTile, PeriodicTable, Trend};
#[cfg(feature = "latex")]
pub use reaction::ReactionEquation;
pub use reaction::{Reaction, atom_counts};
pub use states::{MatterState, StatesOfMatter};
pub use titration::{PhScale, Titration, TitrationCurve, indicator_color};

use fframes::Color;
use mendeleev::{ALL_ELEMENTS, Element};

/// The element with atomic number `number` (1 to 118).
fn element(number: u32) -> Option<Element> {
    let index = usize::try_from(number).ok()?.checked_sub(1)?;
    ALL_ELEMENTS.get(index).copied()
}

/// The symbol of element `number`, `"?"` outside 1 to 118.
pub fn symbol(number: u32) -> &'static str {
    element(number).map_or("?", |e| e.symbol())
}

/// The English name of element `number`, `""` outside 1 to 118.
pub fn name(number: u32) -> &'static str {
    element(number).map_or("", |e| e.name())
}

/// The standard atomic weight of element `number` (the conventional value of an interval,
/// the mass number of the most stable isotope when there is none).
pub fn atomic_weight(number: u32) -> Option<f64> {
    element(number).map(|e| f64::from(&e.atomic_weight()))
}

/// The atomic number of the element with `symbol` (`"Na"` gives 11).
pub fn atomic_number(symbol: &str) -> Option<u32> {
    ALL_ELEMENTS
        .iter()
        .find(|e| e.symbol().eq_ignore_ascii_case(symbol))
        .map(Element::atomic_number)
}

/// The electrons of element `number` in each shell, innermost first, from its ground state
/// configuration: `[2, 8, 1]` for sodium, `[2, 8, 18, 1]` for copper.
pub fn shells(number: u32) -> Vec<u32> {
    fn fill(e: Element, shells: &mut Vec<u32>) {
        let config = e.electronic_configuration();
        if let Some(core) = config.noble_gas {
            fill(core, shells);
        }
        for sub in config.valence_subshells {
            let i = sub.shell_number.saturating_sub(1) as usize;
            if shells.len() <= i {
                shells.resize(i + 1, 0);
            }
            shells[i] += sub.number_of_electrons;
        }
    }
    let mut shells = Vec::new();
    if let Some(e) = element(number) {
        fill(e, &mut shells);
    }
    shells
}

/// The family an element belongs to, which colours its tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    AlkaliMetal,
    AlkalineEarthMetal,
    TransitionMetal,
    PostTransitionMetal,
    Metalloid,
    Nonmetal,
    Halogen,
    NobleGas,
    Lanthanide,
    Actinide,
}

impl Category {
    /// The category of element `number`.
    pub fn of(number: u32) -> Self {
        use Category::*;
        match number {
            3 | 11 | 19 | 37 | 55 | 87 => AlkaliMetal,
            4 | 12 | 20 | 38 | 56 | 88 => AlkalineEarthMetal,
            57..=71 => Lanthanide,
            89..=103 => Actinide,
            2 | 10 | 18 | 36 | 54 | 86 | 118 => NobleGas,
            9 | 17 | 35 | 53 | 85 | 117 => Halogen,
            5 | 14 | 32 | 33 | 51 | 52 => Metalloid,
            1 | 6 | 7 | 8 | 15 | 16 | 34 => Nonmetal,
            21..=30 | 39..=48 | 72..=80 | 104..=112 => TransitionMetal,
            _ => PostTransitionMetal,
        }
    }

    /// A colour for the category that reads on a dark background.
    pub fn color(self) -> Color {
        use Category::*;
        Color::hex(match self {
            AlkaliMetal => "#ff6b6b",
            AlkalineEarthMetal => "#ffa94d",
            TransitionMetal => "#ffd43b",
            PostTransitionMetal => "#69db7c",
            Metalloid => "#38d9a9",
            Nonmetal => "#4dabf7",
            Halogen => "#748ffc",
            NobleGas => "#9775fa",
            Lanthanide => "#f783ac",
            Actinide => "#e599f7",
        })
    }

    /// The name, lowercase (`"alkali metal"`).
    pub fn name(self) -> &'static str {
        use Category::*;
        match self {
            AlkaliMetal => "alkali metal",
            AlkalineEarthMetal => "alkaline earth metal",
            TransitionMetal => "transition metal",
            PostTransitionMetal => "post-transition metal",
            Metalloid => "metalloid",
            Nonmetal => "nonmetal",
            Halogen => "halogen",
            NobleGas => "noble gas",
            Lanthanide => "lanthanide",
            Actinide => "actinide",
        }
    }
}

/// The cell of element `number` in the 18-column table: `(column, row)` from 0, with the
/// lanthanides and actinides in rows 7 and 8 under the main table, from column 2.
pub fn table_cell(number: u32) -> Option<(u32, u32)> {
    let e = element(number)?;
    match number {
        57..=71 => Some((number - 57 + 2, 7)),
        89..=103 => Some((number - 89 + 2, 8)),
        _ => Some((e.group()?.group_number() - 1, e.period() - 1)),
    }
}
/// The colour drawn for an atom of `symbol` in a molecule: carbon and hydrogen in `text`,
/// the common heteroatoms in colours that read on a dark background.
pub(crate) fn atom_color(symbol: &str, text: Color) -> Color {
    match symbol {
        "O" => Color::hex("#ff6b6b"),
        "N" => Color::hex("#6ea8ff"),
        "S" => Color::hex("#ffd166"),
        "P" => Color::hex("#ffa94d"),
        "F" | "Cl" => Color::hex("#7bd389"),
        "Br" => Color::hex("#e08e6d"),
        "I" => Color::hex("#c5a3ff"),
        _ => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_data() {
        assert_eq!(symbol(11), "Na");
        assert_eq!(name(26), "Iron");
        assert_eq!(symbol(0), "?");
        assert_eq!(atomic_number("cl"), Some(17));
        assert!((atomic_weight(6).unwrap() - 12.011).abs() < 0.01);
    }

    #[test]
    fn shells_follow_the_configuration() {
        assert_eq!(shells(1), [1]);
        assert_eq!(shells(11), [2, 8, 1]);
        assert_eq!(shells(26), [2, 8, 14, 2]);
        assert_eq!(shells(29), [2, 8, 18, 1]);
        for n in 1..=118 {
            assert_eq!(shells(n).iter().sum::<u32>(), n, "{n}");
        }
    }

    #[test]
    fn every_element_has_its_own_cell() {
        let mut seen = std::collections::HashSet::new();
        for n in 1..=118 {
            let (c, r) = table_cell(n).unwrap();
            assert!(c < 18 && r < 9, "{n}: {c} {r}");
            assert!(seen.insert((c, r)), "{n} shares {c} {r}");
        }
        assert_eq!(table_cell(1), Some((0, 0)));
        assert_eq!(table_cell(5), Some((12, 1)));
        assert_eq!(table_cell(26), Some((7, 3)));
        assert_eq!(table_cell(56), Some((1, 5)));
        assert_eq!(table_cell(72), Some((3, 5)));
        assert_eq!(table_cell(86), Some((17, 5)));
        assert_eq!(table_cell(57), Some((2, 7)));
        assert_eq!(table_cell(118), Some((17, 6)));
    }

    #[test]
    fn categories() {
        assert_eq!(Category::of(11), Category::AlkaliMetal);
        assert_eq!(Category::of(26), Category::TransitionMetal);
        assert_eq!(Category::of(13), Category::PostTransitionMetal);
        assert_eq!(Category::of(18), Category::NobleGas);
        assert_eq!(Category::of(92), Category::Actinide);
    }
}
