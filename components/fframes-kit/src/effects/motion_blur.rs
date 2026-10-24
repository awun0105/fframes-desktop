use fframes::Svgr;

use crate::{
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// The most copies [`Trail`] and [`CameraMotionBlur`] draw: each one renders the children
/// again, so the cost of a frame grows with it.
pub const MAX_SAMPLES: usize = 16;

/// The children at earlier local times behind themselves, each copy fainter, like Remotion's
/// `<Trail>`. `samples` copies (at most [`MAX_SAMPLES`]) are `spacing` seconds apart; the
/// newest has `opacity` and they fade out linearly behind it. Before the local start there is
/// nothing to repeat, so the trail grows in.
///
/// The children render `samples + 1` times a frame, so a frame costs about that many times
/// theirs. The earlier copies run at a finer frame rate than the video's; children that work
/// in local seconds (`ui.seconds()`, `ui.frames()`) see the right time.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Trail<'a> {
    pub samples: usize,
    pub spacing: f32,
    pub opacity: f32,
    pub children: Children<'a>,
}

impl Themed for Trail<'_> {
    /// Six copies 0.05 seconds apart from half opacity, with no children.
    fn themed(_: &Theme) -> Self {
        Self {
            samples: 6,
            spacing: 0.05,
            opacity: 0.5,
            children: no_children(),
        }
    }
}

impl Trail<'_> {
    /// How far back copy `i` (from 1, the newest) is and how opaque.
    pub fn copy(&self, i: usize) -> (f32, f32) {
        let n = self.samples.min(MAX_SAMPLES);
        let fade = 1.0 - (i - 1) as f32 / n as f32;
        (i as f32 * self.spacing, self.opacity * fade)
    }
}

impl<'a> Component<'a> for Trail<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let n = self.samples.min(MAX_SAMPLES);
        let mut layers: Vec<Svgr<'a>> = (1..=n)
            .rev()
            .filter_map(|i| {
                let (back, opacity) = self.copy(i);
                let mut earlier = ui.earlier(back)?;
                let content = (self.children)(&mut earlier);
                Some(fframes::svgr!(<g opacity={opacity}>{content}</g>))
            })
            .collect();
        layers.push((self.children)(ui));
        layers.into_iter().collect()
    }
}

/// Motion blur as a film camera records it, like Remotion's `<CameraMotionBlur>`: the
/// children are averaged over the time the shutter is open before each frame.
/// `shutter_angle` is in degrees, 360 for the whole frame interval; `samples` (at most
/// [`MAX_SAMPLES`]) are the moments averaged. Each is drawn over the earlier ones with
/// opacity `1 / k` for the `k`-th, so all count the same where the children are opaque.
///
/// The children render `samples` times a frame. The samples run at a finer frame rate than
/// the video's; children that work in local seconds see the right time.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct CameraMotionBlur<'a> {
    pub shutter_angle: f32,
    pub samples: usize,
    pub children: Children<'a>,
}

impl Themed for CameraMotionBlur<'_> {
    /// A 180° shutter and ten samples (Remotion's defaults), with no children.
    fn themed(_: &Theme) -> Self {
        Self {
            shutter_angle: 180.0,
            samples: 10,
            children: no_children(),
        }
    }
}

impl CameraMotionBlur<'_> {
    /// Seconds before the frame of each sample at `fps`, the earliest first and the frame
    /// itself last.
    pub fn offsets(&self, fps: usize) -> Vec<f32> {
        let n = self.samples.clamp(1, MAX_SAMPLES);
        let open = self.shutter_angle.clamp(0.0, 360.0) / 360.0 / fps as f32;
        (0..n).rev().map(|k| open * k as f32 / n as f32).collect()
    }
}

impl<'a> Component<'a> for CameraMotionBlur<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let mut drawn = 0;
        self.offsets(ui.frame.fps)
            .into_iter()
            .filter_map(|back| {
                let mut at = if back > 0.0 {
                    ui.earlier(back)?
                } else {
                    ui.shifted(0, ui.duration)
                };
                drawn += 1;
                let opacity = 1.0 / drawn as f32;
                let content = (self.children)(&mut at);
                Some(fframes::svgr!(<g opacity={opacity}>{content}</g>))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trail_copies_fall_back_and_fade() {
        let trail = Trail::themed(&Theme::default());
        assert_eq!(trail.copy(1), (0.05, 0.5));
        let (back, opacity) = trail.copy(6);
        assert!((back - 0.3).abs() < 1e-6);
        assert!(opacity > 0.0 && opacity < 0.1);
    }

    #[test]
    fn the_shutter_spreads_samples_before_the_frame() {
        let blur = CameraMotionBlur {
            samples: 4,
            ..CameraMotionBlur::themed(&Theme::default())
        };
        // a 180° shutter at 30 fps is open for 1/60 s
        let offsets = blur.offsets(30);
        assert_eq!(offsets.len(), 4);
        assert_eq!(*offsets.last().unwrap(), 0.0);
        assert!((offsets[0] - 0.75 / 60.0).abs() < 1e-6);
        let many = CameraMotionBlur {
            samples: 100,
            ..CameraMotionBlur::themed(&Theme::default())
        };
        assert_eq!(many.offsets(30).len(), MAX_SAMPLES);
    }
}
