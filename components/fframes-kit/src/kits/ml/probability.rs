//! Normal distributions: seeded samples for the point clouds of the kit, their ellipses of
//! equal density, and a Metropolis-Hastings chain that samples them.

use std::f64::consts::TAU;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    explain::AxesMap,
    geometry::Shape,
    kits::draw::stroke,
    motion::{Rng, cubic_in_out},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// Two independent samples of the standard normal distribution (the Box-Muller transform).
pub(crate) fn standard_normals(rng: &mut Rng) -> (f64, f64) {
    let (u, v) = (rng.unit(), rng.unit());
    let r = (-2.0 * u.ln()).sqrt();
    (r * (TAU * v).cos(), r * (TAU * v).sin())
}

/// A 2D normal distribution: its `mean` and its `covariance` matrix
/// `[[var x, cov xy], [cov xy, var y]]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian2D {
    pub mean: (f64, f64),
    pub covariance: [[f64; 2]; 2],
}

impl Default for Gaussian2D {
    /// The standard normal distribution: mean 0, unit variances, no correlation.
    fn default() -> Self {
        Self {
            mean: (0.0, 0.0),
            covariance: [[1.0, 0.0], [0.0, 1.0]],
        }
    }
}

impl Gaussian2D {
    /// The lower triangular `L` with `L·Lᵀ` the covariance (the Cholesky factor), or `None`
    /// when the covariance is not positive definite (not symmetric, or a variance of 0).
    pub(crate) fn cholesky(&self) -> Option<[[f64; 2]; 2]> {
        let [[a, b], [c, d]] = self.covariance;
        if (b - c).abs() > 1e-12 * (1.0 + b.abs()) || a <= 0.0 {
            return None;
        }
        let l00 = a.sqrt();
        let l10 = b / l00;
        let rest = d - l10 * l10;
        (rest > 0.0).then(|| [[l00, 0.0], [l10, rest.sqrt()]])
    }

    /// The point `mean + L·epsilon`, `epsilon` in units of the standard normal distribution
    /// (the reparameterisation of a variational autoencoder); `None` when the covariance is
    /// not positive definite.
    pub fn point(&self, epsilon: (f64, f64)) -> Option<(f64, f64)> {
        let [[l00, _], [l10, l11]] = self.cholesky()?;
        Some((
            self.mean.0 + l00 * epsilon.0,
            self.mean.1 + l10 * epsilon.0 + l11 * epsilon.1,
        ))
    }

    /// 64 points around the ellipse `sigmas` standard deviations from the mean (where the
    /// density is the same), in the units of the distribution; none when the covariance is
    /// not positive definite.
    pub fn ring(&self, sigmas: f64) -> Vec<(f64, f64)> {
        (0..64)
            .filter_map(|i| {
                let a = TAU * f64::from(i) / 64.0;
                self.point((sigmas * a.cos(), sigmas * a.sin()))
            })
            .collect()
    }

    /// The principal axes of the distribution: the standard deviation along each eigenvector
    /// of the covariance and that unit eigenvector, the longer axis first; `None` when the
    /// covariance is not positive definite.
    pub fn principal_axes(&self) -> Option<[(f64, (f64, f64)); 2]> {
        self.cholesky()?;
        let [[a, b], [_, d]] = self.covariance;
        let middle = a.midpoint(d);
        let spread = ((a - d) / 2.0).hypot(b);
        let (big, small) = (middle + spread, middle - spread);
        // (b, λ - a) is an eigenvector of λ unless the matrix is already diagonal
        let major = if b.abs() > 1e-12 {
            let (x, y) = (b, big - a);
            let n = x.hypot(y);
            (x / n, y / n)
        } else if a >= d {
            (1.0, 0.0)
        } else {
            (0.0, 1.0)
        };
        Some([
            (big.sqrt(), major),
            (small.max(0.0).sqrt(), (-major.1, major.0)),
        ])
    }

    /// The probability density at `p`, 0 when the covariance is not positive definite.
    pub fn density(&self, p: (f64, f64)) -> f64 {
        if self.cholesky().is_none() {
            return 0.0;
        }
        let [[a, b], [_, d]] = self.covariance;
        let det = a * d - b * b;
        let (dx, dy) = (p.0 - self.mean.0, p.1 - self.mean.1);
        let q = (d * dx * dx - 2.0 * b * dx * dy + a * dy * dy) / det;
        (-0.5 * q).exp() / (TAU * det.sqrt())
    }

    /// `n` samples, the same for the same `seed`; none when the covariance is not positive
    /// definite.
    pub fn samples(&self, seed: u64, n: usize) -> Vec<(f64, f64)> {
        let Some([[l00, _], [l10, l11]]) = self.cholesky() else {
            return Vec::new();
        };
        let mut rng = Rng::new(seed);
        (0..n)
            .map(|_| {
                let (z0, z1) = standard_normals(&mut rng);
                (self.mean.0 + l00 * z0, self.mean.1 + l10 * z0 + l11 * z1)
            })
            .collect()
    }
}

/// How long a ring or a step takes to appear, in seconds (shorter when the steps are).
const APPEAR: f32 = 0.3;

/// How far something that starts to appear at `at` has appeared at `now`, eased.
fn appear(now: f32, at: f32, ramp: f32) -> f32 {
    cubic_in_out(((now - at) / ramp.max(1e-3)).clamp(0.0, 1.0))
}

fn filled(path: BezPath, color: Color, opacity: f32) -> Shape {
    Shape {
        path,
        fill: Some(color),
        stroke: None,
        stroke_width: 0.0,
        dash: None,
        opacity,
    }
}

/// The closed path through `points` (data units) on `axes`.
fn outline(axes: &AxesMap, points: &[(f64, f64)]) -> BezPath {
    let mut path = BezPath::new();
    for (k, &(x, y)) in points.iter().enumerate() {
        let p = axes.c2p(x, y);
        if k == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
}

/// A 2D normal distribution drawn as its ellipses of equal density (`ManimML`'s
/// `GaussianDistribution`): a ring `sigmas` standard deviations from the mean for each entry,
/// in `color`, each filled at `fill_opacity` (the inner rings stack up darker), the mean as a
/// dot and, with `show_axes`, the principal axes out to the outer ring. On `axes`; draw the
/// [`Axes`](crate::explain::Axes) after it. With `stagger` the rings appear one after the
/// other from `start`, the innermost first.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct GaussianEllipse {
    pub gaussian: Gaussian2D,
    pub axes: AxesMap,
    pub sigmas: Vec<f64>,
    pub color: Color,
    pub fill_opacity: f32,
    pub stroke_width: f32,
    pub show_axes: bool,
    pub start: f32,
    pub stagger: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for GaussianEllipse {
    fn themed(t: &Theme) -> Self {
        Self {
            gaussian: Gaussian2D::default(),
            axes: AxesMap::default(),
            sigmas: vec![1.0, 2.0, 3.0],
            color: t.colors.accent,
            fill_opacity: 0.12,
            stroke_width: 2.5,
            show_axes: false,
            start: 0.0,
            stagger: 0.0,
            enter: None,
            exit: None,
        }
    }
}

impl Default for GaussianEllipse {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for GaussianEllipse {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let now = ui.seconds();
        let g = &self.gaussian;
        let ramp = if self.stagger > 0.0 {
            APPEAR.min(self.stagger)
        } else {
            0.0
        };
        let shown = |k: usize| {
            if ramp > 0.0 {
                appear(now, self.start + k as f32 * self.stagger, ramp)
            } else {
                1.0
            }
        };
        let mut order: Vec<f64> = self.sigmas.clone();
        order.sort_by(f64::total_cmp);
        let mut parts = Vec::new();
        // the outer rings first, so the inner ones lie on top
        for (k, &sigmas) in order.iter().enumerate().rev() {
            let ring = g.ring(sigmas);
            let o = shown(k);
            if ring.is_empty() || o <= 0.0 {
                continue;
            }
            let path = outline(&self.axes, &ring);
            parts.push(filled(path.clone(), self.color, self.fill_opacity * o).draw());
            parts.push(stroke(path, self.color, self.stroke_width, o).draw());
        }
        let outer = order.last().copied().unwrap_or(1.0);
        if let (true, Some(axes)) = (self.show_axes, g.principal_axes()) {
            let mut lines = BezPath::new();
            for (sd, (dx, dy)) in axes {
                let r = sd * outer;
                lines.move_to(self.axes.c2p(g.mean.0 - dx * r, g.mean.1 - dy * r));
                lines.line_to(self.axes.c2p(g.mean.0 + dx * r, g.mean.1 + dy * r));
            }
            let o = shown(order.len().saturating_sub(1));
            parts.push(stroke(lines, self.color, self.stroke_width * 0.6, o * 0.8).draw());
        }
        let dot = Circle::new(
            self.axes.c2p(g.mean.0, g.mean.1),
            f64::from(self.stroke_width) * 2.2,
        );
        parts.push(filled(dot.to_path(0.1), self.color, shown(0)).draw());
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

/// One step of a Metropolis-Hastings chain: the state it starts `from`, the proposal `to`,
/// and whether the chain moved there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct McmcStep {
    pub from: (f64, f64),
    pub to: (f64, f64),
    pub accepted: bool,
}

/// The density of an equal mixture of `target`, up to a constant.
fn mixture(target: &[Gaussian2D], p: (f64, f64)) -> f64 {
    target.iter().map(|g| g.density(p)).sum()
}

/// A random-walk Metropolis-Hastings chain sampling the equal mixture of the normal
/// distributions in `target` (`ManimML`'s MCMC scenes). It starts at `origin` and proposes
/// steps drawn from a normal distribution `proposal` wide; a step is accepted with the
/// probability `min(1, p(to) / p(from))`. The chain is the same for the same `seed`. With
/// `run` it plays one step after another over the run's duration from its start: accepted
/// moves extend the path in `accepted_color`, rejected proposals inside the axes stay as
/// hollow marks in `rejected_color`, and the current state is a dot. On `axes`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct McmcChain {
    pub target: Vec<Gaussian2D>,
    pub origin: (f64, f64),
    pub proposal: f64,
    pub steps: usize,
    pub seed: u64,
    pub axes: AxesMap,
    pub run: Option<ChainRun>,
    pub accepted_color: Color,
    pub rejected_color: Color,
    pub stroke_width: f32,
    pub point_radius: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

/// When a chain plays: from `start`, all its steps over `duration` seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChainRun {
    pub start: f32,
    pub duration: f32,
}

impl Themed for McmcChain {
    fn themed(t: &Theme) -> Self {
        Self {
            target: vec![Gaussian2D::default()],
            origin: (0.0, 0.0),
            proposal: 0.6,
            steps: 60,
            seed: 1,
            axes: AxesMap::default(),
            run: None,
            accepted_color: t.colors.highlight,
            rejected_color: Color::hex("#ff6b6b"),
            stroke_width: 2.5,
            point_radius: 6.0,
            enter: None,
            exit: None,
        }
    }
}

impl Default for McmcChain {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl McmcChain {
    /// The steps of the chain.
    pub fn chain(&self) -> Vec<McmcStep> {
        let mut rng = Rng::new(self.seed);
        let mut at = self.origin;
        let mut density = mixture(&self.target, at);
        (0..self.steps)
            .map(|_| {
                let (zx, zy) = standard_normals(&mut rng);
                let to = (at.0 + zx * self.proposal, at.1 + zy * self.proposal);
                let next = mixture(&self.target, to);
                // a state of no density is left for any state with some
                let accepted = next > 0.0 && (next >= density || rng.unit() * density < next);
                let step = McmcStep {
                    from: at,
                    to,
                    accepted,
                };
                if accepted {
                    (at, density) = (to, next);
                }
                step
            })
            .collect()
    }
}

impl<'a> Component<'a> for McmcChain {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let now = ui.seconds();
        let steps = self.chain();
        let px = |p: (f64, f64)| self.axes.c2p(p.0, p.1);
        // how far along the path of accepted moves each step starts, in pixels
        let mut length = 0.0;
        let starts: Vec<f64> = steps
            .iter()
            .map(|s| {
                let at = length;
                if s.accepted {
                    length += px(s.from).distance(px(s.to));
                }
                at
            })
            .collect();
        let total = length.max(1e-9);
        // the walker moves along the path at an even speed; a step shows from the moment the
        // walker passes the place it starts from
        let (walked, shown): (f64, Box<dyn Fn(f64) -> f32>) = match self.run {
            Some(run) => {
                let p = ((now - run.start) / run.duration.max(1e-3)).clamp(0.0, 1.0);
                let at = move |a: f64| run.start + (a / total) as f32 * run.duration;
                (
                    f64::from(p) * total,
                    Box::new(move |a| appear(now, at(a), APPEAR)),
                )
            }
            None => (total, Box::new(|_| 1.0)),
        };
        let (x, y) = (self.axes.x, self.axes.y);
        let inside =
            |p: (f64, f64)| (x.min..=x.max).contains(&p.0) && (y.min..=y.max).contains(&p.1);
        let r = f64::from(self.point_radius);
        let mut parts = Vec::new();
        let mut at = px(self.origin);
        for (step, &a) in steps.iter().zip(&starts) {
            if step.accepted {
                if walked <= a && self.run.is_some() {
                    continue;
                }
                let len = px(step.from).distance(px(step.to)).max(1e-9);
                at = px(step.from).lerp(px(step.to), ((walked - a) / len).clamp(0.0, 1.0));
                let mut path = BezPath::new();
                path.move_to(px(step.from));
                path.line_to(at);
                parts.push(stroke(path, self.accepted_color, self.stroke_width, 0.8).draw());
                let o = shown(a + len);
                let dot = Circle::new(px(step.to), r * 0.6).to_path(0.1);
                parts.push(filled(dot, self.accepted_color, 0.6 * o).draw());
            } else if inside(step.to) {
                let o = shown(a);
                if o <= 0.0 {
                    continue;
                }
                let mark = Circle::new(px(step.to), r * 0.8).to_path(0.1);
                let width = self.stroke_width * 0.8;
                parts.push(stroke(mark, self.rejected_color, width, 0.85 * o).draw());
            }
        }
        let current = Circle::new(at, r).to_path(0.1);
        parts.push(filled(current.clone(), self.accepted_color, 1.0).draw());
        parts.push(stroke(current, Color::hex("#ffffff"), self.stroke_width * 0.6, 1.0).draw());
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TILTED: Gaussian2D = Gaussian2D {
        mean: (2.0, -1.0),
        covariance: [[4.0, 1.2], [1.2, 1.0]],
    };

    #[test]
    fn the_cholesky_factor_gives_back_the_covariance() {
        let [[l00, l01], [l10, l11]] = TILTED.cholesky().unwrap();
        assert_eq!(l01, 0.0);
        let product = [[l00 * l00, l00 * l10], [l10 * l00, l10 * l10 + l11 * l11]];
        for (row, expected) in product.iter().zip(TILTED.covariance) {
            for (x, e) in row.iter().zip(expected) {
                assert!((x - e).abs() < 1e-12);
            }
        }
        // perfectly correlated, a negative variance, not symmetric
        for covariance in [
            [[1.0, 1.0], [1.0, 1.0]],
            [[-1.0, 0.0], [0.0, 1.0]],
            [[1.0, 0.5], [0.2, 1.0]],
        ] {
            let g = Gaussian2D {
                covariance,
                ..TILTED
            };
            assert_eq!(g.cholesky(), None);
            assert_eq!(g.samples(1, 5), Vec::new());
        }
    }

    #[test]
    fn rings_have_the_same_density_all_round() {
        for (x, y) in Gaussian2D::default().ring(1.0) {
            assert!((x.hypot(y) - 1.0).abs() < 1e-12);
        }
        let [[a, b], [_, d]] = TILTED.covariance;
        let det = a * d - b * b;
        for k in [1.0, 2.0, 3.0] {
            let ring = TILTED.ring(k);
            assert_eq!(ring.len(), 64);
            for (x, y) in ring {
                let (dx, dy) = (x - TILTED.mean.0, y - TILTED.mean.1);
                // xᵀ Σ⁻¹ x with the inverse of the 2 by 2 covariance written out
                let q = (d * dx * dx - 2.0 * b * dx * dy + a * dy * dy) / det;
                assert!((q - k * k).abs() < 1e-9);
            }
        }
        assert_eq!(TILTED.point((0.0, 0.0)), Some(TILTED.mean));
        let flat = Gaussian2D {
            covariance: [[1.0, 1.0], [1.0, 1.0]],
            ..TILTED
        };
        assert_eq!(flat.point((1.0, 0.0)), None);
        assert_eq!(flat.ring(1.0), Vec::new());
    }

    #[test]
    fn samples_follow_the_distribution_and_the_seed() {
        let n = 4000;
        let samples = TILTED.samples(7, n);
        assert_eq!(samples, TILTED.samples(7, n));
        assert_ne!(samples, TILTED.samples(8, n));
        let mean = |f: fn(&(f64, f64)) -> f64| samples.iter().map(f).sum::<f64>() / n as f64;
        let (mx, my) = (mean(|p| p.0), mean(|p| p.1));
        // within 4 standard errors of the mean
        assert!((mx - 2.0).abs() < 4.0 * 2.0 / (n as f64).sqrt());
        assert!((my + 1.0).abs() < 4.0 * 1.0 / (n as f64).sqrt());
        let cov = samples.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum::<f64>() / n as f64;
        assert!((cov - 1.2).abs() < 0.2);
    }

    #[test]
    fn the_ellipse_axes_come_from_the_eigenvalues() {
        let [[a, b], [_, d]] = TILTED.covariance;
        let axes = TILTED.principal_axes().unwrap();
        for (sd, (x, y)) in axes {
            // Σ v = λ v with λ = sd²
            assert!((a * x + b * y - sd * sd * x).abs() < 1e-12);
            assert!((b * x + d * y - sd * sd * y).abs() < 1e-12);
            assert!((x.hypot(y) - 1.0).abs() < 1e-12);
        }
        assert!(axes[0].0 > axes[1].0);
        // the eigenvalues of the 2 by 2 covariance written out
        let root = (((a - d) / 2.0).powi(2) + b * b).sqrt();
        assert!((axes[0].0.powi(2) - (a.midpoint(d) + root)).abs() < 1e-12);
        for k in [1.0, 2.0, 3.0] {
            // the ends of the axes lie on the ring, which reaches no farther out
            for (sd, (x, y)) in axes {
                let end = (TILTED.mean.0 + k * sd * x, TILTED.mean.1 + k * sd * y);
                let q = -2.0 * (TILTED.density(end) / TILTED.density(TILTED.mean)).ln();
                assert!((q - k * k).abs() < 1e-9);
            }
            let farthest = TILTED
                .ring(k)
                .into_iter()
                .map(|(x, y)| (x - TILTED.mean.0).hypot(y - TILTED.mean.1))
                .fold(0.0, f64::max);
            assert!(farthest <= k * axes[0].0 + 1e-9);
            assert!(farthest > k * axes[0].0 * 0.99);
        }
        // a diagonal covariance has the coordinate axes
        let wide = Gaussian2D {
            covariance: [[1.0, 0.0], [0.0, 4.0]],
            ..TILTED
        };
        assert_eq!(
            wide.principal_axes(),
            Some([(2.0, (0.0, 1.0)), (1.0, (-1.0, 0.0))])
        );
        assert_eq!(
            Gaussian2D {
                covariance: [[1.0, 1.0], [1.0, 1.0]],
                ..TILTED
            }
            .principal_axes(),
            None
        );
    }

    #[test]
    fn the_density_integrates_to_one() {
        let (h, mut total) = (0.05, 0.0);
        for i in -200..200 {
            for j in -200..200 {
                let p = (2.0 + f64::from(i) * h, -1.0 + f64::from(j) * h);
                total += TILTED.density(p) * h * h;
            }
        }
        assert!((total - 1.0).abs() < 1e-3);
    }

    #[test]
    fn the_chain_is_the_same_for_a_seed_and_samples_the_target() {
        let chain = McmcChain {
            target: vec![TILTED],
            origin: (-3.0, 3.0),
            proposal: 1.0,
            steps: 20000,
            seed: 3,
            ..Default::default()
        };
        let steps = chain.chain();
        assert_eq!(steps, chain.chain());
        assert_ne!(
            steps,
            McmcChain {
                seed: 4,
                ..chain.clone()
            }
            .chain()
        );
        // each step starts where the chain stands and moves only when accepted
        let mut at = chain.origin;
        for s in &steps {
            assert_eq!(s.from, at);
            if s.accepted {
                at = s.to;
            }
            // a step uphill is always taken
            if TILTED.density(s.to) >= TILTED.density(s.from) {
                assert!(s.accepted);
            }
        }
        let rate = steps.iter().filter(|s| s.accepted).count() as f64 / steps.len() as f64;
        assert!((0.2..0.9).contains(&rate));
        // after a burn-in the states average to the mean
        let states: Vec<(f64, f64)> = steps[2000..]
            .iter()
            .map(|s| if s.accepted { s.to } else { s.from })
            .collect();
        let n = states.len() as f64;
        let mx = states.iter().map(|p| p.0).sum::<f64>() / n;
        let my = states.iter().map(|p| p.1).sum::<f64>() / n;
        assert!((mx - 2.0).abs() < 0.2);
        assert!((my + 1.0).abs() < 0.1);
    }
}
