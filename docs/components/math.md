# fframes-kit: math kit

The math kit is the `math` feature of `fframes-kit`
(`fframes-kit = { .., features = ["math"] }`). Its figures are built from the explain layer
(`Axes`, `NumberLine`, `FunctionGraph`, see the `explain` module docs) and come with the
prelude. Animated values are `ValueTracker`s: a `timeline!` stored in the video and read
with `ui.sample`. Every entry names its scene in the components gallery
(`cargo run --release -p components-gallery -- strip <Scene> -n 8`).

Most figures are overlays. They take the mapping of the line or axes they sit on and draw
only themselves, so you show the line or axes once and put several figures on it.
`FunctionPlot` is the exception: it draws its axes too.

## NumberPoint and Interval

`NumberPoint` is a dot on a number line at a tracked value, with a label above it that can
write the value. `Interval` marks a range of the line with a filled dot at a closed end and
a ring at an open one. Gallery scene: `NumberLinePoint`.

```rust
use fframes::animation::KeyFramesAnimation;
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>, x: &'a KeyFramesAnimation<f32>) -> Svgr<'a> {
    let line = NumberLine {
        range: Range::new(-4.0, 4.0, 1.0),
        length: 12.0,
        numbers: (-4..=4).map(f64::from).collect(),
        ..ui.defaults()
    };
    let point = ui.show(NumberPoint {
        line: line.clone(),
        value: x.into(),
        label: "x = ".into(),
        decimals: Some(1),
        ..ui.defaults()
    });
    // [-1, 2)
    let interval = ui.show(Interval {
        line: line.clone(),
        from: -1.0,
        to: 2.0,
        closed: (true, false),
        ..ui.defaults()
    });
    let line = ui.show(line);
    svgr!(<g>{line}{interval}{point}</g>)
}
```

## FunctionPlot

Axes with a graph, its label and a tracer: a dot that follows a tracked `x` along the
graph, with a dashed line down to the x-axis and the point's coordinates. Build `graph` with
`axes.plot` on the same axes. Gallery scene: `FunctionPlot`.

```rust
use fframes::animation::KeyFramesAnimation;
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>, x: &'a KeyFramesAnimation<f32>) -> Svgr<'a> {
    let axes = Axes {
        x_range: Range::new(-3.0, 3.0, 1.0),
        y_range: Range::new(-1.0, 4.0, 1.0),
        x_length: 9.0,
        y_length: 5.0,
        ..ui.defaults()
    };
    let graph = axes.plot(ui, |x| 0.4 * x * x);
    ui.show(FunctionPlot {
        graph,
        axes,
        label: "0.4 x²".into(),
        label_at: 2.5,
        tracer: Some(x.into()),
        ..ui.defaults()
    })
}
```

## Tangent and Secant

`Tangent` is the tangent of a function at a tracked `x`, a line of fixed length centered
on the point. `Secant` goes through `x` and `x + dx`, with the dashed `dx` and `dy` legs
of Manim's `get_secant_slope_group`. Animate `dx` to 0 and it ends on the tangent. Both take
the axes' mapping, `axes.map(ui)`. Gallery scene: `TangentSecant`.

```rust
use fframes::animation::KeyFramesAnimation;
use fframes_kit::prelude::*;

fn parabola(x: f64) -> f64 {
    0.4 * x * x
}

fn figure<'a>(ui: &mut Ui<'a, '_, '_>, dx: &'a KeyFramesAnimation<f32>) -> Svgr<'a> {
    let axes = Axes {
        x_range: Range::new(-3.0, 3.0, 1.0),
        y_range: Range::new(0.0, 4.0, 1.0),
        ..ui.defaults()
    };
    let map = axes.map(ui);
    let secant = ui.show(Secant {
        f: parabola,
        axes: map,
        x: 1.0.into(),
        dx: dx.into(),
        ..ui.defaults()
    });
    let tangent = ui.show(Tangent {
        f: parabola,
        axes: map,
        x: (-1.5).into(),
        ..ui.defaults()
    });
    let graph = ui.show(axes.plot(ui, parabola));
    let axes = ui.show(axes);
    svgr!(<g>{axes}{graph}{secant}{tangent}</g>)
}
```

## RiemannSum and AreaUnder

`RiemannSum` draws Riemann rectangles with each width of `dxs` in turn, cross-fading from
one to the next, and can write the sum. `AreaUnder` reveals the area under a graph from
left to right and writes the integral of the part shown so far. Build their `rects` and
`area` with `axes.riemann_rectangles` and `axes.area`. `integral(f, g, a, b, n)` is the
trapezoid rule they use. Gallery scenes: `RiemannSum`, `AreaUnder`.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let axes = Axes {
        x_range: Range::new(0.0, 3.0, 1.0),
        y_range: Range::new(0.0, 4.0, 1.0),
        ..ui.defaults()
    };
    let f = |x: f64| 0.4 * x * x;
    let sum = ui.show(RiemannSum {
        rects: axes.riemann_rectangles(ui, f, (0.0, 2.0), 0.5),
        dxs: vec![0.5, 0.25, 0.1, 0.05],
        sum: Some(Label { text: "sum = ".into(), x: 1400.0, y: 300.0, ..ui.defaults() }),
        ..ui.defaults()
    });
    let area = ui.show(AreaUnder {
        area: axes.area(ui, f, (2.0, 3.0)),
        delay: 1.0,
        value: Some(Label { text: "area = ".into(), x: 1400.0, y: 360.0, ..ui.defaults() }),
        ..ui.defaults()
    });
    let graph = ui.show(axes.plot(ui, f));
    let axes = ui.show(axes);
    svgr!(<g>{axes}{sum}{area}{graph}</g>)
}

// the integral of 0.4 x² from 0 to 3 is 0.4 * 27 / 3
let area = fframes_kit::kits::math::integral(|x| 0.4 * x * x, None, 0.0, 3.0, 256);
assert!((area - 3.6).abs() < 1e-3);
```
