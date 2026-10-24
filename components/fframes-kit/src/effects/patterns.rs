use std::fmt::Write;

use fframes::{Color, Svgr};

use super::{Region, alpha_masked, filtered};
use crate::{
    anim::Anim,
    motion::clamp01,
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// Remotion's default colours of [`Checkerboard`], [`Lines`] and [`Rings`].
const PALE_BLUES: [Color; 2] = [Color::hex("#dff4ff"), Color::hex("#7cc6ff")];

/// The most rings [`Rings`] draws.
pub const MAX_RINGS: usize = 1024;

/// The most stripes, cells or dots one pattern draws. A finer pattern is drawn coarser (every
/// size scaled up together) to stay below it.
pub const MAX_PATTERN_SHAPES: usize = 20_000;

/// `layer` over `content`, inside the area and only where `shape` is opaque when there is
/// one.
fn draw_over<'a>(
    content: Svgr<'a>,
    layer: Svgr<'a>,
    shape: Option<Svgr<'a>>,
    id: &str,
    area: (f32, f32),
) -> Svgr<'a> {
    let layer = if let Some(shape) = shape {
        alpha_masked(layer, shape, &format!("{id}-alpha"), area)
    } else {
        let clip = format!("{id}-area");
        let url = format!("url(#{clip})");
        let (w, h) = area;
        fframes::svgr!(
            <g>
                <defs>
                    <clipPath id={clip}>
                        <rect x="0" y="0" width={w} height={h} />
                    </clipPath>
                </defs>
                <g clip-path={url}>{layer}</g>
            </g>
        )
    };
    fframes::svgr!(<g>{content}{layer}</g>)
}

/// The patterns are drawn as paths rather than SVG `<pattern>` tiles: the backends round a
/// small tile to whole pixels differently, so the repeats drift apart across the picture.
///
/// The steps `k` for which `start + k * pitch` reaches from `lo - pitch` to `hi`.
fn steps(lo: f32, hi: f32, start: f32, pitch: f32) -> std::ops::RangeInclusive<i64> {
    let first = ((lo - start) / pitch).floor() as i64 - 1;
    let last = ((hi - start) / pitch).ceil() as i64;
    first..=last
}

/// How much to scale a pattern whose `cells` repeat in `dims` directions so it stays below
/// [`MAX_PATTERN_SHAPES`].
fn coarsen(cells: f32, dims: i32) -> f32 {
    let limit = MAX_PATTERN_SHAPES as f32;
    if cells <= limit {
        1.0
    } else {
        (cells / limit).powf(1.0 / dims as f32)
    }
}

/// Rectangles `(x, y, w, h)` as one path.
fn rects(parts: impl IntoIterator<Item = (f32, f32, f32, f32)>) -> String {
    let mut d = String::new();
    for (x, y, w, h) in parts {
        let _ = write!(d, "M{x} {y}h{w}v{h}h{}Z", -w);
    }
    d
}

/// The diagonal of the area: how far a pattern turned about its middle must reach.
fn reach((w, h): (f32, f32)) -> f32 {
    w.hypot(h)
}

/// The square, centred on the area, that still covers it when turned about its middle: the
/// middle and half the side, `pitch` past the corners.
fn cover(area: (f32, f32), pitch: f32) -> (f32, f32, f32) {
    (area.0 / 2.0, area.1 / 2.0, reach(area) / 2.0 + pitch)
}

/// `layer` turned by `angle` degrees about the middle of the area.
fn turned(layer: Svgr<'_>, angle: f32, (w, h): (f32, f32)) -> Svgr<'_> {
    if angle == 0.0 {
        return layer;
    }
    let turn = format!("rotate({angle} {} {})", w / 2.0, h / 2.0);
    fframes::svgr!(<g transform={turn}>{layer}</g>)
}

/// One path per colour of `colors`, from the path data `d[i]` of each.
fn paths<'a>(colors: &[Color], d: Vec<String>) -> Svgr<'a> {
    let paths: Vec<Svgr<'a>> = colors
        .iter()
        .zip(d)
        .filter(|(_, d)| !d.is_empty())
        .map(|(&fill, d)| fframes::svgr!(<path d={d} fill={fill} />))
        .collect();
    Svgr::from(paths)
}

/// `content` shown only where the luminance of `mask` (drawn over the `w` by `h` area from
/// the origin) is white.
fn masked<'a>(content: Svgr<'a>, mask: Svgr<'a>, id: &str, (w, h): (f32, f32)) -> Svgr<'a> {
    let url = format!("url(#{id})");
    let id = id.to_owned();
    fframes::svgr!(
        <g>
            <defs>
                <mask id={id} maskUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}>
                    {mask}
                </mask>
            </defs>
            <g mask={url}>{content}</g>
        </g>
    )
}

/// The colour of stripe, cell or ring `k` of a cycle through `colors`.
fn cycle(colors: &[Color], k: i64) -> Color {
    colors[k.rem_euclid(colors.len() as i64) as usize]
}

/// The children brightened by adding `intercept` to every channel after multiplying it by
/// `slope`, the alpha unchanged.
fn shifted(content: Svgr<'_>, id: String, slope: f32, intercept: f32) -> Svgr<'_> {
    let primitives = fframes::svgr!(
        <feComponentTransfer>
            <feFuncR type="linear" slope={slope} intercept={intercept} />
            <feFuncG type="linear" slope={slope} intercept={intercept} />
            <feFuncB type="linear" slope={slope} intercept={intercept} />
        </feComponentTransfer>
    );
    filtered(content, id, Region::Pad(0.02), primitives)
}

/// Horizontal CRT lines over its children (Remotion's `scanlines()`). Each line `thickness`
/// units tall, one every `spacing` units, lights the picture and the gaps darken it, so
/// the average brightness stays: `amount` is the strength, 0 unchanged. The light is added
/// to every channel, or with `premultiply` scales the colours, which follows the
/// picture's brightness more closely. `offset` scrolls the lines (animate it for a rolling
/// screen). Keep the lines at least two output pixels apart, finer lines alias. The children
/// render twice, their alpha stays.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Scanlines<'a> {
    pub amount: Anim<'a, f32>,
    pub spacing: f32,
    pub thickness: f32,
    pub offset: f32,
    pub premultiply: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Scanlines<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            amount: Anim::Fixed(0.15),
            spacing: 4.0,
            thickness: 1.0,
            offset: 0.0,
            premultiply: false,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Scanlines<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let amount = clamp01(ui.sample(&self.amount));
        let spacing = self.spacing.max(0.001);
        let thickness = self.thickness.min(spacing);
        if amount <= 0.0 || thickness <= 0.0 {
            return (self.children)(ui);
        }
        let id = ui.id(self.key, 0, "scanlines");
        let duty = thickness / spacing;
        let (up, down) = (amount * (1.0 - duty), amount * duty);
        let ((dim_slope, dim_shift), (lit_slope, lit_shift)) = if self.premultiply {
            ((1.0 - down, 0.0), (1.0 + up, 0.0))
        } else {
            ((1.0, -down), (1.0, up))
        };
        let dim = shifted(
            (self.children)(ui),
            format!("{id}-dim"),
            dim_slope,
            dim_shift,
        );
        let lit = shifted(
            (self.children)(ui),
            format!("{id}-lit"),
            lit_slope,
            lit_shift,
        );
        // a line's middle sits at -offset, and every spacing units from there
        let (w, h) = self.area;
        let k = coarsen(h / spacing, 1);
        let (spacing, thickness) = (spacing * k, thickness * k);
        let start = -self.offset - thickness / 2.0;
        let d = rects(
            steps(0.0, h, start, spacing).map(|i| (0.0, start + i as f32 * spacing, w, thickness)),
        );
        let lines = fframes::svgr!(<path d={d} fill="#ffffff" />);
        let lit = masked(lit, lines, &format!("{id}-lines"), self.area);
        fframes::svgr!(<g>{dim}{lit}</g>)
    }
}

/// Its children seen through a grid of round holes (Remotion's `dotGrid()`): dots
/// `dot_size` units across, `grid_size` units apart, the first centred half a cell from the
/// origin. With `invert` the dots are cut out of the children instead.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct DotGrid<'a> {
    pub dot_size: f32,
    pub grid_size: f32,
    pub invert: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for DotGrid<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            dot_size: 16.0,
            grid_size: 20.0,
            invert: false,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for DotGrid<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let content = (self.children)(ui);
        let id = ui.id(self.key, 0, "dot-grid");
        let grid = self.grid_size.max(0.001);
        let (w, h) = self.area;
        let (hole, around) = if self.invert {
            ("#000000", "#ffffff")
        } else {
            ("#ffffff", "#000000")
        };
        let k = coarsen(w * h / (grid * grid), 2);
        let (grid, r) = (grid * k, self.dot_size.max(0.0) / 2.0 * k);
        let mid = grid / 2.0;
        let mut d = String::new();
        for j in steps(0.0, h, mid, grid) {
            for i in steps(0.0, w, mid, grid) {
                let (x, y) = (mid + i as f32 * grid, mid + j as f32 * grid);
                let _ = write!(
                    d,
                    "M{} {y}a{r} {r} 0 1 0 {} 0a{r} {r} 0 1 0 {} 0Z",
                    x - r,
                    2.0 * r,
                    -2.0 * r
                );
            }
        }
        let dots = fframes::svgr!(<path d={d} fill={hole} />);
        let mask = fframes::svgr!(
            <g>
                <rect x="0" y="0" width={w} height={h} fill={around} />
                {dots}
            </g>
        );
        masked(content, mask, &format!("{id}-mask"), self.area)
    }
}

/// A checkerboard over its children (Remotion's `checkerboard()`): square cells
/// `cell_size` units wide in `colors` taken in turn along each row and column, `gap` units of
/// the children showing between them. `angle` turns the board about the middle of the area
/// in degrees and `offset_x`, `offset_y` move it; with `mask_to_source_alpha` it covers
/// only the opaque parts of the children (which then render twice).
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Checkerboard<'a> {
    pub colors: Vec<Color>,
    pub cell_size: f32,
    pub gap: f32,
    pub angle: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub mask_to_source_alpha: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Checkerboard<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            colors: PALE_BLUES.to_vec(),
            cell_size: 80.0,
            gap: 0.0,
            angle: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            mask_to_source_alpha: false,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Checkerboard<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let content = (self.children)(ui);
        if self.colors.is_empty() {
            return content;
        }
        let shape = self.mask_to_source_alpha.then(|| (self.children)(ui));
        let id = ui.id(self.key, 0, "checkerboard");
        let n = self.colors.len();
        let cell = self.cell_size.max(0.001);
        let k = coarsen((reach(self.area) / cell).powi(2), 2);
        let (cell, gap) = (cell * k, self.gap * k);
        let (cx, cy, half) = cover(self.area, cell);
        let size = (cell - gap).max(0.0);
        let inset = (cell - size) / 2.0;
        let mut d = vec![String::new(); n];
        for j in steps(cy - half, cy + half, self.offset_y, cell) {
            for i in steps(cx - half, cx + half, self.offset_x, cell) {
                let x = self.offset_x + i as f32 * cell + inset;
                let y = self.offset_y + j as f32 * cell + inset;
                d[(i + j).rem_euclid(n as i64) as usize] += &rects([(x, y, size, size)]);
            }
        }
        let board = turned(paths(&self.colors, d), self.angle, self.area);
        draw_over(content, board, shape, &id, self.area)
    }
}

/// A square grid of lines over its children (Remotion's `gridlines()`): lines
/// `line_width` units wide in `line_color`, `grid_size` units apart, over a
/// `background_color` (transparent by default). `rotation` turns the grid about the middle
/// of the area in degrees and `offset_x`, `offset_y` move it; with `mask_to_source_alpha`
/// it covers only the opaque parts of the children (which then render twice). Remotion's
/// `rotationX`, `rotationY` and `perspective` tilt the grid in 3D, which SVG cannot draw;
/// they are not available.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Gridlines<'a> {
    pub grid_size: f32,
    pub line_width: f32,
    pub line_color: Color,
    pub background_color: Color,
    pub rotation: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub mask_to_source_alpha: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Gridlines<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            grid_size: 64.0,
            line_width: 2.0,
            line_color: Color::WHITE,
            background_color: Color::TRANSPARENT,
            rotation: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            mask_to_source_alpha: false,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Gridlines<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let content = (self.children)(ui);
        let shape = self.mask_to_source_alpha.then(|| (self.children)(ui));
        let id = ui.id(self.key, 0, "gridlines");
        let (w, h) = self.area;
        let grid = self.grid_size.max(0.001);
        let lw = self.line_width.clamp(0.0, grid);
        let background = if self.background_color.is_transparent() {
            Svgr::empty()
        } else {
            let fill = self.background_color;
            fframes::svgr!(<rect x="0" y="0" width={w} height={h} fill={fill} />)
        };
        let lines = if lw > 0.0 {
            let fill = self.line_color;
            let k = coarsen(reach(self.area) / grid, 1);
            let (grid, lw) = (grid * k, lw * k);
            let (cx, cy, half) = cover(self.area, grid);
            // the lines' middles run through the offset, every grid units from there
            let (x0, y0) = (self.offset_x - lw / 2.0, self.offset_y - lw / 2.0);
            let across = steps(cx - half, cx + half, x0, grid)
                .map(|i| (x0 + i as f32 * grid, cy - half, lw, 2.0 * half));
            let down = steps(cy - half, cy + half, y0, grid)
                .map(|j| (cx - half, y0 + j as f32 * grid, 2.0 * half, lw));
            let d = rects(across.chain(down));
            turned(
                fframes::svgr!(<path d={d} fill={fill} />),
                self.rotation,
                self.area,
            )
        } else {
            Svgr::empty()
        };
        let layer = fframes::svgr!(<g>{background}{lines}</g>);
        draw_over(content, layer, shape, &id, self.area)
    }
}

/// Which way the stripes of [`Lines`] run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// Stripes over its children (Remotion's `lines()`): bands `thickness` units wide in
/// `colors` taken in turn, `gap` units of the children showing between them, running
/// `direction`. `angle` turns them about the middle of the area in degrees and `offset`
/// moves them across; with `mask_to_source_alpha` they cover only the opaque parts of the
/// children (which then render twice).
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Lines<'a> {
    pub colors: Vec<Color>,
    pub direction: LineDirection,
    pub thickness: f32,
    pub gap: f32,
    pub angle: f32,
    pub offset: f32,
    pub mask_to_source_alpha: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Lines<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            colors: PALE_BLUES.to_vec(),
            direction: LineDirection::Horizontal,
            thickness: 40.0,
            gap: 0.0,
            angle: 0.0,
            offset: 0.0,
            mask_to_source_alpha: false,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Lines<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let content = (self.children)(ui);
        let thickness = self.thickness.max(0.0);
        if self.colors.is_empty() || thickness <= 0.0 {
            return content;
        }
        let shape = self.mask_to_source_alpha.then(|| (self.children)(ui));
        let id = ui.id(self.key, 0, "lines");
        let pitch = thickness + self.gap.max(0.0);
        let k = coarsen(reach(self.area) / pitch, 1);
        let (pitch, thickness) = (pitch * k, thickness * k);
        let (cx, cy, half) = cover(self.area, pitch);
        let horizontal = self.direction == LineDirection::Horizontal;
        let (lo, hi) = if horizontal {
            (cy - half, cy + half)
        } else {
            (cx - half, cx + half)
        };
        let mut d = vec![String::new(); self.colors.len()];
        for i in steps(lo, hi, self.offset, pitch) {
            let at = self.offset + i as f32 * pitch;
            let stripe = if horizontal {
                (cx - half, at, 2.0 * half, thickness)
            } else {
                (at, cy - half, thickness, 2.0 * half)
            };
            d[i.rem_euclid(self.colors.len() as i64) as usize] += &rects([stripe]);
        }
        let layer = turned(paths(&self.colors, d), self.angle, self.area);
        draw_over(content, layer, shape, &id, self.area)
    }
}

/// Concentric rings over its children (Remotion's `rings()`): rings `thickness` units wide
/// in `colors` taken in turn, `gap` units of the children showing between them, round
/// `center` (0 to 1 across the area). `offset` moves the rings outwards (animate it to make
/// them expand); with `mask_to_source_alpha` they cover only the opaque parts of the
/// children (which then render twice). Draws at most [`MAX_RINGS`] rings.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Rings<'a> {
    pub colors: Vec<Color>,
    pub center: (f32, f32),
    pub thickness: f32,
    pub gap: f32,
    pub offset: f32,
    pub mask_to_source_alpha: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Rings<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            colors: PALE_BLUES.to_vec(),
            center: (0.5, 0.5),
            thickness: 40.0,
            gap: 0.0,
            offset: 0.0,
            mask_to_source_alpha: false,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl Rings<'_> {
    /// The rings that reach into the area: their index in the colour cycle and their inner
    /// and outer radius, from the middle outwards.
    pub fn bands(&self) -> Vec<(i64, f32, f32)> {
        let thickness = self.thickness.max(0.0);
        let pitch = thickness + self.gap.max(0.0);
        if thickness <= 0.0 {
            return Vec::new();
        }
        let (w, h) = self.area;
        let (cx, cy) = (self.center.0 * w, self.center.1 * h);
        let far = cx
            .abs()
            .max((w - cx).abs())
            .hypot(cy.abs().max((h - cy).abs()));
        // ring k starts at offset + k * pitch
        let first = ((-self.offset - thickness) / pitch).floor() as i64 + 1;
        (first..)
            .map(|k| {
                let inner = self.offset + k as f32 * pitch;
                (k, inner.max(0.0), inner + thickness)
            })
            .take_while(|&(_, inner, _)| inner < far)
            .take(MAX_RINGS)
            .collect()
    }
}

impl<'a> Component<'a> for Rings<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let content = (self.children)(ui);
        if self.colors.is_empty() {
            return content;
        }
        let shape = self.mask_to_source_alpha.then(|| (self.children)(ui));
        let id = ui.id(self.key, 0, "rings");
        let (cx, cy) = (self.center.0 * self.area.0, self.center.1 * self.area.1);
        let rings: Vec<Svgr<'a>> = self
            .bands()
            .into_iter()
            .map(|(k, inner, outer)| {
                let stroke = cycle(&self.colors, k);
                let r = f32::midpoint(inner, outer);
                let width = outer - inner;
                fframes::svgr!(
                    <circle cx={cx} cy={cy} r={r} fill="none" stroke={stroke} stroke-width={width} />
                )
            })
            .collect();
        let layer = fframes::svgr!(<g>{rings}</g>);
        draw_over(content, layer, shape, &id, self.area)
    }
}

/// A linear gradient `id` from `start` to `end` (0 to 1 across the `w` by `h` area), from
/// `from` to `to`, holding the end colours beyond the ends.
fn ramp<'a>(
    id: &str,
    (start, end): ((f32, f32), (f32, f32)),
    (from, to): (Color, Color),
    (w, h): (f32, f32),
) -> Svgr<'a> {
    let id = id.to_owned();
    fframes::svgr!(
        <linearGradient id={id} gradientUnits="userSpaceOnUse" x1={start.0 * w} y1={start.1 * h}
                        x2={end.0 * w} y2={end.1 * h}>
            <stop offset="0" stop-color={from} />
            <stop offset="1" stop-color={to} />
        </linearGradient>
    )
}

/// Its children's colours pulled towards a gradient (Remotion's `linearGradientTint()`):
/// the tint runs from `start_color` at `start` to `end_color` at `end` (0 to 1 across the
/// area) and `amount` mixes it in, 0 unchanged and 1 the gradient's colours. The alpha
/// stays. Drawn with an `feImage` of the gradient in a filter over the area.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LinearGradientTint<'a> {
    pub start: (f32, f32),
    pub end: (f32, f32),
    pub start_color: Color,
    pub end_color: Color,
    pub amount: Anim<'a, f32>,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for LinearGradientTint<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            start: (0.0, 0.5),
            end: (1.0, 0.5),
            start_color: Color::BLACK,
            end_color: Color::WHITE,
            amount: Anim::Fixed(0.5),
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for LinearGradientTint<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let amount = clamp01(ui.sample(&self.amount));
        let content = (self.children)(ui);
        if amount <= 0.0 {
            return content;
        }
        let id = ui.id(self.key, 0, "gradient-tint");
        let (w, h) = self.area;
        let gradient = format!("{id}-ramp");
        let fill = format!("url(#{gradient})");
        let picture = format!("{id}-picture");
        let href = format!("#{picture}");
        let defs = fframes::svgr!(
            <defs>
                {ramp(&gradient, (self.start, self.end), (self.start_color, self.end_color), self.area)}
                <rect id={picture} x="0" y="0" width={w} height={h} fill={fill} />
            </defs>
        );
        // the gradient cut to the children's alpha, then mixed with them
        let primitives = Svgr::from(vec![
            fframes::svgr!(<feImage href={href} result="ramp" />),
            fframes::svgr!(<feComposite in="ramp" in2="SourceAlpha" operator="in" result="tint" />),
            fframes::svgr!(
                <feComposite in="SourceGraphic" in2="tint" operator="arithmetic"
                             k1="0" k2={1.0 - amount} k3={amount} k4="0" />
            ),
        ]);
        let tinted = filtered(content, id, Region::Area(w, h), primitives);
        fframes::svgr!(<g>{defs}{tinted}</g>)
    }
}

/// A gradient filling the area (Remotion's `linearGradient()`), from `start_color` at
/// `start` to `end_color` at `end` (0 to 1 across the area). It replaces the layer, so it
/// takes no children: put it inside a group with the layers it covers.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LinearGradient<'a> {
    pub start: (f32, f32),
    pub end: (f32, f32),
    pub start_color: Color,
    pub end_color: Color,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
}

impl Themed for LinearGradient<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            start: (0.0, 0.5),
            end: (1.0, 0.5),
            start_color: Color::BLACK,
            end_color: Color::WHITE,
            area: (1920.0, 1080.0),
            key: None,
        }
    }
}

impl<'a> Component<'a> for LinearGradient<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let id = ui.id(self.key, 0, "linear-gradient");
        let (w, h) = self.area;
        let fill = format!("url(#{id})");
        fframes::svgr!(
            <g>
                <defs>
                    {ramp(&id, (self.start, self.end), (self.start_color, self.end_color), self.area)}
                </defs>
                <rect x="0" y="0" width={w} height={h} fill={fill} />
            </g>
        )
    }
}

/// How [`Vignette`] darkens the edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VignetteMode {
    /// The colour is laid over the edges, also where the children are transparent.
    #[default]
    Color,
    /// The edges fade out to transparent.
    Alpha,
}

/// Darkened edges round its children (Remotion's `vignette()`). The middle up to `radius`
/// stays clear, then the vignette grows over `feather` to `amount` (both 0 to 1, where 1
/// is the edge of the area seen from `center`). `roundness` 1 makes it an ellipse with the
/// area's proportions and 0 a rectangle; in between the two shapes are mixed (Remotion
/// mixes the distances, so the in-between shapes differ a little). `mode` lays `color` over
/// the edges, also where the children are transparent, or fades the children out towards
/// them. The curve is a smoothstep sampled at 16 points and drawn with gradients, so it
/// costs no filter.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Vignette<'a> {
    pub amount: Anim<'a, f32>,
    pub radius: f32,
    pub center: (f32, f32),
    pub feather: f32,
    pub roundness: f32,
    pub color: Color,
    pub mode: VignetteMode,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

/// The samples of the vignette's smoothstep between `radius` and `radius + feather`.
const VIGNETTE_STEPS: usize = 16;

impl Themed for Vignette<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            amount: Anim::Fixed(0.5),
            radius: 0.65,
            center: (0.5, 0.5),
            feather: 0.35,
            roundness: 1.0,
            color: Color::BLACK,
            mode: VignetteMode::Color,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

/// The curve of a [`Vignette`] at one moment.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Falloff {
    amount: f32,
    radius: f32,
    feather: f32,
}

impl Falloff {
    /// The strength of the vignette at `distance` from the centre (1 at the edge of the
    /// area).
    fn strength(&self, distance: f32) -> f32 {
        let amount = clamp01(self.amount);
        if self.feather <= 0.0001 {
            return if distance >= self.radius { amount } else { 0.0 };
        }
        let t = clamp01((distance - self.radius) / self.feather);
        t * t * (3.0 - 2.0 * t) * amount
    }

    /// The gradient stops `(offset, grey)` for a gradient whose offset 1 is at `reach`: the
    /// grey is the strength, or what is left of the picture with `keep`.
    fn stops(&self, reach: f32, keep: bool) -> Vec<(f32, f32)> {
        let grey = |d: f32| {
            let s = self.strength(d);
            if keep { 1.0 - s } else { s }
        };
        let mut at = vec![0.0, self.radius.max(0.0)];
        if self.feather > 0.0001 {
            at.extend(
                (1..=VIGNETTE_STEPS)
                    .map(|i| self.radius + self.feather * i as f32 / VIGNETTE_STEPS as f32),
            );
        }
        at.push(reach);
        let mut stops: Vec<(f32, f32)> = at
            .into_iter()
            .map(|d| (clamp01(d / reach), grey(d.max(0.0))))
            .collect();
        // a hard edge: clear up to the radius, full from there
        if self.feather <= 0.0001 {
            let r = clamp01(self.radius / reach);
            stops = vec![
                (0.0, grey(0.0)),
                (r, grey(0.0)),
                (r, grey(self.radius)),
                (1.0, grey(reach)),
            ];
        }
        stops
    }
}

impl Vignette<'_> {
    /// A grey picture over the area: white where the vignette is strongest, or with `keep`
    /// white where the picture stays.
    fn shade<'a>(&self, falloff: Falloff, id: &str, keep: bool) -> Svgr<'a> {
        let (w, h) = self.area;
        let (u, v) = self.center;
        let (cx, cy) = (u * w, v * h);
        // far enough to reach every corner from the centre, and past the feather
        let corner = 2.0 * u.max(1.0 - u).hypot(v.max(1.0 - v));
        let reach = (self.radius + self.feather.max(0.0)).max(corner) + 0.01;
        let stops: Vec<Svgr<'a>> = falloff
            .stops(reach, keep)
            .into_iter()
            .map(|(offset, grey)| {
                let g = (grey * 255.0).round() as u8;
                let color = Color::rgb(g, g, g);
                fframes::svgr!(<stop offset={offset} stop-color={color} />)
            })
            .collect();
        let stops = Svgr::from(stops);
        let round = format!("{id}-round");
        let across = format!("{id}-across");
        let down = format!("{id}-down");
        let (round_fill, across_fill, down_fill) = (
            format!("url(#{round})"),
            format!("url(#{across})"),
            format!("url(#{down})"),
        );
        // the rectangle is the larger of the two distances along the axes: the lighter
        // grey for the strength, the darker one for what stays
        let blend = if keep { "darken" } else { "lighten" };
        let roundness = clamp01(self.roundness);
        let square = if roundness < 1.0 {
            fframes::svgr!(
                <g>
                    <rect x="0" y="0" width={w} height={h} fill={across_fill} />
                    <rect x="0" y="0" width={w} height={h} fill={down_fill} mix-blend-mode={blend} />
                </g>
            )
        } else {
            Svgr::empty()
        };
        let ellipse = if roundness > 0.0 {
            fframes::svgr!(
                <rect x="0" y="0" width={w} height={h} fill={round_fill} opacity={roundness} />
            )
        } else {
            Svgr::empty()
        };
        let half = reach / 2.0;
        fframes::svgr!(
            <g>
                <defs>
                    <radialGradient id={round} cx={u} cy={v} r={half} fx={u} fy={v}>
                        {stops.clone()}
                    </radialGradient>
                    <linearGradient id={across} gradientUnits="userSpaceOnUse" x1={cx} y1="0"
                                    x2={cx + half * w} y2="0" spreadMethod="reflect">
                        {stops.clone()}
                    </linearGradient>
                    <linearGradient id={down} gradientUnits="userSpaceOnUse" x1="0" y1={cy}
                                    x2="0" y2={cy + half * h} spreadMethod="reflect">
                        {stops}
                    </linearGradient>
                </defs>
                <g>{square}{ellipse}</g>
            </g>
        )
    }
}

impl<'a> Component<'a> for Vignette<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let content = (self.children)(ui);
        let falloff = Falloff {
            amount: clamp01(ui.sample(&self.amount)),
            radius: self.radius,
            feather: self.feather,
        };
        if falloff.amount <= 0.0 {
            return content;
        }
        let id = ui.id(self.key, 0, "vignette");
        let (w, h) = self.area;
        let mask = format!("{id}-mask");
        match self.mode {
            VignetteMode::Color => {
                let fill = self.color;
                let shade = self.shade(falloff, &format!("{id}-shade"), false);
                let cover = fframes::svgr!(<rect x="0" y="0" width={w} height={h} fill={fill} />);
                let cover = masked(cover, shade, &mask, self.area);
                fframes::svgr!(<g>{content}{cover}</g>)
            }
            VignetteMode::Alpha => {
                let shade = self.shade(falloff, &format!("{id}-shade"), true);
                masked(content, shade, &mask, self.area)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_cover_the_area_from_the_middle_out() {
        let rings = Rings {
            thickness: 40.0,
            gap: 10.0,
            ..Rings::themed(&Theme::default())
        };
        let bands = rings.bands();
        assert_eq!(bands[0], (0, 0.0, 40.0));
        assert_eq!(bands[1], (1, 50.0, 90.0));
        let far = 960.0f32.hypot(540.0);
        assert!(bands.last().unwrap().1 < far && bands.last().unwrap().2 + 10.0 >= far);
        // moving outwards brings in a ring from the middle, which keeps its colour index
        let moved = Rings {
            offset: 30.0,
            ..rings
        };
        assert_eq!(moved.bands()[0], (-1, 0.0, 20.0));
    }

    #[test]
    fn rings_are_capped() {
        let rings = Rings {
            thickness: 0.1,
            ..Rings::themed(&Theme::default())
        };
        assert_eq!(rings.bands().len(), MAX_RINGS);
    }

    #[test]
    fn vignette_is_clear_inside_the_radius_and_full_past_the_feather() {
        let v = Falloff {
            amount: 0.5,
            radius: 0.65,
            feather: 0.35,
        };
        assert_eq!(v.strength(0.3), 0.0);
        assert!((v.strength(1.0) - 0.5).abs() < 1e-6);
        assert!((v.strength(0.825) - 0.25).abs() < 1e-6);
        let hard = Falloff { feather: 0.0, ..v };
        assert_eq!(hard.strength(0.64), 0.0);
        assert_eq!(hard.strength(0.66), 0.5);
        // every stop lies inside the gradient and the greys rise from the middle
        let stops = v.stops(1.5, false);
        assert!(stops.iter().all(|&(o, _)| (0.0..=1.0).contains(&o)));
        assert!(
            stops
                .windows(2)
                .all(|p| p[0].0 <= p[1].0 && p[0].1 <= p[1].1)
        );
    }
}
