//! Growth rates racing on the same axes.

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Point};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    explain::{Axes, Range},
    geometry::Shape,
    motion::progress,
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::keyed::stroke;

/// How the steps of an algorithm grow with the input size `n`. Logarithms are base 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Growth {
    Constant,
    Log,
    Linear,
    NLogN,
    Quadratic,
    Cubic,
    Exponential,
    Factorial,
}

impl Growth {
    /// All of them, slowest first.
    pub const ALL: [Growth; 8] = [
        Growth::Constant,
        Growth::Log,
        Growth::Linear,
        Growth::NLogN,
        Growth::Quadratic,
        Growth::Cubic,
        Growth::Exponential,
        Growth::Factorial,
    ];

    /// The steps for an input of size `n` (`n!` between whole numbers follows the
    /// logarithm of the two around it). Below 1 the logarithm counts as 0.
    pub fn eval(self, n: f64) -> f64 {
        let log = n.max(1.0).log2();
        match self {
            Growth::Constant => 1.0,
            Growth::Log => log,
            Growth::Linear => n,
            Growth::NLogN => n * log,
            Growth::Quadratic => n * n,
            Growth::Cubic => n * n * n,
            Growth::Exponential => n.exp2(),
            Growth::Factorial => {
                let fact = |k: f64| (2..=k as u64).map(|i| i as f64).product::<f64>();
                let k = n.max(0.0).floor();
                let (a, b) = (fact(k), fact(k + 1.0));
                if n <= k {
                    return a;
                }
                (a.ln() + (b.ln() - a.ln()) * (n - k)).exp()
            }
        }
    }

    /// Big O notation: `O(n log n)`, in characters every font has (`O(2^n)`).
    pub fn label(self) -> &'static str {
        match self {
            Growth::Constant => "O(1)",
            Growth::Log => "O(log n)",
            Growth::Linear => "O(n)",
            Growth::NLogN => "O(n log n)",
            Growth::Quadratic => "O(n²)",
            Growth::Cubic => "O(n³)",
            Growth::Exponential => "O(2^n)",
            Growth::Factorial => "O(n!)",
        }
    }

    fn index(self) -> usize {
        Growth::ALL.iter().position(|&g| g == self).unwrap_or(0)
    }
}

/// A count in a few characters: `1,024`, `3.2e9` from ten digits on.
fn count(value: f64) -> String {
    if !value.is_finite() || value >= 1e9 {
        return format!("{value:.1e}");
    }
    let whole = value.round() as u64;
    let digits = whole.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The pixels of `growth` from `n_range.min` to `n`, ending where it leaves the top of
/// `axes`. The second value is whether it left.
fn curve(
    axes: &crate::explain::AxesMap,
    growth: Growth,
    n: f64,
    samples: usize,
) -> (Vec<Point>, bool) {
    let (lo, top) = (axes.x.min, axes.y.max);
    let mut points = Vec::with_capacity(samples + 1);
    let mut last = (lo, growth.eval(lo));
    for i in 0..=samples {
        let x = lo + (n - lo) * i as f64 / samples as f64;
        let y = growth.eval(x);
        if y > top {
            // where it crosses the top, between the last sample and this one
            let t = if y > last.1 {
                ((top - last.1) / (y - last.1)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let cross = last.0 + (x - last.0) * t;
            points.push(axes.c2p(cross, top));
            return (points, true);
        }
        points.push(axes.c2p(x, y.max(axes.y.min)));
        last = (x, y);
    }
    (points, false)
}

/// Growth rates racing: the curves of `growths` drawn on axes `x_length` by `y_length` (units
/// of `ui.space`) around `center`, `n` along `n_range` and the steps along `steps`. From `delay`,
/// `n` sweeps from the start of its range to the end in `duration` seconds and every curve
/// grows with it, a dot at its head; a curve that leaves the top stops there. A legend beside
/// the axes lists each rate in its colour from `palette` (one colour per [`Growth`], slowest
/// first) with its steps at the current `n`. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ComplexityPlot {
    pub growths: Vec<Growth>,
    pub delay: f32,
    pub duration: f32,
    pub center: (f64, f64),
    pub x_length: f64,
    pub y_length: f64,
    pub n_range: Range,
    pub steps: Range,
    pub legend: bool,
    pub palette: [Color; 8],
    pub axis_color: Color,
    pub label_style: TextStyle,
    pub value_style: TextStyle,
    pub text_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for ComplexityPlot {
    fn themed(t: &Theme) -> Self {
        let mut label_style = t.typography.caption;
        label_style.weight = 600;
        Self {
            growths: vec![
                Growth::Log,
                Growth::Linear,
                Growth::NLogN,
                Growth::Quadratic,
                Growth::Exponential,
            ],
            delay: 0.0,
            duration: 4.0,
            center: (-1.2, 0.0),
            x_length: 8.0,
            y_length: 5.0,
            n_range: Range::new(0.0, 20.0, 5.0),
            steps: Range::new(0.0, 100.0, 25.0),
            legend: true,
            palette: [
                Color::hex("#69db7c"),
                Color::hex("#38d9a9"),
                t.colors.accent,
                Color::hex("#9775fa"),
                t.colors.highlight,
                Color::hex("#ff922b"),
                Color::hex("#ff6b6b"),
                Color::hex("#f06595"),
            ],
            axis_color: t.colors.muted,
            label_style,
            value_style: t.typography.caption,
            text_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for ComplexityPlot {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl ComplexityPlot {
    /// `n` at `seconds` into the race.
    pub fn n_at(&self, seconds: f32) -> f64 {
        let p = f64::from(progress(seconds, self.delay, self.duration));
        self.n_range.min + (self.n_range.max - self.n_range.min) * p
    }
}

impl<'a> Component<'a> for ComplexityPlot {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let axes = Axes {
            x_range: self.n_range,
            y_range: self.steps,
            x_length: self.x_length,
            y_length: self.y_length,
            center: self.center,
            color: self.axis_color,
            x_numbers: self.n_range.ticks(),
            y_numbers: self.steps.ticks(),
            x_label: Some("n".into()),
            y_label: Some("steps".into()),
            label_style: self.label_style,
            ..ui.defaults()
        };
        let map = axes.map(ui);
        let n = self.n_at(ui.seconds());
        let started = ui.seconds() > self.delay;

        let mut curves = Vec::new();
        let mut heads = Vec::new();
        for &growth in &self.growths {
            let color = self.palette[growth.index()];
            let (points, left) = curve(&map, growth, n, 160);
            if started && points.len() > 1 {
                let mut path = BezPath::new();
                path.move_to(points[0]);
                for &p in &points[1..] {
                    path.line_to(p);
                }
                curves.push(stroke(path, color, 4.0, 1.0).draw());
            }
            if let (true, Some(&head)) = (started, points.last()) {
                heads.push(
                    Shape {
                        path: kurbo::Shape::to_path(&Circle::new(head, 8.0), 0.1),
                        fill: Some(color),
                        stroke: None,
                        stroke_width: 0.0,
                        dash: None,
                        opacity: if left { 0.5 } else { 1.0 },
                    }
                    .draw(),
                );
            }
        }

        let mut legend = Vec::new();
        if self.legend {
            let right = map.c2p(self.n_range.max, self.steps.max);
            let x = right.x + 70.0;
            let row = self.label_style.size as f64 * 1.6;
            let value_x = x + 300.0;
            let mut y = right.y + row * 0.4;
            legend.push(line(
                x as f32,
                y as f32,
                self.label_style,
                self.text_color,
                TextAnchor::Start,
                0.0,
                1.0,
                format!("n = {}", n.round()).into(),
            ));
            for &growth in &self.growths {
                y += row;
                let color = self.palette[growth.index()];
                let mut swatch = BezPath::new();
                let mid = y - self.label_style.size as f64 * 0.35;
                swatch.move_to((x, mid));
                swatch.line_to((x + 32.0, mid));
                legend.push(stroke(swatch, color, 5.0, 1.0).draw());
                legend.push(line(
                    (x + 46.0) as f32,
                    y as f32,
                    self.label_style,
                    color,
                    TextAnchor::Start,
                    0.0,
                    1.0,
                    growth.label().into(),
                ));
                legend.push(line(
                    value_x as f32,
                    y as f32,
                    self.value_style,
                    self.text_color,
                    TextAnchor::End,
                    0.0,
                    1.0,
                    count(growth.eval(n)).into(),
                ));
            }
        }

        let axes = ui.show(axes);
        let parts: Svgr<'a> = vec![axes, curves.into(), heads.into(), legend.into()].into();
        fade(ui, parts, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growths_keep_their_order() {
        for n in [16.0, 20.0] {
            let values: Vec<f64> = Growth::ALL.iter().map(|g| g.eval(n)).collect();
            assert!(values.windows(2).all(|w| w[0] <= w[1]), "{n}: {values:?}");
        }
        assert_eq!(Growth::Factorial.eval(5.0), 120.0);
        assert!((Growth::Factorial.eval(4.5) - (24.0_f64 * 120.0).sqrt()).abs() < 1e-9);
        assert_eq!(Growth::Log.eval(0.5), 0.0);
        assert_eq!(Growth::NLogN.label(), "O(n log n)");
    }

    #[test]
    fn counts_are_short() {
        assert_eq!(count(7.4), "7");
        assert_eq!(count(1024.0), "1,024");
        assert_eq!(count(1_048_576.0), "1,048,576");
        assert_eq!(count(3.2e9), "3.2e9");
    }

    #[test]
    fn curves_stop_at_the_top() {
        let axes = Axes {
            x_range: Range::new(0.0, 20.0, 5.0),
            y_range: Range::new(0.0, 100.0, 25.0),
            x_length: 8.0,
            y_length: 5.0,
            ..Axes::default()
        };
        let map = axes.map_in(crate::explain::Space::FULL_HD);
        let (square, left) = curve(&map, Growth::Quadratic, 20.0, 160);
        assert!(left);
        let top = map.c2p(0.0, 100.0).y;
        assert!((square.last().unwrap().y - top).abs() < 1e-6);
        // it crosses at n = 10
        assert!((map.p2c(*square.last().unwrap()).0 - 10.0).abs() < 1e-6);
        let (line, left) = curve(&map, Growth::Linear, 20.0, 160);
        assert!(!left);
        assert_eq!(line.len(), 161);
    }
}
