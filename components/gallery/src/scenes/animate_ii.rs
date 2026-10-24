//! One scene per Manim animation of the second set: creation, indication, movement,
//! transforms, numbers and time.

use fframes_kit::prelude::*;

use super::animate::{CX, CY};
use super::core::title;
use crate::GalleryScene;

/// The scene with `title`, and the content one second in, as Manim's `wait` would.
macro_rules! scenes {
    ($($name:literal, $seconds:expr => |$ui:ident| $body:expr,)*) => {
        vec![$(GalleryScene::new($name, $seconds, |$ui| {
            let head = title($ui, $name);
            let body: Svgr<'_> = $body;
            svgr!(<g>{head}{body}</g>)
        }),)*]
    };
}

fn word<'a>(
    ui: &mut Ui<'a, '_, '_>,
    text: &'static str,
    enter: Option<EnterAnim>,
    exit: Option<ExitAnim>,
) -> Svgr<'a> {
    let t = TextPath {
        text,
        x: 560.0,
        y: 640.0,
        stroke: Some(ui.theme.colors.title),
        enter,
        exit,
        ..ui.defaults()
    };
    ui.show(t)
}

/// A row of seven dots, one part each, drawn as one shape list.
fn dot_row<'a>(ui: &Ui<'a, '_, '_>, enter: EnterAnim) -> Svgr<'a> {
    let c = ui.theme.colors;
    let parts: Vec<Shape> = (0..7)
        .map(|i| {
            Dot {
                cx: CX - 480.0 + 160.0 * i as f32,
                cy: CY,
                radius: 48.0,
                fill: Some(if i % 2 == 0 { c.accent } else { c.highlight }),
                ..ui.defaults()
            }
            .shape()
        })
        .collect();
    fframes_kit::animate::draw_animated(ui, &parts, Some(&enter), None, None)
}

/// Eight dots on a ring and one in the middle, for `SpiralIn`.
fn dot_cluster<'a>(ui: &Ui<'a, '_, '_>, enter: EnterAnim) -> Svgr<'a> {
    let c = ui.theme.colors;
    let parts: Vec<Shape> = (0..9)
        .map(|i| {
            let angle = std::f32::consts::TAU * i as f32 / 8.0;
            let r = if i == 8 { 0.0 } else { 150.0 };
            Dot {
                cx: CX + r * angle.cos(),
                cy: CY + r * angle.sin(),
                radius: 40.0,
                fill: Some(if i % 2 == 0 { c.accent } else { c.highlight }),
                ..ui.defaults()
            }
            .shape()
        })
        .collect();
    fframes_kit::animate::draw_animated(ui, &parts, Some(&enter), None, None)
}

#[allow(clippy::too_many_lines)]
pub fn scenes() -> Vec<GalleryScene> {
    scenes![
        "Unwrite", 2.0 => |ui| word(ui, "fframes kit", None, Some(Unwrite { run_time: 1.6, ..ui.defaults() }.into())),
        "ShowPartial", 2.0 => |ui| {
            let partial = ShowPartial { start: (0.5, 0.5), end: (0.0, 1.0), ..ui.defaults() };
            let s = Square { enter: Some(partial.into()), ..super::animate::square(ui) };
            ui.show(s)
        },
        "ShowIncreasingSubsets", 2.0 => |ui| dot_row(ui, ShowIncreasingSubsets { run_time: 1.5, ..ui.defaults() }.into()),
        "ShowSubmobjectsOneByOne", 2.0 => |ui| dot_row(ui, ShowSubmobjectsOneByOne { run_time: 1.5, ..ui.defaults() }.into()),
        "SpiralIn", 2.0 => |ui| dot_cluster(ui, SpiralIn { scale_factor: 2.0, run_time: 1.5, ..ui.defaults() }.into()),
        "AddTextLetterByLetter", 2.0 => |ui| word(ui, "letter by letter", Some(AddTextLetterByLetter::default().into()), None),
        "RemoveTextLetterByLetter", 2.0 => |ui| word(ui, "letter by letter", None, Some(RemoveTextLetterByLetter::default().into())),
        "TypeWithCursor", 3.0 => |ui| {
            let t = Typewriter {
                text: "typed, then untyped".into(),
                x: 560.0,
                y: 600.0,
                style: ui.theme.typography.title,
                chars_per_second: 16.0,
                erase: Some(0.25),
                ..ui.defaults()
            };
            ui.show(t)
        },
    ]
}
