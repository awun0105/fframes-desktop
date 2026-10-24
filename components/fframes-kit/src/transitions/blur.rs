use fframes::{Color, Svgr, Transform};

use crate::{
    motion::{clamp01, cubic_in_out, lerp},
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{
    Direction, Presentation, Side,
    presentation::{bell, blurred, zoomed},
};

/// Both items zoom and turn through a flash in the middle of the transition, blurring as
/// they go (Remotion's `dreamyZoom()`, after Zeh Fernando's GL transition). The exiting item
/// grows to `scale` and turns by `rotation` degrees while the flash rises; the entering
/// item starts there, turned the other way, and settles while the flash fades. The blur
/// peaks at `blur` (a standard deviation in view-box units) at the flash; Remotion has no
/// blur prop, it is the kit's. `size` is the area in view-box units, as for
/// [`Slide`](super::Slide).
///
/// The blur is the cost: one full-canvas Gaussian per frame of the transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DreamyZoom {
    pub rotation: f32,
    pub scale: f32,
    pub flash_color: Color,
    pub blur: f32,
    pub size: (f32, f32),
}

impl Default for DreamyZoom {
    /// Remotion's rotation and scale, a white flash.
    fn default() -> Self {
        Self {
            rotation: 6.0,
            scale: 1.2,
            flash_color: Color::WHITE,
            blur: 12.0,
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for DreamyZoom {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl DreamyZoom {
    /// How far `side` is from rest at `progress`: 0 at rest, 1 at the flash, `None` while it
    /// is hidden behind the flash.
    pub fn away(progress: f32, side: Side) -> Option<f32> {
        let p = clamp01(progress);
        match side {
            Side::Exiting => (p < 0.5).then_some(p * 2.0),
            Side::Entering => (p >= 0.5).then_some((1.0 - p) * 2.0),
        }
    }
}

impl Presentation for DreamyZoom {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a> {
        let (w, h) = self.size;
        let content = match Self::away(progress, side) {
            Some(a) => {
                let turn = match side {
                    Side::Exiting => self.rotation * a,
                    Side::Entering => -self.rotation * a,
                };
                let scale = lerp(1.0, self.scale, a);
                let content = zoomed(content, scale, turn, (w / 2.0, h / 2.0));
                blurred(content, self.blur * a, self.blur * a, self.size, id, side)
            }
            None => Svgr::empty(),
        };
        let flash = bell(progress);
        // drawn once, by the entering side on top
        if side == Side::Exiting || flash <= 0.0 {
            return content;
        }
        fframes::svgr!(
            <g>
                {content}
                <rect x="0" y="0" width={w} height={h} fill={self.flash_color} opacity={flash} />
            </g>
        )
    }
}

/// The entering item fades in over the exiting one while both blur (Remotion's
/// `linearBlur()`, after gre's GL transition, which averages a 6 by 6 grid of samples).
/// `intensity` is how far the blur spreads, as a share of the width, at the middle of the
/// transition. `axes` weighs it along x and y: `(1.0, 1.0)` blurs both ways like Remotion,
/// `(1.0, 0.0)` only horizontally. The CPU backend blurs along the canvas axes, so a slanted
/// direction is not available. `size` is the area in view-box units, as for
/// [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearBlur {
    pub intensity: f32,
    pub axes: (f32, f32),
    pub size: (f32, f32),
}

impl Default for LinearBlur {
    fn default() -> Self {
        Self {
            intensity: 0.1,
            axes: (1.0, 1.0),
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for LinearBlur {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl LinearBlur {
    /// The blur's standard deviation along its direction at `progress`: a spread of
    /// `intensity` times the width at the middle, as a box of that length.
    pub fn deviation(&self, progress: f32) -> f32 {
        self.intensity.max(0.0) * self.size.0 * bell(progress) / 12f32.sqrt()
    }
}

impl Presentation for LinearBlur {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a> {
        let d = self.deviation(progress);
        let content = blurred(
            content,
            d * self.axes.0,
            d * self.axes.1,
            self.size,
            id,
            side,
        );
        match side {
            Side::Exiting => content,
            Side::Entering => {
                let opacity = clamp01(progress);
                fframes::svgr!(<g opacity={opacity}>{content}</g>)
            }
        }
    }
}

/// Both items pan in one direction like a fast camera move, with a motion blur along it
/// (Remotion's `blurSlide()`). The entering item comes from `direction`; the move eases
/// in and out. The blur grows with the distance travelled in the first half and shrinks
/// with the distance left in the second; `blur` is its length at the middle as a share of
/// `size` along the move. `size` is the area in view-box units, as for
/// [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlurSlide {
    pub direction: Direction,
    pub blur: f32,
    pub size: (f32, f32),
}

impl Default for BlurSlide {
    fn default() -> Self {
        Self {
            direction: Direction::FromLeft,
            blur: 0.5,
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for BlurSlide {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl BlurSlide {
    /// The offset of `side` and the blur's standard deviation along the move at `progress`.
    pub fn motion(&self, progress: f32, side: Side) -> ((f32, f32), f32) {
        let e = cubic_in_out(clamp01(progress));
        let (tx, ty) = self.direction.travel();
        let shift = match side {
            Side::Entering => e - 1.0,
            Side::Exiting => e,
        };
        let extent = if tx == 0.0 { self.size.1 } else { self.size.0 };
        let length = self.blur.max(0.0) * extent * 2.0 * e.min(1.0 - e).max(0.0);
        (
            (tx * self.size.0 * shift, ty * self.size.1 * shift),
            length / 12f32.sqrt(),
        )
    }
}

impl Presentation for BlurSlide {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a> {
        let ((x, y), deviation) = self.motion(progress, side);
        let (sx, sy) = match self.direction {
            Direction::FromLeft | Direction::FromRight => (deviation, 0.0),
            Direction::FromTop | Direction::FromBottom => (0.0, deviation),
        };
        // moved first, so the blur sees only what is on the canvas
        let transform = Transform::translate(x, y);
        let content = fframes::svgr!(<g transform={transform}>{content}</g>);
        blurred(content, sx, sy, self.size, id, side)
    }
}

/// Both items zoom across a moving centre with a blur, cross-fading in the middle
/// (Remotion's `crossZoom()`). An approximation: the GL original averages samples along
/// the rays from the centre (a zoom blur), which SVG cannot draw. The kit scales each item
/// about a centre that moves from a quarter to three quarters of the width and blurs it
/// evenly, both by `strength` times the distance from rest. `size` is the area in view-box
/// units, as for [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossZoom {
    pub strength: f32,
    pub size: (f32, f32),
}

impl Default for CrossZoom {
    fn default() -> Self {
        Self {
            strength: 0.4,
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for CrossZoom {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl CrossZoom {
    /// The centre of the zoom at `progress`, in view-box units.
    pub fn center(&self, progress: f32) -> (f32, f32) {
        let p = clamp01(progress);
        (self.size.0 * lerp(0.25, 0.75, p), self.size.1 / 2.0)
    }

    /// The opacity of the entering item: it takes over around the middle.
    pub fn mix(progress: f32) -> f32 {
        let t = clamp01((clamp01(progress) - 0.35) / 0.3);
        t * t * (3.0 - 2.0 * t)
    }
}

impl Presentation for CrossZoom {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a> {
        let away = bell(progress) * self.strength.max(0.0);
        let content = zoomed(content, 1.0 + away, 0.0, self.center(progress));
        let deviation = away * self.size.0 * 0.02;
        let content = blurred(content, deviation, deviation, self.size, id, side);
        match side {
            Side::Exiting => content,
            Side::Entering => {
                let opacity = Self::mix(progress);
                fframes::svgr!(<g opacity={opacity}>{content}</g>)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dreamy_zoom_swaps_the_items_behind_the_flash() {
        assert_eq!(DreamyZoom::away(0.0, Side::Exiting), Some(0.0));
        assert_eq!(DreamyZoom::away(0.0, Side::Entering), None);
        assert_eq!(DreamyZoom::away(0.5, Side::Exiting), None);
        assert_eq!(DreamyZoom::away(0.5, Side::Entering), Some(1.0));
        assert_eq!(DreamyZoom::away(1.0, Side::Entering), Some(0.0));
    }

    #[test]
    fn blurs_peak_in_the_middle_and_vanish_at_the_ends() {
        let linear = LinearBlur::default();
        assert_eq!(linear.deviation(0.0), 0.0);
        assert_eq!(linear.deviation(1.0), 0.0);
        assert!((linear.deviation(0.5) - 192.0 / 12f32.sqrt()).abs() < 1e-3);
        let slide = BlurSlide::default();
        assert_eq!(slide.motion(0.0, Side::Exiting), ((0.0, 0.0), 0.0));
        assert_eq!(slide.motion(0.0, Side::Entering), ((-1920.0, 0.0), 0.0));
        let ((x, _), d) = slide.motion(0.5, Side::Exiting);
        assert!((x - 960.0).abs() < 1e-3);
        assert!((d - 960.0 / 12f32.sqrt()).abs() < 1e-3);
        assert_eq!(slide.motion(1.0, Side::Entering), ((0.0, 0.0), 0.0));
    }

    #[test]
    fn cross_zoom_hands_over_in_the_middle() {
        assert_eq!(CrossZoom::mix(0.0), 0.0);
        assert!((CrossZoom::mix(0.5) - 0.5).abs() < 1e-6);
        assert_eq!(CrossZoom::mix(1.0), 1.0);
        let z = CrossZoom::default();
        assert_eq!(z.center(0.0).0, 480.0);
        assert_eq!(z.center(1.0).0, 1440.0);
    }
}
