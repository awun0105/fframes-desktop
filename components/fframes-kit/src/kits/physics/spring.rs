use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::{AxesMap, Space},
    geometry::Shape,
    plot::LinePlot,
    shapes::{Dot, Rectangle},
    sim::Sim,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{RATE, clock, line, rk4};

/// A mass on a spring, stepped once: `stiffness` N/m, `mass` kg, `damping` N·s/m, let go
/// at rest `x0` meters from its rest position (`m x'' = -k x - c x'`). Build it in the
/// video's constructor and pass it to [`SpringMass`].
#[derive(Debug, Clone, PartialEq)]
pub struct SpringMotion {
    pub stiffness: f64,
    pub mass: f64,
    pub damping: f64,
    sim: Sim<[f64; 2]>,
}

impl SpringMotion {
    /// `seconds` of an undamped oscillation.
    pub fn new(stiffness: f64, mass: f64, x0: f64, seconds: f64) -> Self {
        Self::damped(stiffness, mass, 0.0, x0, seconds)
    }

    /// `seconds` of an oscillation that loses energy to `damping`.
    pub fn damped(stiffness: f64, mass: f64, damping: f64, x0: f64, seconds: f64) -> Self {
        let mass = mass.max(1e-9);
        let steps = (seconds.max(0.0) * RATE).ceil() as usize;
        let sim = Sim::precompute(1.0 / RATE, steps, [x0, 0.0], |&y, dt| {
            rk4(y, dt, |[x, v]| [v, (-stiffness * x - damping * v) / mass])
        });
        Self {
            stiffness,
            mass,
            damping,
            sim,
        }
    }

    /// The displacement in meters and the velocity in m/s after `t` seconds.
    pub fn state(&self, t: f64) -> (f64, f64) {
        let [x, v] = self.sim.sample(t);
        (x, v)
    }

    /// The kinetic energy of the mass and the energy stored in the spring, in joules.
    pub fn energy(&self, t: f64) -> (f64, f64) {
        let (x, v) = self.state(t);
        (0.5 * self.mass * v * v, 0.5 * self.stiffness * x * x)
    }

    /// The undamped period, `2π √(m / k)`.
    pub fn period(&self) -> f64 {
        std::f64::consts::TAU * (self.mass / self.stiffness.max(1e-9)).sqrt()
    }

    /// `(t, x)` from 0 to `until` seconds, `per_second` points a second, for a plot.
    pub fn trace(&self, until: f64, per_second: f64) -> Vec<(f64, f64)> {
        let until = until.clamp(0.0, self.sim.duration());
        let n = (until * per_second.max(1.0)).ceil() as usize;
        (0..=n)
            .map(|i| {
                let t = if n == 0 {
                    0.0
                } else {
                    until * i as f64 / n as f64
                };
                (t, self.state(t).0)
            })
            .collect()
    }
}

/// A block on a horizontal spring fixed to a wall at `wall` (units of `ui.space`, the middle
/// of the spring's end; `scale` units per meter), oscillating as `motion` says from `delay`
/// seconds after the local start. The block rests `rest_length` meters from the wall,
/// marked by a dashed line. With `plot`, the displacement so far is drawn on those axes
/// (time in seconds along x, meters up y) with a dot at the current value, so the
/// oscillation and its graph move together; show the axes yourself. The wall and floor draw
/// with `enter`, the rest fades with `parts_enter`. Without `motion` nothing is drawn.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct SpringMass<'a> {
    pub motion: Option<&'a SpringMotion>,
    pub wall: (f64, f64),
    pub rest_length: f64,
    pub scale: f64,
    pub delay: f32,
    pub time_scale: f32,
    pub coils: usize,
    pub block_size: f32,
    pub plot: Option<AxesMap>,
    pub color: Color,
    pub block_color: Color,
    pub plot_color: Color,
    pub guide_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for SpringMass<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            motion: None,
            wall: (-6.0, 1.5),
            rest_length: 3.0,
            scale: 1.0,
            delay: 0.0,
            time_scale: 1.0,
            coils: 10,
            block_size: 110.0,
            plot: None,
            color: t.colors.text,
            block_color: t.colors.accent,
            plot_color: t.colors.highlight,
            guide_color: t.colors.muted,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for SpringMass<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// A zigzag from `a` to `b` with `coils` turns `amplitude` pixels to each side, straight
/// for the first and last tenth.
fn zigzag(a: Point, b: Point, coils: usize, amplitude: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(a);
    let d = b - a;
    let length = d.hypot();
    if length < 1e-9 {
        return path;
    }
    let along = d / length;
    let side = Vec2::new(-along.y, along.x) * amplitude;
    let lead = length * 0.1;
    let body = length - 2.0 * lead;
    let points = coils.max(1) * 2;
    path.line_to(a + along * lead);
    for i in 0..points {
        let f = (i as f64 + 0.5) / points as f64;
        let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
        path.line_to(a + along * (lead + body * f) + side * sign);
    }
    path.line_to(b - along * lead);
    path.line_to(b);
    path
}

impl SpringMass<'_> {
    /// The pixel of the middle of the block's wall side at displacement `x`.
    fn block_side(&self, space: Space, x: f64) -> Point {
        space.to_px(
            self.wall.0 + (self.rest_length + x) * self.scale,
            self.wall.1,
        )
    }
}

impl<'a> Component<'a> for SpringMass<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(motion) = self.motion else {
            return Svgr::empty();
        };
        let space = ui.space;
        let thin = ui.theme.stroke.thin;
        let size = f64::from(self.block_size);
        let wall = space.to_px(self.wall.0, self.wall.1);
        let floor_y = wall.y + size / 2.0;
        let floor_end = self.block_side(space, motion.state(0.0).0.abs() * 1.2).x + size * 1.6;
        let rest = self.block_side(space, 0.0);
        let mut rest_line = line(
            ui,
            Point::new(rest.x, wall.y - size),
            Point::new(rest.x, floor_y),
            self.guide_color,
            thin,
        );
        rest_line.dash = Some((8.0, 8.0));
        let frame = [
            line(
                ui,
                Point::new(wall.x, wall.y - size),
                Point::new(wall.x, floor_y),
                self.color,
                self.stroke_width,
            ),
            line(
                ui,
                Point::new(wall.x, floor_y),
                Point::new(floor_end, floor_y),
                self.color,
                self.stroke_width,
            ),
            rest_line,
        ];
        let frame = draw_animated(ui, &frame, self.enter.as_ref(), self.exit.as_ref(), None);

        let t = clock(ui, self.delay, self.time_scale);
        let (x, _) = motion.state(t);
        let side = self.block_side(space, x);
        let block = Rectangle {
            cx: (side.x + size / 2.0) as f32,
            cy: wall.y as f32,
            width: self.block_size,
            height: self.block_size,
            fill: Some(self.block_color),
            stroke: None,
            ..ui.defaults()
        };
        let spring = Shape {
            path: zigzag(wall, side, self.coils, size * 0.16),
            fill: None,
            stroke: Some(self.color),
            stroke_width: self.stroke_width,
            dash: None,
            opacity: 1.0,
        };
        let mut shapes = vec![spring, block.shape()];
        if let Some(axes) = self.plot {
            let until = t.min(axes.x.max);
            let graph = LinePlot {
                points: motion.trace(until, 60.0),
                axes,
                stroke: Some(self.plot_color),
                stroke_width: self.stroke_width,
                ..ui.defaults()
            };
            let now = axes.c2p(until, motion.state(until).0);
            shapes.push(graph.shape());
            shapes.push(
                Dot {
                    cx: now.x as f32,
                    cy: now.y as f32,
                    radius: 9.0,
                    fill: Some(self.plot_color),
                    ..ui.defaults()
                }
                .shape(),
            );
        }
        let parts = draw_animated(ui, &shapes, None, None, None);
        let parts = fade(
            ui,
            parts,
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{frame}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_oscillation_matches_the_closed_form() {
        let motion = SpringMotion::new(16.0, 1.0, 0.5, 6.0);
        // x(t) = x0 cos(ω t) with ω = √(k/m) = 4
        for i in 0..=60 {
            let t = f64::from(i) * 0.1;
            assert!(
                (motion.state(t).0 - 0.5 * (4.0 * t).cos()).abs() < 1e-4,
                "at {t}"
            );
        }
        assert!((motion.period() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        let (k, p) = motion.energy(0.0);
        assert_eq!((k, p), (0.0, 2.0));
        // between two steps the state is interpolated linearly, about 3e-4 off at the peak
        let (k, p) = motion.energy(motion.period() / 4.0);
        assert!((k - 2.0).abs() < 1e-3 && p < 1e-3, "{k} {p}");
    }

    #[test]
    fn the_trace_runs_from_zero_to_the_given_time() {
        let motion = SpringMotion::damped(10.0, 0.5, 0.3, 1.0, 4.0);
        let trace = motion.trace(2.0, 30.0);
        assert_eq!(trace.len(), 61);
        assert_eq!(trace[0], (0.0, 1.0));
        assert!((trace[60].0 - 2.0).abs() < 1e-12);
        // never past the simulated time
        assert!((motion.trace(99.0, 10.0).last().unwrap().0 - 4.0).abs() < 1e-12);
        assert_eq!(motion.trace(0.0, 30.0), vec![(0.0, 1.0)]);
    }

    #[test]
    fn the_zigzag_ends_at_both_points() {
        let path = zigzag(Point::new(0.0, 0.0), Point::new(100.0, 0.0), 4, 10.0);
        let els = path.elements();
        assert_eq!(els.len(), 1 + 1 + 8 + 2);
        assert!(
            matches!(els.last(), Some(kurbo::PathEl::LineTo(p)) if *p == Point::new(100.0, 0.0))
        );
        let bbox = kurbo::Shape::bounding_box(&path);
        assert!((bbox.y0 + 10.0).abs() < 1e-9 && (bbox.y1 - 10.0).abs() < 1e-9);
    }
}
