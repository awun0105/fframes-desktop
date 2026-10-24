use std::f64::consts::PI;

use fframes::{Color, Svgr, animation::Animatable};
use kurbo::{Affine, BezPath, PathEl, Point, Rect, Vec2};

use crate::{
    anim::RateFunc,
    geometry::{Geometry, Look, Shape},
    motion::lerp,
    paths,
    ui::{Component, Ui},
};

use super::{
    EnterAnim, ShapeAnimation, Transform, about,
    create::themed,
    enter_alpha, exit_alpha, grow_scale,
    transform::{MORPH_SEGMENTS, lerp_paint, morph},
};

/// Manim's `path_along_arc`: where a point going from `s` to `e` on an arc of `arc` radians
/// is at `t`. A positive arc turns counterclockwise on screen; below 0.01 the path is
/// straight, and at a half turn the arc's center is the middle of `s` and `e`.
pub(crate) fn arc_point(s: Point, e: Point, arc: f64, t: f64) -> Point {
    if arc.abs() < 0.01 {
        return s.lerp(e, t);
    }
    // Manim's frame, y up
    let (s, e) = (Vec2::new(s.x, -s.y), Vec2::new(e.x, -e.y));
    let v = e - s;
    let mut center = s + v * 0.5;
    if (arc.abs() - PI).abs() > 1e-9 {
        center += Vec2::new(-v.y, v.x) / 2.0 / (arc / 2.0).tan();
    }
    let (sin, cos) = (arc * t).sin_cos();
    let r = s - center;
    let p = center + Vec2::new(r.x * cos - r.y * sin, r.x * sin + r.y * cos);
    Point::new(p.x, -p.y)
}

/// [`paths::lerp_aligned`] with every point moving on an arc of `arc` radians.
pub(crate) fn lerp_along_arc(t: f64, a: &BezPath, b: &BezPath, arc: f64) -> BezPath {
    let go = |p: Point, q: Point| arc_point(p, q, arc, t);
    a.elements()
        .iter()
        .zip(b.elements())
        .map(|(x, y)| match (*x, *y) {
            (PathEl::MoveTo(p), PathEl::MoveTo(q)) => PathEl::MoveTo(go(p, q)),
            (PathEl::LineTo(p), PathEl::LineTo(q)) => PathEl::LineTo(go(p, q)),
            (PathEl::QuadTo(p1, p2), PathEl::QuadTo(q1, q2)) => {
                PathEl::QuadTo(go(p1, q1), go(p2, q2))
            }
            (PathEl::CurveTo(p1, p2, p3), PathEl::CurveTo(q1, q2, q3)) => {
                PathEl::CurveTo(go(p1, q1), go(p2, q2), go(p3, q3))
            }
            (x, _) => x,
        })
        .collect()
}

/// `morph` with the points moving on an arc.
pub(crate) fn morph_on_arc(
    from: &Shape,
    to: &Shape,
    a: &BezPath,
    b: &BezPath,
    t: f32,
    arc: f64,
) -> Shape {
    Shape {
        path: lerp_along_arc(f64::from(t), a, b, arc),
        ..morph(from, to, a, b, t)
    }
}

/// `morph` on an arc of `arc` radians from `from` to `to` at the progress of the timing.
fn render_arc_morph<'a>(
    ui: &Ui<'a, '_, '_>,
    from: &Shape,
    to: &Shape,
    arc: f64,
    (delay, run_time, rate_func): (f32, f32, RateFunc),
) -> Svgr<'a> {
    let p = enter_alpha(ui, delay, run_time, rate_func);
    if p <= 0.0 {
        return from.draw();
    }
    let (a, b) = paths::align_points(&from.path, &to.path, MORPH_SEGMENTS);
    morph_on_arc(from, to, &a, &b, p, arc).draw()
}

/// A morph of `from` into `to` while `from` stays where it is (Manim's
/// `TransformFromCopy`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct TransformFromCopy {
    pub from: Shape,
    pub to: Shape,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl TransformFromCopy {
    /// From `from` to `to` in Manim's 1 second, smooth.
    pub fn new(from: Shape, to: Shape) -> Self {
        Self {
            from,
            to,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for TransformFromCopy {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let original = self.from.draw();
        let copy = Transform {
            from: self.from,
            to: self.to,
            delay: self.delay,
            run_time: self.run_time,
            rate_func: self.rate_func,
        }
        .render(ui);
        fframes::svgr!(<g>{original}{copy}</g>)
    }
}

/// A morph whose points travel clockwise on arcs of `path_arc` radians (Manim's
/// `ClockwiseTransform`, a half turn).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ClockwiseTransform {
    pub from: Shape,
    pub to: Shape,
    /// Negative turns clockwise.
    pub path_arc: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ClockwiseTransform {
    /// From `from` to `to` on half turns in Manim's 1 second, smooth.
    pub fn new(from: Shape, to: Shape) -> Self {
        Self {
            from,
            to,
            path_arc: -std::f32::consts::PI,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for ClockwiseTransform {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let timing = (self.delay, self.run_time, self.rate_func);
        render_arc_morph(ui, &self.from, &self.to, f64::from(self.path_arc), timing)
    }
}

/// A morph whose points travel counterclockwise on arcs of `path_arc` radians (Manim's
/// `CounterclockwiseTransform`, a half turn).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct CounterclockwiseTransform {
    pub from: Shape,
    pub to: Shape,
    /// Positive turns counterclockwise.
    pub path_arc: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl CounterclockwiseTransform {
    /// From `from` to `to` on half turns in Manim's 1 second, smooth.
    pub fn new(from: Shape, to: Shape) -> Self {
        Self {
            from,
            to,
            path_arc: std::f32::consts::PI,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for CounterclockwiseTransform {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let timing = (self.delay, self.run_time, self.rate_func);
        render_arc_morph(ui, &self.from, &self.to, f64::from(self.path_arc), timing)
    }
}

/// Every shape moves on an arc to where the next one is, the last to the first (Manim's
/// `CyclicReplace`, arcs of a quarter turn). The shapes keep their own paths.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct CyclicReplace {
    pub shapes: Vec<Shape>,
    pub path_arc: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl CyclicReplace {
    /// Manim's quarter turns in 1 second, smooth.
    pub fn new(shapes: Vec<Shape>) -> Self {
        Self {
            shapes,
            path_arc: std::f32::consts::FRAC_PI_2,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }

    /// How far shape `i` has moved at progress `p`.
    pub fn offset(&self, i: usize, p: f32) -> Vec2 {
        let n = self.shapes.len();
        if n < 2 {
            return Vec2::ZERO;
        }
        let from = self.shapes[i].bbox().center();
        let to = self.shapes[(i + 1) % n].bbox().center();
        arc_point(from, to, f64::from(self.path_arc), f64::from(p)) - from
    }
}

impl<'a> Component<'a> for CyclicReplace {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        self.shapes
            .iter()
            .enumerate()
            .map(|(i, shape)| {
                let mut look = Look::default();
                if p > 0.0 {
                    look.then(Affine::translate(self.offset(i, p)));
                }
                shape.draw_with(&look)
            })
            .collect()
    }
}

/// Two shapes trade places on arcs (Manim's `Swap`, a [`CyclicReplace`] of two).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Swap {
    pub first: Shape,
    pub second: Shape,
    pub path_arc: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Swap {
    /// Manim's quarter turns in 1 second, smooth.
    pub fn new(first: Shape, second: Shape) -> Self {
        Self {
            first,
            second,
            path_arc: std::f32::consts::FRAC_PI_2,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for Swap {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        CyclicReplace {
            shapes: vec![self.first, self.second],
            path_arc: self.path_arc,
            delay: self.delay,
            run_time: self.run_time,
            rate_func: self.rate_func,
        }
        .render(ui)
    }
}

/// The map that takes box `a` onto box `b`: stretched to fit, or scaled by the height only.
/// A box without width or height counts as 1 pixel wide or high.
fn box_map(a: Rect, b: Rect, stretch: bool) -> Affine {
    const MIN: f64 = 1.0;
    let (sx, sy) = (
        b.width().max(MIN) / a.width().max(MIN),
        b.height().max(MIN) / a.height().max(MIN),
    );
    let scale = if stretch {
        Affine::scale_non_uniform(sx, sy)
    } else {
        Affine::scale(sy)
    };
    Affine::translate(b.center().to_vec2()) * scale * Affine::translate(-a.center().to_vec2())
}

fn lerp_affine(a: Affine, b: Affine, t: f64) -> Affine {
    let (a, b) = (a.as_coeffs(), b.as_coeffs());
    Affine::new(std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t))
}

/// `from` and `to` while one fades into the other: `from` moves and stretches onto `to`'s
/// box as it fades out, `to` comes from `from`'s box as it fades in.
fn fade_pair<'a>(from: &Shape, to: &Shape, stretch: bool, p: f32) -> Svgr<'a> {
    let there = box_map(from.bbox(), to.bbox(), stretch);
    let back = box_map(to.bbox(), from.bbox(), stretch);
    let t = f64::from(p);
    let leaving = Look {
        transform: lerp_affine(Affine::IDENTITY, there, t),
        opacity: 1.0 - p,
        ..Look::default()
    };
    let coming = Look {
        transform: lerp_affine(back, Affine::IDENTITY, t),
        opacity: p,
        ..Look::default()
    };
    let (a, b) = (from.draw_with(&leaving), to.draw_with(&coming));
    fframes::svgr!(<g>{a}{b}</g>)
}

/// `from` fades out while it moves onto `to`, and `to` fades in from where `from` was
/// (Manim's `FadeTransform`). With `stretch` each takes the other's box exactly; without,
/// its height.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct FadeTransform {
    pub from: Shape,
    pub to: Shape,
    pub stretch: bool,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl FadeTransform {
    /// Stretched, in Manim's 1 second, smooth.
    pub fn new(from: Shape, to: Shape) -> Self {
        Self {
            from,
            to,
            stretch: true,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for FadeTransform {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return self.from.draw();
        }
        if p >= 1.0 {
            return self.to.draw();
        }
        fade_pair(&self.from, &self.to, self.stretch, p)
    }
}

/// `FadeTransform` piece by piece (Manim's `FadeTransformPieces`): piece `i` of `from` goes
/// to piece `i` of `to`. Manim splits pieces until the counts match; here the pieces over
/// the shorter list fade out or in where they are.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct FadeTransformPieces {
    pub from: Vec<Shape>,
    pub to: Vec<Shape>,
    pub stretch: bool,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl FadeTransformPieces {
    /// Stretched, in Manim's 1 second, smooth.
    pub fn new(from: Vec<Shape>, to: Vec<Shape>) -> Self {
        Self {
            from,
            to,
            stretch: true,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

/// `shape` at `opacity` (1 draws it as built).
fn faded<'a>(shape: &Shape, opacity: f32) -> Svgr<'a> {
    shape.draw_with(&Look {
        opacity,
        ..Look::default()
    })
}

impl<'a> Component<'a> for FadeTransformPieces {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return self.from.iter().map(Shape::draw).collect();
        }
        if p >= 1.0 {
            return self.to.iter().map(Shape::draw).collect();
        }
        let paired = self.from.len().min(self.to.len());
        let pairs = (0..paired).map(|i| fade_pair(&self.from[i], &self.to[i], self.stretch, p));
        let leaving = self.from[paired..].iter().map(|s| faded(s, 1.0 - p));
        let coming = self.to[paired..].iter().map(|s| faded(s, p));
        pairs.chain(leaving).chain(coming).collect()
    }
}

/// Manim's `get_mobject_key`: the path's points with its box centered and its height 1
/// (its width when it has no height), rounded to three places.
pub fn shape_key(path: &BezPath) -> Vec<i64> {
    let b = kurbo::Shape::bounding_box(path);
    let size = if b.height() > 1e-9 {
        b.height()
    } else {
        b.width()
    };
    let scale = if size > 1e-9 { 1.0 / size } else { 1.0 };
    let c = b.center();
    let mut key = Vec::new();
    let push = |key: &mut Vec<i64>, p: Point| {
        key.push(((p.x - c.x) * scale * 1000.0).round() as i64);
        key.push(((p.y - c.y) * scale * 1000.0).round() as i64);
    };
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => push(&mut key, p),
            PathEl::QuadTo(p1, p2) => {
                push(&mut key, p1);
                push(&mut key, p2);
            }
            PathEl::CurveTo(p1, p2, p3) => {
                push(&mut key, p1);
                push(&mut key, p2);
                push(&mut key, p3);
            }
            PathEl::ClosePath => key.push(i64::MIN),
        }
    }
    key
}

/// Whether two [`shape_key`]s are the same shape: equal up to one step of the rounding, so a
/// glyph outline that lands a hair differently at another position still matches.
pub fn same_shape(a: &[i64], b: &[i64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.abs_diff(*y) <= 1)
}

/// For every piece of `to`, the piece of `from` with the same [`shape_key`] (by
/// [`same_shape`]), in order; each piece of `from` is used once.
pub fn match_shapes(from: &[Shape], to: &[Shape]) -> Vec<Option<usize>> {
    let from_keys: Vec<Vec<i64>> = from.iter().map(|s| shape_key(&s.path)).collect();
    let mut used = vec![false; from.len()];
    to.iter()
        .map(|t| {
            let key = shape_key(&t.path);
            let i = (0..from.len()).find(|&i| !used[i] && same_shape(&from_keys[i], &key))?;
            used[i] = true;
            Some(i)
        })
        .collect()
}

/// Pieces of `from` that have the same shape as pieces of `to` move there, the others fade
/// out while the rest of `to` fades in (Manim's `TransformMatchingShapes`). Shapes match
/// by [`shape_key`], as in Manim, wherever they are and whatever their size. Manim
/// transforms all pieces of one key as a group; here they pair in order.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct TransformMatchingShapes {
    pub from: Vec<Shape>,
    pub to: Vec<Shape>,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl TransformMatchingShapes {
    /// In Manim's 1 second, smooth.
    pub fn new(from: Vec<Shape>, to: Vec<Shape>) -> Self {
        Self {
            from,
            to,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for TransformMatchingShapes {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return self.from.iter().map(Shape::draw).collect();
        }
        if p >= 1.0 {
            return self.to.iter().map(Shape::draw).collect();
        }
        let matches = match_shapes(&self.from, &self.to);
        let mut matched = vec![false; self.from.len()];
        let mut drawn: Vec<Svgr<'a>> = Vec::new();
        for (to, m) in self.to.iter().zip(&matches) {
            match *m {
                Some(i) => {
                    matched[i] = true;
                    let from = &self.from[i];
                    let (a, b) = paths::align_points(&from.path, &to.path, MORPH_SEGMENTS);
                    drawn.push(morph(from, to, &a, &b, p).draw());
                }
                None => drawn.push(faded(to, p)),
            }
        }
        for (from, _) in self.from.iter().zip(matched).filter(|(_, m)| !m) {
            drawn.push(faded(from, 1.0 - p));
        }
        drawn.into_iter().collect()
    }
}

/// The look `t` of the way from `a` (on `from`) to `b` (on `to`).
fn lerp_look(a: &Look, b: &Look, from: &Shape, to: &Shape, t: f32) -> Look {
    let paint = |x: Option<Color>, y: Option<Color>, fx: Option<Color>, fy: Option<Color>| {
        if x.is_none() && y.is_none() {
            None
        } else {
            lerp_paint(x.or(fx), y.or(fy), t)
        }
    };
    Look {
        transform: lerp_affine(a.transform, b.transform, f64::from(t)),
        opacity: lerp(a.opacity, b.opacity, t),
        fill_opacity: lerp(a.fill_opacity, b.fill_opacity, t),
        stroke_opacity: lerp(a.stroke_opacity, b.stroke_opacity, t),
        drawn: (lerp(a.drawn.0, b.drawn.0, t), lerp(a.drawn.1, b.drawn.1, t)),
        fill: paint(a.fill, b.fill, from.fill, to.fill),
        stroke: paint(a.stroke, b.stroke, from.stroke, to.stroke),
        stroke_width: match (a.stroke_width, b.stroke_width) {
            (None, None) => None,
            (x, y) => Some(lerp(
                x.unwrap_or(from.stroke_width),
                y.unwrap_or(to.stroke_width),
                t,
            )),
        },
    }
}

/// Runs `from_anim` on `from` and `to_anim` on `to` at once while one shape morphs into the
/// other (Manim's `TransformAnimations`): the shape and its look go from the first
/// animation's to the second's over `run_time`.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformAnimations {
    pub from: Shape,
    pub from_anim: EnterAnim,
    pub to: Shape,
    pub to_anim: EnterAnim,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl TransformAnimations {
    /// In Manim's 1 second, smooth; time the two animations to run over the same second.
    pub fn new(from: Shape, from_anim: EnterAnim, to: Shape, to_anim: EnterAnim) -> Self {
        Self {
            from,
            from_anim,
            to,
            to_anim,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for TransformAnimations {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let (mut a, mut b) = (Look::default(), Look::default());
        self.from_anim.look(ui, &self.from, &mut a);
        self.to_anim.look(ui, &self.to, &mut b);
        let (pa, pb) = paths::align_points(&self.from.path, &self.to.path, MORPH_SEGMENTS);
        let shape = morph(&self.from, &self.to, &pa, &pb, p);
        shape.draw_with(&lerp_look(&a, &b, &self.from, &self.to, p))
    }
}

/// Turns the shape's fill and outline into `color` and keeps them (Manim's `FadeToColor`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FadeToColor {
    pub color: Color,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(FadeToColor, |t| FadeToColor {
    color: t.colors.highlight,
    delay: 0.0,
    run_time: 1.0,
    rate_func: RateFunc::Smooth,
});

impl ShapeAnimation for FadeToColor {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return;
        }
        look.fill = shape.fill.map(|c| c.apply_progress(&self.color, p));
        look.stroke = shape.stroke.map(|c| c.apply_progress(&self.color, p));
    }
}

/// Scales the shape by `scale_factor` about its center and keeps it so (Manim's
/// `ScaleInPlace`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleInPlace {
    pub scale_factor: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(ScaleInPlace, |_t| ScaleInPlace {
    scale_factor: 2.0,
    delay: 0.0,
    run_time: 1.0,
    rate_func: RateFunc::Smooth,
});

impl ShapeAnimation for ScaleInPlace {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return;
        }
        let s = f64::from(lerp(1.0, self.scale_factor, p)).max(1e-3);
        look.then(about(shape.bbox().center(), s, 0.0));
    }
}

/// Shrinks the shape to nothing about its center before the end (Manim's `ShrinkToCenter`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShrinkToCenter {
    /// Seconds between the end of the animation and the end of `ui.duration`.
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(ShrinkToCenter, |t| ShrinkToCenter {
    delay: 0.0,
    run_time: 1.0,
    rate_func: t.motion.easing.standard,
});

impl ShapeAnimation for ShrinkToCenter {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let Some(p) = exit_alpha(ui, self.delay, self.run_time, self.rate_func) else {
            return;
        };
        look.then(about(shape.bbox().center(), grow_scale(1.0 - p), 0.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(d: &str) -> Shape {
        Shape {
            path: paths::parse_path(d).unwrap(),
            fill: None,
            stroke: Some(Color::WHITE),
            stroke_width: 2.0,
            dash: None,
            opacity: 1.0,
        }
    }

    #[test]
    fn counterclockwise_arcs_pass_above() {
        // from right of the middle to left of it: halfway it is above on screen
        let (s, e) = (Point::new(100.0, 0.0), Point::new(-100.0, 0.0));
        let mid = arc_point(s, e, PI, 0.5);
        assert!((mid - Point::new(0.0, -100.0)).hypot() < 1e-9, "{mid:?}");
        let cw = arc_point(s, e, -PI, 0.5);
        assert!((cw - Point::new(0.0, 100.0)).hypot() < 1e-9, "{cw:?}");
        // ends where it should, on any arc
        for arc in [0.0, 0.5, PI / 2.0, -2.0] {
            assert!((arc_point(s, e, arc, 1.0) - e).hypot() < 1e-9);
            assert!((arc_point(s, e, arc, 0.0) - s).hypot() < 1e-9);
        }
    }

    #[test]
    fn identical_shapes_pair_wherever_and_however_large() {
        let square = shape("M0 0 L10 0 L10 10 L0 10 Z");
        let moved_bigger = shape("M100 100 L130 100 L130 130 L100 130 Z");
        let circle = Shape {
            path: kurbo::Shape::to_path(&kurbo::Circle::new((5.0, 5.0), 5.0), 0.1),
            ..square.clone()
        };
        let line = shape("M0 0 L50 0");
        let longer_line = shape("M0 20 L80 20");
        let pairs = match_shapes(
            &[circle.clone(), square.clone(), line],
            &[moved_bigger, circle, longer_line, square],
        );
        assert_eq!(pairs, vec![Some(1), Some(0), Some(2), None]);
    }

    #[test]
    fn keys_a_hair_apart_still_match() {
        let a = shape("M0 0 L10 0 L10 10 L0 10 Z");
        let b = shape("M700.00004 0 L710 0 L710 10.00004 L700 10 Z");
        let off = shape("M0 0 L10 0 L10 10 L0 10.4 Z");
        assert!(same_shape(&shape_key(&a.path), &shape_key(&b.path)));
        assert_eq!(match_shapes(std::slice::from_ref(&a), &[off]), vec![None]);
        assert_eq!(match_shapes(&[a], &[b]), vec![Some(0)]);
    }

    #[test]
    fn box_map_survives_flat_boxes() {
        let line = shape("M0 0 L50 0").bbox();
        let dot = Rect::new(10.0, 10.0, 10.0, 10.0);
        let m = box_map(line, dot, true);
        assert!(m.as_coeffs().iter().all(|c| c.is_finite()));
        let p = m * line.center();
        assert!((p - dot.center()).hypot() < 1e-9);
    }
}
