//! The choices the export panel offers and what they cost: which formats a worker can
//! write, which frame rates and sizes make sense for a video, and how large the file will
//! be. Pure functions, so the panel only draws what they return.

use fframes_studio_protocol::export::{
    ExportCapabilities, ExportCodec, ExportContainer, ExportFormat, ExportSettings,
};

/// The sizes offered, as fractions of the video's.
pub const SCALES: [f64; 2] = [1., 0.5];

/// The frame rate offered besides the video's own.
pub const REDUCED_FPS: u32 = 30;

/// Every format the panel can offer, in the order it shows them.
pub const FORMATS: [ExportFormat; 4] = [
    ExportFormat {
        container: ExportContainer::Mp4,
        codec: ExportCodec::Mpeg4,
    },
    ExportFormat {
        container: ExportContainer::Mp4,
        codec: ExportCodec::H264,
    },
    ExportFormat {
        container: ExportContainer::Mp4,
        codec: ExportCodec::Hevc,
    },
    ExportFormat {
        container: ExportContainer::Webm,
        codec: ExportCodec::Vp9,
    },
];

/// The name of a format in the panel.
pub fn format_label(format: ExportFormat) -> &'static str {
    match format.codec {
        ExportCodec::Mpeg4 => "MP4 · MPEG-4",
        ExportCodec::H264 => "MP4 · H.264",
        ExportCodec::Hevc => "MP4 · HEVC",
        ExportCodec::Vp9 => "WebM · VP9",
    }
}

/// The formats this worker can write, in panel order.
pub fn available_formats(capabilities: &ExportCapabilities) -> Vec<ExportFormat> {
    FORMATS
        .into_iter()
        .filter(|format| capabilities.formats.contains(format))
        .collect()
}

/// The output frame rates to offer for a video at `video_fps`: its own (`None`) and 30
/// when that is a lower rate the video's divides into.
pub fn fps_options(video_fps: u32) -> Vec<Option<u32>> {
    let mut options = vec![None];
    if REDUCED_FPS < video_fps && video_fps.is_multiple_of(REDUCED_FPS) {
        options.push(Some(REDUCED_FPS));
    }
    options
}

/// The size of the exported frames: the video's scaled and rounded to even sizes, as the
/// renderer does, because yuv420 encoders reject odd ones.
pub fn output_size(width: u32, height: u32, scale: f64) -> (u32, u32) {
    if scale == 1. {
        return (width, height);
    }
    let even = |size: u32| ((f64::from(size) * scale / 2.).round() as u32 * 2).max(2);
    (even(width), even(height))
}

/// The expected size of the file in bytes: the encoder's video bitrate and the container's
/// audio bitrate over `seconds`. Exports encode at those bitrates, so this tracks the real
/// size closely, but it stays an estimate.
pub fn estimate_bytes(
    settings: &ExportSettings,
    capabilities: &ExportCapabilities,
    seconds: f64,
) -> u64 {
    let (width, height) = output_size(capabilities.width, capabilities.height, settings.scale);
    let fps = settings.fps.unwrap_or(capabilities.fps);
    let bits_per_second =
        settings.codec.video_bitrate(width, height, fps) + settings.container.audio_bitrate();
    (bits_per_second as f64 * seconds.max(0.) / 8.).round() as u64
}

/// `settings` with what this worker or video cannot take replaced by the default, one
/// choice at a time: a saved default may name a format a different SDK does not have, or a
/// frame rate this video's does not divide into.
pub fn usable_settings(
    settings: ExportSettings,
    capabilities: &ExportCapabilities,
) -> ExportSettings {
    let default = ExportSettings::default();
    let format = ExportFormat {
        container: settings.container,
        codec: settings.codec,
    };
    let (container, codec) = if capabilities.formats.contains(&format) {
        (settings.container, settings.codec)
    } else {
        (default.container, default.codec)
    };
    let fps = settings
        .fps
        .filter(|fps| fps_options(capabilities.fps).contains(&Some(*fps)));
    let settings = ExportSettings {
        container,
        codec,
        fps,
        ..settings
    };
    if capabilities.accepts(&settings).is_ok() {
        settings
    } else {
        ExportSettings::default()
    }
}

/// A file size as the panel shows it, like "12.4 MB".
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000. && unit < UNITS.len() - 1 {
        value /= 1000.;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities(fps: u32, formats: &[ExportFormat]) -> ExportCapabilities {
        ExportCapabilities {
            version: 1,
            width: 1920,
            height: 1080,
            fps,
            timing: true,
            formats: formats.to_vec(),
        }
    }

    #[test]
    fn the_panel_offers_only_formats_the_worker_has() {
        let caps = capabilities(30, &[FORMATS[0], FORMATS[3]]);
        assert_eq!(available_formats(&caps), [FORMATS[0], FORMATS[3]]);
    }

    #[test]
    fn thirty_fps_is_offered_only_below_the_video_rate_and_only_if_it_divides() {
        assert_eq!(fps_options(30), [None], "equal to the video");
        assert_eq!(fps_options(24), [None], "faster than the video");
        assert_eq!(fps_options(45), [None], "does not divide");
        assert_eq!(fps_options(60), [None, Some(30)]);
    }

    #[test]
    fn every_offered_combination_validates_against_the_video() {
        let caps = capabilities(60, &FORMATS);
        for format in available_formats(&caps) {
            for scale in SCALES {
                for fps in fps_options(caps.fps) {
                    let settings = ExportSettings {
                        container: format.container,
                        codec: format.codec,
                        scale,
                        fps,
                    };
                    assert!(caps.accepts(&settings).is_ok(), "{settings:?}");
                }
            }
        }
    }

    #[test]
    fn scaled_sizes_round_to_even_numbers() {
        assert_eq!(output_size(1920, 1080, 1.), (1920, 1080));
        assert_eq!(output_size(1920, 1080, 0.5), (960, 540));
        assert_eq!(output_size(1000, 563, 0.5), (500, 282));
        assert_eq!(output_size(2, 2, 0.1), (2, 2));
    }

    #[test]
    fn the_estimate_is_the_encoder_bitrate_over_the_duration() {
        let caps = capabilities(30, &FORMATS);
        let h264 = ExportSettings {
            container: ExportContainer::Mp4,
            codec: ExportCodec::H264,
            scale: 1.,
            fps: None,
        };
        // 6 Mbit/s video and 192 kbit/s audio for 10 s.
        assert_eq!(estimate_bytes(&h264, &caps, 10.), 7_740_000);
        let half = ExportSettings { scale: 0.5, ..h264 };
        assert_eq!(estimate_bytes(&half, &caps, 10.), 2_115_000);
        assert_eq!(estimate_bytes(&h264, &caps, 0.), 0);
    }

    #[test]
    fn a_lower_frame_rate_lowers_the_estimate() {
        let caps = capabilities(60, &FORMATS);
        let full = ExportSettings::default();
        let reduced = ExportSettings {
            fps: Some(30),
            ..full
        };
        assert!(estimate_bytes(&reduced, &caps, 10.) < estimate_bytes(&full, &caps, 10.));
    }

    #[test]
    fn a_saved_choice_the_worker_cannot_write_falls_back_to_the_default() {
        let caps = capabilities(30, &[FORMATS[0]]);
        let vp9 = ExportSettings {
            container: ExportContainer::Webm,
            codec: ExportCodec::Vp9,
            scale: 1.,
            fps: None,
        };
        assert_eq!(usable_settings(vp9, &caps), ExportSettings::default());
        let half = ExportSettings {
            scale: 0.5,
            ..ExportSettings::default()
        };
        assert_eq!(usable_settings(half, &caps), half);
    }

    #[test]
    fn each_choice_the_worker_or_video_cannot_take_falls_back_on_its_own() {
        let saved = ExportSettings {
            container: ExportContainer::Webm,
            codec: ExportCodec::Vp9,
            scale: 0.5,
            fps: Some(30),
        };
        // No VP9 here and 25 fps does not divide into 30: the half size stays.
        let usable = usable_settings(saved, &capabilities(25, &[FORMATS[0]]));
        assert_eq!(
            usable,
            ExportSettings {
                scale: 0.5,
                ..ExportSettings::default()
            }
        );
        // A 30 fps video already runs at 30: its own rate is the one offered.
        let usable = usable_settings(saved, &capabilities(30, &FORMATS));
        assert_eq!(usable, ExportSettings { fps: None, ..saved });
        assert_eq!(usable_settings(saved, &capabilities(60, &FORMATS)), saved);
    }

    #[test]
    fn sizes_read_in_decimal_units() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(12_400_000), "12.4 MB");
        assert_eq!(format_size(2_500_000_000), "2.5 GB");
    }
}
