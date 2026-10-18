// Compiled only into the disposable SDK qualification project. Exercise the CPU
// export renderer before its colour conversion/encoder, independently of preview.
fn qualify_preencode(
    video: &studio_video::StudioVideo,
    options: &fframes::RenderOptions<'_, '_>,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    use fframes::{EncoderFrameRenderer, Video};
    let value = |flag: &str| {
        let index = args
            .iter()
            .position(|arg| arg == flag)
            .ok_or("missing flag")?;
        args.get(index + 1).ok_or("missing value")
    };
    let timing = fframes::TimingDoc::read_file(std::path::Path::new(value("--timing-file")?))?;
    let options = fframes::RenderOptions {
        media: options.media,
        timing,
        scale_resolution: 0.5,
        ..Default::default()
    };
    let mut previewer = fframes::Previewer::new(video, &options)?;
    let width = studio_video::StudioVideo::WIDTH as u32 / 2;
    let height = studio_video::StudioVideo::HEIGHT as u32 / 2;
    let input = fframes::EncoderInput::software(
        fframes::ffmpeg_sys_fframes::AVPixelFormat::AV_PIX_FMT_RGBA,
    );
    let mut renderer = fframes::cpu::CpuEncoderFrameRenderer::new(20, &input, width, height)
        .map_err(|error| format!("{error:?}"))?;
    for frame in value("--frames")?.split(',') {
        let index = frame.parse()?;
        let tree = previewer.svg_tree(index)?;
        let rendered = renderer.render_tree(&tree, studio_video::StudioVideo::BACKGROUND_COLOR)?;
        std::fs::write(
            std::path::Path::new(value("--output")?).join(format!("preencode-{index}.rgba")),
            rendered.copy_plane(0, width as usize * 4, height as usize),
        )?;
    }
    Ok(())
}
