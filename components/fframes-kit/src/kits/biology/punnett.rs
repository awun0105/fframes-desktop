//! Punnett squares: the gametes of two parents, their cross, and the ratios it gives.

use fframes::{Color, Svgr};
use kurbo::{Circle, Point, RoundedRect, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::{Look, Shape},
    kits::draw::mix,
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line, width},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// The genes of a genotype: pairs of the same letter, upper case for the dominant allele
/// (`"Aa"`, `"AaBb"`), one to three genes. `None` for anything else.
fn genes(genotype: &str) -> Option<Vec<[char; 2]>> {
    let letters: Vec<char> = genotype.chars().collect();
    if letters.is_empty() || !letters.len().is_multiple_of(2) || letters.len() > 6 {
        return None;
    }
    letters
        .chunks(2)
        .map(|pair| {
            let same = pair[0].is_ascii_alphabetic() && pair[0].eq_ignore_ascii_case(&pair[1]);
            same.then_some([pair[0], pair[1]])
        })
        .collect()
}

/// The gametes of a genotype, one allele of each gene, in the usual order: `"AaBb"` gives
/// `AB`, `Ab`, `aB` and `ab`. A homozygous gene still gives two (identical) gametes, so every
/// square of the same number of genes has the same size. `None` when the genotype is not
/// pairs of one letter.
pub fn gametes(genotype: &str) -> Option<Vec<String>> {
    let genes = genes(genotype)?;
    let mut out = vec![String::new()];
    for [first, second] in genes {
        out = out
            .into_iter()
            .flat_map(|g| [format!("{g}{first}"), format!("{g}{second}")])
            .collect();
    }
    Some(out)
}

/// The cross of two parents: the gametes of each and the genotype in every cell, its
/// alleles written dominant first (`Aa`, never `aA`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Punnett {
    /// The first parent's gametes, across the top.
    pub columns: Vec<String>,
    /// The second parent's gametes, down the side.
    pub rows: Vec<String>,
    /// `cells[row][column]`.
    pub cells: Vec<Vec<String>>,
}

impl Punnett {
    /// Crosses two genotypes of the same genes (`"Aa"` and `"Aa"`, `"AaBb"` and `"Aabb"`).
    /// `None` when either is not pairs of one letter, or when their genes differ.
    pub fn cross(first: &str, second: &str) -> Option<Self> {
        let (a, b) = (genes(first)?, genes(second)?);
        let letters = |g: &[[char; 2]]| -> Vec<char> {
            g.iter().map(|p| p[0].to_ascii_lowercase()).collect()
        };
        if letters(&a) != letters(&b) {
            return None;
        }
        let (columns, rows) = (gametes(first)?, gametes(second)?);
        let cells = rows
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .map(|column| {
                        column
                            .chars()
                            .zip(row.chars())
                            .map(|(x, y)| {
                                // upper case sorts first
                                if x <= y {
                                    format!("{x}{y}")
                                } else {
                                    format!("{y}{x}")
                                }
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        Some(Self {
            columns,
            rows,
            cells,
        })
    }

    /// Number of genes.
    pub fn genes(&self) -> usize {
        self.columns.first().map_or(0, String::len)
    }

    /// Each genotype with how many cells have it, dominant alleles first (`AA`, `Aa`, `aa`).
    pub fn genotype_ratio(&self) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for cell in self.cells.iter().flatten() {
            match counts.iter_mut().find(|(g, _)| g == cell) {
                Some((_, n)) => *n += 1,
                None => counts.push((cell.clone(), 1)),
            }
        }
        counts.sort();
        counts
    }

    /// Each phenotype that occurs, as whether each gene shows its dominant trait, with how
    /// many cells have it; dominant first (`[true, true]`, `[true, false]`, ...).
    pub fn phenotype_ratio(&self) -> Vec<(Vec<bool>, usize)> {
        let mut counts: Vec<(Vec<bool>, usize)> = Vec::new();
        for cell in self.cells.iter().flatten() {
            let phenotype = phenotype(cell);
            match counts.iter_mut().find(|(p, _)| *p == phenotype) {
                Some((_, n)) => *n += 1,
                None => counts.push((phenotype, 1)),
            }
        }
        counts.sort_by(|a, b| b.0.cmp(&a.0));
        counts
    }
}

/// Whether each gene of a genotype shows its dominant trait.
fn phenotype(genotype: &str) -> Vec<bool> {
    let letters: Vec<char> = genotype.chars().collect();
    letters
        .chunks(2)
        .map(|pair| pair.iter().any(char::is_ascii_uppercase))
        .collect()
}

/// A Punnett square crossing `parents` (genotypes such as `"Pp"` or `"YyRr"`, one to three
/// genes), `scale` times its normal size around `center` (units of `ui.space`). The first
/// parent's gametes run across the top and the second's down the side, each in a circle.
/// Every cell is coloured by its phenotype from `phenotype_colors` (dominant first, as in
/// [`Punnett::phenotype_ratio`]) and shows its genotype. Below, the genotype and phenotype
/// ratios count up as the cells fill. Phenotypes are named from `traits`, one
/// (dominant, recessive) pair of names per gene, or written as `P_` and `pp` without them.
///
/// From `delay` the square appears, the gametes split off each parent over 0.5..1.7 s, and
/// from 1.8 s the cells fill row by row over `fill_time` seconds. Invalid parents draw
/// nothing.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct PunnettSquare {
    pub delay: f32,
    pub center: (f64, f64),
    pub scale: f64,
    pub parents: (String, String),
    pub traits: Vec<(String, String)>,
    pub fill_time: f32,
    pub ratios: bool,
    pub phenotype_colors: Vec<Color>,
    pub grid_color: Color,
    pub gamete_color: Color,
    pub highlight_color: Color,
    pub cell_style: TextStyle,
    pub parent_style: TextStyle,
    pub ratio_style: TextStyle,
    pub text_color: Color,
    pub muted_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for PunnettSquare {
    fn themed(t: &Theme) -> Self {
        let mut cell_style = t.typography.body;
        cell_style.weight = 600;
        Self {
            delay: 0.0,
            center: (0.0, 0.0),
            scale: 1.0,
            parents: ("Pp".into(), "Pp".into()),
            traits: vec![("purple".into(), "white".into())],
            fill_time: 2.4,
            ratios: true,
            phenotype_colors: vec![
                Color::hex("#9775fa"),
                Color::hex("#ffd43b"),
                Color::hex("#4dabf7"),
                Color::hex("#ced4da"),
            ],
            grid_color: t.colors.muted,
            gamete_color: t.colors.surface,
            highlight_color: t.colors.accent,
            cell_style,
            parent_style: t.typography.body,
            ratio_style: t.typography.caption,
            text_color: t.colors.text,
            muted_color: t.colors.muted,
            enter: None,
            exit: None,
        }
    }
}

impl Default for PunnettSquare {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// `style`, made smaller if needed so that the widest of `texts` fits into `room` pixels.
fn fit<'s>(
    ui: &Ui<'_, '_, '_>,
    style: TextStyle,
    texts: impl IntoIterator<Item = &'s String>,
    room: f64,
) -> TextStyle {
    let widest = texts
        .into_iter()
        .map(|s| width(ui, style, s).unwrap_or(s.len() as f32 * style.size as f32 * 0.62))
        .fold(0.0_f32, f32::max);
    let mut fitted = style;
    if f64::from(widest) > room && widest > 0.0 {
        fitted.size = ((style.size as f64 * room / f64::from(widest)) as usize).max(8);
    }
    fitted
}

/// When the gametes split off the parents and when the cells start filling, in seconds after
/// the delay.
const SPLIT: (f32, f32) = (0.5, 1.7);
const FILL: f32 = 1.8;
/// How long a cell takes to fill, and a count to change.
const CELL_IN: f32 = 0.3;
const COUNT_IN: f32 = 0.15;

impl PunnettSquare {
    /// The phenotype's colour: its index in the dominant-first order, spread over the palette
    /// so that the all-recessive phenotype takes the last colour, or round it when there are
    /// more phenotypes than colours.
    fn color(&self, phenotype: &[bool]) -> Color {
        let index = phenotype
            .iter()
            .fold(0, |i, &dominant| i * 2 + usize::from(!dominant));
        let kinds = 1 << phenotype.len();
        let colors = self.phenotype_colors.len();
        let index = if kinds <= colors && kinds > 1 {
            index * (colors - 1) / (kinds - 1)
        } else {
            index % colors.max(1)
        };
        self.phenotype_colors
            .get(index)
            .copied()
            .unwrap_or(self.muted_color)
    }

    /// The phenotype's name from `traits`, or its notation (`P_`, `pp`).
    fn phenotype_name(&self, phenotype: &[bool], letters: &[char]) -> String {
        if self.traits.len() == phenotype.len() {
            let names: Vec<&str> = self
                .traits
                .iter()
                .zip(phenotype)
                .map(|((d, r), &dominant)| if dominant { d.as_str() } else { r.as_str() })
                .collect();
            return names.join(" ");
        }
        letters
            .iter()
            .zip(phenotype)
            .map(|(&l, &dominant)| {
                let (big, small) = (l.to_ascii_uppercase(), l.to_ascii_lowercase());
                if dominant {
                    format!("{big}_")
                } else {
                    format!("{small}{small}")
                }
            })
            .collect()
    }
}

impl<'a> Component<'a> for PunnettSquare {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(cross) = Punnett::cross(&self.parents.0, &self.parents.1) else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let s = self.scale;
        let t = ui.seconds() - self.delay;
        let ease = |a: f32, b: f32| cubic_in_out(progress(t, a, b));
        let n = cross.columns.len();
        let size = s * match n {
            1 | 2 => 120.0,
            4 => 92.0,
            _ => 62.0,
        };
        let side = size * n as f64;
        let (left, top) = (middle.x - side / 2.0, middle.y - side / 2.0);
        let cells = n * n;
        let start = |k: usize| FILL + self.fill_time * k as f32 / cells as f32;
        let shown = ease(0.0, 0.5);
        let radius = (26.0 + 4.0 * (cross.genes() as f64 - 1.0)) * s;
        let gamete_style = fit(
            ui,
            self.parent_style,
            cross.columns.iter().chain(&cross.rows),
            2.0 * radius - 14.0,
        );
        let cell_style = fit(
            ui,
            self.cell_style,
            cross.cells.iter().flatten(),
            size - 18.0,
        );
        let mut out = Vec::new();
        let text = |out: &mut Vec<Svgr<'a>>,
                    at: Point,
                    style: TextStyle,
                    color,
                    anchor,
                    opacity: f32,
                    s: String| {
            let y = at.y + style.size as f64 * 0.35;
            out.push(line(
                at.x as f32,
                y as f32,
                style,
                color,
                anchor,
                0.0,
                opacity,
                s.into(),
            ));
        };

        // the parents, and their gametes splitting off them to the headers
        let first_at = Point::new(middle.x, top - 64.0 * s - 70.0 * s);
        let second_at = Point::new(left - 64.0 * s - 80.0 * s, middle.y);
        text(
            &mut out,
            first_at,
            self.parent_style,
            self.text_color,
            TextAnchor::Middle,
            shown,
            self.parents.0.clone(),
        );
        text(
            &mut out,
            second_at,
            self.parent_style,
            self.text_color,
            TextAnchor::Middle,
            shown,
            self.parents.1.clone(),
        );
        let headers = cross
            .columns
            .iter()
            .enumerate()
            .map(|(i, g)| {
                (
                    g,
                    first_at,
                    Point::new(left + size * (i as f64 + 0.5), top - 40.0 * s),
                    true,
                    i,
                )
            })
            .chain(cross.rows.iter().enumerate().map(|(i, g)| {
                (
                    g,
                    second_at,
                    Point::new(left - 40.0 * s, top + size * (i as f64 + 0.5)),
                    false,
                    i,
                )
            }));
        for (gamete, from, to, column, i) in headers {
            let (a, b) = if column {
                (SPLIT.0, SPLIT.0 + 0.8)
            } else {
                (SPLIT.1 - 0.8, SPLIT.1)
            };
            let p = f64::from(ease(a, b));
            let at = from.lerp(to, 0.25 + 0.75 * p);
            // lit while a cell of its column or row fills
            let lit = (0..n)
                .map(|j| {
                    let k = if column { j * n + i } else { i * n + j };
                    let c = start(k);
                    progress(t, c - 0.1, c + 0.1)
                        * (1.0 - progress(t, c + CELL_IN, c + CELL_IN + 0.2))
                })
                .fold(0.0_f32, f32::max);
            let ring = Shape {
                path: Circle::new(at, radius).to_path(0.1),
                fill: Some(self.gamete_color),
                stroke: Some(mix(self.grid_color, self.highlight_color, lit)),
                stroke_width: 2.0 + 1.5 * lit,
                dash: None,
                opacity: 1.0,
            };
            let opacity = progress(t, a, a + 0.2);
            out.push(ring.draw_with(&Look {
                opacity,
                ..Look::default()
            }));
            text(
                &mut out,
                at,
                gamete_style,
                self.text_color,
                TextAnchor::Middle,
                opacity,
                gamete.clone(),
            );
        }

        // the cells, filling row by row
        for (r, row) in cross.cells.iter().enumerate() {
            for (c, genotype) in row.iter().enumerate() {
                let k = r * n + c;
                let x = left + size * c as f64;
                let y = top + size * r as f64;
                let filled = ease(start(k), start(k) + CELL_IN);
                let cell = Shape {
                    path: RoundedRect::new(
                        x + 3.0,
                        y + 3.0,
                        x + size - 3.0,
                        y + size - 3.0,
                        8.0 * s,
                    )
                    .to_path(0.1),
                    fill: Some(mix(
                        self.gamete_color,
                        self.color(&phenotype(genotype)),
                        filled,
                    )),
                    stroke: Some(self.grid_color),
                    stroke_width: 1.5,
                    dash: None,
                    opacity: 1.0,
                };
                out.push(cell.draw_with(&Look {
                    opacity: shown,
                    ..Look::default()
                }));
                let at = Point::new(
                    x + size / 2.0,
                    y + size / 2.0 + 8.0 * f64::from(1.0 - filled),
                );
                text(
                    &mut out,
                    at,
                    cell_style,
                    Color::hex("#101418"),
                    TextAnchor::Middle,
                    filled,
                    genotype.clone(),
                );
            }
        }

        // the ratios, counting up as the cells land
        if self.ratios {
            let letters: Vec<char> = cross.columns[0].chars().collect();
            let genotypes: Vec<(String, Vec<f32>, Option<Color>)> = cross
                .genotype_ratio()
                .into_iter()
                .map(|(g, _)| {
                    let landed = (0..cells)
                        .filter(|&k| cross.cells[k / n][k % n] == g)
                        .map(|k| start(k) + CELL_IN)
                        .collect();
                    (g, landed, None)
                })
                .collect();
            let phenotypes: Vec<(String, Vec<f32>, Option<Color>)> = cross
                .phenotype_ratio()
                .into_iter()
                .map(|(p, _)| {
                    let landed = (0..cells)
                        .filter(|&k| phenotype(&cross.cells[k / n][k % n]) == p)
                        .map(|k| start(k) + CELL_IN)
                        .collect();
                    (
                        self.phenotype_name(&p, &letters),
                        landed,
                        Some(self.color(&p)),
                    )
                })
                .collect();
            let rows = [("genotypes", genotypes), ("phenotypes", phenotypes)];
            let style = self.ratio_style;
            let measure = |s: &str| {
                f64::from(width(ui, style, s).unwrap_or(s.len() as f32 * style.size as f32 * 0.55))
            };
            let row_shown = ease(FILL - 0.4, FILL);
            for (i, (name, terms)) in rows.iter().enumerate() {
                let y = top + side + (54.0 + 46.0 * i as f64) * s;
                // each term is a count and a name; the count is as wide as its final value
                let gap = 8.0;
                let separator = 30.0;
                let widths: Vec<(f64, f64)> = terms
                    .iter()
                    .map(|(term, landed, _)| (measure(&landed.len().to_string()), measure(term)))
                    .collect();
                let total: f64 = widths.iter().map(|(a, b)| a + gap + b).sum::<f64>()
                    + separator * (terms.len().saturating_sub(1)) as f64;
                let mut x = middle.x - total / 2.0;
                text(
                    &mut out,
                    Point::new(x - 24.0, y),
                    style,
                    self.muted_color,
                    TextAnchor::End,
                    row_shown,
                    (*name).to_owned(),
                );
                for (j, ((term, landed, tint), (count_width, term_width))) in
                    terms.iter().zip(&widths).enumerate()
                {
                    if j > 0 {
                        text(
                            &mut out,
                            Point::new(x - separator / 2.0, y),
                            style,
                            self.muted_color,
                            TextAnchor::Middle,
                            row_shown,
                            ":".into(),
                        );
                    }
                    // the count cross-fades from the last value to the new one
                    let count = landed.iter().filter(|&&l| l <= t).count();
                    let changed = landed
                        .iter()
                        .copied()
                        .filter(|&l| l <= t)
                        .fold(f32::MIN, f32::max);
                    let fresh = if count == 0 {
                        1.0
                    } else {
                        progress(t, changed, changed + COUNT_IN)
                    };
                    let right = Point::new(x + count_width, y);
                    text(
                        &mut out,
                        right,
                        style,
                        self.text_color,
                        TextAnchor::End,
                        row_shown * fresh,
                        count.to_string(),
                    );
                    if count > 0 && fresh < 1.0 {
                        text(
                            &mut out,
                            right,
                            style,
                            self.text_color,
                            TextAnchor::End,
                            row_shown * (1.0 - fresh),
                            (count - 1).to_string(),
                        );
                    }
                    // the name lights up as its first cell lands
                    let first = landed.iter().copied().fold(f32::MAX, f32::min);
                    let color = mix(
                        self.muted_color,
                        tint.unwrap_or(self.text_color),
                        progress(t, first, first + COUNT_IN),
                    );
                    text(
                        &mut out,
                        Point::new(x + count_width + gap, y),
                        style,
                        color,
                        TextAnchor::Start,
                        row_shown,
                        term.clone(),
                    );
                    x += count_width + gap + term_width + separator;
                }
            }
        }

        fade(ui, out.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gametes_take_one_allele_of_each_gene() {
        assert_eq!(gametes("Aa").unwrap(), ["A", "a"]);
        assert_eq!(gametes("AaBb").unwrap(), ["AB", "Ab", "aB", "ab"]);
        assert_eq!(gametes("AAbb").unwrap(), ["Ab", "Ab", "Ab", "Ab"]);
        for bad in ["", "A", "Ab", "A1", "AaBbCcDd", "aa b"] {
            assert_eq!(gametes(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_monohybrid_cross_gives_one_two_one_and_three_one() {
        let cross = Punnett::cross("Aa", "Aa").unwrap();
        assert_eq!(cross.cells, [["AA", "Aa"], ["Aa", "aa"]]);
        assert_eq!(
            cross.genotype_ratio(),
            [("AA".into(), 1), ("Aa".into(), 2), ("aa".into(), 1)]
        );
        assert_eq!(cross.phenotype_ratio(), [(vec![true], 3), (vec![false], 1)]);
    }

    #[test]
    fn a_dihybrid_cross_gives_nine_three_three_one() {
        let cross = Punnett::cross("AaBb", "AaBb").unwrap();
        assert_eq!(cross.genes(), 2);
        let counts: Vec<usize> = cross
            .phenotype_ratio()
            .into_iter()
            .map(|(_, n)| n)
            .collect();
        assert_eq!(counts, [9, 3, 3, 1]);
        assert_eq!(cross.genotype_ratio().len(), 9);
        assert_eq!(cross.phenotype_ratio()[1].0, [true, false]);
    }

    #[test]
    fn parents_must_share_their_genes() {
        assert_eq!(Punnett::cross("Aa", "AaBb"), None);
        assert_eq!(Punnett::cross("Aa", "Bb"), None);
        assert!(Punnett::cross("AA", "aa").is_some());
        let test = Punnett::cross("Aa", "aa").unwrap();
        assert_eq!(test.genotype_ratio(), [("Aa".into(), 2), ("aa".into(), 2)]);
    }

    #[test]
    fn phenotypes_are_named_from_the_traits_or_written_out() {
        let square = PunnettSquare::default();
        assert_eq!(square.phenotype_name(&[true], &['p']), "purple");
        assert_eq!(square.phenotype_name(&[false], &['p']), "white");
        let plain = PunnettSquare {
            traits: Vec::new(),
            ..PunnettSquare::default()
        };
        assert_eq!(plain.phenotype_name(&[true, false], &['y', 'r']), "Y_rr");
    }

    #[test]
    fn the_recessive_phenotype_takes_the_last_colour() {
        let square = PunnettSquare::default();
        let colors = &square.phenotype_colors;
        assert_eq!(square.color(&[true]), colors[0]);
        assert_eq!(square.color(&[false]), colors[3]);
        assert_eq!(square.color(&[true, false]), colors[1]);
        assert_eq!(square.color(&[false, false]), colors[3]);
        // eight phenotypes go round four colours
        assert_eq!(square.color(&[false, true, true]), colors[0]);
    }
}
