//! The command line shared by the three cuts (16:9, 9:16 and the short): the same
//! media, encoder settings, audio mix and Skia backend, only the video and the
//! file `render` writes differ.

use std::path::Path;
use std::process::ExitCode;

use fframes::{
    AudioMixOptions, CombinedMediaProvider, EncoderOptions, LimiterOptions, MediaDirectory,
    MediaProvider, RenderOptions, StaticMediaProvider, Video, cli,
};
use fframes_skia_renderer::{
    SkiaBackend, SkiaFFramesRenderer, SkiaPipelineConcurrencyPolicy, SkiaPipelineConfig,
};

use crate::StudioIntroMedia;

/// Parses the command line and runs it for `video` with the Skia backend: Metal on
/// macOS, Vulkan elsewhere with Skia on the CPU when Vulkan is unavailable.
/// `default_output` is the file `render` writes without `-o`.
pub fn run<V: Video + Send + Sync>(video: &V, default_output: &str) -> ExitCode {
    let args = cli::parse::<cli::NoArgs>();
    let json = args.json;

    let media = match StudioIntroMedia::prepare() {
        Ok(media) => media,
        Err(err) => return fail(json, &format!("embedded media: {err}")),
    };
    // The soundtrack and sound effects are the ones of `fframes-intro`: the
    // scenes are cut to its beat grid, so the folder is shared, not copied.
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fframes-intro/dynamic_media");
    let dir = match MediaDirectory::read_folder(&folder) {
        Ok(dir) => dir,
        Err(err) => {
            return fail(
                json,
                &format!("dynamic media folder {}: {err}", folder.display()),
            );
        }
    };
    let dynamic = match dir.process_media_source() {
        Ok(dynamic) => dynamic,
        Err(err) => return fail(json, &format!("dynamic media: {err}")),
    };
    // clips and stills of other examples and of the CLI tools, shown in the 16:9 cut
    let own_folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("dynamic_media");
    let own_dir = match MediaDirectory::read_folder(&own_folder) {
        Ok(dir) => dir,
        Err(err) => {
            return fail(
                json,
                &format!("dynamic media folder {}: {err}", own_folder.display()),
            );
        }
    };
    let own = match own_dir.process_media_source() {
        Ok(own) => own,
        Err(err) => return fail(json, &format!("dynamic media: {err}")),
    };
    // the fonts fframes-components draws some of its text in (Inter for the prompt's greeting)
    let components = match fframes_components::ComponentsMedia::prepare() {
        Ok(components) => components,
        Err(err) => return fail(json, &format!("fframes-components media: {err}")),
    };
    let all =
        CombinedMediaProvider::from([&media as &dyn MediaProvider, &dynamic, &own, &components]);
    let options = RenderOptions {
        media: Some(&all),
        video_encoder_options: EncoderOptions {
            preferred_encoder: Some("libx264"),
            codec_params: Some(&[("crf", "16"), ("preset", "medium"), ("tune", "film")]),
            ..Default::default()
        },
        // the music edit measures about -17.5 LUFS; lift it to about -14 for the web
        audio_mix: AudioMixOptions {
            master_gain_db: 4.5,
            limiter: Some(LimiterOptions {
                ceiling_db: -1.3,
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
        let gpu = match fframes_skia_renderer::metal::SkiaMetalCtx::new(V::WIDTH, V::HEIGHT) {
            Ok(gpu) => gpu,
            Err(err) => return fail(json, &format!("Metal context: {err}")),
        };
        match SkiaFFramesRenderer::new_metal(&gpu, pipeline) {
            Ok(backend) => start(video, options, default_output, args, backend),
            Err(err) => fail(json, &format!("Skia Metal renderer: {err}")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        use fframes::{FFramesLogger, LinesLogger};

        // machines without Vulkan (VMs, CI) render the same frames with Skia on the CPU:
        // the shaders need Skia, the built-in tiny-skia backend would skip them
        let gpu = fframes_skia_renderer::vulkan::SkiaVulkanCtx::new(V::WIDTH, V::HEIGHT);
        let reason = match &gpu {
            Ok(gpu) => match SkiaFFramesRenderer::new_vulkan(gpu, pipeline) {
                Ok(backend) => return start(video, options, default_output, args, backend),
                Err(err) => format!("Skia Vulkan renderer: {err}"),
            },
            Err(err) => err.to_string(),
        };
        // the renderer errors span several lines, the notice stays on one
        let reason = reason.split_whitespace().collect::<Vec<_>>().join(" ");
        // release whatever part of the GPU setup succeeded before falling back
        drop(gpu);
        LinesLogger::new(json).warn(&format!(
            "Vulkan unavailable ({reason}), rendering with Skia on the CPU"
        ));
        let cpu = fframes_skia_renderer::SkiaCpuCtx::new(V::WIDTH, V::HEIGHT);
        match SkiaFFramesRenderer::new_cpu(&cpu, pipeline) {
            Ok(backend) => start(video, options, default_output, args, backend),
            Err(err) => fail(json, &format!("Skia CPU renderer: {err}")),
        }
    }
}

fn start<V: Video + Send + Sync, B: SkiaBackend>(
    video: &V,
    options: RenderOptions<'_, '_>,
    default_output: &str,
    args: cli::Cli<cli::NoArgs>,
    backend: SkiaFFramesRenderer<'_, B>,
) -> ExitCode {
    cli::new(video, options)
        .args(args)
        .default_output(default_output)
        .backend(backend)
        .preview(fframes_native_player::cli_preview)
        .run()
}

/// Reports a setup error the way the CLI reports its own: one JSON document on
/// stdout with `--json`, a plain line on stderr otherwise.
fn fail(json: bool, message: &str) -> ExitCode {
    if json {
        println!("{}", serde_json::json!({ "error": message }));
    } else {
        eprintln!("error: {message}");
    }
    ExitCode::FAILURE
}
