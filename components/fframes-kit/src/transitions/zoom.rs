use fframes::Svgr;

use crate::{
    motion::{clamp01, lerp},
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Presentation, Side, presentation::zoomed};

/// The exiting item zooms in towards the viewer and cross-fades into the entering item,
/// which starts magnified and zooms back out (Remotion's `zoomInOut()`). Remotion takes no
/// options; `scale` is the magnification at the cross-fade, the kit's own prop. `size` is
/// the area in view-box units, as for [`Slide`](super::Slide); both items zoom about its
/// middle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomInOut {
    pub scale: f32,
    pub size: (f32, f32),
}

impl Default for ZoomInOut {
    fn default() -> Self {
        Self {
            scale: 1.5,
            size: (1920.0, 1080.0),
        }
    }
}

impl Themed for ZoomInOut {
    fn themed(_: &Theme) -> Self {
        Self::default()
    }
}

impl ZoomInOut {
    /// The scale and opacity of `side` at `progress`.
    pub fn look(&self, progress: f32, side: Side) -> (f32, f32) {
        let p = clamp01(progress);
        match side {
            Side::Exiting => (lerp(1.0, self.scale, p), 1.0 - p),
            Side::Entering => (lerp(self.scale, 1.0, p), p),
        }
    }
}

impl Presentation for ZoomInOut {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        _ui: &Ui<'a, '_, '_>,
        _id: &str,
    ) -> Svgr<'a> {
        let (scale, opacity) = self.look(progress, side);
        let middle = (self.size.0 / 2.0, self.size.1 / 2.0);
        let content = zoomed(content, scale, 0.0, middle);
        fframes::svgr!(<g opacity={opacity}>{content}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_fades_through_the_magnified_middle() {
        let z = ZoomInOut::default();
        assert_eq!(z.look(0.0, Side::Exiting), (1.0, 1.0));
        assert_eq!(z.look(0.0, Side::Entering), (1.5, 0.0));
        assert_eq!(z.look(0.5, Side::Exiting), z.look(0.5, Side::Entering));
        assert_eq!(z.look(1.0, Side::Entering), (1.0, 1.0));
    }
}
