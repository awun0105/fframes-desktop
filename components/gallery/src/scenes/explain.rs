//! The explain layer: number lines, axes, a number plane and Manim's two graph examples,
//! `SinAndCosFunctionPlot` and `GraphAreaPlot`, drawn with `Create`.

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

/// `Create` over `run_time` seconds, for long curves that would otherwise draw hundreds of
/// pixels a frame.
fn trace(delay: f32, run_time: f32) -> EnterAnim {
    Create {
        delay,
        run_time,
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

fn integers(from: i32, to: i32, step: usize) -> Vec<f64> {
    (from..=to).step_by(step).map(f64::from).collect()
}

fn number_line<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "NumberLine");
    let line = NumberLine {
        range: Range::new(-5.0, 5.0, 1.0),
        length: 11.0,
        center: (0.0, -0.3),
        tips: true,
        numbers: integers(-5, 5, 1),
        enter: Some(create(0.0)),
        numbers_enter: Some(fade(0.6)),
        ..ui.defaults()
    };
    // a dot that walks from -3 to 4
    let t = motion::progress(ui.seconds(), 1.2, 2.6);
    let (cx, cy) = line.n2p(ui, f64::from(motion::lerp(-3.0, 4.0, t)));
    let line = ui.show(line);
    let dot = ui.show(Dot {
        cx,
        cy,
        radius: 14.0,
        fill: Some(ui.theme.colors.highlight),
        enter: Some(
            GrowFromCenter {
                delay: 1.0,
                ..ui.defaults()
            }
            .into(),
        ),
        ..ui.defaults()
    });
    svgr!(<g>{head}{line}{dot}</g>)
}

fn number_plane<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let plane = NumberPlane {
        faded_line_ratio: 2,
        enter: Some(create(0.0)),
        axes: Axes {
            enter: Some(create(0.4)),
            ..NumberPlane::themed(ui.theme).axes
        },
        ..ui.defaults()
    };
    let map = plane.axes.map(ui);
    let plane = ui.show(plane);
    let (x2, y2) = {
        let p = map.c2p(3.0, 2.0);
        (p.x as f32, p.y as f32)
    };
    let origin = map.c2p(0.0, 0.0);
    let vector = ui.show(Arrow {
        x1: origin.x as f32,
        y1: origin.y as f32,
        x2,
        y2,
        tip_length: 32.0,
        tip_width: 28.0,
        stroke: Some(ui.theme.colors.highlight),
        fill: Some(ui.theme.colors.highlight),
        stroke_width: ui.theme.stroke.thick,
        enter: Some(
            GrowArrow {
                delay: 1.2,
                ..ui.defaults()
            }
            .into(),
        ),
        ..ui.defaults()
    });
    let label = ui.show(Label {
        text: "(3, 2)".into(),
        x: x2,
        y: y2,
        direction: Anchor::TopRight,
        enter: Some(fade(1.8)),
        ..ui.defaults()
    });
    let head = title(ui, "NumberPlane");
    svgr!(<g>{plane}{vector}{label}{head}</g>)
}

/// Manim's `SinAndCosFunctionPlot`: sine and cosine on axes from -10 to 10, a vertical line
/// at 2 pi and the graph labels.
fn sin_and_cos<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "SinAndCosFunctionPlot");
    let axes = Axes {
        x_range: Range::new(-10.0, 10.3, 1.0),
        y_range: Range::new(-1.5, 1.5, 1.0),
        x_length: 10.0,
        y_length: 4.6,
        center: (0.0, -0.5),
        color: ui.theme.colors.muted,
        tips: false,
        x_numbers: integers(-10, 10, 2),
        x_label: Some("x".into()),
        y_label: Some("y".into()),
        enter: Some(create(0.0)),
        labels_enter: Some(fade(0.5)),
        ..ui.defaults()
    };
    let sin = FunctionGraph {
        enter: Some(trace(0.6, 2.2)),
        ..axes.plot(ui, f64::sin)
    };
    let cos = FunctionGraph {
        stroke: Some(ui.theme.colors.highlight),
        enter: Some(trace(0.9, 2.2)),
        ..axes.plot(ui, f64::cos)
    };
    let tau = std::f64::consts::TAU;
    let vertical = DashedLine {
        dash: None,
        stroke: Some(ui.theme.colors.title),
        enter: Some(create(3.0)),
        ..axes.vertical_line(ui, tau, tau.cos())
    };
    let sin_label = Label {
        enter: Some(fade(2.6)),
        ..axes.graph_label(ui, &sin, "sin(x)", -10.0, Anchor::Top)
    };
    let cos_label = Label {
        enter: Some(fade(2.9)),
        ..axes.graph_label(ui, &cos, "cos(x)", 10.3, Anchor::Right)
    };
    let line_label = Label {
        fill: ui.theme.colors.title,
        enter: Some(fade(3.4)),
        ..axes.graph_label(ui, &cos, "x = 2π", tau, Anchor::TopRight)
    };
    let axes = ui.show(axes);
    let sin = ui.show(sin);
    let cos = ui.show(cos);
    let vertical = ui.show(vertical);
    let labels = ui.show(vec![sin_label, cos_label, line_label]);
    svgr!(<g>{head}{axes}{sin}{cos}{vertical}{labels}</g>)
}

fn curve_1(x: f64) -> f64 {
    4.0 * x - x * x
}

fn curve_2(x: f64) -> f64 {
    0.8 * x * x - 3.0 * x + 4.0
}

/// Manim's `GraphAreaPlot`: two curves, vertical lines at 2 and 3, Riemann rectangles under
/// the first curve and the area between the curves.
fn graph_area<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "GraphAreaPlot");
    let axes = Axes {
        x_range: Range::new(0.0, 5.0, 1.0),
        y_range: Range::new(0.0, 6.0, 1.0),
        x_length: 10.0,
        y_length: 5.4,
        center: (0.0, -0.6),
        tips: false,
        x_numbers: vec![2.0, 3.0],
        x_label: Some("x".into()),
        y_label: Some("y".into()),
        enter: Some(create(0.0)),
        labels_enter: Some(fade(0.5)),
        ..ui.defaults()
    };
    let c1 = FunctionGraph {
        x_range: (0.0, 4.0),
        enter: Some(create(0.6)),
        ..axes.plot(ui, curve_1)
    };
    let c2 = FunctionGraph {
        x_range: (0.0, 4.0),
        stroke: Some(ui.theme.colors.highlight),
        enter: Some(create(0.8)),
        ..axes.plot(ui, curve_2)
    };
    let line_1 = DashedLine {
        enter: Some(create(1.5)),
        ..axes.vertical_line(ui, 2.0, curve_1(2.0))
    };
    let line_2 = DashedLine {
        enter: Some(create(1.6)),
        ..axes.vertical_line(ui, 3.0, curve_1(3.0))
    };
    let riemann = RiemannRectangles {
        enter: Some(
            GrowFromCenter {
                delay: 1.8,
                ..ui.defaults()
            }
            .into(),
        ),
        ..axes.riemann_rectangles(ui, curve_1, (0.3, 0.6), 0.03)
    };
    let area = Area {
        bounded: Some(curve_1),
        enter: Some(fade(2.0).into()),
        ..axes.area(ui, curve_2, (2.0, 3.0))
    };
    let axes = ui.show(axes);
    let area = ui.show(area);
    let riemann = ui.show(riemann);
    let c1 = ui.show(c1);
    let c2 = ui.show(c2);
    let lines = ui.show(vec![line_1, line_2]);
    svgr!(<g>{head}{axes}{area}{riemann}{c1}{c2}{lines}</g>)
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("NumberLine", 3.0, number_line),
        GalleryScene::new("NumberPlane", 3.0, number_plane),
        GalleryScene::new("SinAndCosFunctionPlot", 4.5, sin_and_cos),
        GalleryScene::new("GraphAreaPlot", 3.5, graph_area),
    ]
}
