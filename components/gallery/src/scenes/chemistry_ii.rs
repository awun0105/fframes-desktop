//! The chemistry kit's second set of figures: molecule files, named molecules, molecule
//! graphs and element cards.

use std::sync::OnceLock;

use fframes_kit::prelude::*;

use crate::GalleryScene;

use super::core::title;

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

pub fn scenes() -> Vec<GalleryScene> {
    vec![GalleryScene::new("MolFile", 3.0, mol_file)]
}
