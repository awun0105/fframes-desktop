//! Manim's `StreamLines`.

use std::sync::Arc;

use fframes::Svgr;
use kurbo::BezPath;

use crate::{
    animate::{EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    paths::{get_length, slice_path},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{Space, VectorField, graph::cached, smooth};

/// A number in `0..1` from `seed` and `i`, the same on every run (splitmix64).
fn hash01(seed: u64, i: u64) -> f64 {
    let mut z = seed ^ i.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (z ^ (z >> 31)) as f64 / u64::MAX as f64
}

/// Manim's `start_animation`: a piece of every line that runs along it, over and over.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Flow {
    /// The visible piece as a share of the line's length.
    pub time_width: f64,
    /// How many times per second a piece runs the length of its line.
    pub speed: f64,
}

impl Default for Flow {
    fn default() -> Self {
        Self {
            time_width: 0.3,
            speed: 0.5,
        }
    }
}

/// Lines that follow a [`VectorField`] (Manim `StreamLines`): from every grid point,
/// moved by a seeded `noise_factor`, a point moves along the field in steps of `dt` for
/// `virtual_time`, until it leaves the grid by more than `padding` units. Each line takes
/// the colour of the field's magnitude at its middle.
///
/// The lines are traced once and cached by the field's function and these props. With
/// `flow`, only a piece of every line is drawn and it runs along the line, each line from
/// its own seeded start; without it, the lines appear with `enter` (for example `Create`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct StreamLines {
    pub field: VectorField,
    pub dt: f64,
    pub virtual_time: f64,
    /// Most points kept per line; longer runs keep every n-th point.
    pub max_anchors_per_line: usize,
    pub padding: f64,
    /// How many lines start from every grid point.
    pub n_repeats: usize,
    /// How far, in units, a start moves from its grid point; `None` is half the grid step.
    pub noise_factor: Option<f64>,
    pub seed: u64,
    pub stroke_width: f32,
    pub opacity: f32,
    pub flow: Option<Flow>,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
}

impl Themed for StreamLines {
    fn themed(t: &Theme) -> Self {
        Self {
            field: VectorField::themed(t),
            dt: 0.05,
            virtual_time: 3.0,
            max_anchors_per_line: 100,
            padding: 3.0,
            n_repeats: 1,
            noise_factor: None,
            seed: 0,
            stroke_width: t.stroke.regular,
            opacity: 1.0,
            flow: None,
            enter: None,
            exit: None,
        }
    }
}

impl Default for StreamLines {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl StreamLines {
    /// The traced lines in units of `ui.space`, separated by a `(NaN, NaN)` point.
    pub fn lines(&self) -> Arc<[(f64, f64)]> {
        let field = &self.field;
        let noise = self.noise_factor.unwrap_or(field.y_range.step / 2.0);
        let params = [
            field.x_range.min,
            field.x_range.max,
            field.x_range.step,
            field.y_range.min,
            field.y_range.max,
            field.y_range.step,
            self.dt,
            self.virtual_time,
            self.padding,
            noise,
        ]
        .map(f64::to_bits)
        .into_iter()
        .chain([
            self.max_anchors_per_line as u64,
            self.n_repeats as u64,
            self.seed,
        ]);
        let key = (4, field.f as usize, params.collect());
        cached(key, || self.trace(noise).into())
    }

    fn trace(&self, noise: f64) -> Vec<(f64, f64)> {
        let field = &self.field;
        let (x, y) = (field.x_range, field.y_range);
        let inside = |px: f64, py: f64| {
            px >= x.min - self.padding
                && px <= x.max + self.padding
                && py >= y.min - self.padding
                && py <= y.max + self.padding
        };
        let steps = if self.dt > 0.0 {
            ((self.virtual_time / self.dt).ceil() as usize).min(100_000)
        } else {
            0
        };
        let stride = steps.div_ceil(self.max_anchors_per_line.max(2) - 1).max(1);
        let mut out = Vec::new();
        let starts: Vec<(f64, f64)> = field
            .grid()
            .flat_map(|p| std::iter::repeat_n(p, self.n_repeats))
            .collect();
        for (i, (sx, sy)) in starts.into_iter().enumerate() {
            let i = i as u64 * 2;
            let mut p = (
                sx + (hash01(self.seed, i) - 0.5) * 2.0 * noise,
                sy + (hash01(self.seed, i + 1) - 0.5) * 2.0 * noise,
            );
            let mut line = vec![p];
            for step in 1..=steps {
                let (vx, vy) = field.at(p.0, p.1);
                p = (p.0 + vx * self.dt, p.1 + vy * self.dt);
                if !(p.0.is_finite() && p.1.is_finite() && inside(p.0, p.1)) {
                    break;
                }
                if step % stride == 0 || step == steps {
                    line.push(p);
                }
            }
            // a start where the field is zero stays a point and draws nothing
            if line.iter().all(|&q| q == line[0]) {
                continue;
            }
            if !out.is_empty() {
                out.push((f64::NAN, f64::NAN));
            }
            out.extend(line);
        }
        out
    }

    /// Every line as a path in pixels of `space`, with its colour.
    fn paths(&self, space: Space) -> Vec<(BezPath, fframes::Color)> {
        let lines = self.lines();
        lines
            .split(|p| p.0.is_nan())
            .filter(|line| line.len() >= 2)
            .map(|line| {
                let (mx, my) = line[line.len() / 2];
                let path = smooth(line.iter().map(|&(x, y)| Some(space.to_px(x, y))));
                (path, self.field.color_at(mx, my))
            })
            .collect()
    }
}

impl<'a> Component<'a> for StreamLines {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = f64::from(ui.seconds());
        let shapes: Vec<Shape> = self
            .paths(ui.space)
            .into_iter()
            .enumerate()
            .filter_map(|(i, (path, color))| {
                let path = match self.flow {
                    None => path,
                    Some(flow) => {
                        let length = get_length(&path);
                        let width = flow.time_width.clamp(0.0, 1.0);
                        // the head runs from the start to past the end, so the tail leaves too
                        let phase = (hash01(self.seed ^ 0x5eed, i as u64) + t * flow.speed).fract();
                        let head = phase * (1.0 + width);
                        let (from, to) = ((head - width).max(0.0), head.min(1.0));
                        if to <= from {
                            return None;
                        }
                        slice_path(&path, from * length, to * length)
                    }
                };
                Some(Shape {
                    path,
                    fill: None,
                    stroke: Some(color),
                    stroke_width: self.stroke_width,
                    dash: None,
                    opacity: self.opacity,
                })
            })
            .collect();
        draw_animated(ui, &shapes, self.enter.as_ref(), self.exit.as_ref(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explain::Range;

    fn rotation() -> StreamLines {
        StreamLines {
            field: VectorField {
                f: |x, y| (-y, x),
                x_range: Range::new(-2.0, 2.0, 1.0),
                y_range: Range::new(-2.0, 2.0, 1.0),
                ..VectorField::default()
            },
            noise_factor: Some(0.0),
            virtual_time: 1.0,
            dt: 0.01,
            ..StreamLines::default()
        }
    }

    #[test]
    fn lines_follow_the_field_and_are_traced_once() {
        let lines = rotation().lines();
        // the origin does not move, every other point of the 5 x 5 grid draws a line
        let pieces: Vec<&[(f64, f64)]> = lines.split(|p| p.0.is_nan()).collect();
        assert_eq!(pieces.len(), 24);
        // a rotation keeps the distance from the middle (Euler steps drift a little)
        for piece in &pieces {
            let r0 = piece[0].0.hypot(piece[0].1);
            for &(x, y) in *piece {
                assert!((x.hypot(y) / r0 - 1.0).abs() < 0.01, "{r0} {x} {y}");
            }
            assert!(piece.len() <= 100);
        }
        assert!(Arc::ptr_eq(&lines, &rotation().lines()));
    }

    #[test]
    fn seeded_noise_moves_the_starts_the_same_way_every_time() {
        let noisy = StreamLines {
            noise_factor: Some(0.3),
            seed: 7,
            ..rotation()
        };
        let a = noisy.lines();
        assert_eq!(a.len(), noisy.trace(0.3).len());
        assert_eq!(a[0], noisy.trace(0.3)[0]);
        assert_ne!(a[0], rotation().lines()[0]);
        assert!(
            (0..100)
                .map(|i| hash01(1, i))
                .all(|h| (0.0..=1.0).contains(&h))
        );
    }
}
