//! The math kit's figures, in the order of the plan's table: a point and an interval on a
//! number line, a plot with a tracer, a tangent and a secant, a Riemann sum, the area
//! under a graph, the unit circle, a linear transformation, a vector sum and the
//! Pythagorean theorem.

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

/// The unit circle turning most of the way round, with its cosine and sine.
fn unit_circle<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static ANGLE: OnceLock<KeyFramesAnimation<f32>> = OnceLock::new();
    let angle =
        ANGLE.get_or_init(|| timeline!(at 1.0 => 4.0, animate 0.5_f32 => 5.4, Easing::EaseInOut));
    let head = title(ui, "UnitCircle");
    let circle = ui.show(UnitCircle {
        center: (0.0, -0.4),
        radius: 2.6,
        angle: angle.into(),
        enter: Some(create(0.0)),
        parts_enter: Some(fade(0.6)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{circle}</g>)
}

/// A shear applied to the plane, next to its matrix.
fn linear_transform<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "LinearTransform");
    let transform = ui.show(LinearTransform {
        matrix: [[1.0, 1.0], [0.0, 1.0]],
        center: (-1.5, -0.4),
        delay: 1.0,
        run_time: 2.0,
        matrix_at: (1600.0, 540.0),
        enter: Some(create(0.0)),
        matrix_enter: Some(fade(0.6)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{transform}</g>)
}

/// Three vectors moving tip to tail, then their sum.
fn vector_sum<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "VectorSum");
    let sum = ui.show(VectorSum {
        vectors: vec![(3.0, 1.0), (1.0, 2.0), (-1.5, 1.0)],
        origin: (-2.5, -2.8),
        colors: vec![
            ui.theme.colors.accent,
            ui.theme.colors.highlight,
            Color::hex("#5fd68f"),
        ],
        labels: vec!["u".into(), "v".into(), "w".into()],
        resultant_label: "u + v + w".into(),
        delay: 1.2,
        run_time: 1.5,
        enter: Some(create(0.0)),
        resultant_enter: Some(create(2.9)),
        labels_enter: Some(fade(0.5)),
        resultant_label_enter: Some(fade(3.4)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{sum}</g>)
}

/// The 3-4-5 triangle with its squares drawn in turn.
fn pythagoras<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Pythagoras");
    let mut small = ui.with_space(Space {
        origin: (960.0, 560.0),
        unit: 70.0,
    });
    let figure = small.show(Pythagoras {
        corner: (-1.5, -2.4),
        enter: Some(create(0.0)),
        squares_enter: [Some(create(0.6)), Some(create(1.2)), Some(create(1.8))],
        labels_enter: Some(fade(2.6)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// One half as halves, quarters and eighths lining up, and three quarters as a pie next to
/// six eighths.
fn fractions<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "Fractions");
    let bars: Vec<Svgr> = [2, 4, 8]
        .into_iter()
        .enumerate()
        .map(|(row, d)| {
            ui.show(FractionBar {
                numerator: d / 2,
                denominator: d,
                x: 260.0,
                y: 300.0 + row as f32 * 170.0,
                width: 760.0,
                enter: Some(create(row as f32 * 0.2)),
                delay: 0.8 + row as f32 * 0.6,
                ..ui.defaults()
            })
        })
        .collect();
    let pies: Vec<Svgr> = [(3, 4, 1290.0), (6, 8, 1640.0)]
        .into_iter()
        .map(|(n, d, cx)| {
            ui.show(FractionPie {
                numerator: n,
                denominator: d,
                cx,
                cy: 470.0,
                radius: 140.0,
                enter: Some(create(0.3)),
                delay: if d == 4 { 1.0 } else { 2.0 },
                ..ui.defaults()
            })
        })
        .collect();
    let bars: Svgr = bars.into();
    let pies: Svgr = pies.into();
    svgr!(<g>{head}{bars}{pies}</g>)
}

/// A 2x3 times 3x2 product worked out entry by entry.
fn matrix_view<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "MatrixView");
    let view = ui.show(MatrixView {
        a: vec![vec![1.0, 2.0, 0.0], vec![-1.0, 3.0, 1.0]],
        b: vec![vec![2.0, 1.0], vec![0.0, 1.0], vec![4.0, -2.0]],
        delay: 0.9,
        step: 0.8,
        matrices_enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{view}</g>)
}

fn small_axes<'a>(ui: &Ui<'a, '_, '_>, center: (f64, f64), x: Range, y: Range) -> Axes<'a> {
    Axes {
        x_range: x,
        y_range: y,
        x_length: 5.6,
        y_length: 2.6,
        center,
        tips: false,
        number_style: ui.theme.typography.caption,
        enter: Some(create(0.0)),
        ..ui.defaults()
    }
}

/// The four plots on their own axes: a line with its points, bars, a histogram and a
/// heatmap.
fn plots<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static SAMPLES: OnceLock<Vec<f64>> = OnceLock::new();
    let samples = SAMPLES.get_or_init(|| {
        // three spread-out sequences summed: a bell from [-3, 3]
        (1..=600)
            .map(|i| {
                let i = f64::from(i);
                ((i * 0.618_034).fract() + (i * 0.414_214).fract() + (i * 0.732_051).fract() - 1.5)
                    * 2.0
            })
            .collect()
    });
    let head = title(ui, "Plots");
    let line_axes = small_axes(
        ui,
        (-3.6, 1.5),
        Range::new(0.0, 6.0, 1.0),
        Range::new(0.0, 4.0, 1.0),
    );
    let points = vec![
        (0.0, 1.0),
        (1.0, 1.6),
        (2.0, 1.2),
        (3.0, 2.4),
        (4.0, 2.1),
        (5.0, 3.4),
        (6.0, 3.0),
    ];
    let map = line_axes.map(ui);
    let line = ui.show(LinePlot {
        points: points.clone(),
        axes: map,
        enter: Some(create(0.5)),
        ..ui.defaults()
    });
    let dots = ui.show(ScatterPlot {
        points,
        axes: map,
        color: ui.theme.colors.highlight,
        delay: 0.5,
        stagger: 0.1,
        ..ui.defaults()
    });

    let bar_axes = small_axes(
        ui,
        (3.6, 1.5),
        Range::new(0.0, 4.0, 1.0),
        Range::new(0.0, 40.0, 10.0),
    );
    let chart = BarChart {
        delay: 0.6,
        stagger: 0.15,
        highlight: Some(2),
        value_style: ui.theme.typography.caption,
        ..fframes_kit::plot::bars(
            ui,
            &bar_axes.map(ui),
            vec![
                ("Q1".into(), 12.0),
                ("Q2".into(), 19.0),
                ("Q3".into(), 34.0),
                ("Q4".into(), 27.0),
            ],
            0.6,
        )
    };
    // the chart writes its values from the start; it comes in with its axes
    let shown = motion::progress(ui.seconds(), 0.2, 0.6);
    let chart = ui.show(chart);
    let chart = svgr!(<g opacity={shown}>{chart}</g>);

    let hist_axes = small_axes(
        ui,
        (-3.6, -2.3),
        Range::new(-3.0, 3.0, 1.0),
        Range::new(0.0, 0.5, 0.25),
    );
    let histogram = ui.show(Histogram {
        values: fframes_kit::plot::bin(samples, (-3.0, 3.0), 24, true),
        range: (-3.0, 3.0),
        axes: hist_axes.map(ui),
        delay: 0.8,
        ..ui.defaults()
    });

    let heat_axes = small_axes(
        ui,
        (3.6, -2.3),
        Range::new(0.0, 8.0, 1.0),
        Range::new(0.0, 4.0, 1.0),
    );
    let values = (0..4)
        .map(|r| {
            (0..8)
                .map(|c| (f64::from(c) * 0.7).sin() * (f64::from(r) * 0.9).cos())
                .collect()
        })
        .collect();
    let heatmap = ui.show(Heatmap {
        values,
        x_range: (0.0, 8.0),
        y_range: (0.0, 4.0),
        axes: heat_axes.map(ui),
        low: ui.theme.colors.accent,
        delay: 1.0,
        ..ui.defaults()
    });
    let axes: Vec<Svgr> = [line_axes, bar_axes, hist_axes, heat_axes]
        .into_iter()
        .map(|a| ui.show(a))
        .collect();
    let axes: Svgr = axes.into();
    svgr!(<g>{head}{heatmap}{axes}{line}{dots}{chart}{histogram}</g>)
}

/// 1500 seeded normal samples pouring into a histogram, then the normal curve that fits
/// them.
fn histogram_to_normal<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static SAMPLES: OnceLock<Vec<f64>> = OnceLock::new();
    static SHOWN: OnceLock<KeyFramesAnimation<f32>> = OnceLock::new();
    let samples =
        SAMPLES.get_or_init(|| fframes_kit::kits::math::normal_samples(7, 1500, 0.0, 1.0));
    let shown =
        SHOWN.get_or_init(|| timeline!(at 0.6 => 3.0, animate 0.0_f32 => 1500.0, Easing::EaseIn));
    let head = title(ui, "HistogramToNormal");
    let axes = Axes {
        x_range: Range::new(-3.0, 3.0, 1.0),
        y_range: Range::new(0.0, 0.5, 0.1),
        x_length: 10.0,
        y_length: 5.0,
        center: (-0.8, -0.6),
        tips: false,
        x_numbers: (-3..=3).map(f64::from).collect(),
        enter: Some(create(0.0)),
        ..ui.defaults()
    };
    let fit = ui.show(NormalFit {
        samples,
        shown: shown.into(),
        axes: axes.map(ui),
        curve_enter: Some(
            Create {
                delay: 3.1,
                run_time: 0.9,
                ..Create::default()
            }
            .into(),
        ),
        stats_at: (1600.0, 260.0),
        stats_enter: Some(fade(0.6)),
        ..ui.defaults()
    });
    let axes = ui.show(axes);
    svgr!(<g>{head}{axes}{fit}</g>)
}

/// A chain of 40 turning circles drawing a heart from its Fourier series.
fn epicycles<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TERMS: OnceLock<Vec<fframes_kit::kits::math::Term>> = OnceLock::new();
    static TURNS: OnceLock<KeyFramesAnimation<f32>> = OnceLock::new();
    let terms = TERMS.get_or_init(|| {
        let heart = Heart {
            height: 5.0,
            ..Heart::default()
        };
        fframes_kit::kits::math::fourier(&heart.path(), 513, 40)
    });
    let turns =
        TURNS.get_or_init(|| timeline!(at 0.8 => 4.8, animate 0.0_f32 => 1.0, Easing::Linear));
    let head = title(ui, "Epicycles");
    let chain = ui.show(Epicycles {
        terms,
        center: (0.0, -0.3),
        turns: turns.into(),
        enter: Some(EnterAnim::FadeIn(fade(0.0))),
        ..ui.defaults()
    });
    svgr!(<g>{head}{chain}</g>)
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        GalleryScene::new("NumberLinePoint", 4.0, number_line_point),
        GalleryScene::new("FunctionPlot", 4.0, function_plot),
        GalleryScene::new("TangentSecant", 4.5, tangent_secant),
        GalleryScene::new("RiemannSum", 4.5, riemann_sum),
        GalleryScene::new("AreaUnder", 3.5, area_under),
        GalleryScene::new("UnitCircle", 4.5, unit_circle),
        GalleryScene::new("LinearTransform", 4.0, linear_transform),
        GalleryScene::new("VectorSum", 4.5, vector_sum),
        GalleryScene::new("Pythagoras", 4.0, pythagoras),
        GalleryScene::new("Fractions", 4.0, fractions),
        GalleryScene::new("MatrixView", 4.6, matrix_view),
        GalleryScene::new("Plots", 4.0, plots),
        GalleryScene::new("HistogramToNormal", 4.6, histogram_to_normal),
        GalleryScene::new("Epicycles", 6.0, epicycles),
    ]
}
