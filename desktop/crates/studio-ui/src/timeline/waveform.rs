//! Cap's `createWaveformPath`: the engine's dBFS peaks as a filled curve rising from the
//! bottom of the mix row. Only the visible seconds are sampled, at about two points per
//! pixel; the view only draws, the peaks come with the prepared audio.
use gpui::{Bounds, Pixels, point, px};
use studio_engine::waveform::{WAVEFORM_MIN_DB, WAVEFORM_PEAKS_PER_SECOND};

/// Cap's `WAVEFORM_SAMPLE_STEP`: the spacing of the peak table in seconds.
const WAVEFORM_SAMPLE_STEP: f64 = 1. / WAVEFORM_PEAKS_PER_SECOND as f64;
const WAVEFORM_CONTROL_STEP: f64 = 0.05;
const MAX_WAVEFORM_SAMPLES: usize = 6000;
const WAVEFORM_SAMPLES_PER_PIXEL: f64 = 2.;

/// Cap's `amplitudeAt`: the peak at `time` seconds as a 0..1 height against the floor.
pub(super) fn waveform_amplitude(peaks: &[f32], time: f64) -> f64 {
    let index = (time * WAVEFORM_PEAKS_PER_SECOND as f64).floor();
    let db = (index >= 0.)
        .then(|| peaks.get(index as usize))
        .flatten()
        .filter(|value| value.is_finite())
        .map_or(WAVEFORM_MIN_DB, |value| *value);
    let floor = f64::from(WAVEFORM_MIN_DB);
    (1. + f64::from(db).max(floor) / -floor).clamp(0., 1.)
}

/// Cap's `numSamples` for a curve `width` pixels wide.
pub(super) fn waveform_sample_count(width: f64) -> usize {
    ((width.max(0.) * WAVEFORM_SAMPLES_PER_PIXEL).ceil() as usize).min(MAX_WAVEFORM_SAMPLES)
}

/// The curve of seconds `range` stretched across `bounds` (window pixels), closed along
/// the bottom edge. `None` without peaks or for an empty range.
pub(super) fn waveform_path(
    peaks: &[f32],
    range: (f64, f64),
    bounds: Bounds<Pixels>,
) -> Option<gpui::Path<Pixels>> {
    let duration = (range.1 - range.0).max(WAVEFORM_SAMPLE_STEP);
    let width = f64::from(f32::from(bounds.size.width));
    if peaks.is_empty() || !duration.is_finite() || !range.0.is_finite() || width <= 0. {
        return None;
    }
    let native_samples = (duration / WAVEFORM_SAMPLE_STEP).ceil() as usize + 1;
    let samples = waveform_sample_count(width)
        .clamp(50, MAX_WAVEFORM_SAMPLES)
        .min(native_samples);
    let time_step = duration / samples as f64;
    let left = f64::from(f32::from(bounds.left()));
    let top = f64::from(f32::from(bounds.top()));
    let height = f64::from(f32::from(bounds.size.height));
    let map = |x: f64, y: f64| {
        point(
            px((left + x.clamp(0., 1.) * width) as f32),
            px((top + y * height) as f32),
        )
    };
    let control_step = (WAVEFORM_CONTROL_STEP / duration).min(0.25);
    let mut builder = gpui::PathBuilder::fill();
    builder.move_to(map(0., 1.));
    for i in 0..=samples {
        let time = range.0 + i as f64 * time_step;
        let x = (time - range.0) / duration;
        let previous_time = time - time_step;
        let previous_x = ((previous_time - range.0) / duration).max(0.);
        let y = 1. - waveform_amplitude(peaks, time);
        let previous_y = 1. - waveform_amplitude(peaks, previous_time);
        builder.cubic_bezier_to(
            map(x, y),
            map(previous_x + control_step / 2., previous_y),
            map(x - control_step / 2., y),
        );
    }
    builder.line_to(map(1., 1.));
    builder.close();
    builder.build().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::size;

    #[test]
    fn peaks_become_heights_against_the_floor() {
        let peaks = [0., -30., -60., -90., f32::NAN];
        let at = |second: f64| waveform_amplitude(&peaks, second);
        assert_eq!(at(0.05), 1.);
        assert_eq!(at(0.1), 0.5);
        assert_eq!(at(0.2), 0.);
        assert_eq!(at(0.3), 0.);
        assert_eq!(at(0.4), 0.);
        // Outside the table: silence.
        assert_eq!(at(-0.1), 0.);
        assert_eq!(at(10.), 0.);
    }

    #[test]
    fn the_curve_samples_the_visible_range_only() {
        assert_eq!(waveform_sample_count(100.), 200);
        assert_eq!(waveform_sample_count(1e9), MAX_WAVEFORM_SAMPLES);
        let bounds = Bounds::new(point(px(10.), px(20.)), size(px(300.), px(44.)));
        assert!(waveform_path(&[], (0., 1.), bounds).is_none());
        let empty = Bounds::new(point(px(10.), px(20.)), size(px(0.), px(44.)));
        assert!(waveform_path(&[0.; 10], (0., 1.), empty).is_none());
        assert!(waveform_path(&[0.; 10], (f64::NAN, 1.), bounds).is_none());
        assert!(waveform_path(&[0., -20., -40.], (0.05, 0.3), bounds).is_some());
    }
}
