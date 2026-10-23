use fframes::{Color, SampleSize, Svgr, VisualizeFrameInput, WindowFunction};

use crate::{
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// The spectrum of `file` at the current frame (`Frame::visualize_audio_frame` with a Hann
/// window), averaged over `smooth` frames on each side. `None` when the file is not loaded:
/// list it in the video's or the scene's `fn audio`. The frame is the container's, so in a
/// scene the file should start with the scene.
pub fn spectrum(
    ui: &Ui<'_, '_, '_>,
    file: &str,
    sample_size: SampleSize,
    smooth: usize,
) -> Option<Vec<f32>> {
    let audio = ui.ctx.get_audio(file)?;
    Some(ui.frame.visualize_audio_frame(VisualizeFrameInput {
        audio,
        sample_size,
        smooth_level: smooth,
        window: Some(WindowFunction::Hann),
    }))
}

/// Where the bars of [`SpectrumBars`] grow from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarAlign {
    /// Up and down from `y`, like a level meter on a podcast cover.
    #[default]
    Center,
    /// Up from `y`.
    Bottom,
}

/// One rounded bar per value, spread over `width` from `x`. A value becomes `value * gain`
/// pixels, kept between `min_height` and `max_height`; NaN draws the shortest bar. Pass the
/// values of [`spectrum`] (often `.step_by(n)` or `center_spectrum_low_frequencies`).
#[derive(Debug, Clone, PartialEq)]
pub struct SpectrumBars {
    pub values: Vec<f32>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub bar_width: f32,
    pub gain: f32,
    pub min_height: f32,
    pub max_height: f32,
    /// `None` rounds the ends fully (half the bar width).
    pub radius: Option<f32>,
    pub fill: Color,
    pub align: BarAlign,
}

impl Themed for SpectrumBars {
    fn themed(t: &Theme) -> Self {
        Self {
            values: Vec::new(),
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            bar_width: 24.0,
            gain: 60.0,
            min_height: 24.0,
            max_height: 720.0,
            radius: None,
            fill: t.colors.accent,
            align: BarAlign::Center,
        }
    }
}

impl Default for SpectrumBars {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl SpectrumBars {
    /// The height in pixels a value is drawn at.
    pub fn height(&self, value: f32) -> f32 {
        let h = value * self.gain;
        if h.is_nan() {
            self.min_height
        } else {
            h.clamp(self.min_height, self.max_height.max(self.min_height))
        }
    }
}

impl<'a> Component<'a> for SpectrumBars {
    fn render(self, _ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        if self.values.is_empty() {
            return Svgr::empty();
        }
        let step = self.width / self.values.len() as f32;
        let radius = self.radius.unwrap_or(self.bar_width / 2.0);
        let bars: Vec<Svgr<'a>> = self
            .values
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let height = self.height(v);
                let x = self.x + i as f32 * step + (step - self.bar_width) / 2.0;
                let y = match self.align {
                    BarAlign::Center => self.y - height / 2.0,
                    BarAlign::Bottom => self.y - height,
                };
                fframes::svgr!(
                    <rect x={x} y={y} width={self.bar_width} height={height} rx={radius} fill={self.fill} />
                )
            })
            .collect();
        fframes::svgr!(<g>{bars}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_stay_between_the_limits() {
        let bars = SpectrumBars {
            gain: 10.0,
            min_height: 5.0,
            max_height: 50.0,
            ..SpectrumBars::default()
        };
        assert_eq!(bars.height(f32::NAN), 5.0);
        assert_eq!(bars.height(0.1), 5.0);
        assert_eq!(bars.height(2.0), 20.0);
        assert_eq!(bars.height(100.0), 50.0);
    }
}
