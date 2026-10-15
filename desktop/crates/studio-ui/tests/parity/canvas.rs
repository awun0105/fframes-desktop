//! Canvas: `select_at` and the mapping from a click on the video to the source object.
use crate::harness::Studio;
use serde_json::Value;
use studio_ui::actions::SelectCanvasAt;

/// A title and a badge painted over its right part, in a 640x360 video.
const OBJECTS: &str = r#"{"frames":90,"width":640,"height":360,"export_hang":true,
    "editor_objects":[
        {"component":"intro","object":"title","bounds":[100,100,200,50]},
        {"component":"intro","object":"badge","bounds":[250,110,40,20]}]}"#;

fn selected(t: &Value) -> Option<&str> {
    t["canvas"]["selection"]["identity"]["object_key"].as_str()
}

fn click(x: f64, y: f64) -> SelectCanvasAt {
    SelectCanvasAt { x, y, cycle: false }
}

#[gpui::test]
fn a_click_selects_the_object_under_it(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed_with(cx, OBJECTS);
    studio.wait("canvas objects", |t| t["canvas"]["object_count"] == 2);
    studio.dispatch(click(120., 120.));
    let selected_title = studio.wait("title selected", |t| selected(t) == Some("title"));
    assert_eq!(
        selected_title["canvas"]["selection"]["identity"]["component_key"],
        "intro"
    );
    // The selected object becomes the scope of the agent's next task.
    let scoped = studio.wait("element scope", |t| {
        t["panel"]["submit_scope"]
            .as_str()
            .is_some_and(|scope| scope.contains("object=title"))
    });
    assert!(
        scoped["panel"]["submit_scope"]
            .as_str()
            .unwrap()
            .starts_with("Element scene=scene-0 component=intro object=title"),
        "{:#}",
        scoped["panel"]
    );
    studio.dispatch(click(20., 20.));
    studio.wait("nothing selected", |t| selected(t).is_none());
}

#[gpui::test]
fn overlapping_objects_select_the_top_one_then_cycle(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed_with(cx, OBJECTS);
    studio.wait("canvas objects", |t| t["canvas"]["object_count"] == 2);
    studio.dispatch(click(260., 115.));
    studio.wait("badge on top", |t| selected(t) == Some("badge"));
    studio.dispatch(SelectCanvasAt {
        x: 260.,
        y: 115.,
        cycle: true,
    });
    studio.wait("title under it", |t| selected(t) == Some("title"));
}

#[gpui::test]
fn selecting_pauses_playback(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed_with(cx, OBJECTS);
    studio.dispatch(studio_ui::actions::TogglePlayback);
    studio.wait("playing", |t| t["playing"] == true);
    studio.dispatch(click(120., 120.));
    studio.wait("paused on a selection", |t| {
        t["playing"] == false && selected(t) == Some("title")
    });
}

#[gpui::test]
fn a_click_on_an_empty_stage_reports_nothing(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    let t = studio.telemetry();
    assert!(t["displayed_revision"].is_null());
    let at = |i: usize| {
        (t["canvas"]["canvas_origin"][i].as_f64().unwrap()
            + t["canvas"]["canvas_extent"][i].as_f64().unwrap() / 2.) as f32
    };
    studio.click((at(0), at(1)));
    studio.press("c");
    let t = studio.settle(std::time::Duration::from_millis(200));
    assert!(t["canvas"]["message"].is_null(), "{}", t["canvas"]);
}
