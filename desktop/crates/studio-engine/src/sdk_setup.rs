//! Finding and installing the managed SDK: the manifest for this host, the default SDK
//! folder, and installation from the packaged artifacts or a user-selected bundle. Callers
//! run these off the UI thread.
use std::{
    fs,
    path::{Path, PathBuf},
};
use studio_bootstrap::ProcessTreeManager;
use studio_sdk::{install::SdkInstaller, manifest::CompatibilityManifest};

pub fn default_sdk_home() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".fframes").join("sdk")
    } else if let Ok(profile) = std::env::var("USERPROFILE") {
        PathBuf::from(profile).join(".fframes").join("sdk")
    } else {
        std::env::temp_dir().join("fframes_sdk")
    }
}

/// What to do when no SDK bundle targets this host.
pub fn missing_sdk_bundle_message(host: &str) -> String {
    let assemble = if cfg!(windows) {
        "desktop/scripts/build-phase-zero-sdk.ps1 -FfmpegRoot <ffmpeg> -Out desktop/target/phase-zero-sdk"
    } else {
        "desktop/scripts/build-phase-zero-sdk.sh --ffmpeg-root <ffmpeg> --out desktop/target/phase-zero-sdk"
    };
    format!(
        "No SDK bundle for {host} found. Import an SDK bundle, set FFRAMES_SDK_BUNDLE to an \
         assembled bundle, or assemble one with `{assemble}`."
    )
}

/// The SDK manifest for this host and the folder of SDKs installed under `sdk_home`.
pub fn defaults_in(sdk_home: PathBuf) -> Result<(CompatibilityManifest, PathBuf), String> {
    let packaged = std::env::var_os("FFRAMES_SDK_BUNDLE")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| Some(p.parent()?.parent()?.join("sdk")))
        });
    // Bundles assembled in a repository checkout, relative to the working folder. Only a
    // development build looks there: a release build never takes its trust root (the
    // manifest that pins every artifact hash) from wherever it was started.
    let dev_bundles: Vec<PathBuf> = if cfg!(debug_assertions) {
        vec![
            PathBuf::from("target/phase-zero-sdk"),
            PathBuf::from("desktop/target/phase-zero-sdk"),
        ]
    } else {
        Vec::new()
    };
    let manifest = resolve_manifest(packaged.as_deref(), &dev_bundles, &sdk_home)?;
    Ok((manifest, sdk_home))
}

/// Picks the SDK manifest for this host: a packaged bundle, the built-in manifest when it
/// targets this host, an assembled development bundle for this host, then the installed
/// SDK's receipt. A host with none of these gets instructions instead of a foreign target.
pub fn resolve_manifest(
    packaged: Option<&Path>,
    dev_bundles: &[PathBuf],
    sdk_home: &Path,
) -> Result<CompatibilityManifest, String> {
    let read = |bundle: &Path| {
        fs::read_to_string(bundle.join("compatibility.json"))
            .map_err(|e| e.to_string())
            .and_then(|json| CompatibilityManifest::from_json_str(&json).map_err(|e| e.to_string()))
    };
    if let Some(bundle) = packaged.filter(|p| p.join("compatibility.json").is_file()) {
        return read(bundle);
    }
    let host = CompatibilityManifest::host_target();
    let for_host = |manifest: &CompatibilityManifest| Some(manifest.target_triple.as_str()) == host;
    let built_in = CompatibilityManifest::default_linux_x64();
    if for_host(&built_in) {
        return Ok(built_in);
    }
    let active = sdk_home.join("active");
    dev_bundles
        .iter()
        .chain([&active])
        .filter_map(|bundle| read(bundle).ok())
        .find(for_host)
        .ok_or_else(|| missing_sdk_bundle_message(host.unwrap_or("this platform")))
}

/// The installer the app uses: a first install is checked in place by building and
/// rendering the starter project into the shared target under `builds`, which also warms
/// it for the first preview.
pub fn installer(sdk_home: &Path, processes: &ProcessTreeManager, builds: &Path) -> SdkInstaller {
    let (check_processes, builds) = (processes.clone(), builds.to_owned());
    SdkInstaller::new(sdk_home)
        .with_process_manager(processes.clone())
        .with_candidate_check(std::sync::Arc::new(move |sdk, manifest| {
            crate::services::worker_project::check_sdk_build(
                sdk,
                manifest,
                &builds,
                &check_processes,
            )
        }))
}

/// Install only the SDK. Product setup must not generate a project.
/// `app_version` is the running app's version the SDK must be compatible with; `builds`
/// is the app's build folder.
pub fn install_sdk(
    manifest: &CompatibilityManifest,
    sdk_home: &Path,
    processes: &ProcessTreeManager,
    app_version: &str,
    builds: &Path,
) -> Result<PathBuf, String> {
    manifest
        .validate_for_current_app_version(app_version)
        .map_err(|error| format!("SDK compatibility rejected before setup: {error}"))?;
    let installer = installer(sdk_home, processes, builds);
    installer
        .recover_interrupted_install()
        .map_err(|error| format!("Interrupted SDK install recovery failed: {error}"))?;
    if let Some(active_sdk) = installer
        .reusable_active_for_app(manifest, app_version, Some(processes))
        .map_err(|error| format!("Installed SDK receipt check failed: {error}"))?
    {
        return Ok(active_sdk);
    }

    // 1. Resolve artifacts strictly from manifest across candidate directories
    let mut candidate_artifact_dirs = vec![
        PathBuf::from("desktop/packaging/sdk/artifacts"),
        PathBuf::from("packaging/sdk/artifacts"),
        PathBuf::from("target/phase-zero-sdk/artifacts"),
        PathBuf::from("desktop/target/phase-zero-sdk/artifacts"),
    ];
    if let Some(bundle) = std::env::var_os("FFRAMES_SDK_BUNDLE") {
        candidate_artifact_dirs.insert(0, PathBuf::from(bundle).join("artifacts"));
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(package) = executable.parent().and_then(|p| p.parent())
    {
        candidate_artifact_dirs.insert(0, package.join("sdk/artifacts"));
    }

    let mut artifact_files = Vec::new();
    for artifact in &manifest.artifacts {
        let filename = artifact
            .url
            .strip_prefix("file://artifacts/")
            .or_else(|| artifact.url.strip_prefix("file://"))
            .unwrap_or(&artifact.name);
        let relative = PathBuf::from(filename);
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(format!(
                "SDK artifact '{}' has an unsafe local bundle path",
                artifact.name
            ));
        }
        let mut resolved = None;
        for dir in &candidate_artifact_dirs {
            let root = dir.canonicalize().ok();
            let candidate = dir.join(&relative).canonicalize().ok();
            if let (Some(root), Some(candidate)) = (root, candidate)
                && candidate.starts_with(root)
                && candidate.is_file()
            {
                resolved = Some(candidate);
                break;
            }
        }
        let file_path = resolved.ok_or_else(|| {
            format!(
                "required artifact '{}' ({}) not found in candidate artifact directories",
                artifact.name, filename
            )
        })?;
        artifact_files.push((artifact.clone(), file_path));
    }

    // 2. A non-receipted or incompatible active SDK is replaced only after the
    // local bundle has been resolved and the new candidate passes build/render probes.
    installer
        .install_from_local_artifacts_for_app(manifest, &artifact_files, app_version)
        .map_err(|e| format!("SDK installation failed: {e}"))?;

    Ok(installer.active_sdk_dir())
}

/// Explicit import path for a user-selected offline SDK bundle. No network lookup
/// occurs and a receipt-backed compatible active SDK can still be reused archive-free.
/// `app_version` is the running app's version the SDK must be compatible with.
pub fn install_sdk_from_bundle(
    bundle: &Path,
    sdk_home: &Path,
    processes: &ProcessTreeManager,
    app_version: &str,
    builds: &Path,
) -> Result<PathBuf, String> {
    let manifest_path = bundle.join("compatibility.json");
    let json = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("SDK bundle manifest {}: {error}", manifest_path.display()))?;
    let manifest = CompatibilityManifest::from_json_str(&json)
        .map_err(|error| format!("SDK bundle manifest is invalid: {error}"))?;
    manifest
        .validate_for_current_app_version(app_version)
        .map_err(|error| format!("SDK bundle is incompatible: {error}"))?;

    let installer = installer(sdk_home, processes, builds);
    installer
        .recover_interrupted_install()
        .map_err(|error| format!("Interrupted SDK install recovery failed: {error}"))?;
    if let Some(active_sdk) = installer
        .reusable_active_for_app(&manifest, app_version, Some(processes))
        .map_err(|error| format!("Installed SDK receipt check failed: {error}"))?
    {
        return Ok(active_sdk);
    }

    let bundle_root = bundle
        .canonicalize()
        .map_err(|error| format!("SDK bundle folder is unavailable: {error}"))?;
    let artifacts_dir = bundle_root
        .join("artifacts")
        .canonicalize()
        .map_err(|error| format!("SDK bundle artifacts folder is unavailable: {error}"))?;
    let mut artifacts = Vec::with_capacity(manifest.artifacts.len());
    for artifact in &manifest.artifacts {
        let filename = artifact
            .url
            .strip_prefix("file://artifacts/")
            .ok_or_else(|| {
                format!(
                    "SDK bundle artifact '{}' has a non-local URL",
                    artifact.name
                )
            })?;
        let relative = PathBuf::from(filename);
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(format!(
                "SDK bundle artifact '{}' has an unsafe path",
                artifact.name
            ));
        }
        let path = artifacts_dir.join(relative);
        let canonical = path.canonicalize().map_err(|error| {
            format!(
                "SDK bundle artifact '{}' is unavailable: {error}",
                artifact.name
            )
        })?;
        if !canonical.starts_with(&artifacts_dir) || !canonical.is_file() {
            return Err(format!(
                "SDK bundle artifact '{}' resolves outside its artifacts directory",
                artifact.name,
            ));
        }
        artifacts.push((artifact.clone(), canonical));
    }
    installer
        .install_from_local_artifacts_for_app(&manifest, &artifacts, app_version)
        .map_err(|error| format!("SDK bundle installation failed: {error}"))?;
    Ok(installer.active_sdk_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_bundle(dir: &Path, target: &str) -> CompatibilityManifest {
        let mut manifest = CompatibilityManifest::default_linux_x64();
        manifest.target_triple = target.into();
        manifest.sdk_id = format!("studio-sdk-{target}-test");
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("compatibility.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();
        manifest
    }

    #[test]
    fn manifest_resolution_never_falls_back_to_a_foreign_target() {
        let host = CompatibilityManifest::host_target().expect("supported host");
        let foreign = if host == "x86_64-unknown-linux-gnu" {
            "x86_64-pc-windows-msvc"
        } else {
            "x86_64-unknown-linux-gnu"
        };
        let root = tempfile::tempdir().unwrap();
        let sdk_home = root.path().join("sdk");
        let dev = root.path().join("dev");
        write_bundle(&dev, foreign);

        let resolved = resolve_manifest(None, std::slice::from_ref(&dev), &sdk_home);
        if host == "x86_64-unknown-linux-gnu" {
            assert_eq!(
                resolved.unwrap(),
                CompatibilityManifest::default_linux_x64()
            );
            return;
        }
        let error = resolved.expect_err("a foreign bundle is not this host's manifest");
        assert!(error.contains(host), "{error}");

        // The installed SDK's receipt is picked up, and an assembled bundle wins over it.
        let installed = write_bundle(&sdk_home.join("active"), host);
        let resolved = resolve_manifest(None, std::slice::from_ref(&dev), &sdk_home);
        assert_eq!(resolved.unwrap(), installed);
        let assembled = write_bundle(&dev, host);
        let resolved = resolve_manifest(None, &[dev], &sdk_home);
        assert_eq!(resolved.unwrap(), assembled);
    }
}
