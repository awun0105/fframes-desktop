//! The installed SDK holds the reviewed Skia archive where `SKIA_BINARIES_URL` points, and a
//! project build never compiles or downloads Skia. `#[ignore]`: they install the assembled
//! bundle named by `SDK_BUNDLE` (see `desktop/scripts/assemble-phase-zero-sdk.py`) and PANIC
//! without it.
//!
//!     SDK_BUNDLE=/path/to/bundle cargo test -p studio-sdk --test installed_skia_archive -- --ignored
use sha2::Digest;
use std::{fs, path::PathBuf, time::Instant};
use studio_bootstrap::{ChildEnvironment, ProcessTreeManager};
use studio_sdk::{CompatibilityManifest, ProjectError, ProjectManager, SdkEnvironment};

fn install(temp: &tempfile::TempDir) -> (PathBuf, CompatibilityManifest) {
    let bundle = PathBuf::from(std::env::var_os("SDK_BUNDLE").expect("SDK_BUNDLE"));
    let manifest = CompatibilityManifest::from_json_str(
        &fs::read_to_string(bundle.join("compatibility.json")).unwrap(),
    )
    .unwrap();
    let artifacts: Vec<_> = manifest
        .artifacts
        .iter()
        .map(|a| (a.clone(), bundle.join(a.url.trim_start_matches("file://"))))
        .collect();
    let sdk = studio_sdk::SdkInstaller::new(temp.path().join("sdk"))
        .install_from_local_artifacts(&manifest, &artifacts)
        .unwrap();
    (sdk, manifest)
}

/// The installed archive of the SDK's target and its reviewed pin.
fn pinned_archive(sdk: &std::path::Path, target: &str) -> (PathBuf, serde_json::Value) {
    let pins: serde_json::Value =
        serde_json::from_str(include_str!("../../../packaging/sdk/skia-binaries.json")).unwrap();
    let pin = pins["archives"][target].clone();
    let key = pin["key"]
        .as_str()
        .expect("a reviewed archive for the SDK's target");
    let url = studio_sdk::skia_binaries_url(sdk).replace("{key}", key);
    (PathBuf::from(url.strip_prefix("file://").unwrap()), pin)
}

#[test]
#[ignore = "requires SDK_BUNDLE, an assembled SDK bundle"]
fn the_installed_sdk_holds_the_pinned_skia_archive_of_its_target() {
    let temp = tempfile::tempdir().unwrap();
    let (sdk, manifest) = install(&temp);
    let (archive, pin) = pinned_archive(&sdk, &manifest.target_triple);
    let bytes = fs::read(&archive).unwrap_or_else(|error| panic!("{}: {error}", archive.display()));
    assert_eq!(bytes.len() as u64, pin["size_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(&bytes)),
        pin["sha256"].as_str().unwrap()
    );
}

#[test]
#[ignore = "requires SDK_BUNDLE, an assembled SDK bundle; compiles a project"]
fn a_project_without_the_archive_fails_fast_instead_of_compiling_skia() {
    let temp = tempfile::tempdir().unwrap();
    let (sdk, manifest) = install(&temp);
    let (archive, _) = pinned_archive(&sdk, &manifest.target_triple);
    fs::rename(&archive, archive.with_extension("moved")).unwrap();
    let project = temp.path().join("project");
    ProjectManager::generate_annotated_worker_project(&project, &sdk).unwrap();
    // A dead proxy: any download attempt fails instead of reaching the network.
    let mut host = ChildEnvironment::default_allowlist();
    host.set("HTTPS_PROXY", "http://127.0.0.1:9")
        .set("HTTP_PROXY", "http://127.0.0.1:9");
    let environment =
        SdkEnvironment::new(&sdk, temp.path().join("target"), manifest, true).freeze(host);
    let processes = ProcessTreeManager::new();
    ProjectManager::ensure_lockfile(&project, &environment, &processes).unwrap();

    let started = Instant::now();
    let error = ProjectManager::build_project(&project, &environment, &processes).unwrap_err();
    let elapsed = started.elapsed();
    let ProjectError::CommandExecutionFailed { stderr, .. } = &error else {
        panic!("{error}");
    };
    assert!(stderr.contains("skia with no-compile feature"), "{stderr}");
    assert!(!stderr.contains("STARTING OFFLINE BUILD"), "{stderr}");
    eprintln!("the project failed after {elapsed:?}");
}
