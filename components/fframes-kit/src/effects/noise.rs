use fframes::Svgr;

use crate::{
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// Film grain over a `size` area from the top left: grey `feTurbulence` noise at `opacity`,
/// drawn by every backend. `frequency` is the noise's base frequency (higher is finer) and
/// `octaves` its detail. The same `seed` gives the same grain; with `animated` the seed moves
/// on every frame so the grain flickers like film. `key` names the filter when two noises
/// share a frame.
///
/// With the `skia` feature, [`ShaderNoise`] draws grain on the GPU instead.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Noise<'a> {
    pub seed: u32,
    pub opacity: f32,
    pub frequency: f32,
    pub octaves: u32,
    pub animated: bool,
    pub size: (f32, f32),
    pub key: Option<&'a str>,
}

impl Themed for Noise<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            seed: 0,
            opacity: 0.12,
            frequency: 0.8,
            octaves: 2,
            animated: true,
            size: (1920.0, 1080.0),
            key: None,
        }
    }
}

impl Default for Noise<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Noise<'_> {
    /// The turbulence seed at local frame `index`.
    pub fn seed_at(&self, index: usize) -> u32 {
        if self.animated {
            self.seed.wrapping_add(index as u32)
        } else {
            self.seed
        }
    }
}

impl<'a> Component<'a> for Noise<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let id = ui.id(self.key, 0, "noise");
        let url = format!("url(#{id})");
        let seed = self.seed_at(ui.frame.index);
        let (w, h) = self.size;
        fframes::svgr!(
            <g>
                <defs>
                    <filter id={id} x="0" y="0" width="100%" height="100%">
                        <feTurbulence type="fractalNoise" baseFrequency={self.frequency}
                                      numOctaves={self.octaves} seed={seed} stitchTiles="stitch" />
                        <feColorMatrix type="saturate" values="0" />
                    </filter>
                </defs>
                <rect x="0" y="0" width={w} height={h} filter={url} opacity={self.opacity} />
            </g>
        )
    }
}

#[cfg(feature = "skia")]
pub use shader::{GRAIN_SKSL, ShaderNoise, grain_shader};

#[cfg(feature = "skia")]
mod shader {
    use std::sync::OnceLock;

    use fframes::{Shader, ShaderUniforms, Svgr};

    use crate::{
        theme::Theme,
        ui::{Component, Themed, Ui},
    };

    /// The `SkSL` of [`ShaderNoise`]: a random grey per `uScale` pixel cell, from `uSeed`.
    pub const GRAIN_SKSL: &str = r"
uniform float uSeed;
uniform float uScale;
uniform float uOpacity;

float hash(float2 p) {
    p = fract(p * float2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

half4 main(float2 coord) {
    float2 cell = floor(coord / uScale);
    float v = hash(cell + uSeed * float2(17.0, 31.0));
    return half4(half3(v), 1.0) * half(uOpacity);
}
";

    /// The grain shader, built once; the renderer compiles and caches it by its id.
    pub fn grain_shader() -> &'static Shader {
        static SHADER: OnceLock<Shader> = OnceLock::new();
        SHADER.get_or_init(|| Shader::sksl(GRAIN_SKSL))
    }

    /// Film grain on the GPU: a grey value per `scale` pixel cell at `opacity`, over a `size`
    /// area from the top left. The same `seed` gives the same grain; with `animated` it moves
    /// on every frame. Only the Skia backend runs shaders: the CPU backend draws nothing and
    /// the editor a placeholder, so use [`Noise`](super::Noise) where those matter.
    #[derive(Debug, Clone, PartialEq)]
    #[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
    pub struct ShaderNoise {
        pub seed: u32,
        pub opacity: f32,
        pub scale: f32,
        pub animated: bool,
        pub size: (f32, f32),
    }

    impl Themed for ShaderNoise {
        fn themed(_: &Theme) -> Self {
            Self {
                seed: 0,
                opacity: 0.12,
                scale: 2.0,
                animated: true,
                size: (1920.0, 1080.0),
            }
        }
    }

    impl Default for ShaderNoise {
        fn default() -> Self {
            Self::themed(&Theme::default())
        }
    }

    impl<'a> Component<'a> for ShaderNoise {
        fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
            let seed = if self.animated {
                self.seed.wrapping_add(ui.frame.index as u32)
            } else {
                self.seed
            };
            // small enough that the hash keeps its precision in f32
            let seed = (seed % 4096) as f32;
            let layer = grain_shader().draw(
                &ui.frame,
                ShaderUniforms::new()
                    .float("uSeed", seed)
                    .float("uScale", self.scale.max(1.0))
                    .float("uOpacity", self.opacity),
            );
            let (w, h) = self.size;
            fframes::svgr!(<image href={layer.href()} x="0" y="0" width={w} height={h} />)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grain_moves_only_when_animated() {
        let noise = Noise {
            seed: 7,
            ..Noise::default()
        };
        assert_eq!(noise.seed_at(0), 7);
        assert_ne!(noise.seed_at(1), noise.seed_at(2));
        let still = Noise {
            animated: false,
            ..noise
        };
        assert_eq!(still.seed_at(3), still.seed_at(40));
    }
}
