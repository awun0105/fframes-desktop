//! Transport: play/pause and frame stepping.
use crate::harness::Studio;
use std::time::Duration;
use studio_ui::actions::{OpenProject, StepBackward, StepForward, TogglePlayback};

#[gpui::test]
fn play_advances_the_playhead_and_pause_holds_it(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(TogglePlayback);
    let playing = studio.wait("playback advanced", |t| {
        t["playing"] == true && t["position"].as_u64() > Some(3)
    });
    assert_eq!(playing["playing"], true);
    studio.dispatch(TogglePlayback);
    let paused = studio.wait("paused", |t| t["playing"] == false);
    let held = paused["position"].as_u64().unwrap();
    let later = studio.settle(Duration::from_millis(300));
    assert_eq!(later["playing"], false);
    assert_eq!(later["position"].as_u64(), Some(held));
}

fn renders(t: &serde_json::Value, section: &str) -> u64 {
    t["renders"][section].as_u64().unwrap()
}

#[gpui::test]
fn playback_renders_the_player_and_leaves_the_other_sections(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(TogglePlayback);
    let before = studio.wait("playback under way", |t| {
        t["playing"] == true && t["paint_frame"].as_u64() >= Some(5)
    });
    let from = before["paint_frame"].as_u64().unwrap();
    let after = studio.wait("twenty more frames painted", |t| {
        t["paint_frame"].as_u64() >= Some(from + 20)
    });
    assert_eq!(after["playing"], true);
    let grew = |section| renders(&after, section) - renders(&before, section);
    assert!(grew("player") >= 10, "{:#}", after["renders"]);
    for section in ["header", "sidebar", "agent"] {
        assert!(
            grew(section) <= 1,
            "{section}: {:#} -> {:#}",
            before["renders"],
            after["renders"]
        );
    }
}

#[gpui::test]
fn stepping_moves_one_frame_and_shows_it(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(StepForward);
    studio.dispatch(StepForward);
    let forward = studio.wait("two frames on", |t| {
        t["position"] == 2 && t["paint_frame"] == 2
    });
    assert_eq!(forward["playing"], false);
    studio.dispatch(StepBackward);
    studio.wait("one frame back", |t| {
        t["position"] == 1 && t["paint_frame"] == 1
    });
    studio.dispatch(StepBackward);
    studio.dispatch(StepBackward);
    studio.wait("stops at the first frame", |t| {
        t["position"] == 0 && t["paint_frame"] == 0
    });
}

#[gpui::test]
fn stepping_pauses_playback(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(TogglePlayback);
    studio.wait("playing", |t| t["playing"] == true);
    studio.dispatch(StepForward);
    studio.wait("paused by the step", |t| t["playing"] == false);
}

#[gpui::test]
fn frames_keep_arriving_after_a_cancelled_dialog(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(OpenProject);
    // The dialog is open: the frame this step asks for waits.
    studio.dispatch(StepForward);
    studio.settle(Duration::from_millis(200));
    studio.cancel_path();
    studio.wait("the waiting frame shows", |t| t["paint_frame"] == 1);
    studio.dispatch(StepForward);
    studio.wait("the next frame shows", |t| {
        t["position"] == 2 && t["paint_frame"] == 2
    });
}
