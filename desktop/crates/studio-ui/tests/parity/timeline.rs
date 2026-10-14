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

fn timeline_height(t: &Value) -> f64 {
    t["split"]["timeline_height"].as_f64().unwrap()
}

fn grip_centre(t: &Value) -> (f32, f32) {
    let grip = &t["split"]["grip_bounds"];
    let at = |i: usize| grip[i].as_f64().unwrap() as f32;
    (at(0) + at(2) / 2., at(1) + at(3) / 2.)
}

/// Drags the resize grip above the timeline by `dy` (negative is up) from its centre.
fn drag_grip(studio: &mut Studio, dy: f32) {
    let (x, y) = grip_centre(&studio.telemetry());
    studio.drag((x, y), (x, y + dy));
}

#[gpui::test]
fn the_grip_lets_go_when_the_button_is_released(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let start = studio.wait("the grip painted", |t| t["split"]["grip_bounds"].is_array());
    let height = timeline_height(&start);
    let (x, y) = grip_centre(&start);

    // A click that never moves releases the grip at once.
    studio.hover((x, y));
    studio.click((x, y));
    assert_eq!(studio.telemetry()["split"]["resizing"], false);
    studio.hover((x, y - 100.));
    studio.dispatch(Seek { frame: 30 });
    let after_click = studio.wait("at 30", |t| selection(t)["position"] == 30);
    assert_eq!(
        timeline_height(&after_click),
        height,
        "{:#}",
        after_click["split"]
    );

    // A release the window never saw (outside it) ends the drag at the next plain move.
    studio.mouse_down((x, y));
    studio.hover((x, y - 100.));
    studio.dispatch(Seek { frame: 40 });
    let after_lost_release = studio.wait("at 40", |t| selection(t)["position"] == 40);
    assert_eq!(after_lost_release["split"]["resizing"], false);
    assert_eq!(
        timeline_height(&after_lost_release),
        height,
        "{:#}",
        after_lost_release["split"]
    );
}

#[gpui::test]
fn the_grip_resizes_the_timeline_within_its_limits(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let start = studio.wait("the grip painted", |t| t["split"]["grip_bounds"].is_array());
    let height = timeline_height(&start);
    let available = start["split"]["available"].as_f64().unwrap();
    let player_min = start["split"]["player_min_height"].as_f64().unwrap();
    let timeline_min = player_min / 320. * 240.;
    assert!(height >= timeline_min, "{:#}", start["split"]);

    drag_grip(&mut studio, -30.);
    let taller = studio.wait("a taller timeline", |t| timeline_height(t) != height);
    let expected = (height + 30.).min(available - player_min);
    assert!(
        (timeline_height(&taller) - expected).abs() < 0.5,
        "{:#}",
        taller["split"]
    );

    drag_grip(&mut studio, -5000.);
    let tallest = studio.wait("the tallest timeline", |t| {
        (timeline_height(t) - (available - player_min)).abs() < 0.5
    });
    assert_eq!(tallest["split"]["available"], start["split"]["available"]);

    drag_grip(&mut studio, 5000.);
    studio.wait("the shortest timeline", |t| {
        (timeline_height(t) - timeline_min).abs() < 0.5
    });
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
