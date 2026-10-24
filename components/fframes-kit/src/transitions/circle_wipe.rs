use fframes::{Color, Svgr};

use crate::{
    motion::clamp01,
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{
    Presentation, Side,
    presentation::{clipped, reach},
};

/// A disc of `color` grows from `center` until it covers the exiting item, then the entering
/// item opens from the same point over it. `center` is a share of `size` (0.5, 0.5 is the
/// middle); `size` is the area in view-box units, as for [`Slide`](super::Slide).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleWipe {
    pub center: (f32, f32),
    pub color: Color,
    pub size: (f32, f32),
}

impl Themed for CircleWipe {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.5, 0.5),
            color: t.colors.title,
            size: (1920.0, 1080.0),
        }
    }
}

impl Default for CircleWipe {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl CircleWipe {
    /// The centre in view-box units.
    pub fn origin(&self) -> (f32, f32) {
        (self.center.0 * self.size.0, self.center.1 * self.size.1)
    }

    /// The radius of the disc in the first half (`Covering`) and of the opening in the
    /// second (`Opening`) at `progress`.
    pub fn stage(&self, progress: f32) -> CircleStage {
        let p = clamp01(progress);
        let r = reach(self.origin(), self.size);
        if p < 0.5 {
            CircleStage::Covering(p * 2.0 * r)
        } else {
            CircleStage::Opening((p * 2.0 - 1.0) * r)
        }
    }
}

/// Where a [`CircleWipe`] is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircleStage {
    /// A disc of this radius covers the exiting item.
    Covering(f32),
    /// The entering item shows inside this radius, over the colour.
    Opening(f32),
}

impl Presentation for CircleWipe {
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
        let (cx, cy) = self.origin();
        let (w, h) = self.size;
        let color = self.color;
        match self.stage(progress) {
            CircleStage::Covering(r) if r <= 0.0 => Svgr::empty(),
            CircleStage::Covering(r) => {
                fframes::svgr!(<circle cx={cx} cy={cy} r={r} fill={color} />)
            }
            CircleStage::Opening(r) if r <= 0.0 => {
                fframes::svgr!(<rect x="0" y="0" width={w} height={h} fill={color} />)
            }
            CircleStage::Opening(r) => {
                let opening = clipped(
                    content,
                    fframes::svgr!(<circle cx={cx} cy={cy} r={r} />),
                    id,
                );
                fframes::svgr!(
                    <g>
                        <rect x="0" y="0" width={w} height={h} fill={color} />
                        {opening}
                    </g>
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_disc_covers_then_the_opening_grows() {
        let wipe = CircleWipe {
            center: (0.25, 0.5),
            ..CircleWipe::default()
        };
        // from a point left of the middle the far corners are on the right
        let far = 1440.0_f32.hypot(540.0);
        assert_eq!(wipe.stage(0.0), CircleStage::Covering(0.0));
        let CircleStage::Covering(r) = wipe.stage(0.4999) else {
            panic!("covering")
        };
        assert!((r - far).abs() < 1.0);
        assert_eq!(wipe.stage(0.5), CircleStage::Opening(0.0));
        let CircleStage::Opening(r) = wipe.stage(1.3) else {
            panic!("opening")
        };
        assert!((r - far).abs() < 1e-3);
    }
}
