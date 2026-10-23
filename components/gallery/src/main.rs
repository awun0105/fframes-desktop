use components_gallery::{GalleryMedia, GalleryVideo};
use fframes::{EncoderOptions, RenderOptions, StaticMediaProvider, cli};
use fframes_skia_renderer::cli::SkiaRenderers;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let media = GalleryMedia::prepare().expect("gallery media");
    let video = GalleryVideo::new(&media);
    let tmp = PathBuf::from("test_render");

    cli::new(
        &video,
        RenderOptions {
            media: Some(&media),
            tmp_files_directory: Some(&tmp),
            video_encoder_options: EncoderOptions {
                preferred_encoder: Some("libx264"),
                codec_params: Some(&[
                    ("crf", "23"),
                    ("preset", "ultrafast"),
                    ("tune", "animation"),
                ]),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .renderers(SkiaRenderers::default())
    .run()
}
