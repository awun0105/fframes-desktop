use std::fmt::Write;

use fframes::{Color, Svgr, Transform};

use super::patterns::{coarsen, draw_over, masked, steps};
use super::{Region, filtered};
use crate::{
    anim::Anim,
    motion::{Rng, clamp01, lerp},
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// The most tiles [`Pattern`] draws. A smaller `scale` is drawn larger to stay below it.
pub const MAX_TILES: usize = 1024;

/// A seed for the cell `(i, j)` of a grid, the same on every machine.
fn cell_seed(seed: u64, i: i64, j: i64) -> u64 {
    let mut rng = Rng::new(seed ^ (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    rng.next_u64() ^ (j as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9)
}

/// A grey `feColorMatrix` matrix that writes `gain` times the red channel plus `offset` to
/// every colour channel, opaque.
fn grey_values(gain: f32, offset: f32) -> String {
    format!("{gain} 0 0 0 {offset} {gain} 0 0 0 {offset} {gain} 0 0 0 {offset} 0 0 0 0 1")
}

/// A smoothstep from 0 at `edge0` to 1 at `edge1`.
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp01((x - edge0) / (edge1 - edge0));
    t * t * (3.0 - 2.0 * t)
}

/// Its children repeated in a grid of small copies (Remotion's `pattern()`). The copies
/// are the children cut to `crop` (left, top, right and bottom insets in units) and scaled
/// by `scale`, `gap_x` and `gap_y` units apart. The grid starts at `origin` moved by
/// `offset_u`, `offset_v` (0 to 1 across the area). Each row is moved right by `row_offset`
/// units more than the one above, starting over every `row_offset_every` rows (0 never),
/// and each column down by `column_offset` units the same way; without `wrap` only the
/// tiles right of and below the origin are drawn. It replaces the children, transparent
/// between the tiles. The children render once and are reused with `<use>`, at most
/// [`MAX_TILES`] times.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Pattern<'a> {
    pub scale: f32,
    pub crop: (f32, f32, f32, f32),
    pub gap_x: f32,
    pub gap_y: f32,
    pub offset_u: f32,
    pub offset_v: f32,
    pub row_offset: f32,
    pub row_offset_every: u32,
    pub column_offset: f32,
    pub column_offset_every: u32,
    pub origin: (f32, f32),
    pub wrap: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Pattern<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            scale: 0.1,
            crop: (0.0, 0.0, 0.0, 0.0),
            gap_x: 0.0,
            gap_y: 0.0,
            offset_u: 0.0,
            offset_v: 0.0,
            row_offset: 0.0,
            row_offset_every: 0,
            column_offset: 0.0,
            column_offset_every: 0,
            origin: (0.0, 0.0),
            wrap: true,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl Pattern<'_> {
    /// The scale the tiles are drawn at and the top left corner of every tile that reaches
    /// into the area.
    pub fn tiles(&self) -> (f32, Vec<(f32, f32)>) {
        let (w, h) = self.area;
        let (left, top, right, bottom) = self.crop;
        let crop = ((w - left - right).max(0.001), (h - top - bottom).max(0.001));
        let mut scale = self.scale.max(0.0001);
        let count = |scale: f32| {
            (w / (crop.0 * scale + self.gap_x.max(0.0)) + 2.0)
                * (h / (crop.1 * scale + self.gap_y.max(0.0)) + 2.0)
        };
        if count(scale) > MAX_TILES as f32 {
            scale *= (count(scale) / MAX_TILES as f32).sqrt();
        }
        let tile = (crop.0 * scale, crop.1 * scale);
        let pitch = (
            (tile.0 + self.gap_x).max(0.001),
            (tile.1 + self.gap_y).max(0.001),
        );
        let (ox, oy) = (
            (self.origin.0 + self.offset_u) * w,
            (self.origin.1 + self.offset_v) * h,
        );
        let phase = |index: i64, every: u32| {
            if every == 0 {
                index as f32
            } else {
                index.rem_euclid(i64::from(every)) as f32
            }
        };
        let shift_x = |row: i64| phase(row, self.row_offset_every) * self.row_offset;
        let shift_y = |column: i64| phase(column, self.column_offset_every) * self.column_offset;
        // the columns of a row, moved by the row's offset
        let columns = |row: i64| steps(-tile.0, w, ox + shift_x(row), pitch.0);
        // how far the columns' offsets move a tile up or down
        let reach = steps(-tile.1, h, oy, pitch.1)
            .flat_map(columns)
            .map(|column| shift_y(column).abs())
            .fold(0.0, f32::max);
        let mut tiles = Vec::new();
        'rows: for row in steps(-tile.1 - reach, h + reach, oy, pitch.1) {
            for column in columns(row) {
                if !self.wrap && (row < 0 || column < 0) {
                    continue;
                }
                let x = ox + shift_x(row) + column as f32 * pitch.0;
                let y = oy + shift_y(column) + row as f32 * pitch.1;
                if x < w && y < h && x + tile.0 > 0.0 && y + tile.1 > 0.0 {
                    tiles.push((x, y));
                    if tiles.len() == MAX_TILES {
                        break 'rows;
                    }
                }
            }
        }
        (scale, tiles)
    }
}

impl<'a> Component<'a> for Pattern<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let content = (self.children)(ui);
        let id = ui.id(self.key, 0, "pattern");
        let (w, h) = self.area;
        let (left, top, right, bottom) = self.crop;
        let (scale, tiles) = self.tiles();
        let tile = format!("{id}-tile");
        let crop = format!("{id}-crop");
        let (href, crop_url) = (format!("#{tile}"), format!("url(#{crop})"));
        let unshift = Transform::translate(-left, -top);
        let size = Transform::scale(scale);
        let copies: Vec<Svgr<'a>> = tiles
            .into_iter()
            .map(|(x, y)| {
                let place = Transform::translate(x, y);
                fframes::svgr!(
                    <g transform={place}>
                        <g transform={size}><use href={href.clone()} /></g>
                    </g>
                )
            })
            .collect();
        let copies = draw_over(Svgr::empty(), Svgr::from(copies), None, &id, self.area);
        fframes::svgr!(
            <g>
                <defs>
                    <clipPath id={crop}>
                        <rect x={left} y={top} width={(w - left - right).max(0.0)}
                              height={(h - top - bottom).max(0.0)} />
                    </clipPath>
                    <g id={tile}>
                        <g transform={unshift}>
                            <g clip-path={crop_url}>{content}</g>
                        </g>
                    </g>
                </defs>
                {copies}
            </g>
        )
    }
}

/// Small holes punched through its children (Remotion's `speckle()`): a share `density` of
/// the cells of a grid gets a round hole `size` units across. `randomness` moves the holes
/// off the cell centres and varies their size (0 keeps a regular grid). One cell is 2.5
/// holes wide; a grid finer than about 20,000 holes is drawn coarser.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Speckle<'a> {
    pub density: f32,
    pub size: f32,
    pub randomness: f32,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Speckle<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            density: 0.08,
            size: 4.0,
            randomness: 1.0,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl Speckle<'_> {
    /// The holes: their centres and radii.
    pub fn holes(&self) -> Vec<(f32, f32, f32)> {
        let density = clamp01(self.density);
        if density <= 0.0 || self.size <= 0.0 {
            return Vec::new();
        }
        let (w, h) = self.area;
        let cell = (self.size * 2.5).max(1.0);
        let k = coarsen(w * h / (cell * cell) * density.max(0.05), 2);
        let (cell, size) = (cell * k, self.size * k);
        let randomness = clamp01(self.randomness);
        let mut holes = Vec::new();
        for j in steps(0.0, h, 0.0, cell) {
            for i in steps(0.0, w, 0.0, cell) {
                let mut rng = Rng::new(cell_seed(0, i, j));
                if rng.unit() as f32 > density {
                    continue;
                }
                let dx = (rng.unit() as f32 - 0.5) * randomness;
                let dy = (rng.unit() as f32 - 0.5) * randomness;
                let vary = lerp(1.0, 0.35 + rng.unit() as f32 * 0.65, randomness);
                let x = (i as f32 + 0.5 + dx) * cell;
                let y = (j as f32 + 0.5 + dy) * cell;
                holes.push((x, y, size * 0.5 * vary));
            }
        }
        holes
    }
}

impl<'a> Component<'a> for Speckle<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let content = (self.children)(ui);
        let holes = self.holes();
        if holes.is_empty() {
            return content;
        }
        let id = ui.id(self.key, 0, "speckle");
        let (w, h) = self.area;
        let mut d = String::new();
        for (x, y, r) in holes {
            let _ = write!(
                d,
                "M{} {y}a{r} {r} 0 1 0 {} 0a{r} {r} 0 1 0 {} 0Z",
                x - r,
                2.0 * r,
                -2.0 * r
            );
        }
        let mask = fframes::svgr!(
            <g>
                <rect x="0" y="0" width={w} height={h} fill="#ffffff" />
                <path d={d} fill="#000000" />
            </g>
        );
        masked(content, mask, &id, self.area)
    }
}

/// Plaid cloth printed on its children (Remotion's `flannel()`): wide and fine stripes
/// of `stripe_color` across and down a `base_color` cloth, a check `size` units wide,
/// shaded by the children's brightness and mixed in by `amount`. `softness` blurs the
/// stripe edges. The children's alpha stays. Remotion's thread-level weave (a pixel-sized
/// ripple) is left out. Drawn with an `feImage` of the plaid in a filter over the area.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Flannel<'a> {
    pub amount: Anim<'a, f32>,
    pub size: f32,
    pub softness: f32,
    pub base_color: Color,
    pub stripe_color: Color,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Flannel<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            amount: Anim::Fixed(0.7),
            size: 96.0,
            softness: 0.18,
            base_color: Color::hex("#c92f3d"),
            stripe_color: Color::hex("#241015"),
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl Flannel<'_> {
    /// The plaid over the area.
    fn plaid<'a>(&self, size: f32) -> Svgr<'a> {
        let (w, h) = self.area;
        // stripes of `width` (a share of `period`) centred in each period, across and down
        let bands = |period: f32, width: f32| {
            let width = width * period;
            let mut d = String::new();
            for i in steps(0.0, w, period / 2.0 - width / 2.0, period) {
                let x = period * (i as f32 + 0.5) - width / 2.0;
                let _ = write!(d, "M{x} 0h{width}v{h}h{}Z", -width);
            }
            for j in steps(0.0, h, period / 2.0 - width / 2.0, period) {
                let y = period * (j as f32 + 0.5) - width / 2.0;
                let _ = write!(d, "M0 {y}h{w}v{width}h{}Z", -w);
            }
            d
        };
        // where the stripes of one kind cross
        let crossings = |period: f32, width: f32| {
            let width = width * period;
            let mut d = String::new();
            for j in steps(0.0, h, period / 2.0 - width / 2.0, period) {
                for i in steps(0.0, w, period / 2.0 - width / 2.0, period) {
                    let x = period * (i as f32 + 0.5) - width / 2.0;
                    let y = period * (j as f32 + 0.5) - width / 2.0;
                    let _ = write!(d, "M{x} {y}h{width}v{width}h{}Z", -width);
                }
            }
            d
        };
        let (wide, fine) = (bands(size, 0.34), bands(size / 2.0, 0.07));
        let crossed = crossings(size, 0.34) + &crossings(size / 2.0, 0.07);
        let (base, stripe) = (self.base_color, self.stripe_color);
        let dark = Color::rgb(
            (f32::from(stripe.r) * 0.72) as u8,
            (f32::from(stripe.g) * 0.72) as u8,
            (f32::from(stripe.b) * 0.72) as u8,
        );
        fframes::svgr!(
            <g>
                <rect x="0" y="0" width={w} height={h} fill={base} />
                <path d={fine} fill={stripe} opacity="0.72" />
                <path d={wide} fill={stripe} />
                <path d={crossed} fill={dark} />
            </g>
        )
    }
}

impl<'a> Component<'a> for Flannel<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let amount = clamp01(ui.sample(&self.amount));
        let content = (self.children)(ui);
        if amount <= 0.0 {
            return content;
        }
        let id = ui.id(self.key, 0, "flannel");
        let (w, h) = self.area;
        let size = self.size.max(1.0) * coarsen(w * h / self.size.max(1.0).powi(2) * 4.0, 2);
        let picture = format!("{id}-plaid");
        let href = format!("#{picture}");
        let plaid = self.plaid(size);
        let defs = fframes::svgr!(<defs><g id={picture}>{plaid}</g></defs>);
        let soft = lerp(0.002, 0.08, clamp01(self.softness)) * size / 2.0;
        let primitives = Svgr::from(vec![
            fframes::svgr!(<feImage href={href} result="cloth" />),
            fframes::svgr!(<feGaussianBlur in="cloth" stdDeviation={soft} result="soft" />),
            // the children's brightness as an opaque grey
            fframes::svgr!(
                <feColorMatrix in="SourceGraphic" type="matrix" result="luma"
                    values="0.2126 0.7152 0.0722 0 0 0.2126 0.7152 0.0722 0 0 0.2126 0.7152 0.0722 0 0 0 0 0 0 1" />
            ),
            // darker cloth where the picture is dark: 0.58 to 1.18 times
            fframes::svgr!(
                <feComposite in="soft" in2="luma" operator="arithmetic" k1="0.6" k2="0.58" k3="0" k4="0" result="shaded" />
            ),
            fframes::svgr!(<feComposite in="shaded" in2="SourceAlpha" operator="in" result="cut" />),
            fframes::svgr!(
                <feComposite in="SourceGraphic" in2="cut" operator="arithmetic"
                             k1="0" k2={1.0 - amount} k3={amount} k4="0" />
            ),
        ]);
        let printed = filtered(content, id, Region::Area(w, h), primitives);
        fframes::svgr!(<g>{defs}{printed}</g>)
    }
}

/// The share of a strand of [`Burlap`] at `t` across one strand (0 to 1): full in the
/// middle, gone at the edges.
fn strand(t: f32) -> f32 {
    1.0 - smoothstep(0.2, 0.5, (t.rem_euclid(1.0) - 0.5).abs())
}

/// Coarse woven sacking over its children (Remotion's `burlap()`): strands `size` units
/// wide run across and down, the gaps between them and blotchy `roughness` darken the
/// picture towards `color`, mixed in by `amount`. `seed` picks the blotches. Remotion's
/// over and under weave, strand jitter and dashes are left out. The strands are repeating
/// gradients, so they stay sharp at any size, but keep them a few output pixels wide;
/// finer cloth aliases. The children render twice, their alpha stays.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Burlap<'a> {
    pub amount: Anim<'a, f32>,
    pub size: f32,
    pub roughness: f32,
    pub seed: u32,
    pub color: Color,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Burlap<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            amount: Anim::Fixed(0.55),
            size: 5.0,
            roughness: 0.7,
            seed: 0,
            color: Color::BLACK,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for Burlap<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let amount = clamp01(ui.sample(&self.amount));
        let content = (self.children)(ui);
        if amount <= 0.0 {
            return content;
        }
        let shape = (self.children)(ui);
        let id = ui.id(self.key, 0, "burlap");
        let (w, h) = self.area;
        let size = self.size.max(0.5);
        // the gaps darken by up to 0.85: each axis carries the square root of it
        let stops: Vec<Svgr<'a>> = (0..=20)
            .map(|i| {
                let t = i as f32 / 20.0;
                let g = (0.85f32.sqrt() * (1.0 - strand(t)) * 255.0).round() as u8;
                let color = Color::rgb(g, g, g);
                fframes::svgr!(<stop offset={t} stop-color={color} />)
            })
            .collect();
        let stops = Svgr::from(stops);
        let (across, down, noise) = (
            format!("{id}-across"),
            format!("{id}-down"),
            format!("{id}-noise"),
        );
        let (across_fill, down_fill, noise_url) = (
            format!("url(#{across})"),
            format!("url(#{down})"),
            format!("url(#{noise})"),
        );
        let roughness = clamp01(self.roughness);
        let frequency = 1.0 / (size * 7.0);
        let blotches = grey_values(2.4, -0.9);
        let shade = fframes::svgr!(
            <g>
                <defs>
                    <linearGradient id={across} gradientUnits="userSpaceOnUse" x1="0" y1="0"
                                    x2={size} y2="0" spreadMethod="repeat">
                        {stops.clone()}
                    </linearGradient>
                    <linearGradient id={down} gradientUnits="userSpaceOnUse" x1="0" y1="0"
                                    x2="0" y2={size} spreadMethod="repeat">
                        {stops}
                    </linearGradient>
                    <filter id={noise} filterUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}
                            color-interpolation-filters="sRGB">
                        <feTurbulence type="fractalNoise" baseFrequency={frequency} numOctaves="3"
                                      seed={self.seed} />
                        <feColorMatrix type="matrix" values={blotches} />
                    </filter>
                </defs>
                <rect x="0" y="0" width={w} height={h} fill={across_fill} />
                <rect x="0" y="0" width={w} height={h} fill={down_fill} mix-blend-mode="multiply" />
                <rect x="0" y="0" width={w} height={h} filter={noise_url} opacity={roughness * 0.6}
                      mix-blend-mode="screen" />
            </g>
        );
        let fill = self.color;
        let cloth =
            fframes::svgr!(<rect x="0" y="0" width={w} height={h} fill={fill} opacity={amount} />);
        let cloth = masked(cloth, shade, &format!("{id}-shade"), self.area);
        draw_over(content, cloth, Some(shape), &id, self.area)
    }
}

/// Paper texture on its children (Remotion's `paper()`): fibres, crumples, fine roughness
/// and `fold_count` creases make a relief lit from the top right, which shades the
/// picture and tints it between `color_back` (in the shade) and `color_front`, mixed in by
/// `amount`. `contrast` is the strength of the light, `fiber`, `crumples`, `roughness` and
/// `folds` the strength of each part and `fiber_size`, `crumple_size` their sizes, all
/// relative to `scale` (the texture is `scale` times the area's height across five
/// units). `seed` picks the texture and the creases.
///
/// Approximations of Remotion's look: the relief is lit through `feTurbulence` and
/// offsets, the picture is not displaced along the folds (that needs
/// `feDisplacementMap`), the tint uses the average light, and `drops` and `fade` are not
/// available. The children's alpha stays.
///
/// The roughness grain and the step the light is measured over follow `scale`, a few
/// pixels at the default. The strong light magnifies how differently the CPU and Skia
/// backends sample `feTurbulence`: the gallery's paper differs in about 1% of the pixels at
/// full size and 6.6% at half size, the CPU grain a little softer.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Paper<'a> {
    pub amount: Anim<'a, f32>,
    pub color_front: Color,
    pub color_back: Color,
    pub contrast: f32,
    pub roughness: f32,
    pub fiber: f32,
    pub fiber_size: f32,
    pub crumples: f32,
    pub crumple_size: f32,
    pub folds: f32,
    pub fold_count: u32,
    pub seed: u32,
    pub scale: f32,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Paper<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            amount: Anim::Fixed(1.0),
            color_front: Color::hex("#9fadbc"),
            color_back: Color::WHITE,
            contrast: 0.3,
            roughness: 0.4,
            fiber: 0.3,
            fiber_size: 0.2,
            crumples: 0.3,
            crumple_size: 0.35,
            folds: 0.65,
            fold_count: 5,
            seed: 6,
            scale: 0.6,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

/// The most creases [`Paper`] draws.
const MAX_FOLDS: u32 = 64;

impl Paper<'_> {
    /// The creases as ridges filled with the gradient `ridge`, white along the crease, over
    /// black.
    fn creases<'a>(&self, unit: f32, ridge: &str) -> Svgr<'a> {
        let (w, h) = self.area;
        let long = w.hypot(h) * 2.0;
        let width = unit * 0.5;
        let mut rng = Rng::new(u64::from(self.seed) ^ 0x0070_6170_6572);
        let ridges: Vec<Svgr<'a>> = (0..self.fold_count.min(MAX_FOLDS))
            .map(|_| {
                let x = rng.range(0.0, f64::from(w)) as f32;
                let y = rng.range(0.0, f64::from(h)) as f32;
                let angle = rng.range(0.0, 180.0) as f32;
                let place = format!("translate({x} {y}) rotate({angle})");
                let fill = format!("url(#{ridge})");
                fframes::svgr!(
                    <rect x={-width / 2.0} y={-long / 2.0} width={width} height={long}
                          transform={place} fill={fill} mix-blend-mode="lighten" />
                )
            })
            .collect();
        fframes::svgr!(
            <g>
                <rect x="0" y="0" width={w} height={h} fill="#000000" />
                {ridges}
            </g>
        )
    }
}

impl<'a> Component<'a> for Paper<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let amount = clamp01(ui.sample(&self.amount));
        let content = (self.children)(ui);
        if amount <= 0.0 {
            return content;
        }
        let id = ui.id(self.key, 0, "paper");
        let (w, h) = self.area;
        let unit = h * self.scale.max(0.01) / 5.0;
        let picture = format!("{id}-creases");
        let href = format!("#{picture}");
        let ridge = format!("{id}-ridge");
        let creases = self.creases(unit, &ridge);
        let defs = fframes::svgr!(
            <defs>
                <linearGradient id={ridge} x1="0" y1="0" x2="1" y2="0">
                    <stop offset="0" stop-color="#000000" />
                    <stop offset="0.5" stop-color="#ffffff" />
                    <stop offset="1" stop-color="#000000" />
                </linearGradient>
                <g id={picture}>{creases}</g>
            </defs>
        );
        // the parts of the relief and their weights, together at most 1
        let parts = [
            clamp01(self.fiber),
            clamp01(self.crumples),
            clamp01(self.roughness) * 0.5,
            clamp01(self.folds),
        ];
        let total = parts.iter().sum::<f32>().max(1.0);
        let [fiber, crumples, roughness, folds] = parts.map(|p| p / total);
        let flat = 1.0 - (fiber + crumples + roughness + folds);
        let fiber_frequency = 2.0 / (unit * self.fiber_size.max(0.01));
        let crumple_frequency = 0.64 / (unit * self.crumple_size.max(0.01));
        // Remotion's roughness is a pixel-sized grain; here it follows the scale, a few
        // pixels at the default, so the backends draw it alike
        let rough_frequency = 1.0 / (unit * 0.02);
        // the light comes from the top right; a surface rising towards it is lit
        let (lx, ly) = (1.0 / 5f32.sqrt(), -2.0 / 5f32.sqrt());
        // at least two output pixels at the default scale: a finer offset is rounded
        // differently by each backend
        let step = (unit * 0.03).max(2.0);
        let gain = 2.0 + 14.0 * clamp01(self.contrast);
        let tone = |front: u8, back: u8| {
            let t = lerp(f32::from(back), f32::from(front), 0.7) / 255.0;
            lerp(1.0, t, 0.65) * 1.014 / 0.58
        };
        let (front, back) = (self.color_front, self.color_back);
        let tint = format!(
            "{} 0 0 0 0 0 {} 0 0 0 0 0 {} 0 0 0 0 0 1 0",
            tone(front.r, back.r),
            tone(front.g, back.g),
            tone(front.b, back.b)
        );
        let seed = self.seed;
        let primitives = Svgr::from(vec![
            fframes::svgr!(<feTurbulence type="fractalNoise" baseFrequency={fiber_frequency} numOctaves="2" seed={seed} result="fiber-noise" />),
            fframes::svgr!(<feTurbulence type="turbulence" baseFrequency={crumple_frequency} numOctaves="3" seed={seed + 1} result="crumples-noise" />),
            fframes::svgr!(<feTurbulence type="fractalNoise" baseFrequency={rough_frequency} numOctaves="1" seed={seed + 2} result="rough-noise" />),
            // turbulence carries noise in its alpha too: keep the red channel, opaque
            fframes::svgr!(<feColorMatrix in="fiber-noise" type="matrix" values={grey_values(1.0, 0.0)} result="fiber" />),
            fframes::svgr!(<feColorMatrix in="crumples-noise" type="matrix" values={grey_values(1.0, 0.0)} result="crumples" />),
            fframes::svgr!(<feColorMatrix in="rough-noise" type="matrix" values={grey_values(1.0, 0.0)} result="rough" />),
            fframes::svgr!(<feImage href={href} result="creases" />),
            fframes::svgr!(<feComposite in="fiber" in2="crumples" operator="arithmetic" k1="0" k2={fiber} k3={crumples} k4="0" result="relief-a" />),
            fframes::svgr!(<feComposite in="relief-a" in2="rough" operator="arithmetic" k1="0" k2="1" k3={roughness} k4="0" result="relief-b" />),
            fframes::svgr!(<feComposite in="relief-b" in2="creases" operator="arithmetic" k1="0" k2="1" k3={folds} k4="0" result="relief-c" />),
            // mid grey makes up the weights to 1, so the relief is opaque
            fframes::svgr!(<feFlood flood-color="#808080" result="flat" />),
            fframes::svgr!(<feComposite in="relief-c" in2="flat" operator="arithmetic" k1="0" k2="1" k3={flat} k4="0" result="relief" />),
            fframes::svgr!(<feOffset in="relief" dx={step * lx} dy={step * ly} result="moved" />),
            // the strip the offset leaves empty along the edges stays flat
            fframes::svgr!(
                <feMerge result="behind">
                    <feMergeNode in="relief" />
                    <feMergeNode in="moved" />
                </feMerge>
            ),
            fframes::svgr!(<feColorMatrix in="behind" type="matrix" values={grey_values(-1.0, 1.0)} result="below" />),
            // mid grey on a flat surface, lighter where it rises towards the light: adding the
            // inverted copy instead of subtracting keeps the result opaque
            fframes::svgr!(<feComposite in="relief" in2="below" operator="arithmetic" k1="0" k2={gain} k3={gain} k4={0.5 - gain} result="light" />),
            // 0.16 to 1 times the picture, the alpha kept, then brightened and tinted
            fframes::svgr!(<feComposite in="SourceGraphic" in2="light" operator="arithmetic" k1="0.84" k2="0.16" k3="0" k4="0" result="lit" />),
            fframes::svgr!(<feColorMatrix in="lit" type="matrix" values={tint} result="paper" />),
            fframes::svgr!(
                <feComposite in="SourceGraphic" in2="paper" operator="arithmetic"
                             k1="0" k2={1.0 - amount} k3={amount} k4="0" />
            ),
        ]);
        let textured = filtered(content, id, Region::Area(w, h), primitives);
        fframes::svgr!(<g>{defs}{textured}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_tiles_cover_the_area_and_stagger() {
        let pattern = Pattern::themed(&Theme::default());
        let (scale, tiles) = pattern.tiles();
        assert_eq!(scale, 0.1);
        assert_eq!(tiles.len(), 100);
        assert!(tiles.contains(&(0.0, 0.0)) && tiles.contains(&(1728.0, 972.0)));
        let staggered = Pattern {
            row_offset: 96.0,
            row_offset_every: 2,
            wrap: false,
            ..pattern
        };
        let (_, tiles) = staggered.tiles();
        assert!(tiles.contains(&(96.0, 108.0)) && tiles.contains(&(0.0, 216.0)));
        assert!(tiles.iter().all(|&(x, y)| x >= 0.0 && y >= 0.0));
    }

    #[test]
    fn pattern_tiles_are_capped() {
        let pattern = Pattern {
            scale: 0.001,
            ..Pattern::themed(&Theme::default())
        };
        let (scale, tiles) = pattern.tiles();
        assert!(scale > 0.001 && tiles.len() <= MAX_TILES);
    }

    #[test]
    fn speckle_is_seeded_and_follows_the_density() {
        let speckle = Speckle {
            size: 8.0,
            ..Speckle::themed(&Theme::default())
        };
        let holes = speckle.holes();
        assert_eq!(holes, speckle.holes());
        let cells = (1920.0 * 1080.0) / (20.0 * 20.0);
        let share = holes.len() as f32 / cells;
        assert!((share - 0.08).abs() < 0.02, "{share}");
        assert!(holes.iter().all(|&(_, _, r)| r > 0.0 && r <= 4.0));
    }

    #[test]
    fn strands_fill_the_middle_and_leave_the_edges() {
        assert_eq!(strand(0.5), 1.0);
        assert_eq!(strand(0.0), 0.0);
        assert_eq!(strand(1.0), 0.0);
        assert!(strand(0.25) > 0.0 && strand(0.25) < 1.0);
    }
}
