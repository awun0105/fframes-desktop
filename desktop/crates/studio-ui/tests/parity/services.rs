//! Build, preview and export services: start, cancel, failure surfaced.
use crate::harness::Studio;
use std::{
    fs,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
use studio_ui::actions::{BuildPreview, CancelExport, CancelPreview, ExportVideo, StartExport};

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

#[gpui::test]
fn a_failed_rebuild_keeps_the_displayed_preview_until_a_build_succeeds(
    cx: &mut gpui::TestAppContext,
) {
    let mut studio = Studio::displayed(cx);
    let shown = studio.telemetry();
    let displayed = shown["displayed_revision"].clone();
    let source = shown["current_source_revision"].clone();
    let file = studio.project_file("src/lib.rs");
    let original = fs::read(&file).unwrap();
    fs::write(&file, [&original[..], b"\n// edited\n"].concat()).unwrap();
    studio.wait("the edit is seen", |t| {
        t["current_source_revision"] != source && t["busy"] == false
    });
    studio.compiler.fail.store(true, Ordering::SeqCst);
    studio.dispatch(BuildPreview);
    let failed = studio.wait("rebuild failed", |t| t["preview"]["status"] == "error");
    assert_eq!(failed["displayed_revision"], displayed, "{failed:#}");
    assert_eq!(failed["audio_identity_matches"], true, "{failed:#}");
    // Restoring the source is seen without a build.
    fs::write(&file, &original).unwrap();
    studio.wait("the restored source is seen", |t| {
        t["current_source_revision"] == source && t["busy"] == false
    });
    studio.compiler.fail.store(false, Ordering::SeqCst);
    studio.dispatch(BuildPreview);
    let rebuilt = studio.wait("rebuilt", |t| {
        t["preview"]["status"] == "displayed" && t["busy"] == false
    });
    assert_eq!(rebuilt["displayed_revision"], displayed, "{rebuilt:#}");
    assert_eq!(rebuilt["audio_identity_matches"], true, "{rebuilt:#}");
    // Seek serials keep growing across installs, so a rebuild is not out of order.
    assert_eq!(rebuilt["image_metrics"]["out_of_order"], 0, "{rebuilt:#}");
}

/// Opens the export panel, waits for the worker's answer about the options and starts an
/// export to `destination` with the panel's settings.
fn export_to(studio: &mut Studio, destination: &std::path::Path) {
    studio.dispatch(ExportVideo);
    studio.wait("export options known", |t| {
        t["export"]["panel_open"] == true
            && t["export"]["support"] != "probing"
            && t["export"]["support"] != "unknown"
    });
    studio.dispatch(StartExport);
    studio.choose_new_path(destination);
}

/// The fake worker cannot encode MP4, so a finished export is left to the native export
/// test with a real SDK; here the export starts rendering the open project.
#[gpui::test]
fn export_renders_the_open_project(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let destination = studio.path("export.mp4");
    export_to(&mut studio, &destination);
    let rendering = studio.wait("export rendering", |t| t["export"]["state"] == "rendering");
    assert_eq!(rendering["export"]["detail"], "1/90");
}

#[gpui::test]
fn cancelling_an_export_leaves_no_file(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let destination = studio.path("export.mp4");
    export_to(&mut studio, &destination);
    studio.wait("export rendering", |t| t["export"]["state"] == "rendering");
    studio.dispatch(CancelExport);
    studio.wait("export cancelled", |t| t["export"]["state"] == "cancelled");
    studio.settle(Duration::from_millis(500));
    assert!(!destination.exists());
}

/// The export panel covers the editor, so the timeline is not drawn; the thumbnails of a
/// preview installed behind it are still kept as they arrive, and the worker stops drawing
/// thumbnails once the visible ones are there.
#[gpui::test]
fn thumbnails_arriving_behind_the_export_panel_are_kept(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let shown = studio.wait("the visible thumbnails cached", |t| {
        t["thumbnail_metrics"]["entries"] == 12
    });
    let displayed = shown["displayed_revision"].clone();
    let file = studio.project_file("src/lib.rs");
    let original = fs::read(&file).unwrap();
    fs::write(&file, [&original[..], b"\n// edited\n"].concat()).unwrap();
    studio.wait("the edit is seen", |t| {
        t["current_source_revision"] != shown["current_source_revision"] && t["busy"] == false
    });
    studio.dispatch(ExportVideo);
    studio.wait("export panel open", |t| t["export"]["panel_open"] == true);
    studio.dispatch(BuildPreview);
    let rebuilt = studio.wait("the new preview's thumbnails cached", |t| {
        t["displayed_revision"] != displayed && t["thumbnail_metrics"]["entries"] == 12
    });
    assert_eq!(rebuilt["export"]["panel_open"], true, "{rebuilt:#}");
    let settled = studio.settle(Duration::from_millis(500));
    assert_eq!(
        settled["pump_metrics"]["thumbnails"], rebuilt["pump_metrics"]["thumbnails"],
        "{settled:#}"
    );
}

#[gpui::test]
fn a_failed_export_is_shown(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed_with(cx, r#"{"frames":90}"#);
    let destination = studio.path("export.mp4");
    export_to(&mut studio, &destination);
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

#[gpui::test]
fn commands_run_while_an_export_renders(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let destination = studio.path("export.mp4");
    export_to(&mut studio, &destination);
    studio.wait("export rendering", |t| t["export"]["state"] == "rendering");
    // Cancelling the (absent) preview build is a command on the open project.
    studio.dispatch(CancelPreview);
    studio.wait("command finished", |t| t["busy"] == false);
    assert_eq!(studio.telemetry()["export"]["state"], "rendering");
}

#[gpui::test]
fn the_export_panel_opens_and_settles_on_the_projects_options(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(ExportVideo);
    let settled = studio.wait("export options known", |t| {
        t["export"]["panel_open"] == true
            && t["export"]["support"] != "probing"
            && t["export"]["support"] != "unknown"
    });
    // The fake worker does not answer the capabilities flag.
    assert_eq!(settled["export"]["support"], "unavailable", "{settled:#}");
    // Without a stated choice the export is the default one.
    assert_eq!(
        settled["export"]["settings"],
        serde_json::to_value(fframes_studio_protocol::export::ExportSettings::default()).unwrap(),
        "{settled:#}"
    );
}

#[gpui::test]
fn the_export_panel_does_not_open_without_a_project(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::new(cx);
    studio.dispatch(ExportVideo);
    studio.dispatch(StartExport);
    let settled = studio.settle(Duration::from_millis(300));
    assert_eq!(settled["export"]["panel_open"], false, "{settled:#}");
    assert_eq!(settled["export"]["state"], "idle", "{settled:#}");
}

#[gpui::test]
fn the_export_panel_does_not_open_without_an_sdk(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::without_managed_sdk(cx).open_project();
    studio.dispatch(ExportVideo);
    studio.dispatch(StartExport);
    let settled = studio.settle(Duration::from_millis(300));
    assert_eq!(settled["export"]["panel_open"], false, "{settled:#}");
    assert_eq!(settled["export"]["state"], "idle", "{settled:#}");
}
