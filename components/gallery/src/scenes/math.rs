//! The math kit's figures, in the order of the plan's table: a point and an interval on a
//! number line, a plot with a tracer, a tangent and a secant, a Riemann sum and the area
//! under a graph.

use std::sync::OnceLock;

use fframes::animation::KeyFramesAnimation;
use fframes_kit::prelude::*;

use super::core::title;
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

fn parabola(x: f64) -> f64 {
    0.4 * x * x
}

/// A point that walks along a number line, over the interval `[-1, 2)`.
fn number_line_point<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static X: OnceLock<KeyFramesAnimation<f32>> = OnceLock::new();
    let x = X.get_or_init(|| {
        timeline!(
            at 1.2 => 2.6, animate -3.0_f32 => 2.5, Easing::EaseInOut,
            at 2.8 => 3.7, animate 2.5_f32 => -0.5, Easing::EaseInOut,
        )
    });
    let head = title(ui, "NumberLinePoint");
    let line = NumberLine {
        range: Range::new(-4.0, 4.0, 1.0),
        length: 12.0,
        numbers: (-4..=4).map(f64::from).collect(),
        enter: Some(create(0.0)),
        numbers_enter: Some(fade(0.3)),
        ..ui.defaults()
    };
    let interval = ui.show(Interval {
        line: line.clone(),
        from: -1.0,
        to: 2.0,
        closed: (true, false),
        enter: Some(create(0.5)),
        ..ui.defaults()
    });
    let point = ui.show(NumberPoint {
        line: line.clone(),
        value: x.into(),
        label: "x = ".into(),
        decimals: Some(1),
        enter: Some(fade(0.8)),
        ..ui.defaults()
    });
    let line = ui.show(line);
    svgr!(<g>{head}{line}{interval}{point}</g>)
}

/// A parabola with a tracer that reads out its coordinates.
fn function_plot<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static X: OnceLock<KeyFramesAnimation<f32>> = OnceLock::new();
    let x = X.get_or_init(|| {
        timeline!(
            at 1.3 => 2.8, animate -2.5_f32 => 2.0, Easing::EaseInOut,
            at 3.0 => 3.8, animate 2.0_f32 => 1.0, Easing::EaseInOut,
        )
    });
    let head = title(ui, "FunctionPlot");
    let axes = Axes {
        x_range: Range::new(-3.0, 3.0, 1.0),
        y_range: Range::new(0.0, 4.0, 1.0),
        x_length: 9.0,
        y_length: 5.0,
        center: (0.0, -0.4),
        tips: false,
        enter: Some(create(0.0)),
        ..ui.defaults()
    };
    let graph = FunctionGraph {
        enter: Some(
            Create {
                delay: 0.3,
                run_time: 0.8,
                ..Create::default()
            }
            .into(),
        ),
        ..axes.plot(ui, parabola)
    };
    let plot = ui.show(FunctionPlot {
        graph,
        axes,
        label: "0.4 x²".into(),
        label_at: -2.6,
        label_direction: Anchor::TopLeft,
        label_enter: Some(fade(0.8)),
        tracer: Some(x.into()),
        tracer_enter: Some(fade(1.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{plot}</g>)
}

/// A secant whose `dx` shrinks until it is the tangent, next to a tangent that slides.
fn tangent_secant<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static DX: OnceLock<KeyFramesAnimation<f32>> = OnceLock::new();
    static AT: OnceLock<KeyFramesAnimation<f32>> = OnceLock::new();
    let dx = DX.get_or_init(|| timeline!(at 1.2 => 3.8, animate 1.5_f32 => 0.0, Easing::EaseInOut));
    let at =
        AT.get_or_init(|| timeline!(at 1.0 => 3.8, animate -2.4_f32 => -0.6, Easing::EaseInOut));
    let head = title(ui, "TangentSecant");
    let axes = Axes {
        x_range: Range::new(-3.0, 3.0, 1.0),
        y_range: Range::new(0.0, 4.0, 1.0),
        x_length: 9.0,
        y_length: 5.0,
        center: (0.0, -0.4),
        tips: false,
        enter: Some(create(0.0)),
        ..ui.defaults()
    };
    let map = axes.map(ui);
    let graph = FunctionGraph {
        enter: Some(create(0.3)),
        ..axes.plot(ui, parabola)
    };
    let secant = ui.show(Secant {
        f: parabola,
        axes: map,
        x: 1.0.into(),
        dx: dx.into(),
        enter: Some(create(0.8)),
        ..ui.defaults()
    });
    let tangent = ui.show(Tangent {
        f: parabola,
        axes: map,
        x: at.into(),
        color: ui.theme.colors.accent,
        enter: Some(create(0.8)),
        ..ui.defaults()
    });
    let axes = ui.show(axes);
    let graph = ui.show(graph);
    svgr!(<g>{head}{axes}{graph}{secant}{tangent}</g>)
}

fn riemann_axes<'a>(ui: &Ui<'a, '_, '_>) -> Axes<'a> {
    Axes {
        x_range: Range::new(0.0, 3.0, 1.0),
        y_range: Range::new(0.0, 4.0, 1.0),
        x_length: 8.0,
        y_length: 5.0,
        center: (-1.5, -0.4),
        tips: false,
        enter: Some(create(0.0)),
        ..ui.defaults()
    }
}

/// Riemann rectangles under the parabola getting finer, with their sum.
fn riemann_sum<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "RiemannSum");
    let axes = riemann_axes(ui);
    let graph = FunctionGraph {
        enter: Some(create(0.3)),
        ..axes.plot(ui, parabola)
    };
    let sum = ui.show(RiemannSum {
        rects: RiemannRectangles {
            enter: Some(EnterAnim::FadeIn(fade(0.6))),
            ..axes.riemann_rectangles(ui, parabola, (0.0, 3.0), 0.5)
        },
        dxs: vec![0.5, 0.25, 0.1, 0.05],
        delay: 0.6,
        hold: 0.8,
        crossfade: 0.4,
        sum: Some(Label {
            text: "sum = ".into(),
            x: 1380.0,
            y: 420.0,
            enter: Some(fade(0.6)),
            ..ui.defaults()
        }),
        ..ui.defaults()
    });
    let exact = ui.show(Label {
        text: "exact 3.600".into(),
        x: 1380.0,
        y: 500.0,
        fill: ui.theme.colors.muted,
        enter: Some(fade(0.6)),
        ..ui.defaults()
    });
    let axes = ui.show(axes);
    let graph = ui.show(graph);
    svgr!(<g>{head}{axes}{sum}{graph}{exact}</g>)
}

/// The area under the parabola from 1 to 3, revealed left to right with its integral.
fn area_under<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "AreaUnder");
    let axes = riemann_axes(ui);
    let graph = FunctionGraph {
        enter: Some(create(0.3)),
        ..axes.plot(ui, parabola)
    };
    let area = ui.show(AreaUnder {
        area: axes.area(ui, parabola, (1.0, 3.0)),
        delay: 0.8,
        run_time: 1.8,
        value: Some(Label {
            text: "area = ".into(),
            x: 1380.0,
            y: 420.0,
            enter: Some(fade(0.8)),
            ..ui.defaults()
        }),
        ..ui.defaults()
    });
    let axes = ui.show(axes);
    let graph = ui.show(graph);
    svgr!(<g>{head}{axes}{area}{graph}</g>)
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("NumberLinePoint", 4.0, number_line_point),
        GalleryScene::new("FunctionPlot", 4.0, function_plot),
        GalleryScene::new("TangentSecant", 4.5, tangent_secant),
        GalleryScene::new("RiemannSum", 4.5, riemann_sum),
        GalleryScene::new("AreaUnder", 3.5, area_under),
    ]
}
