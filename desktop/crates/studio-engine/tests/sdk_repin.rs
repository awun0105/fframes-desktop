//! Moving a project to a newer release of its SDK: offered only for a newer release of the
//! same SDK line, and every refusal leaves `studio.json` byte for byte as it was.
use std::{fs, path::Path};
use studio_engine::{Controller, JobKind, app_paths::AppPaths, controller::repin_offer};
use studio_project::manifest::SdkPin;
use studio_sdk::CompatibilityManifest;

#[path = "support/build_fixture.rs"]
mod fixture;
use fixture::*;

fn release(n: u32) -> CompatibilityManifest {
    let mut sdk = manifest();
    sdk.sdk_id = format!("{}-v{n}", sdk.sdk_id);
    sdk.preview_contract_versions = vec![fframes_studio_protocol::PREVIEW_CONTRACT_VERSION];
    sdk
}

/// A project pinned to release 4 and an installed (incomplete) release 5 at `<temp>/sdk`.
fn pinned_to_four(temp: &Path, next: &CompatibilityManifest) -> (Controller, std::path::PathBuf) {
    let root = temp.join("video");
    let four = release(4);
    studio_project::create(
        &root,
        "Video",
        studio_engine::build_materialization::sdk_pin(&four),
        &four.fframes_version,
        &four.runtime_version,
    )
    .unwrap();
    let sdk = fake_sdk(temp);
    fs::write(
        sdk.join("compatibility.json"),
        serde_json::to_vec(next).unwrap(),
    )
    .unwrap();
    let paths = AppPaths::new(temp.join("data")).unwrap();
    (Controller::open(&root, &paths).unwrap(), sdk)
}

fn studio_json(controller: &Controller) -> Vec<u8> {
    fs::read(controller.project.root.join("studio.json")).unwrap()
}

#[test]
fn only_a_newer_release_of_the_same_sdk_line_that_previews_is_offered() {
    let pin = |sdk: &CompatibilityManifest| studio_engine::build_materialization::sdk_pin(sdk);
    let four = pin(&release(4));
    let offer = repin_offer(&four, &release(5)).expect("v4 -> v5");
    assert_eq!(offer, pin(&release(5)));
    assert_eq!(repin_offer(&four, &release(4)), None, "same release");
    assert_eq!(repin_offer(&four, &release(3)), None, "older release");
    let mut other_line = release(5);
    other_line.sdk_id = "studio-sdk-other-v5".into();
    assert_eq!(repin_offer(&four, &other_line), None);
    let mut no_preview = release(5);
    no_preview.preview_contract_versions.clear();
    assert_eq!(repin_offer(&four, &no_preview), None);
    let unversioned = SdkPin {
        release: manifest().sdk_id,
        compatibility_sha256: four.compatibility_sha256.clone(),
    };
    assert_eq!(repin_offer(&unversioned, &release(5)), None);
}

#[test]
fn every_refused_repin_leaves_studio_json_byte_for_byte() {
    // Framework versions the project does not name.
    let temp = tempfile::tempdir().unwrap();
    let mut other_framework = release(5);
    other_framework.fframes_version = "9.9.9".into();
    let (mut controller, sdk) = pinned_to_four(temp.path(), &other_framework);
    let before = studio_json(&controller);
    let error = controller.repin_sdk(sdk).unwrap_err().to_string();
    assert!(
        error.contains("does not match SDK framework 9.9.9"),
        "{error}"
    );
    assert_eq!(studio_json(&controller), before);

    // An SDK that does not verify (no bundled toolchain).
    let temp = tempfile::tempdir().unwrap();
    let (mut controller, sdk) = pinned_to_four(temp.path(), &release(5));
    let before = studio_json(&controller);
    let error = controller.repin_sdk(sdk.clone()).unwrap_err().to_string();
    assert!(error.contains("SDK unavailable"), "{error}");
    assert_eq!(studio_json(&controller), before);

    // Not a newer release.
    let temp = tempfile::tempdir().unwrap();
    let (mut controller, sdk) = pinned_to_four(temp.path(), &release(3));
    let before = studio_json(&controller);
    let error = controller.repin_sdk(sdk).unwrap_err().to_string();
    assert!(error.contains("not a newer release"), "{error}");
    assert_eq!(studio_json(&controller), before);

    // A running job.
    let temp = tempfile::tempdir().unwrap();
    let (mut controller, sdk) = pinned_to_four(temp.path(), &release(5));
    controller.begin_job(JobKind::Build).unwrap();
    let before = studio_json(&controller);
    let error = controller.repin_sdk(sdk).unwrap_err().to_string();
    assert!(error.contains("job is running"), "{error}");
    assert_eq!(studio_json(&controller), before);
}

#[test]
fn a_newer_format_studio_json_is_refused_and_never_rewritten() {
    let temp = tempfile::tempdir().unwrap();
    let (mut controller, sdk) = pinned_to_four(temp.path(), &release(5));
    let path = controller.project.root.join("studio.json");
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    json["schema_version"] = (json["schema_version"].as_u64().unwrap() + 1).into();
    let newer = serde_json::to_vec_pretty(&json).unwrap();
    fs::write(&path, &newer).unwrap();
    assert!(controller.repin_sdk(sdk).is_err());
    assert_eq!(fs::read(&path).unwrap(), newer);
}

#[test]
#[ignore = "requires SDK_BUNDLE, an assembled SDK bundle"]
fn a_confirmed_repin_moves_only_the_pin_and_the_project_binds_to_the_installed_sdk() {
    let bundle = std::path::PathBuf::from(std::env::var_os("SDK_BUNDLE").expect("SDK_BUNDLE"));
    let installed = CompatibilityManifest::from_json_str(
        &fs::read_to_string(bundle.join("compatibility.json")).unwrap(),
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let artifacts: Vec<_> = installed
        .artifacts
        .iter()
        .map(|a| (a.clone(), bundle.join(a.url.trim_start_matches("file://"))))
        .collect();
    let sdk = studio_sdk::SdkInstaller::new(temp.path().join("sdk"))
        .install_from_local_artifacts(&installed, &artifacts)
        .unwrap();
    let (line, version) = installed.sdk_id.rsplit_once("-v").unwrap();
    let older = SdkPin {
        release: format!("{line}-v{}", version.parse::<u32>().unwrap() - 1),
        compatibility_sha256: "0".repeat(64),
    };
    let root = temp.path().join("video");
    studio_project::create(
        &root,
        "Video",
        older,
        &installed.fframes_version,
        &installed.runtime_version,
    )
    .unwrap();
    let paths = AppPaths::new(temp.path().join("data")).unwrap();
    let mut controller = Controller::open(&root, &paths).unwrap();
    let before: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("studio.json")).unwrap()).unwrap();
    controller.repin_sdk(sdk.clone()).unwrap();

    let after: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("studio.json")).unwrap()).unwrap();
    let pin = studio_engine::build_materialization::sdk_pin(&installed);
    assert_eq!(after["sdk"], serde_json::to_value(&pin).unwrap());
    let (mut before, mut after) = (before, after);
    before.as_object_mut().unwrap().remove("sdk");
    after.as_object_mut().unwrap().remove("sdk");
    assert_eq!(before, after, "only the pin changed");
    assert_eq!(controller.project.manifest.sdk, pin);
    assert_eq!(controller.sdk_path(), Some(sdk.as_path()));
    studio_engine::build_materialization::materialize(
        &controller.project,
        &sdk,
        installed,
        &temp.path().join("builds"),
    )
    .expect("the repinned project binds to the installed SDK");
}
