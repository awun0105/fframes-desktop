//! Graphs of `y = f(x)` on [`Axes`](super::Axes): Manim's `plot`, `get_riemann_rectangles`
//! and `get_area`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use kurbo::{BezPath, Point, Shape as _};

use crate::{
    shapes::{Paint, shape},
    ui::Ui,
};

use super::{AxesMap, Scaling, Space};

/// A function of one variable. A plain `fn` (or a closure that captures nothing), so a graph
/// can cache its samples by the function's address.
pub type Func = fn(f64) -> f64;

/// A curve `t -> (x, y)`, a plain `fn` like [`Func`].
pub type Curve = fn(f64) -> (f64, f64);

/// What was sampled (a graph, a curve, an implicit curve, stream lines, ...), the function's
/// address and the bits of the parameters it was sampled with.
pub(crate) type SampleKey = (u8, usize, Vec<u64>);

/// Most sample runs kept; the cache starts over when it is full.
const CACHE_ENTRIES: usize = 512;

type SampleCache = Mutex<HashMap<SampleKey, Arc<[(f64, f64)]>>>;

fn sample_cache() -> &'static SampleCache {
    static CACHE: OnceLock<SampleCache> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

/// The cached points of `key`, computed by `points` the first time. Runs that are broken
/// into pieces separate them with a `(NaN, NaN)` point.
pub(crate) fn cached(
    key: SampleKey,
    points: impl FnOnce() -> Arc<[(f64, f64)]>,
) -> Arc<[(f64, f64)]> {
    if let Some(hit) = sample_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(&key).cloned())
    {
        return hit;
    }
    let points = points();
    if let Ok(mut cache) = sample_cache().lock() {
        if cache.len() >= CACHE_ENTRIES {
            cache.clear();
        }
        cache.insert(key, points.clone());
    }
    points
}

/// `samples + 1` evenly spaced points `(x, f(x))` from `a` to `b`, in data units. They are
/// computed once per function, range and count, so every frame of a graph reuses them.
pub fn sample(f: Func, a: f64, b: f64, samples: usize) -> Arc<[(f64, f64)]> {
    let samples = samples.max(1);
    cached(
        (
            0,
            f as usize,
            vec![a.to_bits(), b.to_bits(), samples as u64],
        ),
        || {
            (0..=samples)
                .map(|i| {
                    let x = a + (b - a) * i as f64 / samples as f64;
                    (x, f(x))
                })
                .collect()
        },
    )
}

/// `samples + 1` points `(x, f(x))` from `a` to `b`, evenly spaced along an axis with
/// `scaling` (by their exponent on a `LogBase` axis), cached like [`sample`].
pub fn sample_scaled(
    f: Func,
    a: f64,
    b: f64,
    samples: usize,
    scaling: Scaling,
) -> Arc<[(f64, f64)]> {
    let Scaling::LogBase(base) = scaling else {
        return sample(f, a, b, samples);
    };
    let samples = samples.max(1);
    let params = vec![a.to_bits(), b.to_bits(), samples as u64, base.to_bits()];
    let key = (2, f as usize, params);
    cached(key, || {
        let (a, b) = (scaling.to_axis(a), scaling.to_axis(b));
        (0..=samples)
            .map(|i| {
                let x = scaling.from_axis(a + (b - a) * i as f64 / samples as f64);
                (x, f(x))
            })
            .collect()
    })
}

/// `samples + 1` points `f(t)` for evenly spaced `t` from `a` to `b`, cached like
/// [`sample`].
pub fn sample_curve(f: Curve, a: f64, b: f64, samples: usize) -> Arc<[(f64, f64)]> {
    let samples = samples.max(1);
    cached(
        (
            1,
            f as usize,
            vec![a.to_bits(), b.to_bits(), samples as u64],
        ),
        || {
            (0..=samples)
                .map(|i| f(a + (b - a) * i as f64 / samples as f64))
                .collect()
        },
    )
}
/// Values beyond this many units from the axes' range start a new piece of the curve, like
/// a value that is not finite: `tan` near its poles.
const FAR: f64 = 1e4;

/// A smooth curve through pixel points (Catmull-Rom as cubic Béziers), broken where a point
/// is `None`.
pub(crate) fn smooth(points: impl IntoIterator<Item = Option<Point>>) -> BezPath {
    let mut path = BezPath::new();
    let mut run: Vec<Point> = Vec::new();
    let flush = |run: &mut Vec<Point>, path: &mut BezPath| {
        match run.len() {
            0 => {}
            1 => {
                path.move_to(run[0]);
            }
            n => {
                path.move_to(run[0]);
                for i in 0..n - 1 {
                    let p0 = run[i.saturating_sub(1)];
                    let (p1, p2) = (run[i], run[i + 1]);
                    let p3 = run[(i + 2).min(n - 1)];
                    path.curve_to(p1 + (p2 - p0) / 6.0, p2 - (p3 - p1) / 6.0, p2);
                }
            }
        }
        run.clear();
    };
    for p in points {
        match p {
            Some(p) => run.push(p),
            None => flush(&mut run, &mut path),
        }
    }
    flush(&mut run, &mut path);
    path
}

/// The pixel of a data point, `None` when it is not drawable: not finite, far outside the
/// y range, or not positive on a `LogBase` axis.
pub(crate) fn graph_px(map: &AxesMap, x: f64, y: f64) -> Option<Point> {
    let span = (map.y.max - map.y.min).abs().max(1.0);
    let (ax, ay) = (map.x_scaling.to_axis(x), map.y_scaling.to_axis(y));
    (ax.is_finite() && ay.is_finite() && (ay - map.y.min).abs() < FAR * span)
        .then(|| map.a2p(ax, ay))
}

shape! {
    /// The graph of `f` from `x_range.0` to `x_range.1` on axes (Manim `axes.plot(f)`),
    /// usually built with [`Axes::plot`](super::Axes::plot). Its samples are cached; a
    /// function that is not finite somewhere leaves a gap there.
    #[allow(unpredictable_function_pointer_comparisons)] // equal props, not equal functions
    FunctionGraph(t) {
        f: Func = |x| x,
        x_range: (f64, f64) = (-1.0, 1.0),
        axes: AxesMap = AxesMap::default(),
        /// Straight pieces of the sampled curve before smoothing.
        samples: usize = 256,
    }
    paint: Paint::outline(t);
    path(s) {
        let points = sample_scaled(s.f, s.x_range.0, s.x_range.1, s.samples, s.axes.x_scaling);
        smooth(points.iter().map(|&(x, y)| graph_px(&s.axes, x, y)))
    }
}

impl FunctionGraph {
    /// The pixel on the graph above `x` (Manim `input_to_graph_point`, `i2gp`).
    pub fn point(&self, x: f64) -> (f32, f32) {
        let p = self.axes.c2p(x, (self.f)(x));
        (p.x as f32, p.y as f32)
    }
}

shape! {
    /// The curve `t -> f(t)` for `t` from `t_range.0` to `t_range.1` (Manim
    /// `ParametricFunction`), in data units of `axes` when set, otherwise in units of
    /// `space`. Build one with [`ParametricFunction::new`] to take `ui.space`, or with
    /// [`Axes::plot_parametric`](super::Axes::plot_parametric). Its samples are cached.
    #[allow(unpredictable_function_pointer_comparisons)] // equal props, not equal functions
    ParametricFunction(t) {
        f: Curve = |t| (t.cos(), t.sin()),
        t_range: (f64, f64) = (0.0, std::f64::consts::TAU),
        axes: Option<AxesMap> = None,
        space: Space = Space::FULL_HD,
        samples: usize = 256,
    }
    paint: Paint::outline(t);
    path(s) {
        let points = sample_curve(s.f, s.t_range.0, s.t_range.1, s.samples);
        smooth(points.iter().map(|&(x, y)| s.px(x, y)))
    }
}

impl ParametricFunction {
    /// The curve of `f` over `t_range` in `ui.space`, with the theme's paint.
    pub fn new(ui: &Ui<'_, '_, '_>, f: Curve, t_range: (f64, f64)) -> Self {
        Self {
            f,
            t_range,
            space: ui.space,
            ..ui.defaults()
        }
    }

    /// The pixel of a point of the curve, `None` when it is not finite.
    fn px(&self, x: f64, y: f64) -> Option<Point> {
        if !(x.is_finite() && y.is_finite()) {
            return None;
        }
        Some(match &self.axes {
            Some(map) => map.c2p(x, y),
            None => self.space.to_px(x, y),
        })
    }

    /// The pixel of the curve at `t`.
    pub fn point(&self, t: f64) -> (f32, f32) {
        let (x, y) = (self.f)(t);
        let p = self.px(x, y).unwrap_or_default();
        (p.x as f32, p.y as f32)
    }
}
/// Where a Riemann rectangle takes its height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleAt {
    #[default]
    Left,
    Center,
    Right,
}

shape! {
    /// Rectangles of width `dx` from `x_range.0` to `x_range.1`, each as high as `f` at its
    /// left edge, center or right edge (Manim `get_riemann_rectangles`). Below the x-axis
    /// for negative values.
    #[allow(unpredictable_function_pointer_comparisons)] // equal props, not equal functions
    RiemannRectangles(t) {
        f: Func = |x| x,
        x_range: (f64, f64) = (0.0, 1.0),
        dx: f64 = 0.1,
        sample_at: SampleAt = SampleAt::Left,
        axes: AxesMap = AxesMap::default(),
    }
    paint: Paint {
        fill: Some(t.colors.accent),
        stroke: Some(t.colors.background),
        stroke_width: t.stroke.thin,
    };
    path(s) {
        let mut path = BezPath::new();
        let base = s.axes.x_axis_y();
        for (x0, x1, y) in s.rects() {
            let (p0, p1) = (s.axes.c2p(x0, base), s.axes.c2p(x1, y));
            path.extend(kurbo::Rect::from_points(p0, p1).path_elements(0.1));
        }
        path
    }
}

impl RiemannRectangles {
    /// Each rectangle's left and right edge and height, in data units; a height that is not
    /// finite leaves its rectangle out.
    fn rects(&self) -> impl Iterator<Item = (f64, f64, f64)> + '_ {
        let (a, b) = self.x_range;
        let n = if self.dx > 0.0 && b > a {
            (((b - a) / self.dx).round() as usize).clamp(1, 10_000)
        } else {
            0
        };
        (0..n).filter_map(move |i| {
            let x0 = a + self.dx * i as f64;
            let x1 = x0 + self.dx;
            let at = match self.sample_at {
                SampleAt::Left => x0,
                SampleAt::Center => f64::midpoint(x0, x1),
                SampleAt::Right => x1,
            };
            let y = (self.f)(at);
            y.is_finite().then_some((x0, x1, y))
        })
    }

    /// The Riemann sum: the signed area of the rectangles, in data units.
    pub fn sum(&self) -> f64 {
        self.rects().map(|(x0, x1, y)| (x1 - x0) * y).sum()
    }
}

shape! {
    /// The region between `f` and `bounded` (the x-axis when `None`) from `x_range.0` to
    /// `x_range.1` (Manim `get_area(graph, x_range, bounded_graph)`).
    #[allow(unpredictable_function_pointer_comparisons)] // equal props, not equal functions
    Area(t) {
        f: Func = |x| x,
        bounded: Option<Func> = None,
        x_range: (f64, f64) = (0.0, 1.0),
        axes: AxesMap = AxesMap::default(),
        samples: usize = 128,
    }
    paint: Paint {
        fill: Some(t.colors.muted),
        stroke: None,
        stroke_width: t.stroke.regular,
    };
    path(s) {
        let (a, b) = s.x_range;
        let top = sample(s.f, a, b, s.samples);
        let bottom = s.bounded.map(|g| sample(g, a, b, s.samples));
        let base = s.axes.x_axis_y();
        let finite = |p: &(f64, f64)| p.1.is_finite();
        let mut path = BezPath::new();
        let mut points = top
            .iter()
            .filter(|p| finite(p))
            .map(|&(x, y)| s.axes.c2p(x, y))
            .chain(
                (0..top.len())
                    .rev()
                    .filter_map(|i| match &bottom {
                        Some(g) => finite(&g[i]).then_some(g[i]),
                        None => Some((top[i].0, base)),
                    })
                    .map(|(x, y)| s.axes.c2p(x, y)),
            );
        let Some(first) = points.next() else {
            return path;
        };
        path.move_to(first);
        for p in points {
            path.line_to(p);
        }
        path.close_path();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{explain::Range, geometry::Geometry};

    fn square(x: f64) -> f64 {
        x * x
    }

    fn unit_axes() -> AxesMap {
        AxesMap {
            x: Range::new(0.0, 4.0, 1.0),
            y: Range::new(0.0, 4.0, 1.0),
            x_length: 4.0,
            y_length: 4.0,
            center: (0.0, 0.0),
            space: crate::explain::Space {
                origin: (0.0, 0.0),
                unit: 100.0,
            },
            x_scaling: Scaling::Linear,
            y_scaling: Scaling::Linear,
        }
    }

    #[test]
    fn samples_are_cached_per_function_and_range() {
        let a = sample(square, 0.0, 2.0, 8);
        let b = sample(square, 0.0, 2.0, 8);
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(a.len(), 9);
        assert_eq!(a[8], (2.0, 4.0));
        let c = sample(square, 0.0, 3.0, 8);
        assert!(!Arc::ptr_eq(&a, &c));
        let d = sample(f64::sin, 0.0, 2.0, 8);
        assert!(!Arc::ptr_eq(&a, &d));
    }

    #[test]
    fn graph_passes_through_its_samples_and_breaks_at_poles() {
        let graph = FunctionGraph {
            f: square,
            x_range: (0.0, 2.0),
            axes: unit_axes(),
            samples: 4,
            ..FunctionGraph::default()
        };
        // the axes' box is centered on the origin: data (2, 2) is the origin
        assert_eq!(graph.point(2.0), (0.0, -200.0));
        let path = graph.path();
        assert_eq!(path.elements().len(), 5);
        let bbox = path.bbox();
        assert!((bbox.x0 + 200.0).abs() < 1e-9 && (bbox.x1).abs() < 1e-9);

        let pole = FunctionGraph {
            f: |x| 1.0 / x,
            x_range: (-1.0, 1.0),
            axes: unit_axes(),
            samples: 4,
            ..FunctionGraph::default()
        };
        let moves = pole
            .path()
            .elements()
            .iter()
            .filter(|el| matches!(el, kurbo::PathEl::MoveTo(_)))
            .count();
        assert_eq!(moves, 2, "1/x is not drawn through x = 0");
    }

    #[test]
    fn parametric_curves_map_through_the_space_or_the_axes() {
        fn diagonal(t: f64) -> (f64, f64) {
            (t, t)
        }
        let space = crate::explain::Space {
            origin: (0.0, 0.0),
            unit: 100.0,
        };
        let circle = ParametricFunction {
            space,
            samples: 64,
            ..ParametricFunction::default()
        };
        let b = circle.path().bbox();
        assert!(
            (b.x0 + 100.0).abs() < 0.5 && (b.y1 - 100.0).abs() < 0.5,
            "{b:?}"
        );
        // y up: t = pi/2 is the top of the circle
        assert_eq!(circle.point(std::f64::consts::FRAC_PI_2).1.round(), -100.0);
        let on_axes = ParametricFunction {
            axes: Some(unit_axes()),
            ..circle
        };
        // data (0, -1) on axes whose box (0..4 by 0..4) is centered on the origin
        let p = on_axes.point(std::f64::consts::PI * 1.5);
        assert!(
            (p.0 + 200.0).abs() < 1e-3 && (p.1 - 300.0).abs() < 1e-3,
            "{p:?}"
        );
        let a = sample_curve(diagonal, 0.0, 1.0, 4);
        assert!(Arc::ptr_eq(&a, &sample_curve(diagonal, 0.0, 1.0, 4)));
        assert_eq!(a[4], (1.0, 1.0));
    }

    #[test]
    fn riemann_rectangles_and_area_cover_the_integral() {
        let rects = RiemannRectangles {
            f: square,
            x_range: (0.0, 2.0),
            dx: 0.5,
            axes: unit_axes(),
            ..RiemannRectangles::default()
        };
        // left sums: 0 + 0.25 + 1 + 2.25 = 3.5 units * 0.5 wide = 1.75 square units
        let area = rects.path().area().abs() / 1e4;
        assert!((area - 1.75).abs() < 1e-9, "{area}");
        assert!((rects.sum() - 1.75).abs() < 1e-12);

        let under = Area {
            f: square,
            x_range: (0.0, 2.0),
            axes: unit_axes(),
            samples: 400,
            ..Area::default()
        };
        let area = under.path().area().abs() / 1e4;
        assert!((area - 8.0 / 3.0).abs() < 1e-3, "{area}");

        let between = Area {
            f: |x| 2.0 * x,
            bounded: Some(square),
            x_range: (0.0, 2.0),
            axes: unit_axes(),
            samples: 400,
            ..Area::default()
        };
        let area = between.path().area().abs() / 1e4;
        assert!((area - 4.0 / 3.0).abs() < 1e-3, "{area}");
    }
}
