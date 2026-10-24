// feDisplacementMap on the CPU renderer (svgr) against Skia.
//
// Cargo.toml of the repro crate (paths point at an fframes checkout):
//
//   [package]
//   name = "disp-repro"
//   version = "0.1.0"
//   edition = "2024"
//
//   [workspace]
//
//   [dependencies]
//   fframes = { path = "../fframes/fframes", default-features = false, features = ["cpu_renderer", "compile-time-svgtree", "cli"] }
//   fframes_skia_renderer = { path = "../fframes/fframes-skia-renderer", features = ["vulkan", "cli"] }
//
//   # To try the fix, point svgr at a patched checkout:
//   # [patch.crates-io]
//   # svgr = { path = "../svgr/crates/svgr" }
//
// Run (no GPU needed; `skia-cpu` is Skia on the CPU):
//
//   cargo run --release -- --renderer cpu frame 29 -o out-cpu
//   cargo run --release -- --renderer skia-cpu frame 29 -o out-skia
//   odiff out-cpu/29.png out-skia/29.png diff.png --threshold 0.15
//
// The scale grows with the frame index, so frame 0 is undisplaced and frame 29 uses scale 29.
// svgr 0.50.2: 57.27% of the pixels differ. With the fix: 0.50%.

use fframes::{Color, Duration, FFramesContext, Frame, RenderOptions, Svgr, Video, cli, svgr};
use fframes_skia_renderer::cli::SkiaRenderers;

#[derive(Debug)]
struct Repro;

impl Video for Repro {
    const FPS: usize = 30;
    const WIDTH: usize = 960;
    const HEIGHT: usize = 540;
    const BACKGROUND_COLOR: Color = Color::WHITE;

    fn duration(&self) -> Duration<'_> {
        Duration::Frames(30)
    }

    fn audio(&self) -> fframes::AudioMap<'_> {
        fframes::AudioMap::none()
    }

    fn render_frame<'a>(&'a self, frame: Frame, _ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let scale = frame.index as f32;
        svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 960 540" width="960" height="540">
                <defs>
                    // A uniform map: every pixel moves by scale * (0.753 - 0.5).
                    <filter id="flat" x="0" y="0" width="100%" height="100%" color-interpolation-filters="sRGB">
                        <feFlood flood-color="#c0c0c0" result="m" />
                        <feDisplacementMap in="SourceGraphic" in2="m" scale={scale} xChannelSelector="R" yChannelSelector="G" />
                    </filter>
                    // A map with alpha below 1, read premultiplied by svgr 0.50.2.
                    <filter id="turb" x="0" y="0" width="100%" height="100%" color-interpolation-filters="sRGB">
                        <feTurbulence type="turbulence" baseFrequency="0.02" numOctaves="2" seed="3" result="n" />
                        <feDisplacementMap in="SourceGraphic" in2="n" scale={scale} xChannelSelector="R" yChannelSelector="G" />
                    </filter>
                    // Noise with its alpha forced to 1: only the doubled scale shows here.
                    <filter id="turbopaque" x="0" y="0" width="100%" height="100%" color-interpolation-filters="sRGB">
                        <feTurbulence type="fractalNoise" baseFrequency="0.02" numOctaves="2" seed="3" />
                        <feColorMatrix type="matrix" values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 0 1" result="n" />
                        <feDisplacementMap in="SourceGraphic" in2="n" scale={scale} xChannelSelector="R" yChannelSelector="G" />
                    </filter>
                </defs>
                <g filter="url(#flat)">
                    <rect x="20" y="40" width="280" height="460" fill="#2563eb" />
                    <path d="M40 100 L280 100 M40 200 L280 200 M40 300 L280 300 M40 400 L280 400" stroke="#111" stroke-width="12" />
                </g>
                <g filter="url(#turb)">
                    <rect x="340" y="40" width="280" height="460" fill="#16a34a" />
                    <path d="M360 100 L600 100 M360 200 L600 200 M360 300 L600 300 M360 400 L600 400" stroke="#111" stroke-width="12" />
                </g>
                <g filter="url(#turbopaque)">
                    <rect x="660" y="40" width="280" height="460" fill="#f59e0b" />
                    <path d="M680 100 L920 100 M680 200 L920 200 M680 300 L920 300 M680 400 L920 400" stroke="#111" stroke-width="12" />
                </g>
            </svg>
        )
    }
}

fn main() -> std::process::ExitCode {
    cli::new(&Repro, RenderOptions::default())
        .renderers(SkiaRenderers::default())
        .run()
}
