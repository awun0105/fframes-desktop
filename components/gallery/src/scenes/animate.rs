//! One scene per geometry animation and timing group, and Manim's `SquareToCircle`.

use fframes_kit::prelude::*;

use super::core::title;
use crate::GalleryScene;

const CX: f32 = 960.0;
const CY: f32 = 580.0;

fn square(ui: &Ui<'_, '_, '_>) -> Square {
    Square {
        cx: CX,
        cy: CY,
        side_length: 400.0,
        ..ui.defaults()
    }
}

fn circle(ui: &Ui<'_, '_, '_>) -> Circle {
    let c = ui.theme.colors;
    Circle {
        cx: CX,
        cy: CY,
        radius: 220.0,
        stroke: Some(c.highlight),
        fill: Some(c.highlight),
        opacity: 0.8,
        ..ui.defaults()
    }
}

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

fn on_square<'a>(
    ui: &mut Ui<'a, '_, '_>,
    enter: Option<EnterAnim>,
    emphasis: Option<Emphasis>,
) -> Svgr<'a> {
    let s = Square {
        enter,
        emphasis,
        ..square(ui)
    };
    ui.show(s)
}

fn word<'a>(ui: &mut Ui<'a, '_, '_>, text: &'static str, enter: EnterAnim) -> Svgr<'a> {
    let t = TextPath {
        text,
        x: 560.0,
        y: 640.0,
        stroke: Some(ui.theme.colors.title),
        enter: Some(enter),
        ..ui.defaults()
    };
    ui.show(t)
}

fn dots<'a>() -> Vec<Children<'a>> {
    (0..5)
        .map(|i| {
            children(move |ui| {
                let dot = Circle {
                    cx: 560.0 + 200.0 * i as f32,
                    cy: CY,
                    radius: 60.0,
                    enter: Some(
                        GrowFromCenter {
                            run_time: 0.8,
                            ..ui.defaults()
                        }
                        .into(),
                    ),
                    ..ui.defaults()
                };
                ui.show(dot)
            })
        })
        .collect()
}

#[allow(clippy::too_many_lines)]
pub fn scenes() -> Vec<GalleryScene> {
    scenes![
        "Create", 2.0 => |ui| on_square(ui, Some(Create::default().into()), None),
        "Uncreate", 2.0 => |ui| {
            let s = Square { exit: Some(Uncreate::default().into()), ..square(ui) };
            ui.show(s)
        },
        "DrawBorderThenFill", 2.5 => |ui| {
            let s = Square {
                fill: Some(ui.theme.colors.accent),
                enter: Some(DrawBorderThenFill::default().into()),
                ..square(ui)
            };
            ui.show(s)
        },
        "Write", 2.0 => |ui| word(ui, "fframes kit", Write::default().into()),
        "TextPath", 2.0 => |ui| word(ui, "Text as paths", Create { run_time: 1.5, ..ui.defaults() }.into()),
        "GrowFromCenter", 2.0 => |ui| on_square(ui, Some(GrowFromCenter::default().into()), None),
        "GrowFromPoint", 2.0 => |ui| {
            let g = GrowFromPoint { point: (200.0, 900.0), ..ui.defaults() };
            on_square(ui, Some(g.into()), None)
        },
        "GrowArrow", 2.0 => |ui| {
            let a = Arrow {
                x1: 560.0, y1: CY, x2: 1360.0, y2: CY, tip_length: 48.0, tip_width: 40.0,
                enter: Some(GrowArrow::default().into()),
                ..ui.defaults()
            };
            ui.show(a)
        },
        "SpinInFromNothing", 2.0 => |ui| on_square(ui, Some(SpinInFromNothing::default().into()), None),
        "Indicate", 2.0 => |ui| on_square(ui, None, Some(Indicate { delay: 0.5, ..ui.defaults() }.into())),
        "Circumscribe", 2.0 => |ui| on_square(ui, None, Some(Circumscribe { delay: 0.5, ..ui.defaults() }.into())),
        "Flash", 2.0 => |ui| {
            let flash = Flash { delay: 0.5, flash_radius: 260.0, line_length: 80.0, ..ui.defaults() };
            on_square(ui, None, Some(flash.into()))
        },
        "Wiggle", 2.5 => |ui| on_square(ui, None, Some(Wiggle { delay: 0.25, ..ui.defaults() }.into())),
        "Transform", 2.0 => |ui| {
            let morph = Morph { delay: 0.5, ..Morph::new(square(ui).shape(), circle(ui).shape()) };
            ui.show(morph)
        },
        "ReplacementTransform", 2.0 => |ui| {
            let morph = ReplacementTransform {
                delay: 0.5,
                ..ReplacementTransform::new(circle(ui).shape(), square(ui).shape())
            };
            ui.show(morph)
        },
        "MoveAlongPath", 2.0 => |ui| {
            let track = Arc {
                cx: CX, cy: CY + 200.0, radius: 400.0, start_angle: 0.0, angle: std::f32::consts::PI,
                stroke: Some(ui.theme.colors.muted),
                ..ui.defaults()
            };
            let path = paths::reverse_path(&track.path());
            let dot = Dot { cx: CX - 400.0, cy: CY + 200.0, radius: 32.0, ..ui.defaults() };
            let track = ui.show(track);
            let dot = ui.show(dot);
            let moving = MoveAlongPath { delay: 0.25, run_time: 1.5, ..MoveAlongPath::new(path) }.apply(ui, dot);
            svgr!(<g>{track}{moving}</g>)
        },
        "LaggedStart", 2.0 => |ui| ui.show(LaggedStart { lag_ratio: 0.25, run_time: 0.8, children: dots() }),
        "AnimationGroup", 2.0 => |ui| ui.show(AnimationGroup { children: dots() }),
        "Succession", 4.0 => |ui| ui.show(Succession { run_time: 0.8, children: dots() }),
        "SquareToCircle", 4.0 => |ui| {
            let create = ui.show(Sequence {
                from: 0.0,
                duration: Some(1.2),
                children: children(|ui| on_square(ui, Some(Create::default().into()), None)),
            });
            let morph = ui.show(Sequence {
                from: 1.2,
                duration: None,
                children: children(|ui| {
                    let morph = ui.show(Morph::new(square(ui).shape(), circle(ui).shape()));
                    FadeOut { run_time: 0.8, ..ui.defaults() }.apply(ui, morph)
                }),
            });
            svgr!(<g>{create}{morph}</g>)
        },
    ]
}
