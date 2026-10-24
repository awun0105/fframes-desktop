use std::fmt::Write as _;

use fframes::{Color, Svgr};

use crate::{
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// The two control points of each segment of a smooth cubic spline through `points` (one
/// axis), solved with the Thomas algorithm
/// (<https://www.particleincell.com/2012/bezier-splines/>). Needs at least two points.
fn control_points(points: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let n = points.len() - 1;
    let mut p1 = vec![0.0; n + 1];
    let mut p2 = vec![0.0; n + 1];
    let mut a = vec![0.0; n + 1];
    let mut b = vec![0.0; n + 1];
    let mut c = vec![0.0; n + 1];
    let mut r = vec![0.0; n + 1];

    b[0] = 2.0;
    c[0] = 1.0;
    r[0] = points[0] + 2.0 * points[1];
    for i in 1..n {
        a[i] = 1.0;
        b[i] = 4.0;
        c[i] = 1.0;
        r[i] = 4.0 * points[i] + 2.0 * points[i + 1];
    }
    a[n - 1] = 2.0;
    b[n - 1] = 7.0;
    c[n - 1] = 0.0;
    r[n - 1] = 8.0 * points[n - 1] + points[n];

    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m * c[i - 1];
        r[i] -= m * r[i - 1];
    }
    p1[n - 1] = r[n - 1] / b[n - 1];
    for i in (0..n).rev() {
        p1[i] = (r[i] - c[i] * p1[i + 1]) / b[i];
    }
    for i in 0..n - 1 {
        p2[i] = 2.0 * points[i + 1] - p1[i + 1];
    }
    p2[n - 1] = f32::midpoint(points[n], p1[n - 1]);
    (p1, p2)
}

/// A closed path of a smooth wave through `values`: it starts and ends on the baseline `y`
/// at `x` and `x + width`, the values are spread evenly between, and a value rises
/// `value * gain` pixels, at most `max_height`.
pub fn wave_path(x: f32, y: f32, width: f32, gain: f32, max_height: f32, values: &[f32]) -> String {
    let step = width / (values.len() + 1) as f32;
    let (xs, ys): (Vec<f32>, Vec<f32>) = std::iter::once(0.0)
        .chain(values.iter().copied())
        .chain(std::iter::once(0.0))
        .enumerate()
        .map(|(i, v)| {
            let h = (v * gain).min(max_height);
            (x + i as f32 * step, if h.is_nan() { 0.0 } else { h })
        })
        .unzip();
    let (x1, x2) = control_points(&xs);
    let (y1, y2) = control_points(&ys);
    let mut d = format!("M {} {}", xs[0], y - ys[0]);
    for i in 1..xs.len() {
        let _ = write!(
            d,
            " C {} {} {} {} {} {}",
            x1[i - 1],
            y - y1[i - 1],
            x2[i - 1],
            y - y2[i - 1],
            xs[i],
            y - ys[i]
        );
    }
    d.push_str(" Z");
    d
}

/// The audio spectrum (or any values) as a filled smooth wave over a baseline, like the
/// layered waves of an audio announcement. See [`wave_path`] for the geometry.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Waveform {
    pub values: Vec<f32>,
    pub x: f32,
    /// The baseline.
    pub y: f32,
    pub width: f32,
    pub gain: f32,
    pub max_height: f32,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
    pub opacity: f32,
}

impl Themed for Waveform {
    fn themed(t: &Theme) -> Self {
        Self {
            values: Vec::new(),
            x: 0.0,
            y: 1080.0,
            width: 1920.0,
            gain: 1200.0,
            max_height: 600.0,
            fill: Some(t.colors.accent),
            stroke: None,
            stroke_width: t.stroke.thin,
            opacity: 1.0,
        }
    }
}

impl Default for Waveform {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Waveform {
    fn render(self, _ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let d = wave_path(
            self.x,
            self.y,
            self.width,
            self.gain,
            self.max_height,
            &self.values,
        );
        let fill = self.fill.unwrap_or(Color::TRANSPARENT);
        let stroke = self.stroke.unwrap_or(Color::TRANSPARENT);
        fframes::svgr!(
            <path d={d} fill={fill} stroke={stroke} stroke-width={self.stroke_width} opacity={self.opacity} />
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wave_starts_and_ends_on_the_baseline() {
        let d = wave_path(100.0, 500.0, 300.0, 100.0, 50.0, &[0.2, 1.0]);
        assert!(d.starts_with("M 100 500 C "), "{d}");
        assert!(d.ends_with(" 400 500 Z"), "{d}");
        // the second value is capped at 50 px over the baseline
        assert!(d.contains(" 300 450"), "{d}");
        assert!(wave_path(0.0, 0.0, 10.0, 1.0, 1.0, &[]).ends_with(" 10 0 Z"));
    }
}
