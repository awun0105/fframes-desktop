use std::fs;
use studio_engine::{
    Controller, app_paths::AppPaths, build_materialization::sdk_pin, store::Store,
};
use studio_sdk::CompatibilityManifest;

#[test]
fn relocation_relinks_but_duplicate_existing_id_is_never_attached() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("video");
    let paths = AppPaths::new(temp.path().join("history")).unwrap();
    let compatibility = CompatibilityManifest::default_linux_x64();
    let project = studio_project::create(
        &root,
        "Video",
        sdk_pin(&compatibility),
        &compatibility.fframes_version,
        &compatibility.runtime_version,
    )
    .unwrap();
    let mut controller = Controller::open(&root, &paths).unwrap();
    controller.close().unwrap();
    drop(controller);
    let duplicate = temp.path().join("duplicate");
    studio_project::checkpoint::copy_draft(&root, &duplicate).unwrap();
    assert!(
        Controller::open(&duplicate, &paths)
            .err()
            .unwrap()
            .to_string()
            .contains("Duplicate")
    );
    let moved = temp.path().join("moved");
    fs::rename(root, &moved).unwrap();
    let controller = Controller::open(&moved, &paths).unwrap();
    assert!(controller.history_was_available);
    drop(controller);
    let mut store = Store::open(&paths.database()).unwrap();
    assert_eq!(
        store.recents().unwrap()[0].location,
        fs::canonicalize(&moved).unwrap()
    );
    store.remove_recent(&project.manifest.project_id).unwrap();
    assert!(store.recents().unwrap().is_empty());
    assert!(moved.join("studio.json").exists());
    let foreign = AppPaths::new(temp.path().join("other-machine")).unwrap();
    let controller = Controller::open(&moved, &foreign).unwrap();
    assert!(!controller.history_was_available);
}

#[test]
fn a_current_history_reopens_relocates_and_exports_without_rewriting_checkpoints() {
    for relocate in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("video");
        let paths = AppPaths::new(temp.path().join("history")).unwrap();
        let compatibility = CompatibilityManifest::default_linux_x64();
        let project = studio_project::create(
            &root,
            "Video",
            sdk_pin(&compatibility),
            &compatibility.fframes_version,
            &compatibility.runtime_version,
        )
        .unwrap();
        let mut controller = Controller::open(&root, &paths).unwrap();
        let accepted = controller.state().accepted().clone();
        let manifest_path = controller.checkpoints.manifest_path(&accepted);
        let bytes = fs::read(&manifest_path).unwrap();
        controller.close().unwrap();
        drop(controller);
        let opened = if relocate {
            let moved = temp.path().join("moved");
            fs::rename(&root, &moved).unwrap();
            moved
        } else {
            root
        };
        let mut controller = Controller::open(&opened, &paths).unwrap();
        assert!(controller.history_was_available);
        assert_eq!(controller.state().accepted(), &accepted);
        let exported = temp.path().join("export");
        controller.export_checkpoint(&accepted, &exported).unwrap();
        assert_eq!(
            fs::read(exported.join("src/lib.rs")).unwrap(),
            fs::read(opened.join("src/lib.rs")).unwrap()
        );
        assert_ne!(
            studio_project::open(&exported).unwrap().manifest.project_id,
            project.manifest.project_id
        );
        assert_eq!(fs::read(&manifest_path).unwrap(), bytes);
        controller.close().unwrap();
        drop(controller);
        let reopened = Controller::open(&opened, &paths).unwrap();
        assert_eq!(reopened.state().accepted(), &accepted);
        assert_eq!(fs::read(&manifest_path).unwrap(), bytes);
    }
}
