//! Preset/Style panel: apply and reset (gated: see `Studio::mutation_open`).
use crate::harness::{Mutation, Studio};
use fframes_studio::actions::{ApplyPreset, ResetPreset, SetStyleOverride};
use serde_json::Value;
use std::fs;

fn key_of(t: &Value, id: &str) -> String {
    t["presets"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"].as_str().is_some_and(|i| i.contains(id)))
        .unwrap_or_else(|| panic!("no {id} preset: {:#}", t["presets"]))["key"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn studio_with_presets(cx: &mut gpui::TestAppContext) -> (Studio, String) {
    let mut studio = Studio::open(cx);
    let listed = studio.wait("presets listed", |t| {
        t["presets"]["entries"]
            .as_array()
            .is_some_and(|e| !e.is_empty())
    });
    let key = key_of(&listed, "editorial");
    (studio, key)
}

#[gpui::test]
fn apply_and_reset(cx: &mut gpui::TestAppContext) {
    let (mut studio, key) = studio_with_presets(cx);
    let source = studio.telemetry()["current_source_revision"].clone();
    let style = style_files(&studio);
    studio.dispatch(ApplyPreset { key: key.clone() });
    if !studio.mutation_open(Mutation::Preset) {
        let refused = studio.wait("apply refused", |t| {
            t["busy"] == false && !t["presets"]["status"].is_null()
        });
        assert_blocked(&studio, &refused, &source, &style);
        studio.dispatch(ResetPreset { key });
        let refused = studio.wait("reset refused", |t| t["busy"] == false);
        assert_blocked(&studio, &refused, &source, &style);
        return;
    }
    let applied = studio.wait("preset applied", |t| {
        t["presets"]["applied"]["id"]
            .as_str()
            .is_some_and(|id| id.contains("editorial"))
            && t["busy"] == false
    });
    assert_ne!(applied["current_source_revision"], source);
    assert_ne!(style_files(&studio), style);
    studio.dispatch(SetStyleOverride {
        token: "color.accent".into(),
        value: "#ff0000".into(),
    });
    // The committed override asks for a matching preview, and the shell takes no command
    // while that build (or a refresh after the style file changed) is in flight.
    studio.wait("override set", |t| {
        t["presets"]["applied"]["overrides"] == 1
            && t["busy"] == false
            && t["displayed_revision"] == t["current_source_revision"]
    });
    let overrides = studio.project_file("style/overrides.json");
    assert!(
        fs::read_to_string(&overrides)
            .unwrap()
            .contains("color.accent")
    );
    studio.dispatch(ResetPreset { key });
    studio.wait("overrides reset", |t| {
        t["presets"]["applied"]["overrides"] == 0 && t["busy"] == false
    });
    assert!(
        !fs::read_to_string(&overrides)
            .unwrap()
            .contains("color.accent")
    );
}

/// The project's style files and their bytes.
fn style_files(studio: &Studio) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = fs::read_dir(studio.project_file("style"))
        .map(|entries| {
            entries
                .map(|entry| {
                    let entry = entry.unwrap();
                    let name = entry.file_name().to_string_lossy().into_owned();
                    (name, fs::read(entry.path()).unwrap_or_default())
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// Refused with the gate's reason, shown in the panel and the shell, and nothing written.
fn assert_blocked(studio: &Studio, t: &Value, source: &Value, style: &[(String, Vec<u8>)]) {
    let reason = t["presets"]["blocked"]
        .as_str()
        .unwrap_or_else(|| panic!("presets are not shown as blocked: {:#}", t["presets"]));
    assert!(!reason.is_empty());
    let status = t["presets"]["status"][0].as_str().unwrap();
    assert!(status.starts_with("Not applied:"), "{status}");
    assert!(t["error"].as_str().is_some_and(|e| !e.is_empty()), "{t:#}");
    assert_eq!(&t["current_source_revision"], source);
    assert_eq!(style_files(studio), style);
}
