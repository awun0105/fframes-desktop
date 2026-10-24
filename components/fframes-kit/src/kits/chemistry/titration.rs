use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::AxesMap,
    geometry::Shape,
    kits::{label, short},
    layout::Anchor,
    motion::progress,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// The ion product of water at 25 °C.
const KW: f64 = 1e-14;

/// An acid titrated with a strong base (sodium hydroxide): `acid_volume` mL of acid at
/// `acid` mol/L, base at `base` mol/L. A `pka` makes the acid weak, without one it is strong.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Titration {
    pub acid: f64,
    pub acid_volume: f64,
    pub base: f64,
    pub pka: Option<f64>,
}

impl Default for Titration {
    /// 25 mL of 0.1 M acetic acid with 0.1 M sodium hydroxide.
    fn default() -> Self {
        Self::weak(0.1, 25.0, 0.1, 4.76)
    }
}

impl Titration {
    pub fn strong(acid: f64, acid_volume: f64, base: f64) -> Self {
        Self {
            acid,
            acid_volume,
            base,
            pka: None,
        }
    }

    pub fn weak(acid: f64, acid_volume: f64, base: f64, pka: f64) -> Self {
        Self {
            pka: Some(pka),
            ..Self::strong(acid, acid_volume, base)
        }
    }

    /// The base in mL that neutralises the acid.
    pub fn equivalence(&self) -> f64 {
        self.acid * self.acid_volume / self.base
    }

    /// The pH after `added` mL of base. It solves the charge balance
    /// `[H+] + [Na+] = [OH-] + [A-]` exactly, so it holds at the start, in the buffer region,
    /// at the equivalence point and past it.
    pub fn ph(&self, added: f64) -> f64 {
        let added = added.max(0.0);
        let total = self.acid_volume + added;
        let acid = self.acid * self.acid_volume / total;
        let sodium = self.base * added / total;
        let ka = self.pka.map(|p| 10f64.powf(-p));
        // grows with [H+]: the charge balance has one root
        let balance = |h: f64| {
            let anion = ka.map_or(acid, |ka| acid * ka / (ka + h));
            h + sodium - KW / h - anion
        };
        // bisection on log10 [H+], from pH 16 to pH -2
        let (mut low, mut high) = (-16.0_f64, 2.0_f64);
        for _ in 0..80 {
            let mid = f64::midpoint(low, high);
            if balance(10f64.powf(mid)) > 0.0 {
                high = mid;
            } else {
                low = mid;
            }
        }
        -f64::midpoint(low, high)
    }

    /// `(mL, pH)` from 0 to `until` mL in `steps` equal steps, with the equivalence point
    /// added where it falls inside.
    pub fn curve(&self, until: f64, steps: usize) -> Vec<(f64, f64)> {
        let steps = steps.max(1);
        let mut volumes: Vec<f64> = (0..=steps)
            .map(|i| until * i as f64 / steps as f64)
            .collect();
        let e = self.equivalence();
        if e > 0.0 && e < until {
            volumes.push(e);
            volumes.sort_by(f64::total_cmp);
        }
        volumes.into_iter().map(|v| (v, self.ph(v))).collect()
    }
}

/// The pH of a [`Titration`] as the base is added on `axes` (mL across, pH up): from `delay`
/// over `run_time` seconds the base pours in at a constant rate up to `until` mL and the
/// curve draws along with a dot at the current pH. When it gets there the equivalence point
/// is marked with guides to both axes; for a weak acid the half-equivalence point too, where
/// the pH equals the pKa. Show the axes yourself. Everything fades with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct TitrationCurve {
    pub titration: Titration,
    pub axes: Option<AxesMap>,
    pub until: f64,
    pub delay: f32,
    pub run_time: f32,
    pub labels: bool,
    pub color: Color,
    pub marker_color: Color,
    pub guide_color: Color,
    pub text_color: Color,
    pub stroke_width: f32,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for TitrationCurve {
    fn themed(t: &Theme) -> Self {
        Self {
            titration: Titration::default(),
            axes: None,
            until: 50.0,
            delay: 0.0,
            run_time: 4.0,
            labels: true,
            color: t.colors.accent,
            marker_color: t.colors.highlight,
            guide_color: t.colors.muted,
            text_color: t.colors.text,
            stroke_width: t.stroke.regular,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for TitrationCurve {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl TitrationCurve {
    /// The base added by `seconds`, in mL.
    pub fn added_at(&self, seconds: f32) -> f64 {
        let p = progress(seconds, self.delay, self.delay + self.run_time);
        self.until * f64::from(p)
    }

    /// The pH at `seconds`, for a [`PhScale`] pointer or a readout.
    pub fn ph_at(&self, seconds: f32) -> f64 {
        self.titration.ph(self.added_at(seconds))
    }

    /// When the pour reaches `volume` mL.
    fn reaches(&self, volume: f64) -> f32 {
        let share = if self.until > 0.0 {
            volume / self.until
        } else {
            0.0
        };
        self.delay + self.run_time * share as f32
    }
}

impl<'a> Component<'a> for TitrationCurve {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(axes) = self.axes else {
            return Svgr::empty();
        };
        let now = ui.seconds();
        let added = self.added_at(now);
        let stroke = |path: BezPath, color: Color, dash: Option<(f32, f32)>| Shape {
            path,
            fill: None,
            stroke: Some(color),
            stroke_width: self.stroke_width,
            dash,
            opacity: 1.0,
        };
        let dot = |at: Point, color: Color| Shape {
            path: kurbo::Shape::to_path(&kurbo::Circle::new(at, 9.0), 0.1),
            fill: Some(color),
            stroke: None,
            stroke_width: 0.0,
            dash: None,
            opacity: 1.0,
        };

        let mut path = BezPath::new();
        let points = self
            .titration
            .curve(self.until, 600)
            .into_iter()
            .take_while(|&(v, _)| v < added)
            .chain(std::iter::once((added, self.titration.ph(added))));
        for (i, (v, ph)) in points.enumerate() {
            let p = axes.c2p(v, ph);
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        let current = axes.c2p(added, self.titration.ph(added));
        let shapes = [stroke(path, self.color, None), dot(current, self.color)];

        // the marked points: equivalence, and half of it for a weak acid
        let e = self.titration.equivalence();
        let mut marks = vec![(e, "equivalence point".to_owned())];
        if let Some(pka) = self.titration.pka {
            marks.push((e / 2.0, format!("pH = pKa = {}", short(pka, 2))));
        }
        let mut texts = Vec::new();
        for (volume, name) in marks {
            if volume <= 0.0 || volume > self.until || added < volume {
                continue;
            }
            let ph = self.titration.ph(volume);
            let at = axes.c2p(volume, ph);
            let shown = progress(now, self.reaches(volume), self.reaches(volume) + 0.3);
            let mut guides = BezPath::new();
            guides.move_to(axes.c2p(volume, axes.y.min));
            guides.line_to(at);
            guides.line_to(axes.c2p(axes.x.min, ph));
            let marked = draw_animated(
                ui,
                &[
                    stroke(guides, self.guide_color, Some((6.0, 8.0))),
                    dot(at, self.marker_color),
                ],
                None,
                None,
                None,
            );
            let mut parts = vec![marked];
            if self.labels {
                let text = if name.starts_with("equivalence") {
                    format!("{name}, pH {}", short(ph, 2))
                } else {
                    name
                };
                // right of the steep jump at equivalence, under the flat buffer region before it
                let (side, anchor) = if volume == e {
                    (Vec2::new(22.0, 0.0), Anchor::Right)
                } else {
                    (Vec2::new(14.0, 14.0), Anchor::BottomRight)
                };
                parts.push(label(ui, text, at + side, anchor, self.text_color));
            }
            let parts: Svgr<'a> = parts.into();
            texts.push(fframes::svgr!(<g opacity={shown}>{parts}</g>));
        }
        let curve = draw_animated(ui, &shapes, None, None, None);
        let parts: Svgr<'a> = vec![curve, texts.into()].into();
        fade(
            ui,
            parts,
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        )
    }
}

/// The colours of universal indicator from pH 0 to 14.
const INDICATOR: [&str; 15] = [
    "#e5262a", "#f0592b", "#f6a01a", "#f3d21b", "#d6df23", "#9cc93b", "#5cb946", "#2ea84a",
    "#1fa978", "#16a9a5", "#2a8fcb", "#3a64ad", "#4f4ea0", "#5f3f97", "#4a2a80",
];

/// The colour of universal indicator at `ph`, blended between whole values.
pub fn indicator_color(ph: f64) -> Color {
    let ph = ph.clamp(0.0, 14.0);
    let low = ph.floor() as usize;
    let high = (low + 1).min(14);
    fframes::animation::Animatable::apply_progress(
        &Color::hex(INDICATOR[low]),
        &Color::hex(INDICATOR[high]),
        (ph - low as f64) as f32,
    )
}

/// The pH scale as a bar of 15 cells in the colours of universal indicator, `width` by
/// `height` pixels around `center` (units of `ui.space`). With `labels` the values sit under
/// the cells and "acidic", "neutral" and "basic" under them. Each of `markers` (a name and
/// its pH) points at its place above the bar; after `delay` they appear `stagger` seconds
/// apart, in two rows so neighbours do not collide. A `pointer` marks a pH on the bar, for
/// a value that changes. The cells draw with `enter`, the rest fades with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct PhScale<'a> {
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub markers: Vec<(Cow<'a, str>, f64)>,
    pub pointer: Option<f64>,
    pub delay: f32,
    pub stagger: f32,
    pub labels: bool,
    pub text_color: Color,
    pub pointer_color: Color,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for PhScale<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.0, 0.0),
            width: 1350.0,
            height: 70.0,
            markers: Vec::new(),
            pointer: None,
            delay: 0.0,
            stagger: 0.4,
            labels: true,
            text_color: t.colors.text,
            pointer_color: t.colors.title,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for PhScale<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl PhScale<'_> {
    /// The pixel on the middle line of the bar at `ph`, the whole values at the cells'
    /// middles.
    pub fn point(&self, ui: &Ui<'_, '_, '_>, ph: f64) -> Point {
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let cell = f64::from(self.width) / 15.0;
        Point::new(
            middle.x - f64::from(self.width) / 2.0 + (ph.clamp(0.0, 14.0) + 0.5) * cell,
            middle.y,
        )
    }
}

impl<'a> Component<'a> for PhScale<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (w, h) = (f64::from(self.width), f64::from(self.height));
        let cell = w / 15.0;
        let left = self.point(ui, 0.0).x - cell / 2.0;
        let y = self.point(ui, 0.0).y;
        let rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
            kurbo::Shape::to_path(&kurbo::Rect::new(x0, y0, x1, y1), 0.1)
        };
        let cells: Vec<_> = (0..15)
            .map(|i| Shape {
                path: rect(
                    left + cell * i as f64,
                    y - h / 2.0,
                    left + cell * (i + 1) as f64,
                    y + h / 2.0,
                ),
                fill: Some(Color::hex(INDICATOR[i])),
                stroke: None,
                stroke_width: 0.0,
                dash: None,
                opacity: 1.0,
            })
            .collect();
        let cells = draw_animated(ui, &cells, self.enter.as_ref(), self.exit.as_ref(), None);

        let mut parts = Vec::new();
        if self.labels {
            for i in 0..15_u8 {
                let at = Point::new(left + cell * (f64::from(i) + 0.5), y + h / 2.0 + 12.0);
                parts.push(label(
                    ui,
                    i.to_string(),
                    at,
                    Anchor::Bottom,
                    self.text_color,
                ));
            }
            let under = y + h / 2.0 + 64.0;
            for (text, ph) in [("acidic", 1.5), ("neutral", 7.0), ("basic", 12.5)] {
                let at = Point::new(self.point(ui, ph).x, under);
                parts.push(label(
                    ui,
                    text.to_owned(),
                    at,
                    Anchor::Bottom,
                    self.text_color,
                ));
            }
        }

        let now = ui.seconds();
        for (i, (name, ph)) in self.markers.iter().enumerate() {
            let start = self.delay + self.stagger * i as f32;
            let shown = progress(now, start, start + 0.3);
            if shown <= 0.0 {
                continue;
            }
            let x = self.point(ui, *ph).x;
            let top = y - h / 2.0 - 8.0;
            let rise = if i % 2 == 0 { 0.0 } else { 44.0 };
            let mut tick = BezPath::new();
            tick.move_to((x, top - 26.0 - rise));
            tick.line_to((x, top));
            let mut tip = BezPath::new();
            tip.move_to((x - 9.0, top - 12.0));
            tip.line_to((x, top));
            tip.line_to((x + 9.0, top - 12.0));
            let stroke = |path: BezPath| Shape {
                path,
                fill: None,
                stroke: Some(self.text_color),
                stroke_width: ui.theme.stroke.thin,
                dash: None,
                opacity: 1.0,
            };
            let mark = draw_animated(ui, &[stroke(tick), stroke(tip)], None, None, None);
            let text = label(
                ui,
                name.to_string(),
                Point::new(x, top - 32.0 - rise),
                Anchor::Top,
                self.text_color,
            );
            parts.push(fframes::svgr!(<g opacity={shown}>{mark}{text}</g>));
        }

        if let Some(ph) = self.pointer {
            let x = self.point(ui, ph).x;
            let frame = Shape {
                path: rect(x - 7.0, y - h / 2.0 - 10.0, x + 7.0, y + h / 2.0 + 10.0),
                fill: Some(indicator_color(ph)),
                stroke: Some(self.pointer_color),
                stroke_width: ui.theme.stroke.regular,
                dash: None,
                opacity: 1.0,
            };
            parts.push(draw_animated(ui, &[frame], None, None, None));
        }
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{cells}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() < tolerance
    }

    #[test]
    fn strong_acid() {
        let t = Titration::strong(0.1, 25.0, 0.1);
        assert!(close(t.equivalence(), 25.0, 1e-12));
        assert!(close(t.ph(0.0), 1.0, 1e-3));
        assert!(close(t.ph(25.0), 7.0, 1e-3));
        // 2.5 mmol hydroxide over in 75 mL
        assert!(close(t.ph(50.0), 14.0 + (2.5_f64 / 75.0).log10(), 1e-3));
    }

    #[test]
    fn weak_acid() {
        let t = Titration::default();
        assert!(close(t.ph(0.0), 2.88, 0.01));
        // the buffer at half equivalence: pH = pKa
        assert!(close(t.ph(12.5), 4.76, 0.01));
        assert!(close(t.ph(25.0), 8.72, 0.01));
        let curve = t.curve(50.0, 100);
        assert!(
            curve.windows(2).all(|w| w[1].1 >= w[0].1),
            "the pH only rises"
        );
        assert!(curve.iter().any(|&(v, _)| v == 25.0));
    }

    #[test]
    fn indicator() {
        assert_eq!(indicator_color(-1.0), Color::hex(INDICATOR[0]));
        assert_eq!(indicator_color(14.0), Color::hex(INDICATOR[14]));
        assert_eq!(indicator_color(7.0), Color::hex(INDICATOR[7]));
    }
}
