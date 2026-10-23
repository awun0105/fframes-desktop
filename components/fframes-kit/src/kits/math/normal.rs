//! The normal distribution: seeded samples, their mean and spread, and [`NormalFit`].

use std::f64::consts::{PI, TAU};

use fframes::{Color, Svgr};

use crate::{
    anim::{FadeIn, RateFunc, ValueTracker},
    animate::EnterAnim,
    explain::AxesMap,
    label::Label,
    layout::Anchor,
    plot::{Histogram, LinePlot, bin},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::short;

/// `n` samples of the normal distribution with `mean` and standard deviation `sd`, the same
/// for the same `seed` (`SplitMix64` and the Box-Muller transform). The random bits match on
/// every machine; the transform's `ln`, `cos` and `sin` may differ in the last bit.
pub fn normal_samples(seed: u64, n: usize, mean: f64, sd: f64) -> Vec<f64> {
    let mut state = seed;
    let mut uniform = move || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        // 53 random bits in (0, 1], so the logarithm below is finite
        ((z >> 11) as f64 + 1.0) / (1u64 << 53) as f64
    };
    let mut samples = Vec::with_capacity(n);
    while samples.len() < n {
        let (u, v) = (uniform(), uniform());
        let r = (-2.0 * u.ln()).sqrt();
        samples.push(mean + sd * r * (TAU * v).cos());
        if samples.len() < n {
            samples.push(mean + sd * r * (TAU * v).sin());
        }
    }
    samples
}

/// The mean and the (population) standard deviation of `samples`; `(0, 0)` without samples.
pub fn mean_sd(samples: &[f64]) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let var = samples.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n;
    (mean, var.sqrt())
}

/// The density of the normal distribution with `mean` and `sd` at `x`.
pub fn normal_pdf(x: f64, mean: f64, sd: f64) -> f64 {
    if sd <= 0.0 {
        return 0.0;
    }
    let z = (x - mean) / sd;
    (-0.5 * z * z).exp() / (sd * (2.0 * PI).sqrt())
}

/// Samples stacking up into a histogram, then the normal curve that fits them (Manim's
/// central limit theorem scenes). The first `shown` samples (a [`ValueTracker`], so they can
/// pour in) are binned into `bins` bars over `range` as a density, no taller than the top of
/// the y range. The curve has the mean and
/// spread of the samples shown so far and draws with `curve_enter` (a delayed `Create`);
/// with `stats` the mean and standard deviation are written at `stats_at` (pixels).
#[derive(Debug, Clone)]
pub struct NormalFit<'a> {
    pub samples: &'a [f64],
    pub shown: ValueTracker<'a>,
    pub axes: AxesMap,
    pub range: (f64, f64),
    pub bins: usize,
    pub fill: Color,
    pub curve_color: Color,
    pub curve_width: f32,
    pub curve_enter: Option<EnterAnim>,
    pub stats: bool,
    pub stats_at: (f32, f32),
    pub stats_enter: Option<FadeIn>,
}

impl Themed for NormalFit<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            samples: &[],
            shown: 0.0.into(),
            axes: AxesMap::default(),
            range: (-3.0, 3.0),
            bins: 24,
            fill: t.colors.accent,
            curve_color: t.colors.highlight,
            curve_width: t.stroke.thick,
            curve_enter: None,
            stats: true,
            stats_at: (1500.0, 300.0),
            stats_enter: None,
        }
    }
}

impl Default for NormalFit<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl NormalFit<'_> {
    /// The samples shown when the tracker reads `count`.
    fn visible(&self, count: f32) -> &[f64] {
        let n = (count.max(0.0) as usize).min(self.samples.len());
        &self.samples[..n]
    }
}

/// Points along the curve.
const CURVE_SAMPLES: usize = 96;

impl<'a> Component<'a> for NormalFit<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let visible = self.visible(ui.sample(&self.shown));
        // the first few samples make bars far taller than the curve; they stop at the top
        let top = self.axes.y.max;
        let values = bin(visible, self.range, self.bins, true)
            .into_iter()
            .map(|v| v.min(top))
            .collect();
        let histogram = ui.show(Histogram {
            values,
            range: self.range,
            axes: self.axes,
            fill: self.fill,
            gap: 2.0,
            delay: 0.0,
            stagger: 0.0,
            run_time: 0.0,
            rate_func: RateFunc::Linear,
            ..ui.defaults()
        });
        if visible.len() < 2 {
            return histogram;
        }
        let (mean, sd) = mean_sd(visible);
        let (a, b) = self.range;
        let points = (0..=CURVE_SAMPLES)
            .map(|i| {
                let x = a + (b - a) * i as f64 / CURVE_SAMPLES as f64;
                (x, normal_pdf(x, mean, sd))
            })
            .collect();
        let curve = ui.show(LinePlot {
            points,
            axes: self.axes,
            smooth: true,
            stroke: Some(self.curve_color),
            stroke_width: self.curve_width,
            enter: self.curve_enter,
            ..ui.defaults()
        });
        let stats = if self.stats {
            ui.show(Label {
                text: format!(
                    "n = {}   mean {}   sd {}",
                    visible.len(),
                    short(mean, 2),
                    short(sd, 2)
                )
                .into(),
                x: self.stats_at.0,
                y: self.stats_at.1,
                direction: Anchor::Center,
                buff: 0.0,
                enter: self.stats_enter,
                ..ui.defaults()
            })
        } else {
            Svgr::empty()
        };
        fframes::svgr!(<g>{histogram}{curve}{stats}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_samples_have_the_asked_mean_and_spread() {
        let samples = normal_samples(7, 4001, 2.0, 0.5);
        assert_eq!(samples.len(), 4001);
        assert_eq!(samples, normal_samples(7, 4001, 2.0, 0.5));
        assert_ne!(samples[..10], normal_samples(8, 10, 2.0, 0.5)[..]);
        let (mean, sd) = mean_sd(&samples);
        assert!((mean - 2.0).abs() < 0.03, "{mean}");
        assert!((sd - 0.5).abs() < 0.03, "{sd}");
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn the_density_integrates_to_one() {
        let area = super::super::integral(|x| normal_pdf(x, 0.0, 1.0), None, -8.0, 8.0, 512);
        assert!((area - 1.0).abs() < 1e-6);
        assert!((normal_pdf(0.0, 0.0, 1.0) - 0.398_942_28).abs() < 1e-8);
        assert_eq!(normal_pdf(1.0, 0.0, 0.0), 0.0);
        assert_eq!(mean_sd(&[]), (0.0, 0.0));
        assert_eq!(mean_sd(&[1.0, 3.0]), (2.0, 1.0));
    }

    #[test]
    fn the_tracker_picks_how_many_samples_show() {
        let samples = [1.0, 2.0, 3.0];
        let fit = NormalFit {
            samples: &samples,
            ..NormalFit::default()
        };
        assert_eq!(fit.visible(-1.0), &[] as &[f64]);
        assert_eq!(fit.visible(2.7), &[1.0, 2.0]);
        assert_eq!(fit.visible(99.0), &samples);
    }
}
