use fframes::{Color, Svgr};

use crate::{
    anim::RateFunc,
    motion::progress,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// A bar that fills from the left: with `progress: None` it shows how far the container is
/// through `ui.duration` (a video progress bar), otherwise the given value from 0 to 1. The
/// filled part is never thinner than half a pixel, so it is visible from the start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressBar {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub track: Color,
    pub fill: Color,
    pub progress: Option<f32>,
}

impl Themed for ProgressBar {
    fn themed(t: &Theme) -> Self {
        Self {
            x: 96.0,
            y: 1000.0,
            width: 1728.0,
            height: 8.0,
            radius: 0.0,
            track: t.colors.surface,
            fill: t.colors.accent,
            progress: None,
        }
    }
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for ProgressBar {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = self.progress.unwrap_or_else(|| {
            ui.duration_seconds()
                .map_or(0.0, |d| progress(ui.seconds(), 0.0, d))
        });
        let filled = (self.width * p.clamp(0.0, 1.0)).max(0.5);
        fframes::svgr!(
            <g>
                <rect x={self.x} y={self.y} width={self.width} height={self.height} rx={self.radius} fill={self.track} />
                <rect x={self.x} y={self.y} width={filled} height={self.height} rx={self.radius} fill={self.fill} />
            </g>
        )
    }
}

/// A check mark that draws itself on from `delay` seconds over `run_time`. `(x, y)` is the
/// start of the short stroke; the mark is 27 by 20 units times `size`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Check {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: Color,
    pub stroke_width: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Themed for Check {
    fn themed(t: &Theme) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            size: 1.0,
            color: t.colors.accent,
            stroke_width: 5.0,
            delay: 0.0,
            run_time: t.motion.duration.base,
            rate_func: t.motion.easing.standard,
        }
    }
}

impl Default for Check {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Check {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = progress(ui.seconds(), self.delay, self.delay + self.run_time);
        let p = self.rate_func.apply(p, self.run_time);
        if p <= 0.0 {
            return Svgr::empty();
        }
        let s = self.size;
        let (x, y) = (self.x, self.y);
        let d = format!(
            "M{x} {y} l{} {} l{} {}",
            9.0 * s,
            9.0 * s,
            18.0 * s,
            -20.0 * s
        );
        // the two strokes are 12.7 + 26.9 units long; 40 covers them
        let length = 40.0 * s;
        let dash = format!("{} {length}", length * p.min(1.0));
        fframes::svgr!(
            <path d={d} fill="none" stroke={self.color} stroke-width={self.stroke_width}
                  stroke-linecap="round" stroke-linejoin="round" stroke-dasharray={dash} />
        )
    }
}
