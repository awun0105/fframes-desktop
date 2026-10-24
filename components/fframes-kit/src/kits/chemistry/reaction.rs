use std::collections::BTreeMap;

/// The atoms in a neutral formula (`"Ca(OH)2"` gives Ca 1, O 2, H 2): element symbols with
/// counts and parenthesised groups with a count. `None` for anything else (charges, dots,
/// unknown characters).
pub fn atom_counts(formula: &str) -> Option<BTreeMap<String, u32>> {
    fn number(chars: &[char], i: &mut usize) -> u32 {
        let mut n = 0u32;
        let start = *i;
        while *i < chars.len() && chars[*i].is_ascii_digit() {
            n = n
                .saturating_mul(10)
                .saturating_add(chars[*i].to_digit(10).unwrap_or(0));
            *i += 1;
        }
        if *i == start { 1 } else { n }
    }
    fn group(chars: &[char], i: &mut usize, closing: bool) -> Option<BTreeMap<String, u32>> {
        let mut counts = BTreeMap::new();
        while *i < chars.len() {
            let c = chars[*i];
            if c == '(' {
                *i += 1;
                let inner = group(chars, i, true)?;
                let n = number(chars, i);
                for (e, k) in inner {
                    *counts.entry(e).or_insert(0) += k * n;
                }
            } else if c == ')' {
                if !closing {
                    return None;
                }
                *i += 1;
                return Some(counts);
            } else if c.is_ascii_uppercase() {
                let mut symbol = c.to_string();
                *i += 1;
                while *i < chars.len() && chars[*i].is_ascii_lowercase() {
                    symbol.push(chars[*i]);
                    *i += 1;
                }
                super::atomic_number(&symbol)?;
                let n = number(chars, i);
                *counts.entry(symbol).or_insert(0) += n;
            } else {
                return None;
            }
        }
        (!closing).then_some(counts)
    }
    let chars: Vec<char> = formula.chars().collect();
    let mut i = 0;
    let counts = group(&chars, &mut i, false)?;
    (!counts.is_empty()).then_some(counts)
}

/// A reaction between neutral species written as formulas (`"H2"`, `"Ca(OH)2"`), which can
/// balance itself and give its mhchem and `MathTex` sources with or without coefficients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reaction {
    pub reactants: Vec<String>,
    pub products: Vec<String>,
}

fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

impl Reaction {
    pub fn new(reactants: &[&str], products: &[&str]) -> Self {
        Self {
            reactants: reactants.iter().map(|s| (*s).to_owned()).collect(),
            products: products.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn species(&self) -> impl Iterator<Item = &String> {
        self.reactants.iter().chain(&self.products)
    }

    /// The elements of the reaction, sorted.
    pub fn elements(&self) -> Vec<String> {
        let mut all: Vec<String> = self
            .species()
            .filter_map(|s| atom_counts(s))
            .flat_map(BTreeMap::into_keys)
            .collect();
        all.sort();
        all.dedup();
        all
    }

    /// The smallest whole coefficients, reactants then products, that balance every
    /// element. `None` when a formula cannot be read or the reaction has no single balance
    /// (an element on one side only, or independent reactions mixed together).
    pub fn balance(&self) -> Option<Vec<u32>> {
        let counts: Vec<BTreeMap<String, u32>> = self
            .species()
            .map(|s| atom_counts(s))
            .collect::<Option<_>>()?;
        let elements = self.elements();
        let n = counts.len();
        if n < 2 {
            return None;
        }
        // one row per element: reactant counts positive, product counts negative
        let mut rows: Vec<Vec<i128>> = elements
            .iter()
            .map(|e| {
                (0..n)
                    .map(|j| {
                        let k = i128::from(counts[j].get(e).copied().unwrap_or(0));
                        if j < self.reactants.len() { k } else { -k }
                    })
                    .collect()
            })
            .collect();
        // integer row reduction to echelon form
        let mut pivots = Vec::new();
        let mut r = 0;
        for c in 0..n {
            let Some(p) = (r..rows.len()).find(|&i| rows[i][c] != 0) else {
                continue;
            };
            rows.swap(r, p);
            for i in 0..rows.len() {
                if i != r && rows[i][c] != 0 {
                    let (a, b) = (rows[r][c], rows[i][c]);
                    let pivot = rows[r].clone();
                    for (v, p) in rows[i].iter_mut().zip(pivot) {
                        *v = *v * a - p * b;
                    }
                    let g = rows[i].iter().fold(0, |g, &v| gcd(g, v));
                    if g > 1 {
                        rows[i].iter_mut().for_each(|v| *v /= g);
                    }
                }
            }
            pivots.push(c);
            r += 1;
            if r == rows.len() {
                break;
            }
        }
        // a single balance needs exactly one free column
        if pivots.len() + 1 != n {
            return None;
        }
        let free = (0..n).find(|c| !pivots.contains(c))?;
        // x_free = product of the pivots, x_pivot = -row[free] * x_free / row[pivot]
        let scale = pivots
            .iter()
            .enumerate()
            .fold(1i128, |s, (i, &c)| s * rows[i][c] / gcd(s, rows[i][c]));
        let mut x = vec![0i128; n];
        x[free] = scale;
        for (i, &c) in pivots.iter().enumerate() {
            x[c] = -rows[i][free] * scale / rows[i][c];
        }
        let g = x.iter().fold(0, |g, &v| gcd(g, v));
        let sign = if x[free] < 0 { -1 } else { 1 };
        let x: Vec<i128> = x.iter().map(|v| v * sign / g.max(1)).collect();
        x.iter()
            .map(|&v| (v > 0).then(|| u32::try_from(v).ok()).flatten())
            .collect()
    }

    /// The mhchem source, `"2H2 + O2 -> 2H2O"`, with `coefficients` (1 is left out) or
    /// without them.
    pub fn ce(&self, coefficients: Option<&[u32]>) -> String {
        let term = |i: usize, s: &str| match coefficients.and_then(|c| c.get(i)) {
            Some(&k) if k != 1 => format!("{k}{s}"),
            _ => s.to_owned(),
        };
        let side = |list: &[String], offset: usize| {
            list.iter()
                .enumerate()
                .map(|(i, s)| term(offset + i, s))
                .collect::<Vec<_>>()
                .join(" + ")
        };
        format!(
            "{} -> {}",
            side(&self.reactants, 0),
            side(&self.products, self.reactants.len())
        )
    }

    /// The `MathTex` source: every species and every coefficient is its own `{{ }}` part,
    /// so `TransformMatchingTex` moves the species and fades the coefficients in.
    pub fn tex(&self, coefficients: Option<&[u32]>) -> String {
        let term = |i: usize, s: &str| {
            let species = format!("{{{{\\ce{{{s}}}}}}}");
            match coefficients.and_then(|c| c.get(i)) {
                Some(&k) if k != 1 => format!("{{{{\\ce{{{k}}}}}}}\\,{species}"),
                _ => species,
            }
        };
        let side = |list: &[String], offset: usize| {
            list.iter()
                .enumerate()
                .map(|(i, s)| term(offset + i, s))
                .collect::<Vec<_>>()
                .join(" + ")
        };
        format!(
            "{} \\ce{{->}} {}",
            side(&self.reactants, 0),
            side(&self.products, self.reactants.len())
        )
    }

    /// The atoms of each element on the left and on the right with `coefficients` (all 1
    /// without them).
    pub fn tally(&self, coefficients: Option<&[u32]>) -> Vec<(String, u32, u32)> {
        let k = |i: usize| coefficients.and_then(|c| c.get(i)).copied().unwrap_or(1);
        let side = |list: &[String], offset: usize, e: &str| -> u32 {
            list.iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    Some(atom_counts(s)?.get(e).copied().unwrap_or(0) * k(offset + i))
                })
                .sum()
        };
        self.elements()
            .into_iter()
            .map(|e| {
                let left = side(&self.reactants, 0, &e);
                let right = side(&self.products, self.reactants.len(), &e);
                (e, left, right)
            })
            .collect()
    }
}

#[cfg(feature = "latex")]
mod view {
    use fframes::{Color, Svgr};

    use crate::{
        anim::{FadeIn, FadeOut, fade},
        kits::label,
        latex::{Formula, FormulaView, TransformMatchingTex},
        layout::Anchor,
        theme::Theme,
        ui::{Component, Themed, Ui},
    };

    use super::Reaction;

    /// A reaction being balanced: `unbalanced` (typeset from [`Reaction::tex`] without
    /// coefficients) turns into `balanced` (with them) at `delay` over `run_time`, the
    /// species sliding apart and the coefficients fading in. With `tally` and a `reaction`
    /// a line under it counts the atoms of each element on both sides, in `unequal_color`
    /// until they balance and `equal_color` after. The formulas are centred at `center`
    /// (units of `ui.space`), 10 pt as `size` pixels. The tally fades with `parts_enter`.
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
    pub struct ReactionEquation<'a> {
        pub reaction: Option<&'a Reaction>,
        pub unbalanced: Option<&'a Formula>,
        pub balanced: Option<&'a Formula>,
        pub center: (f64, f64),
        pub size: f32,
        pub delay: f32,
        pub run_time: f32,
        pub tally: bool,
        pub color: Color,
        pub equal_color: Color,
        pub unequal_color: Color,
        pub parts_enter: Option<FadeIn>,
        pub parts_exit: Option<FadeOut>,
    }

    impl Themed for ReactionEquation<'_> {
        fn themed(t: &Theme) -> Self {
            Self {
                reaction: None,
                unbalanced: None,
                balanced: None,
                center: (0.0, 0.0),
                size: 110.0,
                delay: 0.0,
                run_time: 1.2,
                tally: true,
                color: t.colors.text,
                equal_color: Color::hex("#69db7c"),
                unequal_color: Color::hex("#ff6b6b"),
                parts_enter: None,
                parts_exit: None,
            }
        }
    }

    impl Default for ReactionEquation<'_> {
        fn default() -> Self {
            Self::themed(&Theme::default())
        }
    }

    impl<'a> Component<'a> for ReactionEquation<'a> {
        fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
            let middle = ui.space.to_px(self.center.0, self.center.1);
            let view = |of: &'a Formula| FormulaView {
                size: self.size,
                x: middle.x as f32,
                y: middle.y as f32,
                anchor: Anchor::Center,
                color: self.color,
                ..FormulaView::new(of).themed(ui.theme)
            };
            let formula = match (self.unbalanced, self.balanced) {
                (Some(from), Some(to)) => ui.show(TransformMatchingTex {
                    delay: self.delay,
                    run_time: self.run_time,
                    ..TransformMatchingTex::new(view(from), view(to))
                }),
                (Some(only), None) | (None, Some(only)) => ui.show(view(only)),
                (None, None) => Svgr::empty(),
            };
            let mut parts = Vec::new();
            if let (true, Some(reaction)) = (self.tally, self.reaction) {
                let now = ui.seconds();
                let half = self.delay + self.run_time / 2.0;
                let coefficients = reaction.balance();
                let done = now >= half;
                let counts = reaction.tally(if done { coefficients.as_deref() } else { None });
                let n = counts.len() as f64;
                let gap = 150.0;
                for (i, (element, left, right)) in counts.into_iter().enumerate() {
                    let x = middle.x + (i as f64 - (n - 1.0) / 2.0) * gap;
                    let y = middle.y + f64::from(self.size) * 0.9;
                    let color = if left == right {
                        self.equal_color
                    } else {
                        self.unequal_color
                    };
                    let text = format!("{element}  {left} | {right}");
                    let at = kurbo::Point::new(x, y);
                    parts.push(label(ui, text, at, Anchor::Center, color));
                }
            }
            let tally = fade(
                ui,
                fframes::svgr!(<g>{parts}</g>),
                self.parts_enter.as_ref(),
                self.parts_exit.as_ref(),
            );
            fframes::svgr!(<g>{formula}{tally}</g>)
        }
    }

    impl Reaction {
        /// Typesets the reaction with or without its balance, for [`ReactionEquation`]. Call
        /// it in the video's constructor. Without a balance both are the same.
        pub fn typeset(
            &self,
            engine: &crate::latex::TexEngine,
            balanced: bool,
        ) -> Result<Formula, crate::latex::TexError> {
            let coefficients = if balanced { self.balance() } else { None };
            let mut formula = crate::latex::MathTex::new(&self.tex(coefficients.as_deref()))
                .template(crate::latex::TexTemplate::chemistry())
                .typeset(engine)?;
            formula.source = self.ce(coefficients.as_deref());
            Ok(formula)
        }
    }
}

#[cfg(feature = "latex")]
pub use view::ReactionEquation;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas_are_counted() {
        let c = atom_counts("Ca(OH)2").unwrap();
        assert_eq!((c["Ca"], c["O"], c["H"]), (1, 2, 2));
        let c = atom_counts("Al2(SO4)3").unwrap();
        assert_eq!((c["Al"], c["S"], c["O"]), (2, 3, 12));
        assert!(atom_counts("H2O+").is_none());
        assert!(atom_counts("Xx").is_none());
        assert!(atom_counts("(H2").is_none());
    }

    #[test]
    fn reactions_balance() {
        let water = Reaction::new(&["H2", "O2"], &["H2O"]);
        assert_eq!(water.balance(), Some(vec![2, 1, 2]));
        assert_eq!(water.ce(water.balance().as_deref()), "2H2 + O2 -> 2H2O");
        assert_eq!(water.ce(None), "H2 + O2 -> H2O");
        let propane = Reaction::new(&["C3H8", "O2"], &["CO2", "H2O"]);
        assert_eq!(propane.balance(), Some(vec![1, 5, 3, 4]));
        let rust = Reaction::new(&["Fe", "O2"], &["Fe2O3"]);
        assert_eq!(rust.balance(), Some(vec![4, 3, 2]));
        let glucose = Reaction::new(&["C6H12O6", "O2"], &["CO2", "H2O"]);
        assert_eq!(glucose.balance(), Some(vec![1, 6, 6, 6]));
        let neutralise = Reaction::new(&["Ca(OH)2", "H3PO4"], &["Ca3(PO4)2", "H2O"]);
        assert_eq!(neutralise.balance(), Some(vec![3, 2, 1, 6]));
        // carbon only on the left
        assert_eq!(Reaction::new(&["C", "O2"], &["O3"]).balance(), None);
    }

    #[test]
    fn the_tally_balances_with_the_coefficients() {
        let water = Reaction::new(&["H2", "O2"], &["H2O"]);
        assert_eq!(
            water.tally(None),
            [("H".to_owned(), 2, 2), ("O".to_owned(), 2, 1)]
        );
        let balance = water.balance().unwrap();
        assert!(water.tally(Some(&balance)).iter().all(|(_, l, r)| l == r));
    }

    #[test]
    fn tex_keeps_coefficients_apart() {
        let water = Reaction::new(&["H2", "O2"], &["H2O"]);
        assert_eq!(
            water.tex(Some(&[2, 1, 2])),
            "{{\\ce{2}}}\\,{{\\ce{H2}}} + {{\\ce{O2}}} \\ce{->} {{\\ce{2}}}\\,{{\\ce{H2O}}}"
        );
    }
}
