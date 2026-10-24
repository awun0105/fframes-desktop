use std::fmt::Write as _;

use fframes::{Color, Svgr};
use mendeleev::AtomicWeight;

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::draw_animated,
    kits::short,
    label::Label,
    layout::Anchor,
    shapes::RoundedRectangle,
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{Category, Script, element, name, subscript_text, symbol};

/// Pauling electronegativities of elements 1 to 118, 0 where none is known. `mendeleev` has
/// no electronegativity, so the values are the Pauling scale of Wikipedia's electronegativity
/// data page (the first value where it gives several, Fr 0.7). It gives none for He, Ne, Ar, Rn
/// and from Rf, and only a range for Pm, Eu and Yb, which are left out.
const PAULING: [f32; 118] = [
    2.20, 0.0, 0.98, 1.57, 2.04, 2.55, 3.04, 3.44, 3.98, 0.0, // H to Ne
    0.93, 1.31, 1.61, 1.90, 2.19, 2.58, 3.16, 0.0, 0.82, 1.00, // Na to Ca
    1.36, 1.54, 1.63, 1.66, 1.55, 1.83, 1.88, 1.91, 1.90, 1.65, // Sc to Zn
    1.81, 2.01, 2.18, 2.55, 2.96, 3.00, 0.82, 0.95, 1.22, 1.33, // Ga to Zr
    1.6, 2.16, 1.9, 2.2, 2.28, 2.20, 1.93, 1.69, 1.78, 1.96, // Nb to Sn
    2.05, 2.1, 2.66, 2.60, 0.79, 0.89, 1.10, 1.12, 1.13, 1.14, // Sb to Nd
    0.0, 1.17, 0.0, 1.2, 1.21, 1.22, 1.23, 1.24, 1.25, 0.0, // Pm to Yb
    1.27, 1.3, 1.5, 2.36, 1.9, 2.2, 2.20, 2.28, 2.54, 2.00, // Lu to Hg
    1.62, 2.33, 2.02, 2.0, 2.2, 0.0, 0.7, 0.9, 1.1, 1.3, // Tl to Th
    1.5, 1.38, 1.36, 1.28, 1.3, 1.28, 1.3, 1.3, 1.3, 1.3, // Pa to Fm
    1.3, 1.3, 1.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // Md to Ds
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // Rg to Og
];

/// The Pauling electronegativity of element `number`, `None` where none is known (He, Ne, Ar,
/// Rn, Pm, Eu, Yb and from Rf).
pub fn electronegativity(number: u32) -> Option<f32> {
    let index = usize::try_from(number).ok()?.checked_sub(1)?;
    PAULING.get(index).copied().filter(|v| *v > 0.0)
}

/// The atomic mass as written on a card: the standard atomic weight to three decimals, or the
/// mass number of the most stable isotope in brackets (`"[294]"`) when there is none.
fn mass_text(number: u32) -> String {
    match element(number).map(|e| e.atomic_weight()) {
        Some(AtomicWeight::Interval { conventional, .. }) => short(conventional, 3),
        Some(AtomicWeight::Uncertainty { weight, .. }) => short(weight, 3),
        Some(AtomicWeight::MassNumber { number }) => format!("[{number}]"),
        None => String::new(),
    }
}

/// The ground state electron configuration as runs, the electron counts as superscripts:
/// `[Ne] 3s² 3p²` for silicon, `1s¹` for hydrogen.
fn configuration_runs(number: u32) -> Vec<(String, Script)> {
    let Some(e) = element(number) else {
        return Vec::new();
    };
    let config = e.electronic_configuration();
    let mut runs = Vec::new();
    let mut base = config
        .noble_gas
        .map(|gas| format!("[{}]", gas.symbol()))
        .unwrap_or_default();
    for sub in config.valence_subshells {
        if !base.is_empty() || !runs.is_empty() {
            base.push(' ');
        }
        let _ = write!(base, "{}{}", sub.shell_number, sub.subshell_label);
        runs.push((std::mem::take(&mut base), Script::Base));
        runs.push((sub.number_of_electrons.to_string(), Script::Super));
    }
    if !base.is_empty() {
        runs.push((base, Script::Base));
    }
    runs
}

/// How an [`ElementCard`] arranges its facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum CardLayout {
    /// A square: the number and electronegativity in the top corners, the symbol, name,
    /// mass and electron configuration under each other.
    #[default]
    Square,
    /// The number, symbol and name in a square, and the mass, electron configuration and
    /// electronegativity listed with their names to the right of it.
    Wide,
}

fn sized(base: TextStyle, size: f32, weight: u16) -> TextStyle {
    TextStyle {
        size: size.round().max(6.0) as usize,
        weight,
        ..base
    }
}

/// One element as a card `size` pixels high centred at `center` (units of `ui.space`): its
/// atomic number, symbol, name, atomic mass, electron configuration and Pauling
/// electronegativity, arranged by `layout` and tinted with the colour of its [`Category`]
/// (or `color` without `category_colors`). manim-chemistry's `MElementObject`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ElementCard {
    pub element: u32,
    pub center: (f64, f64),
    pub size: f32,
    pub layout: CardLayout,
    pub category_colors: bool,
    pub color: Color,
    pub text_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for ElementCard {
    fn themed(t: &Theme) -> Self {
        Self {
            element: 6,
            center: (0.0, 0.0),
            size: 320.0,
            layout: CardLayout::Square,
            category_colors: true,
            color: t.colors.accent,
            text_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for ElementCard {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl ElementCard {
    fn text<'a>(
        &self,
        ui: &mut Ui<'a, '_, '_>,
        text: String,
        at: (f32, f32),
        direction: Anchor,
        style: TextStyle,
        fill: Color,
    ) -> Svgr<'a> {
        ui.show(Label {
            text: text.into(),
            x: at.0,
            y: at.1,
            direction,
            buff: 0.0,
            style,
            fill,
            ..ui.defaults()
        })
    }
}

impl<'a> Component<'a> for ElementCard {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let n = self.element;
        if element(n).is_none() {
            return Svgr::empty();
        }
        let color = if self.category_colors {
            Category::of(n).color()
        } else {
            self.color
        };
        let s = self.size;
        let width = match self.layout {
            CardLayout::Square => s,
            CardLayout::Wide => s * 2.2,
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (cx, cy) = (middle.x as f32, middle.y as f32);
        let (left, top) = (cx - width / 2.0, cy - s / 2.0);
        let corner = s * 0.06;
        let mut back = RoundedRectangle {
            cx,
            cy,
            width,
            height: s,
            corner_radius: corner,
            fill: Some(color),
            stroke: None,
            ..ui.defaults()
        }
        .shape();
        back.opacity = 0.16;
        let border = RoundedRectangle {
            cx,
            cy,
            width,
            height: s,
            corner_radius: corner,
            fill: None,
            stroke: Some(color),
            stroke_width: (s / 80.0).max(1.5),
            ..ui.defaults()
        }
        .shape();
        let mut parts = vec![draw_animated(ui, &[back, border], None, None, None)];

        let base = ui.theme.typography.body;
        let title = ui.theme.colors.title;
        let muted = ui.theme.colors.muted;
        let text = self.text_color;
        let pad = s * 0.07;
        let tile_x = left + s / 2.0;
        let negativity = electronegativity(n).map(|v| short(f64::from(v), 2));
        parts.push(self.text(
            ui,
            n.to_string(),
            (left + pad, top + pad),
            Anchor::BottomRight,
            sized(base, s * 0.11, 600),
            text,
        ));
        parts.push(self.text(
            ui,
            symbol(n).to_owned(),
            (tile_x, top + s * 0.42),
            Anchor::Center,
            sized(base, s * 0.3, 700),
            title,
        ));
        parts.push(self.text(
            ui,
            name(n).to_owned(),
            (tile_x, top + s * 0.63),
            Anchor::Center,
            sized(base, s * 0.09, 500),
            text,
        ));
        let configuration = configuration_runs(n);
        match self.layout {
            CardLayout::Square => {
                if let Some(v) = negativity {
                    parts.push(self.text(
                        ui,
                        v,
                        (left + s - pad, top + pad),
                        Anchor::BottomLeft,
                        sized(base, s * 0.08, 500),
                        muted,
                    ));
                }
                parts.push(self.text(
                    ui,
                    mass_text(n),
                    (tile_x, top + s * 0.75),
                    Anchor::Center,
                    sized(base, s * 0.075, 400),
                    text,
                ));
                parts.push(subscript_text(
                    ui,
                    &configuration,
                    sized(base, s * 0.075, 400),
                    f64::from(tile_x),
                    f64::from(top + s * 0.82),
                    text,
                ));
            }
            CardLayout::Wide => {
                let column = left + s + (width - s) / 2.0;
                let rows = [
                    ("atomic mass", Some(mass_text(n))),
                    ("electron configuration", None),
                    (
                        "electronegativity",
                        Some(negativity.unwrap_or("n/a".into())),
                    ),
                ];
                for (i, (heading, value)) in rows.into_iter().enumerate() {
                    let y = top + s * (0.2 + 0.28 * i as f32);
                    parts.push(self.text(
                        ui,
                        heading.to_owned(),
                        (column, y),
                        Anchor::Center,
                        sized(base, s * 0.06, 500),
                        muted,
                    ));
                    let value_top = y + s * 0.05;
                    let value_style = sized(base, s * 0.085, 500);
                    parts.push(match value {
                        Some(value) => self.text(
                            ui,
                            value,
                            (column, value_top + value_style.size as f32 * 0.5),
                            Anchor::Center,
                            value_style,
                            text,
                        ),
                        None => subscript_text(
                            ui,
                            &configuration,
                            value_style,
                            f64::from(column),
                            f64::from(value_top),
                            text,
                        ),
                    });
                }
            }
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use mendeleev::{ALL_ELEMENTS, AtomicWeight};

    use super::*;

    /// The runs as one string, the superscripts as plain digits.
    fn flat(runs: &[(String, Script)]) -> String {
        runs.iter().map(|(run, _)| run.as_str()).collect()
    }

    #[test]
    fn configurations_match_mendeleev() {
        for e in ALL_ELEMENTS {
            // mendeleev writes superscript digits and leaves out a count of one
            let expected: String = e
                .electronic_configuration()
                .to_string()
                .chars()
                .map(|c| match "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().position(|s| s == c) {
                    Some(d) => char::from(b'0' + d as u8),
                    None => c,
                })
                .collect();
            let runs = configuration_runs(e.atomic_number());
            let without_ones: Vec<_> = runs
                .iter()
                .filter(|(run, script)| !(*script == Script::Super && run == "1"))
                .cloned()
                .collect();
            assert_eq!(flat(&without_ones), expected, "{}", e.symbol());
        }
        assert_eq!(flat(&configuration_runs(1)), "1s1");
        assert_eq!(
            configuration_runs(14),
            vec![
                ("[Ne] 3s".to_owned(), Script::Base),
                ("2".to_owned(), Script::Super),
                (" 3p".to_owned(), Script::Base),
                ("2".to_owned(), Script::Super),
            ]
        );
    }

    #[test]
    fn masses_match_mendeleev() {
        for e in ALL_ELEMENTS {
            let n = e.atomic_number();
            let text = mass_text(n);
            match e.atomic_weight() {
                AtomicWeight::MassNumber { number } => assert_eq!(text, format!("[{number}]")),
                weight => {
                    let value: f64 = text.parse().expect("a number");
                    assert!(
                        (value - f64::from(&weight)).abs() < 0.0006,
                        "{text} for {n}"
                    );
                }
            }
        }
        assert_eq!(mass_text(6), "12.011");
        assert_eq!(mass_text(0), "");
    }

    #[test]
    fn electronegativities() {
        assert_eq!(electronegativity(9), Some(3.98));
        assert_eq!(electronegativity(1), Some(2.20));
        assert_eq!(electronegativity(103), Some(1.3));
        for noble in [2, 10, 18, 86, 61, 104] {
            assert_eq!(electronegativity(noble), None);
        }
        assert_eq!(electronegativity(0), None);
        assert_eq!(electronegativity(119), None);
        let known: Vec<f32> = (1..=118).filter_map(electronegativity).collect();
        assert!(known.iter().all(|v| (0.7..=3.98).contains(v)));
        // down group 1 and across period 2 the values fall and rise
        let e = |n| electronegativity(n).unwrap();
        assert!(e(3) > e(11) && e(11) > e(19));
        assert!((3..9).all(|n| e(n) < e(n + 1)));
    }
}
