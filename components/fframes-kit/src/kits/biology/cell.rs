//! Animal and plant cells with their organelles, each named by a leader line.

use std::f64::consts::PI;

use fframes::{Color, Svgr};
use kurbo::{Affine, Arc, BezPath, Circle, Ellipse, Point, RoundedRect, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::{Look, Shape},
    motion::{Rng, cubic_in_out, progress},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::callouts::{Callout, callouts};

/// The kind of cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellKind {
    #[default]
    Animal,
    Plant,
}

/// The parts of a cell a [`Cell`] draws and names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Organelle {
    CellWall,
    Membrane,
    Nucleus,
    Nucleolus,
    RoughEr,
    Golgi,
    Mitochondrion,
    Chloroplast,
    Vacuole,
    Lysosome,
    Ribosome,
}

impl Organelle {
    /// The parts of `kind`, outside in, in the order a [`Cell`] shows them by default.
    pub fn of(kind: CellKind) -> Vec<Organelle> {
        use Organelle::*;
        match kind {
            CellKind::Animal => vec![
                Membrane,
                Nucleus,
                Nucleolus,
                RoughEr,
                Golgi,
                Mitochondrion,
                Lysosome,
                Ribosome,
            ],
            CellKind::Plant => vec![
                CellWall,
                Membrane,
                Vacuole,
                Nucleus,
                Nucleolus,
                Chloroplast,
                Mitochondrion,
                RoughEr,
                Golgi,
                Ribosome,
            ],
        }
    }

    /// The name on its label.
    pub fn name(self) -> &'static str {
        match self {
            Organelle::CellWall => "cell wall",
            Organelle::Membrane => "cell membrane",
            Organelle::Nucleus => "nucleus",
            Organelle::Nucleolus => "nucleolus",
            Organelle::RoughEr => "rough ER",
            Organelle::Golgi => "Golgi apparatus",
            Organelle::Mitochondrion => "mitochondrion",
            Organelle::Chloroplast => "chloroplast",
            Organelle::Vacuole => "vacuole",
            Organelle::Lysosome => "lysosome",
            Organelle::Ribosome => "ribosomes",
        }
    }
}

/// The colours of a [`Cell`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellPalette {
    pub cytoplasm: Color,
    pub membrane: Color,
    pub wall: Color,
    pub nucleus: Color,
    pub envelope: Color,
    pub nucleolus: Color,
    pub er: Color,
    pub golgi: Color,
    pub mitochondrion: Color,
    pub cristae: Color,
    pub chloroplast: Color,
    pub grana: Color,
    pub vacuole: Color,
    pub tonoplast: Color,
    pub lysosome: Color,
    pub ribosome: Color,
}

impl Default for CellPalette {
    fn default() -> Self {
        Self {
            cytoplasm: Color::hex("#1b2735"),
            membrane: Color::hex("#f783ac"),
            wall: Color::hex("#2f9e44"),
            nucleus: Color::hex("#3d2a7a"),
            envelope: Color::hex("#9775fa"),
            nucleolus: Color::hex("#b197fc"),
            er: Color::hex("#e599f7"),
            golgi: Color::hex("#ffd43b"),
            mitochondrion: Color::hex("#c2410c"),
            cristae: Color::hex("#ffa94d"),
            chloroplast: Color::hex("#2b8a3e"),
            grana: Color::hex("#8ce99a"),
            vacuole: Color::hex("#173f5f"),
            tonoplast: Color::hex("#74c0fc"),
            lysosome: Color::hex("#ff6b6b"),
            ribosome: Color::hex("#ced4da"),
        }
    }
}

/// One organelle as drawn: its shapes in pixels, the point its leader starts from and the
/// point it grows from.
struct Part {
    organelle: Organelle,
    shapes: Vec<Shape>,
    anchor: Point,
    origin: Point,
}

fn filled(path: BezPath, fill: Color, stroke: Option<Color>, width: f64) -> Shape {
    Shape {
        path,
        fill: Some(fill),
        stroke,
        stroke_width: width as f32,
        dash: None,
        opacity: 1.0,
    }
}

fn outline(path: BezPath, color: Color, width: f64) -> Shape {
    Shape {
        path,
        fill: None,
        stroke: Some(color),
        stroke_width: width as f32,
        dash: None,
        opacity: 1.0,
    }
}

/// A smooth closed curve through `points` (Catmull-Rom, as cubic Béziers).
fn smooth_closed(points: &[Point]) -> BezPath {
    let n = points.len();
    let mut path = BezPath::new();
    path.move_to(points[0]);
    for i in 0..n {
        let (p0, p1, p2, p3) = (
            points[(i + n - 1) % n],
            points[i],
            points[(i + 1) % n],
            points[(i + 2) % n],
        );
        path.curve_to(p1 + (p2 - p0) / 6.0, p2 - (p3 - p1) / 6.0, p2);
    }
    path.close_path();
    path
}

/// Where the parts sit, in units of the cell's half width (`y` down), and how to draw them.
struct Build {
    to_px: Affine,
    scale: f64,
    palette: CellPalette,
}

impl Build {
    fn px(&self, x: f64, y: f64) -> Point {
        self.to_px * Point::new(x, y)
    }

    fn path(&self, shape: &impl kurbo::Shape) -> BezPath {
        self.to_px * shape.to_path(0.01)
    }

    fn membrane(&self, kind: CellKind) -> BezPath {
        match kind {
            CellKind::Animal => {
                let points: Vec<Point> = (0..24)
                    .map(|i| {
                        let a = f64::from(i) / 24.0 * 2.0 * PI;
                        let r = 1.0 + 0.025 * (3.0 * a + 0.6).sin() + 0.02 * (5.0 * a + 1.9).sin();
                        Point::new(a.cos() * r, a.sin() * 0.72 * r)
                    })
                    .collect();
                self.to_px * smooth_closed(&points)
            }
            CellKind::Plant => self.path(&RoundedRect::new(-0.95, -0.63, 0.95, 0.63, 0.09)),
        }
    }

    /// A mitochondrion at `at`, turned `angle` radians.
    fn mitochondrion(&self, at: (f64, f64), angle: f64) -> Vec<Shape> {
        let turn = Affine::translate((at.0, at.1)) * Affine::rotate(angle);
        let body = RoundedRect::new(-0.14, -0.065, 0.14, 0.065, 0.065).to_path(0.01);
        let mut cristae = BezPath::new();
        cristae.move_to((-0.1, 0.0));
        for i in 0..7 {
            let x = -0.1 + 0.2 * f64::from(i + 1) / 7.0;
            let y = if i % 2 == 0 { -0.035 } else { 0.035 };
            cristae.line_to((x, y));
        }
        vec![
            filled(
                self.to_px * turn * body,
                self.palette.mitochondrion,
                Some(self.palette.cristae),
                0.012 * self.scale,
            ),
            outline(
                self.to_px * turn * cristae,
                self.palette.cristae,
                0.01 * self.scale,
            ),
        ]
    }

    /// A chloroplast at `at`, turned `angle` radians, with three stacks of thylakoids.
    fn chloroplast(&self, at: (f64, f64), angle: f64) -> Vec<Shape> {
        let turn = Affine::translate((at.0, at.1)) * Affine::rotate(angle);
        let body = Ellipse::new((0.0, 0.0), (0.15, 0.065), 0.0).to_path(0.01);
        let mut grana = BezPath::new();
        for stack in [-0.08, 0.0, 0.08] {
            for k in [-0.025, 0.0, 0.025] {
                grana.move_to((stack - 0.025, k));
                grana.line_to((stack + 0.025, k));
            }
        }
        vec![
            filled(
                self.to_px * turn * body,
                self.palette.chloroplast,
                Some(self.palette.grana),
                0.01 * self.scale,
            ),
            outline(
                self.to_px * turn * grana,
                self.palette.grana,
                0.012 * self.scale,
            ),
        ]
    }

    fn nucleus(&self, at: (f64, f64), r: f64) -> Vec<Shape> {
        let mut pores = outline(
            self.path(&Circle::new(at, r - 0.025)),
            self.palette.envelope,
            0.008 * self.scale,
        );
        pores.dash = Some(((0.05 * self.scale) as f32, (0.025 * self.scale) as f32));
        vec![
            filled(
                self.path(&Circle::new(at, r)),
                self.palette.nucleus,
                Some(self.palette.envelope),
                0.014 * self.scale,
            ),
            pores,
        ]
    }

    /// Cisternae of the rough ER: arcs round the nucleus at `at`, studded with ribosomes.
    fn rough_er(&self, at: (f64, f64), radii: &[f64], start: f64, sweep: f64) -> Vec<Shape> {
        let mut shapes = Vec::new();
        let mut dots = BezPath::new();
        for &r in radii {
            let arc = Arc::new(at, (r, r), start, sweep, 0.0);
            shapes.push(outline(
                self.path(&arc),
                self.palette.er,
                0.018 * self.scale,
            ));
            let count = (r * sweep.abs() / 0.06).floor() as usize;
            for i in 0..=count {
                let a = start + sweep * i as f64 / count.max(1) as f64;
                let p = Point::new(at.0 + a.cos() * (r + 0.02), at.1 + a.sin() * (r + 0.02));
                dots.extend(Circle::new(p, 0.009).path_elements(0.001));
            }
        }
        shapes.push(filled(self.to_px * dots, self.palette.ribosome, None, 0.0));
        shapes
    }

    /// The Golgi apparatus at `at`: stacked, curved cisternae and two vesicles.
    fn golgi(&self, at: (f64, f64), width: f64) -> Vec<Shape> {
        let mut shapes = Vec::new();
        for k in 0..4 {
            let y = at.1 + f64::from(k) * 0.045;
            let w = width * (1.0 - f64::from(k) * 0.12);
            let mut sac = BezPath::new();
            sac.move_to((at.0 - w / 2.0, y));
            sac.quad_to((at.0, y - 0.07), (at.0 + w / 2.0, y));
            shapes.push(outline(
                self.to_px * sac,
                self.palette.golgi,
                0.026 * self.scale,
            ));
        }
        for (dx, dy) in [(width * 0.62, -0.02), (width * 0.55, 0.12)] {
            shapes.push(filled(
                self.path(&Circle::new((at.0 + dx, at.1 + dy), 0.022)),
                self.palette.golgi,
                None,
                0.0,
            ));
        }
        shapes
    }

    fn circles(&self, centers: &[(f64, f64)], r: f64, color: Color) -> Vec<Shape> {
        centers
            .iter()
            .map(|&c| filled(self.path(&Circle::new(c, r)), color, None, 0.0))
            .collect()
    }

    /// Free ribosomes: `count` dots placed at random in the cytoplasm from `seed`, clear of
    /// the round areas `keep_out` (centre, radius).
    fn ribosomes(
        &self,
        kind: CellKind,
        seed: u64,
        count: usize,
        keep_out: &[((f64, f64), f64)],
    ) -> Vec<Point> {
        let mut rng = Rng::new(seed);
        let mut dots = Vec::new();
        for _ in 0..count * 40 {
            if dots.len() == count {
                break;
            }
            let (x, y) = (rng.range(-0.9, 0.9), rng.range(-0.6, 0.6));
            let inside = match kind {
                CellKind::Animal => x * x + (y / 0.72) * (y / 0.72) < 0.8,
                CellKind::Plant => x.abs() < 0.88 && y.abs() < 0.56,
            };
            let clear = keep_out
                .iter()
                .all(|&((cx, cy), r)| (x - cx).hypot(y - cy) > r);
            if inside && clear {
                dots.push(Point::new(x, y));
            }
        }
        dots
    }

    fn parts(&self, kind: CellKind, seed: u64) -> Vec<Part> {
        let pal = self.palette;
        let part = |organelle, shapes, anchor: (f64, f64), origin: (f64, f64)| Part {
            organelle,
            shapes,
            anchor: self.px(anchor.0, anchor.1),
            origin: self.px(origin.0, origin.1),
        };
        let membrane = |anchor| {
            part(
                Organelle::Membrane,
                vec![filled(
                    self.membrane(kind),
                    pal.cytoplasm,
                    Some(pal.membrane),
                    0.02 * self.scale,
                )],
                anchor,
                (0.0, 0.0),
            )
        };
        let dots = |centers: &[Point]| {
            let mut path = BezPath::new();
            for &c in centers {
                path.extend(Circle::new(c, 0.012).path_elements(0.001));
            }
            vec![filled(self.to_px * path, pal.ribosome, None, 0.0)]
        };
        match kind {
            CellKind::Animal => {
                let nucleus = (-0.22, 0.02);
                let mitochondria = [
                    ((0.5, 0.28), 0.35),
                    ((-0.6, -0.36), -0.5),
                    ((0.12, -0.5), 0.08),
                ];
                let lysosomes = [(0.72, -0.04), (-0.5, 0.42)];
                let ribosomes = self.ribosomes(
                    kind,
                    seed,
                    26,
                    &[
                        (nucleus, 0.6),
                        ((0.45, -0.24), 0.2),
                        ((0.5, 0.28), 0.17),
                        ((-0.6, -0.36), 0.17),
                        ((0.12, -0.5), 0.17),
                        ((0.72, -0.04), 0.08),
                        ((-0.5, 0.42), 0.08),
                    ],
                );
                vec![
                    membrane((0.58, 0.58)),
                    part(
                        Organelle::Nucleus,
                        self.nucleus(nucleus, 0.3),
                        (-0.4, -0.2),
                        nucleus,
                    ),
                    part(
                        Organelle::Nucleolus,
                        self.circles(&[(-0.17, 0.06)], 0.09, pal.nucleolus),
                        (-0.2, 0.12),
                        (-0.17, 0.06),
                    ),
                    part(
                        Organelle::RoughEr,
                        self.rough_er(nucleus, &[0.38, 0.45, 0.52], -1.2, 2.4),
                        (0.284, 0.149),
                        nucleus,
                    ),
                    part(
                        Organelle::Golgi,
                        self.golgi((0.47, -0.3), 0.3),
                        (0.6, -0.27),
                        (0.47, -0.25),
                    ),
                    part(
                        Organelle::Mitochondrion,
                        mitochondria
                            .iter()
                            .flat_map(|&(at, angle)| self.mitochondrion(at, angle))
                            .collect(),
                        (0.6, 0.33),
                        (0.12, -0.05),
                    ),
                    part(
                        Organelle::Lysosome,
                        self.circles(&lysosomes, 0.05, pal.lysosome),
                        (0.72, -0.04),
                        (0.1, 0.2),
                    ),
                    part(
                        Organelle::Ribosome,
                        dots(&ribosomes),
                        (
                            ribosomes.first().map_or(0.0, |p| p.x),
                            ribosomes.first().map_or(0.0, |p| p.y),
                        ),
                        (0.0, 0.0),
                    ),
                ]
            }
            CellKind::Plant => {
                let nucleus = (-0.66, -0.28);
                let chloroplasts = [
                    ((0.0, -0.52), 0.0),
                    ((-0.32, -0.53), 0.0),
                    ((0.8, 0.12), PI / 2.0),
                    ((0.35, 0.53), 0.0),
                    ((-0.78, 0.28), PI / 2.0),
                ];
                let mitochondria = [((0.8, -0.34), PI / 2.0), ((-0.42, 0.5), 0.2)];
                let ribosomes = self.ribosomes(
                    kind,
                    seed,
                    16,
                    &[
                        ((0.15, 0.05), 0.52),
                        (nucleus, 0.42),
                        ((-0.55, 0.27), 0.18),
                        ((0.0, -0.52), 0.16),
                        ((-0.32, -0.53), 0.16),
                        ((0.8, 0.12), 0.16),
                        ((0.35, 0.53), 0.16),
                        ((-0.78, 0.28), 0.16),
                        ((0.8, -0.34), 0.15),
                        ((-0.42, 0.5), 0.15),
                    ],
                );
                let wall = self.path(&RoundedRect::new(-1.0, -0.68, 1.0, 0.68, 0.12));
                vec![
                    part(
                        Organelle::CellWall,
                        vec![outline(wall, pal.wall, 0.045 * self.scale)],
                        (-0.7, 0.68),
                        (0.0, 0.0),
                    ),
                    membrane((0.95, 0.4)),
                    part(
                        Organelle::Vacuole,
                        vec![filled(
                            self.path(&RoundedRect::new(-0.32, -0.35, 0.62, 0.43, 0.22)),
                            pal.vacuole,
                            Some(pal.tonoplast),
                            0.014 * self.scale,
                        )],
                        (0.3, 0.0),
                        (0.15, 0.05),
                    ),
                    part(
                        Organelle::Nucleus,
                        self.nucleus(nucleus, 0.2),
                        (-0.8, -0.38),
                        nucleus,
                    ),
                    part(
                        Organelle::Nucleolus,
                        self.circles(&[(-0.63, -0.25)], 0.06, pal.nucleolus),
                        (-0.63, -0.25),
                        (-0.63, -0.25),
                    ),
                    part(
                        Organelle::Chloroplast,
                        chloroplasts
                            .iter()
                            .flat_map(|&(at, angle)| self.chloroplast(at, angle))
                            .collect(),
                        (0.0, -0.52),
                        (0.0, 0.0),
                    ),
                    part(
                        Organelle::Mitochondrion,
                        mitochondria
                            .iter()
                            .flat_map(|&(at, angle)| self.mitochondrion(at, angle))
                            .collect(),
                        (0.8, -0.34),
                        (0.0, 0.0),
                    ),
                    part(
                        Organelle::RoughEr,
                        self.rough_er(nucleus, &[0.25, 0.3], 0.2, 1.5),
                        (-0.62, 0.018),
                        nucleus,
                    ),
                    part(
                        Organelle::Golgi,
                        self.golgi((-0.55, 0.22), 0.22),
                        (-0.6, 0.24),
                        (-0.55, 0.26),
                    ),
                    part(
                        Organelle::Ribosome,
                        dots(&ribosomes),
                        (
                            ribosomes.first().map_or(0.0, |p| p.x),
                            ribosomes.first().map_or(0.0, |p| p.y),
                        ),
                        (0.0, 0.0),
                    ),
                ]
            }
        }
    }
}

/// A cell, `size` pixels from its centre to its side, centred on `center` (units of
/// `ui.space`). From `delay` its `organelles` (all of `kind`'s, [`Organelle::of`], when
/// empty) appear one every `stagger` seconds in the order given: each fades in and grows
/// into place, the membrane and wall draw their outline first. With `labels` a leader line
/// then draws from each to its name, in columns left and right of the cell. Free ribosomes
/// are placed at random from `seed`. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Cell {
    pub kind: CellKind,
    pub organelles: Vec<Organelle>,
    pub delay: f32,
    pub stagger: f32,
    pub center: (f64, f64),
    pub size: f32,
    pub labels: bool,
    pub seed: u64,
    pub palette: CellPalette,
    pub leader_color: Color,
    pub label_color: Color,
    pub label_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for Cell {
    fn themed(t: &Theme) -> Self {
        Self {
            kind: CellKind::Animal,
            organelles: Vec::new(),
            delay: 0.0,
            stagger: 0.5,
            center: (0.0, 0.0),
            size: 330.0,
            labels: true,
            seed: 7,
            palette: CellPalette::default(),
            leader_color: t.colors.muted,
            label_color: t.colors.text,
            label_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Cell {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let scale = f64::from(self.size);
        let build = Build {
            to_px: Affine::translate(middle.to_vec2()) * Affine::scale(scale),
            scale,
            palette: self.palette,
        };
        let order = if self.organelles.is_empty() {
            Organelle::of(self.kind)
        } else {
            self.organelles.clone()
        };
        let parts = build.parts(self.kind, self.seed);
        let t = ui.seconds() - self.delay;

        let mut drawn = Vec::new();
        let mut named = Vec::new();
        for (i, organelle) in order.iter().enumerate() {
            let Some(part) = parts.iter().find(|p| p.organelle == *organelle) else {
                continue;
            };
            let start = i as f32 * self.stagger;
            let shown = cubic_in_out(progress(t, start, start + 0.5));
            if shown <= 0.0 {
                continue;
            }
            let outer = matches!(organelle, Organelle::Membrane | Organelle::CellWall);
            let grow = if outer {
                1.0
            } else {
                0.85 + 0.15 * f64::from(shown)
            };
            let around = part.origin.to_vec2();
            let mut look = Look {
                transform: Affine::translate(around)
                    * Affine::scale(grow)
                    * Affine::translate(-around),
                ..Look::default()
            };
            if outer {
                // the outline draws at an even pace, then the inside fills
                look.drawn = (0.0, progress(t, start, start + 1.2));
                for shape in &part.shapes {
                    let mut stroke_only = shape.clone();
                    stroke_only.fill = None;
                    let mut fill_only = shape.clone();
                    fill_only.stroke = None;
                    fill_only.opacity = progress(t, start + 0.6, start + 1.2);
                    drawn.push(fill_only.draw());
                    drawn.push(stroke_only.draw_with(&look));
                }
            } else {
                look.opacity = shown;
                drawn.extend(part.shapes.iter().map(|s| s.draw_with(&look)));
            }
            if self.labels {
                named.push(Callout {
                    text: organelle.name().into(),
                    anchor: part.anchor,
                    start: start + 0.35,
                });
            }
        }
        let left = middle.x - scale - 60.0;
        let right = middle.x + scale + 60.0;
        let labels = callouts(
            &named,
            t,
            (left, right),
            (middle.y - scale * 0.72, middle.y + scale * 0.72),
            self.label_style,
            self.label_color,
            self.leader_color,
        );
        let parts: Svgr<'a> = vec![drawn.into(), labels].into();
        fade(ui, parts, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit() -> Build {
        Build {
            to_px: Affine::IDENTITY,
            scale: 1.0,
            palette: CellPalette::default(),
        }
    }

    #[test]
    fn every_organelle_of_a_kind_is_drawn_inside_the_cell() {
        for kind in [CellKind::Animal, CellKind::Plant] {
            let parts = unit().parts(kind, 7);
            for organelle in Organelle::of(kind) {
                let part = parts.iter().find(|p| p.organelle == organelle);
                let part = part.unwrap_or_else(|| panic!("{kind:?} has no {organelle:?}"));
                assert_ne!(part.shapes.len(), 0);
                let a = part.anchor;
                assert!(
                    a.x.abs() <= 1.0 && a.y.abs() <= 0.72,
                    "{organelle:?} at {a:?}"
                );
            }
        }
    }

    #[test]
    fn ribosomes_are_seeded_and_keep_clear() {
        let keep_out = [((0.0, 0.0), 0.5)];
        let a = unit().ribosomes(CellKind::Animal, 3, 20, &keep_out);
        assert_eq!(a.len(), 20);
        assert_eq!(a, unit().ribosomes(CellKind::Animal, 3, 20, &keep_out));
        assert_ne!(a, unit().ribosomes(CellKind::Animal, 4, 20, &keep_out));
        assert!(a.iter().all(|p| p.x.hypot(p.y) > 0.5));
    }
}
