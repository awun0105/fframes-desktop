//! Checkpoints: create and restore. Neither writes the project's source (a restore makes a
//! copy), so unlike presets they are not behind the mutation gate on any platform.
use crate::harness::Studio;
use std::{fs, path::PathBuf};
use studio_ui::actions::{RestoreCheckpoint, SaveCheckpoint};

fn edit(studio: &mut Studio, text: &str) -> String {
    fs::write(studio.project_file("src/lib.rs"), text).unwrap();
    let before = studio.telemetry()["current_source_revision"].clone();
    let changed = studio.wait("the edit is seen", |t| {
        t["current_source_revision"] != before && t["busy"] == false
    });
    changed["current_source_revision"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[gpui::test]
fn saving_a_checkpoint_accepts_the_current_source(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    let source = edit(&mut studio, "// checkpointed\n");
    assert_ne!(studio.telemetry()["accepted_revision"], source.as_str());
    studio.dispatch(SaveCheckpoint);
    let saved = studio.wait("checkpoint saved", |t| {
        t["accepted_revision"] == source.as_str() && t["busy"] == false
    });
    assert_eq!(saved["job"], "Succeeded", "{saved:#}");
    let manifest = PathBuf::from(saved["checkpoint_path"].as_str().unwrap());
    assert!(manifest.is_file(), "{}", manifest.display());
}

#[gpui::test]
fn restore_copies_the_last_checkpoint_into_a_new_folder(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    edit(&mut studio, "// checkpointed\n");
    studio.dispatch(SaveCheckpoint);
    studio.wait("checkpoint saved", |t| {
        t["accepted_revision"] == t["current_source_revision"] && t["busy"] == false
    });
    // A later edit is not part of the checkpoint.
    edit(&mut studio, "// after the checkpoint\n");
    let destination = studio.path("restored");
    studio.dispatch(RestoreCheckpoint);
    studio.choose_new_path(&destination);
    let restored = studio.wait("restored", |t| {
        destination.join("src/lib.rs").is_file() && t["busy"] == false
    });
    assert!(restored["error"].is_null(), "{restored:#}");
    assert_eq!(
        fs::read_to_string(destination.join("src/lib.rs")).unwrap(),
        "// checkpointed\n"
    );
    assert!(studio_project::open(&destination).is_ok());
    // The open project is untouched.
    assert_eq!(
        fs::read_to_string(studio.project_file("src/lib.rs")).unwrap(),
        "// after the checkpoint\n"
    );
}

#[gpui::test]
fn a_project_from_an_older_studio_is_refused_and_left_as_it_is(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::new(cx);
    // Projects from before timing documents were generated with instructions version 1.
    let manifest = studio.project_file("studio.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    value["generated_instruction_version"] = 1.into();
    fs::write(&manifest, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let before = fs::read(&manifest).unwrap();
    let project = studio.project_file("");
    studio.dispatch(studio_ui::actions::OpenProject);
    studio.choose_path(&project);
    let refused = studio.wait("open refused", |t| {
        t["busy"] == false && !t["error"].is_null()
    });
    assert_eq!(refused["project_open"], false, "{refused:#}");
    let error = refused["error"].to_string();
    assert!(error.contains("studio.json"), "{error}");
    assert!(error.contains("older Studio"), "{error}");
    assert_eq!(fs::read(&manifest).unwrap(), before);
}
