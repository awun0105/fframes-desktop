//! Timeline: scene cycling, range selection, zoom, seek from the ruler.
use crate::harness::Studio;
use serde_json::{Value, json};
use studio_ui::actions::{
    CycleScene, MarkRangeEnd, MarkRangeStart, Seek, SeekToEnd, SeekToStart, ZoomIn, ZoomOut,
};

fn selection(t: &Value) -> &Value {
    &t["timeline"]["selection"]
}

#[gpui::test]
fn seeking_moves_the_playhead_and_shows_the_frame(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(Seek { frame: 60 });
    let shown = studio.wait("frame 60 shown", |t| {
        t["position"] == 60 && t["paint_frame"] == 60
    });
    assert_eq!(selection(&shown)["position"], 60);
    studio.dispatch(SeekToEnd);
    studio.wait("at the end", |t| t["position"] == 90);
    studio.dispatch(SeekToStart);
    studio.wait("at the start", |t| {
        t["position"] == 0 && t["paint_frame"] == 0
    });
}

#[gpui::test]
fn cycling_selects_the_scene_under_the_playhead(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(Seek { frame: 50 });
    studio.wait("in the second scene", |t| selection(t)["position"] == 50);
    studio.dispatch(CycleScene);
    let selected = studio.wait("scene selected", |t| !selection(t)["scene"].is_null());
    assert_eq!(selection(&selected)["scene"], "scene-1");
    // The selection becomes the scope of the agent's next task.
    let scoped = studio.wait("scene scope", |t| !t["panel"]["submit_scope"].is_null());
    assert_eq!(scoped["panel"]["submit_scope"], "Scene s1 · [45..90)");
}

#[gpui::test]
fn marking_a_range_selects_it(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(Seek { frame: 10 });
    studio.wait("at 10", |t| selection(t)["position"] == 10);
    studio.dispatch(MarkRangeStart);
    studio.dispatch(Seek { frame: 30 });
    studio.wait("at 30", |t| selection(t)["position"] == 30);
    studio.dispatch(MarkRangeEnd);
    let range = studio.wait("range 10..30", |t| selection(t)["range"] == json!([10, 30]));
    assert!(
        range["panel"]["submit_scope"]
            .as_str()
            .is_some_and(|scope| scope.starts_with("Frames [10..")),
        "{:#}",
        range["panel"]
    );
}

#[gpui::test]
fn zoom_doubles_and_halves(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let fitted = studio.telemetry()["timeline"]["pixels_per_second"]
        .as_f64()
        .unwrap();
    studio.dispatch(ZoomIn);
    let zoomed = studio.wait("zoomed in", |t| {
        t["timeline"]["pixels_per_second"].as_f64() != Some(fitted)
    });
    let zoomed = zoomed["timeline"]["pixels_per_second"].as_f64().unwrap();
    assert!((zoomed - 2. * fitted).abs() < 1e-6, "{fitted} -> {zoomed}");
    studio.dispatch(ZoomOut);
    let back = studio.wait("zoomed out", |t| {
        t["timeline"]["pixels_per_second"].as_f64() != Some(zoomed)
    });
    let back = back["timeline"]["pixels_per_second"].as_f64().unwrap();
    assert!((back - fitted).abs() < 1e-6, "{zoomed} -> {back}");
}
