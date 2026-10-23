use fframes::{Color, Svgr};

use crate::{
    anim::RateFunc,
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::AxesMap,
    geometry::Shape,
    motion::progress,
    shapes::Dot,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// Dots at data points on axes. From `delay` they grow in one after another, `stagger`
/// seconds apart, each over `run_time`. Points that are not finite are left out.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterPlot {
    pub points: Vec<(f64, f64)>,
    pub axes: AxesMap,
    pub radius: f32,
    pub color: Color,
    pub delay: f32,
    pub stagger: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
    /// Applied to all the dots once they have grown.
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
}

impl Themed for ScatterPlot {
    fn themed(t: &Theme) -> Self {
        Self {
            points: Vec::new(),
            axes: AxesMap::default(),
            radius: 9.0,
            color: t.colors.accent,
            delay: 0.0,
            stagger: 0.03,
            run_time: t.motion.duration.fast,
            rate_func: t.motion.easing.enter,
            enter: None,
            exit: None,
        }
    }
}

impl Default for ScatterPlot {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl ScatterPlot {
    /// How far dot `i` has grown at `t`, from 0 to 1.
    pub fn grown(&self, i: usize, t: f32) -> f32 {
        let start = self.delay + i as f32 * self.stagger;
        self.rate_func
            .apply(progress(t, start, start + self.run_time), self.run_time)
            .max(0.0)
    }
}

impl<'a> Component<'a> for ScatterPlot {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let dots: Vec<Shape> = self
            .points
            .iter()
            .enumerate()
            .filter(|(_, (x, y))| x.is_finite() && y.is_finite())
            .filter_map(|(i, &(x, y))| {
                let p = self.grown(i, t);
                let at = self.axes.c2p(x, y);
                (p > 0.0).then(|| {
                    Dot {
                        cx: at.x as f32,
                        cy: at.y as f32,
                        radius: self.radius * p,
                        fill: Some(self.color),
                        ..ui.defaults()
                    }
                    .shape()
                })
            })
            .collect();
        draw_animated(ui, &dots, self.enter.as_ref(), self.exit.as_ref(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dots_grow_in_turn() {
        let plot = ScatterPlot {
            points: vec![(0.0, 0.0); 3],
            delay: 1.0,
            stagger: 0.5,
            run_time: 1.0,
            rate_func: RateFunc::Linear,
            ..ScatterPlot::default()
        };
        assert_eq!(plot.grown(0, 1.5), 0.5);
        assert_eq!(plot.grown(1, 1.5), 0.0);
        assert_eq!(plot.grown(2, 9.0), 1.0);
    }
}
