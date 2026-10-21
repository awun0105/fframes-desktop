//! Timeline: scene cycling, range selection, zoom, seek from the ruler.
use crate::harness::Studio;
use serde_json::{Value, json};
use studio_ui::actions::{
    CycleScene, MarkRangeEnd, MarkRangeStart, RedoTimelineEdit, Seek, SeekToEnd, SeekToStart,
    SplitAtPlayhead, UndoTimelineEdit, ZoomIn, ZoomOut,
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
    // It shows as a chip in the composer, the message it goes with.
    assert_eq!(
        scoped["panel"]["composer"]["context"],
        "Scene s1 · [45..90)"
    );
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
    assert!(height >= timeline_min(&start), "{:#}", start["split"]);

    // The editor column can settle after the grip first paints (a notice or the status
    // strip arriving), so each limit is read from the same telemetry as the height.
    drag_grip(&mut studio, -30.);
    let taller = studio.wait("a taller timeline", |t| timeline_height(t) != height);
    let expected = (height + 30.).min(timeline_max(&taller));
    assert!(
        (timeline_height(&taller) - expected).abs() < 0.5,
        "{:#}",
        taller["split"]
    );

    drag_grip(&mut studio, -5000.);
    // The player row keeps its minimum: a taller timeline never pushes the column.
    studio.wait("the tallest timeline", |t| {
        let player_row = t["sections"]["player"][3].as_f64().unwrap_or(0.) + 2.;
        (timeline_height(t) - timeline_max(t)).abs() < 0.5
            && (player_row - t["split"]["player_min_height"].as_f64().unwrap()).abs() < 0.5
    });

    drag_grip(&mut studio, 5000.);
    studio.wait("the shortest timeline", |t| {
        (timeline_height(t) - timeline_min(t)).abs() < 0.5
    });
}

/// The tallest timeline the split allows: the player keeps its minimum.
fn timeline_max(t: &serde_json::Value) -> f64 {
    let split = &t["split"];
    split["available"].as_f64().unwrap() - split["player_min_height"].as_f64().unwrap()
}

/// The shortest timeline: its minimum, scaled with the player's when both do not fit.
fn timeline_min(t: &serde_json::Value) -> f64 {
    t["split"]["player_min_height"].as_f64().unwrap() / 320. * 240.
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

#[gpui::test]
fn the_fit_button_fits_a_zoomed_timeline_again(cx: &mut gpui::TestAppContext) {
    let pixels_per_second = |t: &Value| t["timeline"]["pixels_per_second"].as_f64().unwrap();
    let fit_button = |t: &Value| {
        let b = t["buttons"]["timeline-fit"].as_array()?;
        let at = |i: usize| b[i].as_f64().map(|v| v as f32);
        Some((at(0)? + at(2)? / 2., at(1)? + at(3)? / 2.))
    };
    let mut studio = Studio::displayed(cx);
    studio.report_button_bounds();
    let fitted = studio.wait("the fit button measured", |t| fit_button(t).is_some());
    let fitted_scale = pixels_per_second(&fitted);
    studio.dispatch(ZoomIn);
    studio.dispatch(ZoomIn);
    let zoomed = studio.wait("zoomed in", |t| pixels_per_second(t) == 4. * fitted_scale);
    studio.click(fit_button(&zoomed).unwrap());
    studio.wait("fitted again", |t| pixels_per_second(t) == fitted_scale);
}

#[gpui::test]
fn edits_wait_for_a_ready_timeline_and_say_why(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let shown = studio.wait("the edit state is reported", |t| {
        t["timeline"]["edit"]["enabled"].is_boolean()
    });
    let edit = &shown["timeline"]["edit"];
    // This fixture is not timeline-ready; a ready project is covered by the engine tests.
    assert_eq!(edit["enabled"], false, "{edit:#}");
    assert!(
        edit["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "{edit:#}"
    );
    // Splitting and undoing change nothing and raise no error.
    studio.dispatch(SplitAtPlayhead);
    studio.dispatch(UndoTimelineEdit);
    studio.dispatch(RedoTimelineEdit);
    studio.press("secondary-z");
    let after = studio.settle(std::time::Duration::from_millis(300));
    assert!(after["error"].is_null(), "{:#}", after["error"]);
    assert_eq!(after["timeline"]["edit"]["drag"], Value::Null);
}

#[gpui::test]
fn undo_belongs_to_the_text_field_while_one_has_focus(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.settle(std::time::Duration::from_millis(50));
    studio.dispatch(studio_ui::actions::FocusTimeline);
    studio.settle(std::time::Duration::from_millis(50));
    let outside = studio.actions_bound_to("secondary-z");
    assert!(
        outside
            .iter()
            .any(|name| name.ends_with("UndoTimelineEdit")),
        "{outside:?}"
    );
    studio.focus_prompt();
    let in_field = studio.actions_bound_to("secondary-z");
    assert!(
        !in_field
            .iter()
            .any(|name| name.ends_with("UndoTimelineEdit")),
        "{in_field:?}"
    );
}

#[gpui::test]
fn a_split_then_a_seek_does_not_present_frames_out_of_order(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed_with(
        cx,
        r#"{"frames":90,"fps":30,"scenes":[[0,45],[45,90]],"tracks":[[0.0,1.0]],"audio":"tone","timing":true,"vary_pixels":true}"#,
    );
    let ready = studio.wait("editing is enabled", |t| {
        t["timeline"]["edit"]["enabled"] == true
    });
    let before = ready["timeline"]["edit"]["clips"].as_array().unwrap().len();
    studio.dispatch(Seek { frame: 20 });
    studio.wait("frame 20 shown", |t| t["paint_frame"] == 20);
    studio.dispatch(SplitAtPlayhead);
    let split = studio.wait("the split is laid out", |t| {
        t["timeline"]["edit"]["clips"]
            .as_array()
            .is_some_and(|clips| clips.len() == before + 1)
    });
    assert_eq!(split["total_frames"], 90, "{split:#}");
    studio.dispatch(Seek { frame: 60 });
    let after = studio.wait("frame 60 shown after the split", |t| {
        t["position"] == 60 && t["paint_frame"] == 60
    });
    assert_eq!(after["image_metrics"]["out_of_order"], 0, "{after:#}");
    assert!(after["error"].is_null(), "{:#}", after["error"]);
}

/// The center of a shell button the telemetry measured.
fn button(t: &Value, id: &str) -> Option<(f32, f32)> {
    let b = t["buttons"][id].as_array()?;
    let at = |i: usize| b[i].as_f64().map(|v| v as f32);
    Some((at(0)? + at(2)? / 2., at(1)? + at(3)? / 2.))
}

/// Split and zoom in the transport and undo and redo in the header do what their
/// shortcuts do; undo and redo only answer once the timeline reports a step to take.
#[gpui::test]
fn the_transport_and_header_buttons_edit_the_timeline(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed_with(
        cx,
        r#"{"frames":90,"fps":30,"scenes":[[0,45],[45,90]],"tracks":[[0.0,1.0]],"audio":"tone","timing":true}"#,
    );
    studio.report_button_bounds();
    let clips = |t: &Value| t["timeline"]["edit"]["clips"].as_array().map(Vec::len);
    let ready = studio.wait("editing is enabled and the buttons measured", |t| {
        t["timeline"]["edit"]["enabled"] == true
            && [
                "timeline-split",
                "timeline-undo",
                "timeline-redo",
                "timeline-zoom-in",
            ]
            .iter()
            .all(|id| button(t, id).is_some())
    });
    let before = clips(&ready).unwrap();
    let fitted = ready["timeline"]["pixels_per_second"].as_f64().unwrap();
    studio.dispatch(Seek { frame: 20 });
    studio.wait("frame 20 shown", |t| t["paint_frame"] == 20);

    studio.click(button(&ready, "timeline-split").unwrap());
    studio.wait("split by the button", |t| clips(t) == Some(before + 1));
    studio.click(button(&ready, "timeline-undo").unwrap());
    studio.wait("undone by the button", |t| clips(t) == Some(before));
    studio.click(button(&ready, "timeline-redo").unwrap());
    studio.wait("redone by the button", |t| clips(t) == Some(before + 1));

    studio.click(button(&ready, "timeline-zoom-in").unwrap());
    studio.wait("zoomed in by the button", |t| {
        t["timeline"]["pixels_per_second"].as_f64() == Some(2. * fitted)
    });
}
