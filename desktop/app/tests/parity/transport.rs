//! Transport: play/pause and frame stepping.
use crate::harness::Studio;
use fframes_studio::actions::{OpenProject, StepBackward, StepForward, TogglePlayback};
use std::time::Duration;

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
