//! Effects over or around other content: [`Trail`] and [`CameraMotionBlur`] (Remotion's
//! motion blur), [`LightLeak`], [`Blur`] and film grain with [`Noise`] (and [`ShaderNoise`]
//! on the GPU with the `skia` feature). The glow and drop shadow filters are
//! [`Glow`](crate::decor::Glow) and [`DropShadow`](crate::decor::DropShadow).
//!
//! The colour effects ([`Brightness`] to [`Lut`]) are SVG filters around their children;
//! [`ColorStack`] composes several colour matrices into one filter.

use fframes::Svgr;

/// `content` through a filter `id` made of `primitives`, in sRGB like every kit filter. The
/// filter region is the content's box grown by `pad` (a share of the box) on every side.
pub(crate) fn filtered<'a>(
    content: Svgr<'a>,
    id: String,
    pad: f32,
    primitives: Svgr<'a>,
) -> Svgr<'a> {
    let url = format!("url(#{id})");
    let start = format!("{}%", -pad * 100.0);
    let size = format!("{}%", (1.0 + 2.0 * pad) * 100.0);
    fframes::svgr!(
        <g>
            <defs>
                <filter id={id} x={start.clone()} y={start} width={size.clone()} height={size}
                        color-interpolation-filters="sRGB">
                    {primitives}
                </filter>
            </defs>
            <g filter={url}>{content}</g>
        </g>
    )
}

/// A wrapper effect drawn by one filter: the struct with its fields, `key` and `children`,
/// its theme defaults, and a `Component` that draws the children through the filter whose
/// primitives `$primitives` builds (from `&self` as `$this` and the `Ui` as `$ui`).
macro_rules! filter_effect {
    (
        $(#[$meta:meta])*
        $name:ident($kind:literal, pad $pad:expr) {
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
                $crate::effects::filtered(content, id, $pad, primitives)
            }
        }
    };
}

mod blur;
mod color;
mod light_leak;
mod motion_blur;
mod noise;

pub use blur::Blur;
pub use color::{
    Brightness, ColorCorrection, ColorKey, ColorMatrix, ColorStack, Contrast, Curves, Duotone,
    Exposure, Grayscale, HueRotate, Invert, Levels, Lut, Saturation, ShadowsHighlights,
    THERMAL_PALETTE, ThermalVision, Tint, Vibrance, WhiteBalance,
};
pub use light_leak::{Flare, LightLeak};
pub use motion_blur::{CameraMotionBlur, MAX_SAMPLES, Trail};
pub use noise::Noise;
#[cfg(feature = "skia")]
pub use noise::{GRAIN_SKSL, ShaderNoise, grain_shader};
