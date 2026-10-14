//! The project backend the shell runs commands against: app storage, the open project's
//! controller, SDK compatibility and the preset catalog. Commands run off the UI thread
//! and leave a fresh [`Presentation`] for it.
use crate::presets::{Catalog, PresetCommand, PresetReport, PresetView};
use crate::project_presentation::ProjectPresentation;
use crate::services::{build_service::BuildService, preview_coordinator::BuildSpec};
use crate::{
    Controller, app_paths::AppPaths, build_materialization::sdk_pin, containment::OwnershipPolicy,
    store::Store,
};
use parking_lot::Mutex;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use studio_project::{ProjectId, manifest::CargoEntry};
use studio_sdk::{CompatibilityManifest, Doctor};

#[derive(Clone)]
pub struct Recent {
    pub id: ProjectId,
    pub name: String,
    pub path: PathBuf,
    pub missing: bool,
}
#[derive(Clone, Default)]
pub struct Presentation {
    pub project: Option<ProjectPresentation>,
    pub recents: Vec<Recent>,
    pub sdk: String,
    pub state: Option<crate::ProjectState>,
    pub owner: Option<studio_bootstrap::ProcessTreeManager>,
    /// The open project's controller, shared with the agent workflow.
    pub controller: Option<Arc<Mutex<Controller>>>,
    pub paths: Option<AppPaths>,
    /// The installed SDK agent tasks compile and preview against (set only while it
    /// verifies and supports the preview contract). Separate from agent readiness.
    pub agent_sdk: Option<AgentSdk>,
    /// Style-preset controls state of the open project (`None` when none is open).
    pub preset: Option<PresetView>,
}

#[derive(Clone)]
pub struct AgentSdk {
    pub dir: PathBuf,
    pub compatibility: CompatibilityManifest,
}

pub enum Command {
    Initialize,
    Create(PathBuf),
    Open(PathBuf),
    Locate(PathBuf, ProjectId),
    Import(PathBuf),
    Asset(PathBuf),
    Refresh,
    Checkpoint,
    Build,
    CancelBuild,
    Close,
    RemoveRecent(ProjectId),
    SelectSdk(PathBuf),
    InstallSdk,
    ImportSdkBundle(PathBuf),
    Restore(PathBuf, studio_project::SourceRevision),
    Independent(PathBuf),
    /// A style-preset operation, fenced by the source revision the UI last showed.
    Preset(PresetCommand, Option<studio_project::SourceRevision>),
}
pub struct Backend {
    pub paths: Option<AppPaths>,
    /// Shared with the agent workflow, which locks it only for short engine calls and for
    /// capture/publication on its own job threads.
    pub controller: Option<Arc<Mutex<Controller>>>,
    pub compatibility: Option<CompatibilityManifest>,
    /// Why no manifest resolved for this host; shown while `compatibility` is `None`.
    pub compatibility_error: String,
    pub sdk_home: Option<PathBuf>,
    pub sdk: String,
    pub agent_sdk: Option<AgentSdk>,
    pub closed: Arc<AtomicBool>,
    pub processes: studio_bootstrap::ProcessTreeManager,
    pub build_spec: Option<BuildSpec>,
    /// Bundled presets plus the verified packages imported into the app's store.
    pub catalog: Catalog,
    /// The outcome of the last preset command, taken by the dispatch that ran it.
    pub preset_report: Option<PresetReport>,
    pub environment: StudioEnvironment,
    /// The running app's version; an SDK must be compatible with it.
    pub app_version: &'static str,
}
impl Backend {
    pub fn command(&mut self, command: Command) -> Result<(), String> {
        if matches!(command, Command::Initialize) {
            self.paths = Some(
                match &self.environment.app_data {
                    Some(data) => AppPaths::new(data),
                    None => AppPaths::system(),
                }
                .map_err(|e| e.to_string())?,
            );
            self.catalog = Catalog::load(self.paths.as_ref().expect("just set"));
            self.refresh_compatibility();
            return Ok(());
        }
        let paths = self
            .paths
            .as_ref()
            .ok_or("App storage unavailable; restart after fixing the reported directory")?
            .clone();
        match command {
            Command::Initialize => (),
            Command::Create(path) => {
                let compatibility = self
                    .compatibility
                    .as_ref()
                    .ok_or_else(|| self.compatibility_error.clone())?;
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("New video");
                studio_project::create(
                    &path,
                    name,
                    sdk_pin(compatibility),
                    &compatibility.fframes_version,
                    "0.1.0",
                )
                .map_err(|e| e.to_string())?;
                self.open(&path, &paths)?;
            }
            Command::Open(path) => self.open(&path, &paths)?,
            Command::Locate(path, id) => {
                if studio_project::open(&path)
                    .map_err(|e| e.to_string())?
                    .manifest
                    .project_id
                    != id
                {
                    return Err("Selected folder belongs to a different project; locate this recent project's folder or use Open for another project".into());
                }
                self.open(&path, &paths)?;
            }
            Command::Import(path) => {
                // The selected package is explicit. Walk only to its containing workspace root.
                if path.file_name().and_then(|n| n.to_str()) != Some("Cargo.toml") {
                    return Err("Choose the selected package's Cargo.toml".into());
                }
                let package_root = path.parent().ok_or("Choose a package Cargo.toml")?;
                let cargo = studio_project::lifecycle::read_cargo(
                    package_root,
                    &"Cargo.toml"
                        .to_owned()
                        .try_into()
                        .map_err(|e: studio_project::ProjectError| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                let package = cargo
                    .get("package")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .ok_or(
                        "Choose a package Cargo.toml inside the workspace, not its virtual root",
                    )?
                    .to_owned();
                let mut root = package_root.to_path_buf();
                for ancestor in package_root.ancestors() {
                    if ancestor.join("Cargo.toml").is_file() {
                        let candidate = studio_project::lifecycle::read_cargo(
                            ancestor,
                            &"Cargo.toml".to_owned().try_into().unwrap(),
                        )
                        .map_err(|e| e.to_string())?;
                        if candidate.get("workspace").is_some() {
                            root = ancestor.to_path_buf();
                            break;
                        }
                    }
                }
                let manifest = path
                    .strip_prefix(&root)
                    .map_err(|e| e.to_string())?
                    .to_str()
                    .ok_or("Cargo path must be UTF-8")?
                    .replace('\\', "/")
                    .try_into()
                    .map_err(|e: studio_project::ProjectError| e.to_string())?;
                studio_project::import(
                    &root,
                    CargoEntry {
                        manifest,
                        package,
                        worker_target: "studio_worker".into(),
                    },
                    sdk_pin(
                        self.compatibility
                            .as_ref()
                            .ok_or("SDK manifest unavailable")?,
                    ),
                )
                .map_err(|e| e.to_string())?;
                self.open(&root, &paths)?;
            }
            Command::Asset(path) => {
                let closed = self.closed.clone();
                self.current()?
                    .lock()
                    .copy_asset_with_cancel(&path, || closed.load(Ordering::Acquire))
                    .map_err(|e| e.to_string())?;
            }
            Command::Refresh => {
                if let Some(controller) = &self.controller {
                    controller.lock().reconcile().map_err(|e| e.to_string())?;
                }
                self.check_sdk();
            }
            Command::Checkpoint => self
                .current()?
                .lock()
                .checkpoint()
                .map_err(|e| e.to_string())?,
            Command::Build => {
                let builds = paths.builds();
                let fallback = self.sdk_home.as_ref().map(|h| h.join("active"));
                let controller = self.current()?;
                let mut controller = controller.lock();
                let sdk = controller
                    .sdk_path()
                    .map(PathBuf::from)
                    .or(fallback)
                    .ok_or("Select an installed SDK before building preview")?;
                let compatibility = CompatibilityManifest::from_json_str(
                    &std::fs::read_to_string(sdk.join("compatibility.json"))
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                if !compatibility
                    .preview_contract_versions
                    .contains(&fframes_studio_protocol::PREVIEW_CONTRACT_VERSION)
                {
                    return Err("Selected SDK supports only the legacy worker. Select an M2 preview SDK; imported source is not rewritten.".into());
                }
                if controller.state().active_tag().is_some() {
                    controller.cancel().map_err(|e| e.to_string())?;
                }
                let tag = controller
                    .begin_job(crate::JobKind::Build)
                    .map_err(|e| e.to_string())?;
                self.build_spec = Some(BuildSpec {
                    project: controller.project.clone(),
                    sdk,
                    compatibility,
                    builds,
                    tag,
                    compiler: controller.operation_processes(),
                    worker: controller.processes.sub_manager(),
                    service: self.environment.builds.clone(),
                });
            }
            Command::CancelBuild => {
                let controller = self.current()?;
                let mut controller = controller.lock();
                if controller.state().active_tag().is_some() {
                    controller.cancel().map_err(|e| e.to_string())?;
                }
            }
            Command::Close => {
                if let Some(controller) = &self.controller {
                    let mut controller = controller.lock();
                    // Compiles for this project stop; leases held by live workers survive.
                    self.environment.builds.close_project(&String::from(
                        controller.project.manifest.project_id.clone(),
                    ));
                    controller.close().map_err(|e| e.to_string())?;
                }
                self.controller = None;
                self.check_sdk();
            }
            Command::RemoveRecent(id) => Store::open(&paths.database())
                .and_then(|mut s| s.remove_recent(&id))
                .map_err(|e| e.to_string())?,
            Command::SelectSdk(path) => {
                self.current()?
                    .lock()
                    .select_sdk(path)
                    .map_err(|e| e.to_string())?;
                self.check_sdk();
            }
            Command::InstallSdk => {
                if let Err(error) = crate::sdk_setup::install_sdk(
                    self.compatibility
                        .as_ref()
                        .ok_or_else(|| self.compatibility_error.clone())?,
                    self.sdk_home.as_deref().ok_or("SDK home unavailable")?,
                    &self.processes,
                    self.app_version,
                ) {
                    self.sdk = "Failed · repair the reported installation error and retry".into();
                    return Err(error);
                }
                self.refresh_compatibility();
            }
            Command::ImportSdkBundle(bundle) => {
                if let Err(error) = crate::sdk_setup::install_sdk_from_bundle(
                    &bundle,
                    self.sdk_home.as_deref().ok_or("SDK home unavailable")?,
                    &self.processes,
                    self.app_version,
                ) {
                    self.sdk = "Failed · repair the reported installation error and retry".into();
                    return Err(error);
                }
                // An imported SDK may be the first manifest this host has.
                self.refresh_compatibility();
            }
            Command::Restore(path, revision) => self
                .current()?
                .lock()
                .export_checkpoint(&revision, &path)
                .map_err(|e| e.to_string())?,
            Command::Independent(path) => {
                studio_project::lifecycle::assign_independent_identity(&path)
                    .map_err(|e| e.to_string())?;
                self.open(&path, &paths)?;
            }
            Command::Preset(command, expected) => {
                let controller = self.current()?;
                let report = crate::presets::execute(
                    command,
                    &controller,
                    &mut self.catalog,
                    &paths,
                    expected.as_ref(),
                );
                let refusal = report.refused().then(|| report.status_lines().join(" "));
                self.preset_report = Some(report);
                if let Some(refusal) = refusal {
                    return Err(refusal);
                }
            }
        }
        Ok(())
    }
    pub fn current(&self) -> Result<Arc<Mutex<Controller>>, String> {
        self.controller
            .clone()
            .ok_or_else(|| "Open a project first".into())
    }
    pub fn open(&mut self, path: &Path, paths: &AppPaths) -> Result<(), String> {
        if self
            .controller
            .as_ref()
            .is_some_and(|c| c.lock().project.root == path)
        {
            return self
                .current()?
                .lock()
                .reconcile()
                .map_err(|e| e.to_string());
        }
        let candidate = Controller::open(path, paths).map_err(|e| e.to_string())?;
        if let Some(previous) = &self.controller {
            let mut previous = previous.lock();
            self.environment
                .builds
                .close_project(&String::from(previous.project.manifest.project_id.clone()));
            previous.close().map_err(|e| e.to_string())?;
        }
        self.controller = Some(Arc::new(Mutex::new(candidate)));
        self.check_sdk();
        Ok(())
    }
    /// Resolves the SDK manifest for this host; without one the reason becomes the SDK status.
    pub fn refresh_compatibility(&mut self) {
        match crate::sdk_setup::defaults_in(self.environment.sdk_home()) {
            Ok((compatibility, sdk_home)) => {
                self.compatibility = Some(compatibility);
                self.sdk_home = Some(sdk_home);
            }
            Err(error) => {
                self.compatibility = None;
                self.sdk_home = Some(self.environment.sdk_home());
                self.compatibility_error = error;
            }
        }
        self.check_sdk();
    }
    pub fn check_sdk(&mut self) {
        self.agent_sdk = None;
        let sdk = self
            .controller
            .as_ref()
            .and_then(|c| c.lock().sdk_path().map(PathBuf::from))
            .or_else(|| self.sdk_home.as_ref().map(|h| h.join("active")))
            .filter(|path| path.exists());
        // Host prerequisites come from the SDK that is actually selected, which may be the
        // only manifest this host has; the app default stands in while none is installed.
        let selected = sdk.as_ref().map(|path| {
            std::fs::read_to_string(path.join("compatibility.json"))
                .map_err(|e| e.to_string())
                .and_then(|s| CompatibilityManifest::from_json_str(&s).map_err(|e| e.to_string()))
        });
        let preflight = match &selected {
            Some(Ok(manifest)) => manifest,
            Some(Err(_)) | None => match &self.compatibility {
                Some(compatibility) => compatibility,
                None => {
                    self.sdk = self.compatibility_error.clone();
                    return;
                }
            },
        };
        let report = Doctor::run_host_preflight(preflight);
        if !report.is_ready() {
            self.sdk = format!("Host prerequisites needed: {}", report.format_summary());
            return;
        }
        self.sdk = match (sdk, selected) {
            (Some(path), Some(selected)) => {
                let selected = match selected {
                    Ok(manifest) => manifest,
                    Err(error) => {
                        self.sdk = format!("SDK failed: {error}; select a complete installed SDK");
                        return;
                    }
                };
                if self
                    .controller
                    .as_ref()
                    .is_some_and(|c| c.lock().project.manifest.sdk != sdk_pin(&selected))
                {
                    self.sdk = "Incompatible SDK pin; select the project's compatible SDK".into();
                    return;
                }
                let report = Doctor::verify_candidate_sdk_with_processes(
                    &path,
                    &selected,
                    Some(&self.processes),
                );
                if report.is_ready() {
                    // Agent tasks compile and preview candidates, which needs the M2
                    // preview contract; a legacy-worker SDK stays usable for the rest.
                    if selected
                        .preview_contract_versions
                        .contains(&fframes_studio_protocol::PREVIEW_CONTRACT_VERSION)
                    {
                        self.agent_sdk = Some(AgentSdk {
                            dir: path.clone(),
                            compatibility: selected.clone(),
                        });
                    }
                    format!("Available · {}", path.display())
                } else {
                    format!(
                        "SDK failed verification: {}; select/reinstall compatible SDK",
                        report.format_summary()
                    )
                }
            }
            _ => "Missing · install the managed SDK or select an installed compatible SDK".into(),
        };
    }
    pub fn presentation(&self) -> Result<Presentation, String> {
        let recents = match &self.paths {
            Some(paths) => Store::open(&paths.database())
                .and_then(|s| s.recents())
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|r| Recent {
                    missing: !r.location.is_dir(),
                    id: r.state.project().clone(),
                    name: r.name,
                    path: r.location,
                })
                .collect(),
            None => vec![],
        };
        // One short look at the controller; the workflow's job threads may hold it too.
        let open = self.controller.as_ref().map(|c| {
            let mut controller = c.lock();
            (
                ProjectPresentation::from_controller(&controller),
                controller.state().clone(),
                controller.processes.clone(),
                PresetView::capture(&mut controller, &self.catalog),
            )
        });
        Ok(Presentation {
            project: open.as_ref().map(|(project, _, _, _)| project.clone()),
            recents,
            sdk: self.sdk.clone(),
            state: open.as_ref().map(|(_, state, _, _)| state.clone()),
            preset: open.as_ref().map(|(_, _, _, preset)| preset.clone()),
            owner: open.map(|(_, _, owner, _)| owner),
            controller: self.controller.clone(),
            paths: self.paths.clone(),
            agent_sdk: self.agent_sdk.clone(),
        })
    }
}

/// Where the shell keeps its data, what compiles its workers and whom it trusts with
/// writes. [`StudioEnvironment::system`] is the installed app; a host that runs the shell
/// elsewhere (a harness, a sandbox) passes its own.
#[derive(Clone)]
pub struct StudioEnvironment {
    /// App data folder; `None` takes the platform's.
    pub app_data: Option<PathBuf>,
    /// Folder of installed SDKs (`active` is the default selection); `None` takes
    /// `~/.fframes/sdk`.
    pub sdk_home: Option<PathBuf>,
    /// Compiles preview, agent and export workers; equal build keys compile once.
    pub builds: BuildService,
    /// Who may say an agent adapter's writers are contained.
    pub ownership: OwnershipPolicy,
}

impl StudioEnvironment {
    pub fn system() -> Self {
        Self {
            app_data: None,
            sdk_home: None,
            builds: crate::services::worker_project::shared_build_service(),
            ownership: OwnershipPolicy::Validated,
        }
    }

    pub fn sdk_home(&self) -> PathBuf {
        self.sdk_home
            .clone()
            .unwrap_or_else(crate::sdk_setup::default_sdk_home)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn restore_command_exports_selected_checkpoint_without_requiring_healthy_source() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("video");
        let paths = AppPaths::new(temp.path().join("history")).unwrap();
        let compatibility = CompatibilityManifest::default_linux_x64();
        studio_project::create(
            &root,
            "Video",
            sdk_pin(&compatibility),
            &compatibility.fframes_version,
            "0.1.0",
        )
        .unwrap();
        let mut controller = Controller::open(&root, &paths).unwrap();
        let selected = controller.state().accepted().clone();
        let saved = fs::read(root.join("src/lib.rs")).unwrap();
        fs::write(root.join("src/lib.rs"), b"// newer checkpoint").unwrap();
        controller.checkpoint().unwrap();
        assert_ne!(controller.state().accepted(), &selected);
        fs::write(root.join("studio.json"), b"invalid source manifest").unwrap();
        let mut backend = Backend {
            paths: Some(paths),
            controller: Some(Arc::new(Mutex::new(controller))),
            compatibility: None,
            compatibility_error: "SDK manifest unavailable".into(),
            sdk_home: None,
            sdk: String::new(),
            agent_sdk: None,
            closed: Arc::new(AtomicBool::new(false)),
            processes: studio_bootstrap::ProcessTreeManager::new(),
            build_spec: None,
            catalog: Catalog::default(),
            preset_report: None,
            environment: StudioEnvironment::system(),
            app_version: "0.0.0",
        };
        let destination = temp.path().join("restored");
        backend
            .command(Command::Restore(destination.clone(), selected))
            .unwrap();
        assert_eq!(fs::read(destination.join("src/lib.rs")).unwrap(), saved);
        assert!(studio_project::open(&destination).is_ok());
        assert_eq!(
            fs::read(root.join("src/lib.rs")).unwrap(),
            b"// newer checkpoint"
        );
        assert_eq!(
            fs::read(root.join("studio.json")).unwrap(),
            b"invalid source manifest"
        );
    }
}
