//! The biology kit's figures, in the order of the plan's table.

use fframes_kit::prelude::*;

use super::core::title;
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

pub fn scenes() -> Vec<GalleryScene> {
    vec![
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
    ]
}
