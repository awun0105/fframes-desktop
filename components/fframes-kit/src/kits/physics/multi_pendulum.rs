use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, TracedPath, draw_animated},
    geometry::Shape,
    shapes::Dot,
    sim::Sim,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{GRAVITY, RATE, clock, line, rk4_slice};

/// The swing of a chain of pendulums, Manim-physics' `MultiPendulum`: point masses
/// (`masses`, kg) on massless rods (`lengths`, meters), the first hanging from a fixed
/// pivot, released at rest from `angles` (radians from straight down, counterclockwise).
/// Solved from the Lagrangian with classic Runge-Kutta at 240 steps a second, without
/// friction, once in the constructor; keep it in the video and pass it to
/// [`MultiPendulum`].
#[derive(Debug, Clone, PartialEq)]
pub struct MultiPendulumMotion {
    pub lengths: Vec<f64>,
    pub masses: Vec<f64>,
    sim: Sim<Vec<f64>>,
}

/// `x` of `a x = b` for the augmented matrix `[a | b]`, by Gaussian elimination with partial
/// pivoting; 0 where the system has no single solution.
fn solve(mut m: Vec<Vec<f64>>) -> Vec<f64> {
    let n = m.len();
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs()))
            .unwrap_or(col);
        if m[pivot][col].abs() < 1e-12 {
            return vec![0.0; n];
        }
        m.swap(col, pivot);
        let (upper, lower) = m.split_at_mut(col + 1);
        let pivot = &upper[col];
        for row in lower {
            let f = row[col] / pivot[col];
            for (value, p) in row[col..].iter_mut().zip(&pivot[col..]) {
                *value -= f * p;
            }
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let known: f64 = (row + 1..n).map(|k| m[row][k] * x[k]).sum();
        x[row] = (m[row][n] - known) / m[row][row];
    }
    x
}

/// The time derivative of `[θ.., ω..]`: `M(θ) θ̈ = f(θ, ω)` with
/// `M_ij = μ_ij l_i l_j cos(θ_i - θ_j)`, `μ_ij` the mass at and below the lower of the two
/// links, and `f_i = -Σ_j μ_ij l_i l_j sin(θ_i - θ_j) ω_j² - μ_ii g l_i sin θ_i`.
fn derivative(lengths: &[f64], below: &[f64], y: &[f64]) -> Vec<f64> {
    let n = lengths.len();
    let (theta, omega) = y.split_at(n);
    let mut m = vec![vec![0.0; n + 1]; n];
    for i in 0..n {
        let mut rhs = -below[i] * GRAVITY * lengths[i] * theta[i].sin();
        for j in 0..n {
            let mu = below[i.max(j)] * lengths[i] * lengths[j];
            let d = theta[i] - theta[j];
            m[i][j] = mu * d.cos();
            rhs -= mu * d.sin() * omega[j] * omega[j];
        }
        m[i][n] = rhs;
    }
    let mut dy = omega.to_vec();
    dy.extend(solve(m));
    dy
}

impl MultiPendulumMotion {
    /// `seconds` of the swing. The chain has as many links as the shortest of the three
    /// lists; lengths and masses below a micro-unit are raised to it.
    pub fn new(lengths: &[f64], masses: &[f64], angles: &[f64], seconds: f64) -> Self {
        let n = lengths.len().min(masses.len()).min(angles.len());
        let lengths: Vec<f64> = lengths[..n].iter().map(|l| l.max(1e-6)).collect();
        let masses: Vec<f64> = masses[..n].iter().map(|m| m.max(1e-6)).collect();
        let below: Vec<f64> = (0..n).map(|k| masses[k..].iter().sum()).collect();
        let mut initial = angles[..n].to_vec();
        initial.resize(2 * n, 0.0);
        let steps = (seconds.max(0.0) * RATE).ceil() as usize;
        let sim = Sim::precompute(1.0 / RATE, steps, initial, |y, dt| {
            rk4_slice(y, dt, |s| derivative(&lengths, &below, s))
        });
        Self {
            lengths,
            masses,
            sim,
        }
    }

    /// Links in the chain.
    pub fn links(&self) -> usize {
        self.lengths.len()
    }

    /// The angles in radians and the angular speeds in rad/s after `t` seconds.
    pub fn state(&self, t: f64) -> (Vec<f64>, Vec<f64>) {
        let mut angles = self.sim.sample(t);
        let speeds = angles.split_off(self.links());
        (angles, speeds)
    }

    /// The bobs after `t` seconds, in meters from the pivot, `y` up.
    pub fn positions(&self, t: f64) -> Vec<(f64, f64)> {
        let (angles, _) = self.state(t);
        let (mut x, mut y) = (0.0, 0.0);
        angles
            .iter()
            .zip(&self.lengths)
            .map(|(a, l)| {
                x += l * a.sin();
                y -= l * a.cos();
                (x, y)
            })
            .collect()
    }

    /// The kinetic and the potential energy in joules after `t` seconds; the potential is
    /// 0 with every rod hanging straight down.
    pub fn energy(&self, t: f64) -> (f64, f64) {
        let (angles, speeds) = self.state(t);
        let (mut vx, mut vy, mut height, mut rest) = (0.0, 0.0, 0.0, 0.0);
        let (mut kinetic, mut potential) = (0.0, 0.0);
        for i in 0..self.links() {
            let (a, w, l, m) = (angles[i], speeds[i], self.lengths[i], self.masses[i]);
            vx += l * w * a.cos();
            vy += l * w * a.sin();
            height -= l * a.cos();
            rest -= l;
            kinetic += 0.5 * m * (vx * vx + vy * vy);
            potential += m * GRAVITY * (height - rest);
        }
        (kinetic, potential)
    }

    /// Seconds simulated.
    pub fn duration(&self) -> f64 {
        self.sim.duration()
    }
}

/// Manim-physics' `MultiPendulum` drawn: the chain of `motion` hanging from `pivot` (units
/// of `ui.space`, `scale` units per meter), swinging from `delay` seconds after the local
/// start. With `trail` the last bob leaves a trail like [`TracedPath`], the last
/// `trail_time` seconds of it (all of it with `None`). The ceiling draws with `enter`; the
/// rods, bobs and trail fade with `parts_enter`. Without `motion` nothing is drawn.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct MultiPendulum<'a> {
    pub motion: Option<&'a MultiPendulumMotion>,
    pub pivot: (f64, f64),
    pub scale: f64,
    pub delay: f32,
    pub time_scale: f32,
    pub bob_radius: f32,
    pub trail: bool,
    pub trail_time: Option<f32>,
    pub color: Color,
    pub bob_color: Color,
    pub trail_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for MultiPendulum<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            motion: None,
            pivot: (0.0, 2.5),
            scale: 1.0,
            delay: 0.0,
            time_scale: 1.0,
            bob_radius: 18.0,
            trail: true,
            trail_time: Some(3.0),
            color: t.colors.text,
            bob_color: t.colors.accent,
            trail_color: t.colors.highlight,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for MultiPendulum<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for MultiPendulum<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(motion) = self.motion else {
            return Svgr::empty();
        };
        let space = ui.space;
        let pivot = space.to_px(self.pivot.0, self.pivot.1);
        let bob = |(x, y): (f64, f64)| {
            space.to_px(self.pivot.0 + x * self.scale, self.pivot.1 + y * self.scale)
        };
        let width = space.px(0.5);
        let ceiling = line(
            ui,
            pivot - Vec2::new(width, 0.0),
            pivot + Vec2::new(width, 0.0),
            self.color,
            self.stroke_width,
        );
        let ceiling = draw_animated(
            ui,
            &[ceiling],
            self.enter.as_ref(),
            self.exit.as_ref(),
            None,
        );

        let mut shapes = Vec::new();
        if self.trail {
            let times = TracedPath {
                dissipating_time: self.trail_time,
                delay: self.delay,
                ..TracedPath::themed(ui.theme)
            }
            .times(ui.seconds(), ui.frame.fps);
            let mut path = BezPath::new();
            for (i, s) in times.into_iter().enumerate() {
                let t = f64::from(((s - self.delay) * self.time_scale).max(0.0));
                let Some(&last) = motion.positions(t).last() else {
                    break;
                };
                let p: Point = bob(last);
                if i == 0 {
                    path.move_to(p);
                } else {
                    path.line_to(p);
                }
            }
            shapes.push(Shape {
                path,
                fill: None,
                stroke: Some(self.trail_color),
                stroke_width: self.stroke_width,
                dash: None,
                opacity: 1.0,
            });
        }
        let t = clock(ui, self.delay, self.time_scale);
        let mut from = pivot;
        let bobs: Vec<Point> = motion.positions(t).into_iter().map(bob).collect();
        for &to in &bobs {
            shapes.push(line(ui, from, to, self.color, self.stroke_width));
            from = to;
        }
        let dot = |at: Point, radius: f32, color: Color| {
            Dot {
                cx: at.x as f32,
                cy: at.y as f32,
                radius,
                fill: Some(color),
                ..ui.defaults()
            }
            .shape()
        };
        shapes.push(dot(pivot, 7.0, self.color));
        for &at in &bobs {
            shapes.push(dot(at, self.bob_radius, self.bob_color));
        }
        let parts = draw_animated(ui, &shapes, None, None, None);
        let parts = fade(
            ui,
            parts,
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{ceiling}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::super::PendulumMotion;
    use super::*;

    #[test]
    fn one_link_swings_like_a_pendulum() {
        let chain = MultiPendulumMotion::new(&[1.5], &[2.0], &[1.1], 5.0);
        let single = PendulumMotion::new(1.5, 1.1, 5.0);
        for i in 0..=50 {
            let t = f64::from(i) * 0.1;
            let (angles, speeds) = chain.state(t);
            let (a, w) = single.state(t);
            assert!((angles[0] - a).abs() < 1e-9, "at {t}: {} {a}", angles[0]);
            assert!((speeds[0] - w).abs() < 1e-9, "at {t}");
        }
    }

    #[test]
    fn a_double_pendulum_keeps_its_energy() {
        let chain = MultiPendulumMotion::new(&[1.0, 0.8], &[1.0, 0.6], &[2.0, -1.0], 10.0);
        let (k0, p0) = chain.energy(0.0);
        assert_eq!(k0, 0.0);
        let mut drift: f64 = 0.0;
        for i in 0..=2400 {
            let (k, p) = chain.energy(f64::from(i) / RATE);
            drift = drift.max(((k + p) - p0).abs() / p0);
        }
        let mut between: f64 = 0.0;
        for i in 0..=1000 {
            let (k, p) = chain.energy(f64::from(i) * 0.01);
            between = between.max(((k + p) - p0).abs() / p0);
        }
        // measured 2.2e-7 on the steps; sampled between them the blend of two states adds
        // 1.1e-3, from the interpolation, not the solver
        assert!(drift < 1e-6, "{drift}");
        assert!(between < 5e-3, "{between}");
    }

    #[test]
    fn the_bobs_hang_below_the_pivot_at_rest() {
        let chain = MultiPendulumMotion::new(&[1.0, 0.5, 0.25], &[1.0; 3], &[0.0; 3], 1.0);
        assert_eq!(chain.links(), 3);
        assert_eq!(
            chain.positions(0.7),
            vec![(0.0, -1.0), (0.0, -1.5), (0.0, -1.75)]
        );
        assert_eq!(chain.energy(0.7), (0.0, 0.0));
    }

    #[test]
    fn mismatched_lists_use_the_shortest() {
        let chain = MultiPendulumMotion::new(&[1.0, 1.0], &[1.0], &[0.3, 0.2], 0.5);
        assert_eq!(chain.links(), 1);
        assert_eq!(chain.positions(0.2).len(), 1);
    }
}
