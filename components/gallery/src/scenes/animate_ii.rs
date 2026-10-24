//! One scene per Manim animation of the second set: creation, indication, movement,
//! transforms, numbers and time.

use fframes_kit::prelude::*;

use super::animate::{CX, CY, square};
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

fn bar(ui: &Ui<'_, '_, '_>, emphasis: Emphasis) -> Rectangle {
    Rectangle {
        cx: CX,
        cy: CY,
        width: 520.0,
        height: 160.0,
        fill: Some(ui.theme.colors.accent),
        emphasis: Some(emphasis),
        ..ui.defaults()
    }
}

/// A 3 by 3 unit square around the origin of `ui.space`, outlined.
fn unit_square(ui: &Ui<'_, '_, '_>) -> Shape {
    let o = ui.space.to_px(0.0, 0.0);
    let side = ui.space.px(3.0) as f32;
    Square {
        cx: o.x as f32,
        cy: o.y as f32,
        side_length: side,
        stroke: Some(ui.theme.colors.highlight),
        stroke_width: 6.0,
        ..ui.defaults()
    }
    .shape()
}

fn wave_homotopy(x: f64, y: f64, t: f64) -> (f64, f64) {
    (x + 0.5 * t * (2.0 * y).sin(), y + 0.3 * t * (3.0 * x).cos())
}

/// A square on the left and a circle on the right, outlined, for the morphs.
fn pair(ui: &Ui<'_, '_, '_>) -> (Shape, Shape) {
    let c = ui.theme.colors;
    let left = Square {
        cx: CX - 300.0,
        cy: CY,
        side_length: 240.0,
        stroke: Some(c.accent),
        stroke_width: 6.0,
        ..ui.defaults()
    };
    let right = Circle {
        cx: CX + 300.0,
        cy: CY,
        radius: 135.0,
        stroke: Some(c.highlight),
        fill: Some(c.highlight),
        stroke_width: 6.0,
        ..ui.defaults()
    };
    (left.shape(), right.shape())
}

/// The glyphs of `text` at `y`, one shape each.
fn glyphs(ui: &Ui<'_, '_, '_>, text: &'static str, x: f32, y: f32) -> Vec<Shape> {
    let t = TextPath {
        text,
        x,
        y,
        stroke: Some(ui.theme.colors.title),
        ..ui.defaults()
    };
    t.parts(ui.ctx).unwrap_or_default()
}

/// Half as high and 1.6 times as wide about the canvas middle.
fn squash(s: Shape) -> Shape {
    let path = paths::translate_path(&paths::scale_path(&s.path, 1.6, 0.5), -576.0, 290.0);
    Shape { path, ..s }
}

fn lissajous(t: f32) -> (f32, f32) {
    (CX + 420.0 * (1.4 * t).sin(), CY + 240.0 * (2.1 * t).sin())
}

#[allow(clippy::too_many_lines)]
pub fn scenes() -> Vec<GalleryScene> {
    scenes![
        "Unwrite", 2.0 => |ui| word(ui, "fframes kit", None, Some(Unwrite { run_time: 1.6, ..ui.defaults() }.into())),
        "ShowPartial", 2.0 => |ui| {
            let partial = ShowPartial { start: (0.5, 0.5), end: (0.0, 1.0), ..ui.defaults() };
            let s = Square { enter: Some(partial.into()), ..square(ui) };
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
        "GrowFromEdge", 2.0 => |ui| {
            let g = GrowFromEdge { edge: Anchor::Bottom, ..ui.defaults() };
            ui.show(Square { enter: Some(g.into()), ..square(ui) })
        },
        "Blink", 2.5 => |ui| {
            let b = Blink { blinks: 2, delay: 0.25, ..ui.defaults() };
            ui.show(Square { emphasis: Some(b.into()), ..square(ui) })
        },
        "FocusOn", 2.0 => |ui| {
            let f = FocusOn { point: Some((CX + 200.0, CY - 200.0)), ..ui.defaults() };
            ui.show(Square { emphasis: Some(f.into()), ..square(ui) })
        },
        "ShowPassingFlash", 2.0 => |ui| {
            let s = square(ui);
            let flash = ShowPassingFlash {
                time_width: 0.25,
                delay: 0.25,
                run_time: 1.5,
                ..ShowPassingFlash::new(Shape { stroke: Some(ui.theme.colors.highlight), stroke_width: 8.0, ..s.shape() })
            };
            let base = ui.show(s);
            let flash = ui.show(flash);
            svgr!(<g>{base}{flash}</g>)
        },
        "ShowPassingFlashWithThinningStrokeWidth", 2.0 => |ui| {
            let c = Circle { cx: CX, cy: CY, radius: 220.0, stroke: Some(ui.theme.colors.highlight), stroke_width: 12.0, ..ui.defaults() };
            let flash = ShowPassingFlashWithThinningStrokeWidth {
                time_width: 0.4,
                delay: 0.25,
                run_time: 1.5,
                ..ShowPassingFlashWithThinningStrokeWidth::new(c.shape())
            };
            ui.show(flash)
        },
        "Broadcast", 3.0 => |ui| ui.show(Broadcast { point: (CX, CY), radius: 360.0, ..ui.defaults() }),
        "AnimatedBoundary", 4.0 => |ui| {
            let c = ui.theme.colors;
            let s = square(ui);
            let boundary = AnimatedBoundary {
                max_stroke_width: 8.0,
                cycle_rate: 1.0,
                ..AnimatedBoundary::new(s.shape(), vec![c.accent, c.highlight, c.title, c.muted])
            };
            let base = ui.show(s);
            let boundary = ui.show(boundary);
            svgr!(<g>{base}{boundary}</g>)
        },
        "ApplyWave", 2.5 => |ui| {
            let t = TextPath { text: "ApplyWave", x: 640.0, y: 640.0, ..ui.defaults() };
            let Some(whole) = t.parts(ui.ctx).and_then(|parts| {
                parts.into_iter().reduce(|mut whole, p| {
                    whole.path.extend(p.path.iter());
                    whole
                })
            }) else {
                return Svgr::empty();
            };
            ui.show(ApplyWave { delay: 0.25, amplitude: 60.0, ..ApplyWave::new(whole) })
        },
        "Rotate", 2.0 => |ui| {
            let r = Rotate { angle: std::f32::consts::FRAC_PI_2, delay: 0.25, ..ui.defaults() };
            ui.show(bar(ui, r.into()))
        },
        "Rotating", 3.0 => |ui| {
            let r = Rotating { run_time: 2.0, forever: true, ..ui.defaults() };
            ui.show(bar(ui, r.into()))
        },
        "Homotopy", 2.0 => |ui| {
            let h = Homotopy { delay: 0.25, run_time: 1.5, ..Homotopy::new(unit_square(ui), wave_homotopy) };
            ui.show(h)
        },
        "SmoothedVectorizedHomotopy", 2.0 => |ui| {
            let h = Homotopy { delay: 0.25, run_time: 1.5, ..Homotopy::new(unit_square(ui), wave_homotopy) };
            ui.show(SmoothedVectorizedHomotopy { homotopy: h })
        },
        "ComplexHomotopy", 2.0 => |ui| {
            // from z to z squared, scaled down to stay on the canvas
            let h = ComplexHomotopy {
                delay: 0.25,
                run_time: 1.5,
                ..ComplexHomotopy::new(unit_square(ui), |(re, im), t| {
                    let (sq_re, sq_im) = ((re * re - im * im) / 2.0, re * im);
                    (re + (sq_re - re) * t, im + (sq_im - im) * t)
                })
            };
            ui.show(h)
        },
        "PhaseFlow", 2.0 => |ui| {
            let o = ui.space.to_px(1.5, 0.0);
            let dot = Circle { cx: o.x as f32, cy: o.y as f32, radius: 70.0, fill: Some(ui.theme.colors.accent), ..ui.defaults() };
            let flow = PhaseFlow {
                virtual_time: std::f32::consts::PI,
                delay: 0.25,
                run_time: 1.5,
                ..PhaseFlow::new(dot.shape(), |x, y| (-y, x))
            };
            ui.show(flow)
        },
        "TracedPath", 3.0 => |ui| {
            let trace = ui.show(TracedPath { point: &lissajous, stroke_width: 6.0, ..ui.defaults() });
            let (x, y) = lissajous(ui.seconds());
            let dot = ui.show(Dot { cx: x, cy: y, radius: 20.0, ..ui.defaults() });
            svgr!(<g>{trace}{dot}</g>)
        },
        "TransformFromCopy", 2.0 => |ui| {
            let (a, b) = pair(ui);
            ui.show(TransformFromCopy { delay: 0.25, run_time: 1.5, ..TransformFromCopy::new(a, b) })
        },
        "ClockwiseTransform", 2.0 => |ui| {
            let (a, b) = pair(ui);
            ui.show(ClockwiseTransform { delay: 0.25, run_time: 1.5, ..ClockwiseTransform::new(a, b) })
        },
        "CounterclockwiseTransform", 2.0 => |ui| {
            let (a, b) = pair(ui);
            ui.show(CounterclockwiseTransform { delay: 0.25, run_time: 1.5, ..CounterclockwiseTransform::new(a, b) })
        },
        "Swap", 2.0 => |ui| {
            let (a, b) = pair(ui);
            ui.show(Swap { delay: 0.25, run_time: 1.5, ..Swap::new(a, b) })
        },
        "CyclicReplace", 2.0 => |ui| {
            let c = ui.theme.colors;
            let shapes = (0..4)
                .map(|i| {
                    let x = CX - 450.0 + 300.0 * i as f32;
                    let fill = Some(if i % 2 == 0 { c.accent } else { c.highlight });
                    Square { cx: x, cy: CY, side_length: 80.0 + 40.0 * i as f32, fill, ..ui.defaults() }.shape()
                })
                .collect();
            ui.show(CyclicReplace { delay: 0.25, run_time: 1.5, ..CyclicReplace::new(shapes) })
        },
        "FadeTransform", 2.0 => |ui| {
            let (a, b) = pair(ui);
            ui.show(FadeTransform { delay: 0.25, run_time: 1.5, ..FadeTransform::new(a, b) })
        },
        "FadeTransformPieces", 2.0 => |ui| {
            let from = glyphs(ui, "pieces", 700.0, 500.0);
            let to = glyphs(ui, "fade in", 640.0, 760.0);
            ui.show(FadeTransformPieces { delay: 0.25, run_time: 1.5, ..FadeTransformPieces::new(from, to) })
        },
        "TransformMatchingShapes", 2.0 => |ui| {
            let from = glyphs(ui, "the morse code", 480.0, 500.0);
            let to = glyphs(ui, "here come dots", 480.0, 760.0);
            ui.show(TransformMatchingShapes { delay: 0.25, run_time: 1.5, ..TransformMatchingShapes::new(from, to) })
        },
        "TransformAnimations", 2.0 => |ui| {
            let (a, b) = pair(ui);
            let create: EnterAnim = Create { run_time: 1.5, delay: 0.25, ..ui.defaults() }.into();
            let grow: EnterAnim = GrowFromCenter { run_time: 1.5, delay: 0.25, ..ui.defaults() }.into();
            ui.show(TransformAnimations { delay: 0.25, run_time: 1.5, ..TransformAnimations::new(a, create, b, grow) })
        },
        "FadeToColor", 2.0 => |ui| {
            let f = FadeToColor { delay: 0.25, run_time: 1.5, ..ui.defaults() };
            ui.show(bar(ui, f.into()))
        },
        "ScaleInPlace", 2.0 => |ui| {
            let s = ScaleInPlace { scale_factor: 1.5, delay: 0.25, run_time: 1.5, ..ui.defaults() };
            ui.show(bar(ui, s.into()))
        },
        "ShrinkToCenter", 2.0 => |ui| {
            let s = ShrinkToCenter { delay: 0.25, run_time: 1.5, ..ui.defaults() };
            ui.show(Square { exit: Some(s.into()), ..square(ui) })
        },
        "ApplyMatrix", 2.0 => |ui| {
            let m = ApplyMatrix { delay: 0.25, run_time: 1.5, ..ApplyMatrix::new(unit_square(ui), [[1.0, 1.0], [0.0, 1.0]]) };
            ui.show(m)
        },
        "ApplyPointwiseFunction", 2.0 => |ui| {
            let f = ApplyPointwiseFunction {
                delay: 0.25,
                run_time: 1.5,
                ..ApplyPointwiseFunction::new(unit_square(ui), |x, y| (x + 0.3 * y.sin() * 2.0, y + 0.2 * (2.0 * x).sin()))
            };
            ui.show(f)
        },
        "ApplyComplexFunction", 2.0 => |ui| {
            // multiplication by i: a quarter turn on arcs
            let f = ApplyComplexFunction { delay: 0.25, run_time: 1.5, ..ApplyComplexFunction::new(unit_square(ui), |(re, im)| (-im, re)) };
            ui.show(f)
        },
        "ApplyFunction", 2.0 => |ui| {
            let s = square(ui).shape();
            ui.show(ApplyFunction { delay: 0.25, run_time: 1.5, ..ApplyFunction::new(s, squash) })
        },
        "ApplyPointwiseFunctionToCenter", 2.0 => |ui| {
            let (a, _) = pair(ui);
            let f = ApplyPointwiseFunctionToCenter { delay: 0.25, run_time: 1.5, ..ApplyPointwiseFunctionToCenter::new(a, |x, y| (-x, y + 1.0)) };
            ui.show(f)
        },
        "ChangeDecimalToValue", 2.0 => |ui| {
            let n = DecimalNumber { x: CX, y: CY + 40.0, anchor: TextAnchor::Middle, value: 0.0.into(), ..ui.defaults() };
            ui.show(ChangeDecimalToValue { delay: 0.25, run_time: 1.5, ..ChangeDecimalToValue::new(n, 1000.0) })
        },
        "ChangingDecimal", 2.0 => |ui| {
            let n = DecimalNumber { x: CX, y: CY + 40.0, anchor: TextAnchor::Middle, num_decimal_places: 4, ..ui.defaults() };
            ui.show(ChangingDecimal { delay: 0.25, run_time: 1.5, ..ChangingDecimal::new(n, |a| (a * std::f32::consts::PI).sin()) })
        },
        "ChangeSpeed", 2.5 => |ui| {
            // the dot crosses at an even pace inside; the speed curve starts it slow and ends it fast
            let speed = ChangeSpeed {
                run_time: 2.0,
                speed: vec![(0.0, 0.2), (1.0, 2.0)],
                children: children(|ui| {
                    let x = CX - 600.0 + 1200.0 * (ui.seconds() / 2.0).clamp(0.0, 1.0);
                    let trail = Line { x1: CX - 600.0, y1: CY, x2: x, y2: CY, ..ui.defaults() };
                    let dot = Dot { cx: x, cy: CY, radius: 30.0, fill: Some(ui.theme.colors.accent), ..ui.defaults() };
                    let (trail, dot) = (ui.show(trail), ui.show(dot));
                    svgr!(<g>{trail}{dot}</g>)
                }),
            };
            ui.show(speed)
        },
        "LaggedStartMap", 2.0 => |ui| {
            let lagged = LaggedStart {
                lag_ratio: 0.15,
                ..LaggedStart::map(0..7, |ui, &i| {
                    let grow = GrowFromCenter { run_time: 0.8, ..ui.defaults() };
                    ui.show(Dot { cx: CX - 480.0 + 160.0 * i as f32, cy: CY, radius: 48.0, enter: Some(grow.into()), ..ui.defaults() })
                })
            };
            ui.show(lagged)
        },
    ]
}
