use std::path::Path;
use std::process::ExitCode;

use fframes::{
    AudioMixOptions, CombinedMediaProvider, EncoderOptions, LimiterOptions, MediaDirectory,
    MediaProvider, RenderOptions, StaticMediaProvider, Video, cli,
};
use fframes_skia_renderer::{
    SkiaBackend, SkiaFFramesRenderer, SkiaPipelineConcurrencyPolicy, SkiaPipelineConfig,
};
use vinuni_future_rises::{VinUniFutureRises, VinUniFutureRisesMedia};

/// Renders with Skia: Metal on macOS, Vulkan elsewhere, Skia on the CPU when Vulkan is
/// unavailable (the shaders need Skia; the built-in tiny-skia backend would skip them).
fn main() -> ExitCode {
    let args = cli::parse::<cli::NoArgs>();
    let video = VinUniFutureRises;
    let media = match VinUniFutureRisesMedia::prepare() {
        Ok(media) => media,
        Err(err) => return fail(&format!("embedded media: {err}")),
    };
    let components = match fframes_components::ComponentsMedia::prepare() {
        Ok(components) => components,
        Err(err) => return fail(&format!("fframes-components media: {err}")),
    };
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("dynamic_media");
    if !folder.join("aerial.mp4").is_file() {
        return fail(&format!(
            "the VinUniversity footage is missing from {}: run `py tools/fetch-media.py` in examples/vinuni-future-rises",
            folder.display()
        ));
    }
    let dir = match MediaDirectory::read_folder(&folder) {
        Ok(dir) => dir,
        Err(err) => return fail(&format!("dynamic media folder {}: {err}", folder.display())),
    };
    let dynamic = match dir.process_media_source() {
        Ok(dynamic) => dynamic,
        Err(err) => return fail(&format!("dynamic media: {err}")),
    };
    let all = CombinedMediaProvider::from([&media as &dyn MediaProvider, &components, &dynamic]);
    let options = RenderOptions {
        media: Some(&all),
        video_encoder_options: EncoderOptions {
            preferred_encoder: Some("libx264"),
            codec_params: Some(&[("crf", "18"), ("preset", "medium"), ("tune", "film")]),
            ..Default::default()
        },
        // AAC at the mix's own 48 kHz: resampling to 44.1 kHz adds true-peak overshoot
        audio_encoder_options: EncoderOptions {
            bitrate: Some(256_000),
            sample_rate: 48_000,
            ..Default::default()
        },
        // gain is set from `audio analyze` to land about -14 LUFS for the web; the ceiling
        // leaves headroom for the encoder's overshoot (true peak measured on the mp4)
        audio_mix: AudioMixOptions {
            master_gain_db: 4.6,
            limiter: Some(LimiterOptions {
                ceiling_db: -2.6,
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let pipeline = SkiaPipelineConfig {
        concurrency_policy: SkiaPipelineConcurrencyPolicy::MaxPerformance,
        ..Default::default()
    };

    #[cfg(target_os = "macos")]
    {
        let gpu = match fframes_skia_renderer::metal::SkiaMetalCtx::new(
            VinUniFutureRises::WIDTH,
            VinUniFutureRises::HEIGHT,
        ) {
            Ok(gpu) => gpu,
            Err(err) => return fail(&format!("Metal context: {err}")),
        };
        match SkiaFFramesRenderer::new_metal(&gpu, pipeline) {
            Ok(backend) => start(&video, options, args, backend),
            Err(err) => fail(&format!("Skia Metal renderer: {err}")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let gpu = fframes_skia_renderer::vulkan::SkiaVulkanCtx::new(
            VinUniFutureRises::WIDTH,
            VinUniFutureRises::HEIGHT,
        );
        let reason = match &gpu {
            Ok(gpu) => match SkiaFFramesRenderer::new_vulkan(gpu, pipeline) {
                Ok(backend) => return start(&video, options, args, backend),
                Err(err) => format!("Skia Vulkan renderer: {err}"),
            },
            Err(err) => err.to_string(),
        };
        let reason = reason.split_whitespace().collect::<Vec<_>>().join(" ");
        drop(gpu);
        eprintln!("Vulkan unavailable ({reason}), rendering with Skia on the CPU");
        let cpu = fframes_skia_renderer::SkiaCpuCtx::new(
            VinUniFutureRises::WIDTH,
            VinUniFutureRises::HEIGHT,
        );
        match SkiaFFramesRenderer::new_cpu(&cpu, pipeline) {
            Ok(backend) => start(&video, options, args, backend),
            Err(err) => fail(&format!("Skia CPU renderer: {err}")),
        }
    }
}

fn start<B: SkiaBackend>(
    video: &VinUniFutureRises,
    options: RenderOptions<'_, '_>,
    args: cli::Cli<cli::NoArgs>,
    backend: SkiaFFramesRenderer<'_, B>,
) -> ExitCode {
    cli::new(video, options)
        .args(args)
        .default_output("vinuni-future-rises.mp4")
        .backend(backend)
        .preview(fframes_native_player::cli_preview)
        .run()
}

fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::FAILURE
}
