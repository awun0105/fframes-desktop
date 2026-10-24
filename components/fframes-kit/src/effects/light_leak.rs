use std::f32::consts::PI;

use fframes::{Color, Svgr};

use crate::{
    motion::{Rng, clamp01},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// Hues of the blobs before `hue_shift`, in degrees: orange, red, amber and pink, like light
/// leaking into a film camera.
const HUES: [f32; 4] = [28.0, 8.0, 45.0, 345.0];

/// Light leaking into the camera, like Remotion's `<LightLeak>`: warm blobs of light flare up
/// over the picture and fade out again within `length` seconds (the enclosing container's
/// length when `None`), brightest at `opacity` in the middle. `seed` picks where the blobs are
/// and how they drift, and `hue_shift` turns their colours round the colour wheel in degrees
/// (180 for blue light). Drawn with radial gradients in screen blending, so it works on every
/// backend; `size` is the area from the top left and `key` names the gradients when two leaks
/// share a frame.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LightLeak<'a> {
    pub seed: u64,
    pub hue_shift: f32,
    pub length: Option<f32>,
    pub opacity: f32,
    pub size: (f32, f32),
    pub key: Option<&'a str>,
}

impl Themed for LightLeak<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            seed: 0,
            hue_shift: 0.0,
            length: None,
            opacity: 0.85,
            size: (1920.0, 1080.0),
            key: None,
        }
    }
}

impl Default for LightLeak<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// One blob of light at a moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flare {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub color: Color,
    pub opacity: f32,
}

impl LightLeak<'_> {
    /// The blobs at `progress` through the leak, 0 to 1. Each flares up and fades along a
    /// half sine from its own start, so they peak one after another.
    pub fn flares(&self, progress: f32) -> Vec<Flare> {
        let (w, h) = (f64::from(self.size.0), f64::from(self.size.1));
        let p = clamp01(progress);
        let mut rng = Rng::new(self.seed);
        HUES.iter()
            .map(|&hue| {
                // anchored past the top or bottom edge, drifting a little
                let x = rng.range(-0.15, 1.15) * w;
                let y = if rng.unit() < 0.5 {
                    rng.range(-0.15, 0.2) * h
                } else {
                    rng.range(0.8, 1.15) * h
                };
                let (dx, dy) = (rng.range(-0.2, 0.2) * w, rng.range(-0.1, 0.1) * h);
                let base = rng.range(0.45, 0.8) * h;
                let start = rng.range(0.0, 0.3) as f32;
                let jitter = rng.range(-8.0, 8.0) as f32;
                let local = clamp01((p - start) / (1.0 - start));
                let glow = (local * PI).sin().max(0.0);
                Flare {
                    x: (x + dx * f64::from(p)) as f32,
                    y: (y + dy * f64::from(p)) as f32,
                    radius: base as f32 * (0.5 + 0.7 * glow),
                    color: hsl(hue + self.hue_shift + jitter, 1.0, 0.6),
                    opacity: self.opacity * glow,
                }
            })
            .collect()
    }
}

/// A colour from hue in degrees, saturation and lightness in `0..=1`.
fn hsl(hue: f32, s: f32, l: f32) -> Color {
    let h = hue.rem_euclid(360.0) / 60.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let byte = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::rgb(byte(r), byte(g), byte(b))
}

impl<'a> Component<'a> for LightLeak<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(length) = self.length.or_else(|| ui.duration_seconds()) else {
            return Svgr::empty();
        };
        if length <= 0.0 {
            return Svgr::empty();
        }
        let flares: Svgr<'a> = self
            .flares(ui.seconds() / length)
            .into_iter()
            .enumerate()
            .filter(|(_, f)| f.opacity > 0.0 && f.radius > 0.0)
            .map(|(i, f)| {
                let id = ui.id(self.key, i, "light-leak");
                let fill = format!("url(#{id})");
                let color = f.color;
                fframes::svgr!(
                    <g>
                        <defs>
                            <radialGradient id={id} cx="0.5" cy="0.5" r="0.5">
                                <stop offset="0" stop-color={color} stop-opacity="1" />
                                <stop offset="0.45" stop-color={color} stop-opacity="0.55" />
                                <stop offset="1" stop-color={color} stop-opacity="0" />
                            </radialGradient>
                        </defs>
                        <circle cx={f.x} cy={f.y} r={f.radius} fill={fill} opacity={f.opacity} />
                    </g>
                )
            })
            .collect();
        fframes::svgr!(<g mix-blend-mode="screen">{flares}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_leak_flares_up_and_fades_out() {
        let leak = LightLeak::default();
        let total = |p: f32| leak.flares(p).iter().map(|f| f.opacity).sum::<f32>();
        assert_eq!(total(0.0), 0.0);
        assert!(total(0.5) > 1.0);
        assert!(total(1.0) < 1e-3);
        // the same seed gives the same light, another seed other light
        assert_eq!(leak.flares(0.4), LightLeak::default().flares(0.4));
        let other = LightLeak {
            seed: 9,
            ..LightLeak::default()
        };
        assert_ne!(other.flares(0.4), leak.flares(0.4));
    }

    #[test]
    fn hue_shift_turns_the_colours() {
        assert_eq!(hsl(0.0, 1.0, 0.5), Color::rgb(255, 0, 0));
        assert_eq!(hsl(120.0, 1.0, 0.5), Color::rgb(0, 255, 0));
        assert_eq!(hsl(-120.0, 1.0, 0.5), Color::rgb(0, 0, 255));
        let warm = LightLeak::default().flares(0.5);
        let cold = LightLeak {
            hue_shift: 180.0,
            ..LightLeak::default()
        }
        .flares(0.5);
        // orange light turns blue
        assert!(warm[0].color.r > warm[0].color.b && cold[0].color.b > cold[0].color.r);
    }
}
