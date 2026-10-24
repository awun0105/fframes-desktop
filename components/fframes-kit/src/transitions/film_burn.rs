use fframes::Svgr;

use crate::{
    motion::{clamp01, hash, lerp},
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{
    Presentation, Side,
    presentation::{bell, blurred},
};

/// A burning film overlay flares up while the items cross-fade under a light blur
/// (Remotion's `filmBurn()`, after Anastasia Dunbar's GL transition). An approximation: a
/// warm radial gradient that grows from a point picked by `seed`, broken up by
/// `feTurbulence` noise of the same seed, peaks over the middle of the transition. `blur`
/// is the blur's standard deviation at the middle, in view-box units; Remotion's radial
/// blur has no prop, it is the kit's. `size` is the area in view-box units, as for
/// [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilmBurn {
    pub seed: f32,
    pub blur: f32,
    pub size: (f32, f32),
}

impl Default for FilmBurn {
    /// Remotion's seed.
    fn default() -> Self {
        Self {
            seed: 2.31,
            blur: 6.0,
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for FilmBurn {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl FilmBurn {
    /// The centre of the burn as a share of `size`, from `seed`, drifting with `progress`.
    pub fn origin(&self, progress: f32) -> (f32, f32) {
        let s = (self.seed * 1000.0) as u32;
        let x = lerp(0.15, 0.55, hash(s));
        let y = lerp(0.2, 0.8, hash(s.wrapping_add(1)));
        (x + 0.3 * clamp01(progress), y)
    }

    /// The opacity of the entering item: it takes over behind the burn.
    pub fn mix(progress: f32) -> f32 {
        let t = clamp01((clamp01(progress) - 0.3) / 0.4);
        t * t * (3.0 - 2.0 * t)
    }
}

impl Presentation for FilmBurn {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a> {
        let flare = bell(progress);
        let content = blurred(
            content,
            self.blur * flare,
            self.blur * flare,
            self.size,
            id,
            side,
        );
        if side == Side::Exiting {
            return content;
        }
        let opacity = Self::mix(progress);
        if flare <= 0.0 {
            return fframes::svgr!(<g opacity={opacity}>{content}</g>);
        }
        let (w, h) = self.size;
        let (ox, oy) = self.origin(progress);
        let (cx, cy) = (ox * w, oy * h);
        let r = w * lerp(0.2, 1.1, flare);
        // the noise eats into the burn, less of it the hotter the burn is
        let alpha = format!(
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 3 0 0 0 {}",
            3.0 * (flare - 0.9)
        );
        let seed = (self.seed * 100.0) as u32;
        let gradient = format!("{id}-gradient");
        let fill = format!("url(#{gradient})");
        let noise = format!("{id}-noise");
        let filter = format!("url(#{noise})");
        fframes::svgr!(
            <g>
                <g opacity={opacity}>{content}</g>
                <defs>
                    <radialGradient id={gradient} gradientUnits="userSpaceOnUse" cx={cx} cy={cy} r={r}>
                        <stop offset="0" stop-color="#fff6d8" />
                        <stop offset="0.3" stop-color="#ffb347" />
                        <stop offset="0.65" stop-color="#e2401c" />
                        <stop offset="1" stop-color="#7a0f05" stop-opacity="0" />
                    </radialGradient>
                    <filter id={noise} filterUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}
                            color-interpolation-filters="sRGB">
                        <feTurbulence type="fractalNoise" baseFrequency="0.006" numOctaves="2"
                                      seed={seed} result="noise" />
                        <feColorMatrix in="noise" type="matrix" values={alpha} result="holes" />
                        <feComposite in="SourceGraphic" in2="holes" operator="in" />
                    </filter>
                </defs>
                <rect x="0" y="0" width={w} height={h} fill={fill} filter={filter} />
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burns_in_the_middle_from_a_seeded_point() {
        let burn = FilmBurn::default();
        assert_eq!(burn.origin(0.0), burn.origin(0.0));
        let other = FilmBurn { seed: 7.0, ..burn };
        assert_ne!(burn.origin(0.0), other.origin(0.0));
        assert_eq!(FilmBurn::mix(0.0), 0.0);
        assert!((FilmBurn::mix(0.5) - 0.5).abs() < 1e-6);
        assert_eq!(FilmBurn::mix(1.0), 1.0);
    }
}
