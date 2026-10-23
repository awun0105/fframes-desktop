# fframes-kit: math kit

The math kit is the `math` feature of `fframes-kit`
(`fframes-kit = { .., features = ["math"] }`). Its figures are built from the explain layer
(`Axes`, `NumberLine`, `FunctionGraph`, see the `explain` module docs) and come with the
prelude. Animated values are `ValueTracker`s: a `timeline!` stored in the video and read
with `ui.sample`. Every entry names its scene in the components gallery
(`cargo run --release -p components-gallery -- strip <Scene> -n 8`).

The calculus figures are overlays. They take the mapping of the line or axes they sit on
and draw only themselves, so you show the line or axes once and put several figures on it.
`FunctionPlot` draws its axes too. The geometry figures (`UnitCircle`, `LinearTransform`,
`VectorSum`, `Pythagoras`) stand alone and are placed in units of `ui.space`; show them in
a `ui.with_space(..)` to draw them at another scale.

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

## UnitCircle

The unit circle at a tracked angle (radians, counterclockwise): the radius to the point,
the cosine along the x-axis, the sine up to the point and the angle's arc. With `labels`
it writes `cos 0.71`, `sin 0.71` and the angle in degrees. The circle and its axes draw
with `enter`, the moving parts fade in with `parts_enter`. Gallery scene: `UnitCircle`.

```rust
use fframes::animation::KeyFramesAnimation;
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>, angle: &'a KeyFramesAnimation<f32>) -> Svgr<'a> {
    ui.show(UnitCircle {
        center: (0.0, -0.4),
        radius: 2.6,
        angle: angle.into(),
        ..ui.defaults()
    })
}
```

## LinearTransform and VectorSum

`LinearTransform` is Manim's `ApplyMatrix` on a `LinearTransformationScene`. It moves a grid
and the two basis vectors from the identity to `matrix` over `run_time`, over the
original grid drawn faintly, and writes the matrix at `matrix_at` with the `Matrix`
component. `VectorSum` adds vectors tip to tail: each one moves from `origin` to the tip
of the one before, then the resultant draws with `resultant_enter`. `apply(m, p)` and
`towards(m, p)` are the matrix product and the interpolation from the identity they use.
Gallery scenes: `LinearTransform`, `VectorSum`.

```rust
use fframes_kit::prelude::*;
use fframes_kit::kits::math::apply;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let shear = ui.show(LinearTransform {
        matrix: [[1.0, 1.0], [0.0, 1.0]],
        delay: 1.0,
        ..ui.defaults()
    });
    let sum = ui.show(VectorSum {
        vectors: vec![(3.0, 1.0), (1.0, 2.0)],
        labels: vec!["u".into(), "v".into()],
        resultant_label: "u + v".into(),
        delay: 3.5,
        resultant_enter: Some(Create { delay: 5.0, ..Create::default() }.into()),
        resultant_label_enter: Some(FadeIn { delay: 5.5, ..FadeIn::default() }),
        ..ui.defaults()
    });
    svgr!(<g>{shear}{sum}</g>)
}

assert_eq!(apply([[1.0, 1.0], [0.0, 1.0]], (2.0, 3.0)), (5.0, 3.0));
```

## Pythagoras

A right triangle with legs `a` and `b` and a square on each side, each drawn with its own
`squares_enter` and labeled with its area (`a² = 9`, `b² = 16`, `c² = 25`). `corner` is the
right angle. Gallery scene: `Pythagoras`, drawn at 70 pixels per unit.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let create = |delay| Some(EnterAnim::from(Create { delay, ..Create::default() }));
    let mut small = ui.with_space(Space { origin: (960.0, 560.0), unit: 70.0 });
    small.show(Pythagoras {
        corner: (-1.5, -2.4),
        squares_enter: [create(0.6), create(1.2), create(1.8)],
        ..ui.defaults()
    })
}

assert_eq!(Pythagoras::default().areas(), [9.0, 16.0, 25.0]);
```

## FractionBar and FractionPie

A bar or a circle cut into `denominator` equal parts, of which the first `numerator` fill one
after another (`stagger` seconds apart, from `delay`). Bars of the same width stacked under
each other show equal fractions lining up. The fraction is written next to the bar or under
the circle. Both are placed in pixels. Gallery scene: `Fractions`.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let bars: Vec<Svgr> = [2, 4, 8]
        .into_iter()
        .enumerate()
        .map(|(row, d)| {
            ui.show(FractionBar {
                numerator: d / 2,
                denominator: d,
                y: 300.0 + row as f32 * 170.0,
                delay: row as f32 * 0.6,
                ..ui.defaults()
            })
        })
        .collect();
    let pie = ui.show(FractionPie { numerator: 3, denominator: 4, cx: 1600.0, ..ui.defaults() });
    let bars: Svgr = bars.into();
    svgr!(<g>{bars}{pie}</g>)
}
```

## MatrixView

A matrix product worked out entry by entry: `a × b = c`, centered on `(x, y)`. From
`delay`, every `step` seconds the next entry of the product is written, row by row, while its
row of `a` and column of `b` are highlighted. `product(a, b)` is the multiplication, `None`
when the sizes do not fit. Gallery scene: `MatrixView`.

```rust
use fframes_kit::prelude::*;
use fframes_kit::kits::math::product;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(MatrixView {
        a: vec![vec![1.0, 2.0, 0.0], vec![-1.0, 3.0, 1.0]],
        b: vec![vec![2.0, 1.0], vec![0.0, 1.0], vec![4.0, -2.0]],
        delay: 1.0,
        step: 0.8,
        ..ui.defaults()
    })
}

let c = product(&[vec![1.0, 2.0]], &[vec![3.0], vec![4.0]]);
assert_eq!(c, Some(vec![vec![11.0]]));
```

## Plots

`fframes_kit::plot` draws data on axes. It needs no feature, and the plots come with the
prelude. Like the overlays above, each plot takes the axes' mapping and draws only itself.
Gallery scene: `Plots`.

- `LinePlot`: a line through data points, straight or `smooth`. A point that is not finite
  breaks it.
- `ScatterPlot`: dots at data points that grow in one after another.
- `Histogram`: bars of `values` across `range`, standing on the x-axis.
  `bin(samples, range, bins, density)` counts samples into bins. With `density` the bars'
  area is 1, so a probability density drawn over them fits.
- `Heatmap`: a grid of cells coloured from `low` to `high`, with the first row at the top.
- `plot::bars(ui, &axes, bars, 0.6)`: the `BarChart` component placed on axes, the bars
  sharing the x span evenly.

```rust
use fframes_kit::prelude::*;
use fframes_kit::plot::{bars, bin};

fn figure<'a>(ui: &mut Ui<'a, '_, '_>, samples: &[f64]) -> Svgr<'a> {
    let axes = Axes {
        x_range: Range::new(-3.0, 3.0, 1.0),
        y_range: Range::new(0.0, 0.5, 0.25),
        ..ui.defaults()
    };
    let map = axes.map(ui);
    let histogram = ui.show(Histogram {
        values: bin(samples, (-3.0, 3.0), 24, true),
        range: (-3.0, 3.0),
        axes: map,
        ..ui.defaults()
    });
    let line = ui.show(LinePlot {
        points: vec![(-3.0, 0.0), (0.0, 0.4), (3.0, 0.0)],
        axes: map,
        smooth: true,
        ..ui.defaults()
    });
    let chart = ui.show(bars(ui, &map, vec![("a".into(), 0.2), ("b".into(), 0.4)], 0.6));
    let axes = ui.show(axes);
    svgr!(<g>{axes}{histogram}{line}{chart}</g>)
}

assert_eq!(bin(&[0.1, 0.2, 0.9], (0.0, 1.0), 2, false), vec![2.0, 1.0]);
```
