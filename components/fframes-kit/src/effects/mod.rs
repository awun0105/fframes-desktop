//! Effects over or around other content: [`Trail`] and [`CameraMotionBlur`] (Remotion's
//! motion blur), [`LightLeak`], [`Blur`] and film grain with [`Noise`] (and [`ShaderNoise`]
//! on the GPU with the `skia` feature). The glow and drop shadow filters are
//! [`Glow`](crate::decor::Glow) and [`DropShadow`](crate::decor::DropShadow).
//!
//! The colour effects ([`Brightness`] to [`Lut`]) are SVG filters around their children;
//! [`ColorStack`] composes several colour matrices into one filter. [`Outline`],
//! [`Emboss`], [`BevelAlpha`], [`LongShadows`], [`RoughenEdges`] and [`Shrinkwrap`] work on
//! the alpha edges of their children. [`WhiteNoise`] and [`FractalNoise`] draw noise over
//! them. [`Scanlines`] to [`Vignette`] lay patterns and gradients over them, [`Halftone`] and
//! [`HalftoneLinearGradient`] print them as dots, [`Starburst`] draws rays and [`LightTrail`]
//! streaks light behind them; [`Pattern`] tiles them and [`Paper`], [`Burlap`], [`Flannel`]
//! and [`Speckle`] texture them. [`Mirror`], [`Tile`], [`Skew`], [`Scale`] and [`Translate`]
//! move them; [`VenetianBlinds`], [`Evolve`] and [`TvSignalOff`] reveal or replace them.
//! [`RegionBlur`], [`LinearProgressiveBlur`] and [`RadialProgressiveBlur`] blur part of them.

use fframes::Svgr;

/// Where a filter draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Region {
    /// The content's box grown by this share of the box on every side.
    Pad(f32),
    /// A `w` by `h` area from the origin, in the user units of the content.
    Area(f32, f32),
}

/// `content` through a filter `id` made of `primitives`, in sRGB like every kit filter.
pub(crate) fn filtered<'a>(
    content: Svgr<'a>,
    id: String,
    region: Region,
    primitives: Svgr<'a>,
) -> Svgr<'a> {
    let url = format!("url(#{id})");
    let filter = match region {
        Region::Pad(pad) => {
            let start = format!("{}%", -pad * 100.0);
            let size = format!("{}%", (1.0 + 2.0 * pad) * 100.0);
            fframes::svgr!(
                <filter id={id} x={start.clone()} y={start} width={size.clone()} height={size}
                        color-interpolation-filters="sRGB">
                    {primitives}
                </filter>
            )
        }
        Region::Area(w, h) => fframes::svgr!(
            <filter id={id} filterUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}
                    color-interpolation-filters="sRGB">
                {primitives}
            </filter>
        ),
    };
    fframes::svgr!(
        <g>
            <defs>{filter}</defs>
            <g filter={url}>{content}</g>
        </g>
    )
}

/// `overlay` drawn only where `shape` is opaque, through a mask `id` over a `w` by `h` area
/// from the origin. The mask draws `shape` white with its own alpha, so its colours do not
/// matter.
pub(crate) fn alpha_masked<'a>(
    overlay: Svgr<'a>,
    shape: Svgr<'a>,
    id: &str,
    (w, h): (f32, f32),
) -> Svgr<'a> {
    let white = format!("{id}-white");
    let white_url = format!("url(#{white})");
    let url = format!("url(#{id})");
    let id = id.to_owned();
    fframes::svgr!(
        <g>
            <defs>
                <filter id={white} color-interpolation-filters="sRGB">
                    <feColorMatrix type="matrix" values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 1 0" />
                </filter>
                <mask id={id} maskUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}>
                    <g filter={white_url}>{shape}</g>
                </mask>
            </defs>
            <g mask={url}>{overlay}</g>
        </g>
    )
}

/// A 3 by 3 `kernelMatrix` that measures how fast its input rises along `dir`, times `gain`
/// (a unit rise per pixel along a unit `dir` gives `6 * gain`). The SVG kernel is mirrored,
/// so the entry in column `c`, row `r` weighs the pixel at offset `(1 - c, 1 - r)`.
pub(crate) fn slope_kernel(dir: (f32, f32), gain: f32) -> String {
    let mut parts = Vec::with_capacity(9);
    for r in 0..3 {
        for c in 0..3 {
            let (ox, oy) = ((1 - c) as f32, (1 - r) as f32);
            parts.push(format!("{}", gain * (dir.0 * ox + dir.1 * oy)));
        }
    }
    parts.join(" ")
}

/// A wrapper effect drawn by one filter: the struct with its fields, `key` and `children`,
/// its theme defaults, and a `Component` that draws the children through the filter whose
/// primitives `$primitives` builds (from `&self` as `$this` and the `Ui` as `$ui`).
macro_rules! filter_effect {
    (
        $(#[$meta:meta])*
        $name:ident($kind:literal, $region:expr) {
            $($(#[$fmeta:meta])* $field:ident: $ty:ty = $default:expr),* $(,)?
        }
        themed($theme:ident)
        primitives(|$this:ident, $ui:ident| $primitives:expr)
    ) => {
        $(#[$meta])*
        #[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
        pub struct $name<'a> {
            $($(#[$fmeta])* pub $field: $ty,)*
            /// Names the filter when two effects of this kind share a frame.
            pub key: Option<&'a str>,
            pub children: $crate::ui::Children<'a>,
        }

        impl $crate::ui::Themed for $name<'_> {
            #[allow(unused_variables)]
            fn themed($theme: &$crate::theme::Theme) -> Self {
                Self {
                    $($field: $default,)*
                    key: None,
                    children: $crate::ui::no_children(),
                }
            }
        }

        impl<'a> $crate::ui::Component<'a> for $name<'a> {
            fn render(self, ui: &mut $crate::ui::Ui<'a, '_, '_>) -> fframes::Svgr<'a> {
                let primitives = {
                    let $this = &self;
                    let $ui = &*ui;
                    $primitives
                };
                let content = (self.children)(ui);
                let id = ui.id(self.key, 0, $kind);
                $crate::effects::filtered(content, id, $region, primitives)
            }
        }
    };
}

mod blur;
mod color;
mod distort;
mod edges;
mod geometry;
mod halftone;
mod light_leak;
mod motion_blur;
mod noise;
mod patterns;
mod progressive_blur;
mod reveal;
mod textures;

pub use blur::Blur;
pub use color::{
    Brightness, ColorCorrection, ColorKey, ColorMatrix, ColorStack, Contrast, Curves, Duotone,
    Exposure, Grayscale, HueRotate, Invert, Levels, Lut, Saturation, ShadowsHighlights,
    THERMAL_PALETTE, ThermalVision, Tint, Vibrance, WhiteBalance,
};
pub use distort::{FractalNoise, FractalType, WhiteNoise};
pub use edges::{BevelAlpha, Emboss, LongShadows, Outline, RoughenEdges, Shrinkwrap};
pub use geometry::{Mirror, MirrorDirection, Scale, Skew, Tile, Translate};
pub use halftone::{Halftone, HalftoneColor, HalftoneLinearGradient, HalftoneShape};
pub use light_leak::{Flare, LightLeak, LightTrail, MAX_TRAIL_SAMPLES, Starburst};
pub use motion_blur::{CameraMotionBlur, MAX_SAMPLES, Trail};
pub use noise::Noise;
#[cfg(feature = "skia")]
pub use noise::{GRAIN_SKSL, ShaderNoise, grain_shader};
pub use patterns::{
    Checkerboard, DotGrid, Gridlines, LineDirection, LinearGradient, LinearGradientTint, Lines,
    MAX_PATTERN_SHAPES, MAX_RINGS, Rings, Scanlines, Vignette, VignetteMode,
};
pub use progressive_blur::{
    LinearProgressiveBlur, MAX_BLUR_LEVELS, RadialProgressiveBlur, RegionBlur, blur_levels,
};
pub use reveal::{BlindsDirection, Evolve, EvolveDirection, TvSignalOff, VenetianBlinds};
pub use textures::{Burlap, Flannel, MAX_TILES, Paper, Pattern, Speckle};

#[cfg(test)]
mod tests {
    /// The id kind of every effect: the literal passed to `ui.id` or to `filter_effect!`.
    fn kinds(source: &str) -> Vec<&str> {
        source
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if let Some(call) = line.split("ui.id(").nth(1) {
                    let literal = call.rsplit('"').nth(1)?;
                    return (!literal.contains(' ')).then_some(literal);
                }
                // `Brightness("brightness", Region::Pad(0.05)) {`
                let (name, rest) = line.split_once("(\"")?;
                let pascal = name.starts_with(|c: char| c.is_ascii_uppercase())
                    && name.chars().all(|c| c.is_ascii_alphanumeric());
                (pascal && rest.contains("\", Region::")).then(|| rest.split('"').next())?
            })
            .collect()
    }

    #[test]
    fn every_effect_names_its_filters_with_its_own_kind() {
        let sources = [
            include_str!("blur.rs"),
            include_str!("color.rs"),
            include_str!("distort.rs"),
            include_str!("edges.rs"),
            include_str!("geometry.rs"),
            include_str!("halftone.rs"),
            include_str!("light_leak.rs"),
            include_str!("motion_blur.rs"),
            include_str!("noise.rs"),
            include_str!("patterns.rs"),
            include_str!("progressive_blur.rs"),
            include_str!("reveal.rs"),
            include_str!("textures.rs"),
        ];
        let mut all: Vec<&str> = sources.iter().flat_map(|s| kinds(s)).collect();
        assert!(all.len() >= 59, "found only {} kinds", all.len());
        all.sort_unstable();
        let repeated: Vec<_> = all
            .windows(2)
            .filter(|w| w[0] == w[1])
            .map(|w| w[0])
            .collect();
        // two effects with one key would share a filter id
        assert!(repeated.is_empty(), "kinds used twice: {repeated:?}");
    }
}
