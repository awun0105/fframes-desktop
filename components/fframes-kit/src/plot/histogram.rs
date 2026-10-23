use fframes::{Color, Svgr};
use kurbo::{BezPath, Rect, Shape as _};

use crate::{
    anim::RateFunc,
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::AxesMap,
    geometry::Shape,
    motion::progress,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// The samples counted into `bins` equal bins from `range.0` to `range.1`; a sample equal to
/// `range.1` goes into the last bin, samples outside the range and not finite are left out.
/// With `density` the counts are divided by the number of samples and the bin width, so the
/// bars' area is 1 and a probability density drawn over them fits.
pub fn bin(samples: &[f64], range: (f64, f64), bins: usize, density: bool) -> Vec<f64> {
    let (a, b) = range;
    let mut counts = vec![0.0; bins];
    if bins == 0 || b <= a {
        return counts;
    }
    let width = (b - a) / bins as f64;
    for &s in samples
        .iter()
        .filter(|s| s.is_finite() && (a..=b).contains(*s))
    {
        let i = (((s - a) / width) as usize).min(bins - 1);
        counts[i] += 1.0;
    }
    if density && !samples.is_empty() {
        let n = samples.len() as f64;
        for c in &mut counts {
            *c /= n * width;
        }
    }
    counts
}

/// Bars of `values` (one per bin, usually from [`bin`]) side by side from `range.0` to
/// `range.1` on axes, standing on the x-axis. From `delay` they grow one after another,
/// `stagger` seconds apart; `stagger: 0.0` grows them together. `gap` is the pixels between
/// bars.
#[derive(Debug, Clone, PartialEq)]
pub struct Histogram {
    pub values: Vec<f64>,
    pub range: (f64, f64),
    pub axes: AxesMap,
    pub gap: f32,
    pub fill: Color,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
    pub delay: f32,
    pub stagger: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
}

impl Themed for Histogram {
    fn themed(t: &Theme) -> Self {
        Self {
            values: Vec::new(),
            range: (0.0, 1.0),
            axes: AxesMap::default(),
            gap: 2.0,
            fill: t.colors.accent,
            stroke: None,
            stroke_width: t.stroke.thin,
            delay: 0.0,
            stagger: 0.04,
            run_time: t.motion.duration.base,
            rate_func: t.motion.easing.enter,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Histogram {
    /// How far bar `i` has grown at `t`, from 0 to 1.
    pub fn grown(&self, i: usize, t: f32) -> f32 {
        let start = self.delay + i as f32 * self.stagger;
        self.rate_func
            .apply(progress(t, start, start + self.run_time), self.run_time)
    }

    /// Each bar's rectangle in pixels at `t`; empty and not yet grown bars are left out.
    fn rects(&self, t: f32) -> Vec<Rect> {
        let (a, b) = self.range;
        let n = self.values.len();
        if n == 0 || b <= a {
            return Vec::new();
        }
        let width = (b - a) / n as f64;
        let base = self.axes.x_axis_y();
        let half_gap = f64::from(self.gap) / 2.0;
        self.values
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_finite() && **v != 0.0)
            .filter_map(|(i, &v)| {
                let p = f64::from(self.grown(i, t));
                let x0 = a + width * i as f64;
                let p0 = self.axes.c2p(x0, base);
                let p1 = self.axes.c2p(x0 + width, base + (v - base) * p);
                let rect = Rect::from_points(p0, p1);
                let rect = Rect::new(rect.x0 + half_gap, rect.y0, rect.x1 - half_gap, rect.y1);
                (p > 0.0 && rect.width() > 0.0).then_some(rect)
            })
            .collect()
    }
}

impl<'a> Component<'a> for Histogram {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let mut path = BezPath::new();
        for rect in self.rects(ui.seconds()) {
            path.extend(rect.path_elements(0.1));
        }
        let shape = Shape {
            path,
            fill: Some(self.fill),
            stroke: self.stroke,
            stroke_width: self.stroke_width,
            dash: None,
            opacity: 1.0,
        };
        draw_animated(ui, &[shape], self.enter.as_ref(), self.exit.as_ref(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explain::Range;

    #[test]
    fn samples_fall_into_their_bins() {
        let samples = [0.0, 0.1, 0.5, 0.99, 1.0, 1.5, -0.1, f64::NAN];
        assert_eq!(bin(&samples, (0.0, 1.0), 2, false), vec![2.0, 3.0]);
        // 8 samples, width 0.5: the counts become densities
        assert_eq!(bin(&samples, (0.0, 1.0), 2, true), vec![0.5, 0.75]);
        assert_eq!(bin(&samples, (1.0, 0.0), 2, false), vec![0.0, 0.0]);
        assert_eq!(bin(&samples, (0.0, 1.0), 0, false), Vec::<f64>::new());
    }

    #[test]
    fn bars_stand_on_the_axis_and_fill_their_bins() {
        let axes = AxesMap {
            x: Range::new(0.0, 4.0, 1.0),
            y: Range::new(0.0, 4.0, 1.0),
            ..AxesMap::default()
        };
        let h = Histogram {
            values: vec![1.0, 0.0, 4.0, 2.0],
            range: (0.0, 4.0),
            axes,
            gap: 0.0,
            delay: 0.0,
            stagger: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Linear,
            ..Histogram::default()
        };
        let rects = h.rects(1.0);
        assert_eq!(rects.len(), 3); // the empty bin has no bar
        let tall = rects[1];
        let (low, high) = (axes.c2p(2.0, 0.0), axes.c2p(3.0, 4.0));
        assert!((tall.x0 - low.x).abs() < 1e-9 && (tall.x1 - high.x).abs() < 1e-9);
        assert!((tall.y1 - low.y).abs() < 1e-9 && (tall.y0 - high.y).abs() < 1e-9);
        // half grown, half as high
        let half = h.rects(0.5)[1];
        assert!((half.height() - tall.height() / 2.0).abs() < 1e-9);
        assert_eq!(h.rects(0.0), Vec::new());
    }
}
