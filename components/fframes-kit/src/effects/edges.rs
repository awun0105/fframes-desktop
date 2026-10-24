use fframes::{Color, Svgr};

use super::{Region, alpha_masked, filtered, slope_kernel};
use crate::{
    anim::Anim,
    motion::clamp01,
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// The doublings of [`LongShadows`]: 2⁸ copies, so a shadow up to 255 units long moves by
/// at most one unit per copy.
const MAX_DOUBLINGS: u32 = 8;

/// The primitive that passes the source through unchanged.
fn unchanged<'a>() -> Svgr<'a> {
    fframes::svgr!(<feOffset dx="0" dy="0" />)
}

/// `color` as a flood colour and opacity, its alpha times `opacity`.
fn flood(color: Color, opacity: f32) -> (Color, f32) {
    (
        color.with_alpha(255),
        clamp01(opacity) * f32::from(color.a) / 255.0,
    )
}

/// The unit vector towards a light `degrees` clockwise from straight up (y down).
fn light_from_above(degrees: f32) -> (f32, f32) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    (sin, -cos)
}

filter_effect!(
    /// A solid outline `width` units wide around the opaque parts of its children (Remotion's
    /// `outline()`), in `color` at `opacity`. With `outline_only` the children are replaced
    /// by the filled silhouette in the outline colour. The filter region grows a tenth of the
    /// children's box on every side; wider outlines are cut there.
    Outline("outline", Region::Pad(0.1)) {
        width: Anim<'a, f32> = Anim::Fixed(8.0),
        color: Color = Color::WHITE,
        opacity: f32 = 1.0,
        outline_only: bool = false,
    }
    themed(t)
    primitives(|this, ui| {
        let width = ui.sample(&this.width).max(0.0);
        let (color, opacity) = flood(this.color, this.opacity);
        let grown = fframes::svgr!(
            <feMorphology in="SourceAlpha" operator="dilate" radius={width} result="outline-grown" />
        );
        let paint = fframes::svgr!(<feFlood flood-color={color} flood-opacity={opacity} />);
        let ring = fframes::svgr!(<feComposite in2="outline-grown" operator="in" result="outline-ring" />);
        let merge = if this.outline_only {
            Svgr::empty()
        } else {
            fframes::svgr!(
                <feMerge>
                    <feMergeNode in="outline-ring" />
                    <feMergeNode in="SourceGraphic" />
                </feMerge>
            )
        };
        Svgr::from(vec![grown, paint, ring, merge])
    })
);

filter_effect!(
    /// Highlights and shadows along the alpha edges of its children, so text, logos and
    /// shapes look raised (Remotion's `bevelAlpha()`). `edge_thickness` is the width of the
    /// rounded bevel, `light_angle` the light's direction in degrees clockwise from above (0
    /// lights the top edges), `light_color` the colour added to the lit edges (its alpha sets
    /// the strength) and `light_intensity` the strength of both, 0 to 1. A fully opaque
    /// picture has no alpha edges and stays as it is.
    ///
    /// The slope is measured over neighbouring pixels, so at a smaller output scale (a draft
    /// render) the same thickness gives a stronger bevel.
    BevelAlpha("bevel", Region::Pad(0.05)) {
        edge_thickness: Anim<'a, f32> = Anim::Fixed(2.0),
        light_angle: f32 = -60.0,
        light_color: Color = Color::WHITE,
        light_intensity: f32 = 0.4,
    }
    themed(t)
    primitives(|this, ui| {
        let thickness = ui.sample(&this.edge_thickness);
        let intensity = clamp01(this.light_intensity);
        if thickness <= 0.0 || intensity <= 0.0 {
            unchanged()
        } else {
            let sigma = thickness / 2.0;
            let (ux, uy) = light_from_above(this.light_angle);
            // a blurred edge rises by at most 0.4 / sigma per unit; the gain brings that to 1
            let gain = sigma / 2.4;
            let lit = slope_kernel((-ux, -uy), gain);
            let dark = slope_kernel((ux, uy), gain);
            let (light, light_opacity) = flood(this.light_color, intensity);
            Svgr::from(vec![
                fframes::svgr!(<feGaussianBlur in="SourceAlpha" stdDeviation={sigma} result="bevel-height" />),
                fframes::svgr!(
                    <feConvolveMatrix in="bevel-height" order="3" kernelMatrix={lit} divisor="1"
                                      preserveAlpha="false" result="bevel-lit" />
                ),
                fframes::svgr!(
                    <feConvolveMatrix in="bevel-height" order="3" kernelMatrix={dark} divisor="1"
                                      preserveAlpha="false" result="bevel-dark" />
                ),
                fframes::svgr!(<feFlood flood-color={light} flood-opacity={light_opacity} />),
                fframes::svgr!(<feComposite in2="bevel-lit" operator="in" result="bevel-light" />),
                fframes::svgr!(<feFlood flood-color="#000000" flood-opacity={intensity} />),
                fframes::svgr!(<feComposite in2="bevel-dark" operator="in" result="bevel-shadow" />),
                fframes::svgr!(
                    <feMerge>
                        <feMergeNode in="SourceGraphic" />
                        <feMergeNode in="bevel-light" />
                        <feMergeNode in="bevel-shadow" />
                    </feMerge>
                ),
                fframes::svgr!(<feComposite in2="SourceAlpha" operator="in" />),
            ])
        }
    })
);

filter_effect!(
    /// The edge of its children roughened like torn paper (Remotion's `roughenEdges()`):
    /// `amount` 0 to 1, `border` how far the edge moves in and out, `scale` the size of the
    /// noise (smaller is finer) and `seed` the pattern; derive the seed from the frame to
    /// animate the edge. The alpha is blurred, shifted by `feTurbulence` noise and cut at a
    /// threshold, so the edge stays smooth at any scale.
    RoughenEdges("roughen", Region::Pad(0.15)) {
        amount: Anim<'a, f32> = Anim::Fixed(1.0),
        border: f32 = 26.5,
        scale: f32 = 0.07,
        seed: f32 = 231.2,
    }
    themed(t)
    primitives(|this, ui| {
        let amount = clamp01(ui.sample(&this.amount));
        if amount <= 0.0 || this.border <= 0.0 {
            unchanged()
        } else {
            let sigma = this.border / 3.0;
            let frequency = 0.004 / this.scale.clamp(0.01, 4.0);
            let seed = this.seed.round();
            let (k3, k4) = (2.0 * amount, -amount);
            Svgr::from(vec![
                fframes::svgr!(<feGaussianBlur in="SourceAlpha" stdDeviation={sigma} result="rough-soft" />),
                fframes::svgr!(
                    <feTurbulence type="fractalNoise" baseFrequency={frequency} numOctaves="3"
                                  seed={seed} result="rough-noise" />
                ),
                fframes::svgr!(
                    <feColorMatrix in="rough-noise" type="matrix"
                                   values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0" result="rough-grain" />
                ),
                fframes::svgr!(
                    <feComposite in="rough-soft" in2="rough-grain" operator="arithmetic"
                                 k1="0" k2="1" k3={k3} k4={k4} result="rough-edge" />
                ),
                // cut at a half, over a ramp of a tenth so 8-bit steps do not show
                fframes::svgr!(
                    <feComponentTransfer in="rough-edge" result="rough-mask">
                        <feFuncA type="linear" slope="10" intercept="-4.5" />
                    </feComponentTransfer>
                ),
                fframes::svgr!(<feComposite in="SourceGraphic" in2="rough-mask" operator="in" />),
            ])
        }
    })
);

filter_effect!(
    /// Its children shrink-wrapped in plastic (Remotion's `shrinkwrap()`): glossy glints on
    /// `feTurbulence` wrinkles, brightest at `highlight_intensity`, a stretched highlight
    /// along the edges facing the top left at `edge_tension`, all scaled by `amount` (0
    /// unchanged). `wrinkle_density` from 0 (broad folds) to 1 (tight texture) and `seed`
    /// pick the wrinkles. Remotion also displaces the picture along the wrinkles; that needs
    /// `feDisplacementMap`, which the CPU backend draws wrongly, so it is left out.
    Shrinkwrap("shrinkwrap", Region::Pad(0.05)) {
        amount: Anim<'a, f32> = Anim::Fixed(1.0),
        highlight_intensity: f32 = 0.75,
        wrinkle_density: f32 = 0.42,
        edge_tension: f32 = 0.45,
        seed: u32 = 0,
    }
    themed(t)
    primitives(|this, ui| {
        let amount = clamp01(ui.sample(&this.amount));
        if amount <= 0.0 {
            unchanged()
        } else {
            let frequency = 0.006 + 0.03 * clamp01(this.wrinkle_density);
            // the light comes from the top left, so the slopes that face it fall away to the
            // bottom right
            let away = (std::f32::consts::FRAC_1_SQRT_2, std::f32::consts::FRAC_1_SQRT_2);
            let gloss = slope_kernel(away, 4.0);
            let rim = slope_kernel(away, 2.5);
            let glint = this.highlight_intensity.max(0.0) * amount;
            // only the steepest slopes glint
            let (glint_slope, glint_cut) = (3.0 * glint, -1.5 * glint);
            let tension = clamp01(this.edge_tension) * amount;
            let sheen = 0.06 * amount;
            Svgr::from(vec![
                fframes::svgr!(
                    <feTurbulence type="turbulence" baseFrequency={frequency} numOctaves="3"
                                  seed={this.seed} result="wrap-noise" />
                ),
                fframes::svgr!(
                    <feColorMatrix in="wrap-noise" type="matrix"
                                   values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0" result="wrap-height" />
                ),
                fframes::svgr!(<feGaussianBlur in="wrap-height" stdDeviation="3" result="wrap-smooth" />),
                fframes::svgr!(
                    <feConvolveMatrix in="wrap-smooth" order="3" kernelMatrix={gloss} divisor="1"
                                      preserveAlpha="false" result="wrap-slope" />
                ),
                fframes::svgr!(
                    <feComponentTransfer in="wrap-slope" result="wrap-glint">
                        <feFuncA type="linear" slope={glint_slope} intercept={glint_cut} />
                    </feComponentTransfer>
                ),
                fframes::svgr!(<feGaussianBlur in="SourceAlpha" stdDeviation="6" result="wrap-soft" />),
                fframes::svgr!(
                    <feConvolveMatrix in="wrap-soft" order="3" kernelMatrix={rim} divisor="1"
                                      preserveAlpha="false" result="wrap-edge" />
                ),
                fframes::svgr!(
                    <feComponentTransfer in="wrap-edge" result="wrap-tension">
                        <feFuncA type="linear" slope={tension} intercept="0" />
                    </feComponentTransfer>
                ),
                fframes::svgr!(<feFlood flood-color="#ffffff" flood-opacity={sheen} />),
                fframes::svgr!(<feComposite in2="SourceAlpha" operator="in" result="wrap-sheen" />),
                fframes::svgr!(<feFlood flood-color="#ffffff" />),
                fframes::svgr!(<feComposite in2="wrap-glint" operator="in" result="wrap-light" />),
                fframes::svgr!(<feFlood flood-color="#ffffff" />),
                fframes::svgr!(<feComposite in2="wrap-tension" operator="in" result="wrap-rim" />),
                fframes::svgr!(
                    <feMerge>
                        <feMergeNode in="SourceGraphic" />
                        <feMergeNode in="wrap-sheen" />
                        <feMergeNode in="wrap-light" />
                        <feMergeNode in="wrap-rim" />
                    </feMerge>
                ),
                fframes::svgr!(<feComposite in2="SourceAlpha" operator="in" />),
            ])
        }
    })
);

/// The alpha silhouette of its children turned into a solid shadow `length` units long
/// towards `angle` (degrees clockwise from the right, 45 down and right), in `color` at
/// `opacity` (Remotion's `longShadows()`): text and logos with long shadows, or a flat 3D
/// extrusion. Drawn inside the `area` from the origin, so like Remotion the shadow stops at
/// the canvas edge.
///
/// The shadow is the silhouette merged with itself shifted, then the pair with itself
/// shifted twice as far, and so on: at most eight doublings give 256 copies, one unit apart
/// for a shadow up to 255 units long, for the cost of sixteen primitives. Semi-transparent
/// silhouettes grow more opaque where copies overlap.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LongShadows<'a> {
    pub length: Anim<'a, f32>,
    pub angle: f32,
    pub color: Color,
    pub opacity: f32,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for LongShadows<'_> {
    /// Remotion's defaults: 240 units at 45 degrees in `#777777`.
    fn themed(_: &Theme) -> Self {
        Self {
            length: Anim::Fixed(240.0),
            angle: 45.0,
            color: Color::rgb(0x77, 0x77, 0x77),
            opacity: 1.0,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

/// How many times [`LongShadows`] doubles its copies for a shadow `length` units long, and
/// the distance between neighbouring copies.
pub(crate) fn shadow_steps(length: f32) -> (u32, f32) {
    if length < 1.0 {
        return (0, 0.0);
    }
    let doublings = (length + 1.0)
        .log2()
        .ceil()
        .clamp(1.0, MAX_DOUBLINGS as f32) as u32;
    (doublings, length / ((1u32 << doublings) - 1) as f32)
}

impl<'a> Component<'a> for LongShadows<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let length = ui.sample(&self.length).max(0.0);
        let content = (self.children)(ui);
        let (doublings, step) = shadow_steps(length);
        if doublings == 0 {
            return content;
        }
        let (sin, cos) = self.angle.to_radians().sin_cos();
        let mut primitives = Vec::with_capacity(2 * doublings as usize + 4);
        let mut last = "SourceAlpha".to_owned();
        for i in 0..doublings {
            let shift = step * (1u32 << i) as f32;
            let moved = format!("ls-moved-{i}");
            let merged = format!("ls-{i}");
            primitives.push(fframes::svgr!(
                <feOffset in={last.clone()} dx={shift * cos} dy={shift * sin} result={moved.clone()} />
            ));
            primitives.push(fframes::svgr!(
                <feMerge result={merged.clone()}>
                    <feMergeNode in={last} />
                    <feMergeNode in={moved} />
                </feMerge>
            ));
            last = merged;
        }
        let (color, opacity) = flood(self.color, self.opacity);
        primitives.push(fframes::svgr!(<feFlood flood-color={color} flood-opacity={opacity} />));
        primitives
            .push(fframes::svgr!(<feComposite in2={last} operator="in" result="ls-shadow" />));
        primitives.push(fframes::svgr!(
            <feMerge>
                <feMergeNode in="ls-shadow" />
                <feMergeNode in="SourceGraphic" />
            </feMerge>
        ));
        let id = ui.id(self.key, 0, "long-shadows");
        let (w, h) = self.area;
        filtered(content, id, Region::Area(w, h), Svgr::from(primitives))
    }
}

/// A raised relief of repeated dashes pressed into its children (Remotion's `emboss()`):
/// cells `size` units square, each holding a dash `line_width` wide, lit from `light_angle`
/// (degrees counter-clockwise from the right, 135 the top left) so one side of each dash
/// catches the light and the other falls into shadow. `depth` (0 to 1) is the height of the
/// relief, `amount` (0 to 1) its strength, `angle` turns the pattern and `offset` scrolls it
/// (animate it to move the relief). The source colours and transparency stay: the relief is
/// a pattern over the `area` from the origin, masked by the children's alpha, so the
/// children render twice.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Emboss<'a> {
    pub amount: Anim<'a, f32>,
    pub size: f32,
    pub line_width: f32,
    pub depth: f32,
    pub angle: f32,
    pub light_angle: f32,
    pub offset: f32,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Emboss<'_> {
    /// Remotion's defaults.
    fn themed(_: &Theme) -> Self {
        Self {
            amount: Anim::Fixed(0.7),
            size: 26.0,
            line_width: 7.0,
            depth: 0.75,
            angle: 0.0,
            light_angle: 135.0,
            offset: 0.0,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl Emboss<'_> {
    /// How much each side of a dash faces the light, -1 to 1: top, bottom, left and right in
    /// the pattern's own turned frame.
    pub fn facing(&self) -> [f32; 4] {
        let (ly, lx) = self.light_angle.to_radians().sin_cos();
        // towards the light, y down, then into the pattern's frame
        let (ux, uy) = (lx, -ly);
        let (sin, cos) = self.angle.to_radians().sin_cos();
        let (px, py) = (ux * cos + uy * sin, -ux * sin + uy * cos);
        [-py, py, -px, px]
    }
}

impl<'a> Component<'a> for Emboss<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let amount = clamp01(ui.sample(&self.amount));
        let content = (self.children)(ui);
        if amount <= 0.0 || self.size <= 0.0 {
            return content;
        }
        let shape = (self.children)(ui);
        let id = ui.id(self.key, 0, "emboss");
        let pattern = format!("{id}-relief");
        let fill = format!("url(#{pattern})");
        let s = self.size;
        let lw = self.line_width.clamp(0.0, s);
        let long = s * 0.62;
        let (x0, y0) = ((s - long) / 2.0, (s - lw) / 2.0);
        let edge = lw * 0.5 * clamp01(self.depth);
        let [top, bottom, left, right] = self.facing();
        let strip = |x: f32, y: f32, w: f32, h: f32, facing: f32| {
            let color = if facing > 0.0 { "#ffffff" } else { "#000000" };
            let opacity = facing.abs() * amount * 0.8;
            fframes::svgr!(<rect x={x} y={y} width={w} height={h} fill={color} opacity={opacity} />)
        };
        let strips = vec![
            strip(x0, y0, long, edge, top),
            strip(x0, y0 + lw - edge, long, edge, bottom),
            strip(x0, y0, edge, lw, left),
            strip(x0 + long - edge, y0, edge, lw, right),
        ];
        let turn = format!("rotate({}) translate({} 0)", self.angle, self.offset);
        let (w, h) = self.area;
        let relief = fframes::svgr!(
            <g>
                <defs>
                    <pattern id={pattern} x="0" y="0" width={s} height={s}
                             patternUnits="userSpaceOnUse" patternTransform={turn}>
                        {strips}
                    </pattern>
                </defs>
                <rect x="0" y="0" width={w} height={h} fill={fill} />
            </g>
        );
        let masked = alpha_masked(relief, shape, &format!("{id}-mask"), self.area);
        fframes::svgr!(<g>{content}{masked}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_shadows_reach_their_length_in_unit_steps() {
        assert_eq!(shadow_steps(0.5), (0, 0.0));
        for length in [1.0, 7.0, 100.0, 240.0, 255.0] {
            let (n, step) = shadow_steps(length);
            let copies = (1u32 << n) - 1;
            assert!((step * copies as f32 - length).abs() < 1e-3, "{length}");
            assert!(step <= 1.0 + 1e-6, "{length}: {step}");
        }
        // capped at eight doublings
        assert_eq!(shadow_steps(1000.0).0, MAX_DOUBLINGS);
    }

    #[test]
    fn the_lit_side_of_the_dashes_faces_the_light() {
        let emboss = Emboss::themed(&Theme::default());
        // light from the top left: the top and left sides catch it
        let [top, bottom, left, right] = emboss.facing();
        assert!(top > 0.0 && left > 0.0 && bottom < 0.0 && right < 0.0);
        let turned = Emboss {
            angle: 90.0,
            ..Emboss::themed(&Theme::default())
        };
        // turned a quarter clockwise, the dash's top faces right and its left faces up
        let [top, _, left, _] = turned.facing();
        assert!(top < 0.0 && left > 0.0);
    }
}
