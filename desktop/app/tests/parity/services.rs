//! Build, preview and export services: start, cancel, failure surfaced.
use crate::harness::Studio;
use fframes_studio::actions::{BuildPreview, CancelExport, CancelPreview, ExportVideo};
use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[gpui::test]
fn build_shows_the_preview(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    studio.dispatch(BuildPreview);
    let shown = studio.wait("preview displayed", |t| {
        t["preview"]["status"] == "displayed"
    });
    assert_eq!(shown["total_frames"], 90);
    assert_eq!(shown["fps"], 30);
    assert_eq!(studio.compiler.finished.load(Ordering::SeqCst), 1);
}

#[gpui::test]
fn cancelling_a_build_stops_the_compile(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    studio.compiler.release.store(false, Ordering::SeqCst);
    studio.dispatch(BuildPreview);
    studio.wait("compile started", |t| t["preview"]["status"] == "building");
    studio.compiler.wait_started(1);
    studio.dispatch(CancelPreview);
    let cancelled = studio.wait("build cancelled", |t| {
        t["preview"]["status"] != "building" && t["busy"] == false
    });
    assert_ne!(cancelled["preview"]["status"], "displayed");
    let deadline = Instant::now() + Duration::from_secs(30);
    while studio.compiler.killed.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline, "the compile was never stopped");
        studio.settle(Duration::from_millis(50));
    }
    assert_eq!(studio.compiler.finished.load(Ordering::SeqCst), 0);
}

#[gpui::test]
fn a_failed_build_is_shown(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    studio.compiler.fail.store(true, Ordering::SeqCst);
    studio.dispatch(BuildPreview);
    let failed = studio.wait("build failed", |t| t["preview"]["status"] == "error");
    assert!(
        failed["preview"]["error"]
            .as_str()
            .unwrap()
            .contains("cannot find value `title`"),
        "{failed:#}"
    );
}

/// The fake worker cannot encode MP4, so a finished export is left to the native export
/// test with a real SDK; here the export starts rendering the open project.
#[gpui::test]
fn export_renders_the_open_project(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(ExportVideo);
    studio.choose_new_path(&studio.path("export.mp4"));
    let rendering = studio.wait("export rendering", |t| t["export"]["state"] == "rendering");
    assert_eq!(rendering["export"]["detail"], "1/90");
}

#[gpui::test]
fn cancelling_an_export_leaves_no_file(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let destination = studio.path("export.mp4");
    studio.dispatch(ExportVideo);
    studio.choose_new_path(&destination);
    studio.wait("export rendering", |t| t["export"]["state"] == "rendering");
    studio.dispatch(CancelExport);
    studio.wait("export cancelled", |t| t["export"]["state"] == "cancelled");
    studio.settle(Duration::from_millis(500));
    assert!(!destination.exists());
}

#[gpui::test]
fn a_failed_export_is_shown(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed_with(cx, r#"{"frames":90}"#);
    let destination = studio.path("export.mp4");
    studio.dispatch(ExportVideo);
    studio.choose_new_path(&destination);
    let failed = studio.wait("export failed", |t| t["export"]["state"] == "failed");
    assert!(
        failed["export"]["detail"]
            .as_str()
            .unwrap()
            .contains("fake worker cannot encode MP4"),
        "{failed:#}"
    );
    assert!(!destination.exists());
}
