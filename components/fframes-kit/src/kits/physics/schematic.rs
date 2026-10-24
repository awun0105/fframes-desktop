use std::f64::consts::FRAC_1_SQRT_2;

use fframes::{Color, Svgr};
use kurbo::{Affine, BezPath, Circle as KCircle, Point, Shape as _, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::Space,
    geometry::Shape,
    layout::Anchor,
    motion::{cubic_in_out, progress},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{label, zigzag};

/// Seconds a switch takes to close and the angle of an open one, as in [`Circuit`](super::Circuit).
const CLOSING: f32 = 0.3;
const OPEN_DEGREES: f64 = 35.0;
/// Pixels: the current's dots, the junction dots.
const DOT_RADIUS: f32 = 7.0;
const JUNCTION_RADIUS: f64 = 6.0;
/// Units within which two points of a schematic are the same point.
const SAME: f64 = 1e-6;

/// A Manim-circuit part, or one of the kit's own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolKind {
    Resistor,
    Capacitor,
    Inductor,
    /// A circle with `+` and `−`, `+` at terminal `b`.
    VoltageSource,
    /// A circle with an arrow from `a` to `b`.
    CurrentSource,
    /// One terminal on top.
    Ground,
    /// Inputs `-` (above) and `+` on the left, `out` on the right.
    Opamp,
    /// The long plate is `+`, at terminal `b`.
    Battery,
    /// Open, then closing over 0.3 s from `closes_at` seconds of local time; never with
    /// `None`.
    Switch {
        closes_at: Option<f32>,
    },
    /// Glows with the symbol's `lit`, or once current flows in a [`Schematic`].
    Bulb,
    /// Conducts from `a` (anode) to `b` (cathode), the triangle pointing at the bar.
    Diode,
}

type Terminals = &'static [(&'static str, (f64, f64))];

impl SymbolKind {
    /// The terminals by name, in the symbol's own units: a two-terminal part runs from
    /// `(-1, 0)` (`a`) to `(1, 0)` (`b`), with `-` and `+` for the sources and `anode` and
    /// `cathode` for the diode as other names of the same points.
    pub fn terminals(self) -> Terminals {
        const TWO: Terminals = &[("a", (-1.0, 0.0)), ("b", (1.0, 0.0))];
        const POLAR: Terminals = &[
            ("a", (-1.0, 0.0)),
            ("b", (1.0, 0.0)),
            ("-", (-1.0, 0.0)),
            ("+", (1.0, 0.0)),
        ];
        const DIODE: Terminals = &[
            ("a", (-1.0, 0.0)),
            ("b", (1.0, 0.0)),
            ("anode", (-1.0, 0.0)),
            ("cathode", (1.0, 0.0)),
        ];
        const GROUND: Terminals = &[("a", (0.0, 0.0))];
        const OPAMP: Terminals = &[("-", (-1.0, 0.4)), ("+", (-1.0, -0.4)), ("out", (1.0, 0.0))];
        match self {
            SymbolKind::VoltageSource | SymbolKind::Battery => POLAR,
            SymbolKind::Diode => DIODE,
            SymbolKind::Ground => GROUND,
            SymbolKind::Opamp => OPAMP,
            _ => TWO,
        }
    }

    /// Where the label goes, in the symbol's units, before `flip_label`.
    fn label_at(self) -> (f64, f64) {
        match self {
            SymbolKind::Ground => (0.0, -0.95),
            SymbolKind::Opamp => (0.0, 1.0),
            SymbolKind::VoltageSource
            | SymbolKind::CurrentSource
            | SymbolKind::Bulb
            | SymbolKind::Switch { .. } => (0.0, 0.7),
            _ => (0.0, 0.6),
        }
    }
}

/// A circuit symbol, Manim-circuit's `Resistor`, `Capacitor` and the rest: `kind` drawn at
/// `at` (units of `ui.space`), turned `rotation` degrees counterclockwise, `size` units from
/// its middle to a terminal. [`terminal`](Self::terminal) gives where a wire attaches. The
/// label is drawn beside it, on the other side with `flip_label`; an empty one is not
/// drawn. The symbol draws with `enter`, the label and a bulb's glow (`lit`, 0 to 1) fade
/// with `parts_enter`. A [`Schematic`] wires symbols together.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Symbol<'a> {
    pub kind: SymbolKind,
    pub at: (f64, f64),
    pub rotation: f64,
    pub size: f64,
    pub label: &'a str,
    pub flip_label: bool,
    pub lit: f32,
    pub color: Color,
    pub label_color: Color,
    pub glow_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Symbol<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            kind: SymbolKind::Resistor,
            at: (0.0, 0.0),
            rotation: 0.0,
            size: 0.6,
            label: "",
            flip_label: false,
            lit: 0.0,
            color: t.colors.text,
            label_color: t.colors.text,
            glow_color: t.colors.highlight,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Symbol<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// What a symbol draws: strokes, thick strokes, filled shapes in its own units, `+` (true)
/// and `−` signs that stay upright when it turns, and the bulb's glow (center and radius in
/// its units).
#[derive(Default)]
struct Drawing {
    thin: Vec<BezPath>,
    thick: Vec<BezPath>,
    filled: Vec<BezPath>,
    signs: Vec<((f64, f64), bool)>,
    glow: Option<((f64, f64), f64)>,
}

/// Half the width of a sign, in the symbol's units.
const SIGN: f64 = 0.1;

fn polyline(points: &[(f64, f64)]) -> BezPath {
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

fn seg(a: (f64, f64), b: (f64, f64)) -> BezPath {
    polyline(&[a, b])
}

fn ring(center: (f64, f64), radius: f64) -> BezPath {
    KCircle::new(center, radius).to_path(0.001)
}

/// Leads from both terminals to `±inner` on the axis.
fn leads(inner: f64) -> [BezPath; 2] {
    [
        seg((-1.0, 0.0), (-inner, 0.0)),
        seg((inner, 0.0), (1.0, 0.0)),
    ]
}

impl Symbol<'_> {
    /// The symbol's units to pixels.
    fn placement(&self, space: Space) -> Affine {
        let k = self.size * f64::from(space.unit);
        Affine::translate(space.to_px(self.at.0, self.at.1).to_vec2())
            * Affine::scale_non_uniform(k, -k)
            * Affine::rotate(self.rotation.to_radians())
    }

    /// A point in the symbol's units, in units of `ui.space`.
    fn place(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        (
            self.at.0 + self.size * (x * cos - y * sin),
            self.at.1 + self.size * (x * sin + y * cos),
        )
    }

    /// Where the terminal `name` is, in units of `ui.space`; `None` for a name the kind
    /// does not have.
    pub fn terminal(&self, name: &str) -> Option<(f64, f64)> {
        self.kind
            .terminals()
            .iter()
            .find(|(n, _)| *n == name)
            .map(|&(_, p)| self.place(p))
    }

    /// The angle of a switch's lever at `now` seconds, from open (35°) to closed (0).
    pub fn lever_angle(&self, now: f32) -> f64 {
        let closing = match self.kind {
            SymbolKind::Switch {
                closes_at: Some(at),
            } => cubic_in_out(progress(now, at, at + CLOSING)),
            _ => 0.0,
        };
        OPEN_DEGREES.to_radians() * f64::from(1.0 - closing)
    }

    fn drawing(&self, now: f32) -> Drawing {
        let mut d = Drawing::default();
        match self.kind {
            SymbolKind::Resistor => {
                d.thin.extend(leads(0.6));
                d.thin
                    .push(zigzag((-0.6, 0.0).into(), (0.6, 0.0).into(), 4, 0.2));
            }
            SymbolKind::Capacitor => {
                d.thin.extend(leads(0.12));
                d.thin.push(seg((-0.12, -0.45), (-0.12, 0.45)));
                d.thin.push(seg((0.12, -0.45), (0.12, 0.45)));
            }
            SymbolKind::Inductor => {
                // four half-turns of 0.15 above the axis
                let mut points = vec![(-1.0, 0.0)];
                for bump in 0..4 {
                    let x0 = -0.6 + 0.3 * f64::from(bump);
                    for i in 0..=12 {
                        let a = std::f64::consts::PI * f64::from(i) / 12.0;
                        points.push((x0 + 0.15 - 0.15 * a.cos(), 0.15 * a.sin()));
                    }
                }
                points.push((1.0, 0.0));
                d.thin.push(polyline(&points));
            }
            SymbolKind::VoltageSource => {
                d.thin.extend(leads(0.45));
                d.thin.push(ring((0.0, 0.0), 0.45));
                d.signs.push(((0.2, 0.0), true));
                d.signs.push(((-0.2, 0.0), false));
            }
            SymbolKind::CurrentSource => {
                d.thin.extend(leads(0.45));
                d.thin.push(ring((0.0, 0.0), 0.45));
                d.thin.push(seg((-0.25, 0.0), (0.25, 0.0)));
                d.thin
                    .push(polyline(&[(0.1, 0.12), (0.25, 0.0), (0.1, -0.12)]));
            }
            SymbolKind::Ground => {
                d.thin.push(seg((0.0, 0.0), (0.0, -0.4)));
                for (y, half) in [(-0.4, 0.4), (-0.55, 0.26), (-0.7, 0.12)] {
                    d.thin.push(seg((-half, y), (half, y)));
                }
            }
            SymbolKind::Opamp => {
                let mut triangle = polyline(&[(-0.7, 0.8), (-0.7, -0.8), (0.7, 0.0)]);
                triangle.close_path();
                d.thin.push(triangle);
                d.thin.push(seg((-1.0, 0.4), (-0.7, 0.4)));
                d.thin.push(seg((-1.0, -0.4), (-0.7, -0.4)));
                d.thin.push(seg((0.7, 0.0), (1.0, 0.0)));
                d.signs.push(((-0.5, 0.4), false));
                d.signs.push(((-0.5, -0.4), true));
            }
            SymbolKind::Battery => {
                d.thin.extend(leads(0.1));
                d.thin.push(seg((0.1, -0.45), (0.1, 0.45)));
                d.thick.push(seg((-0.1, -0.22), (-0.1, 0.22)));
            }
            SymbolKind::Switch { .. } => {
                d.thin.extend(leads(0.5));
                d.filled.push(ring((-0.5, 0.0), 0.07));
                d.filled.push(ring((0.5, 0.0), 0.07));
                let a = self.lever_angle(now);
                d.thin.push(seg((-0.5, 0.0), (-0.5 + a.cos(), a.sin())));
            }
            SymbolKind::Bulb => {
                d.thin.extend(leads(0.4));
                d.thin.push(ring((0.0, 0.0), 0.4));
                let c = 0.4 * FRAC_1_SQRT_2;
                d.thin.push(seg((-c, -c), (c, c)));
                d.thin.push(seg((-c, c), (c, -c)));
                d.glow = Some(((0.0, 0.0), 0.4));
            }
            SymbolKind::Diode => {
                d.thin.extend(leads(0.3));
                let mut triangle = polyline(&[(-0.3, 0.3), (-0.3, -0.3), (0.3, 0.0)]);
                triangle.close_path();
                d.filled.push(triangle);
                d.thin.push(seg((0.3, -0.3), (0.3, 0.3)));
            }
        }
        d
    }

    /// The symbol's strokes and fills, the glow of a bulb lit `lit`, and where its label
    /// goes, in pixels.
    fn parts(&self, space: Space, now: f32, lit: f32) -> (Vec<Shape>, Vec<Shape>, (Point, Anchor)) {
        let place = self.placement(space);
        let drawing = self.drawing(now);
        let stroke = |path: BezPath, width: f32, fill: Option<Color>| Shape {
            path: place * path,
            fill,
            stroke: Some(self.color),
            stroke_width: width,
            dash: None,
            opacity: 1.0,
        };
        let mut shapes = Vec::new();
        for path in drawing.thin {
            shapes.push(stroke(path, self.stroke_width, None));
        }
        for path in drawing.thick {
            shapes.push(stroke(path, self.stroke_width * 2.5, None));
        }
        for path in drawing.filled {
            shapes.push(stroke(path, self.stroke_width, Some(self.color)));
        }
        let half = SIGN * self.size * f64::from(space.unit);
        for (at, plus) in drawing.signs {
            let c = place * Point::from(at);
            let mut path = seg((c.x - half, c.y), (c.x + half, c.y));
            if plus {
                path.extend(seg((c.x, c.y - half), (c.x, c.y + half)));
            }
            shapes.push(Shape {
                path,
                ..stroke(BezPath::new(), self.stroke_width, None)
            });
        }
        let mut glow = Vec::new();
        if let (Some((center, radius)), true) = (drawing.glow, lit > 0.0) {
            for (scale, opacity) in [(2.2, 0.2), (1.0, 0.85)] {
                glow.push(Shape {
                    path: place * ring(center, radius * scale),
                    fill: Some(self.glow_color),
                    stroke: None,
                    stroke_width: 0.0,
                    dash: None,
                    opacity: lit.min(1.0) * opacity,
                });
            }
        }
        let (x, y) = self.kind.label_at();
        let y = if self.flip_label { -y } else { y };
        let middle = place * Point::ORIGIN;
        let at = place * Point::new(x, y);
        let off = at - middle;
        let anchor = if off.x.abs() > off.y.abs() {
            if off.x > 0.0 {
                Anchor::Right
            } else {
                Anchor::Left
            }
        } else if off.y > 0.0 {
            Anchor::Bottom
        } else {
            Anchor::Top
        };
        (shapes, glow, (at, anchor))
    }
}

impl<'a> Component<'a> for Symbol<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (shapes, glow, (at, anchor)) = self.parts(ui.space, ui.seconds(), self.lit);
        let drawn = draw_animated(ui, &shapes, self.enter.as_ref(), self.exit.as_ref(), None);
        let glow = draw_animated(ui, &glow, None, None, None);
        let text = if self.label.is_empty() {
            Svgr::empty()
        } else {
            label(ui, self.label.to_owned(), at, anchor, self.label_color)
        };
        let (enter, exit) = (self.parts_enter.as_ref(), self.parts_exit.as_ref());
        // the glow under the bulb's lines, the label on top
        let glow = fade(ui, glow, enter, exit);
        let text = fade(ui, text, enter, exit);
        fframes::svgr!(<g>{glow}{drawn}{text}</g>)
    }
}

/// One end of a [`Connection`]: a symbol's terminal (its index in `symbols` and the
/// terminal's name), or a point in units of `ui.space`, for a wire that ends on another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pin<'a> {
    Terminal(usize, &'a str),
    At(f64, f64),
}

impl<'a> From<(usize, &'a str)> for Pin<'a> {
    fn from((symbol, name): (usize, &'a str)) -> Self {
        Pin::Terminal(symbol, name)
    }
}

impl From<(f64, f64)> for Pin<'_> {
    fn from((x, y): (f64, f64)) -> Self {
        Pin::At(x, y)
    }
}

/// A wire of a [`Schematic`] from `from` to `to` through the corners `via` (units of
/// `ui.space`). Without corners a wire whose ends are not in line goes horizontally first,
/// then vertically. Its current flows from `from` to `to` at `current` times the
/// schematic's speed: negative runs the other way, 0 shows no dots.
#[derive(Debug, Clone, PartialEq)]
pub struct Connection<'a> {
    pub from: Pin<'a>,
    pub to: Pin<'a>,
    pub via: Vec<(f64, f64)>,
    pub current: f32,
}

impl<'a> Connection<'a> {
    pub fn new(from: impl Into<Pin<'a>>, to: impl Into<Pin<'a>>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            via: Vec::new(),
            current: 1.0,
        }
    }

    /// The corners the wire turns at, in order.
    #[must_use]
    pub fn via(self, corners: impl IntoIterator<Item = (f64, f64)>) -> Self {
        Self {
            via: corners.into_iter().collect(),
            ..self
        }
    }

    #[must_use]
    pub fn current(self, current: f32) -> Self {
        Self { current, ..self }
    }
}

/// Symbols wired together. `symbols` are drawn as they are (their `enter` and `exit` aside)
/// and `wires` connect their terminals at right angles through the corners each one gives;
/// there is no automatic routing. Where three or more conductors meet (a terminal with two
/// wires, or a wire ending on another) a junction dot is drawn with `junctions`.
///
/// From `current_at` seconds of local time the current flows as dots along every wire,
/// `dot_spacing` pixels apart at `current_speed` pixels a second, and bulbs light up. Each
/// wire moves its own dots, so they do not line up from one wire to the next. A wire whose
/// terminal does not exist is left out; [`unresolved`](Self::unresolved) lists them. The
/// wires and symbols draw with `enter`, the labels, dots and glow fade with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Schematic<'a> {
    pub symbols: Vec<Symbol<'a>>,
    pub wires: Vec<Connection<'a>>,
    pub current_at: Option<f32>,
    pub current_speed: f32,
    pub dot_spacing: f32,
    pub junctions: bool,
    pub wire_color: Color,
    pub dot_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Schematic<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            symbols: Vec::new(),
            wires: Vec::new(),
            current_at: None,
            current_speed: 160.0,
            dot_spacing: 56.0,
            junctions: true,
            wire_color: t.colors.text,
            dot_color: t.colors.accent,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Schematic<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

fn same(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < SAME && (a.1 - b.1).abs() < SAME
}

/// Whether `p` lies on the segment `a`–`b` of a wire.
fn on_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> bool {
    let (ab, ap) = (
        Vec2::new(b.0 - a.0, b.1 - a.1),
        Vec2::new(p.0 - a.0, p.1 - a.1),
    );
    let length = ab.hypot();
    if length < SAME {
        return same(p, a);
    }
    let along = ap.dot(ab) / length;
    ab.cross(ap).abs() / length < SAME && (-SAME..=length + SAME).contains(&along)
}

/// The point `s` pixels along the polyline `points`.
fn along(points: &[Point], s: f64) -> Point {
    let mut left = s;
    for pair in points.windows(2) {
        let length = (pair[1] - pair[0]).hypot();
        if left <= length {
            return pair[0].lerp(pair[1], if length > 0.0 { left / length } else { 0.0 });
        }
        left -= length;
    }
    points.last().copied().unwrap_or(Point::ORIGIN)
}

/// Positions along a wire `length` pixels long of dots `spacing` apart that have moved
/// `offset` pixels.
fn dot_offsets(length: f64, offset: f64, spacing: f64) -> Vec<f64> {
    if spacing <= 0.0 || length <= 0.0 {
        return Vec::new();
    }
    let first = offset.rem_euclid(spacing);
    let count = ((length - first) / spacing).floor().clamp(-1.0, 4096.0) as i32 + 1;
    (0..count.max(0))
        .map(|i| first + f64::from(i) * spacing)
        .collect()
}

impl Schematic<'_> {
    /// Where `pin` is, in units of `ui.space`.
    pub fn pin(&self, pin: Pin<'_>) -> Option<(f64, f64)> {
        match pin {
            Pin::Terminal(i, name) => self.symbols.get(i)?.terminal(name),
            Pin::At(x, y) => Some((x, y)),
        }
    }

    /// The corners of `wire` from its start to its end, in units of `ui.space`, or `None`
    /// when one of its ends does not resolve.
    pub fn route(&self, wire: &Connection<'_>) -> Option<Vec<(f64, f64)>> {
        let (from, to) = (self.pin(wire.from)?, self.pin(wire.to)?);
        let mut points = vec![from];
        if wire.via.is_empty() {
            if (from.0 - to.0).abs() > SAME && (from.1 - to.1).abs() > SAME {
                points.push((to.0, from.1));
            }
        } else {
            points.extend(&wire.via);
        }
        points.push(to);
        Some(points)
    }

    /// The indices of the wires with an end that names a missing symbol or terminal.
    pub fn unresolved(&self) -> Vec<usize> {
        (0..self.wires.len())
            .filter(|&i| self.route(&self.wires[i]).is_none())
            .collect()
    }

    /// Seconds the current has flowed at `now`, 0 before it starts or without it.
    pub fn flowing(&self, now: f32) -> f32 {
        self.current_at.map_or(0.0, |at| (now - at).max(0.0))
    }

    /// The points where three or more conductors meet, in units of `ui.space`: the ends of
    /// wires, counting a terminal as one conductor and a wire that runs on through the
    /// point as two.
    pub fn junctions(&self) -> Vec<(f64, f64)> {
        let routes: Vec<Vec<(f64, f64)>> =
            self.wires.iter().filter_map(|w| self.route(w)).collect();
        let ends: Vec<((f64, f64), bool)> = self
            .wires
            .iter()
            .filter_map(|w| Some((w, self.route(w)?)))
            .flat_map(|(w, r)| {
                [
                    (r[0], matches!(w.from, Pin::Terminal(..))),
                    (r[r.len() - 1], matches!(w.to, Pin::Terminal(..))),
                ]
            })
            .collect();
        let mut found: Vec<(f64, f64)> = Vec::new();
        for &(p, _) in &ends {
            if found.iter().any(|&q| same(p, q)) {
                continue;
            }
            let here: Vec<bool> = ends
                .iter()
                .filter(|(q, _)| same(p, *q))
                .map(|&(_, t)| t)
                .collect();
            let through = routes
                .iter()
                .filter(|r| !same(p, r[0]) && !same(p, r[r.len() - 1]))
                .filter(|r| r.windows(2).any(|s| on_segment(p, s[0], s[1])))
                .count();
            let degree = here.len() + usize::from(here.contains(&true)) + 2 * through;
            if degree >= 3 {
                found.push(p);
            }
        }
        found
    }
}

impl<'a> Component<'a> for Schematic<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let now = ui.seconds();
        let flowing = self.flowing(now);
        let lit = progress(flowing, 0.0, 0.3);
        let mut shapes = Vec::new();
        let mut moving = Vec::new();
        for wire in &self.wires {
            let Some(route) = self.route(wire) else {
                continue;
            };
            let points: Vec<Point> = route.iter().map(|&(x, y)| space.to_px(x, y)).collect();
            shapes.push(Shape {
                path: polyline(&points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>()),
                fill: None,
                stroke: Some(self.wire_color),
                stroke_width: self.stroke_width,
                dash: None,
                opacity: 1.0,
            });
            if flowing > 0.0 && wire.current != 0.0 {
                let length: f64 = points.windows(2).map(|s| (s[1] - s[0]).hypot()).sum();
                let offset = f64::from(flowing * self.current_speed * wire.current);
                for s in dot_offsets(length, offset, f64::from(self.dot_spacing)) {
                    let at = along(&points, s);
                    moving.push(Shape {
                        path: ring((at.x, at.y), f64::from(DOT_RADIUS)),
                        fill: Some(self.dot_color),
                        stroke: None,
                        stroke_width: 0.0,
                        dash: None,
                        opacity: 1.0,
                    });
                }
            }
        }
        if self.junctions {
            for (x, y) in self.junctions() {
                let at = space.to_px(x, y);
                shapes.push(Shape {
                    path: ring((at.x, at.y), JUNCTION_RADIUS),
                    fill: Some(self.wire_color),
                    stroke: None,
                    stroke_width: 0.0,
                    dash: None,
                    opacity: 1.0,
                });
            }
        }
        let mut labels = Vec::new();
        for symbol in &self.symbols {
            let lit = if symbol.kind == SymbolKind::Bulb {
                symbol.lit.max(lit)
            } else {
                symbol.lit
            };
            let (parts, glow, at) = symbol.parts(space, now, lit);
            shapes.extend(parts);
            moving.extend(glow);
            if !symbol.label.is_empty() {
                labels.push((symbol.label, at, symbol.label_color));
            }
        }
        let drawn = draw_animated(ui, &shapes, self.enter.as_ref(), self.exit.as_ref(), None);
        let texts: Vec<Svgr<'a>> = labels
            .into_iter()
            .map(|(text, (at, anchor), color)| label(ui, text.to_owned(), at, anchor, color))
            .collect();
        let moving = draw_animated(ui, &moving, None, None, None);
        let (enter, exit) = (self.parts_enter.as_ref(), self.parts_exit.as_ref());
        // dots and glow under the wires, as in `Circuit`, the labels on top
        let moving = fade(ui, moving, enter, exit);
        let texts = fade(ui, texts.into(), enter, exit);
        fframes::svgr!(<g>{moving}{drawn}{texts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn terminals_turn_with_the_symbol() {
        let resistor = Symbol {
            at: (1.0, 2.0),
            rotation: 90.0,
            ..Symbol::default()
        };
        assert!(close(resistor.terminal("a").unwrap(), (1.0, 1.4)));
        assert!(close(resistor.terminal("b").unwrap(), (1.0, 2.6)));
        assert_eq!(resistor.terminal("+"), None);
        let ground = Symbol {
            kind: SymbolKind::Ground,
            at: (-2.0, -1.0),
            ..Symbol::default()
        };
        assert!(close(ground.terminal("a").unwrap(), (-2.0, -1.0)));
        let battery = Symbol {
            kind: SymbolKind::Battery,
            rotation: 90.0,
            ..Symbol::default()
        };
        // + on top when standing up
        assert!(battery.terminal("+").unwrap().1 > battery.terminal("-").unwrap().1);
    }

    #[test]
    fn an_opamp_has_its_inverting_input_on_top() {
        let opamp = Symbol {
            kind: SymbolKind::Opamp,
            at: (3.0, 0.0),
            size: 1.0,
            ..Symbol::default()
        };
        assert!(close(opamp.terminal("-").unwrap(), (2.0, 0.4)));
        assert!(close(opamp.terminal("+").unwrap(), (2.0, -0.4)));
        assert!(close(opamp.terminal("out").unwrap(), (4.0, 0.0)));
    }

    fn two_resistors() -> Schematic<'static> {
        Schematic {
            symbols: vec![
                Symbol::default(),
                Symbol {
                    at: (3.0, -2.0),
                    rotation: 90.0,
                    ..Symbol::default()
                },
            ],
            ..Schematic::default()
        }
    }

    #[test]
    fn a_wire_without_corners_turns_once_at_a_right_angle() {
        let schematic = two_resistors();
        let route = schematic
            .route(&Connection::new((0, "b"), (1, "b")))
            .unwrap();
        assert_eq!(route.len(), 3);
        assert!(close(route[0], (0.6, 0.0)));
        assert!(close(route[2], (3.0, -1.4)));
        for s in route.windows(2) {
            assert!((s[0].0 - s[1].0).abs() < 1e-9 || (s[0].1 - s[1].1).abs() < 1e-9);
        }
        // in line: straight
        let straight = schematic
            .route(&Connection::new((0, "b"), (3.0, 0.0)))
            .unwrap();
        assert_eq!(straight.len(), 2);
    }

    #[test]
    fn missing_terminals_are_reported_not_drawn() {
        let schematic = Schematic {
            wires: vec![
                Connection::new((0, "a"), (1, "a")),
                Connection::new((0, "out"), (1, "a")),
                Connection::new((0, "a"), (7, "a")),
            ],
            ..two_resistors()
        };
        assert_eq!(schematic.unresolved(), vec![1, 2]);
    }

    #[test]
    fn a_tee_gets_a_junction_and_a_corner_does_not() {
        let schematic = Schematic {
            wires: vec![
                // along the top, turning down at a corner
                Connection::new((0, "b"), (1, "b")).via([(3.0, 0.0)]),
                // a branch ending on the first wire
                Connection::new((1.5, 0.0), (1.5, -2.0)),
            ],
            ..two_resistors()
        };
        assert_eq!(schematic.junctions(), vec![(1.5, 0.0)]);
        // a terminal with two wires is a junction too
        let fork = Schematic {
            wires: vec![
                Connection::new((0, "b"), (2.0, 0.0)),
                Connection::new((0, "b"), (0.6, 1.0)),
            ],
            ..two_resistors()
        };
        assert_eq!(fork.junctions().len(), 1);
        assert!(close(fork.junctions()[0], (0.6, 0.0)));
    }

    #[test]
    fn dots_move_with_the_current() {
        let at = |offset: f64| dot_offsets(300.0, offset, 56.0);
        assert_eq!(at(0.0), vec![0.0, 56.0, 112.0, 168.0, 224.0, 280.0]);
        // forward by speed times time, backward with a negative current
        let (speed, dt) = (160.0, 0.1);
        assert!((at(speed * dt)[0] - 16.0).abs() < 1e-9);
        assert!((at(-speed * dt)[0] - 40.0).abs() < 1e-9);
        assert!(at(0.0).iter().all(|&s| s <= 300.0));
        let schematic = Schematic {
            current_at: Some(1.0),
            ..Schematic::default()
        };
        assert_eq!(schematic.flowing(0.5), 0.0);
        assert!((schematic.flowing(1.25) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn a_switch_closes_from_its_time() {
        let switch = Symbol {
            kind: SymbolKind::Switch {
                closes_at: Some(1.0),
            },
            ..Symbol::default()
        };
        assert!((switch.lever_angle(0.5) - OPEN_DEGREES.to_radians()).abs() < 1e-12);
        assert_eq!(switch.lever_angle(2.0), 0.0);
        let open = Symbol {
            kind: SymbolKind::Switch { closes_at: None },
            ..Symbol::default()
        };
        assert!(open.lever_angle(100.0) > 0.5);
    }
}
