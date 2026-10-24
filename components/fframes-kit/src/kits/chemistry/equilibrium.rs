use fframes::{Color, Svgr};
use kurbo::{BezPath, Point};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    kits::{label, short},
    layout::Anchor,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// One stretch of time in which the concentrations relax from `start` to `end`.
#[derive(Debug, Clone, PartialEq)]
struct Stretch {
    at: f64,
    start: Vec<f64>,
    end: Vec<f64>,
}

/// A reaction in a closed vessel settling into equilibrium. Each species has a name, a
/// coefficient (negative for a reactant, positive for a product) and a starting
/// concentration in mol/L. From the start the concentrations move along the reaction to
/// where the reaction quotient `Q` equals `k`, closing in by `e` every `settle` seconds.
/// [`EquilibriumModel::disturb`] adds to (or takes from) a species at a later time, and the
/// mixture settles again, as Le Chatelier's principle says.
#[derive(Debug, Clone, PartialEq)]
pub struct EquilibriumModel {
    names: Vec<String>,
    coefficients: Vec<i32>,
    k: f64,
    settle: f64,
    stretches: Vec<Stretch>,
}

impl EquilibriumModel {
    pub fn new(species: &[(&str, i32, f64)], k: f64, settle: f64) -> Self {
        let coefficients: Vec<i32> = species.iter().map(|s| s.1).collect();
        let start: Vec<f64> = species.iter().map(|s| s.2.max(0.0)).collect();
        let end = settled(&coefficients, &start, k);
        Self {
            names: species.iter().map(|s| s.0.to_owned()).collect(),
            coefficients,
            k,
            settle: settle.max(1e-6),
            stretches: vec![Stretch {
                at: 0.0,
                start,
                end,
            }],
        }
    }

    /// Changes species `index` by `delta` mol/L at `at` seconds (after the last change);
    /// the mixture then settles again.
    #[must_use]
    pub fn disturb(mut self, at: f64, index: usize, delta: f64) -> Self {
        let last = self.stretches.last().map_or(0.0, |s| s.at);
        if index >= self.names.len() || at < last {
            return self;
        }
        let mut start = self.concentrations(at);
        start[index] = (start[index] + delta).max(0.0);
        let end = settled(&self.coefficients, &start, self.k);
        self.stretches.push(Stretch { at, start, end });
        self
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn coefficients(&self) -> &[i32] {
        &self.coefficients
    }

    pub fn k(&self) -> f64 {
        self.k
    }

    /// The concentrations at `seconds`, in the order of the species.
    pub fn concentrations(&self, seconds: f64) -> Vec<f64> {
        let stretch = self
            .stretches
            .iter()
            .rev()
            .find(|s| s.at <= seconds)
            .unwrap_or(&self.stretches[0]);
        let left = (-(seconds - stretch.at).max(0.0) / self.settle).exp();
        stretch
            .start
            .iter()
            .zip(&stretch.end)
            .map(|(s, e)| e + (s - e) * left)
            .collect()
    }

    /// The reaction quotient of `concentrations`.
    pub fn quotient(&self, concentrations: &[f64]) -> f64 {
        log_quotient(&self.coefficients, concentrations, 0.0).exp()
    }

    /// The highest concentration any species reaches, for the scale of the bars.
    pub fn peak(&self) -> f64 {
        self.stretches
            .iter()
            .flat_map(|s| s.start.iter().chain(&s.end))
            .copied()
            .fold(0.0, f64::max)
    }
}

/// `ln Q` after the reaction has advanced by `extent` mol/L.
fn log_quotient(coefficients: &[i32], start: &[f64], extent: f64) -> f64 {
    coefficients
        .iter()
        .zip(start)
        .map(|(&nu, &c)| f64::from(nu) * (c + f64::from(nu) * extent).ln())
        .sum()
}

/// The concentrations at equilibrium from `start`. `ln Q` grows with the extent, so a
/// bisection between the extents that use up a reactant and a product finds it.
fn settled(coefficients: &[i32], start: &[f64], k: f64) -> Vec<f64> {
    let limit = |sign: f64| {
        coefficients
            .iter()
            .zip(start)
            .filter(|(nu, _)| f64::from(**nu) * sign < 0.0)
            .map(|(&nu, &c)| -c / f64::from(nu))
            .fold(f64::INFINITY * sign, |a, b| {
                if sign > 0.0 { a.min(b) } else { a.max(b) }
            })
    };
    let (mut low, mut high) = (limit(-1.0), limit(1.0));
    if !(low.is_finite() && high.is_finite() && low < high && k > 0.0) {
        return start.to_vec();
    }
    let target = k.ln();
    for _ in 0..200 {
        let mid = f64::midpoint(low, high);
        if log_quotient(coefficients, start, mid) < target {
            low = mid;
        } else {
            high = mid;
        }
    }
    let extent = f64::midpoint(low, high);
    coefficients
        .iter()
        .zip(start)
        .map(|(&nu, &c)| (c + f64::from(nu) * extent).max(0.0))
        .collect()
}

/// The concentrations of an [`EquilibriumModel`] as bars, `width` by `height` pixels around
/// `center` (units of `ui.space`), reactants in `reactant_color` and products in
/// `product_color`. The model's clock starts at `delay`. With `labels` each bar has its
/// name under it and its concentration over it, and a line above compares `Q` with `K`:
/// which way the reaction runs, or that it is at equilibrium. Up to `max` mol/L fits the
/// height (the model's peak without one). The baseline draws with `enter`, the rest fades
/// with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Equilibrium<'a> {
    pub model: Option<&'a EquilibriumModel>,
    pub delay: f32,
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub max: Option<f64>,
    pub labels: bool,
    pub reactant_color: Color,
    pub product_color: Color,
    pub axis_color: Color,
    pub text_color: Color,
    pub equal_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Equilibrium<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            model: None,
            delay: 0.0,
            center: (0.0, 0.0),
            width: 900.0,
            height: 420.0,
            max: None,
            labels: true,
            reactant_color: t.colors.accent,
            product_color: t.colors.highlight,
            axis_color: t.colors.muted,
            text_color: t.colors.text,
            equal_color: Color::hex("#69db7c"),
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Equilibrium<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// A formula split into runs of letters and runs of digits, the digits marked `true`.
fn runs(formula: &str) -> Vec<(String, bool)> {
    let mut runs: Vec<(String, bool)> = Vec::new();
    for c in formula.chars() {
        let digit = c.is_ascii_digit();
        match runs.last_mut() {
            Some((run, d)) if *d == digit => run.push(c),
            _ => runs.push((c.to_string(), digit)),
        }
    }
    runs
}

/// `formula` centred on `x` with its top at `top`, the digits as subscripts.
fn formula_text<'a>(ui: &Ui<'a, '_, '_>, formula: &str, x: f64, top: f64, fill: Color) -> Svgr<'a> {
    super::subscript_text(ui, &runs(formula), x, top, fill)
}

impl<'a> Component<'a> for Equilibrium<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(model) = self.model else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (w, h) = (f64::from(self.width), f64::from(self.height));
        let base = middle.y + h / 2.0;
        let left = middle.x - w / 2.0;
        let mut baseline = BezPath::new();
        baseline.move_to((left, base));
        baseline.line_to((left + w, base));
        let baseline = Shape {
            path: baseline,
            fill: None,
            stroke: Some(self.axis_color),
            stroke_width: self.stroke_width,
            dash: None,
            opacity: 1.0,
        };
        let baseline = draw_animated(
            ui,
            &[baseline],
            self.enter.as_ref(),
            self.exit.as_ref(),
            None,
        );

        let t = f64::from(ui.seconds() - self.delay);
        let now = model.concentrations(t);
        let max = self.max.unwrap_or_else(|| model.peak()).max(1e-9);
        let slot = w / now.len().max(1) as f64;
        let bar = slot * 0.5;
        let mut bars = Vec::new();
        let mut texts = Vec::new();
        for (i, (&c, &nu)) in now.iter().zip(model.coefficients()).enumerate() {
            let x = left + slot * (i as f64 + 0.5);
            let top = base - (c / max).min(1.0) * h;
            bars.push(Shape {
                path: kurbo::Shape::to_path(
                    &kurbo::Rect::new(x - bar / 2.0, top, x + bar / 2.0, base),
                    0.1,
                ),
                fill: Some(if nu < 0 {
                    self.reactant_color
                } else {
                    self.product_color
                }),
                stroke: None,
                stroke_width: 0.0,
                dash: None,
                opacity: 1.0,
            });
            if self.labels {
                let name = &model.names()[i];
                texts.push(formula_text(ui, name, x, base + 14.0, self.text_color));
                let over = Point::new(x, top - 10.0);
                texts.push(label(ui, short(c, 2), over, Anchor::Top, self.text_color));
            }
        }
        let mut parts = vec![draw_animated(ui, &bars, None, None, None)];
        if self.labels {
            let q = model.quotient(&now);
            let k = model.k();
            let ratio = (q / k).ln();
            let (text, color) = if ratio.abs() < 0.02 {
                (
                    format!("Q = K = {}: at equilibrium", short(k, 2)),
                    self.equal_color,
                )
            } else if ratio < 0.0 {
                (
                    format!("Q = {} < K = {}: runs forward", short(q, 2), short(k, 2)),
                    self.text_color,
                )
            } else {
                (
                    format!("Q = {} > K = {}: runs in reverse", short(q, 2), short(k, 2)),
                    self.text_color,
                )
            };
            let above = Point::new(middle.x, middle.y - h / 2.0 - 40.0);
            texts.push(label(ui, text, above, Anchor::Top, color));
        }
        parts.extend(texts);
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{baseline}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hydrogen_iodide() -> EquilibriumModel {
        EquilibriumModel::new(
            &[("H2", -1, 1.0), ("I2", -1, 1.0), ("HI", 2, 0.0)],
            49.0,
            0.5,
        )
    }

    #[test]
    fn settles_where_q_equals_k() {
        let model = hydrogen_iodide();
        // (2x)^2 / (1 - x)^2 = 49, so x = 7/9
        let end = model.concentrations(100.0);
        assert!((end[0] - 2.0 / 9.0).abs() < 1e-9);
        assert!((end[2] - 14.0 / 9.0).abs() < 1e-9);
        assert!((model.quotient(&end) - 49.0).abs() < 1e-6);
        // and starts where it was put
        assert_eq!(model.concentrations(0.0), vec![1.0, 1.0, 0.0]);
        // atoms are kept: hydrogen in H2 and HI
        let mid = model.concentrations(0.4);
        assert!((2.0 * mid[0] + mid[2] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn a_disturbance_shifts_it() {
        let model = hydrogen_iodide().disturb(3.0, 0, 1.0);
        let before = model.concentrations(2.99);
        let after = model.concentrations(30.0);
        // more hydrogen makes more hydrogen iodide and uses up iodine
        assert!(after[2] > before[2] && after[1] < before[1]);
        assert!((model.quotient(&after) - 49.0).abs() < 1e-6);
        assert!(model.peak() >= after[0]);
        // a change before the last one is ignored
        assert_eq!(model.clone().disturb(1.0, 0, 1.0), model);
    }

    #[test]
    fn formula_runs() {
        let runs = runs("N2O4");
        let expected = [("N", false), ("2", true), ("O", false), ("4", true)];
        assert_eq!(runs, expected.map(|(r, d)| (r.to_owned(), d)));
    }
}
