//! Keyboard: the keys the preview and the timeline answer when they have focus, pressed
//! as real keystrokes through the keymap.
use crate::harness::Studio;
use serde_json::{Value, json};
use std::time::Duration;
use studio_ui::actions::{FocusPreview, FocusTimeline, Seek, SelectCanvasAt, ZoomIn};

/// A title and a badge painted over its right part, in a 640x360 video.
const OBJECTS: &str = r#"{"frames":90,"width":640,"height":360,"export_hang":true,
    "editor_objects":[
        {"component":"intro","object":"title","bounds":[100,100,200,50]},
        {"component":"intro","object":"badge","bounds":[250,110,40,20]}]}"#;

fn selection(t: &Value) -> &Value {
    &t["timeline"]["selection"]
}

fn selected_object(t: &Value) -> Option<&str> {
    t["canvas"]["selection"]["identity"]["object_key"].as_str()
}

fn pixels_per_second(t: &Value) -> f64 {
    t["timeline"]["pixels_per_second"].as_f64().unwrap()
}

fn focus(studio: &mut Studio, action: impl gpui::Action) {
    studio.dispatch(action);
    studio.settle(Duration::from_millis(50));
}

#[gpui::test]
fn the_preview_keys_play_step_and_seek(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    focus(&mut studio, FocusPreview);
    studio.press("right right");
    studio.wait("two frames on", |t| {
        t["position"] == 2 && t["paint_frame"] == 2
    });
    studio.press("left");
    studio.wait("one frame back", |t| {
        t["position"] == 1 && t["paint_frame"] == 1
    });
    studio.press("end");
    studio.wait("at the end", |t| t["position"] == 90);
    studio.press("home");
    studio.wait("at the start", |t| {
        t["position"] == 0 && t["paint_frame"] == 0
    });
    studio.press("space");
    studio.wait("playing", |t| t["playing"] == true);
    studio.press("space");
    studio.wait("paused", |t| t["playing"] == false);
}

#[gpui::test]
fn the_preview_keys_cycle_clear_and_reset_the_canvas(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed_with(cx, OBJECTS);
    studio.wait("canvas objects", |t| t["canvas"]["object_count"] == 2);
    studio.dispatch(SelectCanvasAt {
        x: 260.,
        y: 115.,
        cycle: false,
    });
    studio.wait("badge on top", |t| selected_object(t) == Some("badge"));
    focus(&mut studio, FocusPreview);
    studio.press("c");
    studio.wait("the title under it", |t| {
        selected_object(t) == Some("title")
    });
    studio.press("escape");
    studio.wait("nothing selected", |t| selected_object(t).is_none());

    let fitted = studio.telemetry();
    let origin = &fitted["canvas"]["canvas_origin"];
    let extent = &fitted["canvas"]["canvas_extent"];
    let centre =
        |axis: usize| (origin[axis].as_f64().unwrap() + extent[axis].as_f64().unwrap() / 2.) as f32;
    let fitted = fitted["canvas"]["image_bounds"].clone();
    studio.scroll(centre(0), centre(1), 40., true);
    studio.wait("zoomed in", |t| t["canvas"]["image_bounds"] != fitted);
    studio.press("0");
    studio.wait("fitted again", |t| t["canvas"]["image_bounds"] == fitted);
}

#[gpui::test]
fn the_timeline_keys_drive_playback_and_selection(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    focus(&mut studio, FocusTimeline);
    studio.press("right right");
    studio.wait("two frames on", |t| t["position"] == 2);
    studio.press("left");
    studio.wait("one frame back", |t| t["position"] == 1);
    studio.press("end");
    studio.wait("at the end", |t| t["position"] == 90);
    studio.press("home");
    studio.wait("at the start", |t| t["position"] == 0);
    studio.press("m");
    studio.wait("muted", |t| t["muted"] == true);
    studio.press("m");
    studio.wait("unmuted", |t| t["muted"] == false);
    studio.press("space");
    studio.wait("playing", |t| t["playing"] == true);
    studio.press("space");
    studio.wait("paused", |t| t["playing"] == false);

    studio.dispatch(Seek { frame: 50 });
    studio.wait("in the second scene", |t| selection(t)["position"] == 50);
    studio.press("s");
    let selected = studio.wait("scene selected", |t| !selection(t)["scene"].is_null());
    assert_eq!(selection(&selected)["scene"], "scene-1");

    studio.dispatch(Seek { frame: 10 });
    studio.wait("at 10", |t| selection(t)["position"] == 10);
    studio.press("[");
    studio.dispatch(Seek { frame: 30 });
    studio.wait("at 30", |t| selection(t)["position"] == 30);
    studio.press("]");
    studio.wait("range 10..30", |t| selection(t)["range"] == json!([10, 30]));
}

#[gpui::test]
fn the_timeline_keys_zoom_and_page(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    focus(&mut studio, FocusTimeline);
    let fitted = pixels_per_second(&studio.telemetry());
    studio.press("=");
    studio.wait("zoomed in", |t| pixels_per_second(t) == 2. * fitted);
    studio.press("-");
    studio.wait("zoomed out", |t| pixels_per_second(t) == fitted);
    studio.press("+");
    studio.wait("zoomed in by plus", |t| pixels_per_second(t) == 2. * fitted);

    studio.dispatch(ZoomIn);
    studio.wait("zoomed in twice", |t| pixels_per_second(t) == 4. * fitted);
    let start = studio.telemetry()["timeline"]["scroll_x"].as_f64().unwrap();
    studio.press("pagedown");
    let paged = studio.wait("paged forward", |t| {
        t["timeline"]["scroll_x"].as_f64().unwrap() > start
    });
    let paged = paged["timeline"]["scroll_x"].as_f64().unwrap();
    studio.press("pageup");
    studio.wait("paged back", |t| {
        t["timeline"]["scroll_x"].as_f64().unwrap() < paged
    });
}
