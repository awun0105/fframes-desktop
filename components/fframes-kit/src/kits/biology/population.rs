//! Population growth over time: exponential, logistic (up to a carrying capacity) and the
//! cycles of predators and their prey.

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Point, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    explain::{Axes, AxesMap, Range},
    geometry::Shape,
    kits::draw::stroke,
    motion::progress,
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// How a [`PopulationCurve`] grows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopulationModel {
    /// `N = N0·e^(rt)`: no limit.
    Exponential,
    /// `N = K / (1 + (K - N0)/N0 · e^(-rt))`: growth slows as `N` nears the carrying
    /// capacity `K`.
    Logistic,
    /// Lotka-Volterra: prey grow at `prey_birth` and are eaten at `predation` per predator;
    /// predators grow at `predator_birth` per prey eaten and die at `predator_death`. The
    /// populations start at `n0` prey and `predators0` predators.
    PredatorPrey {
        prey_birth: f64,
        predation: f64,
        predator_birth: f64,
        predator_death: f64,
        predators0: f64,
    },
}

impl PopulationModel {
    /// Predators and prey that cycle about every six time units.
    pub const PREDATOR_PREY: PopulationModel = PopulationModel::PredatorPrey {
        prey_birth: 1.0,
        predation: 0.1,
        predator_birth: 0.075,
        predator_death: 1.5,
        predators0: 5.0,
    };
}

/// A round step for about four ticks up to `max`: 1, 2 or 5 times a power of ten.
fn nice_step(max: f64) -> f64 {
    let rough = (max / 4.0).max(1e-9);
    let power = 10f64.powf(rough.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * power)
        .find(|&s| s >= rough)
        .unwrap_or(10.0 * power)
}

/// Population growth on axes of time against population, `x_length` by `y_length` (units of
/// `ui.space`) around `center`. From `delay` the curves draw from 0 to `years` in `duration`
/// seconds, a dot at each head; curves that leave the top stop there.
///
/// - [`PopulationModel::Logistic`]: the population grows from `n0` at rate `r` and levels
///   off at the dashed carrying capacity `k`; with `compare` the exponential curve with the
///   same start and rate is drawn faded for comparison.
/// - [`PopulationModel::Exponential`]: the same curve without a limit.
/// - [`PopulationModel::PredatorPrey`]: prey and predators rise and fall in turn.
///
/// A legend names each curve in its colour from `colors`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct PopulationCurve {
    pub model: PopulationModel,
    pub r: f64,
    pub k: f64,
    pub n0: f64,
    pub years: f64,
    pub compare: bool,
    pub delay: f32,
    pub duration: f32,
    pub center: (f64, f64),
    pub x_length: f64,
    pub y_length: f64,
    pub colors: [Color; 2],
    pub axis_color: Color,
    pub label_style: TextStyle,
    pub text_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for PopulationCurve {
    fn themed(t: &Theme) -> Self {
        let mut label_style = t.typography.caption;
        label_style.weight = 600;
        Self {
            model: PopulationModel::Logistic,
            r: 0.8,
            k: 1000.0,
            n0: 10.0,
            years: 15.0,
            compare: true,
            delay: 0.0,
            duration: 5.0,
            center: (-0.8, -0.3),
            x_length: 8.5,
            y_length: 5.2,
            colors: [Color::hex("#69db7c"), Color::hex("#ff8787")],
            axis_color: t.colors.muted,
            label_style,
            text_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for PopulationCurve {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl PopulationCurve {
    /// The populations at `count + 1` evenly spaced times from 0 to `years`: one series, or
    /// prey and then predators.
    pub fn series(&self, count: usize) -> Vec<Vec<f64>> {
        let count = count.max(1);
        let time = |i: usize| self.years * i as f64 / count as f64;
        match self.model {
            PopulationModel::Exponential => {
                vec![
                    (0..=count)
                        .map(|i| self.n0 * (self.r * time(i)).exp())
                        .collect(),
                ]
            }
            PopulationModel::Logistic => vec![
                (0..=count)
                    .map(|i| {
                        let a = (self.k - self.n0) / self.n0.max(1e-9);
                        self.k / (1.0 + a * (-self.r * time(i)).exp())
                    })
                    .collect(),
            ],
            PopulationModel::PredatorPrey {
                prey_birth,
                predation,
                predator_birth,
                predator_death,
                predators0,
            } => {
                // fourth-order Runge-Kutta, eight steps between samples
                let rate = |(x, y): (f64, f64)| {
                    (
                        prey_birth * x - predation * x * y,
                        predator_birth * x * y - predator_death * y,
                    )
                };
                let h = self.years / (count * 8) as f64;
                let mut state = (self.n0, predators0);
                let (mut prey, mut predators) = (vec![state.0], vec![state.1]);
                for _ in 0..count {
                    for _ in 0..8 {
                        let (x, y) = state;
                        let k1 = rate(state);
                        let k2 = rate((x + h / 2.0 * k1.0, y + h / 2.0 * k1.1));
                        let k3 = rate((x + h / 2.0 * k2.0, y + h / 2.0 * k2.1));
                        let k4 = rate((x + h * k3.0, y + h * k3.1));
                        state = (
                            x + h / 6.0 * (k1.0 + 2.0 * k2.0 + 2.0 * k3.0 + k4.0),
                            y + h / 6.0 * (k1.1 + 2.0 * k2.1 + 2.0 * k3.1 + k4.1),
                        );
                    }
                    prey.push(state.0);
                    predators.push(state.1);
                }
                vec![prey, predators]
            }
        }
    }

    /// The top of the population axis.
    fn top(&self, series: &[Vec<f64>]) -> f64 {
        // room above the highest point, or above the carrying capacity
        let room = if let PopulationModel::PredatorPrey { .. } = self.model {
            series.iter().flatten().copied().fold(0.0, f64::max) * 1.1
        } else {
            self.k * 1.25
        };
        let step = nice_step(room);
        (room / step).ceil() * step
    }
}

/// The curve through `values` (one per sample, ending at `upto` samples, fractional), cut
/// where it leaves the top of `map`. The second value is whether it left.
fn trace(map: &AxesMap, years: f64, values: &[f64], upto: f64) -> (Vec<Point>, bool) {
    let count = values.len() - 1;
    let top = map.y.max;
    let at = |i: f64| {
        let lo = (i.floor() as usize).min(count);
        let hi = (lo + 1).min(count);
        let f = i - lo as f64;
        (
            years * i / count as f64,
            values[lo] + (values[hi] - values[lo]) * f,
        )
    };
    let mut points = Vec::new();
    let whole = upto.floor() as usize;
    let mut stops: Vec<f64> = (0..=whole.min(count)).map(|i| i as f64).collect();
    if upto > whole as f64 {
        stops.push(upto);
    }
    let mut last = None;
    for i in stops {
        let (x, y) = at(i);
        if y > top {
            // cut at the top, between the last point and this one
            if let Some((lx, ly)) = last {
                let f = (top - ly) / (y - ly);
                points.push(map.c2p(lx + (x - lx) * f, top));
            }
            return (points, true);
        }
        points.push(map.c2p(x, y));
        last = Some((x, y));
    }
    (points, false)
}

fn path_of(points: &[Point]) -> BezPath {
    let mut path = BezPath::new();
    for (i, &p) in points.iter().enumerate() {
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path
}

fn dot(at: Point, color: Color, opacity: f32) -> Svgr<'static> {
    Shape {
        path: Circle::new(at, 8.0).to_path(0.1),
        fill: Some(color),
        stroke: None,
        stroke_width: 0.0,
        dash: None,
        opacity,
    }
    .draw()
}

impl<'a> Component<'a> for PopulationCurve {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        const SAMPLES: usize = 240;
        let series = self.series(SAMPLES);
        let top = self.top(&series);
        let x_range = Range::new(0.0, self.years, nice_step(self.years));
        let y_range = Range::new(0.0, top, nice_step(top));
        let axes = Axes {
            x_range,
            y_range,
            x_length: self.x_length,
            y_length: self.y_length,
            center: self.center,
            color: self.axis_color,
            x_numbers: x_range.ticks(),
            y_numbers: y_range.ticks(),
            x_label: Some("time".into()),
            y_label: Some("population".into()),
            label_style: self.label_style,
            ..ui.defaults()
        };
        let map = axes.map(ui);
        let t = ui.seconds() - self.delay;
        let upto = SAMPLES as f64 * f64::from(progress(t, 0.0, self.duration));

        let mut out = Vec::new();
        let mut names: Vec<(&str, Color, bool)> = Vec::new();
        if self.model == PopulationModel::Logistic {
            let (a, b) = (map.c2p(0.0, self.k), map.c2p(self.years, self.k));
            let mut limit = stroke(path_of(&[a, b]), self.text_color, 2.0, 0.7);
            limit.dash = Some((10.0, 8.0));
            out.push(limit.draw());
            out.push(line(
                b.x as f32,
                (b.y - 14.0) as f32,
                self.label_style,
                self.text_color,
                TextAnchor::End,
                0.0,
                0.8,
                "carrying capacity K".into(),
            ));
            if self.compare {
                let ghost = PopulationCurve {
                    model: PopulationModel::Exponential,
                    ..self.clone()
                }
                .series(SAMPLES);
                let (points, left) = trace(&map, self.years, &ghost[0], upto);
                let color = self.colors[1];
                if t > 0.0 && points.len() > 1 {
                    let mut curve = stroke(path_of(&points), color, 4.0, 0.6);
                    curve.dash = Some((12.0, 8.0));
                    out.push(curve.draw());
                }
                if let (true, Some(&head)) = (t > 0.0, points.last()) {
                    out.push(dot(head, color, if left { 0.3 } else { 0.6 }));
                }
                names.push(("exponential (no limit)", color, true));
            }
        }
        let curve_names: &[&str] = match self.model {
            PopulationModel::Exponential => &["exponential"],
            PopulationModel::Logistic => &["logistic"],
            PopulationModel::PredatorPrey { .. } => &["prey", "predators"],
        };
        for (i, values) in series.iter().enumerate() {
            let color = self.colors[i.min(1)];
            let (points, left) = trace(&map, self.years, values, upto);
            if t > 0.0 && points.len() > 1 {
                out.push(stroke(path_of(&points), color, 5.0, 1.0).draw());
            }
            if let (true, Some(&head)) = (t > 0.0, points.last()) {
                out.push(dot(head, color, if left { 0.5 } else { 1.0 }));
            }
            names.insert(i, (curve_names[i], color, false));
        }

        // the legend, at the top left inside the axes, under the axis label
        let corner = map.c2p(0.0, top);
        let row = self.label_style.size as f64 * 1.5;
        for (i, (name, color, dashed)) in names.into_iter().enumerate() {
            let y = corner.y + 80.0 + row * i as f64;
            let x = corner.x + 40.0;
            let mid = y - self.label_style.size as f64 * 0.35;
            let mut swatch = stroke(
                path_of(&[Point::new(x, mid), Point::new(x + 36.0, mid)]),
                color,
                5.0,
                1.0,
            );
            if dashed {
                swatch.dash = Some((8.0, 5.0));
            }
            out.push(swatch.draw());
            out.push(line(
                (x + 50.0) as f32,
                y as f32,
                self.label_style,
                color,
                TextAnchor::Start,
                0.0,
                1.0,
                name.into(),
            ));
        }

        let axes = ui.show(axes);
        let parts: Svgr<'a> = vec![axes, out.into()].into();
        fade(ui, parts, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logistic_growth_levels_off_at_the_carrying_capacity() {
        let curve = PopulationCurve::default();
        let logistic = &curve.series(100)[0];
        assert_eq!(logistic[0], curve.n0);
        assert!(logistic.windows(2).all(|w| w[0] < w[1]));
        assert!((logistic[100] - curve.k).abs() < curve.k * 0.01);
        let exponential = &PopulationCurve {
            model: PopulationModel::Exponential,
            ..PopulationCurve::default()
        }
        .series(100)[0];
        assert!(exponential[100] > curve.k * 100.0);
    }

    #[test]
    fn predators_and_prey_cycle() {
        let curve = PopulationCurve {
            model: PopulationModel::PREDATOR_PREY,
            years: 20.0,
            ..PopulationCurve::default()
        };
        let series = curve.series(400);
        let (prey, predators) = (&series[0], &series[1]);
        // both rise and fall more than once, and stay positive
        let turns = |v: &[f64]| v.windows(3).filter(|w| w[1] > w[0] && w[1] > w[2]).count();
        assert!(turns(prey) >= 2 && turns(predators) >= 2);
        assert!(prey.iter().chain(predators).all(|&n| n > 0.0));
        // the predators peak after the prey
        let peak = |v: &[f64]| (1..v.len() - 1).find(|&i| v[i] > v[i - 1] && v[i] > v[i + 1]);
        assert!(peak(predators) > peak(prey));
    }

    #[test]
    fn steps_are_round() {
        assert_eq!(nice_step(15.0), 5.0);
        assert_eq!(nice_step(1250.0), 500.0);
        assert_eq!(nice_step(70.0), 20.0);
    }
}
