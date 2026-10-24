//! Manim's `NumberLine`, `Axes` and `NumberPlane`, placed in the units of `ui.space`.

use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{Emphasis, EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    label::Label,
    layout::Anchor,
    shapes::{DashedLine, straight},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{Area, Curve, Func, FunctionGraph, ParametricFunction, RiemannRectangles, Space};

/// A data range and its tick step, Manim's `[min, max, step]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl Range {
    pub const fn new(min: f64, max: f64, step: f64) -> Self {
        Self { min, max, step }
    }

    /// `max - min`.
    pub fn span(self) -> f64 {
        self.max - self.min
    }

    /// The ticks from `min` in steps of `step` up to `max`; none for a step that is not
    /// positive, at most 10 000.
    pub fn ticks(self) -> Vec<f64> {
        if !(self.step > 0.0 && self.span().is_finite() && self.span() >= 0.0) {
            return Vec::new();
        }
        let n = ((self.span() / self.step + 1e-9).floor() as usize).min(9_999);
        (0..=n).map(|i| self.min + self.step * i as f64).collect()
    }

    /// Where `v` is between `min` (0) and `max` (1).
    fn fraction(self, v: f64) -> f64 {
        if self.span() == 0.0 {
            0.5
        } else {
            (v - self.min) / self.span()
        }
    }

    /// `v` clamped into the range, for where the other axis crosses.
    fn clamp(self, v: f64) -> f64 {
        v.clamp(self.min.min(self.max), self.min.max(self.max))
    }
}

/// The mapping of axes from data to pixels: their ranges, their size in units, the unit
/// point of their center and the space they are drawn in. It is `Copy`, so graphs, lines and
/// labels built from the same axes each keep one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxesMap {
    pub x: Range,
    pub y: Range,
    /// Length of the x-axis in units of `space`.
    pub x_length: f64,
    pub y_length: f64,
    /// The middle of the axes' box, in units of `space`.
    pub center: (f64, f64),
    pub space: Space,
}

impl Default for AxesMap {
    fn default() -> Self {
        Axes::default().map_in(Space::FULL_HD)
    }
}

impl AxesMap {
    /// The point in units of `space` of the data point `(x, y)`.
    pub fn to_space(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.center.0 + (self.x.fraction(x) - 0.5) * self.x_length,
            self.center.1 + (self.y.fraction(y) - 0.5) * self.y_length,
        )
    }

    /// The data point at a point in units of `space`.
    pub fn from_space(&self, sx: f64, sy: f64) -> (f64, f64) {
        let back = |s: f64, center: f64, length: f64, r: Range| {
            if length == 0.0 {
                r.min
            } else {
                r.min + ((s - center) / length + 0.5) * r.span()
            }
        };
        (
            back(sx, self.center.0, self.x_length, self.x),
            back(sy, self.center.1, self.y_length, self.y),
        )
    }

    /// The pixel of the data point `(x, y)` (Manim `coords_to_point`, `c2p`).
    pub fn c2p(&self, x: f64, y: f64) -> Point {
        let (sx, sy) = self.to_space(x, y);
        self.space.to_px(sx, sy)
    }

    /// The data point at a pixel (Manim `point_to_coords`, `p2c`).
    pub fn p2c(&self, p: Point) -> (f64, f64) {
        let (sx, sy) = self.space.from_px(p);
        self.from_space(sx, sy)
    }

    /// The data `y` the x-axis is drawn at: 0, or the nearest end of the y range.
    pub fn x_axis_y(&self) -> f64 {
        self.y.clamp(0.0)
    }

    /// The data `x` the y-axis is drawn at: 0, or the nearest end of the x range.
    pub fn y_axis_x(&self) -> f64 {
        self.x.clamp(0.0)
    }
}

/// How an axis line, its tips and its ticks look.
#[derive(Debug, Clone, Copy)]
struct AxisLook {
    color: Color,
    stroke_width: f32,
    tips: bool,
    tip_length: f32,
    tip_width: f32,
    tick_size: f32,
}

impl AxisLook {
    fn line(&self, path: &mut BezPath, from: Point, to: Point) {
        if self.tips {
            path.extend(straight(
                from,
                to,
                f64::from(self.tip_length),
                f64::from(self.tip_width),
                false,
            ));
        } else {
            path.move_to(from);
            path.line_to(to);
        }
    }

    /// Ticks across the axis through `points`; `across` is the unit direction of a tick.
    fn ticks(&self, path: &mut BezPath, points: impl IntoIterator<Item = Point>, across: Vec2) {
        let half = across * f64::from(self.tick_size);
        for p in points {
            path.move_to(p - half);
            path.line_to(p + half);
        }
    }

    /// The lines (filled, for the tips) and the ticks (outlined).
    fn shapes(&self, lines: BezPath, ticks: BezPath) -> [Shape; 2] {
        [
            Shape {
                path: lines,
                fill: Some(self.color),
                stroke: Some(self.color),
                stroke_width: self.stroke_width,
                dash: None,
                opacity: 1.0,
            },
            Shape {
                path: ticks,
                fill: None,
                stroke: Some(self.color),
                stroke_width: self.stroke_width,
                dash: None,
                opacity: 1.0,
            },
        ]
    }
}

/// `v` with `decimals` decimals, without a sign on zero.
fn number(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    if s.starts_with('-') && s[1..].chars().all(|c| c == '0' || c == '.') {
        s[1..].to_owned()
    } else {
        s
    }
}

/// A horizontal number line (Manim `NumberLine`): `range` spread over `length` units
/// centered on `center`, with ticks at every step and the `numbers` written below them.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct NumberLine {
    pub range: Range,
    /// Length in units of `ui.space`.
    pub length: f64,
    /// The middle of the line, in units of `ui.space`.
    pub center: (f64, f64),
    pub color: Color,
    pub stroke_width: f32,
    /// Arrow tip at the right end.
    pub tips: bool,
    pub tip_length: f32,
    pub tip_width: f32,
    pub include_ticks: bool,
    /// Half the length of a tick, in pixels.
    pub tick_size: f32,
    /// Values written below their ticks (Manim `numbers_to_include`).
    pub numbers: Vec<f64>,
    pub decimals: usize,
    pub number_style: TextStyle,
    pub number_fill: Color,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub emphasis: Option<Emphasis>,
    /// How the numbers appear and go; the line itself uses `enter` and `exit`.
    pub numbers_enter: Option<FadeIn>,
    pub numbers_exit: Option<FadeOut>,
}

impl Themed for NumberLine {
    fn themed(t: &Theme) -> Self {
        Self {
            range: Range::new(-8.0, 8.0, 1.0),
            length: 12.0,
            center: (0.0, 0.0),
            color: t.colors.text,
            stroke_width: t.stroke.thin,
            tips: false,
            tip_length: 24.0,
            tip_width: 20.0,
            include_ticks: true,
            tick_size: 10.0,
            numbers: Vec::new(),
            decimals: 0,
            number_style: t.typography.caption,
            number_fill: t.colors.text,
            enter: None,
            exit: None,
            emphasis: None,
            numbers_enter: None,
            numbers_exit: None,
        }
    }
}

impl Default for NumberLine {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl NumberLine {
    fn look(&self) -> AxisLook {
        AxisLook {
            color: self.color,
            stroke_width: self.stroke_width,
            tips: self.tips,
            tip_length: self.tip_length,
            tip_width: self.tip_width,
            tick_size: self.tick_size,
        }
    }

    /// The pixel of the number `x` in `space` (Manim `number_to_point`, `n2p`).
    pub fn n2p_in(&self, space: Space, x: f64) -> Point {
        let sx = self.center.0 + (self.range.fraction(x) - 0.5) * self.length;
        space.to_px(sx, self.center.1)
    }

    /// The pixel of the number `x` in `ui.space`.
    pub fn n2p(&self, ui: &Ui<'_, '_, '_>, x: f64) -> (f32, f32) {
        let p = self.n2p_in(ui.space, x);
        (p.x as f32, p.y as f32)
    }

    /// The number at the pixel column `px` (Manim `point_to_number`, `p2n`).
    pub fn p2n(&self, ui: &Ui<'_, '_, '_>, px: f32) -> f64 {
        let (sx, _) = ui.space.from_px(Point::new(f64::from(px), 0.0));
        if self.length == 0.0 {
            return self.range.min;
        }
        self.range.min + ((sx - self.center.0) / self.length + 0.5) * self.range.span()
    }
}

impl<'a> Component<'a> for NumberLine {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let look = self.look();
        let (start, end) = (
            self.n2p_in(ui.space, self.range.min),
            self.n2p_in(ui.space, self.range.max),
        );
        let mut lines = BezPath::new();
        look.line(&mut lines, start, end);
        let mut ticks = BezPath::new();
        if self.include_ticks {
            let points = self
                .range
                .ticks()
                .into_iter()
                .map(|t| self.n2p_in(ui.space, t));
            look.ticks(&mut ticks, points, Vec2::new(0.0, 1.0));
        }
        let drawn = draw_animated(
            ui,
            &look.shapes(lines, ticks),
            self.enter.as_ref(),
            self.exit.as_ref(),
            self.emphasis.as_ref(),
        );
        let size = self.number_style.size as f32;
        let numbers: Svgr<'a> = self
            .numbers
            .iter()
            .map(|&n| {
                let p = self.n2p_in(ui.space, n);
                line(
                    p.x as f32,
                    p.y as f32 + self.tick_size + size * 1.1,
                    self.number_style,
                    self.number_fill,
                    TextAnchor::Middle,
                    0.0,
                    1.0,
                    Cow::Owned(number(n, self.decimals)),
                )
            })
            .collect();
        let numbers = fade(
            ui,
            numbers,
            self.numbers_enter.as_ref(),
            self.numbers_exit.as_ref(),
        );
        fframes::svgr!(<g>{drawn}{numbers}</g>)
    }
}

/// Two number lines that cross where `x = 0` and `y = 0` (Manim `Axes`), or at the ends of
/// the ranges that do not contain 0. Build the graphs and lines that go on them with
/// [`Axes::plot`], [`Axes::vertical_line`], [`Axes::riemann_rectangles`] and
/// [`Axes::area`]; they keep the axes' mapping.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Axes<'a> {
    pub x_range: Range,
    pub y_range: Range,
    /// Length of the x-axis in units of `ui.space`.
    pub x_length: f64,
    pub y_length: f64,
    /// The middle of the axes' box, in units of `ui.space`.
    pub center: (f64, f64),
    pub color: Color,
    pub stroke_width: f32,
    /// Arrow tips at the positive ends.
    pub tips: bool,
    pub tip_length: f32,
    pub tip_width: f32,
    pub include_ticks: bool,
    /// Half the length of a tick, in pixels.
    pub tick_size: f32,
    /// Values written below the x-axis (Manim `numbers_to_include`). A number where the
    /// y-axis crosses is left out, as in Manim.
    pub x_numbers: Vec<f64>,
    /// Values written left of the y-axis.
    pub y_numbers: Vec<f64>,
    pub decimals: usize,
    pub number_style: TextStyle,
    pub number_fill: Color,
    /// Written up and right of the end of the x-axis (Manim `get_axis_labels`).
    pub x_label: Option<Cow<'a, str>>,
    pub y_label: Option<Cow<'a, str>>,
    pub label_style: TextStyle,
    pub label_fill: Color,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub emphasis: Option<Emphasis>,
    /// How the numbers and labels appear and go; the lines use `enter` and `exit`.
    pub labels_enter: Option<FadeIn>,
    pub labels_exit: Option<FadeOut>,
}

impl Themed for Axes<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            x_range: Range::new(-6.0, 6.0, 1.0),
            y_range: Range::new(-3.0, 3.0, 1.0),
            x_length: 12.0,
            y_length: 6.0,
            center: (0.0, 0.0),
            color: t.colors.text,
            stroke_width: t.stroke.thin,
            tips: true,
            tip_length: 24.0,
            tip_width: 20.0,
            include_ticks: true,
            tick_size: 10.0,
            x_numbers: Vec::new(),
            y_numbers: Vec::new(),
            decimals: 0,
            number_style: t.typography.caption,
            number_fill: t.colors.text,
            x_label: None,
            y_label: None,
            label_style: t.typography.body,
            label_fill: t.colors.title,
            enter: None,
            exit: None,
            emphasis: None,
            labels_enter: None,
            labels_exit: None,
        }
    }
}

impl Default for Axes<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Axes<'_> {
    /// The mapping in `space`.
    pub fn map_in(&self, space: Space) -> AxesMap {
        AxesMap {
            x: self.x_range,
            y: self.y_range,
            x_length: self.x_length,
            y_length: self.y_length,
            center: self.center,
            space,
        }
    }

    /// The mapping in `ui.space`.
    pub fn map(&self, ui: &Ui<'_, '_, '_>) -> AxesMap {
        self.map_in(ui.space)
    }

    /// The pixel of the data point `(x, y)` (Manim `c2p`).
    pub fn c2p(&self, ui: &Ui<'_, '_, '_>, x: f64, y: f64) -> (f32, f32) {
        let p = self.map(ui).c2p(x, y);
        (p.x as f32, p.y as f32)
    }

    /// The data point at a pixel (Manim `p2c`).
    pub fn p2c(&self, ui: &Ui<'_, '_, '_>, x: f32, y: f32) -> (f64, f64) {
        self.map(ui).p2c(Point::new(f64::from(x), f64::from(y)))
    }

    /// The graph of `f` over the whole x range (Manim `axes.plot(f)`); change `x_range`,
    /// the paint or `enter` with struct update syntax.
    pub fn plot(&self, ui: &Ui<'_, '_, '_>, f: Func) -> FunctionGraph {
        FunctionGraph {
            f,
            x_range: (self.x_range.min, self.x_range.max),
            axes: self.map(ui),
            ..ui.defaults()
        }
    }

    /// The curve `t -> f(t)` in the axes' data units (Manim `plot_parametric_curve`).
    pub fn plot_parametric(
        &self,
        ui: &Ui<'_, '_, '_>,
        f: Curve,
        t_range: (f64, f64),
    ) -> ParametricFunction {
        ParametricFunction {
            axes: Some(self.map(ui)),
            ..ParametricFunction::new(ui, f, t_range)
        }
    }
    /// A dashed line from the data point `(x, y)` straight down (or up) to the x-axis, in
    /// the highlight colour (Manim `get_vertical_line(c2p(x, y))`).
    pub fn vertical_line(&self, ui: &Ui<'_, '_, '_>, x: f64, y: f64) -> DashedLine {
        let map = self.map(ui);
        let (top, base) = (map.c2p(x, y), map.c2p(x, map.x_axis_y()));
        DashedLine {
            x1: top.x as f32,
            y1: top.y as f32,
            x2: base.x as f32,
            y2: base.y as f32,
            stroke: Some(ui.theme.colors.highlight),
            ..ui.defaults()
        }
    }

    /// A label next to the point of `graph` above `x`, in the graph's colour (Manim
    /// `get_graph_label(graph, label, x_val, direction)`).
    pub fn graph_label<'t>(
        &self,
        ui: &Ui<'_, '_, '_>,
        graph: &FunctionGraph,
        text: impl Into<Cow<'t, str>>,
        x: f64,
        direction: Anchor,
    ) -> Label<'t> {
        let (px, py) = graph.point(x);
        Label {
            text: text.into(),
            x: px,
            y: py,
            direction,
            fill: graph.stroke.unwrap_or(ui.theme.colors.title),
            ..ui.defaults()
        }
    }

    /// Rectangles of width `dx` under `f` from `x_range.0` to `x_range.1` (Manim
    /// `get_riemann_rectangles`).
    pub fn riemann_rectangles(
        &self,
        ui: &Ui<'_, '_, '_>,
        f: Func,
        x_range: (f64, f64),
        dx: f64,
    ) -> RiemannRectangles {
        RiemannRectangles {
            f,
            x_range,
            dx,
            axes: self.map(ui),
            opacity: 0.6,
            ..ui.defaults()
        }
    }

    /// The region between `f` and the x-axis from `x_range.0` to `x_range.1`; set `bounded`
    /// for the region between two graphs (Manim `get_area`).
    pub fn area(&self, ui: &Ui<'_, '_, '_>, f: Func, x_range: (f64, f64)) -> Area {
        Area {
            f,
            x_range,
            axes: self.map(ui),
            opacity: 0.5,
            ..ui.defaults()
        }
    }

    fn look(&self) -> AxisLook {
        AxisLook {
            color: self.color,
            stroke_width: self.stroke_width,
            tips: self.tips,
            tip_length: self.tip_length,
            tip_width: self.tip_width,
            tick_size: self.tick_size,
        }
    }

    /// The axis lines and ticks.
    fn shapes(&self, map: &AxesMap) -> [Shape; 2] {
        let look = self.look();
        let (ax, ay) = (map.y_axis_x(), map.x_axis_y());
        let mut lines = BezPath::new();
        look.line(
            &mut lines,
            map.c2p(self.x_range.min, ay),
            map.c2p(self.x_range.max, ay),
        );
        look.line(
            &mut lines,
            map.c2p(ax, self.y_range.min),
            map.c2p(ax, self.y_range.max),
        );
        let mut ticks = BezPath::new();
        if self.include_ticks {
            let xs = self.x_range.ticks().into_iter().map(|x| map.c2p(x, ay));
            look.ticks(&mut ticks, xs, Vec2::new(0.0, 1.0));
            let ys = self.y_range.ticks().into_iter().map(|y| map.c2p(ax, y));
            look.ticks(&mut ticks, ys, Vec2::new(1.0, 0.0));
        }
        look.shapes(lines, ticks)
    }
}

impl<'a> Component<'a> for Axes<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let map = self.map(ui);
        let drawn = draw_animated(
            ui,
            &self.shapes(&map),
            self.enter.as_ref(),
            self.exit.as_ref(),
            self.emphasis.as_ref(),
        );
        let (ax, ay) = (map.y_axis_x(), map.x_axis_y());
        let size = self.number_style.size as f32;
        let gap = self.tick_size + size * 0.4;
        let x_numbers = self.x_numbers.iter().filter(|&&x| x != ax).map(|&x| {
            let p = map.c2p(x, ay);
            line(
                p.x as f32,
                p.y as f32 + gap + size * 0.72,
                self.number_style,
                self.number_fill,
                TextAnchor::Middle,
                0.0,
                1.0,
                Cow::Owned(number(x, self.decimals)),
            )
        });
        let y_numbers = self.y_numbers.iter().filter(|&&y| y != ay).map(|&y| {
            let p = map.c2p(ax, y);
            line(
                p.x as f32 - gap,
                p.y as f32 + size * 0.36,
                self.number_style,
                self.number_fill,
                TextAnchor::End,
                0.0,
                1.0,
                Cow::Owned(number(y, self.decimals)),
            )
        });
        let mut texts: Vec<Svgr<'a>> = x_numbers.chain(y_numbers).collect();
        let ends = [
            (self.x_label, map.c2p(self.x_range.max, ay)),
            (self.y_label, map.c2p(ax, self.y_range.max)),
        ];
        for (label, end) in ends {
            if let Some(text) = label {
                texts.push(ui.show(Label {
                    text,
                    x: end.x as f32,
                    y: end.y as f32,
                    direction: Anchor::TopRight,
                    style: self.label_style,
                    fill: self.label_fill,
                    ..ui.defaults()
                }));
            }
        }
        let texts = fade(
            ui,
            texts.into_iter().collect(),
            self.labels_enter.as_ref(),
            self.labels_exit.as_ref(),
        );
        fframes::svgr!(<g>{drawn}{texts}</g>)
    }
}

/// Axes over a grid of lines at every tick (Manim `NumberPlane`), filling Manim's whole
/// frame by default. `faded_line_ratio` 2 adds a fainter line between two grid lines, 3 two
/// of them.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct NumberPlane<'a> {
    pub axes: Axes<'a>,
    pub line_color: Color,
    pub line_width: f32,
    pub line_opacity: f32,
    pub faded_line_ratio: usize,
    pub faded_opacity: f32,
    /// How the grid appears and goes; the axes use their own `enter` and `exit`.
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
}

impl Themed for NumberPlane<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            axes: Axes {
                x_range: Range::new(-7.0, 7.0, 1.0),
                y_range: Range::new(-4.0, 4.0, 1.0),
                x_length: 14.0,
                y_length: 8.0,
                tips: false,
                include_ticks: false,
                ..Axes::themed(t)
            },
            line_color: t.colors.accent,
            line_width: t.stroke.thin,
            line_opacity: 0.45,
            faded_line_ratio: 1,
            faded_opacity: 0.18,
            enter: None,
            exit: None,
        }
    }
}

impl Default for NumberPlane<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl NumberPlane<'_> {
    /// The grid lines at every tick and the fainter lines between them.
    fn grid(&self, map: &AxesMap) -> [Shape; 2] {
        let (x, y) = (self.axes.x_range, self.axes.y_range);
        let ratio = self.faded_line_ratio.max(1);
        let mut main = BezPath::new();
        let mut faded = BezPath::new();
        let mut lines = |range: Range, vertical: bool| {
            let ticks = range.ticks();
            for (i, &t) in ticks.iter().enumerate() {
                let subs = if i + 1 < ticks.len() { ratio } else { 1 };
                for k in 0..subs {
                    let v = t + range.step * k as f64 / ratio as f64;
                    let (a, b) = if vertical {
                        (map.c2p(v, y.min), map.c2p(v, y.max))
                    } else {
                        (map.c2p(x.min, v), map.c2p(x.max, v))
                    };
                    let path = if k == 0 { &mut main } else { &mut faded };
                    path.move_to(a);
                    path.line_to(b);
                }
            }
        };
        lines(x, true);
        lines(y, false);
        let grid = |path: BezPath, opacity: f32| Shape {
            path,
            fill: None,
            stroke: Some(self.line_color),
            stroke_width: self.line_width,
            dash: None,
            opacity,
        };
        [
            grid(faded, self.faded_opacity),
            grid(main, self.line_opacity),
        ]
    }
}

impl<'a> Component<'a> for NumberPlane<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let map = self.axes.map(ui);
        let grid = draw_animated(
            ui,
            &self.grid(&map),
            self.enter.as_ref(),
            self.exit.as_ref(),
            None,
        );
        let axes = ui.show(self.axes);
        fframes::svgr!(<g>{grid}{axes}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_tick_from_min_to_max() {
        assert_eq!(Range::new(0.0, 3.0, 1.0).ticks(), vec![0.0, 1.0, 2.0, 3.0]);
        assert_eq!(Range::new(-10.0, 10.3, 2.0).ticks().len(), 11);
        assert_eq!(Range::new(0.0, 1.0, 0.1).ticks().len(), 11);
        assert_eq!(Range::new(0.0, 1.0, 0.0).ticks(), Vec::<f64>::new());
        assert_eq!(Range::new(1.0, 0.0, 1.0).ticks(), Vec::<f64>::new());
    }

    #[test]
    fn c2p_and_p2c_round_trip() {
        let axes = Axes {
            x_range: Range::new(-10.0, 10.3, 1.0),
            y_range: Range::new(-1.5, 1.5, 1.0),
            x_length: 10.0,
            center: (0.5, -0.25),
            ..Axes::default()
        };
        let map = axes.map_in(Space::FULL_HD);
        for (x, y) in [(0.0, 0.0), (-10.0, -1.5), (10.3, 1.5), (3.7, -0.2)] {
            let (bx, by) = map.p2c(map.c2p(x, y));
            assert!((bx - x).abs() < 1e-9 && (by - y).abs() < 1e-9, "{x} {y}");
        }
        // the box's corners are half its size from its center, in units
        let low = map.to_space(-10.0, -1.5);
        assert!((low.0 - (0.5 - 5.0)).abs() < 1e-9 && (low.1 - (-0.25 - 3.0)).abs() < 1e-9);
        // y up: a larger y is higher on screen
        assert!(map.c2p(0.0, 1.0).y < map.c2p(0.0, 0.0).y);
        // axes cross at 0, or at the end of a range without 0
        assert_eq!((map.y_axis_x(), map.x_axis_y()), (0.0, 0.0));
        let positive = Axes {
            x_range: Range::new(1.0, 5.0, 1.0),
            y_range: Range::new(2.0, 6.0, 1.0),
            ..Axes::default()
        }
        .map_in(Space::FULL_HD);
        assert_eq!((positive.y_axis_x(), positive.x_axis_y()), (1.0, 2.0));
    }

    #[test]
    fn number_line_maps_numbers_to_pixels() {
        let nl = NumberLine {
            range: Range::new(0.0, 10.0, 1.0),
            length: 10.0,
            center: (0.0, 1.0),
            ..NumberLine::default()
        };
        let p = nl.n2p_in(Space::FULL_HD, 5.0);
        assert_eq!(p, Point::new(960.0, 405.0));
        let p = nl.n2p_in(Space::FULL_HD, 0.0);
        assert_eq!(p.x, 960.0 - 5.0 * 135.0);
    }

    #[test]
    fn numbers_have_no_negative_zero() {
        assert_eq!(number(-0.0, 0), "0");
        assert_eq!(number(-0.04, 1), "0.0");
        assert_eq!(number(-2.0, 0), "-2");
        assert_eq!(number(1.23456, 2), "1.23");
    }

    #[test]
    fn plane_grid_has_a_line_per_tick_and_faded_lines_between() {
        let plane = NumberPlane {
            faded_line_ratio: 2,
            ..NumberPlane::default()
        };
        let [faded, main] = plane.grid(&plane.axes.map_in(Space::FULL_HD));
        let count = |s: &Shape| {
            s.path
                .elements()
                .iter()
                .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
                .count()
        };
        assert_eq!(count(&main), 15 + 9);
        assert_eq!(count(&faded), 14 + 8);
    }
}
