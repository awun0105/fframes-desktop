use std::{collections::BTreeMap, f32::consts::PI, fmt::Write};

use fframes::{Color, Svgr, animation::Animatable};

use super::{Region, edges::shadow_steps, filtered};
use crate::{
    anim::Anim,
    motion::{Rng, clamp01},
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
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

/// The most samples a [`LightTrail`] weighs, like Remotion.
pub const MAX_TRAIL_SAMPLES: u32 = 64;

/// A streak of light trailing its children (Remotion's `lightTrail()`): copies of their
/// silhouette in `color`, `distance` units long towards `direction` (degrees clockwise from
/// the right, 180 to the left), each copy `decay` (0 to 1) as bright as the one before,
/// added to the children at `intensity`. `samples` (1 to 64) sets how Remotion spaces the
/// copies, so the falloff and brightness follow it; `threshold` (0 to 1) leaves out pixels
/// less opaque than it. Drawn inside the `area` from the origin.
///
/// Remotion also lets bright semi-transparent pixels through the threshold; here only the
/// alpha counts. The copies are averaged by doubling, like [`LongShadows`](super::LongShadows):
/// at most 256 copies for the cost of sixteen primitives.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LightTrail<'a> {
    pub direction: f32,
    pub distance: Anim<'a, f32>,
    pub intensity: f32,
    pub decay: f32,
    pub threshold: f32,
    pub samples: u32,
    pub color: Color,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for LightTrail<'_> {
    /// Remotion's defaults: a white trail 80 units to the left.
    fn themed(_: &Theme) -> Self {
        Self {
            direction: 180.0,
            distance: Anim::Fixed(80.0),
            intensity: 1.0,
            decay: 0.9,
            threshold: 0.0,
            samples: 32,
            color: Color::WHITE,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl LightTrail<'_> {
    /// How bright the trail of a solid silhouette is, before the colour's alpha: Remotion's
    /// sum of the sample weights over their count, times `intensity`.
    pub fn gain(&self) -> f32 {
        let samples = self.samples.clamp(1, MAX_TRAIL_SAMPLES);
        let decay = clamp01(self.decay);
        let sum: f32 = (0..samples as i32).map(|i| decay.powi(i)).sum();
        self.intensity.max(0.0) * sum / samples as f32
    }
}

impl<'a> Component<'a> for LightTrail<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let distance = ui.sample(&self.distance).max(0.0);
        let content = (self.children)(ui);
        let gain = self.gain() * f32::from(self.color.a) / 255.0;
        if distance <= 0.0 || gain <= 0.0 {
            return content;
        }
        let samples = self.samples.clamp(1, MAX_TRAIL_SAMPLES) as f32;
        let (sin, cos) = self.direction.to_radians().sin_cos();
        let first = distance / samples;
        let mut primitives = Vec::new();
        let threshold = clamp01(self.threshold).min(0.999);
        let mut source = "SourceAlpha";
        if threshold > 0.0 {
            let slope = 1.0 / (1.0 - threshold);
            primitives.push(fframes::svgr!(
                <feComponentTransfer in="SourceAlpha" result="lt-passing">
                    <feFuncA type="linear" slope={slope} intercept={-threshold * slope} />
                </feComponentTransfer>
            ));
            primitives.push(fframes::svgr!(
                <feComposite in="SourceAlpha" in2="lt-passing" operator="arithmetic"
                             k1="1" k2="0" k3="0" k4="0" result="lt-lit" />
            ));
            source = "lt-lit";
        }
        primitives.push(fframes::svgr!(
            <feOffset in={source} dx={first * cos} dy={first * sin} result="lt-start" />
        ));
        let mut last = "lt-start".to_owned();
        // each doubling averages the trail so far with itself shifted, the copy weighed by
        // the decay over the shift, so the average never clips
        let (doublings, step) = shadow_steps(distance - first);
        let falloff = clamp01(self.decay).powf(step * samples / distance);
        for i in 0..doublings {
            let span = (1u32 << i) as f32;
            let weight = falloff.powf(span);
            let moved = format!("lt-moved-{i}");
            let merged = format!("lt-{i}");
            primitives.push(fframes::svgr!(
                <feOffset in={last.clone()} dx={step * span * cos} dy={step * span * sin} result={moved.clone()} />
            ));
            primitives.push(fframes::svgr!(
                <feComposite in={last} in2={moved} operator="arithmetic" result={merged.clone()}
                             k1="0" k2={1.0 / (1.0 + weight)} k3={weight / (1.0 + weight)} k4="0" />
            ));
            last = merged;
        }
        let color = self.color.with_alpha(255);
        primitives.push(fframes::svgr!(<feFlood flood-color={color} />));
        primitives.push(fframes::svgr!(<feComposite in2={last} operator="in" result="lt-trail" />));
        primitives.push(fframes::svgr!(
            <feComposite in="SourceGraphic" in2="lt-trail" operator="arithmetic"
                         k1="0" k2="1" k3={gain} k4="0" />
        ));
        let id = ui.id(self.key, 0, "light-trail");
        let (w, h) = self.area;
        filtered(content, id, Region::Area(w, h), Svgr::from(primitives))
    }
}

/// The steps a [`Starburst`] blends neighbouring rays over.
const BLEND_STEPS: usize = 12;

/// Rays of colour from a point (Remotion's `starburst()`): `rays` (2 to 100) wedges round
/// `origin` (0 to 1 across the area), coloured with `colors` in turn and turned `rotation`
/// degrees clockwise; the first ray starts on the left. `smoothness` (0 to 1) blends each
/// ray into the next over that share of a ray. It replaces the layer, so it takes no
/// children.
///
/// Remotion blends from both sides of a boundary towards the colour across it, which swaps
/// the two colours at the boundary itself; here the blend runs smoothly across it, in twelve
/// steps.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Starburst<'a> {
    pub rays: u32,
    pub colors: Vec<Color>,
    pub rotation: f32,
    pub smoothness: f32,
    pub origin: (f32, f32),
    pub area: (f32, f32),
    pub key: Option<&'a str>,
}

impl Themed for Starburst<'_> {
    fn themed(theme: &Theme) -> Self {
        Self {
            rays: 12,
            colors: vec![theme.colors.accent, theme.colors.background],
            rotation: 0.0,
            smoothness: 0.0,
            origin: (0.5, 0.5),
            area: (1920.0, 1080.0),
            key: None,
        }
    }
}

impl Starburst<'_> {
    /// The rays as `(start, end, colour index)`, in degrees clockwise from the right.
    pub fn wedges(&self) -> Vec<(f32, f32, usize)> {
        if self.colors.is_empty() {
            return Vec::new();
        }
        let rays = self.rays.clamp(2, 100);
        let sector = 360.0 / rays as f32;
        (0..rays as usize)
            .map(|k| {
                let start = 180.0 + self.rotation + k as f32 * sector;
                (start, start + sector, k % self.colors.len())
            })
            .collect()
    }
}

impl<'a> Component<'a> for Starburst<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let wedges = self.wedges();
        let Some(&(first, last, _)) = wedges.first() else {
            return Svgr::empty();
        };
        let sector = last - first;
        let (w, h) = self.area;
        let (ox, oy) = (self.origin.0 * w, self.origin.1 * h);
        let far = [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)]
            .iter()
            .map(|&(x, y): &(f32, f32)| (x - ox).hypot(y - oy))
            .fold(0.0, f32::max)
            + 2.0;
        let wedge = |d: &mut String, from: f32, to: f32| {
            let (s0, c0) = from.to_radians().sin_cos();
            let (s1, c1) = to.to_radians().sin_cos();
            let _ = write!(
                d,
                "M{ox} {oy}L{} {}A{far} {far} 0 0 1 {} {}Z",
                ox + far * c0,
                oy + far * s0,
                ox + far * c1,
                oy + far * s1
            );
        };
        let mut hard = vec![String::new(); self.colors.len()];
        for &(from, to, color) in &wedges {
            wedge(&mut hard[color], from, to);
        }
        let mut paths: Vec<Svgr<'a>> = hard
            .into_iter()
            .zip(&self.colors)
            .filter(|(d, _)| !d.is_empty())
            .map(|(d, &fill)| fframes::svgr!(<path d={d} fill={fill} />))
            .collect();
        // the blend across each boundary, in steps that overlap the next a little
        let half = clamp01(self.smoothness) * sector / 2.0;
        if half > 0.0 {
            let width = 2.0 * half / BLEND_STEPS as f32;
            let mut steps: BTreeMap<(usize, usize), String> = BTreeMap::new();
            for &(_, to, color) in &wedges {
                for j in 0..BLEND_STEPS {
                    let from = to - half + j as f32 * width;
                    wedge(
                        steps.entry((j, color)).or_default(),
                        from,
                        from + width * 1.25,
                    );
                }
            }
            for ((j, color), d) in steps {
                let next = self.colors[(color + 1) % self.colors.len()];
                let t = (j as f32 + 0.5) / BLEND_STEPS as f32;
                let fill =
                    Animatable::apply_progress(&self.colors[color], &next, t * t * (3.0 - 2.0 * t));
                paths.push(fframes::svgr!(<path d={d} fill={fill} />));
            }
        }
        let id = ui.id(self.key, 0, "starburst");
        let url = format!("url(#{id})");
        let base = self.colors[0];
        fframes::svgr!(
            <g>
                <defs>
                    <clipPath id={id}>
                        <rect x="0" y="0" width={w} height={h} />
                    </clipPath>
                </defs>
                <g clip-path={url}>
                    <rect x="0" y="0" width={w} height={h} fill={base} />
                    {Svgr::from(paths)}
                </g>
            </g>
        )
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
    fn trail_brightness_follows_the_samples() {
        let trail = LightTrail::themed(&Theme::default());
        // Remotion: 32 samples at 0.9 decay sum to about 9.66
        assert!(
            (trail.gain() - 9.657 / 32.0).abs() < 1e-3,
            "{}",
            trail.gain()
        );
        let flat = LightTrail {
            decay: 1.0,
            intensity: 2.0,
            ..LightTrail::themed(&Theme::default())
        };
        assert_eq!(flat.gain(), 2.0);
    }

    #[test]
    fn starburst_rays_start_on_the_left_and_cycle_colours() {
        let burst = Starburst {
            rays: 5,
            colors: vec![Color::WHITE, Color::BLACK],
            rotation: 10.0,
            ..Starburst::themed(&Theme::default())
        };
        let wedges = burst.wedges();
        assert_eq!(wedges.len(), 5);
        assert_eq!(wedges[0], (190.0, 262.0, 0));
        assert_eq!(
            wedges.iter().map(|w| w.2).collect::<Vec<_>>(),
            [0, 1, 0, 1, 0]
        );
        let none = Starburst {
            colors: Vec::new(),
            ..burst
        };
        assert_eq!(none.wedges().len(), 0);
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
