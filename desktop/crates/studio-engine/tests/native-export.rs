use std::fs;
use studio_engine::{Controller, JobKind, JobResult, app_paths::AppPaths};
use studio_sdk::CompatibilityManifest;

fn create_test_project(root: &std::path::Path) {
    let compatibility = CompatibilityManifest::default_linux_x64();
    studio_project::create(
        root,
        "Export source",
        studio_engine::build_materialization::sdk_pin(&compatibility),
        &compatibility.fframes_version,
        &compatibility.runtime_version,
    )
    .unwrap();
}

#[test]
fn saved_revision_must_have_a_successful_preview_build_before_freezing() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    create_test_project(&root);
    let paths = AppPaths::new(temp.path().join("app")).unwrap();
    let controller = Controller::open(&root, &paths).unwrap();

    let error = controller
        .freeze_export_source()
        .err()
        .expect("unbuilt project must be refused")
        .to_string();
    assert!(error.contains("build the current saved revision"));
}

#[test]
fn export_source_is_an_immutable_checkpoint_copy_not_the_live_checkout() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    create_test_project(&root);
    let paths = AppPaths::new(temp.path().join("app")).unwrap();
    let mut controller = Controller::open(&root, &paths).unwrap();
    let source = controller.state().source().clone();
    let build = controller.begin_job(JobKind::Build).unwrap();
    controller
        .complete(&build, JobResult::Built(source.clone()))
        .unwrap();

    let frozen = controller.freeze_export_source().unwrap();
    assert_eq!(frozen.revision(), &source);
    assert_eq!(frozen.project().inventory.revision, source);
    assert_eq!(frozen.label(), "Saved project · preview build verified");

    fs::write(
        root.join("src/lib.rs"),
        "// mutable checkout changed after capture",
    )
    .unwrap();
    assert_ne!(
        fs::read(root.join("src/lib.rs")).unwrap(),
        fs::read(frozen.root().join("src/lib.rs")).unwrap()
    );
    assert_eq!(frozen.project().inventory.revision, *frozen.revision());
}

#[test]
fn export_refuses_a_newer_unvalidated_live_source() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    create_test_project(&root);
    let paths = AppPaths::new(temp.path().join("app")).unwrap();
    let mut controller = Controller::open(&root, &paths).unwrap();
    let source = controller.state().source().clone();
    let build = controller.begin_job(JobKind::Build).unwrap();
    controller
        .complete(&build, JobResult::Built(source))
        .unwrap();
    fs::write(root.join("src/lib.rs"), "// a newer unvalidated source").unwrap();

    let error = controller
        .freeze_export_source()
        .err()
        .expect("changed live source must be refused")
        .to_string();
    assert!(error.contains("differs from the selected export revision"));
}
