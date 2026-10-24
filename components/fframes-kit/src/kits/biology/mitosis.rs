//! Mitosis: one cell dividing into two, stage by stage, from interphase to cytokinesis.

use std::f64::consts::PI;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Point, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    kits::draw::{partial, stroke},
    motion::{Rng, cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::cell::CellPalette;

/// A stage of mitosis, in the order a [`MitosisStages`] plays them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MitosisStage {
    Interphase,
    Prophase,
    Metaphase,
    Anaphase,
    Telophase,
    Cytokinesis,
}

impl MitosisStage {
    /// Every stage, in order.
    pub const ALL: [MitosisStage; 6] = [
        MitosisStage::Interphase,
        MitosisStage::Prophase,
        MitosisStage::Metaphase,
        MitosisStage::Anaphase,
        MitosisStage::Telophase,
        MitosisStage::Cytokinesis,
    ];

    /// The name on the stage label.
    pub fn name(self) -> &'static str {
        match self {
            MitosisStage::Interphase => "Interphase",
            MitosisStage::Prophase => "Prophase",
            MitosisStage::Metaphase => "Metaphase",
            MitosisStage::Anaphase => "Anaphase",
            MitosisStage::Telophase => "Telophase",
            MitosisStage::Cytokinesis => "Cytokinesis",
        }
    }
}

/// `progress` eased, on the stage clock.
fn ease(s: f32, start: f32, end: f32) -> f64 {
    f64::from(cubic_in_out(progress(s, start, end)))
}

/// The cell's outline: an ellipse `rx` by `ry` stretched along x by `stretch`, its waist
/// pinched in by `pinch` (1 closes it in the middle).
fn outline(center: Point, rx: f64, ry: f64, stretch: f64, pinch: f64) -> BezPath {
    let mut path = BezPath::new();
    let steps = 120;
    for i in 0..steps {
        let a = 2.0 * PI * f64::from(i) / f64::from(steps);
        let (cos, sin) = (a.cos(), a.sin());
        let waist = 1.0 - pinch * (-(cos * cos) / 0.06).exp();
        let p = Point::new(
            center.x + rx * (1.0 + stretch) * cos,
            center.y + ry * sin * waist,
        );
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
}

/// One chromatid around its centromere `at`: arms `arm` long up and down, their ends bent
/// `bend` pixels towards `side` (+1 right, -1 left), wavy by `wave` while uncondensed.
fn chromatid(at: Point, arm: f64, bend: f64, side: f64, wave: f64, phase: f64) -> BezPath {
    let mut path = BezPath::new();
    let steps = 24;
    for i in 0..=steps {
        let u = -1.0 + 2.0 * f64::from(i) / f64::from(steps);
        let p = Point::new(
            at.x + side * bend * u * u + wave * (u * 9.0 + phase).sin(),
            at.y + arm * u,
        );
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path
}

fn disc(center: Point, radius: f64, fill: Color, edge: Color, width: f32, opacity: f32) -> Shape {
    Shape {
        path: Circle::new(center, radius).to_path(0.1),
        fill: Some(fill),
        stroke: Some(edge),
        stroke_width: width,
        dash: None,
        opacity,
    }
}

/// Mitosis: a cell `size` pixels from its centre to its side, centred on `center` (units of
/// `ui.space`), going through the stages of [`MitosisStage::ALL`] from `delay`, each
/// `stage_time` seconds long. In interphase its `chromosomes` lie loose in the nucleus; in
/// prophase they condense into sister chromatids joined at the centromere, the nuclear
/// envelope breaks down and the centrosomes move to the poles; in metaphase spindle fibres
/// line them up in the middle; in anaphase the sisters are pulled to opposite poles; in
/// telophase two nuclei form around them; in cytokinesis the cell pinches into two.
/// Chromosomes come in homologous pairs of one length, coloured from `colors` by parent;
/// where each lies in interphase comes from `seed`. With `label` the stage is named above.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct MitosisStages {
    pub delay: f32,
    pub stage_time: f32,
    pub center: (f64, f64),
    pub size: f32,
    pub chromosomes: usize,
    pub arm: f32,
    pub colors: Vec<Color>,
    pub seed: u64,
    pub palette: CellPalette,
    pub spindle_color: Color,
    pub label: bool,
    pub label_style: TextStyle,
    pub label_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl MitosisStages {
    /// The stage shown `seconds` into the video.
    pub fn stage_at(&self, seconds: f32) -> MitosisStage {
        let s = (seconds - self.delay) / self.stage_time.max(1e-3);
        MitosisStage::ALL[s.floor().clamp(0.0, 5.0) as usize]
    }
}

impl Themed for MitosisStages {
    fn themed(t: &Theme) -> Self {
        Self {
            delay: 0.0,
            stage_time: 1.6,
            center: (0.0, -0.3),
            size: 300.0,
            chromosomes: 4,
            arm: 30.0,
            colors: vec![Color::hex("#ff8787"), Color::hex("#74c0fc")],
            seed: 3,
            palette: CellPalette::default(),
            spindle_color: Color::hex("#ced4da"),
            label: true,
            label_style: t.typography.body,
            label_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for MitosisStages {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for MitosisStages {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let s = (ui.seconds() - self.delay) / self.stage_time.max(1e-3);
        let rx = f64::from(self.size);
        let ry = rx * 0.78;
        let nucleus_r = rx * 0.4;
        let palette = &self.palette;

        // the cell stretches in anaphase and telophase, then pinches in two
        let stretch = 0.3 * ease(s, 3.2, 5.0);
        let pinch = ease(s, 4.5, 5.85);
        let pole = rx * 0.86 * (1.0 + stretch);
        let mut out = vec![
            Shape {
                path: outline(middle, rx, ry, stretch, pinch),
                fill: Some(palette.cytoplasm),
                stroke: Some(palette.membrane),
                stroke_width: 5.0,
                dash: None,
                opacity: 1.0,
            }
            .draw(),
        ];

        // the nucleus of interphase breaks down in prophase
        let envelope = 1.0 - ease(s, 1.3, 2.0) as f32;
        if envelope > 0.0 {
            out.push(
                disc(
                    middle,
                    nucleus_r * (1.0 + 0.08 * (1.0 - f64::from(envelope))),
                    palette.nucleus,
                    palette.envelope,
                    3.0,
                    envelope,
                )
                .draw(),
            );
            let nucleolus = 1.0 - ease(s, 1.0, 1.5) as f32;
            out.push(
                disc(
                    Point::new(middle.x + nucleus_r * 0.35, middle.y - nucleus_r * 0.3),
                    nucleus_r * 0.18,
                    palette.nucleolus,
                    palette.nucleolus,
                    1.0,
                    nucleolus * 0.8,
                )
                .draw(),
            );
        }

        // where the chromosomes end up: their groups in the daughter cells
        let reach = pole * 0.72;
        // two nuclei form around the groups in telophase
        let daughters = ease(s, 4.3, 5.0) as f32;
        if daughters > 0.0 {
            for side in [-1.0, 1.0] {
                out.push(
                    disc(
                        Point::new(middle.x + side * reach, middle.y),
                        nucleus_r * (0.55 + 0.15 * f64::from(daughters)),
                        palette.nucleus,
                        palette.envelope,
                        3.0,
                        daughters,
                    )
                    .draw(),
                );
            }
        }

        // the centrosomes move apart from beside the nucleus to the poles
        let apart = ease(s, 1.0, 2.2);
        let poles: Vec<Point> = [-1.0, 1.0]
            .iter()
            .map(|&side| {
                let angle = -PI / 2.0 + side * PI / 2.0 * apart;
                let r = (nucleus_r + 26.0) + (pole - nucleus_r - 26.0) * apart;
                Point::new(middle.x + r * angle.cos(), middle.y + r * angle.sin())
            })
            .collect();

        let n = self.chromosomes.max(1);
        let arm_of =
            |i: usize| f64::from(self.arm) * if (i / 2).is_multiple_of(2) { 1.0 } else { 0.7 };
        let gap = (ry * 1.4 / n as f64).min(f64::from(self.arm) * 2.0 + 18.0);
        let mut rng = Rng::new(self.seed);
        let homes: Vec<Point> = (0..n)
            .map(|i| {
                let angle = 2.0 * PI * i as f64 / n as f64 + rng.range(-0.4, 0.4);
                let r = nucleus_r * rng.range(0.3, 0.55);
                Point::new(middle.x + r * angle.cos(), middle.y + r * angle.sin())
            })
            .collect();

        let condensed = ease(s, 1.0, 1.8) * (1.0 - 0.7 * ease(s, 4.2, 5.0));
        let to_plate = ease(s, 2.0, 2.7);
        let pulled = ease(s, 3.1, 3.9);
        // in telophase each group gathers into its new nucleus
        let gathered = ease(s, 4.0, 4.8);
        let fibres = (ease(s, 1.8, 2.3) * (1.0 - ease(s, 4.0, 4.5))) as f32;
        let grown = ease(s, 1.8, 2.6);

        let mut spindle = Vec::new();
        let mut chromatids = Vec::new();
        for (i, &home) in homes.iter().enumerate() {
            let arm = arm_of(i) * (1.0 - 0.4 * gathered);
            let plate_y = middle.y + (i as f64 - (n as f64 - 1.0) / 2.0) * gap;
            let centre = Point::new(
                home.x + (middle.x - home.x) * to_plate,
                home.y + (plate_y - home.y) * to_plate,
            );
            // one of each pair from each parent
            let color = match self.colors.len() {
                0 => palette.membrane,
                len => self.colors[(i % 2) % len],
            };
            for (k, side) in [-1.0, 1.0].into_iter().enumerate() {
                let x = centre.x + side * (2.0 + 3.0 * condensed + (reach - 3.0) * pulled);
                let y = centre.y + (middle.y - centre.y) * (0.3 * pulled + 0.55 * gathered);
                let at = Point::new(x, y);
                // the arms spread apart into an X, then trail behind the centromere
                let bend = arm * (0.35 * condensed - 0.95 * pulled);
                let path = chromatid(
                    at,
                    arm * (1.0 + 1.2 * (1.0 - condensed)),
                    bend,
                    side,
                    10.0 * (1.0 - condensed),
                    i as f64 * 1.7 + k as f64 * 0.6,
                );
                chromatids.push(
                    stroke(
                        path,
                        color,
                        (3.0 + 9.0 * condensed) as f32,
                        (0.55 + 0.45 * condensed) as f32,
                    )
                    .draw(),
                );
                if fibres > 0.0 {
                    let from = poles[k];
                    let reached = partial(&[from, at], grown);
                    let mut fibre = BezPath::new();
                    fibre.move_to(reached[0]);
                    for p in &reached[1..] {
                        fibre.line_to(*p);
                    }
                    spindle.push(stroke(fibre, self.spindle_color, 2.0, 0.7 * fibres).draw());
                }
            }
        }
        // polar fibres reach past the middle from both poles
        if fibres > 0.0 {
            for (k, side) in [-1.0, 1.0].into_iter().enumerate() {
                for dy in [-0.3, 0.3] {
                    let end = Point::new(middle.x - side * pole * 0.2, middle.y + dy * ry);
                    let reached = partial(&[poles[k], end], grown);
                    let mut fibre = BezPath::new();
                    fibre.move_to(reached[0]);
                    for p in &reached[1..] {
                        fibre.line_to(*p);
                    }
                    spindle.push(stroke(fibre, self.spindle_color, 1.5, 0.4 * fibres).draw());
                }
            }
        }
        out.extend(spindle);
        out.extend(chromatids);

        // the centrosomes, with their asters once they move
        let aster = ease(s, 1.2, 1.8) * (1.0 - ease(s, 4.2, 4.8));
        for at in &poles {
            if aster > 0.0 {
                for ray in 0..8 {
                    let a = 2.0 * PI * f64::from(ray) / 8.0;
                    let mut path = BezPath::new();
                    path.move_to((at.x + 12.0 * a.cos(), at.y + 12.0 * a.sin()));
                    path.line_to((at.x + 26.0 * a.cos(), at.y + 26.0 * a.sin()));
                    out.push(stroke(path, self.spindle_color, 2.0, 0.5 * aster as f32).draw());
                }
            }
            out.push(disc(*at, 8.0, self.spindle_color, self.spindle_color, 1.0, 1.0).draw());
        }

        if self.label {
            let y = (middle.y - ry - 60.0) as f32;
            for (k, stage) in MitosisStage::ALL.iter().enumerate() {
                let k = k as f32;
                let shown_in = if k == 0.0 {
                    1.0
                } else {
                    progress(s, k - 0.12, k + 0.12)
                };
                let shown_out = if k == 5.0 {
                    1.0
                } else {
                    1.0 - progress(s, k + 0.88, k + 1.12)
                };
                let shown = shown_in * shown_out;
                if shown > 0.0 {
                    out.push(line(
                        middle.x as f32,
                        y,
                        self.label_style,
                        self.label_color,
                        TextAnchor::Middle,
                        0.0,
                        shown,
                        stage.name().into(),
                    ));
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
    fn stages_follow_the_clock() {
        let figure = MitosisStages {
            delay: 1.0,
            stage_time: 2.0,
            ..MitosisStages::default()
        };
        assert_eq!(figure.stage_at(0.0), MitosisStage::Interphase);
        assert_eq!(figure.stage_at(3.5), MitosisStage::Prophase);
        assert_eq!(figure.stage_at(7.0), MitosisStage::Anaphase);
        assert_eq!(figure.stage_at(100.0), MitosisStage::Cytokinesis);
    }

    #[test]
    fn the_pinched_outline_closes_in_the_middle() {
        let center = Point::new(0.0, 0.0);
        let path = outline(center, 100.0, 80.0, 0.3, 1.0);
        let near_waist = path
            .elements()
            .iter()
            .filter_map(kurbo::PathEl::end_point)
            .filter(|p| p.x.abs() < 1.0)
            .all(|p| p.y.abs() < 1.0);
        assert!(near_waist);
        let open = outline(center, 100.0, 80.0, 0.0, 0.0);
        assert!((open.bounding_box().height() - 160.0).abs() < 0.5);
    }
}
