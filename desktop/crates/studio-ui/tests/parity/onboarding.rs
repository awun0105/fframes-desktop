//! The first-run setup: a first launch opens it in place of the workspace, Next walks its
//! steps (the agent after the project), Finish records it with the working-copy permission
//! policy it suggests, and Skip setup records it while keeping "Ask every time".
use crate::harness::Studio;
use serde_json::Value;
use std::path::Path;

fn center(t: &Value, id: &str) -> Option<(f32, f32)> {
    let b = t["buttons"][id].as_array()?;
    let at = |i: usize| b[i].as_f64().map(|v| v as f32);
    Some((at(0)? + at(2)? / 2., at(1)? + at(3)? / 2.))
}

fn saved(app_data: &Path) -> Value {
    std::fs::read_to_string(app_data.join("app-settings.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}

/// Clicks the measured button `id` once the setup shows `step`.
fn click_on_step(studio: &mut Studio, step: &str, id: &str) {
    let t = studio.wait(&format!("{id} on the {step} step"), |t| {
        t["onboarding"] == step && center(t, id).is_some()
    });
    studio.click(center(&t, id).unwrap());
}

#[gpui::test]
fn a_first_launch_walks_the_steps_and_finish_records_the_suggested_policy(
    cx: &mut gpui::TestAppContext,
) {
    let mut studio = Studio::first_run(cx);
    let app_data = studio.app_data.clone();
    studio.report_button_bounds();
    let t = studio.wait("the setup", |t| t["onboarding"] == "welcome");
    assert_eq!(t["permission_policy"], "ask_every_time");

    click_on_step(&mut studio, "welcome", "onboarding-next");
    // The Appearance step is the settings control: picking Dark saves it.
    click_on_step(&mut studio, "appearance", "theme-2");
    studio.wait("the dark theme on disk", |_| {
        saved(&app_data)["theme"] == "dark"
    });
    click_on_step(&mut studio, "appearance", "onboarding-next");
    for (step, next) in [
        ("sdk", "project"),
        ("project", "agent"),
        ("agent", "permissions"),
    ] {
        click_on_step(&mut studio, step, "onboarding-next");
        studio.wait(&format!("the {next} step"), |t| t["onboarding"] == next);
    }
    assert!(
        saved(&app_data)["onboarding_completed"].is_null(),
        "nothing is recorded before Finish"
    );
    click_on_step(&mut studio, "permissions", "onboarding-next");
    let t = studio.wait("the workspace", |t| t["onboarding"].is_null());
    assert_eq!(t["permission_policy"], "allow_in_working_copy");
    studio.wait("the setup on disk", |_| {
        let settings = saved(&app_data);
        settings["onboarding_completed"] == true
            && settings["permission_policy"] == "allow_in_working_copy"
            && settings["theme"] == "dark"
    });
}

#[gpui::test]
fn skip_setup_records_it_and_keeps_asking_every_time(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::first_run(cx);
    let app_data = studio.app_data.clone();
    studio.report_button_bounds();
    click_on_step(&mut studio, "welcome", "onboarding-skip");
    let t = studio.wait("the workspace", |t| t["onboarding"].is_null());
    assert_eq!(t["permission_policy"], "ask_every_time");
    studio.wait("the skip on disk", |_| {
        let settings = saved(&app_data);
        settings["onboarding_completed"] == true
            && settings["permission_policy"] == "ask_every_time"
    });
}

#[gpui::test]
fn a_recorded_setup_opens_the_workspace(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::new(cx);
    let t = studio.wait("app storage", |t| t["busy"] == false);
    assert!(t["onboarding"].is_null(), "{t:#}");
}
