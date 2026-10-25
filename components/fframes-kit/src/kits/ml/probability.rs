//! Seeded samples of normal distributions, for the point clouds of the kit.

use std::f64::consts::TAU;

use crate::motion::Rng;

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
}
