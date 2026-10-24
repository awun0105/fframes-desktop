use fframes::{Color, Svgr};
use kurbo::{Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::Space,
    kits::short,
    layout::Anchor,
    motion::progress,
    shapes::{Circle, Dot, Rectangle},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{clock, label};

/// A source moving right from `start` (units of `ui.space`, `scale` units per meter) at
/// `source_speed` m/s, sending out a wavefront every `period` seconds that spreads at
/// `wave_speed` m/s. The fronts bunch up ahead of the source and spread out behind it; at or
/// above the wave speed they pile into a cone. A front fades as it grows and is gone at
/// `max_radius` meters. Listeners `ahead` and `behind` (meters from `start`) are marked, and
/// with `labels` each shows the frequency it hears once the first front has reached it,
/// with the source's own frequency above the source. The listeners draw with `enter`, the
/// source and the fronts with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Doppler {
    pub start: (f64, f64),
    pub scale: f64,
    pub source_speed: f64,
    pub wave_speed: f64,
    pub period: f64,
    pub max_radius: f64,
    pub ahead: Option<f64>,
    pub behind: Option<f64>,
    pub labels: bool,
    pub delay: f32,
    pub time_scale: f32,
    pub source_color: Color,
    pub wave_color: Color,
    pub listener_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Doppler {
    fn themed(t: &Theme) -> Self {
        Self {
            start: (-4.0, 0.0),
            scale: 1.0,
            source_speed: 1.0,
            wave_speed: 3.0,
            period: 0.4,
            max_radius: 6.0,
            ahead: Some(10.0),
            behind: Some(-2.5),
            labels: true,
            delay: 0.0,
            time_scale: 1.0,
            source_color: t.colors.accent,
            wave_color: t.colors.text,
            listener_color: t.colors.highlight,
            stroke_width: t.stroke.thin,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Doppler {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Doppler {
    /// Meters from `start`, the source after `t` seconds.
    pub fn source(&self, t: f64) -> f64 {
        self.source_speed * t.max(0.0)
    }

    /// The frequency in Hz a listener at `x` meters from `start` hears while the source is
    /// where it is after `t` seconds: `f c / (c - v)` ahead of it, `f c / (c + v)` behind.
    /// `None` ahead of a source at or above the wave speed, where no front has come.
    pub fn heard(&self, x: f64, t: f64) -> Option<f64> {
        let (c, v) = (self.wave_speed, self.source_speed);
        let f = 1.0 / self.period.max(1e-9);
        let toward = if x >= self.source(t) { v } else { -v };
        (c - toward > 1e-9).then(|| f * c / (c - toward))
    }

    /// The emission time and the radius in meters of every front alive after `t` seconds,
    /// oldest first.
    pub fn fronts(&self, t: f64) -> Vec<(f64, f64)> {
        if self.period <= 0.0 || self.wave_speed <= 0.0 || t <= 0.0 {
            return Vec::new();
        }
        let oldest = ((t - self.max_radius / self.wave_speed) / self.period)
            .ceil()
            .max(0.0) as usize;
        let newest = (t / self.period).floor() as usize;
        (oldest..=newest)
            .map(|k| {
                let at = k as f64 * self.period;
                (at, self.wave_speed * (t - at))
            })
            .filter(|&(_, r)| r > 0.0 && r <= self.max_radius)
            .collect()
    }

    fn px(&self, space: Space, x: f64) -> Point {
        space.to_px(self.start.0 + x * self.scale, self.start.1)
    }
}

impl<'a> Component<'a> for Doppler {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let t = clock(ui, self.delay, self.time_scale);
        let listeners: Vec<f64> = [self.behind, self.ahead].into_iter().flatten().collect();
        let marks: Vec<_> = listeners
            .iter()
            .map(|&x| {
                let at = self.px(space, x);
                Dot {
                    cx: at.x as f32,
                    cy: at.y as f32,
                    radius: 12.0,
                    fill: Some(self.listener_color),
                    ..ui.defaults()
                }
                .shape()
            })
            .collect();
        let marks = draw_animated(ui, &marks, self.enter.as_ref(), self.exit.as_ref(), None);

        let mut shapes = Vec::new();
        for (at, r) in self.fronts(t) {
            let center = self.px(space, self.source(at));
            let mut front = Circle {
                cx: center.x as f32,
                cy: center.y as f32,
                radius: space.px(r * self.scale) as f32,
                fill: None,
                stroke: Some(self.wave_color),
                stroke_width: self.stroke_width,
                ..ui.defaults()
            }
            .shape();
            front.opacity = (1.0 - r / self.max_radius).clamp(0.0, 1.0) as f32;
            shapes.push(front);
        }
        let source = self.px(space, self.source(t));
        shapes.push(
            Dot {
                cx: source.x as f32,
                cy: source.y as f32,
                radius: 16.0,
                fill: Some(self.source_color),
                ..ui.defaults()
            }
            .shape(),
        );
        let mut parts = vec![draw_animated(ui, &shapes, None, None, None)];
        if self.labels {
            let f = 1.0 / self.period.max(1e-9);
            let text = format!("f = {} Hz", short(f, 2));
            // on a chip of the background, so the fronts do not run through the text
            let style = ui.theme.typography.caption;
            let size = style.size as f32;
            let width =
                crate::text::width(ui, style, &text).unwrap_or(0.55 * size * text.len() as f32);
            let above = source - Vec2::new(0.0, 28.0);
            let height = size * style.line_height;
            let chip = Rectangle {
                cx: above.x as f32,
                cy: above.y as f32 - height / 2.0,
                width: width + 16.0,
                height: height + 6.0,
                fill: Some(ui.theme.colors.background),
                stroke: None,
                ..ui.defaults()
            };
            parts.push(draw_animated(ui, &[chip.shape()], None, None, None));
            parts.push(label(ui, text, above, Anchor::Top, self.source_color));
            for &x in &listeners {
                // heard once the first front, sent at the start, has come
                let reached = (x.abs() / self.wave_speed.max(1e-9)) as f32;
                let shown = progress(t as f32, reached, reached + 0.3);
                let Some(heard) = self.heard(x, t).filter(|_| shown > 0.0) else {
                    continue;
                };
                let text = format!("{} Hz", short(heard, 2));
                let under = self.px(space, x) + Vec2::new(0.0, 24.0);
                let text = label(ui, text, under, Anchor::Bottom, self.listener_color);
                parts.push(fframes::svgr!(<g opacity={shown}>{text}</g>));
            }
        }
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{parts}{marks}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ahead_is_higher_and_behind_is_lower() {
        let d = Doppler {
            source_speed: 1.0,
            wave_speed: 3.0,
            period: 0.5,
            ..Doppler::default()
        };
        // f = 2 Hz: 2 · 3 / 2 ahead, 2 · 3 / 4 behind
        assert!((d.heard(10.0, 1.0).unwrap() - 3.0).abs() < 1e-12);
        assert!((d.heard(-3.0, 1.0).unwrap() - 1.5).abs() < 1e-12);
        let fast = Doppler {
            source_speed: 4.0,
            ..d
        };
        assert_eq!(fast.heard(10.0, 1.0), None);
    }

    #[test]
    fn fronts_ahead_are_closer_together() {
        let d = Doppler::default();
        let t = 3.0;
        let fronts = d.fronts(t);
        assert!(fronts.len() > 3);
        // the leading edges of two neighbouring fronts are (c - v) T apart ahead
        let edge = |(at, r): (f64, f64)| d.source(at) + r;
        let gap = edge(fronts[1]) - edge(fronts[2]);
        let expected = (d.wave_speed - d.source_speed) * d.period;
        assert!((gap - expected).abs() < 1e-9, "{gap}");
        // and none is larger than the limit
        assert!(fronts.iter().all(|&(_, r)| r <= d.max_radius));
        assert_eq!(d.fronts(0.0), Vec::new());
    }
}
