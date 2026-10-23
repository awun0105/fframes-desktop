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

fn lissajous(t: f64) -> (f64, f64) {
    (2.0 * (3.0 * t).sin(), 2.0 * (2.0 * t).sin())
}

/// A box of `space` the same size as the canvas, so the two coordinate systems show.
fn frame_of<'a>(ui: &mut Ui<'a, '_, '_>, x: f32, y: f32, w: f32, h: f32) -> Svgr<'a> {
    ui.show(RoundedRectangle {
        cx: x + w / 2.0,
        cy: y + h / 2.0,
        width: w,
        height: h,
        corner_radius: 24.0,
        stroke: Some(ui.theme.colors.surface),
        ..ui.defaults()
    })
}

/// Two canvases side by side: a Lissajous curve in units of the left one, axes in the right
/// one. Each has its own origin and 130 px per unit.
fn canvas<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Canvas");
    let (y, w, h) = (220.0, 820.0, 780.0);
    let left_box = frame_of(ui, 96.0, y, w, h);
    let right_box = frame_of(ui, 1004.0, y, w, h);
    let left = ui.show(Canvas {
        space: Space::frame(96.0, y, w, h, 6.0),
        children: children(|ui| {
            let curve = ParametricFunction {
                enter: Some(trace(0.0, 2.4)),
                ..ParametricFunction::new(ui, lissajous, (0.0, std::f64::consts::TAU))
            };
            let (cx, cy) = curve.point(0.0);
            let origin = ui.show(Dot {
                cx,
                cy,
                radius: 10.0,
                fill: Some(ui.theme.colors.highlight),
                ..ui.defaults()
            });
            let curve = ui.show(curve);
            svgr!(<g>{curve}{origin}</g>)
        }),
    });
    let right = ui.show(Canvas {
        space: Space::frame(1004.0, y, w, h, 6.0),
        children: children(|ui| {
            let axes = Axes {
                x_range: Range::new(-3.0, 3.0, 1.0),
                y_range: Range::new(-2.0, 2.0, 1.0),
                x_length: 5.6,
                y_length: 4.0,
                x_numbers: integers(-3, 3, 1),
                enter: Some(create(0.0)),
                labels_enter: Some(fade(0.4)),
                ..ui.defaults()
            };
            let graph = FunctionGraph {
                enter: Some(trace(0.6, 1.6)),
                ..axes.plot(ui, |x| 1.5 * (2.0 * x).sin() / (1.0 + x * x))
            };
            let axes = ui.show(axes);
            let graph = ui.show(graph);
            svgr!(<g>{axes}{graph}</g>)
        }),
    });
    svgr!(<g>{head}{left_box}{right_box}{left}{right}</g>)
}

/// Five shapes arranged in a row and six in a grid, each group drawn with one `Create`.
fn vgroup<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "VGroup");
    let row = VGroup::new(vec![
        Circle {
            radius: 70.0,
            ..ui.defaults()
        }
        .shape(),
        Square {
            side_length: 130.0,
            ..ui.defaults()
        }
        .shape(),
        Triangle {
            radius: 80.0,
            ..ui.defaults()
        }
        .shape(),
        Star {
            outer_radius: 80.0,
            inner_radius: 34.0,
            ..ui.defaults()
        }
        .shape(),
        RegularPolygon {
            radius: 70.0,
            n: 6,
            ..ui.defaults()
        }
        .shape(),
    ])
    .arrange(Anchor::Right, 60.0)
    .move_to(960.0, 400.0);
    let dots: Vec<Shape> = (0..6)
        .map(|i| {
            Dot {
                radius: 20.0 + 6.0 * i as f32,
                fill: Some(ui.theme.colors.highlight),
                ..ui.defaults()
            }
            .shape()
        })
        .collect();
    let grid = VGroup::new(dots)
        .arrange_in_grid(3, (80.0, 40.0))
        .move_to(960.0, 780.0);
    let row = ui.show(VGroup {
        enter: Some(create(0.0)),
        ..row
    });
    let grid = ui.show(VGroup {
        enter: Some(
            GrowFromCenter {
                delay: 0.8,
                ..ui.defaults()
            }
            .into(),
        ),
        ..grid
    });
    svgr!(<g>{head}{row}{grid}</g>)
}

/// Leaders from three points of a curve, and three labels at almost one point spread so
/// they do not overlap.
fn labels<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Leader");
    let axes = Axes {
        x_range: Range::new(0.0, 6.0, 1.0),
        y_range: Range::new(0.0, 4.0, 1.0),
        x_length: 8.0,
        y_length: 4.4,
        center: (-2.2, -0.6),
        tips: false,
        enter: Some(create(0.0)),
        ..ui.defaults()
    };
    let curve = FunctionGraph {
        enter: Some(trace(0.3, 1.4)),
        ..axes.plot(ui, |x| 2.0 + 1.5 * (x * 1.1).sin())
    };
    let leaders: Vec<Leader> = [
        (1.4, "maximum", (70.0, -90.0)),
        (4.3, "minimum", (70.0, 90.0)),
        (5.6, "end", (100.0, 50.0)),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (x, text, (dx, dy)))| {
        let from = curve.point(x);
        Leader {
            from,
            to: (from.0 + dx, from.1 + dy),
            text: text.into(),
            enter: Some(create(1.6 + 0.2 * i as f32)),
            label_enter: Some(fade(1.9 + 0.2 * i as f32)),
            ..ui.defaults()
        }
    })
    .collect();
    // three labels placed at nearly the same point, then spread apart
    let mut stacked: Vec<Label> = ["first", "second", "third"]
        .into_iter()
        .enumerate()
        .map(|(i, text)| Label {
            text: text.into(),
            x: 1400.0 + 6.0 * i as f32,
            y: 420.0 + 4.0 * i as f32,
            enter: Some(fade(0.4 + 0.3 * i as f32)),
            ..ui.defaults()
        })
        .collect();
    fframes_kit::label::spread(ui, &mut stacked, 12.0);
    let axes = ui.show(axes);
    let curve = ui.show(curve);
    let leaders = ui.show(leaders);
    let stacked = ui.show(stacked);
    svgr!(<g>{head}{axes}{curve}{leaders}{stacked}</g>)
}

/// Angle and angular velocity of a damped pendulum, stepped every 1/240 s for 4 s.
fn pendulum() -> &'static Sim<(f64, f64)> {
    static SIM: std::sync::OnceLock<Sim<(f64, f64)>> = std::sync::OnceLock::new();
    SIM.get_or_init(|| {
        Sim::precompute(1.0 / 240.0, 960, (1.2, 0.0), |&(theta, omega), dt| {
            // semi-implicit Euler: g / L = 9.81 / 1.5, light damping
            let omega = omega + (-(9.81 / 1.5) * theta.sin() - 0.35 * omega) * dt;
            (theta + omega * dt, omega)
        })
    })
}

/// A pendulum whose motion was simulated once and is sampled every frame.
fn sim<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Sim");
    let (theta, omega) = pendulum().sample(f64::from(ui.seconds()));
    let pivot = ui.space.to_px(0.0, 2.6);
    let bob = ui.space.to_px(3.6 * theta.sin(), 2.6 - 3.6 * theta.cos());
    let rod = ui.show(Line {
        x1: pivot.x as f32,
        y1: pivot.y as f32,
        x2: bob.x as f32,
        y2: bob.y as f32,
        stroke: Some(ui.theme.colors.muted),
        ..ui.defaults()
    });
    let mass = ui.show(Dot {
        cx: bob.x as f32,
        cy: bob.y as f32,
        radius: 36.0,
        ..ui.defaults()
    });
    let reading = ui.show(Label {
        text: format!("angle {theta:+.2} rad   speed {omega:+.2} rad/s").into(),
        x: 960.0,
        y: 1000.0,
        direction: Anchor::Center,
        ..ui.defaults()
    });
    svgr!(<g>{head}{rod}{mass}{reading}</g>)
}

/// A highlight that fades in behind `area`.
fn highlight(ui: &Ui<'_, '_, '_>, area: Option<fframes_kit::kurbo::Rect>, delay: f32) -> Rectangle {
    let area = area.unwrap_or_default();
    Rectangle {
        cx: area.center().x as f32,
        cy: area.center().y as f32,
        width: area.width() as f32,
        height: area.height() as f32,
        fill: Some(ui.theme.colors.highlight),
        stroke: None,
        opacity: 0.3,
        enter: Some(EnterAnim::FadeIn(fade(delay))),
        ..ui.defaults()
    }
}

/// A matrix times a vector, with one entry highlighted.
fn matrix<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Matrix");
    let entries = |rows: &[&[&'static str]]| -> Vec<Vec<std::borrow::Cow<'static, str>>> {
        rows.iter()
            .map(|row| row.iter().map(|&e| e.into()).collect())
            .collect()
    };
    let a = Matrix {
        entries: entries(&[&["2", "0", "-1"], &["1", "3", "4"], &["0", "5", "2"]]),
        x: 640.0,
        enter: Some(create(0.0)),
        entries_enter: Some(fade(0.4)),
        ..ui.defaults()
    };
    let v = Matrix {
        entries: entries(&[&["x"], &["y"], &["z"]]),
        x: 1060.0,
        enter: Some(create(0.6)),
        entries_enter: Some(fade(0.9)),
        ..ui.defaults()
    };
    let name = a.bounds(ui).map(|b| Label {
        text: "A =".into(),
        x: b.x0 as f32,
        y: b.center().y as f32,
        direction: Anchor::Left,
        buff: 24.0,
        enter: Some(fade(0.2)),
        ..ui.defaults()
    });
    let mark = highlight(ui, a.cell(ui, 1, 1), 1.6);
    let mark = ui.show(mark);
    let a = ui.show(a);
    let v = ui.show(v);
    let name = name.map_or_else(Svgr::empty, |l| ui.show(l));
    svgr!(<g>{head}{mark}{a}{v}{name}</g>)
}

/// Manim's table example: labels on both sides, lines drawn with `Create`, a highlighted
/// cell.
fn table<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Table");
    let table = Table {
        rows: vec![
            vec!["3.2".into(), "4.1".into(), "2.8".into()],
            vec!["5.0".into(), "1.9".into(), "6.3".into()],
        ],
        col_labels: vec!["Mon".into(), "Tue".into(), "Wed".into()],
        row_labels: vec!["North".into(), "South".into()],
        y: 560.0,
        enter: Some(trace(0.0, 1.2)),
        entries_enter: Some(fade(0.8)),
        ..ui.defaults()
    };
    let mark = highlight(ui, table.cell(ui, 1, 2), 1.8);
    let mark = ui.show(mark);
    let table = ui.show(table);
    svgr!(<g>{head}{mark}{table}</g>)
}

/// A rectangle with braces labelling its width and height.
fn brace_label<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "BraceLabel");
    let (x0, y0, x1, y1) = (660.0, 380.0, 1160.0, 680.0);
    let rect = ui.show(Rectangle {
        cx: f32::midpoint(x0, x1),
        cy: f32::midpoint(y0, y1),
        width: x1 - x0,
        height: y1 - y0,
        enter: Some(create(0.0)),
        ..ui.defaults()
    });
    let width = ui.show(BraceLabel {
        brace: Brace {
            x1: x0,
            y1: y1 + 16.0,
            x2: x1,
            y2: y1 + 16.0,
            depth: 36.0,
            enter: Some(create(0.6)),
            ..ui.defaults()
        },
        text: "width 5".into(),
        label_enter: Some(fade(1.0)),
        ..ui.defaults()
    });
    // drawn bottom to top, so the tip points right
    let height = ui.show(BraceLabel {
        brace: Brace {
            x1: x1 + 16.0,
            y1,
            x2: x1 + 16.0,
            y2: y0,
            depth: 36.0,
            enter: Some(create(1.2)),
            ..ui.defaults()
        },
        text: "height 3".into(),
        label_enter: Some(fade(1.6)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{rect}{width}{height}</g>)
}

/// `x` of the tracker: from -2 to 2.5, then back to 0.5.
fn tracker() -> &'static fframes::animation::KeyFramesAnimation<f32> {
    static X: std::sync::OnceLock<fframes::animation::KeyFramesAnimation<f32>> =
        std::sync::OnceLock::new();
    X.get_or_init(|| {
        timeline!(
            at 0.8 => 2.2, animate -2.0_f32 => 2.5, Easing::EaseInOut,
            at 2.6 => 3.6, animate 2.5_f32 => 0.5, Easing::EaseInOut,
        )
    })
}

fn parabola(x: f64) -> f64 {
    0.4 * x * x
}

/// A dot on a parabola that follows a `ValueTracker`, with its value as a number.
fn value_tracker<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ValueTracker");
    let x: ValueTracker = tracker().into();
    let x = f64::from(ui.sample(&x));
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
    let curve = FunctionGraph {
        enter: Some(trace(0.2, 0.6)),
        ..axes.plot(ui, parabola)
    };
    let (px, py) = curve.point(x);
    let drop = axes.vertical_line(ui, x, parabola(x));
    let dot = ui.show(Dot {
        cx: px,
        cy: py,
        radius: 14.0,
        fill: Some(ui.theme.colors.highlight),
        ..ui.defaults()
    });
    let reading = ui.show(Label {
        text: format!("x = {x:.2}").into(),
        x: px,
        y: py,
        direction: Anchor::TopLeft,
        buff: 16.0,
        ..ui.defaults()
    });
    let axes = ui.show(axes);
    let curve = ui.show(curve);
    let drop = ui.show(drop);
    svgr!(<g>{head}{axes}{curve}{drop}{dot}{reading}</g>)
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("NumberLine", 3.0, number_line),
        GalleryScene::new("NumberPlane", 3.0, number_plane),
        GalleryScene::new("SinAndCosFunctionPlot", 4.5, sin_and_cos),
        GalleryScene::new("GraphAreaPlot", 3.5, graph_area),
        GalleryScene::new("Canvas", 3.0, canvas),
        GalleryScene::new("VGroup", 2.5, vgroup),
        GalleryScene::new("Leader", 3.0, labels),
        GalleryScene::new("Sim", 4.0, sim),
        GalleryScene::new("Matrix", 2.5, matrix),
        GalleryScene::new("Table", 2.5, table),
        GalleryScene::new("BraceLabel", 2.5, brace_label),
        GalleryScene::new("ValueTracker", 4.0, value_tracker),
    ]
}
