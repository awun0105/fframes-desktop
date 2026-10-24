use fframes::{Color, Svgr};

use super::Region;
use crate::{anim::Anim, motion::clamp01};

/// Luminance weights of the SVG `saturate` and `hueRotate` matrices.
const LUMA: [f32; 3] = [0.213, 0.715, 0.072];

/// Samples of a curve in a `feComponentTransfer` table: linear between them, which keeps a
/// smooth curve within 1/1000 of itself.
const TABLE_SIZE: usize = 33;

/// A colour matrix as `feColorMatrix` takes it: four rows of five, each output channel a
/// weighted sum of the input red, green, blue and alpha plus an offset, all from 0 to 1.
/// The constructors follow the CSS filter functions and the SVG matrices; [`then`](Self::then)
/// chains two into one, which is how [`ColorStack`] draws several colour effects with one
/// filter.
///
/// ```
/// use fframes_kit::prelude::*;
///
/// let gray = ColorMatrix::grayscale(1.0).apply([1.0, 0.0, 0.0, 1.0]);
/// assert!((gray[0] - 0.213).abs() < 1e-6 && gray[0] == gray[1] && gray[1] == gray[2]);
/// assert_eq!(ColorMatrix::invert(1.0).apply([1.0, 1.0, 1.0, 1.0]), [0.0, 0.0, 0.0, 1.0]);
/// let warm_and_bright = ColorMatrix::brightness(1.2).then(ColorMatrix::hue_rotate(-10.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorMatrix(pub [f32; 20]);

impl Default for ColorMatrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl ColorMatrix {
    /// Leaves every colour as it is.
    pub const IDENTITY: Self = Self([
        1.0, 0.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 0.0,
    ]);

    /// Each colour channel times `gain`, plus `offset`; alpha unchanged.
    pub fn channels(gain: [f32; 3], offset: [f32; 3]) -> Self {
        let mut m = Self::IDENTITY.0;
        for c in 0..3 {
            m[c * 5 + c] = gain[c];
            m[c * 5 + 4] = offset[c];
        }
        Self(m)
    }

    /// Every channel as the luminance mapped onto the line from `from` (black) to `to`
    /// (white), as red, green and blue from 0 to 1.
    fn luminance_to(from: [f32; 3], to: [f32; 3]) -> Self {
        let mut m = Self::IDENTITY.0;
        for c in 0..3 {
            for (i, w) in LUMA.iter().enumerate() {
                m[c * 5 + i] = (to[c] - from[c]) * w;
            }
            m[c * 5 + 4] = from[c];
        }
        Self(m)
    }

    /// CSS `brightness()`: the colours times `amount`, 1 unchanged.
    pub fn brightness(amount: f32) -> Self {
        Self::channels([amount; 3], [0.0; 3])
    }

    /// CSS `contrast()`: the colours pushed away from mid grey by `amount`, 1 unchanged.
    pub fn contrast(amount: f32) -> Self {
        Self::channels([amount; 3], [0.5 - 0.5 * amount; 3])
    }

    /// Exposure in photographic stops: every stop doubles the light, 0 unchanged.
    pub fn exposure(stops: f32) -> Self {
        Self::brightness(2f32.powf(stops))
    }

    /// The SVG `saturate` matrix: 0 grey, 1 unchanged, above 1 more saturated.
    pub fn saturation(amount: f32) -> Self {
        let s = amount;
        let [r, g, b] = LUMA;
        Self([
            r + (1.0 - r) * s,
            g - g * s,
            b - b * s,
            0.0,
            0.0,
            r - r * s,
            g + (1.0 - g) * s,
            b - b * s,
            0.0,
            0.0,
            r - r * s,
            g - g * s,
            b + (1.0 - b) * s,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ])
    }

    /// CSS `grayscale()`: 1 fully grey, 0 unchanged.
    pub fn grayscale(amount: f32) -> Self {
        Self::saturation(1.0 - clamp01(amount))
    }

    /// The SVG `hueRotate` matrix: the hues turned round the colour wheel by `degrees`.
    pub fn hue_rotate(degrees: f32) -> Self {
        let (sin, cos) = degrees.to_radians().sin_cos();
        Self([
            0.213 + cos * 0.787 - sin * 0.213,
            0.715 - cos * 0.715 - sin * 0.715,
            0.072 - cos * 0.072 + sin * 0.928,
            0.0,
            0.0,
            0.213 - cos * 0.213 + sin * 0.143,
            0.715 + cos * 0.285 + sin * 0.140,
            0.072 - cos * 0.072 - sin * 0.283,
            0.0,
            0.0,
            0.213 - cos * 0.213 - sin * 0.787,
            0.715 - cos * 0.715 + sin * 0.715,
            0.072 + cos * 0.928 + sin * 0.072,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ])
    }

    /// CSS `invert()`: 1 the negative, 0 unchanged, 0.5 flat grey.
    pub fn invert(amount: f32) -> Self {
        let a = clamp01(amount);
        Self::channels([1.0 - 2.0 * a; 3], [a; 3])
    }

    /// The picture in shades of `color` (its luminance times the colour), mixed with the
    /// original by `amount`.
    pub fn tint(color: Color, amount: f32) -> Self {
        Self::IDENTITY.mix(Self::luminance_to([0.0; 3], rgb(color)), amount)
    }

    /// Dark tones turn `dark` and light tones `light`, through the luminance; mixed with the
    /// original by `amount`.
    pub fn duotone(dark: Color, light: Color, amount: f32) -> Self {
        Self::IDENTITY.mix(Self::luminance_to(rgb(dark), rgb(light)), amount)
    }

    /// White balance: `temperature` from -1 (cold, blue) to 1 (warm, orange) and `tint` from
    /// -1 (green) to 1 (magenta), as gains of up to a fifth on the red, green and blue
    /// channels.
    pub fn white_balance(temperature: f32, tint: f32) -> Self {
        let (t, m) = (temperature.clamp(-1.0, 1.0), tint.clamp(-1.0, 1.0));
        Self::channels([1.0 + 0.2 * t, 1.0 - 0.2 * m, 1.0 - 0.2 * t], [0.0; 3])
    }

    /// Vibrance as a gentler saturation: `amount` 1 saturates by half as much as
    /// [`saturation`](Self::saturation)`(2.0)`. An approximation: real vibrance spares colours
    /// that are already saturated, which a matrix cannot tell apart.
    pub fn vibrance(amount: f32) -> Self {
        Self::saturation(1.0 + 0.5 * amount)
    }

    /// This matrix mixed towards `other` by `amount` (0 this, 1 `other`).
    pub fn mix(self, other: Self, amount: f32) -> Self {
        let a = clamp01(amount);
        let mut m = self.0;
        for (v, o) in m.iter_mut().zip(other.0) {
            *v += (o - *v) * a;
        }
        Self(m)
    }

    /// This matrix followed by `next`, as one matrix. Unlike two nested filters it does not
    /// clamp the colours to 0 to 1 in between, so a brightness that overshoots white and a
    /// contrast after it can differ from the nested pair.
    pub fn then(self, next: Self) -> Self {
        let (a, b) = (self.0, next.0);
        let mut m = [0.0; 20];
        for r in 0..4 {
            for c in 0..5 {
                let sum: f32 = (0..4).map(|k| b[r * 5 + k] * a[k * 5 + c]).sum();
                m[r * 5 + c] = sum + if c == 4 { b[r * 5 + 4] } else { 0.0 };
            }
        }
        Self(m)
    }

    /// The colour `rgba` (unpremultiplied, 0 to 1) through the matrix, clamped like a filter.
    pub fn apply(&self, rgba: [f32; 4]) -> [f32; 4] {
        let m = &self.0;
        std::array::from_fn(|r| {
            let sum: f32 = (0..4).map(|k| m[r * 5 + k] * rgba[k]).sum();
            clamp01(sum + m[r * 5 + 4])
        })
    }

    /// The `values` of a `feColorMatrix`.
    pub fn values(&self) -> String {
        let parts: Vec<String> = self.0.iter().map(|v| format!("{v}")).collect();
        parts.join(" ")
    }

    /// The `feColorMatrix` primitive of this matrix.
    pub fn primitive<'a>(&self) -> Svgr<'a> {
        let values = self.values();
        fframes::svgr!(<feColorMatrix type="matrix" values={values} />)
    }
}

/// Red, green and blue of `color` from 0 to 1.
fn rgb(color: Color) -> [f32; 3] {
    [color.r, color.g, color.b].map(|c| f32::from(c) / 255.0)
}

/// `f` from 0 to 1 sampled as the `tableValues` of a transfer function.
fn table(f: impl Fn(f32) -> f32) -> String {
    let parts: Vec<String> = (0..TABLE_SIZE)
        .map(|i| format!("{:.4}", clamp01(f(i as f32 / (TABLE_SIZE - 1) as f32))))
        .collect();
    parts.join(" ")
}

/// A `feComponentTransfer` with the tables `r`, `g` and `b` (alpha unchanged).
fn transfer<'a>(r: String, g: String, b: String) -> Svgr<'a> {
    fframes::svgr!(
        <feComponentTransfer>
            <feFuncR type="table" tableValues={r} />
            <feFuncG type="table" tableValues={g} />
            <feFuncB type="table" tableValues={b} />
        </feComponentTransfer>
    )
}

/// A smooth curve through `points` (x from 0 to 1, sorted by x) that never overshoots
/// between them (monotone cubic, Fritsch and Carlson). Flat before the first point and
/// after the last; the identity without points.
pub(crate) fn curve(points: &[(f32, f32)], x: f32) -> f32 {
    let n = points.len();
    match n {
        0 => return x,
        1 => return points[0].1,
        _ => {}
    }
    if x <= points[0].0 {
        return points[0].1;
    }
    if x >= points[n - 1].0 {
        return points[n - 1].1;
    }
    let slope = |i: usize| {
        let (a, b) = (points[i], points[i + 1]);
        if b.0 > a.0 {
            (b.1 - a.1) / (b.0 - a.0)
        } else {
            0.0
        }
    };
    // tangents: the mean of the neighbouring slopes, zero where the curve turns
    let tangent = |i: usize| {
        if i == 0 {
            return slope(0);
        }
        if i == n - 1 {
            return slope(n - 2);
        }
        let (a, b) = (slope(i - 1), slope(i));
        if a * b <= 0.0 {
            0.0
        } else {
            3.0 * a * b / (a + 2.0 * b).max(b + 2.0 * a)
        }
    };
    let i = points.windows(2).position(|w| x < w[1].0).unwrap_or(n - 2);
    let (a, b) = (points[i], points[i + 1]);
    let h = b.0 - a.0;
    if h <= 0.0 {
        return b.1;
    }
    let t = (x - a.0) / h;
    let (t2, t3) = (t * t, t * t * t);
    (2.0 * t3 - 3.0 * t2 + 1.0) * a.1
        + (t3 - 2.0 * t2 + t) * h * tangent(i)
        + (-2.0 * t3 + 3.0 * t2) * b.1
        + (t3 - t2) * h * tangent(i + 1)
}

filter_effect!(
    /// Its children brighter or darker (CSS `brightness()`): `amount` 1 unchanged, 0 black.
    Brightness("brightness", Region::Pad(0.05)) { amount: Anim<'a, f32> = Anim::Fixed(1.3) }
    themed(t)
    primitives(|this, ui| ColorMatrix::brightness(ui.sample(&this.amount)).primitive())
);

filter_effect!(
    /// Its children with more or less contrast (CSS `contrast()`): `amount` 1 unchanged, 0
    /// flat grey.
    Contrast("contrast", Region::Pad(0.05)) { amount: Anim<'a, f32> = Anim::Fixed(1.4) }
    themed(t)
    primitives(|this, ui| ColorMatrix::contrast(ui.sample(&this.amount)).primitive())
);

filter_effect!(
    /// Its children more or less saturated (SVG `saturate`): `amount` 1 unchanged, 0 grey.
    Saturation("saturation", Region::Pad(0.05)) { amount: Anim<'a, f32> = Anim::Fixed(1.6) }
    themed(t)
    primitives(|this, ui| ColorMatrix::saturation(ui.sample(&this.amount)).primitive())
);

filter_effect!(
    /// Its children with their hues turned round the colour wheel by `degrees` (SVG
    /// `hueRotate`), Remotion's hue effect.
    HueRotate("hue", Region::Pad(0.05)) { degrees: Anim<'a, f32> = Anim::Fixed(90.0) }
    themed(t)
    primitives(|this, ui| ColorMatrix::hue_rotate(ui.sample(&this.degrees)).primitive())
);

filter_effect!(
    /// Its children in grey (CSS `grayscale()`): `amount` 1 fully grey.
    Grayscale("grayscale", Region::Pad(0.05)) { amount: Anim<'a, f32> = Anim::Fixed(1.0) }
    themed(t)
    primitives(|this, ui| ColorMatrix::grayscale(ui.sample(&this.amount)).primitive())
);

filter_effect!(
    /// Its children as a negative (CSS `invert()`): `amount` 1 fully inverted.
    Invert("invert", Region::Pad(0.05)) { amount: Anim<'a, f32> = Anim::Fixed(1.0) }
    themed(t)
    primitives(|this, ui| ColorMatrix::invert(ui.sample(&this.amount)).primitive())
);

filter_effect!(
    /// Its children in shades of `color`, mixed with the original by `amount`.
    Tint("tint", Region::Pad(0.05)) {
        color: Color = t.colors.accent,
        amount: Anim<'a, f32> = Anim::Fixed(0.8),
    }
    themed(t)
    primitives(|this, ui| ColorMatrix::tint(this.color, ui.sample(&this.amount)).primitive())
);

filter_effect!(
    /// Its children in two colours: dark tones turn `dark` and light tones `light`, mixed
    /// with the original by `amount`.
    Duotone("duotone", Region::Pad(0.05)) {
        dark: Color = t.colors.background,
        light: Color = t.colors.accent,
        amount: Anim<'a, f32> = Anim::Fixed(1.0),
    }
    themed(t)
    primitives(|this, ui| {
        ColorMatrix::duotone(this.dark, this.light, ui.sample(&this.amount)).primitive()
    })
);

filter_effect!(
    /// Its children exposed brighter or darker by `stops`: every stop doubles the light.
    Exposure("exposure", Region::Pad(0.05)) { stops: Anim<'a, f32> = Anim::Fixed(0.5) }
    themed(t)
    primitives(|this, ui| ColorMatrix::exposure(ui.sample(&this.stops)).primitive())
);

filter_effect!(
    /// Its children warmer or colder: `temperature` from -1 (blue) to 1 (orange) and `tint`
    /// from -1 (green) to 1 (magenta).
    WhiteBalance("white-balance", Region::Pad(0.05)) {
        temperature: Anim<'a, f32> = Anim::Fixed(0.5),
        tint: f32 = 0.0,
    }
    themed(t)
    primitives(|this, ui| {
        ColorMatrix::white_balance(ui.sample(&this.temperature), this.tint).primitive()
    })
);

filter_effect!(
    /// Its children with gentler extra saturation than [`Saturation`]; see
    /// [`ColorMatrix::vibrance`] for how it approximates vibrance.
    Vibrance("vibrance", Region::Pad(0.05)) { amount: Anim<'a, f32> = Anim::Fixed(0.8) }
    themed(t)
    primitives(|this, ui| ColorMatrix::vibrance(ui.sample(&this.amount)).primitive())
);

filter_effect!(
    /// A basic colour correction in one matrix: `brightness`, then `contrast`, then
    /// `saturation` (1 leaves each unchanged), then the hues turned by `hue` degrees.
    ColorCorrection("color-correction", Region::Pad(0.05)) {
        brightness: f32 = 1.0,
        contrast: f32 = 1.1,
        saturation: f32 = 1.2,
        hue: f32 = 0.0,
    }
    themed(t)
    primitives(|this, _ui| {
        ColorMatrix::brightness(this.brightness)
            .then(ColorMatrix::contrast(this.contrast))
            .then(ColorMatrix::saturation(this.saturation))
            .then(ColorMatrix::hue_rotate(this.hue))
            .primitive()
    })
);

filter_effect!(
    /// Several colour matrices drawn as one: `matrices` in order, chained with
    /// [`ColorMatrix::then`] into one `feColorMatrix`, so the stack costs one filter. Nested
    /// colour effects clamp between steps and a stack does not; see `then`. The table
    /// effects ([`Levels`], [`Curves`], [`Lut`], [`ShadowsHighlights`], [`ThermalVision`],
    /// [`ColorKey`]) do not reduce to a matrix: nest them around or inside the stack.
    ///
    /// ```
    /// use fframes_kit::prelude::*;
    ///
    /// fn graded<'a>(ui: &mut Ui<'a, '_, '_>, picture: Children<'a>) -> Svgr<'a> {
    ///     ui.show(ColorStack {
    ///         matrices: vec![
    ///             ColorMatrix::exposure(0.3),
    ///             ColorMatrix::contrast(1.2),
    ///             ColorMatrix::white_balance(0.4, 0.0),
    ///         ],
    ///         children: picture,
    ///         ..ui.defaults()
    ///     })
    /// }
    /// ```
    ColorStack("color-stack", Region::Pad(0.05)) { matrices: Vec<ColorMatrix> = Vec::new() }
    themed(t)
    primitives(|this, _ui| {
        this.matrices
            .iter()
            .fold(ColorMatrix::IDENTITY, |m, next| m.then(*next))
            .primitive()
    })
);

filter_effect!(
    /// Photoshop's levels on every colour channel: the `input` black and white points
    /// stretch to 0 and 1, `gamma` bends the mid tones (above 1 brighter), and the result
    /// lands between the `output` black and white points.
    Levels("levels", Region::Pad(0.05)) {
        input: (f32, f32) = (0.08, 0.92),
        gamma: Anim<'a, f32> = Anim::Fixed(1.2),
        output: (f32, f32) = (0.0, 1.0),
    }
    themed(t)
    primitives(|this, ui| {
        let gamma = ui.sample(&this.gamma).max(0.01);
        let (lo, hi) = this.input;
        let (out_lo, out_hi) = this.output;
        let f = table(|v| {
            let x = if hi > lo { clamp01((v - lo) / (hi - lo)) } else { v };
            out_lo + (out_hi - out_lo) * x.powf(1.0 / gamma)
        });
        transfer(f.clone(), f.clone(), f)
    })
);

filter_effect!(
    /// Tone curves: a smooth curve through `points` (input, output, both 0 to 1, sorted by
    /// input) on every colour channel, then the optional per-channel curves `red`, `green`
    /// and `blue`. The default is a gentle S curve that adds contrast.
    Curves("curves", Region::Pad(0.05)) {
        points: &'a [(f32, f32)] = &[(0.0, 0.0), (0.25, 0.2), (0.75, 0.82), (1.0, 1.0)],
        red: Option<&'a [(f32, f32)]> = None,
        green: Option<&'a [(f32, f32)]> = None,
        blue: Option<&'a [(f32, f32)]> = None,
    }
    themed(t)
    primitives(|this, _ui| {
        let channel = |own: Option<&[(f32, f32)]>| {
            table(|v| {
                let v = curve(this.points, v);
                own.map_or(v, |own| curve(own, v))
            })
        };
        transfer(channel(this.red), channel(this.green), channel(this.blue))
    })
);

filter_effect!(
    /// A look-up table per colour channel: `red`, `green` and `blue` list the outputs for
    /// evenly spaced inputs from 0 to 1 (an empty list leaves the channel unchanged). An
    /// approximation of a LUT file: a `.cube` LUT maps colours in 3D, these tables map each
    /// channel on its own, so they hold curves and colour casts but not hue-dependent looks.
    Lut("lut", Region::Pad(0.05)) {
        red: &'a [f32] = &[0.05, 0.3, 0.62, 0.86, 1.0],
        green: &'a [f32] = &[0.02, 0.27, 0.56, 0.82, 0.97],
        blue: &'a [f32] = &[0.12, 0.3, 0.5, 0.72, 0.88],
    }
    themed(t)
    primitives(|this, _ui| {
        let values = |list: &[f32]| {
            if list.is_empty() {
                return "0 1".to_owned();
            }
            let parts: Vec<String> = list.iter().map(|v| format!("{}", clamp01(*v))).collect();
            parts.join(" ")
        };
        transfer(values(this.red), values(this.green), values(this.blue))
    })
);

filter_effect!(
    /// Shadows and highlights: `shadows` lifts the dark tones (up to a sixth at 1) and
    /// `highlights` pulls the light tones down, through a curve on every channel. An
    /// approximation: the real effect looks at each pixel's neighbourhood, this curve only at
    /// the pixel's own value.
    ShadowsHighlights("shadows-highlights", Region::Pad(0.05)) {
        shadows: Anim<'a, f32> = Anim::Fixed(0.6),
        highlights: f32 = 0.3,
    }
    themed(t)
    primitives(|this, ui| {
        let (s, h) = (ui.sample(&this.shadows), this.highlights);
        let f = table(|v| {
            // bumps that peak at a third and two thirds
            let low = 6.75 * v * (1.0 - v) * (1.0 - v);
            let high = 6.75 * v * v * (1.0 - v);
            v + (s * low - h * high) / 6.0
        });
        transfer(f.clone(), f.clone(), f)
    })
);

/// The colours of [`ThermalVision`] from cold to hot: black, indigo, purple, red, orange,
/// yellow and white.
pub const THERMAL_PALETTE: [Color; 7] = [
    Color::rgb(0, 0, 0),
    Color::rgb(26, 10, 94),
    Color::rgb(122, 31, 162),
    Color::rgb(224, 36, 94),
    Color::rgb(255, 122, 0),
    Color::rgb(255, 212, 0),
    Color::rgb(255, 255, 255),
];

filter_effect!(
    /// Its children as a thermal camera sees them: the luminance mapped onto `palette`, cold
    /// to hot, evenly spaced (the default is [`THERMAL_PALETTE`]).
    ThermalVision("thermal", Region::Pad(0.05)) { palette: &'a [Color] = &THERMAL_PALETTE }
    themed(t)
    primitives(|this, _ui| {
        let channel = |pick: fn(Color) -> u8| {
            let parts: Vec<String> = this
                .palette
                .iter()
                .map(|c| format!("{}", f32::from(pick(*c)) / 255.0))
                .collect();
            if parts.is_empty() { "0 1".to_owned() } else { parts.join(" ") }
        };
        let gray = ColorMatrix::grayscale(1.0).primitive();
        let heat = transfer(channel(|c| c.r), channel(|c| c.g), channel(|c| c.b));
        Svgr::from(vec![gray, heat])
    })
);

/// How much of `key`'s hue a colour has: 1 for `key` itself, 0 for greys, below 0 for the
/// opposite hues. `None` for a grey key, which has no hue to key on.
pub(crate) fn keyness(key: Color) -> Option<[f32; 3]> {
    let k = rgb(key);
    let mean = (k[0] + k[1] + k[2]) / 3.0;
    let d = k.map(|c| c - mean);
    let norm: f32 = (0..3).map(|i| d[i] * k[i]).sum();
    (norm > 1e-3).then(|| d.map(|c| c / norm))
}

filter_effect!(
    /// A chroma key: what has the hue of `color` (a green or blue screen) turns transparent.
    /// Colours whose share of that hue is within `tolerance` of the key's vanish, and
    /// `softness` is the width of the fade into opaque. Linear in the colour, so it works
    /// best on saturated screens; a grey `color` keys nothing.
    ColorKey("color-key", Region::Pad(0.05)) {
        color: Color = Color::rgb(0, 255, 0),
        tolerance: f32 = 0.4,
        softness: f32 = 0.2,
    }
    themed(t)
    primitives(|this, _ui| {
        match keyness(this.color) {
            // alpha = (1 - tolerance - keyness) / softness, then times the original alpha
            Some(d) => {
                let soft = this.softness.max(1e-3);
                let mut m = ColorMatrix::IDENTITY.0;
                for (i, w) in d.iter().enumerate() {
                    m[15 + i] = -w / soft;
                }
                m[18] = 0.0;
                m[19] = (1.0 - this.tolerance) / soft;
                let mask = fframes::svgr!(<feComposite in2="SourceAlpha" operator="in" />);
                Svgr::from(vec![ColorMatrix(m).primitive(), mask])
            }
            None => ColorMatrix::IDENTITY.primitive(),
        }
    })
);

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 4], b: [f32; 4]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
    }

    #[test]
    fn matrices_on_known_colours() {
        let red = [1.0, 0.0, 0.0, 1.0];
        let white = [1.0; 4];
        assert!(close(
            ColorMatrix::grayscale(1.0).apply(red),
            [0.213, 0.213, 0.213, 1.0]
        ));
        assert!(close(
            ColorMatrix::invert(1.0).apply(white),
            [0.0, 0.0, 0.0, 1.0]
        ));
        assert!(close(
            ColorMatrix::invert(0.5).apply(red),
            [0.5, 0.5, 0.5, 1.0]
        ));
        assert!(close(
            ColorMatrix::brightness(0.5).apply(white),
            [0.5, 0.5, 0.5, 1.0]
        ));
        assert!(close(
            ColorMatrix::contrast(0.0).apply(red),
            [0.5, 0.5, 0.5, 1.0]
        ));
        assert!(close(
            ColorMatrix::exposure(-1.0).apply(white),
            [0.5, 0.5, 0.5, 1.0]
        ));
        assert!(close(ColorMatrix::hue_rotate(0.0).apply(red), red));
        assert!(close(ColorMatrix::saturation(1.0).apply(red), red));
        let navy = Color::rgb(0, 0, 128);
        let duo = ColorMatrix::duotone(navy, Color::WHITE, 1.0);
        assert!(close(duo.apply(white), white));
        assert!(close(
            duo.apply([0.0, 0.0, 0.0, 1.0]),
            [0.0, 0.0, 128.0 / 255.0, 1.0]
        ));
        let tint = ColorMatrix::tint(Color::rgb(255, 0, 0), 1.0);
        assert!(close(tint.apply(white), red));
    }

    #[test]
    fn then_chains_like_applying_twice() {
        let a = ColorMatrix::hue_rotate(40.0);
        let b = ColorMatrix::contrast(0.8).then(ColorMatrix::tint(Color::rgb(0, 128, 255), 0.4));
        let colour = [0.3, 0.6, 0.2, 0.8];
        assert!(close(a.then(b).apply(colour), b.apply(a.apply(colour))));
        assert_eq!(ColorMatrix::IDENTITY.then(a), a);
    }

    #[test]
    fn curves_pass_through_their_points_without_overshoot() {
        let points = [(0.0, 0.0), (0.3, 0.1), (0.6, 0.9), (1.0, 1.0)];
        for (x, y) in points {
            assert!((curve(&points, x) - y).abs() < 1e-5);
        }
        let mut last = 0.0;
        for i in 0..=100 {
            let y = curve(&points, i as f32 / 100.0);
            assert!(y >= last - 1e-6 && (0.0..=1.0).contains(&y));
            last = y;
        }
        assert_eq!(curve(&[], 0.4), 0.4);
    }

    #[test]
    fn the_key_colour_is_transparent_and_others_stay() {
        let d = keyness(Color::rgb(0, 255, 0)).unwrap();
        let k = |c: [f32; 3]| (0..3).map(|i| d[i] * c[i]).sum::<f32>();
        assert!((k([0.0, 1.0, 0.0]) - 1.0).abs() < 1e-5);
        assert!(k([1.0, 1.0, 1.0]).abs() < 1e-5);
        assert!(k([1.0, 0.0, 0.0]) < 0.0);
        assert_eq!(keyness(Color::rgb(90, 90, 90)), None);
    }
}
