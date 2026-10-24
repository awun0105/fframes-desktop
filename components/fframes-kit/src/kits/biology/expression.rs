//! Gene expression: DNA transcribed into mRNA, and mRNA translated into a chain of amino
//! acids by a ribosome, with the genetic code.

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Ellipse, Point, RoundedRect, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    kits::draw::{mix, stroke},
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::dna::{BaseColors, complement};

/// The amino acid of each codon, one letter each, `*` for stop, in the order `UUU`, `UUC`,
/// `UUA`, `UUG`, `UCU` .. `GGG` (`U`, `C`, `A`, `G` for each of the three bases).
const CODE: &[u8; 64] = b"FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG";

/// The amino acid `codon` codes for, as its three-letter name (`Met`), or `None` for a stop
/// codon or anything that is not three bases. `T` reads as `U`.
pub fn amino_acid(codon: &str) -> Option<&'static str> {
    let index = |c: char| match c.to_ascii_uppercase() {
        'U' | 'T' => Some(0),
        'C' => Some(1),
        'A' => Some(2),
        'G' => Some(3),
        _ => None,
    };
    let bases: Vec<char> = codon.chars().collect();
    let [a, b, c] = bases.as_slice() else {
        return None;
    };
    let i = index(*a)? * 16 + index(*b)? * 4 + index(*c)?;
    Some(match CODE[i] {
        b'F' => "Phe",
        b'L' => "Leu",
        b'S' => "Ser",
        b'Y' => "Tyr",
        b'C' => "Cys",
        b'W' => "Trp",
        b'P' => "Pro",
        b'H' => "His",
        b'Q' => "Gln",
        b'R' => "Arg",
        b'I' => "Ile",
        b'M' => "Met",
        b'T' => "Thr",
        b'N' => "Asn",
        b'K' => "Lys",
        b'V' => "Val",
        b'A' => "Ala",
        b'D' => "Asp",
        b'E' => "Glu",
        b'G' => "Gly",
        _ => return None,
    })
}

/// The mRNA made from a DNA coding strand: the same bases with `U` for `T`.
pub fn transcribe(coding: &str) -> String {
    coding
        .chars()
        .map(|c| match c {
            'T' => 'U',
            't' => 'u',
            other => other,
        })
        .collect()
}

/// The reading frame of `mrna`: the codons from the first `AUG` (or from the start when
/// there is none) up to and including the first stop codon, with the index of the first
/// base. A last codon shorter than three bases is left out.
pub fn reading_frame(mrna: &str) -> (usize, Vec<String>) {
    let bases: Vec<char> = mrna.chars().filter(|c| !c.is_whitespace()).collect();
    let text: String = bases.iter().collect::<String>().to_ascii_uppercase();
    let start = text.find("AUG").unwrap_or(0);
    let mut codons = Vec::new();
    for chunk in bases[start..].chunks(3) {
        if chunk.len() < 3 {
            break;
        }
        let codon: String = chunk.iter().collect();
        let stop = amino_acid(&codon).is_none();
        codons.push(codon);
        if stop {
            break;
        }
    }
    (start, codons)
}

/// A base drawn as a tab with its letter, `width` wide, its body from `top` down `height`.
#[allow(clippy::too_many_arguments)]
fn tab<'a>(
    x: f64,
    top: f64,
    width: f64,
    height: f64,
    base: char,
    colors: &BaseColors,
    style: TextStyle,
    text: Color,
    opacity: f32,
) -> Svgr<'a> {
    let body = Shape {
        path: RoundedRect::new(x - width / 2.0, top, x + width / 2.0, top + height, 6.0)
            .to_path(0.1),
        fill: Some(colors.of(base)),
        stroke: None,
        stroke_width: 0.0,
        dash: None,
        opacity,
    };
    let letter = line(
        x as f32,
        (top + height / 2.0 + style.size as f64 * 0.36) as f32,
        style,
        text,
        TextAnchor::Middle,
        0.0,
        opacity,
        base.to_string().into(),
    );
    fframes::svgr!(<g>{body.draw()}{letter}</g>)
}

fn backbone<'a>(x0: f64, x1: f64, y: impl Fn(f64) -> f64, color: Color, opacity: f32) -> Svgr<'a> {
    let mut path = BezPath::new();
    let steps = ((x1 - x0) / 6.0).ceil().max(1.0) as usize;
    for i in 0..=steps {
        let x = x0 + (x1 - x0) * i as f64 / steps as f64;
        if i == 0 {
            path.move_to((x, y(x)));
        } else {
            path.line_to((x, y(x)));
        }
    }
    stroke(path, color, 6.0, opacity).draw()
}

fn blob(center: Point, radii: (f64, f64), fill: Color, opacity: f32) -> Shape {
    Shape {
        path: Ellipse::new(center, radii, 0.0).to_path(0.1),
        fill: Some(fill),
        stroke: Some(fill),
        stroke_width: 3.0,
        dash: None,
        opacity,
    }
}

/// Transcription: the DNA of `coding` (its coding strand on top, the template strand below
/// it, base pairs meeting in the middle) with RNA polymerase moving along it from `delay`,
/// one base every `base_time` seconds, `spacing` pixels apart, centred on `center` (units of
/// `ui.space`). Where it is the strands open into a bubble and the template base it reads
/// lights up; the matching mRNA base (`U` for `T`) drops into the mRNA row below. At the end
/// the polymerase lets go and the bubble closes. With `labels` the rows are named.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Transcription {
    pub coding: String,
    pub delay: f32,
    pub base_time: f32,
    pub center: (f64, f64),
    pub spacing: f32,
    pub labels: bool,
    pub bases: BaseColors,
    pub strand_color: Color,
    pub rna_color: Color,
    pub polymerase_color: Color,
    pub lit_color: Color,
    pub base_style: TextStyle,
    pub label_style: TextStyle,
    pub label_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for Transcription {
    fn themed(t: &Theme) -> Self {
        let mut base_style = t.typography.caption;
        base_style.size = 22;
        base_style.weight = 700;
        Self {
            coding: "ATGGCTTCGAAATGGTAA".into(),
            delay: 0.0,
            base_time: 0.35,
            center: (0.0, 0.0),
            spacing: 52.0,
            labels: true,
            bases: BaseColors::default(),
            strand_color: t.colors.text,
            rna_color: t.colors.highlight,
            polymerase_color: t.colors.accent,
            lit_color: t.colors.title,
            base_style,
            label_style: t.typography.caption,
            label_color: t.colors.muted,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Transcription {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Transcription {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let coding: Vec<char> = self.coding.chars().filter(|c| !c.is_whitespace()).collect();
        let n = coding.len();
        if n == 0 {
            return Svgr::empty();
        }
        let t = ui.seconds() - self.delay;
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let spacing = f64::from(self.spacing);
        let left = middle.x - spacing * n as f64 / 2.0;
        let x_of = |i: usize| left + (i as f64 + 0.5) * spacing;
        let tab_h = 34.0;
        let tab_w = spacing - 8.0;
        let top = middle.y - 110.0;
        let rna = middle.y + 110.0;
        let mrna: Vec<char> = transcribe(&self.coding)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();

        // the polymerase reads base `reading` (between bases as it moves), then lets go
        let base_time = self.base_time.max(0.01);
        let reading = f64::from(t / base_time) - 0.5;
        let end = n as f32 * base_time;
        let present = 1.0 - progress(t, end, end + 0.6);
        let shown = progress(t, -0.4, 0.0) * present;
        let xp = left + (reading + 0.5).clamp(0.0, n as f64) * spacing;
        let open = |x: f64| {
            let d = (x - xp) / (spacing * 1.8);
            f64::from(shown) * (1.0 - d * d).max(0.0).powi(2) * 34.0
        };
        let coding_y = |x: f64| top - open(x);
        let template_y = |x: f64| top + 2.0 * tab_h + open(x);

        let mut dna = vec![
            backbone(
                left,
                left + spacing * n as f64,
                coding_y,
                self.strand_color,
                1.0,
            ),
            backbone(
                left,
                left + spacing * n as f64,
                template_y,
                self.strand_color,
                1.0,
            ),
        ];
        let mut made = Vec::new();
        for (i, &base) in coding.iter().enumerate() {
            let x = x_of(i);
            let glow = (1.0 - (reading - i as f64).abs()).clamp(0.0, 1.0) as f32 * shown;
            dna.push(tab(
                x,
                coding_y(x),
                tab_w,
                tab_h,
                base,
                &self.bases,
                self.base_style,
                Color::hex("#101318"),
                1.0,
            ));
            let pair = complement(base);
            let lit = mix(self.bases.of(pair), self.lit_color, glow * 0.7);
            let colors = BaseColors {
                a: lit,
                t: lit,
                g: lit,
                c: lit,
                u: lit,
            };
            dna.push(tab(
                x,
                template_y(x) - tab_h,
                tab_w,
                tab_h,
                pair,
                if glow > 0.0 { &colors } else { &self.bases },
                self.base_style,
                Color::hex("#101318"),
                1.0,
            ));
            // the mRNA base, made as the polymerase reads its template base
            let age = t - i as f32 * base_time - base_time * 0.5;
            if age > 0.0 {
                let p = f64::from(cubic_in_out(progress(age, 0.0, base_time * 1.4)));
                let from = template_y(x) + 8.0;
                let y = from + (rna - tab_h - from) * p;
                made.push(tab(
                    x,
                    y,
                    tab_w,
                    tab_h,
                    mrna.get(i).copied().unwrap_or(base),
                    &self.bases,
                    self.base_style,
                    Color::hex("#101318"),
                    progress(age, 0.0, base_time * 0.6),
                ));
            }
        }
        // the mRNA backbone grows behind the bases already in their row
        let grown = ((f64::from(t / base_time) - 1.9).clamp(0.0, n as f64)) * spacing;
        if grown > 0.0 {
            made.push(backbone(left, left + grown, |_| rna, self.rna_color, 1.0));
        }
        let mut front = Vec::new();
        if shown > 0.0 {
            let center = Point::new(xp, top + tab_h + 30.0);
            front.push(
                blob(
                    center,
                    (spacing * 2.2, 105.0),
                    self.polymerase_color,
                    0.55 * shown,
                )
                .draw(),
            );
        }
        let mut labels = Vec::new();
        if self.labels {
            let lx = (left - 28.0) as f32;
            let size = self.label_style.size as f64 * 0.35;
            for (y, name) in [
                (top, "coding strand"),
                (top + 2.0 * tab_h, "template strand"),
                (rna - tab_h / 2.0, "mRNA"),
            ] {
                labels.push(line(
                    lx,
                    (y + size) as f32,
                    self.label_style,
                    self.label_color,
                    TextAnchor::End,
                    0.0,
                    1.0,
                    name.into(),
                ));
            }
            labels.push(line(
                xp as f32,
                (top - 80.0) as f32,
                self.label_style,
                self.polymerase_color,
                TextAnchor::Middle,
                0.0,
                shown,
                "RNA polymerase".into(),
            ));
        }
        // the polymerase sits behind the strands it opens
        let parts: Svgr<'a> = vec![front.into(), dna.into(), made.into(), labels.into()].into();
        fade(ui, parts, self.enter.as_ref(), self.exit.as_ref())
    }
}

/// Translation: the mRNA `mrna` along the bottom, `spacing` pixels a base, with its codons
/// from the start codon underlined, centred on `center` (units of `ui.space`). From `delay`
/// a ribosome reads one codon every `codon_time` seconds: the tRNA with the matching
/// anticodon drops in carrying its amino acid, the growing chain moves over onto it and the
/// tRNA before it leaves. At the stop codon the chain is released and the ribosome falls
/// apart. The chain so far is written under the mRNA (`Met-Ala-Ser`).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Translation {
    pub mrna: String,
    pub delay: f32,
    pub codon_time: f32,
    pub center: (f64, f64),
    pub spacing: f32,
    pub bases: BaseColors,
    pub strand_color: Color,
    pub ribosome_color: Color,
    pub trna_color: Color,
    pub amino_color: Color,
    pub base_style: TextStyle,
    pub amino_style: TextStyle,
    pub chain_style: TextStyle,
    pub text_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for Translation {
    fn themed(t: &Theme) -> Self {
        let mut base_style = t.typography.caption;
        base_style.size = 22;
        base_style.weight = 700;
        let mut amino_style = t.typography.caption;
        amino_style.size = 18;
        amino_style.weight = 700;
        Self {
            mrna: "GCCAUGGCUUCGAAAUGGUAAGC".into(),
            delay: 0.0,
            codon_time: 1.2,
            center: (0.0, 0.0),
            spacing: 44.0,
            bases: BaseColors::default(),
            strand_color: t.colors.highlight,
            ribosome_color: t.colors.accent,
            trna_color: t.colors.muted,
            amino_color: Color::hex("#9775fa"),
            base_style,
            amino_style,
            chain_style: t.typography.caption,
            text_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Translation {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Where bead `n` of a chain sits from its newest bead (`n` = 0): up and to the left, in a
/// zigzag, `d` pixels apart.
fn chain_offset(n: f64, d: f64) -> (f64, f64) {
    let zig = (n * std::f64::consts::PI).sin().abs();
    (-n * d * 0.55, -n * d * 0.8 - zig * 6.0)
}

impl<'a> Component<'a> for Translation {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let bases: Vec<char> = self.mrna.chars().filter(|c| !c.is_whitespace()).collect();
        let n = bases.len();
        if n == 0 {
            return Svgr::empty();
        }
        let (start, codons) = reading_frame(&self.mrna);
        let t = ui.seconds() - self.delay;
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let spacing = f64::from(self.spacing);
        let left = middle.x - spacing * n as f64 / 2.0;
        let base_y = middle.y + 120.0;
        let tab_h = 30.0;
        let codon_x = |k: usize| left + (start as f64 + 3.0 * k as f64 + 1.5) * spacing;
        let amino: Vec<Option<&str>> = codons.iter().map(|c| amino_acid(c)).collect();
        let stops = amino.last().is_some_and(Option::is_none);

        let mut back = Vec::new();
        back.push(backbone(
            left,
            left + spacing * n as f64,
            |_| base_y + tab_h,
            self.strand_color,
            1.0,
        ));
        for (i, &b) in bases.iter().enumerate() {
            back.push(tab(
                left + (i as f64 + 0.5) * spacing,
                base_y,
                spacing - 6.0,
                tab_h,
                b,
                &self.bases,
                self.base_style,
                Color::hex("#101318"),
                1.0,
            ));
        }
        for k in 0..codons.len() {
            let x = codon_x(k);
            let mut mark = BezPath::new();
            mark.move_to((x - spacing * 1.4, base_y + tab_h + 16.0));
            mark.line_to((x + spacing * 1.4, base_y + tab_h + 16.0));
            back.push(stroke(mark, self.text_color, 3.0, 0.5).draw());
        }

        let codon_time = self.codon_time.max(0.01);
        let steps = codons.len();
        if steps == 0 || t < 0.0 {
            return fade(ui, back.into(), self.enter.as_ref(), self.exit.as_ref());
        }
        let k = ((t / codon_time) as usize).min(steps - 1);
        let tau = ((t - k as f32 * codon_time) / codon_time).min(1.5);
        let releasing = stops && k + 1 == steps;

        // the ribosome slides on to codon k and, at the stop codon, falls apart
        let slide = f64::from(cubic_in_out(progress(tau, 0.0, 0.25)));
        let rx = if k == 0 {
            codon_x(0)
        } else {
            codon_x(k - 1) + (codon_x(k) - codon_x(k - 1)) * slide
        };
        let apart = if releasing {
            f64::from(cubic_in_out(progress(tau, 0.4, 1.0)))
        } else {
            0.0
        };
        let ribo_opacity = 0.45 * (1.0 - apart as f32) * progress(t, 0.0, 0.3);
        let mut ribosome = vec![
            blob(
                Point::new(rx - spacing * 1.5, base_y - 70.0 - apart * 60.0),
                (spacing * 3.4, 85.0),
                self.ribosome_color,
                ribo_opacity,
            )
            .draw(),
            blob(
                Point::new(rx - spacing * 1.5, base_y + tab_h + 30.0 + apart * 40.0),
                (spacing * 3.2, 40.0),
                self.ribosome_color,
                ribo_opacity,
            )
            .draw(),
        ];

        // tRNAs: the one for codon k drops in, the one before it leaves
        let bead_r = 24.0;
        let trna_top = base_y - 150.0;
        let anchor_of = |j: usize| Point::new(codon_x(j), trna_top - bead_r);
        let mut trnas = Vec::new();
        let draw_trna = |j: usize, dy: f64, dx: f64, opacity: f32, out: &mut Vec<Svgr<'a>>| {
            let x = codon_x(j) + dx;
            let codon: Vec<char> = codons[j].chars().collect();
            let mut stem = BezPath::new();
            stem.move_to((x, trna_top + dy));
            stem.line_to((x, base_y - tab_h - 8.0 + dy));
            out.push(stroke(stem, self.trna_color, 10.0, opacity).draw());
            let body = Shape {
                path: RoundedRect::new(
                    x - spacing * 1.5 + 3.0,
                    base_y - tab_h - 14.0 + dy,
                    x + spacing * 1.5 - 3.0,
                    base_y - tab_h + 2.0 + dy,
                    6.0,
                )
                .to_path(0.1),
                fill: Some(self.trna_color),
                stroke: None,
                stroke_width: 0.0,
                dash: None,
                opacity,
            };
            out.push(body.draw());
            for (m, &c) in codon.iter().enumerate() {
                // anticodons are RNA too: U where DNA would pair with T
                let anti = match complement(c) {
                    'T' => 'U',
                    't' => 'u',
                    other => other,
                };
                out.push(tab(
                    x + (m as f64 - 1.0) * spacing,
                    base_y - tab_h + dy,
                    spacing - 6.0,
                    tab_h,
                    anti,
                    &self.bases,
                    self.base_style,
                    Color::hex("#101318"),
                    opacity,
                ));
            }
        };
        let arrive = f64::from(cubic_in_out(progress(tau, 0.0, 0.35)));
        if amino.get(k).copied().flatten().is_some() {
            draw_trna(k, -(1.0 - arrive) * 160.0, 0.0, arrive as f32, &mut trnas);
        }
        let leave = f64::from(cubic_in_out(progress(tau, 0.6, 0.95)));
        if k > 0 && leave < 1.0 {
            draw_trna(
                k - 1,
                -leave * 160.0,
                -leave * spacing * 2.0,
                1.0 - leave as f32,
                &mut trnas,
            );
        }

        // the chain: beads 0..k, moving over from tRNA k - 1 to tRNA k
        let transfer = f64::from(cubic_in_out(progress(tau, 0.4, 0.65)));
        let d = bead_r * 2.6;
        let release = if releasing {
            f64::from(cubic_in_out(progress(tau, 0.3, 1.0)))
        } else {
            0.0
        };
        let carried = if amino.get(k).copied().flatten().is_some() {
            k + 1
        } else {
            k
        };
        let mut placed: Vec<(Point, f32)> = Vec::new();
        let mut beads = Vec::new();
        let mut names = Vec::new();
        for j in 0..carried {
            let Some(name) = amino[j] else { continue };
            let pos = if j == k {
                let a = anchor_of(k);
                Point::new(a.x, a.y - (1.0 - arrive) * 160.0)
            } else {
                // before the transfer the chain hangs from tRNA k - 1, after it from tRNA k
                let before = anchor_of(k.saturating_sub(1));
                let (bx, by) = chain_offset((k - 1 - j) as f64, d);
                let (ax, ay) = chain_offset((k - j) as f64, d);
                let after = anchor_of(k);
                let into = if amino.get(k).copied().flatten().is_some() {
                    transfer
                } else {
                    0.0
                };
                Point::new(
                    before.x + bx + (after.x + ax - before.x - bx) * into,
                    before.y + by + (after.y + ay - before.y - by) * into,
                )
            };
            let pos = Point::new(pos.x - release * 40.0, pos.y - release * 120.0);
            let opacity = if j == k { arrive as f32 } else { 1.0 };
            placed.push((pos, opacity));
            beads.push(
                Shape {
                    path: Circle::new(pos, bead_r).to_path(0.1),
                    fill: Some(self.amino_color),
                    stroke: Some(self.text_color),
                    stroke_width: 2.0,
                    dash: None,
                    opacity,
                }
                .draw(),
            );
            beads.push(line(
                pos.x as f32,
                (pos.y + self.amino_style.size as f64 * 0.36) as f32,
                self.amino_style,
                self.text_color,
                TextAnchor::Middle,
                0.0,
                opacity,
                name.into(),
            ));
            names.push(name);
        }
        // the peptide bonds between neighbouring beads, under them; the newest bond forms
        // as the chain moves over
        let mut bonds = Vec::new();
        for (j, w) in placed.windows(2).enumerate() {
            let newest = j + 2 == placed.len() && carried == k + 1;
            let opacity = if newest { transfer as f32 } else { 1.0 } * w[1].1;
            let mut bond = BezPath::new();
            bond.move_to(w[0].0);
            bond.line_to(w[1].0);
            bonds.push(stroke(bond, self.text_color, 4.0, opacity).draw());
        }
        // the names of the chain; the one arriving now cross-fades in with its tRNA
        let newest = carried == k + 1;
        let mut text = Vec::new();
        let mut name_line = |names: &[&str], shown: f32| {
            if !names.is_empty() && shown > 0.0 {
                text.push(line(
                    middle.x as f32,
                    (base_y + tab_h + 140.0) as f32,
                    self.chain_style,
                    self.text_color,
                    TextAnchor::Middle,
                    0.0,
                    shown,
                    names.join("-").into(),
                ));
            }
        };
        if newest {
            name_line(&names[..names.len() - 1], 1.0 - arrive as f32);
            name_line(&names, arrive as f32);
        } else {
            name_line(&names, 1.0);
        }
        ribosome.extend(trnas);
        let parts: Svgr<'a> = vec![
            back.into(),
            ribosome.into(),
            bonds.into(),
            beads.into(),
            text.into(),
        ]
        .into();
        fade(ui, parts, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_genetic_code_reads_codons() {
        assert_eq!(amino_acid("AUG"), Some("Met"));
        assert_eq!(amino_acid("UUU"), Some("Phe"));
        assert_eq!(amino_acid("ggc"), Some("Gly"));
        assert_eq!(amino_acid("TGG"), Some("Trp"));
        for stop in ["UAA", "UAG", "UGA"] {
            assert_eq!(amino_acid(stop), None);
        }
        assert_eq!(amino_acid("AU"), None);
        assert_eq!(amino_acid("AUX"), None);
    }

    #[test]
    fn reading_frames_start_at_aug_and_end_at_a_stop() {
        assert_eq!(transcribe("ATGc"), "AUGc");
        let (start, codons) = reading_frame("GCCAUGGCUUCGAAAUGGUAAGC");
        assert_eq!(start, 3);
        assert_eq!(codons, ["AUG", "GCU", "UCG", "AAA", "UGG", "UAA"]);
        let protein: Vec<_> = codons.iter().filter_map(|c| amino_acid(c)).collect();
        assert_eq!(protein, ["Met", "Ala", "Ser", "Lys", "Trp"]);
        // no start codon: from the first base, a short tail is left out
        assert_eq!(reading_frame("GGCUUUA").1, ["GGC", "UUU"]);
    }
}
