//! Build, preview and export services: start, cancel, failure surfaced.
use crate::harness::Studio;
use fframes_studio::actions::BuildPreview;

#[gpui::test]
fn build_shows_the_preview(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    studio.dispatch(BuildPreview);
    let shown = studio.wait("preview displayed", |t| {
        t["preview"]["status"] == "displayed"
    });
    assert_eq!(shown["total_frames"], 90);
    assert_eq!(shown["fps"], 30);
    assert_eq!(
        studio
            .compiler
            .finished
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}
