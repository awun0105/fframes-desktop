//! The chemistry kit's second set of figures: molecule files, named molecules, molecule
//! graphs and element cards.

use std::sync::OnceLock;

use fframes_kit::prelude::*;

use crate::GalleryScene;

use super::core::{hue, neon, title};

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

/// Paracetamol drawn from a 2D molfile, its aromatic ring as alternating bonds.
const PARACETAMOL: &str = r"paracetamol
  fframes-kit

 11 11  0  0  0  0  0  0  0  0999 V2000
    1.5000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    0.7500    1.2990    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
   -0.7500    1.2990    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
   -1.5000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
   -0.7500   -1.2990    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    0.7500   -1.2990    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    3.0000    0.0000    0.0000 N   0  0  0  0  0  0  0  0  0  0  0  0
    3.7500    1.2990    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    3.0000    2.5980    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0
    5.2500    1.2990    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
   -3.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  4  0
  2  3  4  0
  3  4  4  0
  4  5  4  0
  5  6  4  0
  6  1  4  0
  1  7  1  0
  7  8  1  0
  8  9  2  0
  8 10  1  0
  4 11  1  0
M  END
";

/// Ethanol with its hydrogens in 3D, the first record of an SDF file.
#[cfg(feature = "three-d")]
const ETHANOL: &str = r"ethanol
  fframes-kit

  9  8  0  0  0  0  0  0  0  0999 V2000
    1.1879   -0.3829    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    0.0000    0.5526    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
   -1.1867   -0.2472    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0
   -1.9237    0.3850    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0
    2.0985    0.2306    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0
    1.1184   -1.0093    0.8869 H   0  0  0  0  0  0  0  0  0  0  0  0
    1.1184   -1.0093   -0.8869 H   0  0  0  0  0  0  0  0  0  0  0  0
   -0.0227    1.1812    0.8852 H   0  0  0  0  0  0  0  0  0  0  0  0
   -0.0227    1.1812   -0.8852 H   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  1  0
  2  3  1  0
  3  4  1  0
  1  5  1  0
  1  6  1  0
  1  7  1  0
  2  8  1  0
  2  9  1  0
M  END
> <name>
ethanol

$$$$
";

/// A 2D molfile as a skeletal formula, and with `three-d` a 3D SDF record as balls and
/// sticks beside it.
fn mol_file<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static FLAT: OnceLock<Structure> = OnceLock::new();
    let flat = FLAT.get_or_init(|| {
        MolFile::parse(PARACETAMOL)
            .map(|f| f.structure())
            .unwrap_or_default()
    });
    let head = title(ui, "MolFile");
    #[cfg(feature = "three-d")]
    let (x, solid) = {
        static SOLID: OnceLock<Option<Structure3D>> = OnceLock::new();
        let solid = SOLID.get_or_init(|| MolFile::parse(ETHANOL).ok()?.structure_3d());
        let figure = ui.show(Molecule3D {
            structure: solid.as_ref(),
            center: (3.6, -0.3),
            width: 760.0,
            height: 640.0,
            spin: 30.0,
            enter: Some(fade(0.8)),
            ..ui.defaults()
        });
        (-3.2, figure)
    };
    #[cfg(not(feature = "three-d"))]
    let (x, solid) = (0.0, Svgr::empty());
    let figure = ui.show(Molecule2D {
        structure: Some(flat),
        center: (x, -0.3),
        bond_length: 90.0,
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.6)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}{solid}</g>)
}

/// Three molecules looked up by name, each named under its formula.
fn named_molecule<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "NamedMolecule");
    let figures: Vec<_> = ["caffeine", "vanillin", "dopamine"]
        .into_iter()
        .zip(0_u8..)
        .map(|(name, i)| {
            let x = -4.8 + 4.8 * f64::from(i);
            let delay = 0.4 * f32::from(i);
            let molecule = ui.show(NamedMolecule {
                name: name.into(),
                center: (x, 0.1),
                bond_length: 60.0,
                enter: Some(create(delay)),
                parts_enter: Some(fade(delay + 0.5)),
                ..ui.defaults()
            });
            let at = ui.space.to_px(x, -2.4);
            let label = ui.show(Label {
                text: name.into(),
                x: at.x as f32,
                y: at.y as f32,
                direction: Anchor::Center,
                enter: Some(fade(delay + 0.8)),
                ..ui.defaults()
            });
            svgr!(<g>{molecule}{label}</g>)
        })
        .collect();
    svgr!(<g>{head}{figures}</g>)
}

/// Hexan-1-ol as a graph: its hydroxyl lights up and changes colour while the end of the
/// chain turns half a turn about the middle bond, out of the page half way.
fn bond_rotation<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static ALCOHOL: OnceLock<Structure> = OnceLock::new();
    let alcohol = ALCOHOL.get_or_init(|| Structure::from_smiles("OCCCCCC").unwrap_or_default());
    let head = title(ui, "BondRotation");
    let hydroxyl = functional_groups(alcohol, FunctionalGroup::Hydroxyl)
        .into_iter()
        .next()
        .unwrap_or_default();
    // the middle bond of the chain
    let bond = alcohol
        .bonds
        .iter()
        .position(|b| (b.a.min(b.b), b.a.max(b.b)) == (3, 4))
        .unwrap_or(0);
    let accent = ui.theme.colors.accent;
    let highlight = ui.theme.colors.highlight;
    let figure = ui.show(GraphMolecule {
        structure: Some(alcohol),
        center: (0.0, -0.3),
        bond_length: 110.0,
        atom_radius: 26.0,
        highlights: vec![GroupHighlight {
            atoms: hydroxyl.clone(),
            color: accent,
            start: 0.4,
            duration: 0.6,
        }],
        rotations: vec![BondRotation {
            bond,
            degrees: 180.0,
            start: 1.0,
            duration: 2.0,
        }],
        recolors: vec![Recolor {
            parts: hydroxyl.iter().map(|&o| MoleculePart::Atom(o)).collect(),
            color: highlight,
            start: 1.5,
            duration: 1.0,
        }],
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// Carbon and iron as square cards and bromine as a wide card, one after the other.
fn element_card<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ElementCard");
    let cards: Vec<_> = [
        (6, -5.0, CardLayout::Square),
        (26, -2.2, CardLayout::Square),
        (35, 3.3, CardLayout::Wide),
    ]
    .into_iter()
    .zip(0_u8..)
    .map(|((element, x, layout), i)| {
        ui.show(ElementCard {
            element,
            center: (x, 0.0),
            size: 300.0,
            layout,
            enter: Some(fade(0.3 * f32::from(i))),
            ..ui.defaults()
        })
    })
    .collect();
    svgr!(<g>{head}{cards}</g>)
}

/// The molecules of [`molecule_parade`], in order.
const PARADE: [&str; 5] = ["caffeine", "dopamine", "serotonin", "vitamin c", "aspirin"];

/// Five molecules from their names: each draws on large in its own neon colour, then shrinks
/// into its place in a lineup, which ends named.
fn molecule_parade<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "MoleculeParade");
    let t = ui.seconds();
    let n = PARADE.len() as f32;
    let figures: Vec<Svgr<'a>> = PARADE
        .into_iter()
        .zip(0_u8..)
        .map(|(name, i)| {
            let start = 0.3 + 0.95 * f32::from(i);
            let color = hue(f32::from(i) / n);
            let molecule = ui.show(NamedMolecule {
                name: name.into(),
                center: (0.0, 0.0),
                bond_length: 80.0,
                bond_color: color,
                stroke_width: 5.0,
                enter: Some(
                    Create {
                        delay: start,
                        run_time: 0.7,
                        ..Create::default()
                    }
                    .into(),
                ),
                parts_enter: Some(fade(start + 0.5)),
                ..ui.defaults()
            });
            // from the middle, large, down into its slot in the row
            let p = motion::cubic_in_out(motion::progress(t, start + 0.85, start + 1.3));
            let slot = (960.0 + (f32::from(i) - 2.0) * 360.0, 600.0);
            let (x, y) = (
                motion::lerp(960.0, slot.0, p),
                motion::lerp(520.0, slot.1, p),
            );
            let scale = motion::lerp(1.15, 0.58, p);
            let label = ui.show(Label {
                text: name.into(),
                x: slot.0,
                y: slot.1 + 190.0,
                direction: Anchor::Center,
                fill: color,
                enter: Some(fade(start + 1.2)),
                ..ui.defaults()
            });
            svgr!(
                <g>
                    <g transform={Transform::translate(x, y)}>
                        <g transform={Transform::scale(scale)}>
                            <g transform={Transform::translate(-960.0, -540.0)}>{molecule}</g>
                        </g>
                    </g>
                    {label}
                </g>
            )
        })
        .collect();
    let glow = neon("gallery-parade-glow", 5.0);
    svgr!(
        <g>
            <defs>{glow}</defs>
            {head}
            <g filter="url(#gallery-parade-glow)">{figures}</g>
        </g>
    )
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("MolFile", 3.0, mol_file),
        GalleryScene::new("NamedMolecule", 3.0, named_molecule),
        GalleryScene::new("BondRotation", 4.0, bond_rotation),
        GalleryScene::new("ElementCard", 2.0, element_card),
        GalleryScene::new("MoleculeParade", 6.5, molecule_parade),
    ]
}
