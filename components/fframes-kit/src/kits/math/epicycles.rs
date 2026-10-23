//! Fourier series of a closed path and [`Epicycles`], the chain of circles that draws it.

use std::f64::consts::TAU;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point};

use crate::{
    anim::ValueTracker,
    animate::{EnterAnim, ExitAnim, draw_animated},
    paths,
    shapes::{Circle, Dot, Line},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// One term of a Fourier series: a vector `(re, im)` turning `freq` times per turn of the
/// series, counterclockwise for a positive `freq`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Term {
    pub freq: i32,
    pub re: f64,
    pub im: f64,
}

impl Term {
    /// The radius of the term's circle.
    pub fn amplitude(&self) -> f64 {
        self.re.hypot(self.im)
    }

    /// The vector at `turns` turns of the series.
    fn at(&self, turns: f64) -> (f64, f64) {
        let (sin, cos) = (TAU * f64::from(self.freq) * turns).sin_cos();
        (self.re * cos - self.im * sin, self.re * sin + self.im * cos)
    }
}

/// The `terms` largest terms of the Fourier series of the closed `path`, largest first.
/// The path is read y down like SVG and the terms are y up, relative to the path's mean
/// point (the constant term is left out, so the drawing is centered on it). The path is
/// resampled at `samples` points evenly spaced along its length. They tell the frequencies
/// up to `(samples - 1) / 2` apart, so there are at most that many pairs of terms; an odd
/// `samples` with every term redraws the samples exactly. Empty for an empty path.
pub fn fourier(path: &BezPath, samples: usize, terms: usize) -> Vec<Term> {
    let length = paths::get_length(path);
    if samples < 2 || length <= 0.0 || !length.is_finite() {
        return Vec::new();
    }
    // `samples` points at `i * length / samples`: the closing point is the first one
    let points: Vec<(f64, f64)> = (0..samples)
        .filter_map(|i| paths::get_point_at_length(path, length * i as f64 / samples as f64))
        .map(|p| (p.x, -p.y))
        .collect();
    if points.len() < 2 {
        return Vec::new();
    }
    let n = points.len() as f64;
    let mean = points
        .iter()
        .fold((0.0, 0.0), |(x, y), p| (x + p.0 / n, y + p.1 / n));
    let max_freq = (points.len() - 1) / 2;
    let mut series: Vec<Term> = (1..=max_freq as i32)
        .flat_map(|k| [k, -k])
        .map(|freq| {
            let (mut re, mut im) = (0.0, 0.0);
            for (i, &(x, y)) in points.iter().enumerate() {
                let (sin, cos) = (-TAU * f64::from(freq) * i as f64 / n).sin_cos();
                let (x, y) = (x - mean.0, y - mean.1);
                re += x * cos - y * sin;
                im += x * sin + y * cos;
            }
            Term {
                freq,
                re: re / n,
                im: im / n,
            }
        })
        .collect();
    series.sort_by(|a, b| b.amplitude().total_cmp(&a.amplitude()));
    series.truncate(terms);
    series
}

/// The end of the chain of `terms` at `turns` turns, relative to its center.
pub fn tip(terms: &[Term], turns: f64) -> (f64, f64) {
    terms.iter().fold((0.0, 0.0), |(x, y), term| {
        let (dx, dy) = term.at(turns);
        (x + dx, y + dy)
    })
}

/// A chain of turning circles whose end draws a closed path (Manim's `FourierCirclesScene`,
/// the 3Blue1Brown Fourier series). `terms` come from [`fourier`], largest first, and are
/// placed with the series' center at `center` (units of `ui.space`, y up), `scale` units
/// for one unit of the series. `turns` (a tracker) is how far the series has turned; the
/// trace is what the end drew in the last `trace_length` turns (1 keeps the whole drawing).
/// The circles and vectors draw with `enter`.
#[derive(Debug, Clone)]
pub struct Epicycles<'a> {
    pub terms: &'a [Term],
    pub center: (f64, f64),
    pub scale: f64,
    pub turns: ValueTracker<'a>,
    pub circles: bool,
    pub vectors: bool,
    pub circle_color: Color,
    pub vector_color: Color,
    pub trace_color: Color,
    pub stroke_width: f32,
    pub trace_width: f32,
    pub trace_length: f64,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
}

impl Themed for Epicycles<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            terms: &[],
            center: (0.0, 0.0),
            scale: 1.0,
            turns: 0.0.into(),
            circles: true,
            vectors: true,
            circle_color: t.colors.muted,
            vector_color: t.colors.text,
            trace_color: t.colors.highlight,
            stroke_width: t.stroke.thin,
            trace_width: t.stroke.thick,
            trace_length: 1.0,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Epicycles<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Trace points per turn of the series.
const TRACE_SAMPLES: f64 = 360.0;

impl Epicycles<'_> {
    /// Pixels of a point of the series.
    fn px(&self, ui: &Ui<'_, '_, '_>, (x, y): (f64, f64)) -> Point {
        ui.space.to_px(
            self.center.0 + x * self.scale,
            self.center.1 + y * self.scale,
        )
    }

    /// The turns the trace covers at `turns`: from `turns - trace_length` (not before 0) to
    /// `turns`, evenly at about [`TRACE_SAMPLES`] per turn and ending exactly at `turns`.
    fn trace_turns(&self, turns: f64) -> Vec<f64> {
        if turns <= 0.0 || !turns.is_finite() {
            return Vec::new();
        }
        let from = (turns - self.trace_length.max(0.0)).max(0.0);
        let steps = ((turns - from) * TRACE_SAMPLES).ceil().max(1.0) as usize;
        (0..=steps)
            .map(|i| from + (turns - from) * i as f64 / steps as f64)
            .collect()
    }
}

impl<'a> Component<'a> for Epicycles<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let turns = f64::from(ui.sample(&self.turns));
        let mut trace = BezPath::new();
        for (i, at) in self.trace_turns(turns).into_iter().enumerate() {
            let p = self.px(ui, tip(self.terms, at));
            if i == 0 {
                trace.move_to(p);
            } else {
                trace.line_to(p);
            }
        }
        let trace = if trace.elements().len() > 1 {
            let d = paths::serialize(&trace);
            fframes::svgr!(
                <path d={d} fill="none" stroke={self.trace_color} stroke-width={self.trace_width}
                      stroke-linecap="round" stroke-linejoin="round" />
            )
        } else {
            Svgr::empty()
        };

        let unit = f64::from(ui.space.unit) * self.scale;
        let mut chain = Vec::new();
        let mut from = (0.0, 0.0);
        for term in self.terms {
            let (dx, dy) = term.at(turns);
            let to = (from.0 + dx, from.1 + dy);
            let (a, b) = (self.px(ui, from), self.px(ui, to));
            if self.circles {
                chain.push(
                    Circle {
                        cx: a.x as f32,
                        cy: a.y as f32,
                        radius: (term.amplitude() * unit) as f32,
                        stroke: Some(self.circle_color),
                        stroke_width: self.stroke_width,
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
            if self.vectors {
                chain.push(
                    Line {
                        x1: a.x as f32,
                        y1: a.y as f32,
                        x2: b.x as f32,
                        y2: b.y as f32,
                        stroke: Some(self.vector_color),
                        stroke_width: self.stroke_width,
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
            from = to;
        }
        if !self.terms.is_empty() {
            let end = self.px(ui, from);
            chain.push(
                Dot {
                    cx: end.x as f32,
                    cy: end.y as f32,
                    radius: 8.0,
                    fill: Some(self.trace_color),
                    ..ui.defaults()
                }
                .shape(),
            );
        }
        let chain = draw_animated(ui, &chain, self.enter.as_ref(), self.exit.as_ref(), None);
        fframes::svgr!(<g>{trace}{chain}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle_path(cx: f64, cy: f64, r: f64, clockwise_on_screen: bool) -> BezPath {
        let mut path = BezPath::new();
        let sign = if clockwise_on_screen { 1.0 } else { -1.0 };
        for i in 0..=64 {
            let a = TAU * f64::from(i) / 64.0;
            let p = Point::new(cx + r * a.cos(), cy + sign * r * a.sin());
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        path.close_path();
        path
    }

    #[test]
    fn a_circle_is_one_term_turning_its_way() {
        // counterclockwise on screen (y up): one term at +1 with the circle's radius
        let terms = fourier(&circle_path(500.0, 300.0, 2.0, false), 128, 5);
        assert_eq!(terms.len(), 5);
        assert_eq!(terms[0].freq, 1);
        assert!((terms[0].amplitude() - 2.0).abs() < 0.01, "{terms:?}");
        assert!(terms[1..].iter().all(|t| t.amplitude() < 0.01));
        // the other way round is the frequency -1
        let terms = fourier(&circle_path(0.0, 0.0, 1.0, true), 128, 1);
        assert_eq!(terms[0].freq, -1);
    }

    #[test]
    fn the_chain_ends_on_the_path() {
        let mut square = BezPath::new();
        square.move_to((0.0, 0.0));
        square.line_to((2.0, 0.0));
        square.line_to((2.0, 2.0));
        square.line_to((0.0, 2.0));
        square.close_path();
        // every term of 257 samples redraws each sample exactly
        let terms = fourier(&square, 257, 999);
        assert_eq!(terms.len(), 256);
        // the start of the path is near (-1, 1) from the mean point, in y up
        let start = tip(&terms, 0.0);
        assert!((start.0 + 1.0).abs() < 1e-3 && (start.1 - 1.0).abs() < 1e-3);
        // sample 64 is 64 / 257 of the 8 long outline further along the top edge
        let (x, y) = tip(&terms, 64.0 / 257.0);
        let along = 8.0 * 64.0 / 257.0;
        assert!((x - start.0 - along).abs() < 1e-9 && (y - start.1).abs() < 1e-9);
        assert!(
            terms
                .windows(2)
                .all(|w| w[0].amplitude() >= w[1].amplitude())
        );
        assert_eq!(fourier(&BezPath::new(), 64, 8), Vec::new());
        assert_eq!(fourier(&square, 8, 99).len(), 6);
    }

    #[test]
    fn the_trace_covers_the_last_turns() {
        let chain = Epicycles {
            trace_length: 0.5,
            ..Epicycles::default()
        };
        assert_eq!(chain.trace_turns(0.0), Vec::<f64>::new());
        let turns = chain.trace_turns(2.0);
        assert_eq!((turns[0], *turns.last().unwrap()), (1.5, 2.0));
        assert_eq!(turns.len(), 181);
        let whole = Epicycles::default().trace_turns(0.25);
        assert_eq!((whole[0], whole.len()), (0.0, 91));
    }
}
