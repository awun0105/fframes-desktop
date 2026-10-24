//! Lines and regions of calculus on axes: [`Tangent`], [`Secant`], [`RiemannSum`] and
//! [`AreaUnder`].

use fframes::{Color, Svgr};
use kurbo::{Point, Vec2};

use crate::{
    anim::{RateFunc, ValueTracker},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::{Area, AxesMap, Func, RiemannRectangles},
    geometry::{Geometry, Shape},
    label::Label,
    motion::progress,
    shapes::{DashedLine, Dot, Line},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// `∫ (f - g)` from `a` to `b` (`g` is 0 when `None`) by the trapezoid rule on `n` steps.
/// Values that are not finite count as 0.
pub fn integral(f: Func, g: Option<Func>, a: f64, b: f64, n: usize) -> f64 {
    let n = n.max(1);
    let h = (b - a) / n as f64;
    let at = |x: f64| {
        let v = f(x) - g.map_or(0.0, |g| g(x));
        if v.is_finite() { v } else { 0.0 }
    };
    let inner: f64 = (1..n).map(|i| at(a + h * i as f64)).sum();
    h * (inner + f64::midpoint(at(a), at(b)))
}

/// The pixel of `(x, f(x))` and the unit direction of the graph there on screen, from a
/// central difference. `None` where `f` is not finite or flat in pixels.
fn direction_at(f: Func, map: &AxesMap, x: f64) -> Option<(Point, Vec2)> {
    let y = f(x);
    let h = ((map.x.max - map.x.min).abs() * 1e-4).max(1e-9);
    let (a, b) = (map.c2p(x - h, f(x - h)), map.c2p(x + h, f(x + h)));
    let d = b - a;
    let length = d.hypot();
    (y.is_finite() && length.is_finite() && length > 1e-12).then(|| (map.c2p(x, y), d / length))
}

fn line_shape(ui: &Ui<'_, '_, '_>, a: Point, b: Point, color: Color, width: f32) -> Shape {
    Line {
        x1: a.x as f32,
        y1: a.y as f32,
        x2: b.x as f32,
        y2: b.y as f32,
        stroke: Some(color),
        stroke_width: width,
        ..ui.defaults()
    }
    .shape()
}

fn dot_shape(ui: &Ui<'_, '_, '_>, p: Point, radius: f32, color: Color) -> Shape {
    Dot {
        cx: p.x as f32,
        cy: p.y as f32,
        radius,
        fill: Some(color),
        ..ui.defaults()
    }
    .shape()
}

/// The tangent of `f` at `x`, `length` pixels long and centered on the point, with a dot
/// on the point when `dot` is above 0 (Manim's `TangentLine`). `x` is a
/// [`ValueTracker`], so the line can slide along the graph.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Tangent<'a> {
    pub f: Func,
    pub axes: AxesMap,
    pub x: ValueTracker<'a>,
    pub length: f32,
    pub color: Color,
    pub stroke_width: f32,
    pub dot: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
}

impl Themed for Tangent<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            f: |x| x,
            axes: AxesMap::default(),
            x: 0.0.into(),
            length: 400.0,
            color: t.colors.highlight,
            stroke_width: t.stroke.regular,
            dot: 10.0,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Tangent<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Tangent<'_> {
    /// The ends of the line when the tracker is at `x`.
    fn ends(&self, x: f64) -> Option<(Point, Point, Point)> {
        let (p, d) = direction_at(self.f, &self.axes, x)?;
        let half = d * f64::from(self.length) / 2.0;
        Some((p - half, p + half, p))
    }
}

impl<'a> Component<'a> for Tangent<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let x = f64::from(ui.sample(&self.x));
        let Some((a, b, p)) = self.ends(x) else {
            return Svgr::empty();
        };
        let mut parts = vec![line_shape(ui, a, b, self.color, self.stroke_width)];
        if self.dot > 0.0 {
            parts.push(dot_shape(ui, p, self.dot, self.color));
        }
        draw_animated(ui, &parts, self.enter.as_ref(), self.exit.as_ref(), None)
    }
}

/// The secant of `f` through `x` and `x + dx`, extended `extend` pixels past both points,
/// with dots on them and, when `legs` is set, the dashed `dx` and `dy` legs (Manim's
/// `get_secant_slope_group`). As `dx` goes to 0 it turns into the tangent.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Secant<'a> {
    pub f: Func,
    pub axes: AxesMap,
    pub x: ValueTracker<'a>,
    pub dx: ValueTracker<'a>,
    pub extend: f32,
    pub legs: bool,
    pub color: Color,
    pub legs_color: Color,
    pub stroke_width: f32,
    pub dot: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
}

impl Themed for Secant<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            f: |x| x,
            axes: AxesMap::default(),
            x: 0.0.into(),
            dx: 1.0.into(),
            extend: 120.0,
            legs: true,
            color: t.colors.highlight,
            legs_color: t.colors.muted,
            stroke_width: t.stroke.regular,
            dot: 10.0,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Secant<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The two points of a secant and the ends of its line.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SecantPoints {
    p0: Point,
    p1: Point,
    from: Point,
    to: Point,
}

impl Secant<'_> {
    fn points(&self, x: f64, dx: f64) -> Option<SecantPoints> {
        let (f, map) = (self.f, &self.axes);
        let (y0, y1) = (f(x), f(x + dx));
        if !(y0.is_finite() && y1.is_finite()) {
            return None;
        }
        let (p0, p1) = (map.c2p(x, y0), map.c2p(x + dx, y1));
        let d = p1 - p0;
        // the points meet: the secant has become the tangent
        let u = if d.hypot() > 1e-6 {
            d / d.hypot()
        } else {
            direction_at(f, map, x)?.1
        };
        // p1 lies ahead of p0 along u
        let extend = u * f64::from(self.extend);
        Some(SecantPoints {
            p0,
            p1,
            from: p0 - extend,
            to: p1 + extend,
        })
    }
}

impl<'a> Component<'a> for Secant<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (x, dx) = (
            f64::from(ui.sample(&self.x)),
            f64::from(ui.sample(&self.dx)),
        );
        let Some(s) = self.points(x, dx) else {
            return Svgr::empty();
        };
        let mut parts = Vec::new();
        if self.legs {
            let corner = Point::new(s.p1.x, s.p0.y);
            for (a, b) in [(s.p0, corner), (corner, s.p1)] {
                parts.push(
                    DashedLine {
                        x1: a.x as f32,
                        y1: a.y as f32,
                        x2: b.x as f32,
                        y2: b.y as f32,
                        stroke: Some(self.legs_color),
                        stroke_width: self.stroke_width / 2.0,
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
        }
        parts.push(line_shape(ui, s.from, s.to, self.color, self.stroke_width));
        if self.dot > 0.0 {
            parts.push(dot_shape(ui, s.p0, self.dot, self.color));
            parts.push(dot_shape(ui, s.p1, self.dot, self.color));
        }
        draw_animated(ui, &parts, self.enter.as_ref(), self.exit.as_ref(), None)
    }
}

/// The step of a list of `steps` shown `t` seconds in, and how far (0 to 1) its cross-fade
/// to the next step is. Each step holds `hold` seconds and then fades over `crossfade`.
fn stage(t: f32, steps: usize, hold: f32, crossfade: f32) -> (usize, f32) {
    if steps == 0 {
        return (0, 0.0);
    }
    let period = (hold + crossfade).max(f32::EPSILON);
    let t = t.max(0.0);
    let i = ((t / period).floor() as usize).min(steps - 1);
    if i + 1 == steps {
        return (i, 0.0);
    }
    let local = t - i as f32 * period;
    (i, progress(local, hold, hold + crossfade))
}

/// Riemann rectangles that get finer: `rects` is drawn with each width of `dxs` in turn,
/// `hold` seconds each, cross-fading to the next over `crossfade` seconds, from `delay`.
/// `sum` writes the Riemann sum after its text with `decimals`, fading with the
/// rectangles. Build `rects` with [`Axes::riemann_rectangles`](crate::explain::Axes::riemann_rectangles).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct RiemannSum<'a> {
    pub rects: RiemannRectangles,
    pub dxs: Vec<f64>,
    pub delay: f32,
    pub hold: f32,
    pub crossfade: f32,
    pub sum: Option<Label<'a>>,
    pub decimals: usize,
}

impl Themed for RiemannSum<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            rects: RiemannRectangles::themed(t),
            dxs: vec![0.5, 0.25, 0.1],
            delay: 0.0,
            hold: 1.0,
            crossfade: t.motion.duration.base,
            sum: None,
            decimals: 3,
        }
    }
}

impl Default for RiemannSum<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for RiemannSum<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (i, fade) = stage(
            ui.seconds() - self.delay,
            self.dxs.len(),
            self.hold,
            self.crossfade,
        );
        let mut drawn = Vec::new();
        for (step, opacity) in [(i, 1.0 - fade), (i + 1, fade)] {
            let Some(&dx) = self.dxs.get(step) else {
                continue;
            };
            if opacity <= 0.0 {
                continue;
            }
            let rects = RiemannRectangles {
                dx,
                ..self.rects.clone()
            };
            let value = rects.sum();
            let shapes = ui.show(rects);
            let sum = match &self.sum {
                Some(label) => ui.show(Label {
                    text: format!("{}{value:.d$}", label.text, d = self.decimals).into(),
                    ..label.clone()
                }),
                None => Svgr::empty(),
            };
            drawn.push(fframes::svgr!(<g opacity={opacity}>{shapes}{sum}</g>));
        }
        drawn.into()
    }
}

/// The area under a graph revealed from left to right over `run_time` seconds after
/// `delay` (Manim's `get_area` with a growing `x_range`), with the integral of the part
/// shown so far written after the text of `value`. Build `area` with
/// [`Axes::area`](crate::explain::Axes::area); `key` tells the clip paths of two areas in
/// one frame apart.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct AreaUnder<'a> {
    pub area: Area,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
    pub value: Option<Label<'a>>,
    pub decimals: usize,
    pub key: Option<&'a str>,
}

impl Themed for AreaUnder<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            area: Area::themed(t),
            delay: 0.0,
            run_time: 1.5,
            rate_func: RateFunc::default(),
            value: None,
            decimals: 3,
            key: None,
        }
    }
}

impl Default for AreaUnder<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl AreaUnder<'_> {
    /// How much of the range is shown `t` seconds in, from 0 to 1.
    fn shown(&self, t: f32) -> f32 {
        let p = progress(t, self.delay, self.delay + self.run_time);
        self.rate_func.apply(p, self.run_time)
    }
}

impl<'a> Component<'a> for AreaUnder<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let shown = f64::from(self.shown(ui.seconds()));
        let (a, b) = self.area.x_range;
        let x = a + (b - a) * shown;
        let value = match &self.value {
            Some(label) => {
                let v = integral(self.area.f, self.area.bounded, a, x, 256);
                ui.show(Label {
                    text: format!("{}{v:.d$}", label.text, d = self.decimals).into(),
                    ..label.clone()
                })
            }
            None => Svgr::empty(),
        };
        if shown <= 0.0 {
            return value;
        }
        let bounds = self.area.bbox().inflate(4.0, 4.0);
        let (from, to) = (self.area.axes.c2p(a, 0.0).x, self.area.axes.c2p(x, 0.0).x);
        let (left, width) = (from.min(to) as f32, (to - from).abs() as f32);
        let (top, height) = (bounds.y0 as f32, bounds.height() as f32);
        let id = ui.id(self.key, 0, "area-under");
        let url = format!("url(#{id})");
        let area = ui.show(self.area);
        fframes::svgr!(
            <g>
                <defs>
                    <clipPath id={id}>
                        <rect x={left} y={top} width={width} height={height} />
                    </clipPath>
                </defs>
                <g clip-path={url}>{area}</g>
                {value}
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explain::{Range, Space};

    fn square(x: f64) -> f64 {
        x * x
    }

    /// Data `(x, y)` is pixel `(100 x, -100 y)`.
    fn unit_axes() -> AxesMap {
        AxesMap {
            x: Range::new(0.0, 4.0, 1.0),
            y: Range::new(0.0, 4.0, 1.0),
            x_length: 4.0,
            y_length: 4.0,
            center: (2.0, 2.0),
            space: Space {
                origin: (0.0, 0.0),
                unit: 100.0,
            },
        }
    }

    #[test]
    fn integrals_match_their_antiderivatives() {
        assert!((integral(square, None, 0.0, 2.0, 256) - 8.0 / 3.0).abs() < 1e-4);
        let between = integral(|x| 2.0 * x, Some(square), 0.0, 2.0, 256);
        assert!((between - 4.0 / 3.0).abs() < 1e-4);
        assert_eq!(integral(square, None, 1.0, 1.0, 16), 0.0);
        assert_eq!(integral(|x| 1.0 / x, None, 0.0, 0.0, 4), 0.0);
    }

    #[test]
    fn the_tangent_has_the_slope_of_the_graph() {
        let tangent = Tangent {
            f: square,
            axes: unit_axes(),
            length: 200.0,
            ..Tangent::default()
        };
        let (a, b, p) = tangent.ends(1.0).unwrap();
        assert!((p - Point::new(100.0, -100.0)).hypot() < 1e-9);
        // slope 2: 1 right, 2 up, on screen y down
        let d = b - a;
        assert!((d.y / d.x + 2.0).abs() < 1e-4, "{d:?}");
        assert!((d.hypot() - 200.0).abs() < 1e-6);
        assert!(
            Tangent {
                f: |x| 1.0 / x,
                ..tangent
            }
            .ends(0.0)
            .is_none()
        );
    }

    #[test]
    fn a_vanishing_secant_becomes_the_tangent() {
        let secant = Secant {
            f: square,
            axes: unit_axes(),
            extend: 50.0,
            ..Secant::default()
        };
        let s = secant.points(1.0, 1.0).unwrap();
        assert_eq!(
            (s.p0, s.p1),
            (Point::new(100.0, -100.0), Point::new(200.0, -400.0))
        );
        // slope 3, extended 50 px past both points
        assert!(((s.to - s.p1).hypot() - 50.0).abs() < 1e-9);
        assert!(((s.p0 - s.from).hypot() - 50.0).abs() < 1e-9);
        let flat = secant.points(1.0, 0.0).unwrap();
        let d = flat.to - flat.from;
        assert!((d.y / d.x + 2.0).abs() < 1e-4, "{d:?}");
        // a negative dx gives the same line from the other side
        let back = secant.points(2.0, -1.0).unwrap();
        assert!((back.from - s.to).hypot() < 1e-9 && (back.to - s.from).hypot() < 1e-9);
    }

    #[test]
    fn steps_hold_then_cross_fade() {
        assert_eq!(stage(0.5, 3, 1.0, 0.5), (0, 0.0));
        assert_eq!(stage(1.25, 3, 1.0, 0.5), (0, 0.5));
        assert_eq!(stage(1.6, 3, 1.0, 0.5), (1, 0.0));
        assert_eq!(stage(99.0, 3, 1.0, 0.5), (2, 0.0));
        assert_eq!(stage(-1.0, 3, 1.0, 0.5), (0, 0.0));
        assert_eq!(stage(f32::NAN, 3, 1.0, 0.5), (0, 0.0));
        assert_eq!(stage(1.0, 0, 1.0, 0.5), (0, 0.0));
    }

    #[test]
    fn the_area_grows_with_the_sweep() {
        let under = AreaUnder {
            delay: 1.0,
            run_time: 2.0,
            rate_func: RateFunc::Linear,
            ..AreaUnder::default()
        };
        assert_eq!(under.shown(0.5), 0.0);
        assert_eq!(under.shown(2.0), 0.5);
        assert_eq!(under.shown(9.0), 1.0);
    }
}
