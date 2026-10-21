//! The settings window: the header's Settings button opens it beside the editor, its pages
//! hold every setting (the theme, audio output, review policy, SDK, presets and export
//! defaults), and Ctrl+W (Cmd+W on macOS) closes it while the editor stays open.
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

const CLOSE_WINDOW: &str = if cfg!(target_os = "macos") {
    "cmd-w"
} else {
    "ctrl-w"
};

#[gpui::test]
fn the_settings_window_saves_the_theme_and_closes_without_the_editor(
    cx: &mut gpui::TestAppContext,
) {
    let mut studio = Studio::open(cx);
    let app_data = studio.app_data.clone();
    studio.report_button_bounds();
    let t = studio.wait("the Settings button measured", |t| {
        center(t, "settings").is_some() && t["header_appearance"] == "light"
    });
    assert_eq!(t["settings_open"], false);
    let settings = center(&t, "settings").unwrap();

    studio.click(settings);
    let t = studio.wait("the settings window", |t| {
        t["settings_open"] == true
            && t["sections"]["settings"][2]
                .as_f64()
                .is_some_and(|w| w > 0.)
            && center(t, "theme-2").is_some()
    });
    let window = studio.settings_window();
    // A second press brings the open window to the front instead of opening another.
    studio.click(settings);
    studio.settle(std::time::Duration::from_millis(100));
    assert_eq!(studio.settings_window(), window);

    studio.in_settings_window();
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

    studio.press(CLOSE_WINDOW);
    studio.in_main_window();
    studio.wait("the settings window closed", |t| {
        t["settings_open"] == false
    });
    assert_eq!(studio.settings_window(), None);
    // The editor's window is still there and opens settings again.
    studio.click(settings);
    studio.wait("the settings window again", |t| t["settings_open"] == true);
}

/// The editor stays in view beside the settings window, so playback goes on.
#[gpui::test]
fn opening_settings_keeps_playing(cx: &mut gpui::TestAppContext) {
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
    assert_eq!(t["playing"], true, "{:#}", t["preview"]);
}

fn panel_center(t: &Value, id: &str) -> Option<(f32, f32)> {
    center(&t["panel"], id)
}

/// Scrolls the page under `at` down until the panel's `id` is inside a window `height` high.
fn scroll_into_view(studio: &mut Studio, at: (f32, f32), height: f32, id: &str) -> Value {
    let visible = |t: &Value| panel_center(t, id).is_some_and(|(_, y)| y > 0. && y < height - 40.);
    for _ in 0..10 {
        let t = studio.settle(std::time::Duration::from_millis(48));
        if visible(&t) {
            return t;
        }
        studio.scroll(at.0, at.1, -200., false);
    }
    studio.wait(&format!("{id} in view"), visible)
}

/// The agent's adapter setup is the settings window's Agent page: the panel's settings
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
    studio.in_settings_window();
    // The field and Save are below the fold of the window: scroll each into view.
    let height = studio.target_height();
    let page = (panel_center(&t, "setup-Label").unwrap().0, height / 2.);
    let t = scroll_into_view(&mut studio, page, height, "setup-Label");
    studio.click(panel_center(&t, "setup-Label").unwrap());
    studio.press("end");
    studio.press("space");
    studio.type_text("two");
    assert_eq!(
        studio.telemetry()["playing"],
        false,
        "Space stayed in the field"
    );
    let t = scroll_into_view(&mut studio, page, height, "agent-save-adapter");
    studio.click(panel_center(&t, "agent-save-adapter").unwrap());
    studio.wait("the label saved", |_| {
        studio_engine::providers::host::load_settings(&paths)
            .ok()
            .flatten()
            .is_some_and(|adapter| adapter.provider == "scripted two")
    });
    assert_eq!(studio.telemetry()["settings_open"], true);
}

fn saved_export(app_data: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(app_data.join("app-settings.json")).ok()?;
    let settings: Value = serde_json::from_str(&text).ok()?;
    Some(settings["export"].clone())
}

/// Opens the settings window from the header and draws in it from then on.
fn open_settings(studio: &mut Studio) -> Value {
    studio.report_button_bounds();
    let t = studio.wait("the Settings button", |t| center(t, "settings").is_some());
    studio.click(center(&t, "settings").unwrap());
    let t = studio.wait("the settings window", |t| {
        t["settings_open"] == true && center(t, "settings-export").is_some()
    });
    studio.in_settings_window();
    t
}

/// Clicks the nav row `id` and waits for its page.
fn show_page(studio: &mut Studio, id: &str, page: &str) {
    let t = studio.telemetry();
    studio.click(center(&t, id).unwrap());
    studio.wait(&format!("the {page} page"), |t| t["settings_page"] == page);
}

/// The audio output, the managed SDK and the style presets moved out of the editor into
/// the General, SDK and Presets pages.
#[gpui::test]
fn the_general_sdk_and_presets_pages_hold_what_left_the_editor(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    open_settings(&mut studio);
    let t = studio.wait("the audio output on the General page", |t| {
        t["settings_page"] == "General" && center(t, "no-audio-output").is_some()
    });
    studio.click(center(&t, "no-audio-output").unwrap());
    studio.wait("no output", |t| t["output_device"] == "Unavailable");
    let t = studio.telemetry();
    studio.click(center(&t, "retry-audio-output").unwrap());
    studio.wait("the default output", |t| t["output_device"] == "Default");

    show_page(&mut studio, "settings-sdk", "Sdk");
    studio.wait("the SDK's Install", |t| center(t, "install").is_some());
    show_page(&mut studio, "settings-presets", "Presets");
}

/// The Export page saves the defaults every export starts from, one choice at a time.
#[gpui::test]
fn the_export_page_saves_the_defaults(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let app_data = studio.app_data.clone();
    open_settings(&mut studio);
    show_page(&mut studio, "settings-export", "Export");
    let want = [
        ("default-format-3", "container", serde_json::json!("webm")),
        ("default-scale-1", "scale", serde_json::json!(0.5)),
        ("default-fps-1", "fps", serde_json::json!(30)),
    ];
    for (id, field, value) in want {
        let t = studio.wait(id, |t| center(t, id).is_some());
        studio.click(center(&t, id).unwrap());
        studio.wait(&format!("{field} saved"), |_| {
            saved_export(&app_data).is_some_and(|export| export[field] == value)
        });
    }
    // Each choice kept the ones before it.
    let export = saved_export(&app_data).unwrap();
    assert_eq!(export["codec"], "vp9", "{export}");
    assert_eq!(export["scale"], 0.5, "{export}");
}

/// The review policy moved from the agent's Project tab to the Agent page.
#[gpui::test]
fn the_agent_page_chooses_the_review_policy(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::with_agent(cx);
    studio.report_button_bounds();
    let t = studio.wait("the panel's settings button", |t| {
        panel_center(t, "agent-open-setup").is_some()
    });
    studio.click(panel_center(&t, "agent-open-setup").unwrap());
    let t = studio.wait("the Review control", |t| {
        t["settings_page"] == "Agent"
            && center(t, "review-policy-1").is_some()
            && t["panel"]["review_policy"] == "AutoApply"
    });
    studio.in_settings_window();
    studio.click(center(&t, "review-policy-1").unwrap());
    let t = studio.wait("review before Apply", |t| {
        t["panel"]["review_policy"] == "ManualReview"
    });
    studio.click(center(&t, "review-policy-0").unwrap());
    studio.wait("automatic Apply", |t| {
        t["panel"]["review_policy"] == "AutoApply"
    });
}
