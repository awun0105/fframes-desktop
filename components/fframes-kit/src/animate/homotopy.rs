use fframes::Svgr;
use kurbo::{BezPath, PathEl, Point};

use crate::{
    anim::RateFunc,
    explain::Space,
    geometry::Shape,
    paths,
    ui::{Component, Ui},
};

use super::enter_alpha;

/// About how many cubic pieces a homotopy cuts a path into before it maps the points; the
/// cost of a frame grows with it.
pub const HOMOTOPY_SEGMENTS: usize = 256;

/// The most Runge-Kutta steps a [`PhaseFlow`] takes per point and frame to follow its field.
pub const PHASE_FLOW_STEPS: usize = 120;

/// `path` cut into about [`HOMOTOPY_SEGMENTS`] pieces with every point mapped through `f`,
/// which works in the units of `space` (`y` up).
fn warp_in(path: &BezPath, space: Space, f: impl Fn(f64, f64) -> (f64, f64)) -> BezPath {
    let threshold = (paths::get_length(path) / HOMOTOPY_SEGMENTS as f64).max(0.5);
    paths::warp_path(path, threshold, |p| {
        let (x, y) = space.from_px(p);
        let (x, y) = f(x, y);
        space.to_px(x, y)
    })
}

/// The path with new handles through the same points (Catmull-Rom), smooth where the
/// pieces meet.
fn smoothed(path: &BezPath) -> BezPath {
    let mut out = BezPath::new();
    for sub in paths::get_subpaths(path) {
        let mut anchors = Vec::new();
        let mut closed = false;
        for el in sub.elements() {
            match *el {
                PathEl::MoveTo(p)
                | PathEl::LineTo(p)
                | PathEl::QuadTo(_, p)
                | PathEl::CurveTo(_, _, p) => {
                    anchors.push(p);
                }
                PathEl::ClosePath => closed = true,
            }
        }
        if closed && anchors.len() > 1 && anchors.first() == anchors.last() {
            anchors.pop();
        }
        let n = anchors.len();
        if n < 2 {
            continue;
        }
        let at = |i: isize| -> Point {
            if closed {
                anchors[i.rem_euclid(n as isize) as usize]
            } else {
                anchors[i.clamp(0, n as isize - 1) as usize]
            }
        };
        out.move_to(anchors[0]);
        let pieces = if closed { n } else { n - 1 } as isize;
        for i in 0..pieces {
            let (a, b) = (at(i), at(i + 1));
            let c1 = a + (b - at(i - 1)) / 6.0;
            let c2 = b - (at(i + 2) - a) / 6.0;
            out.curve_to(c1, c2, b);
        }
        if closed {
            out.close_path();
        }
    }
    out
}

/// Moves every point of the shape through `homotopy(x, y, t)` while `t` runs from 0 to 1
/// (Manim's `Homotopy`). Points are in the units of `ui.space`, `y` up; before the start
/// the shape is at `t = 0`, after the end at `t = 1`. The path is cut into about
/// [`HOMOTOPY_SEGMENTS`] pieces first, so straight lines bend.
#[derive(Debug, Clone)]
pub struct Homotopy {
    pub shape: Shape,
    pub homotopy: fn(f64, f64, f64) -> (f64, f64),
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Homotopy {
    /// Manim's 3 seconds, smooth.
    pub fn new(shape: Shape, homotopy: fn(f64, f64, f64) -> (f64, f64)) -> Self {
        Self {
            shape,
            homotopy,
            delay: 0.0,
            run_time: 3.0,
            rate_func: RateFunc::Smooth,
        }
    }

    /// The path at progress `t`.
    pub fn path_at(&self, space: Space, t: f32) -> BezPath {
        let t = f64::from(t);
        warp_in(&self.shape.path, space, |x, y| (self.homotopy)(x, y, t))
    }
}

impl<'a> Component<'a> for Homotopy {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let path = self.path_at(ui.space, p);
        Shape { path, ..self.shape }.draw()
    }
}

/// [`Homotopy`] whose path gets smooth handles after every step (Manim's
/// `SmoothedVectorizedHomotopy`), for maps that would leave corners.
#[derive(Debug, Clone)]
pub struct SmoothedVectorizedHomotopy {
    pub homotopy: Homotopy,
}

impl<'a> Component<'a> for SmoothedVectorizedHomotopy {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let h = self.homotopy;
        let p = enter_alpha(ui, h.delay, h.run_time, h.rate_func);
        let path = smoothed(&h.path_at(ui.space, p));
        Shape { path, ..h.shape }.draw()
    }
}

/// [`Homotopy`] on complex numbers: `homotopy((re, im), t)` gives where the point `re + i im`
/// is at `t` (Manim's `ComplexHomotopy`), in the units of `ui.space`.
#[derive(Debug, Clone)]
pub struct ComplexHomotopy {
    pub shape: Shape,
    pub homotopy: fn((f64, f64), f64) -> (f64, f64),
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ComplexHomotopy {
    /// Manim's 3 seconds, smooth.
    pub fn new(shape: Shape, homotopy: fn((f64, f64), f64) -> (f64, f64)) -> Self {
        Self {
            shape,
            homotopy,
            delay: 0.0,
            run_time: 3.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for ComplexHomotopy {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = f64::from(enter_alpha(ui, self.delay, self.run_time, self.rate_func));
        let path = warp_in(&self.shape.path, ui.space, |x, y| {
            (self.homotopy)((x, y), p)
        });
        Shape { path, ..self.shape }.draw()
    }
}

/// Lets every point of the shape flow along the field `function(x, y) -> (dx, dy)` for
/// `virtual_time` time units by the end (Manim's `PhaseFlow`), in the units of `ui.space`.
/// The flow takes at most [`PHASE_FLOW_STEPS`] Runge-Kutta steps per point and frame.
#[derive(Debug, Clone)]
pub struct PhaseFlow {
    pub shape: Shape,
    pub function: fn(f64, f64) -> (f64, f64),
    pub virtual_time: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl PhaseFlow {
    /// One time unit in 1 second, linear, like Manim.
    pub fn new(shape: Shape, function: fn(f64, f64) -> (f64, f64)) -> Self {
        Self {
            shape,
            function,
            virtual_time: 1.0,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Linear,
        }
    }

    /// Where the point `(x, y)` has flowed after `time` time units.
    pub fn flow(&self, x: f64, y: f64, time: f64) -> (f64, f64) {
        let steps = ((time.abs() * 30.0).ceil() as usize).clamp(1, PHASE_FLOW_STEPS);
        let dt = time / steps as f64;
        let f = self.function;
        let (mut x, mut y) = (x, y);
        for _ in 0..steps {
            let k1 = f(x, y);
            let k2 = f(x + dt / 2.0 * k1.0, y + dt / 2.0 * k1.1);
            let k3 = f(x + dt / 2.0 * k2.0, y + dt / 2.0 * k2.1);
            let k4 = f(x + dt * k3.0, y + dt * k3.1);
            x += dt / 6.0 * (k1.0 + 2.0 * k2.0 + 2.0 * k3.0 + k4.0);
            y += dt / 6.0 * (k1.1 + 2.0 * k2.1 + 2.0 * k3.1 + k4.1);
        }
        (x, y)
    }
}

impl<'a> Component<'a> for PhaseFlow {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 {
            return self.shape.draw();
        }
        let time = f64::from(self.virtual_time * p);
        let path = warp_in(&self.shape.path, ui.space, |x, y| self.flow(x, y, time));
        Shape { path, ..self.shape }.draw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> Shape {
        Shape {
            path: paths::parse_path("M960 540 L1095 540").unwrap(),
            fill: None,
            stroke: None,
            stroke_width: 1.0,
            dash: None,
            opacity: 1.0,
        }
    }

    #[test]
    fn homotopy_moves_points_in_units() {
        // up by t units: at t = 1 the line is 135 px higher
        let h = Homotopy::new(line(), |x, y, t| (x, y + t));
        let b = kurbo::Shape::bounding_box(&h.path_at(Space::FULL_HD, 1.0));
        assert!(
            (b.y0 - 405.0).abs() < 1e-6 && (b.x1 - 1095.0).abs() < 1e-6,
            "{b:?}"
        );
        let still = kurbo::Shape::bounding_box(&h.path_at(Space::FULL_HD, 0.0));
        assert!((still.y0 - 540.0).abs() < 1e-6);
    }

    #[test]
    fn phase_flow_follows_the_field() {
        let flow = PhaseFlow::new(line(), |_, _| (1.0, 0.0));
        let (x, y) = flow.flow(0.0, 0.0, 2.0);
        assert!((x - 2.0).abs() < 1e-9 && y == 0.0);
        // half a turn of a rotation stays on its circle
        let turn = PhaseFlow::new(line(), |x, y| (-y, x));
        let (x, y) = turn.flow(1.5, 0.0, std::f64::consts::PI);
        assert!((x + 1.5).abs() < 1e-6 && y.abs() < 1e-6, "{x} {y}");
    }

    #[test]
    fn smoothing_keeps_the_points() {
        let square = paths::parse_path("M0 0 L10 0 L10 10 L0 10 Z").unwrap();
        let smooth = smoothed(&square);
        let ends: Vec<Point> = smooth
            .elements()
            .iter()
            .filter_map(|el| match el {
                PathEl::CurveTo(_, _, p) => Some(*p),
                _ => None,
            })
            .collect();
        assert_eq!(
            ends,
            vec![
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
                Point::new(0.0, 0.0)
            ]
        );
    }
}
