//! The biology kit's figures, in the order of the plan's table.

use fframes_kit::prelude::*;

use super::core::{hue, neon, title};
use crate::GalleryScene;

fn fade(delay: f32) -> FadeIn {
    FadeIn {
        delay,
        ..FadeIn::default()
    }
}

fn cell<'a>(ui: &mut Ui<'a, '_, '_>, kind: CellKind, name: &'static str) -> Svgr<'a> {
    let head = title(ui, name);
    let figure = ui.show(Cell {
        kind,
        delay: 0.6,
        center: (0.0, -0.3),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn animal_cell<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    cell(ui, CellKind::Animal, "Cell")
}

fn plant_cell<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    cell(ui, CellKind::Plant, "PlantCell")
}

fn dna_helix<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "DnaHelix");
    let figure = ui.show(DnaHelix {
        delay: 0.6,
        center: (0.0, -0.3),
        unzip_at: Some(5.4),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// The helix in 3D, its pairs building from the left while it turns.
#[cfg(feature = "three-d")]
fn dna_helix_3d<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "DnaHelix3D");
    let figure = ui.show(DnaHelix3D {
        delay: 0.6,
        center: (0.0, -0.2),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn transcription<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Transcription");
    let figure = ui.show(Transcription {
        delay: 0.6,
        center: (0.4, -0.2),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn translation<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Translation");
    let figure = ui.show(Translation {
        delay: 0.6,
        center: (0.0, -0.6),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn mitosis<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "MitosisStages");
    let figure = ui.show(MitosisStages {
        delay: 0.6,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn neuron<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Neuron");
    let figure = ui.show(Neuron {
        delay: 0.6,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn action_potential<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ActionPotential");
    let figure = ui.show(ActionPotential {
        delay: 0.8,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn population<'a>(ui: &mut Ui<'a, '_, '_>, name: &'static str, model: PopulationModel) -> Svgr<'a> {
    let head = title(ui, name);
    let figure = ui.show(PopulationCurve {
        model,
        years: if model == PopulationModel::Logistic {
            15.0
        } else {
            20.0
        },
        delay: 0.8,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn logistic<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    population(ui, "PopulationCurve", PopulationModel::Logistic)
}

fn predator_prey<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    population(ui, "PredatorPrey", PopulationModel::PREDATOR_PREY)
}

fn phylo_tree<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "PhyloTree");
    let figure = ui.show(PhyloTree {
        delay: 0.6,
        highlight: Some("hair".into()),
        highlight_at: 5.6,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn food_web<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "FoodWeb");
    let figure = ui.show(FoodWeb {
        delay: 0.6,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn circulation<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Circulation");
    let figure = ui.show(Circulation {
        delay: 0.6,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn enzyme<'a>(ui: &mut Ui<'a, '_, '_>, induced: bool, name: &'static str) -> Svgr<'a> {
    let head = title(ui, name);
    let figure = ui.show(EnzymeFit {
        induced,
        delay: 0.6,
        center: (0.0, -0.1),
        scale: 1.3,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn induced_fit<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    enzyme(ui, true, "EnzymeFit")
}

fn lock_and_key<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    enzyme(ui, false, "LockAndKey")
}

fn punnett<'a>(
    ui: &mut Ui<'a, '_, '_>,
    name: &'static str,
    parents: (&str, &str),
    traits: &[(&str, &str)],
    colors: Option<&[&str]>,
) -> Svgr<'a> {
    let head = title(ui, name);
    let figure = ui.show(PunnettSquare {
        parents: (parents.0.into(), parents.1.into()),
        traits: traits.iter().map(|&(d, r)| (d.into(), r.into())).collect(),
        phenotype_colors: colors.map_or_else(
            || ui.defaults::<PunnettSquare>().phenotype_colors,
            |c| c.iter().map(|&h| Color::hex(h)).collect(),
        ),
        delay: 0.6,
        center: (0.0, -0.2),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn monohybrid<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    punnett(
        ui,
        "PunnettSquare",
        ("Pp", "Pp"),
        &[("purple", "white")],
        None,
    )
}

fn dihybrid<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    punnett(
        ui,
        "DihybridCross",
        ("YyRr", "YyRr"),
        &[("yellow", "green"), ("round", "wrinkled")],
        // yellow round, yellow wrinkled, green round, green wrinkled
        Some(&["#ffd43b", "#f59f00", "#8ce99a", "#2f9e44"]),
    )
}

/// The neurons of [`neural_relay`]: a zigzag of three, each one's terminals on the next
/// one's dendrites, and when each fires after the one before.
const RELAY: usize = 3;
const RELAY_LENGTH: f32 = 680.0;
const RELAY_SPEED: f32 = 900.0;

/// Three neurons in a zigzag pass a signal along: each fires when the impulse of the one
/// before reaches its terminals, twice, in its own colour.
fn neural_relay<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "NeuralRelay");
    // the impulse takes this long down an axon, a little less than the whole cell
    let hop = RELAY_LENGTH / RELAY_SPEED;
    let unit = f64::from(ui.space.unit);
    let figures: Vec<Svgr<'a>> = (0..RELAY)
        .map(|i| {
            let x = -3.7 + 3.7 * i as f64;
            let down = i % 2 == 0;
            let y = if down { 0.35 } else { -0.75 };
            let angle = if down { 14.0 } else { -14.0 };
            let color = hue(0.55 + 0.12 * i as f32);
            let first = 1.5 + hop * i as f32;
            let neuron = ui.show(Neuron {
                delay: 0.25 * i as f32,
                center: (x, y),
                length: RELAY_LENGTH,
                impulses: vec![first, first + 2.0],
                speed: RELAY_SPEED,
                labels: false,
                seed: 3 + i as u64,
                body_color: color,
                nucleus_color: color.with_alpha(90),
                myelin_color: color.with_alpha(170),
                impulse_color: Color::hex("#fff4b0"),
                enter: Some(fade(0.25 * i as f32)),
                ..ui.defaults()
            });
            let (cx, cy) = (960.0 + x * unit, 540.0 - y * unit);
            svgr!(
                <g transform={Transform { translate_x: cx, translate_y: cy, rotate: angle.into(), ..Default::default() }}>
                    <g transform={Transform::translate(-cx, -cy)}>{neuron}</g>
                </g>
            )
        })
        .collect();
    let glow = neon("gallery-relay-glow", 7.0);
    svgr!(
        <g>
            <defs>{glow}</defs>
            {head}
            <g filter="url(#gallery-relay-glow)">{figures}</g>
        </g>
    )
}

pub fn scenes() -> Vec<GalleryScene> {
    let scenes = vec![
        // eight parts half a second apart from 0.6 s, the last name at about 5.1 s
        GalleryScene::new("Cell", 6.5, animal_cell),
        GalleryScene::new("PlantCell", 7.5, plant_cell),
        // 30 pairs of 0.18 s from 0.6 s, then three seconds of unzipping from 6 s
        GalleryScene::new("DnaHelix", 10.0, dna_helix),
        // 18 bases of 0.35 s from 0.6 s, then the polymerase lets go
        GalleryScene::new("Transcription", 8.5, transcription),
        // six codons of 1.2 s from 0.6 s, the last one the stop codon
        GalleryScene::new("Translation", 8.5, translation),
        // six stages of 1.6 s from 0.6 s
        GalleryScene::new("MitosisStages", 11.0, mitosis),
        // the names by 5 s, then two impulses from 5.2 s and 7.2 s
        GalleryScene::new("Neuron", 9.0, neuron),
        // the trace from 0.8 s to 5.8 s, then a pause on it
        GalleryScene::new("ActionPotential", 7.0, action_potential),
        // the curves from 0.8 s to 5.8 s, then a pause on them
        GalleryScene::new("PopulationCurve", 7.0, logistic),
        GalleryScene::new("PredatorPrey", 7.0, predator_prey),
        // six levels of 0.55 s after the stem, then the mammals light up at 5.6 s
        GalleryScene::new("PhyloTree", 7.0, phylo_tree),
        // five levels of 0.8 s from 0.6 s, then energy flows from about 4.6 s
        GalleryScene::new("FoodWeb", 7.5, food_web),
        // the vessels draw from 1 s to 4.2 s, then the blood flows
        GalleryScene::new("Circulation", 8.0, circulation),
        // the substrate arrives from 1.2 s, splits at 5.4 s and the products are gone by 6.8 s
        GalleryScene::new("EnzymeFit", 7.5, induced_fit),
        GalleryScene::new("LockAndKey", 7.5, lock_and_key),
        // the gametes split off by 2.3 s, then the cells fill from 2.4 s to 4.8 s
        GalleryScene::new("PunnettSquare", 6.5, monohybrid),
        GalleryScene::new("DihybridCross", 6.5, dihybrid),
        GalleryScene::new("NeuralRelay", 6.0, neural_relay),
    ];
    // the 3D helix follows the 2D one
    #[cfg(feature = "three-d")]
    let scenes = {
        let mut scenes = scenes;
        // 24 pairs of 0.12 s from 0.6 s, then it keeps turning
        scenes.insert(3, GalleryScene::new("DnaHelix3D", 5.0, dna_helix_3d));
        scenes
    };
    scenes
}
