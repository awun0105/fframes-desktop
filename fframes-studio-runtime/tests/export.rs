use std::path::Path;

use fframes::{AudioTimestamp, Color, Duration, Frame, RenderOptions, Svgr, Video};
use fframes_studio_protocol::export::{ExportCodec, ExportContainer, ExportSettings};
use fframes_studio_runtime::{export_capabilities, run_export};

/// Scenes are long enough that every export is encoded in more than one segment (a
/// segment is at least two GOPs of 12 frames).
const SCENE_FRAMES: usize = 40;
const SECONDS: f64 = 3. * SCENE_FRAMES as f64 / 10.;

#[derive(Debug)]
struct SolidScene(&'static str, Color);
impl fframes::Scene for SolidScene {
    fn duration(&self) -> Duration<'_> {
        Duration::Frames(SCENE_FRAMES)
    }
    fn editor_instance_key(&self) -> Option<&str> {
        Some(self.0)
    }
    fn render_frame<'a>(&'a self, _: Frame, _: &fframes::FFramesContext<'a, '_>) -> Svgr<'a> {
        fframes::svgr!(<rect x="0" y="0" width="64" height="36" fill={self.1} />)
    }
}

struct ScenesVideo([SolidScene; 3]);
impl ScenesVideo {
    fn new() -> Self {
        Self([
            SolidScene("a", Color::rgb(255, 0, 0)),
            SolidScene("b", Color::rgb(0, 255, 0)),
            SolidScene("c", Color::rgb(0, 0, 255)),
        ])
    }
}
impl Video for ScenesVideo {
    const FPS: usize = 10;
    const WIDTH: usize = 64;
    const HEIGHT: usize = 36;
    fn duration(&self) -> Duration<'_> {
        Duration::Auto
    }
    fn audio(&self) -> fframes::AudioMap<'_> {
        fframes::AudioMap::from([("tone.wav", AudioTimestamp::Second(0.)..AudioTimestamp::Eof)])
    }
    fn define_scenes(&self) -> fframes::Scenes<'_> {
        let scenes: Vec<&dyn fframes::Scene> = self.0.iter().map(|s| s as _).collect();
        fframes::Scenes::from(scenes)
    }
    fn render_frame<'a>(&'a self, frame: Frame, ctx: &fframes::FFramesContext<'a, '_>) -> Svgr<'a> {
        fframes::svgr!(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 36">{ctx.render_scenes(&frame)}</svg>)
    }
}

/// A mono 16-bit 44.1 kHz sine as long as the video.
fn tone_wav() -> Vec<u8> {
    let rate = 44_100u32;
    let samples: Vec<i16> = (0..(rate as f64 * SECONDS) as usize)
        .map(|i| ((i as f64 * 440. * std::f64::consts::TAU / f64::from(rate)).sin() * 8000.) as i16)
        .collect();
    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&(rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

/// Exports with the arguments after `--export-v1 --output <output>`.
fn export(output: &Path, extra: &[&str]) -> Result<(), String> {
    let media_dir = output.with_extension("media");
    std::fs::create_dir_all(&media_dir).unwrap();
    std::fs::write(media_dir.join("tone.wav"), tone_wav()).unwrap();
    let folder = fframes::MediaDirectory::read_folder(&media_dir).unwrap();
    let media = folder.process_media_source().unwrap();
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let mut args = vec!["--export-v1".to_owned(), "--output".to_owned()];
    args.push(output.to_string_lossy().into_owned());
    args.extend(extra.iter().map(|arg| (*arg).to_owned()));
    run_export(&ScenesVideo::new(), &options, &args).map_err(|e| e.to_string())
}

/// RGB of the centre of a decoded frame.
fn centre_pixel(path: &Path, fps: usize, frame: i64) -> [u8; 3] {
    let mut decoder = unsafe { fframes::media::FFmpegDecoder::new(path, fps, 1) }.unwrap();
    assert!(unsafe { decoder.decode_up_to(frame) }.unwrap());
    let image = unsafe {
        decoder
            .get_raw_frame()
            .convert_last_decoded_frame_into_svg_image(None)
    }
    .unwrap();
    let at = ((image.height / 2 * image.width + image.width / 2) * 4) as usize;
    [image.data[at], image.data[at + 1], image.data[at + 2]]
}

fn assert_close(actual: [u8; 3], expected: [u8; 3]) {
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(a, e)| a.abs_diff(e) <= 24),
        "{actual:?} is not close to {expected:?}"
    );
}

/// Seconds of video and of audio in an exported file.
fn durations(path: &Path, fps: usize) -> (f64, f64) {
    let video = unsafe { fframes::media::FFmpegDecoder::new(path, fps, 1) }.unwrap();
    let frames = video.get_last_frame_offset().unwrap() + 1;
    let (rate, samples) = fframes::media::AudioDecoder::new(path, None)
        .unwrap()
        .decode_all_samples()
        .unwrap();
    (
        frames as f64 / fps as f64,
        samples.len() as f64 / f64::from(rate),
    )
}

#[test]
fn every_reported_format_exports_at_the_requested_size_and_rate() {
    let capabilities = export_capabilities::<ScenesVideo>();
    assert_eq!(
        (capabilities.width, capabilities.height, capabilities.fps),
        (64, 36, 10)
    );
    assert!(capabilities.timing);
    assert!(
        capabilities
            .formats
            .iter()
            .any(|format| format.codec == ExportCodec::Mpeg4)
    );
    let temp = tempfile::tempdir().unwrap();
    for format in &capabilities.formats {
        let settings = ExportSettings {
            container: format.container,
            codec: format.codec,
            scale: 0.5,
            fps: Some(5),
        };
        let output = temp.path().join(format!(
            "{}.{}",
            format.codec.stream_codec_name(),
            format.container.extension()
        ));
        let args = settings.to_args();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        export(&output, &args).unwrap_or_else(|error| panic!("{format:?}: {error}"));
        let streams = fframes::media::probe_streams(&output).unwrap();
        assert_eq!(
            streams.video_codec.as_deref(),
            Some(format.codec.stream_codec_name())
        );
        assert_eq!(
            streams.audio_codec.as_deref(),
            Some(match format.container {
                ExportContainer::Mp4 => "aac",
                ExportContainer::Webm => "opus",
            })
        );
        assert_eq!((streams.width, streams.height, streams.fps), (32, 18, 5.));
        // Output frame 50 is video frame 100, in the last segment and the last scene.
        assert_close(centre_pixel(&output, 5, 50), [0, 0, 255]);
        assert_close(centre_pixel(&output, 5, 10), [255, 0, 0]);
        let (video, audio) = durations(&output, 5);
        assert!(
            (video - SECONDS).abs() < 0.01 && (audio - SECONDS).abs() < 0.1,
            "{format:?}: {video}s of video and {audio}s of audio instead of {SECONDS}s"
        );
    }
}

#[test]
fn the_default_export_is_mpeg4_at_full_size() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("default.mp4");
    export(&output, &[]).unwrap();
    let streams = fframes::media::probe_streams(&output).unwrap();
    assert_eq!(streams.video_codec.as_deref(), Some("mpeg4"));
    assert_eq!(streams.audio_codec.as_deref(), Some("aac"));
    assert_eq!((streams.width, streams.height, streams.fps), (64, 36, 10.));
    assert_close(centre_pixel(&output, 10, 0), [255, 0, 0]);
    assert_close(centre_pixel(&output, 10, 100), [0, 0, 255]);
}

#[test]
fn export_plays_the_timing_file_and_fails_without_it() {
    let temp = tempfile::tempdir().unwrap();
    let timing = temp.path().join("timeline.json");
    std::fs::write(
        &timing,
        r#"{"version":1,"clips":[
            {"key":"c","trim_start_frames":0,"trim_end_frames":0},
            {"key":"a","trim_start_frames":5,"trim_end_frames":0}]}"#,
    )
    .unwrap();
    let output = temp.path().join("timed.mp4");
    export(&output, &["--timing-file", timing.to_str().unwrap()]).unwrap();
    // c 0..40, a trimmed 40..75, then b, which the document omits, 75..115.
    assert_close(centre_pixel(&output, 10, 0), [0, 0, 255]);
    assert_close(centre_pixel(&output, 10, 50), [255, 0, 0]);
    assert_close(centre_pixel(&output, 10, 100), [0, 255, 0]);

    let missing = temp.path().join("missing.json");
    let output = temp.path().join("missing.mp4");
    let error = export(&output, &["--timing-file", missing.to_str().unwrap()]).unwrap_err();
    assert!(error.contains("does not exist"), "{error}");

    std::fs::write(&timing, r#"{"version":1,"clips":[],"extra":1}"#).unwrap();
    let output = temp.path().join("malformed.mp4");
    assert!(export(&output, &["--timing-file", timing.to_str().unwrap()]).is_err());
}

#[test]
fn invalid_settings_fail_before_rendering() {
    let temp = tempfile::tempdir().unwrap();
    let webm_h264 = ExportSettings {
        container: ExportContainer::Webm,
        codec: ExportCodec::H264,
        ..Default::default()
    }
    .to_args();
    let webm_h264: Vec<&str> = webm_h264.iter().map(String::as_str).collect();
    for (name, args) in [
        ("h264.webm", webm_h264.as_slice()),
        ("half.mp4", &["--fps", "3"][..]),
        ("wrong.webm", &[][..]),
    ] {
        let output = temp.path().join(name);
        assert!(export(&output, args).is_err(), "{name}");
        assert!(!output.exists(), "{name}");
    }
}
