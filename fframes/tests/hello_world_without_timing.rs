//! Frames of a video without a timing document must stay exactly what they were before
//! timing documents existed. The snapshots were approved from the renderer before the
//! timing hook was added; they match byte for byte on the machine that approved them
//! (Windows), and within the default anti-aliasing tolerance on other platforms.
//!
//! The video is `examples/hello-world`, included by path: the example crate itself
//! enables GPL encoders whose `FFmpeg` build is not available on every test machine.
#[path = "../../examples/hello-world/src/hello_world.rs"]
#[allow(dead_code)]
mod hello_world;

use fframes::{CpuFrameRenderer, Previewer, RenderOptions, StaticMediaProvider, snapshot};
use hello_world::{HelloWorldMedia, HelloWorldVideo};

#[test]
fn hello_world_frames_without_timing_are_unchanged() {
    let media = HelloWorldMedia::prepare().unwrap();
    let video = HelloWorldVideo {
        media: &media,
        slug: "Renderer!",
    };
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let mut previewer = Previewer::new(&video, &options).unwrap();
    let mut options = snapshot::SnapshotOptions {
        directory: std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../examples/hello-world/_frame_snapshots"),
        ..Default::default()
    };
    if cfg!(windows) {
        options.channel_threshold = 0;
        options.max_diff_ratio = 0.;
    }
    snapshot::assert_frames(
        &mut previewer,
        &mut CpuFrameRenderer::default(),
        &["0", "0.5s", "15s", "end"],
        &options,
    );
}
