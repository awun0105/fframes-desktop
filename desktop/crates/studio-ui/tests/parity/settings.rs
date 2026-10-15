//! The settings frame: the header's Settings button opens it in the window, its Appearance
//! page picks the theme and saves it with the app's settings, and Back returns to the editor.
use crate::harness::Studio;
use serde_json::Value;
use std::path::Path;

fn center(t: &Value, id: &str) -> Option<(f32, f32)> {
    let b = t["buttons"][id].as_array()?;
    let at = |i: usize| b[i].as_f64().map(|v| v as f32);
    Some((at(0)? + at(2)? / 2., at(1)? + at(3)? / 2.))
}

fn saved_theme(app_data: &Path) -> Option<String> {
    let text = std::fs::read_to_string(app_data.join("app-settings.json")).ok()?;
    let settings: Value = serde_json::from_str(&text).ok()?;
    settings["theme"].as_str().map(str::to_owned)
}

#[gpui::test]
fn the_appearance_page_saves_the_theme_and_back_returns_to_the_editor(
    cx: &mut gpui::TestAppContext,
) {
    let mut studio = Studio::open(cx);
    let app_data = studio.app_data.clone();
    studio.report_button_bounds();
    let t = studio.wait("the Settings button measured", |t| {
        center(t, "settings").is_some() && t["header_appearance"] == "light"
    });
    assert_eq!(t["settings_open"], false);
    let header_renders = t["renders"]["header"].as_u64().unwrap();

    studio.click(center(&t, "settings").unwrap());
    let t = studio.wait("the settings frame", |t| {
        t["settings_open"] == true
            && t["sections"]["settings"][2]
                .as_f64()
                .is_some_and(|w| w > 0.)
            && center(t, "theme-2").is_some()
    });
    assert!(
        t["renders"]["header"].as_u64().unwrap() > header_renders,
        "the cached header renders again with the open frame"
    );

    studio.click(center(&t, "theme-2").unwrap());
    studio.wait("the dark theme saved and drawn", |t| {
        t["header_appearance"] == "dark"
    });
    studio.wait("the dark theme on disk", |_| {
        saved_theme(&app_data).as_deref() == Some("dark")
    });

    let t = studio.telemetry();
    studio.click(center(&t, "theme-0").unwrap());
    studio.wait("the system theme saved and drawn", |t| {
        t["header_appearance"] == "light"
    });
    studio.wait("the system theme on disk", |_| {
        saved_theme(&app_data).as_deref() == Some("system")
    });

    let t = studio.wait("the Back button measured", |t| {
        center(t, "settings-back").is_some()
    });
    studio.click(center(&t, "settings-back").unwrap());
    studio.wait("the editor again", |t| t["settings_open"] == false);
}

/// The editor is out of view while the frame shows, so playback stops when it opens.
#[gpui::test]
fn opening_settings_pauses_playback(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    studio.report_button_bounds();
    studio.dispatch(studio_ui::actions::TogglePlayback);
    let t = studio.wait("playing", |t| {
        t["playing"] == true && center(t, "settings").is_some()
    });
    studio.click(center(&t, "settings").unwrap());
    // Read at once: the clip would also stop by itself at its end.
    let t = studio.telemetry();
    assert_eq!(t["settings_open"], true);
    assert_eq!(t["playing"], false, "{:#}", t["preview"]);
}

fn panel_center(t: &Value, id: &str) -> Option<(f32, f32)> {
    center(&t["panel"], id)
}

/// The agent's adapter setup is the settings frame's Agent page: the panel's settings
/// button opens it, a Space typed in its fields is text (not play/pause), and Save writes
/// what was typed.
#[gpui::test]
fn the_agent_page_edits_and_saves_the_adapter(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::with_agent(cx);
    let paths = studio_engine::app_paths::AppPaths::new(studio.app_data.clone()).unwrap();
    studio.report_button_bounds();
    let t = studio.wait("the panel's settings button", |t| {
        panel_center(t, "agent-open-setup").is_some()
    });
    studio.click(panel_center(&t, "agent-open-setup").unwrap());
    let t = studio.wait("the Agent page", |t| {
        t["settings_open"] == true
            && panel_center(t, "setup-Label").is_some()
            && panel_center(t, "agent-save-adapter").is_some()
    });
    studio.click(panel_center(&t, "setup-Label").unwrap());
    studio.press("end");
    studio.press("space");
    studio.type_text("two");
    assert_eq!(
        studio.telemetry()["playing"],
        false,
        "Space stayed in the field"
    );
    // Save is below the fold of the page: scroll it into view.
    let (x, y) = panel_center(&t, "setup-Label").unwrap();
    studio.scroll(x, y, -800., false);
    let t = studio.wait("Save in view", |t| {
        let height = t["viewport"][1].as_f64().unwrap() as f32;
        panel_center(t, "agent-save-adapter").is_some_and(|(_, y)| y < height - 40.)
    });
    studio.click(panel_center(&t, "agent-save-adapter").unwrap());
    studio.wait("the label saved", |_| {
        studio_engine::providers::host::load_settings(&paths)
            .ok()
            .flatten()
            .is_some_and(|adapter| adapter.provider == "scripted two")
    });
    assert_eq!(studio.telemetry()["settings_open"], true);
}
