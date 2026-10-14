//! The setup screen (the SDK it reports, selecting or importing one) and Detect agents.
use crate::harness::{Studio, fake_rustc};
use serde_json::Value;
use std::{fs, path::PathBuf, time::Duration};
use studio_ui::actions::{DetectAgents, ImportSdkBundle, SelectSdk, UseDetectedAgent};

fn sdk_status(t: &Value) -> &str {
    t["sdk"].as_str().unwrap_or_default()
}

#[gpui::test]
fn an_installed_sdk_is_available_to_the_agent(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    let t = studio.telemetry();
    assert!(sdk_status(&t).starts_with("Available"), "{t:#}");
    assert_eq!(t["sdk_ready_for_agent"], true);
}

#[gpui::test]
fn the_shell_reports_the_version_it_was_given(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::new(cx);
    assert_eq!(studio.telemetry()["app_version"], env!("CARGO_PKG_VERSION"));
}

#[gpui::test]
fn without_a_managed_sdk_one_is_selected(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::without_managed_sdk(cx).open_project();
    let missing = studio.telemetry();
    assert!(
        !sdk_status(&missing).starts_with("Available"),
        "{missing:#}"
    );
    assert_eq!(missing["sdk_ready_for_agent"], false);
    studio.dispatch(SelectSdk);
    let sdk = studio.sdk.clone();
    studio.choose_path(&sdk);
    let selected = studio.wait("SDK selected", |t| {
        sdk_status(t).starts_with("Available") && t["busy"] == false
    });
    assert_eq!(selected["sdk_ready_for_agent"], true, "{selected:#}");
}

#[gpui::test]
fn a_broken_sdk_bundle_is_reported(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::without_managed_sdk(cx);
    let bundle = studio.path("not-a-bundle");
    fs::create_dir_all(&bundle).unwrap();
    studio.dispatch(ImportSdkBundle);
    studio.choose_path(&bundle);
    let failed = studio.wait("import failed", |t| {
        t["busy"] == false && !t["error"].is_null()
    });
    assert!(sdk_status(&failed).starts_with("Failed"), "{failed:#}");
    assert!(!studio.path("sdk-home/active").exists());
}

/// The ACP adapter of the built-in Antigravity profile, placed in the app's adapters folder.
fn install_adapter(studio: &Studio) -> PathBuf {
    let adapters = studio.app_data.join("adapters");
    fs::create_dir_all(&adapters).unwrap();
    let adapter = adapters.join(format!("antigravity-acp{}", std::env::consts::EXE_SUFFIX));
    fs::copy(fake_rustc(), &adapter).unwrap();
    // As the app names it: app data is resolved to its canonical path.
    adapter.canonicalize().unwrap()
}

fn detected<'a>(t: &'a Value, profile: &str) -> &'a Value {
    t["panel"]["setup"]["detected"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["profile"] == profile))
        .unwrap_or(&Value::Null)
}

fn scanned(t: &Value) -> bool {
    t["panel"]["setup"]["scanning"] == false
        && t["panel"]["setup"]["detected"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
}

#[gpui::test]
fn a_detected_adapter_is_turned_on(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    studio.wait("agent panel attached", |t| t["panel"]["attached"] == true);
    let adapter = install_adapter(&studio);
    studio.dispatch(DetectAgents);
    let found = studio.wait("agents detected", scanned);
    let row = detected(&found, "antigravity");
    assert_eq!(row["state"], "Ready", "{row:#}");
    assert_eq!(
        row["adapter"],
        adapter.file_name().unwrap().to_string_lossy().as_ref()
    );

    studio.dispatch(UseDetectedAgent {
        profile: "antigravity".into(),
    });
    let selected = studio.wait("adapter turned on", |t| {
        t["panel"]["setup"]["selected_profile"] == "antigravity"
            && t["panel"]["setup"]["turn_on"]
                .as_str()
                .is_some_and(|s| s.starts_with("Testing"))
    });
    assert!(
        selected["panel"]["setup"]["turn_on"]
            .as_str()
            .is_some_and(|s| s.contains("antigravity")),
        "{:#}",
        selected["panel"]["setup"]
    );
    // Testing starts once the registry is saved.
    let saved: Value =
        serde_json::from_slice(&fs::read(studio.app_data.join("provider-registry.json")).unwrap())
            .unwrap();
    assert_eq!(saved["selected_profile_id"], "antigravity");
    let profile = saved["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|profile| profile["id"] == "antigravity")
        .unwrap();
    assert_eq!(
        profile["adapter"]["executable"],
        adapter.to_string_lossy().as_ref()
    );
}

#[gpui::test]
fn an_agent_that_was_not_detected_is_not_turned_on(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    studio.wait("agent panel attached", |t| t["panel"]["attached"] == true);
    studio.dispatch(DetectAgents);
    let found = studio.wait("agents detected", scanned);
    assert_ne!(detected(&found, "antigravity")["state"], "Ready");
    studio.dispatch(UseDetectedAgent {
        profile: "antigravity".into(),
    });
    let refused = studio.settle(Duration::from_millis(200));
    assert_eq!(refused["panel"]["setup"]["turn_on"], "Idle");
    assert_ne!(refused["panel"]["setup"]["selected_profile"], "antigravity");
    assert!(!studio.app_data.join("provider-registry.json").exists());
}
