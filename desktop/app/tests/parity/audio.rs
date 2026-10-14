//! Audio: mute and output switching. Real playback through a device is a manual check;
//! these pin the shell's choice of output against the device list the audio service reports
//! (empty on hosts without audio hardware).
use crate::harness::Studio;
use fframes_studio::actions::{
    DisableAudioOutput, NextAudioOutput, RetryAudioOutput, ToggleMute, TogglePlayback,
};

#[gpui::test]
fn mute_toggles(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    assert_eq!(studio.telemetry()["muted"], false);
    studio.dispatch(ToggleMute);
    studio.wait("muted", |t| t["muted"] == true);
    studio.dispatch(ToggleMute);
    studio.wait("unmuted", |t| t["muted"] == false);
}

#[gpui::test]
fn next_output_walks_the_device_list(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let start = studio.telemetry();
    assert_eq!(start["output_device"], "Default");
    let devices: Vec<String> = serde_json::from_value(start["output_devices"].clone()).unwrap();
    // Default, then every device in order, then none, then Default again.
    let mut expected: Vec<String> = devices.iter().map(|d| format!("Named({d:?})")).collect();
    expected.push("Unavailable".into());
    expected.push("Default".into());
    for want in expected {
        studio.dispatch(NextAudioOutput);
        studio.wait(&format!("output {want}"), |t| {
            t["output_device"] == want.as_str()
        });
    }
}

#[gpui::test]
fn no_output_plays_on_the_monotonic_clock_and_retry_returns_to_default(
    cx: &mut gpui::TestAppContext,
) {
    let mut studio = Studio::displayed(cx);
    studio.dispatch(DisableAudioOutput);
    studio.wait("no output", |t| t["output_device"] == "Unavailable");
    studio.dispatch(TogglePlayback);
    let playing = studio.wait("playing without audio", |t| {
        t["playing"] == true && t["position"].as_u64() > Some(3)
    });
    assert_eq!(playing["audio_ready"], false, "{playing:#}");
    studio.dispatch(TogglePlayback);
    studio.dispatch(RetryAudioOutput);
    studio.wait("default output", |t| t["output_device"] == "Default");
}
