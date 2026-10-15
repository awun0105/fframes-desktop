use std::fs;
use studio_engine::build_materialization::{materialize, sdk_pin};
use studio_sdk::CompatibilityManifest;

#[test]
fn overlay_keeps_source_and_sdk_unchanged_and_rejects_external_dependencies() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("video");
    let sdk = temp.path().join("sdk");
    let compatibility = CompatibilityManifest::default_linux_x64();
    for name in [
        "fframes",
        "fframes-studio-runtime",
        "fframes-studio-protocol",
    ] {
        let dir = sdk.join("framework/framework").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Cargo.toml"), "[package]").unwrap();
    }
    fs::create_dir_all(sdk.join("framework/vendor")).unwrap();
    let project = studio_project::create(
        &root,
        "Video",
        sdk_pin(&compatibility),
        &compatibility.fframes_version,
        compatibility.studio_runtime_version(),
    )
    .unwrap();
    let build = materialize(
        &project,
        &sdk,
        compatibility.clone(),
        &temp.path().join("builds"),
    )
    .unwrap();
    assert_eq!(
        studio_project::open(&root).unwrap().inventory,
        project.inventory
    );
    assert!(!root.join(".cargo").exists());
    assert!(
        build
            .environment
            .target_dir
            .starts_with(temp.path().join("builds"))
    );
    assert_eq!(
        build.root.parent().unwrap().parent().unwrap(),
        temp.path().join("builds")
    );
    assert!(
        fs::read_to_string(build.root.join("Cargo.toml"))
            .unwrap()
            .contains("path =")
    );
    let cargo = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        format!("{cargo}\n[dependencies.outside]\npath = '../sdk'\n"),
    )
    .unwrap();
    let project = studio_project::open(&root).unwrap();
    assert!(materialize(&project, &sdk, compatibility, &temp.path().join("builds")).is_err());
}

#[test]
fn contained_dependency_with_absolute_path_is_rewritten_to_copied_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    let sdk = temp.path().join("sdk");
    let compatibility = CompatibilityManifest::default_linux_x64();
    for name in [
        "fframes",
        "fframes-studio-runtime",
        "fframes-studio-protocol",
    ] {
        let dir = sdk.join("framework/framework").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Cargo.toml"), "[package]").unwrap();
    }
    fs::create_dir_all(sdk.join("framework/vendor")).unwrap();

    let _project = studio_project::create(
        &root,
        "Video",
        sdk_pin(&compatibility),
        &compatibility.fframes_version,
        compatibility.studio_runtime_version(),
    )
    .unwrap();

    let helper = root.join("helper");
    fs::create_dir_all(&helper).unwrap();
    fs::write(
        helper.join("Cargo.toml"),
        "[package]\nname = \"helper\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(helper.join("lib.rs"), "pub fn help() {}\n").unwrap();

    let cargo = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "{cargo}\n[dependencies.helper]\npath = {}\n",
            toml::Value::String(helper.to_string_lossy().into_owned())
        ),
    )
    .unwrap();

    let project = studio_project::open(&root).unwrap();
    let build = materialize(
        &project,
        &sdk,
        compatibility.clone(),
        &temp.path().join("builds"),
    )
    .unwrap();

    let materialized_cargo: toml::Value = fs::read_to_string(build.root.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let copied_helper = build.root.join("helper");
    let rewritten_path = materialized_cargo["dependencies"]["helper"]["path"]
        .as_str()
        .unwrap();
    assert_eq!(
        std::path::Path::new(rewritten_path),
        copied_helper.as_path()
    );
}

#[test]
fn cancelled_copy_removes_partial_tree_and_preserves_captured_source() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("video");
    let sdk = temp.path().join("sdk");
    fs::create_dir(&sdk).unwrap();
    let compatibility = CompatibilityManifest::default_linux_x64();
    studio_project::create(
        &root,
        "Video",
        sdk_pin(&compatibility),
        &compatibility.fframes_version,
        compatibility.studio_runtime_version(),
    )
    .unwrap();
    let bytes = vec![93; 2 * 1024 * 1024];
    fs::write(root.join("media/large.bin"), &bytes).unwrap();
    let project = studio_project::open(&root).unwrap();
    let calls = std::cell::Cell::new(0);
    let builds = temp.path().join("builds");
    let result = studio_engine::build_materialization::materialize_with_cancel(
        &project,
        &sdk,
        compatibility,
        &builds,
        &|| {
            calls.set(calls.get() + 1);
            calls.get() >= 4
        },
    );
    assert!(result.err().unwrap().to_string().contains("cancelled"));
    assert_eq!(fs::read(root.join("media/large.bin")).unwrap(), bytes);
    assert_eq!(
        studio_project::open(&root).unwrap().inventory,
        project.inventory
    );
    fn check(path: &std::path::Path) {
        for e in fs::read_dir(path).unwrap() {
            let path = e.unwrap().path();
            if path.is_dir() {
                assert!(
                    !path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("build-")
                );
                check(&path);
            }
        }
    }
    check(&builds);
}

#[test]
fn projects_bind_to_the_framework_their_pin_and_instructions_select() {
    let temp = tempfile::tempdir().unwrap();
    let sdk = temp.path().join("sdk");
    let builds = temp.path().join("builds");
    let mut compatibility = CompatibilityManifest::default_linux_x64();
    compatibility.fframes_version = "9.0.0".into();
    compatibility.runtime_version = Some("9.1.0".into());
    compatibility.legacy = Some(studio_sdk::manifest::LegacyFramework {
        fframes_version: "1.2.0".into(),
        runtime_version: "0.1.0".into(),
        framework_rel_path: "framework/framework-legacy".into(),
        sdk_releases: vec!["studio-sdk-earlier".into()],
    });
    compatibility.validate().unwrap();
    for tree in ["framework/framework", "framework/framework-legacy"] {
        for name in [
            "fframes",
            "fframes-studio-runtime",
            "fframes-studio-protocol",
        ] {
            let dir = sdk.join(tree).join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("Cargo.toml"), "[package]").unwrap();
        }
    }
    fs::create_dir_all(sdk.join("framework/vendor")).unwrap();
    let fframes_path = |project: &studio_project::OpenProject| {
        let build = materialize(project, &sdk, compatibility.clone(), &builds)?;
        let cargo: toml::Value = fs::read_to_string(build.root.join("Cargo.toml"))
            .unwrap()
            .parse()
            .unwrap();
        let path = cargo["dependencies"]["fframes"]["path"].as_str().unwrap();
        Ok::<_, studio_project::ProjectError>(std::path::PathBuf::from(path))
    };
    let current_sdk = fs::canonicalize(&sdk).unwrap();

    let current = studio_project::create(
        &temp.path().join("current"),
        "Video",
        sdk_pin(&compatibility),
        "9.0.0",
        "9.1.0",
    )
    .unwrap();
    assert_eq!(
        fframes_path(&current).unwrap(),
        current_sdk.join("framework/framework/fframes")
    );

    // A project from an earlier SDK, before migration.
    let root = temp.path().join("legacy");
    let mut legacy =
        studio_project::create(&root, "Video", sdk_pin(&compatibility), "1.2.0", "0.1.0").unwrap();
    legacy.manifest.generated_instruction_version = 1;
    legacy.manifest.sdk.release = "studio-sdk-earlier".into();
    studio_project::lifecycle::write_manifest(&root, &legacy.manifest).unwrap();
    let legacy = studio_project::open(&root).unwrap();
    assert_eq!(
        fframes_path(&legacy).unwrap(),
        current_sdk.join("framework/framework-legacy/fframes")
    );

    let cargo = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    for (edited, reason) in [
        (
            cargo.replace("=0.1.0", "=9.1.0"),
            "mixed framework releases",
        ),
        (
            cargo.replace("version = \"=1.2.0\"\n", ""),
            "a framework crate without a version",
        ),
    ] {
        assert_ne!(edited, cargo, "{reason}");
        fs::write(root.join("Cargo.toml"), edited).unwrap();
        let project = studio_project::open(&root).unwrap();
        assert!(fframes_path(&project).is_err(), "{reason}");
    }
    fs::write(root.join("Cargo.toml"), &cargo).unwrap();

    for (instructions, release) in [(1, "studio-sdk-unknown"), (2, "studio-sdk-earlier")] {
        let mut manifest = legacy.manifest.clone();
        manifest.generated_instruction_version = instructions;
        manifest.sdk.release = release.into();
        studio_project::lifecycle::write_manifest(&root, &manifest).unwrap();
        let project = studio_project::open(&root).unwrap();
        let error = fframes_path(&project).unwrap_err().to_string();
        assert!(error.contains("SDK pin mismatch"), "{error}");
    }
}
