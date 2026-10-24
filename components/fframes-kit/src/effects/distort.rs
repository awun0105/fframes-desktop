use fframes::{Svgr, Transform};

use super::alpha_masked;
use crate::{
    anim::Anim,
    motion::clamp01,
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// `primitives` (a noise image) drawn over `content` at `opacity`, only where `shape` is
/// opaque, inside the `area` from the origin. The noise moves by `offset`: it is drawn on a
/// rectangle in a shifted frame, so the turbulence follows the shift and still covers the
/// area.
fn noise_over<'a>(
    content: Svgr<'a>,
    shape: Svgr<'a>,
    id: &str,
    (w, h): (f32, f32),
    (ox, oy): (f32, f32),
    opacity: f32,
    primitives: Svgr<'a>,
) -> Svgr<'a> {
    let filter = format!("{id}-filter");
    let url = format!("url(#{filter})");
    let shift = Transform::translate(ox, oy);
    let noise = fframes::svgr!(
        <g opacity={opacity}>
            <defs>
                <filter id={filter} filterUnits="userSpaceOnUse" x={-ox} y={-oy} width={w} height={h}
                        color-interpolation-filters="sRGB">
                    {primitives}
                </filter>
            </defs>
            <g transform={shift}>
                <rect x={-ox} y={-oy} width={w} height={h} filter={url} />
            </g>
        </g>
    );
    let masked = alpha_masked(noise, shape, &format!("{id}-mask"), (w, h));
    fframes::svgr!(<g>{content}{masked}</g>)
}

/// A grey `feColorMatrix` from the red channel of turbulence: `gain` times the distance
/// from `centre`, around mid grey plus `brightness`, opaque.
fn grey<'a>(input: &str, gain: f32, centre: f32, brightness: f32) -> Svgr<'a> {
    let offset = 0.5 - gain * centre + brightness;
    let values =
        format!("{gain} 0 0 0 {offset} {gain} 0 0 0 {offset} {gain} 0 0 0 {offset} 0 0 0 0 1");
    let input = input.to_owned();
    fframes::svgr!(<feColorMatrix in={input} type="matrix" values={values} />)
}

/// Grey static over its children (Remotion's `whiteNoise()`): TV static, signal loss, snow.
/// `amount` mixes it in, 0 unchanged and 1 the visible pixels fully replaced; the
/// children's alpha stays. `seed` picks the pattern: derive it from the frame
/// (`ui.frame.index as u32`) to make the static move. `grain` is the size of a speck in
/// units: keep it at least two output pixels (2 for a full-size render, more for a scaled
/// down one), since finer noise aliases and each backend samples it differently. Drawn with
/// `feTurbulence` over the `area` from the origin; the children render twice.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct WhiteNoise<'a> {
    pub amount: Anim<'a, f32>,
    pub seed: u32,
    pub grain: f32,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for WhiteNoise<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            amount: Anim::Fixed(1.0),
            seed: 0,
            grain: 2.0,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for WhiteNoise<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let amount = clamp01(ui.sample(&self.amount));
        let content = (self.children)(ui);
        if amount <= 0.0 {
            return content;
        }
        let shape = (self.children)(ui);
        let id = ui.id(self.key, 0, "white-noise");
        let frequency = 1.0 / self.grain.max(0.1);
        let primitives = Svgr::from(vec![
            fframes::svgr!(
                <feTurbulence type="fractalNoise" baseFrequency={frequency} numOctaves="1"
                              seed={self.seed} result="static" />
            ),
            grey("static", 3.0, 0.5, 0.0),
        ]);
        noise_over(
            content,
            shape,
            &id,
            self.area,
            (0.0, 0.0),
            amount,
            primitives,
        )
    }
}

/// The look of [`FractalNoise`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FractalType {
    /// Layered soft clouds (`feTurbulence type="fractalNoise"`).
    #[default]
    Basic,
    /// Sharp ridges like smoke or marble veins (`type="turbulence"`).
    Turbulent,
}

/// Layered grey noise for clouds, smoke and organic textures (Remotion's
/// `fractalNoise()`), drawn over its children where they are opaque and mixed in by
/// `amount` (1 replaces the colours).
///
/// - `scale` is the size of the largest cells in units and `complexity` the number of layers
///   (1 to 10, each half the size and half the strength of the one before).
/// - `contrast` stretches the greys about mid grey, `brightness` is added after it, and
///   `invert` flips them.
/// - `offset_x` and `offset_y` move the texture; `seed` picks it.
/// - `evolution` in degrees morphs it in place: every full turn moves on to the next seed,
///   cross-fading on the way. With `cycle_evolution` the seeds repeat after `cycle` turns,
///   so the texture loops.
///
/// Approximations of Remotion's props: `feTurbulence` has two fractal types and fixed layer
/// steps, so Remotion's other fractal types, `noiseType`, `subInfluence` and `subScaling` are
/// not available; the cross-fade is a little softer halfway between two seeds.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct FractalNoise<'a> {
    pub fractal_type: FractalType,
    pub scale: f32,
    pub complexity: u32,
    pub evolution: Anim<'a, f32>,
    pub cycle_evolution: bool,
    pub cycle: u32,
    pub contrast: f32,
    pub brightness: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub seed: u32,
    pub invert: bool,
    pub amount: Anim<'a, f32>,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for FractalNoise<'_> {
    /// Remotion's defaults (six layers of 100 unit clouds, fully mixed in).
    fn themed(_: &Theme) -> Self {
        Self {
            fractal_type: FractalType::Basic,
            scale: 100.0,
            complexity: 6,
            evolution: Anim::Fixed(0.0),
            cycle_evolution: false,
            cycle: 1,
            contrast: 1.0,
            brightness: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            seed: 0,
            invert: false,
            amount: Anim::Fixed(1.0),
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl FractalNoise<'_> {
    /// The two seeds `evolution` degrees cross-fades between, and how far it is from the
    /// first to the second.
    pub fn seeds(&self, evolution: f32) -> (u32, u32, f32) {
        let turns = evolution / 360.0;
        let whole = turns.floor();
        let fade = turns - whole;
        let step = |k: i64| {
            let k = if self.cycle_evolution {
                k.rem_euclid(i64::from(self.cycle.max(1)))
            } else {
                k
            };
            self.seed.wrapping_add(k as u32)
        };
        let k = whole as i64;
        (step(k), step(k + 1), fade)
    }
}

impl<'a> Component<'a> for FractalNoise<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let amount = clamp01(ui.sample(&self.amount));
        let content = (self.children)(ui);
        if amount <= 0.0 {
            return content;
        }
        let shape = (self.children)(ui);
        let id = ui.id(self.key, 0, "fractal-noise");
        let kind = match self.fractal_type {
            FractalType::Basic => "fractalNoise",
            FractalType::Turbulent => "turbulence",
        };
        // fractal noise sits about mid grey, turbulence about a quarter
        let (gain, centre) = match self.fractal_type {
            FractalType::Basic => (1.6, 0.5),
            FractalType::Turbulent => (3.2, 0.25),
        };
        let sign = if self.invert { -1.0 } else { 1.0 };
        let frequency = 1.0 / self.scale.max(1.0);
        let octaves = self.complexity.clamp(1, 10);
        let (first, second, fade) = self.seeds(ui.sample(&self.evolution));
        let layer = |seed: u32, result: &str| {
            let result = result.to_owned();
            fframes::svgr!(
                <feTurbulence type={kind} baseFrequency={frequency} numOctaves={octaves}
                              seed={seed} result={result} />
            )
        };
        let mut primitives = vec![layer(first, "noise-a")];
        let input = if fade > 0.0 {
            primitives.push(layer(second, "noise-b"));
            primitives.push(fframes::svgr!(
                <feComposite in="noise-a" in2="noise-b" operator="arithmetic"
                             k1="0" k2={1.0 - fade} k3={fade} k4="0" result="noise-mix" />
            ));
            "noise-mix"
        } else {
            "noise-a"
        };
        primitives.push(grey(
            input,
            sign * gain * self.contrast.max(0.0),
            centre,
            self.brightness,
        ));
        noise_over(
            content,
            shape,
            &id,
            self.area,
            (self.offset_x, self.offset_y),
            amount,
            Svgr::from(primitives),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evolution_moves_through_the_seeds_and_cycles() {
        let noise = FractalNoise {
            seed: 10,
            ..FractalNoise::themed(&Theme::default())
        };
        assert_eq!(noise.seeds(0.0), (10, 11, 0.0));
        assert_eq!(noise.seeds(540.0), (11, 12, 0.5));
        let looping = FractalNoise {
            cycle_evolution: true,
            cycle: 2,
            ..noise
        };
        // after two turns the pattern is back where it started
        assert_eq!(looping.seeds(720.0).0, looping.seeds(0.0).0);
        assert_eq!(looping.seeds(360.0), (11, 10, 0.0));
    }
}
