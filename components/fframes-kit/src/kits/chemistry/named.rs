use std::borrow::Cow;

use fframes::{
    Color, Svgr,
    diagnostics::{Diagnostic, report},
};

use crate::{
    anim::{FadeIn, FadeOut},
    animate::{EnterAnim, ExitAnim},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::Molecule;

/// Common molecules by name and their SMILES, written for the kit from public facts.
const MOLECULES: &[(&str, &str)] = &[
    // small inorganic molecules
    ("water", "O"),
    ("hydrogen", "[H][H]"),
    ("oxygen", "O=O"),
    ("ozone", "[O-][O+]=O"),
    ("nitrogen", "N#N"),
    ("carbon dioxide", "O=C=O"),
    ("carbon monoxide", "[C-]#[O+]"),
    ("ammonia", "N"),
    ("hydrogen peroxide", "OO"),
    ("hydrogen sulfide", "S"),
    ("hydrogen chloride", "Cl"),
    ("hydrogen cyanide", "C#N"),
    ("sulfur dioxide", "O=S=O"),
    ("sulfuric acid", "OS(=O)(=O)O"),
    ("nitric acid", "O[N+](=O)[O-]"),
    ("phosphoric acid", "OP(=O)(O)O"),
    ("sodium chloride", "[Na+].[Cl-]"),
    ("sodium hydroxide", "[Na+].[OH-]"),
    // hydrocarbons
    ("methane", "C"),
    ("ethane", "CC"),
    ("propane", "CCC"),
    ("butane", "CCCC"),
    ("isobutane", "CC(C)C"),
    ("pentane", "CCCCC"),
    ("hexane", "CCCCCC"),
    ("heptane", "CCCCCCC"),
    ("octane", "CCCCCCCC"),
    ("cyclopropane", "C1CC1"),
    ("cyclohexane", "C1CCCCC1"),
    ("ethylene", "C=C"),
    ("ethene", "C=C"),
    ("propylene", "CC=C"),
    ("acetylene", "C#C"),
    ("ethyne", "C#C"),
    ("limonene", "CC1=CCC(CC1)C(C)=C"),
    // aromatics and heterocycles
    ("benzene", "c1ccccc1"),
    ("toluene", "Cc1ccccc1"),
    ("xylene", "Cc1ccccc1C"),
    ("styrene", "C=Cc1ccccc1"),
    ("naphthalene", "c1ccc2ccccc2c1"),
    ("anthracene", "c1ccc2cc3ccccc3cc2c1"),
    ("phenol", "Oc1ccccc1"),
    ("aniline", "Nc1ccccc1"),
    ("chlorobenzene", "Clc1ccccc1"),
    ("nitrobenzene", "[O-][N+](=O)c1ccccc1"),
    ("benzaldehyde", "O=Cc1ccccc1"),
    ("benzoic acid", "OC(=O)c1ccccc1"),
    ("salicylic acid", "OC(=O)c1ccccc1O"),
    ("pyridine", "c1ccncc1"),
    ("pyrrole", "c1cc[nH]c1"),
    ("furan", "c1ccoc1"),
    ("thiophene", "c1ccsc1"),
    ("imidazole", "c1cnc[nH]1"),
    ("indole", "c1ccc2[nH]ccc2c1"),
    (
        "trinitrotoluene",
        "Cc1c(cc(cc1[N+](=O)[O-])[N+](=O)[O-])[N+](=O)[O-]",
    ),
    // alcohols, ethers, carbonyls, acids
    ("methanol", "CO"),
    ("ethanol", "CCO"),
    ("propanol", "CCCO"),
    ("isopropanol", "CC(C)O"),
    ("isopropyl alcohol", "CC(C)O"),
    ("butanol", "CCCCO"),
    ("ethylene glycol", "OCCO"),
    ("glycerol", "OCC(O)CO"),
    ("menthol", "CC(C)[C@@H]1CC[C@@H](C)C[C@H]1O"),
    ("dimethyl ether", "COC"),
    ("diethyl ether", "CCOCC"),
    ("formaldehyde", "C=O"),
    ("acetaldehyde", "CC=O"),
    ("acetone", "CC(C)=O"),
    ("cinnamaldehyde", "O=C/C=C/c1ccccc1"),
    ("vanillin", "COc1cc(C=O)ccc1O"),
    ("formic acid", "OC=O"),
    ("acetic acid", "CC(=O)O"),
    ("propionic acid", "CCC(=O)O"),
    ("butyric acid", "CCCC(=O)O"),
    ("oxalic acid", "OC(=O)C(=O)O"),
    ("lactic acid", "CC(O)C(=O)O"),
    ("citric acid", "OC(=O)CC(O)(CC(=O)O)C(=O)O"),
    ("ethyl acetate", "CCOC(C)=O"),
    (
        "nitroglycerin",
        "C(C(CO[N+](=O)[O-])O[N+](=O)[O-])O[N+](=O)[O-]",
    ),
    // nitrogen, sulfur and halogen compounds
    ("methylamine", "CN"),
    ("ethylamine", "CCN"),
    ("trimethylamine", "CN(C)C"),
    ("urea", "NC(N)=O"),
    ("acetonitrile", "CC#N"),
    ("dimethyl sulfoxide", "CS(C)=O"),
    ("chloroform", "ClC(Cl)Cl"),
    ("dichloromethane", "ClCCl"),
    ("carbon tetrachloride", "ClC(Cl)(Cl)Cl"),
    // sugars and vitamins
    ("glucose", "OC[C@H]1OC(O)[C@H](O)[C@@H](O)[C@@H]1O"),
    ("fructose", "OC[C@@H]1O[C@@](O)(CO)[C@@H](O)[C@@H]1O"),
    (
        "sucrose",
        "OC[C@H]1O[C@@](CO)(O[C@H]2O[C@H](CO)[C@@H](O)[C@H](O)[C@H]2O)[C@@H](O)[C@@H]1O",
    ),
    ("ascorbic acid", "OC[C@H](O)[C@H]1OC(=O)C(O)=C1O"),
    ("vitamin c", "OC[C@H](O)[C@H]1OC(=O)C(O)=C1O"),
    // amino acids
    ("glycine", "NCC(=O)O"),
    ("alanine", "C[C@H](N)C(=O)O"),
    ("serine", "N[C@@H](CO)C(=O)O"),
    ("cysteine", "N[C@@H](CS)C(=O)O"),
    ("phenylalanine", "N[C@@H](Cc1ccccc1)C(=O)O"),
    ("tyrosine", "N[C@@H](Cc1ccc(O)cc1)C(=O)O"),
    ("tryptophan", "N[C@@H](Cc1c[nH]c2ccccc12)C(=O)O"),
    ("glutamic acid", "N[C@@H](CCC(=O)O)C(=O)O"),
    ("lysine", "NCCCC[C@H](N)C(=O)O"),
    // nucleobases
    ("adenine", "Nc1ncnc2[nH]cnc12"),
    ("guanine", "Nc1nc2[nH]cnc2c(=O)[nH]1"),
    ("cytosine", "Nc1cc[nH]c(=O)n1"),
    ("thymine", "Cc1c[nH]c(=O)[nH]c1=O"),
    ("uracil", "O=c1cc[nH]c(=O)[nH]1"),
    // drugs, hormones and natural products
    ("caffeine", "Cn1cnc2c1c(=O)n(C)c(=O)n2C"),
    ("aspirin", "CC(=O)Oc1ccccc1C(=O)O"),
    ("paracetamol", "CC(=O)Nc1ccc(O)cc1"),
    ("acetaminophen", "CC(=O)Nc1ccc(O)cc1"),
    ("ibuprofen", "CC(C)Cc1ccc(cc1)C(C)C(=O)O"),
    ("nicotine", "CN1CCC[C@H]1c1cccnc1"),
    ("dopamine", "NCCc1ccc(O)c(O)c1"),
    ("serotonin", "NCCc1c[nH]c2ccc(O)cc12"),
    ("adrenaline", "CNC[C@H](O)c1ccc(O)c(O)c1"),
    ("epinephrine", "CNC[C@H](O)c1ccc(O)c(O)c1"),
    ("histamine", "NCCc1cnc[nH]1"),
    ("melatonin", "CC(=O)NCCc1c[nH]c2ccc(OC)cc12"),
    ("capsaicin", "COc1cc(CNC(=O)CCCC/C=C/C(C)C)ccc1O"),
    (
        "cholesterol",
        "CC(C)CCC[C@@H](C)[C@H]1CC[C@H]2[C@@H]3CC=C4C[C@@H](O)CC[C@]4(C)[C@H]3CC[C@]12C",
    ),
    (
        "testosterone",
        "C[C@]12CC[C@H]3[C@@H](CCC4=CC(=O)CC[C@@]43C)[C@@H]1CC[C@@H]2O",
    ),
    (
        "morphine",
        "CN1CC[C@]23[C@@H]4Oc5c3c(C[C@@H]1[C@@H]2C=C[C@@H]4O)ccc5O",
    ),
    (
        "quinine",
        "COc1ccc2nccc([C@@H](O)[C@@H]3C[C@@H]4CCN3C[C@@H]4C=C)c2c1",
    ),
    (
        "penicillin g",
        "CC1(C)S[C@@H]2[C@H](NC(=O)Cc3ccccc3)C(=O)N2[C@H]1C(=O)O",
    ),
];

/// The SMILES of the molecule called `name` in the kit's table of about a hundred common
/// molecules (`"caffeine"`, `"glucose"`, `"vitamin C"`). Case, hyphens and underscores do
/// not matter.
pub fn named_smiles(name: &str) -> Option<&'static str> {
    let wanted: String = name
        .trim()
        .chars()
        .map(|c| match c {
            '-' | '_' => ' ',
            c => c.to_ascii_lowercase(),
        })
        .collect();
    MOLECULES
        .iter()
        .find(|(n, _)| *n == wanted)
        .map(|(_, smiles)| *smiles)
}

/// The names in the table, with their SMILES.
pub fn named_molecules() -> impl Iterator<Item = (&'static str, &'static str)> {
    MOLECULES.iter().copied()
}

/// A molecule by its common name (manim-chemistry's `NamedMolecule`), looked up in a table
/// of about a hundred common molecules ([`named_molecules`]) and drawn like
/// [`Molecule`]. Nothing is looked up over the network. A name that is not in the table
/// draws nothing, and `inspect` warns about it.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct NamedMolecule<'a> {
    pub name: Cow<'a, str>,
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

impl Themed for NamedMolecule<'_> {
    fn themed(t: &Theme) -> Self {
        let m = Molecule::themed(t);
        Self {
            name: Cow::Borrowed("caffeine"),
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

impl Default for NamedMolecule<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for NamedMolecule<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(smiles) = named_smiles(&self.name) else {
            report(Diagnostic::RendererWarning {
                message: format!("NamedMolecule: no molecule named {:?}", self.name),
            });
            return Svgr::empty();
        };
        Molecule {
            smiles: Cow::Borrowed(smiles),
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
        }
        .render(ui)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kits::chemistry::Structure;

    #[test]
    fn every_smiles_in_the_table_is_laid_out() {
        let mut names = std::collections::HashSet::new();
        for (name, smiles) in named_molecules() {
            assert!(names.insert(name), "{name} twice");
            assert_eq!(name, name.to_lowercase());
            let s = Structure::from_smiles(smiles).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!s.atoms.is_empty(), "{name}");
            assert!(
                s.atoms.iter().all(|a| a.x.is_finite() && a.y.is_finite()),
                "{name}"
            );
        }
        assert!(names.len() >= 100, "{}", names.len());
    }

    #[test]
    fn heavy_atoms_of_well_known_molecules() {
        for (name, heavy) in [
            ("benzene", 6),
            ("glucose", 12),
            ("aspirin", 13),
            ("caffeine", 14),
            ("testosterone", 21),
            ("morphine", 21),
            ("sucrose", 23),
            ("quinine", 24),
            ("cholesterol", 28),
        ] {
            let s = Structure::from_smiles(named_smiles(name).unwrap()).unwrap();
            assert_eq!(s.atoms.len(), heavy, "{name}");
        }
    }

    #[test]
    fn names_are_found_loosely() {
        assert_eq!(named_smiles("Vitamin-C"), named_smiles("ascorbic acid"));
        assert_eq!(named_smiles("  CARBON_DIOXIDE "), Some("O=C=O"));
        assert_eq!(named_smiles("unobtainium"), None);
    }
}
