use fframes::{Color, Svgr};
use kurbo::BezPath;

use crate::{
    geometry::Shape,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// The most points a [`TracedPath`] samples per frame; a longer trail samples more sparsely.
pub const MAX_TRACE_SAMPLES: usize = 240;

fn origin(_: f32) -> (f32, f32) {
    (0.0, 0.0)
}

/// The line a moving point has drawn so far (Manim's `TracedPath`). `point(t)` is where the
/// point is `t` local seconds in, in canvas units; the line runs through it at every frame
/// from `delay` (or the last `dissipating_time` seconds) up to now. The trail is sampled
/// again every frame, at most [`MAX_TRACE_SAMPLES`] times, on a fixed grid of times so the
/// old part does not shimmer.
#[derive(Clone, Copy)]
pub struct TracedPath<'a> {
    pub point: &'a dyn Fn(f32) -> (f32, f32),
    pub stroke: Color,
    pub stroke_width: f32,
    /// Seconds of trail to keep; `None` keeps it all.
    pub dissipating_time: Option<f32>,
    /// Local second where the trail starts.
    pub delay: f32,
}

impl std::fmt::Debug for TracedPath<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracedPath")
            .field("stroke", &self.stroke)
            .field("stroke_width", &self.stroke_width)
            .field("dissipating_time", &self.dissipating_time)
            .field("delay", &self.delay)
            .finish_non_exhaustive()
    }
}

impl Themed for TracedPath<'_> {
    /// A point that stays at the origin; set `point`.
    fn themed(t: &Theme) -> Self {
        Self {
            point: &origin,
            stroke: t.colors.highlight,
            stroke_width: t.stroke.regular,
            dissipating_time: None,
            delay: 0.0,
        }
    }
}

impl TracedPath<'_> {
    /// The times sampled at `now` with `fps` frames per second: the start, every frame on
    /// the grid since, and `now`.
    pub fn times(&self, now: f32, fps: usize) -> Vec<f32> {
        let start = self
            .dissipating_time
            .map_or(self.delay, |d| (now - d).max(self.delay));
        if now <= start {
            return Vec::new();
        }
        let mut step = 1.0 / fps.max(1) as f32;
        if (now - start) / step > MAX_TRACE_SAMPLES as f32 {
            step = (now - start) / MAX_TRACE_SAMPLES as f32;
        }
        let first = (start / step).ceil() as usize;
        let last = (now / step).floor() as usize;
        let mut times = Vec::with_capacity(last.saturating_sub(first) + 3);
        times.push(start);
        times.extend(
            (first..=last)
                .map(|k| k as f32 * step)
                .filter(|&t| t > start && t < now),
        );
        times.push(now);
        times
    }
}

impl<'a> Component<'a> for TracedPath<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let times = self.times(ui.seconds(), ui.frame.fps);
        if times.len() < 2 {
            return Svgr::empty();
        }
        let mut path = BezPath::new();
        for (i, t) in times.into_iter().enumerate() {
            let (x, y) = (self.point)(t);
            let p = (f64::from(x), f64::from(y));
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        Shape {
            path,
            fill: None,
            stroke: Some(self.stroke),
            stroke_width: self.stroke_width,
            dash: None,
            opacity: 1.0,
        }
        .draw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_every_frame_and_caps_long_trails() {
        let t = TracedPath::themed(&Theme::default());
        let times = t.times(1.0, 30);
        assert_eq!(times.first(), Some(&0.0));
        assert_eq!(times.last(), Some(&1.0));
        assert!(times.len() >= 30 && times.len() <= 32, "{}", times.len());
        assert!(t.times(100.0, 60).len() <= MAX_TRACE_SAMPLES + 2);
        let fading = TracedPath {
            dissipating_time: Some(0.5),
            ..t
        };
        assert_eq!(fading.times(2.0, 30).first(), Some(&1.5));
        assert_eq!(t.times(0.0, 30), Vec::<f32>::new());
    }
}
