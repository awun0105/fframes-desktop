use fframes::{Color, Svgr};

use crate::{
    motion::{clamp01, lerp},
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Presentation, Side};

/// Lowest and highest values of the fractal noise the threshold runs through.
const NOISE_RANGE: (f32, f32) = (0.15, 0.85);
/// Noise distance over which the revealed edge goes from opaque to transparent.
const SOFTNESS: f32 = 0.05;

/// The exiting item burns away in a noise pattern with a glowing edge and the entering item
/// shows where it has burnt (Remotion's `dissolve()`). Remotion orders the burn by the
/// luminance of each pixel; the entering item is drawn on top and cannot see the exiting
/// pixels in SVG, so the kit orders it by a seeded `feTurbulence` noise instead.
///
/// `line_width` is the width of the glowing edge in noise units, `spread_color` its outer
/// colour and `hot_color` its core, as in Remotion. `pow` narrows the core to
/// `line_width / pow` and `intensity` is the edge's opacity (both shape the glow curve in
/// Remotion). `seed` and `frequency` pick the noise; the pattern stays put through the
/// transition. `size` is the area in view-box units, as for [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dissolve {
    pub line_width: f32,
    pub spread_color: Color,
    pub hot_color: Color,
    pub pow: f32,
    pub intensity: f32,
    pub seed: u32,
    pub frequency: f32,
    pub size: (f32, f32),
}

impl Default for Dissolve {
    /// Remotion's edge, a coarse noise.
    fn default() -> Self {
        Self {
            line_width: 0.1,
            spread_color: Color::rgb(0xff, 0x00, 0x00),
            hot_color: Color::rgb(0xe6, 0xe6, 0x33),
            pow: 5.0,
            intensity: 1.0,
            seed: 0,
            frequency: 0.004,
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for Dissolve {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl Dissolve {
    /// The noise value below which the entering item shows at `progress`: below the noise
    /// and its edge at 0, above the noise at 1.
    pub fn threshold(&self, progress: f32) -> f32 {
        let start = NOISE_RANGE.0 - self.line_width.max(0.0) - SOFTNESS;
        lerp(start, NOISE_RANGE.1, clamp01(progress))
    }
}

/// A colour matrix whose alpha is opaque where the red channel (the noise) is below `t` and
/// falls to 0 over `ramp` above it.
fn below(t: f32, ramp: f32) -> String {
    let k = 1.0 / ramp.max(SOFTNESS);
    format!(
        "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 {} 0 0 0 {}",
        -k,
        (t + ramp) * k
    )
}

impl Presentation for Dissolve {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a> {
        if side == Side::Exiting || progress >= 1.0 {
            return content;
        }
        let t = self.threshold(progress);
        let line = self.line_width.max(0.0);
        // the glow fades away from the burnt part, the core faster; both reach past the soft
        // edge of the reveal
        let reveal = below(t, SOFTNESS);
        let spread = below(t, line + SOFTNESS);
        let hot = below(t, line / self.pow.max(1.0) + SOFTNESS);
        let glow = clamp01(self.intensity);
        let (w, h) = self.size;
        let url = format!("url(#{id})");
        let id = id.to_owned();
        fframes::svgr!(
            <g>
                <defs>
                    <filter id={id} filterUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}
                            color-interpolation-filters="sRGB">
                        <feTurbulence type="fractalNoise" baseFrequency={self.frequency}
                                      numOctaves="3" seed={self.seed} result="noise" />
                        <feColorMatrix in="noise" type="matrix" values={reveal} result="reveal" />
                        <feComposite in="SourceGraphic" in2="reveal" operator="in" result="shown" />
                        <feColorMatrix in="noise" type="matrix" values={spread} result="outer" />
                        <feComposite in="outer" in2="reveal" operator="out" result="outer-band" />
                        <feFlood flood-color={self.spread_color} flood-opacity={glow} />
                        <feComposite in2="outer-band" operator="in" result="spread" />
                        <feColorMatrix in="noise" type="matrix" values={hot} result="inner" />
                        <feComposite in="inner" in2="reveal" operator="out" result="inner-band" />
                        <feFlood flood-color={self.hot_color} flood-opacity={glow} />
                        <feComposite in2="inner-band" operator="in" result="hot" />
                        <feMerge>
                            <feMergeNode in="shown" />
                            <feMergeNode in="spread" />
                            <feMergeNode in="hot" />
                        </feMerge>
                    </filter>
                </defs>
                <g filter={url}>{content}</g>
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_threshold_runs_through_the_whole_noise() {
        let d = Dissolve::default();
        // nothing and no edge shows at the start
        assert!(d.threshold(0.0) + d.line_width < NOISE_RANGE.0);
        assert_eq!(d.threshold(1.0), NOISE_RANGE.1);
        assert!(d.threshold(0.5) > d.threshold(0.4));
        assert_eq!(
            below(0.45, 0.05),
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 -20 0 0 0 10"
        );
    }
}
