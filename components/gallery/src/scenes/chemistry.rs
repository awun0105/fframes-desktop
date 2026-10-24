//! The chemistry kit's figures, in the order of the plan's table.

use std::sync::OnceLock;

use fframes_kit::prelude::*;

use super::{core::title, latex::typeset};
use crate::GalleryScene;

fn create(delay: f32) -> EnterAnim {
    Create {
        delay,
        ..Create::default()
    }
    .into()
}

fn fade(delay: f32) -> FadeIn {
    FadeIn {
        delay,
        ..FadeIn::default()
    }
}

/// Sodium and chlorine side by side, their electrons filling the shells.
fn bohr_atom<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "BohrAtom");
    let atoms: Vec<_> = [(11, -3.6), (17, 3.6)]
        .into_iter()
        .map(|(element, x)| {
            ui.show(BohrAtom {
                element,
                center: (x, -0.2),
                shell_gap: 50.0,
                delay: 0.6,
                enter: Some(create(0.0)),
                parts_enter: Some(fade(0.3)),
                ..ui.defaults()
            })
        })
        .collect();
    svgr!(<g>{head}{atoms}</g>)
}

/// Four elements from four families.
fn element_tile<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ElementTile");
    let tiles: Vec<_> = [1, 6, 11, 26]
        .into_iter()
        .enumerate()
        .map(|(i, element)| {
            ui.show(ElementTile {
                element,
                center: (-4.5 + 3.0 * i as f64, -0.3),
                enter: Some(fade(0.2 + 0.2 * i as f32)),
                ..ui.defaults()
            })
        })
        .collect();
    svgr!(<g>{head}{tiles}</g>)
}

/// The table rising in by atomic number, then the halogens picked out.
fn periodic_table<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "PeriodicTable");
    let table = ui.show(PeriodicTable {
        center: (0.0, -0.25),
        delay: 0.3,
        highlight_group: Some(17),
        highlight_at: 2.2,
        ..ui.defaults()
    });
    svgr!(<g>{head}{table}</g>)
}

/// The atomic radius sweeping over the table: small at the top right, large at the bottom
/// left.
fn periodic_trend<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "PeriodicTrend");
    let table = ui.show(PeriodicTable {
        center: (0.0, -0.25),
        delay: 0.3,
        stagger: 0.0,
        trend: Some(Trend::AtomicRadius),
        trend_at: 1.2,
        ..ui.defaults()
    });
    svgr!(<g>{head}{table}</g>)
}

/// A carbon with its four bonds in 3D: two in the page, one wedged out and one hashed
/// behind.
fn molecule_2d<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static STRUCTURE: OnceLock<Structure> = OnceLock::new();
    let structure = STRUCTURE.get_or_init(|| {
        let (s, c) = (0.5, 0.866);
        Structure::new(
            vec![
                ChemAtom::new("C", 0.0, 0.0),
                ChemAtom::new("Br", -c, s),
                ChemAtom::new("F", c, s),
                ChemAtom::new("Cl", -0.35, -0.95),
                ChemAtom::new("H", 0.35, -0.95),
            ],
            vec![
                ChemBond::new(0, 1, BondKind::Single),
                ChemBond::new(0, 2, BondKind::Single),
                ChemBond::new(0, 3, BondKind::Wedge),
                ChemBond::new(0, 4, BondKind::Dash),
            ],
        )
    });
    let head = title(ui, "Molecule2D");
    let figure = ui.show(Molecule2D {
        structure: Some(structure),
        center: (0.0, -0.2),
        bond_length: 150.0,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.6)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// Aspirin and caffeine from their SMILES strings.
fn molecule<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Molecule");
    let aspirin = ui.show(Molecule {
        smiles: "CC(=O)Oc1ccccc1C(=O)O".into(),
        center: (-3.6, -0.3),
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.8)),
        ..ui.defaults()
    });
    let caffeine = ui.show(Molecule {
        smiles: "Cn1cnc2c1c(=O)n(C)c(=O)n2C".into(),
        center: (3.6, -0.3),
        enter: Some(create(0.3)),
        parts_enter: Some(fade(1.1)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{aspirin}{caffeine}</g>)
}

/// Propane burning: the bare equation, then its balance, with the atoms counted under it.
fn reaction_equation<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    struct Typeset {
        reaction: Reaction,
        unbalanced: Formula,
        balanced: Formula,
    }
    static TYPESET: OnceLock<Typeset> = OnceLock::new();
    let t = TYPESET.get_or_init(|| {
        let engine = TexEngine::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tex-cache"));
        let reaction = Reaction::new(&["C3H8", "O2"], &["CO2", "H2O"]);
        let unbalanced = typeset(&reaction.ce(None), reaction.typeset(&engine, false));
        let balanced = typeset(
            &reaction.ce(reaction.balance().as_deref()),
            reaction.typeset(&engine, true),
        );
        Typeset {
            reaction,
            unbalanced,
            balanced,
        }
    });
    let head = title(ui, "ReactionEquation");
    let figure = ui.show(ReactionEquation {
        reaction: Some(&t.reaction),
        unbalanced: Some(&t.unbalanced),
        balanced: Some(&t.balanced),
        center: (0.0, 0.3),
        delay: 1.0,
        run_time: 1.5,
        parts_enter: Some(fade(0.3)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// Sodium giving its outer electron to chlorine.
fn bond_formation<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "BondFormation");
    let figure = ui.show(BondFormation {
        center: (0.0, -0.3),
        at: 1.0,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.3)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// Two oxygen atoms closing in to share two pairs.
fn covalent_bond<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "CovalentBond");
    let figure = ui.show(BondFormation {
        kind: BondType::Covalent,
        left: 8,
        right: 8,
        center: (0.0, -0.3),
        distance: 600.0,
        at: 1.0,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.3)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// An exothermic reaction, then the lower path of its catalyst.
fn energy_profile<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "EnergyProfile");
    let figure = ui.show(EnergyProfile {
        center: (0.0, -0.45),
        catalyst: Some(95.0),
        catalyst_at: 2.0,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(1.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// Acetic acid titrated with sodium hydroxide, a pH scale under it following along.
fn titration_curve<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "TitrationCurve");
    let axes = Axes {
        x_range: Range::new(0.0, 50.0, 10.0),
        y_range: Range::new(0.0, 14.0, 2.0),
        x_length: 8.0,
        y_length: 4.2,
        center: (-0.6, 0.3),
        x_numbers: (0..=5).map(|i| f64::from(i) * 10.0).collect(),
        y_numbers: (0..=7).map(|i| f64::from(i) * 2.0).collect(),
        x_label: Some("NaOH (mL)".into()),
        y_label: Some("pH".into()),
        enter: Some(create(0.0)),
        ..ui.defaults()
    };
    let curve = TitrationCurve {
        axes: Some(axes.map(ui)),
        delay: 0.6,
        run_time: 4.5,
        ..ui.defaults()
    };
    let ph = curve.ph_at(ui.seconds());
    let curve = ui.show(curve);
    let axes = ui.show(axes);
    let scale = ui.show(PhScale {
        center: (-0.6, -2.75),
        width: 900.0,
        height: 36.0,
        pointer: Some(ph),
        enter: Some(create(0.2)),
        parts_enter: Some(fade(0.4)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{axes}{curve}{scale}</g>)
}

/// Everyday substances on the pH scale.
fn ph_scale<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "PhScale");
    let markers = [
        ("battery acid", 0.5),
        ("lemon juice", 2.0),
        ("coffee", 5.0),
        ("water", 7.0),
        ("baking soda", 9.0),
        ("bleach", 12.5),
    ];
    let figure = ui.show(PhScale {
        center: (0.0, -0.3),
        markers: markers.map(|(name, ph)| (name.into(), ph)).to_vec(),
        delay: 0.6,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.3)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// Ice melting and boiling away.
fn states_of_matter<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "StatesOfMatter");
    let figure = ui.show(StatesOfMatter {
        seed: 3,
        center: (0.0, -0.1),
        at: 1.0,
        duration: 5.0,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.3)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// Hydrogen and iodine settling, then more hydrogen pushing the equilibrium on.
fn equilibrium<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static MODEL: OnceLock<EquilibriumModel> = OnceLock::new();
    let model = MODEL.get_or_init(|| {
        EquilibriumModel::new(
            &[("H2", -1, 1.0), ("I2", -1, 1.0), ("HI", 2, 0.0)],
            49.0,
            0.5,
        )
        .disturb(3.0, 0, 1.0)
    });
    let head = title(ui, "Equilibrium");
    let figure = ui.show(Equilibrium {
        model: Some(model),
        delay: 0.8,
        center: (0.0, -0.4),
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.3)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("BohrAtom", 5.0, bohr_atom),
        GalleryScene::new("ElementTile", 2.5, element_tile),
        GalleryScene::new("PeriodicTable", 4.0, periodic_table),
        GalleryScene::new("PeriodicTrend", 3.5, periodic_trend),
        GalleryScene::new("Molecule2D", 3.0, molecule_2d),
        GalleryScene::new("Molecule", 3.5, molecule),
        GalleryScene::new("ReactionEquation", 3.5, reaction_equation),
        GalleryScene::new("BondFormation", 3.5, bond_formation),
        GalleryScene::new("CovalentBond", 3.5, covalent_bond),
        GalleryScene::new("EnergyProfile", 3.5, energy_profile),
        GalleryScene::new("TitrationCurve", 6.0, titration_curve),
        GalleryScene::new("PhScale", 3.5, ph_scale),
        GalleryScene::new("StatesOfMatter", 7.0, states_of_matter),
        GalleryScene::new("Equilibrium", 7.0, equilibrium),
    ]
}
