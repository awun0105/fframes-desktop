use fframes::{Color, Svgr};

use crate::{
    motion::{clamp01, lerp},
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Presentation, Side, presentation::zoomed};

/// A hard editorial cut with a short punch-in on both items and a flash at the edit
/// (Remotion's `pushCut()`). The exiting item grows to `outgoing_scale` until
/// `cut_progress`, then the entering item shows at `incoming_start_scale` and grows to
/// `incoming_end_scale`. A flash of `flash_color` at `flash_opacity` fades over
/// `flash_frames` frames after the cut. `origin` is a share of `size` (Remotion's
/// `transformOrigin`); `size` is the area in view-box units, as for
/// [`Slide`](super::Slide). Use a linear timing: a spring crosses the cut more than once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PushCut {
    pub cut_progress: f32,
    pub outgoing_scale: f32,
    pub incoming_start_scale: f32,
    pub incoming_end_scale: f32,
    pub origin: (f32, f32),
    pub flash_color: Color,
    pub flash_opacity: f32,
    pub flash_frames: f32,
    pub size: (f32, f32),
}

impl Default for PushCut {
    /// Remotion's defaults.
    fn default() -> Self {
        Self {
            cut_progress: 5.0 / 11.0,
            outgoing_scale: 1.04,
            incoming_start_scale: 1.04,
            incoming_end_scale: 1.07,
            origin: (0.5, 0.5),
            flash_color: Color::rgb(0xf5, 0xf2, 0xed),
            flash_opacity: 0.2,
            flash_frames: 2.0,
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for PushCut {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl PushCut {
    /// The scale of `side` at `progress`, or `None` while it is hidden.
    pub fn scale(&self, progress: f32, side: Side) -> Option<f32> {
        let p = clamp01(progress);
        let cut = self.cut_progress.clamp(0.0, 1.0);
        match side {
            Side::Exiting if p < cut => Some(lerp(1.0, self.outgoing_scale, p / cut.max(1e-6))),
            Side::Entering if p >= cut => Some(lerp(
                self.incoming_start_scale,
                self.incoming_end_scale,
                (p - cut) / (1.0 - cut).max(1e-6),
            )),
            _ => None,
        }
    }

    /// The flash's opacity `frames` after the cut: `flash_opacity` at the cut, 0 from
    /// `flash_frames` on.
    pub fn flash(&self, frames: f32) -> f32 {
        if frames < 0.0 || self.flash_frames <= 0.0 {
            return 0.0;
        }
        self.flash_opacity * clamp01(1.0 - frames / self.flash_frames)
    }
}

impl Presentation for PushCut {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        ui: &Ui<'a, '_, '_>,
        _id: &str,
    ) -> Svgr<'a> {
        let Some(scale) = self.scale(progress, side) else {
            return Svgr::empty();
        };
        let origin = (self.origin.0 * self.size.0, self.origin.1 * self.size.1);
        let content = zoomed(content, scale, 0.0, origin);
        // the flash belongs to the entering side only, it would double otherwise
        let flash = match (side, ui.transition_seconds(Side::Entering)) {
            (Side::Entering, Some(length)) => {
                let after_cut = (clamp01(progress) - self.cut_progress) * length;
                self.flash(after_cut * ui.frame.fps as f32)
            }
            _ => 0.0,
        };
        if flash <= 0.0 {
            return content;
        }
        let (w, h) = self.size;
        fframes::svgr!(
            <g>
                {content}
                <rect x="0" y="0" width={w} height={h} fill={self.flash_color} opacity={flash} />
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn punches_in_and_cuts_once() {
        let cut = PushCut::default();
        assert_eq!(cut.scale(0.0, Side::Exiting), Some(1.0));
        assert_eq!(cut.scale(0.0, Side::Entering), None);
        let c = cut.cut_progress;
        assert!((cut.scale(c - 1e-4, Side::Exiting).unwrap() - 1.04).abs() < 1e-3);
        assert_eq!(cut.scale(c, Side::Exiting), None);
        assert_eq!(cut.scale(c, Side::Entering), Some(1.04));
        assert!((cut.scale(1.0, Side::Entering).unwrap() - 1.07).abs() < 1e-6);
        assert!((cut.flash(0.0) - 0.2).abs() < 1e-6);
        assert!((cut.flash(1.0) - 0.1).abs() < 1e-6);
        assert_eq!(cut.flash(2.0), 0.0);
        assert_eq!(cut.flash(-1.0), 0.0);
    }
}
