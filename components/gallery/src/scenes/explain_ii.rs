//! Numbers, text, coordinate systems, fields and graphs, one scene per group.

use fframes_kit::{explain, kurbo, prelude::*};

use crate::{GalleryScene, scenes::core::title};

fn create() -> EnterAnim {
    Create::default().into()
}

fn fade_in(delay: f32) -> FadeIn {
    FadeIn {
        delay,
        ..FadeIn::default()
    }
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        // the numbers change on every frame, their digits stay in their slots
        GalleryScene::new("DecimalNumber", 3.0, decimal_number),
        // the plane is created, a tracked number moves along a path with its label
        GalleryScene::new("ComplexPlane", 2.0, complex_plane),
        // the grid is created and a rose is drawn on it
        GalleryScene::new("PolarPlane", 2.0, polar_plane),
        // log axes with an exponential drawn straight, a unit interval below
        GalleryScene::new("LogBase", 2.0, log_base),
        // the cells grow in, the braces and labels fade in
        GalleryScene::new("SampleSpace", 2.0, sample_space),
        // axes are created, then two implicit curves are drawn on them
        GalleryScene::new("ImplicitFunction", 2.0, implicit_function),
        // the arrows grow in, coloured by their length
        GalleryScene::new("ArrowVectorField", 2.0, arrow_vector_field),
        // a piece of every stream line runs along it
        GalleryScene::new("StreamLines", 4.0, stream_lines),
        // one graph in eight layouts, created
        GalleryScene::new("GraphLayouts", 2.0, graph_layouts),
        // a labelled directed graph with styled and curved edges
        GalleryScene::new("DiGraph", 2.0, di_graph),
    ]
}

fn decimal_number<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = ui.show(Title {
        text: "Numbers and text".into(),
        match_underline_width_to_text: true,
        ..ui.defaults()
    });
    // a value that moves on every frame, as a closure of time
    let p = ui.seconds() / 3.0;
    let mono = TextStyle {
        family: "JetBrains Mono",
        ..ui.theme.typography.title
    };
    let money = ui.show(DecimalNumber {
        value: (9_876.54 * p).into(),
        x: 700.0,
        y: 340.0,
        anchor: TextAnchor::End,
        unit: " €",
        ..ui.defaults()
    });
    let signed = ui.show(DecimalNumber {
        value: (4.0 * p - 2.0).into(),
        x: 860.0,
        y: 340.0,
        include_sign: true,
        num_decimal_places: 3,
        style: mono,
        ..ui.defaults()
    });
    let count = ui.show(Integer {
        value: (120.0 * p).into(),
        x: 1720.0,
        y: 340.0,
        anchor: TextAnchor::End,
        ..ui.defaults()
    });
    let variable = ui.show(Variable {
        label: "t".into(),
        value: ui.seconds().into(),
        x: 200.0,
        y: 500.0,
        unit: " s",
        ..ui.defaults()
    });
    let list = ui.show(BulletedList {
        items: vec![
            "Digits sit in fixed slots".into(),
            "Integer rounds".into(),
            "Variable shows label = value".into(),
        ],
        x: 200.0,
        y: 640.0,
        ..ui.defaults()
    });
    let markup = ui.show(MarkupText {
        markup: "Markup: <u>underline</u>, <span foreground='#ff9f43'>colour</span> and <span size='x-large'>size</span>".into(),
        x: 1000.0,
        y: 680.0,
        style: ui.theme.typography.body,
        ..ui.defaults()
    });
    svgr!(<g>{head}{money}{signed}{count}{variable}{list}{markup}</g>)
}

fn complex_plane<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ComplexPlane");
    let plane = ComplexPlane {
        plane: NumberPlane {
            axes: Axes {
                x_range: Range::new(-5.0, 5.0, 1.0),
                y_range: Range::new(-3.0, 3.0, 1.0),
                x_length: 10.0,
                y_length: 6.0,
                center: (0.0, -0.5),
                x_numbers: vec![-4.0, -2.0, 2.0, 4.0],
                y_numbers: vec![-2.0, -1.0, 1.0, 2.0],
                enter: Some(create()),
                labels_enter: Some(fade_in(0.6)),
                ..ui.defaults()
            },
            enter: Some(create()),
            ..ui.defaults()
        },
    };
    // z moves from 1 to 3 + 2i on a quarter turn, as a closure of time
    let t = f64::from(motion::clamp01((ui.seconds() - 0.8) / 1.0));
    let z = (1.0 + 2.0 * t, 2.0 * (t * std::f64::consts::FRAC_PI_2).sin());
    let (x, y) = plane.n2p(ui, z);
    let label = plane.label(ui, z, 1);
    let grid = ui.show(plane);
    let dot = ui.show(Dot {
        cx: x,
        cy: y,
        enter: Some(
            GrowFromCenter {
                delay: 0.6,
                ..ui.defaults()
            }
            .into(),
        ),
        ..ui.defaults()
    });
    let label = ui.show(Label {
        enter: Some(fade_in(0.6)),
        ..label
    });
    svgr!(<g>{grid}{head}{dot}{label}</g>)
}

/// A four-petal rose, `r = 3 |cos 2θ|`, in pixels of the plane.
fn rose(plane: &PolarPlane, space: Space, until: f64) -> kurbo::BezPath {
    let mut path = kurbo::BezPath::new();
    let steps = 360;
    for i in 0..=steps {
        let theta = std::f64::consts::TAU * until * f64::from(i) / f64::from(steps);
        let p = plane.pr2pt_in(space, (3.0 * (2.0 * theta).cos().abs(), theta));
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path
}

fn polar_plane<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "PolarPlane");
    let plane = PolarPlane {
        center: (0.0, -0.2),
        size: 6.0,
        enter: Some(create()),
        labels_enter: Some(fade_in(0.6)),
        ..ui.defaults()
    };
    let curve = rose(&plane, ui.space, 1.0);
    let grid = ui.show(plane);
    let curve = ui.show(SvgPath {
        path: curve,
        stroke: Some(ui.theme.colors.highlight),
        stroke_width: 5.0,
        fill: None,
        enter: Some(
            Create {
                delay: 0.6,
                ..ui.defaults()
            }
            .into(),
        ),
        ..ui.defaults()
    });
    svgr!(<g>{head}{grid}{curve}</g>)
}

fn log_base<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "LogBase");
    let axes = Axes {
        x_range: Range::new(0.0, 10.0, 1.0),
        y_range: Range::new(0.0, 4.0, 1.0),
        y_scaling: Scaling::LogBase(10.0),
        x_length: 10.0,
        y_length: 4.4,
        center: (0.0, 0.5),
        x_numbers: vec![2.0, 4.0, 6.0, 8.0, 10.0],
        y_numbers: vec![10.0, 100.0, 1000.0],
        enter: Some(create()),
        labels_enter: Some(fade_in(0.6)),
        ..ui.defaults()
    };
    // 2^x is a straight line on a log axis
    let graph = FunctionGraph {
        enter: Some(
            Create {
                delay: 0.6,
                ..ui.defaults()
            }
            .into(),
        ),
        ..axes.plot(ui, |x| 2f64.powf(x * 1.2))
    };
    let axes = ui.show(axes);
    let graph = ui.show(graph);
    let unit = ui.show(NumberLine {
        center: (0.0, -2.9),
        enter: Some(create()),
        numbers_enter: Some(fade_in(0.6)),
        ..NumberLine::unit_interval(ui.theme)
    });
    svgr!(<g>{head}{axes}{graph}{unit}</g>)
}

fn sample_space<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "SampleSpace");
    let space = ui.show(SampleSpace {
        columns: vec![
            SampleColumn::new(
                0.35,
                "rain",
                vec![SampleRow::new(0.8, "late"), SampleRow::new(0.2, "on time")],
            ),
            SampleColumn::new(
                0.65,
                "no rain",
                vec![
                    SampleRow::new(0.25, "late"),
                    SampleRow::new(0.75, "on time"),
                ],
            ),
        ],
        center: (0.0, 0.2),
        width: 8.0,
        height: 4.6,
        enter: Some(GrowFromCenter::default().into()),
        labels_enter: Some(fade_in(0.6)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{space}</g>)
}

fn implicit_function<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ImplicitFunction");
    let axes = Axes {
        x_range: Range::new(-5.0, 5.0, 1.0),
        y_range: Range::new(-3.0, 3.0, 1.0),
        x_length: 10.0,
        y_length: 6.0,
        center: (0.0, -0.5),
        enter: Some(create()),
        ..ui.defaults()
    };
    let draw = Some(
        Create {
            delay: 0.6,
            ..ui.defaults()
        }
        .into(),
    );
    // Manim's example, x y² - x² y = 2, and a circle
    let curve = ui.show(ImplicitFunction {
        stroke: Some(ui.theme.colors.highlight),
        enter: draw,
        ..axes.plot_implicit_curve(ui, |x, y| x * y * y - x * x * y - 2.0)
    });
    let circle = ui.show(ImplicitFunction {
        enter: draw,
        ..axes.plot_implicit_curve(ui, |x, y| x * x + y * y - 4.0)
    });
    let axes = ui.show(axes);
    svgr!(<g>{head}{axes}{curve}{circle}</g>)
}

/// A field that turns and drifts, after Manim's stream lines example.
fn swirl(x: f64, y: f64) -> (f64, f64) {
    (
        (x / 2.0).sin() - (y / 2.0).cos(),
        (x / 2.0).sin() + 0.3 * (y / 1.5).cos(),
    )
}

fn arrow_vector_field<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ArrowVectorField");
    let field = ui.show(ArrowVectorField {
        field: VectorField {
            f: swirl,
            x_range: Range::new(-6.5, 6.5, 1.0),
            y_range: Range::new(-3.5, 2.5, 1.0),
            ..ui.defaults()
        },
        enter: Some(GrowFromCenter::default().into()),
        ..ui.defaults()
    });
    svgr!(<g>{field}{head}</g>)
}

fn stream_lines<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "StreamLines");
    let lines = ui.show(StreamLines {
        field: VectorField {
            f: swirl,
            x_range: Range::new(-7.0, 7.0, 0.7),
            y_range: Range::new(-4.0, 2.6, 0.6),
            ..ui.defaults()
        },
        padding: 0.0,
        seed: 3,
        flow: Some(Flow::default()),
        ..ui.defaults()
    });
    svgr!(<g>{lines}{head}</g>)
}

/// A ring of six with two chords and a tail of two.
const RING: [(usize, usize); 10] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 4),
    (4, 5),
    (5, 0),
    (0, 3),
    (1, 4),
    (5, 6),
    (6, 7),
];

fn graph_layouts<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "GraphLayouts");
    let layouts = [
        ("circular", GraphLayout::Circular),
        ("kamada_kawai", GraphLayout::KamadaKawai),
        (
            "shell",
            GraphLayout::Shell {
                shells: vec![vec![0], vec![1, 2, 3, 4, 5]],
            },
        ),
        ("spectral", GraphLayout::Spectral),
        (
            "spring",
            GraphLayout::Spring {
                seed: 1,
                iterations: 50,
            },
        ),
        (
            "partite",
            GraphLayout::Partite {
                parts: vec![vec![0, 2, 4], vec![1, 3, 5]],
            },
        ),
        ("tree", GraphLayout::Tree { root: 0 }),
        ("spiral", GraphLayout::Spiral),
    ];
    let names = ["0", "1", "2", "3", "4", "5", "6", "7"];
    let caption = ui.theme.typography.caption;
    let muted = ui.theme.colors.muted;
    let mut parts = Vec::new();
    for (i, (name, layout)) in layouts.into_iter().enumerate() {
        let center = (-5.1 + 3.4 * (i % 4) as f64, 1.2 - 3.5 * (i / 4) as f64);
        let graph = ui.show(explain::Graph {
            layout,
            layout_scale: 1.1,
            center,
            radius: 0.09,
            vertex_styles: vec![VertexStyle {
                vertex: 0,
                fill: Some(ui.theme.colors.highlight),
                ..VertexStyle::default()
            }],
            enter: Some(create()),
            ..explain::Graph::new(ui, &names, &RING)
        });
        let at = ui.space.to_px(center.0, center.1 - 1.55);
        parts.push(graph);
        parts.push(svgr!(
            <text x={at.x} y={at.y} font-family={caption.family} font-size={caption.size}
                  fill={muted} text-anchor="middle">{name}</text>
        ));
    }
    let parts: Svgr<'a> = parts.into_iter().collect();
    svgr!(<g>{head}{parts}</g>)
}

fn di_graph<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "DiGraph");
    let accent = ui.theme.colors.accent;
    let graph = ui.show(DiGraph {
        graph: explain::Graph {
            layout: GraphLayout::Circular,
            layout_scale: 2.6,
            center: (0.0, -0.4),
            labels: true,
            edge_styles: vec![
                EdgeStyle {
                    edge: (0, 1),
                    color: Some(accent),
                    width: Some(6.0),
                    label: Some("start".into()),
                    ..EdgeStyle::default()
                },
                EdgeStyle {
                    edge: (4, 0),
                    bend: Some(-60.0),
                    ..EdgeStyle::default()
                },
            ],
            enter: Some(create()),
            labels_enter: Some(fade_in(0.6)),
            ..explain::Graph::new(
                ui,
                &["A", "B", "C", "D", "E"],
                &[(0, 1), (1, 2), (2, 1), (2, 3), (3, 4), (4, 0), (1, 3)],
            )
        },
    });
    svgr!(<g>{head}{graph}</g>)
}
