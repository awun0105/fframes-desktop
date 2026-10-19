use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use studio_project::{
    OpenProject, ProjectError, ProjectPath, SourceInventory, lifecycle::read_cargo,
    revision::TIMING_FILE,
};
use studio_sdk::{
    CompatibilityManifest, environment::SdkEnvironment, manifest::FRAMEWORK_REL_PATH,
};

pub struct MaterializedBuild {
    pub root: PathBuf,
    pub environment: SdkEnvironment,
    pub package: String,
    pub worker_target: String,
    pub manifest: PathBuf,
    pub isolated_bin_dir: PathBuf,
    // The worker and any frame/audio consumers retain this lease. Never prune a live tree.
    _directory: tempfile::TempDir,
}

pub fn sdk_pin(manifest: &CompatibilityManifest) -> studio_project::manifest::SdkPin {
    studio_project::manifest::SdkPin {
        release: manifest.sdk_id.clone(),
        compatibility_sha256: manifest.digest(),
    }
}

/// The framework crates inside an SDK that one project builds against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameworkSet<'a> {
    pub fframes_version: &'a str,
    /// Version of `fframes-studio-runtime` and `fframes-studio-protocol`.
    pub runtime_version: &'a str,
    /// The crates' directory inside the SDK.
    pub rel_path: &'a str,
}

/// The framework `project` builds against in the SDK described by `compatibility`, or
/// `None` when the project is pinned to another SDK.
pub fn framework_for<'a>(
    project: &studio_project::manifest::Manifest,
    compatibility: &'a CompatibilityManifest,
) -> Option<FrameworkSet<'a>> {
    (project.sdk == sdk_pin(compatibility)).then_some(FrameworkSet {
        fframes_version: &compatibility.fframes_version,
        runtime_version: &compatibility.runtime_version,
        rel_path: FRAMEWORK_REL_PATH,
    })
}

/// Whether every `Cargo.toml` of `project` binds to the framework of the SDK at `sdk` as it
/// would at build time (exact framework versions, crates present), whatever the project is
/// pinned to. Reads the live files; nothing is written.
pub fn check_binding(
    project: &OpenProject,
    sdk: &Path,
    compatibility: &CompatibilityManifest,
) -> Result<(), ProjectError> {
    let framework = FrameworkSet {
        fframes_version: &compatibility.fframes_version,
        runtime_version: &compatibility.runtime_version,
        rel_path: FRAMEWORK_REL_PATH,
    };
    for file in &project.inventory.files {
        if file.path.as_str().rsplit('/').next() != Some("Cargo.toml") {
            continue;
        }
        let mut cargo = read_cargo(&project.root, &file.path)?;
        bind(
            &mut cargo,
            &project.root,
            &project.root,
            Path::new(file.path.as_str()).parent().unwrap(),
            &sdk.join(framework.rel_path),
            framework,
        )?;
    }
    Ok(())
}

/// SDK binding never mutates portable source, its lock/config, or the installed SDK.
pub fn materialize(
    project: &OpenProject,
    sdk: &Path,
    compatibility: CompatibilityManifest,
    app_builds: &Path,
) -> Result<MaterializedBuild, ProjectError> {
    materialize_with_cancel(project, sdk, compatibility, app_builds, &|| false)
}

/// Cargo target directory shared by every build with the same compatibility manifest.
pub fn target_dir(app_builds: &Path, compatibility: &CompatibilityManifest) -> PathBuf {
    app_builds
        .join("targets")
        .join(compatibility.digest())
        .join(&compatibility.target_triple)
}

pub fn materialize_with_cancel(
    project: &OpenProject,
    sdk: &Path,
    compatibility: CompatibilityManifest,
    app_builds: &Path,
    cancelled: &impl Fn() -> bool,
) -> Result<MaterializedBuild, ProjectError> {
    let sdk = fs::canonicalize(sdk).map_err(|e| {
        ProjectError::new(
            sdk,
            "managed build",
            e.to_string(),
            "Choose a compatible SDK or copy/relink dependencies inside the workspace",
        )
    })?;
    let target = target_dir(app_builds, &compatibility);
    let environment = SdkEnvironment::new(sdk, target, compatibility, true);
    materialize_in_environment(project, environment, app_builds, cancelled)
}

/// Materialize `project` for exactly `environment`: its (canonical) SDK directory, its
/// compatibility manifest and its target directory. The returned build keeps that
/// environment, so a frozen one is the very environment the compile and every worker use.
pub fn materialize_in_environment(
    project: &OpenProject,
    environment_binding: SdkEnvironment,
    app_builds: &Path,
    cancelled: &impl Fn() -> bool,
) -> Result<MaterializedBuild, ProjectError> {
    let sdk = environment_binding.sdk_dir.clone();
    let compatibility = environment_binding.manifest.clone();
    let error = |path: &Path, reason: String| {
        ProjectError::new(
            path,
            "managed build",
            reason,
            "Choose a compatible SDK or copy/relink dependencies inside the workspace",
        )
    };
    compatibility
        .validate()
        .map_err(|e| error(&sdk, e.to_string()))?;
    let Some(framework) = framework_for(&project.manifest, &compatibility) else {
        return Err(error(&sdk, "SDK pin mismatch".into()));
    };
    if !project.worker_available {
        return Err(error(
            &project.root,
            "Worker bridge missing; add an explicit worker entry before building".into(),
        ));
    }
    for ancestor in project.root.ancestors().skip(1) {
        if ancestor.join(".cargo/config.toml").exists() || ancestor.join(".cargo/config").exists() {
            return Err(error(
                ancestor,
                "Inherited Cargo configuration is not portable".into(),
            ));
        }
    }
    for file in &project.inventory.files {
        if matches!(file.path.as_str(), ".cargo/config.toml" | ".cargo/config") {
            return Err(error(
                &project.root.join(file.path.as_str()),
                "Imported Cargo configuration conflicts with SDK binding; original retained".into(),
            ));
        }
    }
    // Keep the transient tree shallow: Windows can reject a child process whose working
    // directory inherits the former project/revision/compatibility/triple nesting.
    fs::create_dir_all(app_builds).map_err(|e| error(app_builds, e.to_string()))?;
    let staging = tempfile::Builder::new()
        .prefix("build-")
        .tempdir_in(app_builds)
        .map_err(|e| error(app_builds, e.to_string()))?;
    let root = staging.path().join("project");
    fs::create_dir(&root).map_err(|e| error(&root, e.to_string()))?;
    let mut buffer = [0; 64 * 1024];
    // The timing document stays out: the build is shared by every timing of the same
    // code, which the preview and export hand to the worker at run time.
    let build_revision = project.inventory.build_revision();
    for source in &project.inventory.files {
        if source.path.as_str() == TIMING_FILE {
            continue;
        }
        let target = root.join(source.path.as_str());
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| error(&target, e.to_string()))?;
        let mut input = source.path.open_file(&project.root)?;
        let mut output = fs::File::create(&target).map_err(|e| error(&target, e.to_string()))?;
        loop {
            if cancelled() {
                return Err(error(&root, "Materialization cancelled".into()));
            }
            let n = input
                .read(&mut buffer)
                .map_err(|e| error(&target, e.to_string()))?;
            if n == 0 {
                break;
            }
            output
                .write_all(&buffer[..n])
                .map_err(|e| error(&target, e.to_string()))?;
        }
        #[cfg(unix)]
        if source.executable {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
                .map_err(|e| error(&target, e.to_string()))?;
        }
    }
    if SourceInventory::scan_with_cancel(&root, cancelled)?.build_revision() != build_revision
        || SourceInventory::scan_with_cancel(&project.root, cancelled)?.build_revision()
            != build_revision
    {
        return Err(error(
            &project.root,
            "Source changed during materialization; refresh and retry".into(),
        ));
    }
    for file in &project.inventory.files {
        if cancelled() {
            return Err(error(&root, "Materialization cancelled".into()));
        }
        if file.path.as_str().rsplit('/').next() != Some("Cargo.toml") {
            continue;
        }
        // Read exactly the captured Cargo bytes, not a later edit in the live source.
        let mut cargo = read_cargo(&root, &file.path)?;
        bind(
            &mut cargo,
            &project.root,
            &root,
            Path::new(file.path.as_str()).parent().unwrap(),
            &sdk.join(framework.rel_path),
            framework,
        )?;
        let cargo_path = root.join(file.path.as_str());
        let cargo_text =
            toml::to_string_pretty(&cargo).map_err(|e| error(&cargo_path, e.to_string()))?;
        // This tree is private staging, not the user's portable project. Replace the
        // copied manifest directly: atomic rename-over-existing is not supported on Windows.
        fs::write(&cargo_path, cargo_text.as_bytes())
            .map_err(|e| error(&cargo_path, e.to_string()))?;
    }
    fs::create_dir_all(root.join(".cargo")).map_err(|e| error(&root, e.to_string()))?;
    let vendor = sdk.join("framework/vendor");
    if !vendor.is_dir() {
        return Err(error(&vendor, "SDK vendor directory missing".into()));
    }
    let cargo_config_path = root.join(".cargo/config.toml");
    let cargo_config = format!(
        "[source.crates-io]\nreplace-with = \"studio-vendor\"\n[source.studio-vendor]\ndirectory = {}\n",
        toml::Value::String(vendor.to_string_lossy().into_owned())
    );
    // This file is also inside the private staging tree; a failed write discards that tree.
    fs::write(&cargo_config_path, cargo_config.as_bytes())
        .map_err(|e| error(&cargo_config_path, e.to_string()))?;
    // SDK path substitution has a distinct graph/lock, never overwrite the portable lock.
    if root.join("Cargo.lock").exists() {
        fs::remove_file(root.join("Cargo.lock")).map_err(|e| error(&root, e.to_string()))?;
    }
    if SourceInventory::scan_with_cancel(&project.root, cancelled)?.build_revision()
        != build_revision
    {
        return Err(error(
            &project.root,
            "Source changed during SDK binding; refresh and retry".into(),
        ));
    }
    let environment = environment_binding;
    let staging_path = staging.path();
    let isolated_bin_dir = staging_path.join("bin");
    fs::create_dir_all(&isolated_bin_dir).map_err(|e| error(&isolated_bin_dir, e.to_string()))?;
    let root = staging_path.join("project");
    Ok(MaterializedBuild {
        manifest: root.join(project.manifest.entry.manifest.as_str()),
        root,
        environment,
        package: project.manifest.entry.package.clone(),
        worker_target: project.manifest.entry.worker_target.clone(),
        isolated_bin_dir,
        _directory: staging,
    })
}

fn bind(
    value: &mut toml::Value,
    project_root: &Path,
    copied_root: &Path,
    package_dir: &Path,
    framework_dir: &Path,
    framework: FrameworkSet<'_>,
) -> Result<(), ProjectError> {
    if let Some(table) = value.as_table_mut() {
        for (key, value) in table {
            if matches!(key.as_str(), "patch" | "replace") {
                return Err(ProjectError::new(
                    project_root.join(package_dir),
                    key,
                    "Cargo overrides need explicit SDK compatibility",
                    "Keep source unchanged and resolve the override before managed build",
                ));
            }
            if matches!(
                key.as_str(),
                "dependencies" | "dev-dependencies" | "build-dependencies"
            ) {
                if let Some(dependencies) = value.as_table_mut() {
                    for (name, dep) in dependencies {
                        let crate_name = dep
                            .get("package")
                            .and_then(toml::Value::as_str)
                            .unwrap_or(name)
                            .to_owned();
                        if let Some(path) = dep.get("path").and_then(toml::Value::as_str) {
                            let path_obj = Path::new(path);
                            let native = if path_obj.is_absolute() {
                                path_obj.to_path_buf()
                            } else {
                                project_root.join(package_dir).join(path)
                            };
                            let contained = fs::canonicalize(&native)
                                .ok()
                                .filter(|p| p.starts_with(project_root));
                            if contained.is_none() {
                                return Err(ProjectError::new(
                                    native,
                                    name,
                                    "external path dependency",
                                    "Copy this dependency into the project workspace and relink it explicitly",
                                ));
                            }
                            let relative = contained
                                .unwrap()
                                .strip_prefix(project_root)
                                .unwrap()
                                .to_string_lossy()
                                .replace('\\', "/");
                            if !relative.is_empty() {
                                ProjectPath::try_from(relative.clone())?
                                    .resolve_existing(project_root)?;
                            }
                            if path_obj.is_absolute() {
                                let rewritten = copied_root.join(&relative);
                                dep.as_table_mut().unwrap().insert(
                                    "path".into(),
                                    toml::Value::String(rewritten.to_string_lossy().into_owned()),
                                );
                            }
                        }
                        if matches!(
                            crate_name.as_str(),
                            "fframes" | "fframes-studio-runtime" | "fframes-studio-protocol"
                        ) && dep.get("workspace").and_then(toml::Value::as_bool) != Some(true)
                        {
                            let expected = if crate_name == "fframes" {
                                framework.fframes_version
                            } else {
                                framework.runtime_version
                            };
                            let version = dep
                                .as_str()
                                .or_else(|| dep.get("version").and_then(toml::Value::as_str));
                            // Every framework crate names its exact release, so a project
                            // never builds against a different one than it was written for.
                            if version.is_none_or(|v| v.trim_start_matches('=') != expected) {
                                return Err(ProjectError::new(
                                    project_root.join(package_dir),
                                    name,
                                    format!(
                                        "dependency version does not match SDK framework {expected}"
                                    ),
                                    "Select an SDK matching this version",
                                ));
                            }
                            if dep.is_str() {
                                *dep = toml::Value::Table(toml::map::Map::new());
                            }
                            let crate_dir = framework_dir.join(&crate_name);
                            if !crate_dir.join("Cargo.toml").is_file() {
                                return Err(ProjectError::new(
                                    crate_dir,
                                    name,
                                    "SDK crate missing",
                                    "Install the complete compatible SDK",
                                ));
                            }
                            dep.as_table_mut().unwrap().insert(
                                "path".into(),
                                toml::Value::String(crate_dir.to_string_lossy().into_owned()),
                            );
                        }
                    }
                }
            } else {
                bind(
                    value,
                    project_root,
                    copied_root,
                    package_dir,
                    framework_dir,
                    framework,
                )?;
            }
        }
    }
    Ok(())
}
