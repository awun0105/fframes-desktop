//! DNA: bases, their complements and a twisting double helix that unzips to replicate.

use std::f64::consts::PI;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    kits::draw::{mix, stroke},
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// The base pairing with `base` (`A`-`T`, `G`-`C`; `U` pairs with `A`), in the case given.
/// Anything else pairs with itself.
pub fn complement(base: char) -> char {
    let pair = match base.to_ascii_uppercase() {
        'A' => 'T',
        'T' | 'U' => 'A',
        'G' => 'C',
        'C' => 'G',
        other => other,
    };
    if base.is_ascii_lowercase() {
        pair.to_ascii_lowercase()
    } else {
        pair
    }
}

/// The colours of the four bases and of the strands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaseColors {
    pub a: Color,
    pub t: Color,
    pub g: Color,
    pub c: Color,
    /// `U` in RNA.
    pub u: Color,
}

impl Default for BaseColors {
    fn default() -> Self {
        Self {
            a: Color::hex("#69db7c"),
            t: Color::hex("#ff6b6b"),
            g: Color::hex("#ffd43b"),
            c: Color::hex("#4dabf7"),
            u: Color::hex("#da77f2"),
        }
    }
}

impl BaseColors {
    /// The colour of `base`; anything that is not a base is grey.
    pub fn of(&self, base: char) -> Color {
        match base.to_ascii_uppercase() {
            'A' => self.a,
            'T' => self.t,
            'G' => self.g,
            'C' => self.c,
            'U' => self.u,
            _ => Color::hex("#adb5bd"),
        }
    }
}

/// The gap between the two strands, as a turn of the helix: the minor groove is the
/// narrower side.
const GROOVE: f64 = 0.75 * PI;

/// A DNA double helix seen from the side, the first base of `sequence` at the left, centred
/// on `center` (units of `ui.space`). Each turn is `pitch` pixels long and ten base pairs;
/// the strands are `radius` pixels from the axis and twist `spin` turns a second. The back of
/// each strand is dimmer than its front.
///
/// From `delay` a reading light passes the pairs, one every `step_time` seconds when `scan`
/// is on, and writes the lit pair above and below the helix. From `unzip_at` seconds (after
/// `delay`) the strands come apart from the left in `unzip_time` seconds and straighten;
/// with `replicate` a new partner strand in `new_color` grows along each of them behind the
/// fork. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct DnaHelix {
    pub sequence: String,
    pub delay: f32,
    pub center: (f64, f64),
    pub pitch: f32,
    pub radius: f32,
    pub spin: f32,
    pub scan: bool,
    pub step_time: f32,
    pub unzip_at: Option<f32>,
    pub unzip_time: f32,
    pub replicate: bool,
    pub bases: BaseColors,
    pub strand_color: Color,
    pub new_color: Color,
    pub lit_color: Color,
    pub letter_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for DnaHelix {
    fn themed(t: &Theme) -> Self {
        let mut letter_style = t.typography.body;
        letter_style.weight = 700;
        Self {
            sequence: "ATGCGTACCTGAATCGGATCCATGCAGTTA".into(),
            delay: 0.0,
            center: (0.0, 0.0),
            pitch: 340.0,
            radius: 80.0,
            spin: 0.12,
            scan: true,
            step_time: 0.18,
            unzip_at: None,
            unzip_time: 3.0,
            replicate: true,
            bases: BaseColors::default(),
            strand_color: t.colors.text,
            new_color: t.colors.highlight,
            lit_color: t.colors.title,
            letter_style,
            enter: None,
            exit: None,
        }
    }
}

impl Default for DnaHelix {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// How far the strands at `x` (from 0 at the first base to `length`) have come apart when
/// the unzipping is `u` of the way through, with a fork `fork` pixels long.
pub(crate) fn opened(x: f64, length: f64, u: f64, fork: f64) -> f64 {
    let front = u * (length + fork);
    ((front - x) / fork).clamp(0.0, 1.0)
}

impl DnaHelix {
    /// How far the unzipping is, 0 to 1, `seconds` after `delay`.
    fn unzipped(&self, seconds: f32) -> f64 {
        self.unzip_at.map_or(0.0, |at| {
            f64::from(cubic_in_out(progress(seconds, at, at + self.unzip_time)))
        })
    }
}

impl<'a> Component<'a> for DnaHelix {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let bases: Vec<char> = self
            .sequence
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let n = bases.len();
        if n == 0 {
            return Svgr::empty();
        }
        let t = ui.seconds() - self.delay;
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (pitch, radius) = (f64::from(self.pitch), f64::from(self.radius));
        let spacing = pitch / 10.0;
        let length = spacing * n as f64;
        let left = middle.x - length / 2.0;
        let phase = 2.0 * PI * f64::from(self.spin) * f64::from(ui.seconds());
        let u = self.unzipped(t);
        let fork = pitch * 0.5;
        let gap = radius * 2.6;
        // the partner strands sit this far from their parents once apart
        let partner = radius * 1.8;

        // strand `s` (0 top, 1 bottom) at `x` from the left: its height and depth (1 front)
        let at = |s: usize, x: f64| {
            let theta = 2.0 * PI * x / pitch + phase + if s == 1 { GROOVE } else { 0.0 };
            let open = opened(x, length, u, fork);
            let side = if s == 0 { -1.0 } else { 1.0 };
            let y = radius * theta.sin() * (1.0 - open) + side * gap * open;
            let depth = theta.cos() * (1.0 - open) + open;
            (middle.y + y, depth)
        };
        let grown = |x: f64| {
            if self.replicate {
                // the new strands follow half a fork behind the opening and finish with it
                opened(x + fork * 0.5, length + fork * 0.5, u, fork)
            } else {
                0.0
            }
        };

        let mut back = Vec::new();
        let mut front = Vec::new();
        let samples = (length / 5.0).ceil() as usize;
        for s in 0..2 {
            // runs of the strand on the same side, drawn as one path each
            let mut run = BezPath::new();
            let mut run_front = None;
            for i in 0..=samples {
                let x = length * i as f64 / samples as f64;
                let (y, depth) = at(s, x);
                let is_front = depth >= 0.0;
                let p = Point::new(left + x, y);
                match run_front {
                    Some(f) if f == is_front => run.line_to(p),
                    _ => {
                        if let Some(f) = run_front {
                            run.line_to(p);
                            let shape = if f {
                                stroke(run, self.strand_color, 7.0, 1.0)
                            } else {
                                stroke(run, self.strand_color, 5.0, 0.4)
                            };
                            if f { &mut front } else { &mut back }.push(shape.draw());
                        }
                        run = BezPath::new();
                        run.move_to(p);
                        run_front = Some(is_front);
                    }
                }
            }
            if let Some(f) = run_front {
                let shape = if f {
                    stroke(run, self.strand_color, 7.0, 1.0)
                } else {
                    stroke(run, self.strand_color, 5.0, 0.4)
                };
                if f { &mut front } else { &mut back }.push(shape.draw());
            }
            // the new partner strand, straight, as far as it has grown
            if self.replicate && u > 0.0 {
                let side = if s == 0 { -1.0 } else { 1.0 };
                let y = middle.y + side * (gap - partner);
                let mut path = BezPath::new();
                let mut started = false;
                for i in 0..=samples {
                    let x = length * i as f64 / samples as f64;
                    if grown(x) < 0.5 {
                        break;
                    }
                    let p = Point::new(left + x, y);
                    if started {
                        path.line_to(p);
                    } else {
                        path.move_to(p);
                        started = true;
                    }
                }
                if started {
                    front.push(stroke(path, self.new_color, 7.0, 1.0).draw());
                }
            }
        }

        // the reading light, between pairs as it moves on
        let reading = if self.scan {
            f64::from(t / self.step_time.max(0.01))
        } else {
            -10.0
        };
        let lit = |i: usize| (1.0 - (reading - i as f64 - 0.5).abs()).clamp(0.0, 1.0) as f32;

        let mut rungs = Vec::new();
        let mut letters = Vec::new();
        for (i, &base) in bases.iter().enumerate() {
            let x = (i as f64 + 0.5) * spacing;
            let (ya, da) = at(0, x);
            let (yb, db) = at(1, x);
            let open = opened(x, length, u, fork);
            let glow = lit(i);
            let depth = f64::midpoint(da, db);
            let opacity = (0.55 + 0.45 * depth.clamp(-1.0, 1.0)) as f32;
            let width = 6.0 + 3.0 * glow;
            let xp = left + x;
            let pair = complement(base);
            let half_rung = |from: f64, to: f64, color: Color, opacity: f32| {
                let mut path = BezPath::new();
                path.move_to((xp, from));
                path.line_to((xp, to));
                stroke(path, mix(color, self.lit_color, glow * 0.6), width, opacity).draw()
            };
            // each half stays with its strand: closed, the halves meet in the middle; apart,
            // they reach for the new partners
            let half = partner * 0.5;
            let mid = f64::midpoint(ya, yb);
            let ta = mid + (ya + half - mid) * open;
            let tb = mid + (yb - half - mid) * open;
            rungs.push(half_rung(ya, ta, self.bases.of(base), opacity));
            rungs.push(half_rung(yb, tb, self.bases.of(pair), opacity));
            // the new bases grow from the new strands to meet them
            let g = grown(x);
            if g > 0.0 {
                let new_a = middle.y - (gap - partner);
                let new_b = middle.y + (gap - partner);
                let g32 = g as f32;
                rungs.push(half_rung(new_a, new_a - half * g, self.bases.of(pair), g32));
                rungs.push(half_rung(new_b, new_b + half * g, self.bases.of(base), g32));
            }
            if glow > 0.0 {
                // just outside the strands, wherever they are
                let reach = radius + (gap - radius) * open;
                let top = middle.y - reach - 30.0;
                let bottom = middle.y + reach + 30.0 + self.letter_style.size as f64 * 0.7;
                for (y, letter) in [(top, base), (bottom, pair)] {
                    letters.push(line(
                        xp as f32,
                        y as f32,
                        self.letter_style,
                        self.bases.of(letter),
                        TextAnchor::Middle,
                        0.0,
                        glow,
                        letter.to_string().into(),
                    ));
                }
            }
        }
        let parts: Svgr<'a> = vec![back.into(), rungs.into(), front.into(), letters.into()].into();
        fade(ui, parts, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bases_pair_up() {
        let strand = "ATGCatgcU";
        let pairs: String = strand.chars().map(complement).collect();
        assert_eq!(pairs, "TACGtacgA");
        assert_eq!(complement('N'), 'N');
        assert_eq!(BaseColors::default().of('g'), BaseColors::default().g);
    }

    #[test]
    fn strands_open_from_the_left() {
        // closed before, open after, a fork in between
        assert_eq!(opened(0.0, 1000.0, 0.0, 100.0), 0.0);
        assert_eq!(opened(1000.0, 1000.0, 1.0, 100.0), 1.0);
        let u = 0.5;
        let front = u * 1100.0;
        assert_eq!(opened(front - 200.0, 1000.0, u, 100.0), 1.0);
        assert_eq!(opened(front - 50.0, 1000.0, u, 100.0), 0.5);
        assert_eq!(opened(front + 10.0, 1000.0, u, 100.0), 0.0);
    }
}
