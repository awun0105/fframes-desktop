use crate::design_system::{self, TextStyle};
use crate::ui::{
    Badge, Button, ButtonVariant, Icon, Notice, StatusDot, Tone, card, divider, heading, icon,
    muted,
};
use gpui::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use studio_bootstrap::ProcessTreeManager;
use studio_sdk::{
    Doctor, ProbeStatus, environment::SdkEnvironment, install::SdkInstaller,
    manifest::CompatibilityManifest, project::ProjectManager,
};

fn default_sdk_home() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".fframes").join("sdk")
    } else if let Ok(profile) = std::env::var("USERPROFILE") {
        PathBuf::from(profile).join(".fframes").join("sdk")
    } else {
        std::env::temp_dir().join("fframes_sdk")
    }
}

fn default_projects_root() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".fframes").join("projects")
    } else if let Ok(profile) = std::env::var("USERPROFILE") {
        PathBuf::from(profile).join(".fframes").join("projects")
    } else {
        std::env::temp_dir().join("fframes_projects")
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupState {
    Checking,
    PrerequisitesNeeded {
        missing_packages: Vec<String>,
        summary: String,
    },
    SdkInstallNeeded {
        manifest_digest: String,
    },
    Installing {
        progress_percent: u8,
        step: String,
    },
    SdkReady {
        active_path: PathBuf,
        manifest_digest: String,
    },
    CreatingProject {
        name: String,
    },
    Building {
        name: String,
    },
    Rendered {
        frame_path: PathBuf,
        build_time: Duration,
        render_time: Duration,
    },
    Failed {
        error: String,
    },
}

pub struct SetupView {
    pub state: SetupState,
    pub manifest: CompatibilityManifest,
    pub sdk_home: PathBuf,
    pub projects_root: PathBuf,
    pub process_tree: ProcessTreeManager,
    /// The step that was active when setup failed; only meaningful in `Failed`.
    failed_step: Option<usize>,
}

impl SetupView {
    /// Shared discovery for the spike and product shell; callers run this off the UI thread.
    pub fn defaults() -> Result<(CompatibilityManifest, PathBuf), String> {
        let packaged_manifest = std::env::var_os("FFRAMES_SDK_BUNDLE")
            .map(PathBuf::from)
            .map(|p| p.join("compatibility.json"))
            .or_else(|| {
                std::env::current_exe().ok().and_then(|p| {
                    p.parent()?
                        .parent()
                        .map(|p| p.join("sdk/compatibility.json"))
                })
            });
        let manifest_result = packaged_manifest.filter(|p| p.is_file()).map(|p| {
            std::fs::read_to_string(p)
                .map_err(|e| e.to_string())
                .and_then(|json| {
                    CompatibilityManifest::from_json_str(&json).map_err(|e| e.to_string())
                })
        });
        Ok((
            manifest_result
                .transpose()?
                .unwrap_or_else(CompatibilityManifest::default_linux_x64),
            default_sdk_home(),
        ))
    }

    pub fn new(cx: &mut Context<Self>) -> Self {
        let defaults = Self::defaults();
        let (manifest, sdk_home) = defaults.as_ref().cloned().unwrap_or_else(|_| {
            (
                CompatibilityManifest::default_linux_x64(),
                default_sdk_home(),
            )
        });
        let mut view = Self {
            state: SetupState::Checking,
            manifest,
            sdk_home,
            projects_root: default_projects_root(),
            process_tree: ProcessTreeManager::new(),
            failed_step: None,
        };
        if let Err(error) = defaults {
            view.fail(error);
        } else {
            view.run_preflight(cx);
        }
        view
    }

    /// Enters `Failed`, remembering which step was active so it can be marked.
    fn fail(&mut self, error: String) {
        self.failed_step = failing_step(&self.state);
        self.state = SetupState::Failed { error };
    }

    pub fn run_preflight(&mut self, cx: &mut Context<Self>) {
        self.state = SetupState::Checking;
        let report = Doctor::run_host_preflight(&self.manifest);

        if report.overall_status == ProbeStatus::Fail {
            if !report.missing_system_packages.is_empty() {
                self.state = SetupState::PrerequisitesNeeded {
                    missing_packages: report.missing_system_packages.clone(),
                    summary: report.format_summary(),
                };
            } else {
                self.fail(report.format_summary());
            }
            cx.notify();
            return;
        }

        let active_path = self.sdk_home.join("active");
        if active_path.exists() {
            let candidate_report = Doctor::verify_candidate_sdk_with_processes(
                &active_path,
                &self.manifest,
                Some(&self.process_tree),
            );
            if candidate_report.is_ready() {
                self.state = SetupState::SdkReady {
                    active_path,
                    manifest_digest: self.manifest.digest(),
                };
                cx.notify();
                return;
            }
        }

        self.state = SetupState::SdkInstallNeeded {
            manifest_digest: self.manifest.digest(),
        };
        cx.notify();
    }

    pub fn start_install_and_build(&mut self, cx: &mut Context<Self>) {
        self.state = SetupState::Installing {
            progress_percent: 20,
            step: "Checking / preparing SDK artifacts and dependencies...".to_string(),
        };
        cx.notify();

        let manifest = self.manifest.clone();
        let sdk_home = self.sdk_home.clone();
        let projects_root = self.projects_root.clone();
        let process_tree = self.process_tree.clone();

        cx.spawn(async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move {
                    Self::execute_install_and_build_pipeline(
                        manifest,
                        sdk_home,
                        projects_root,
                        process_tree,
                    )
                })
                .await;

            let _ = this.update(cx, |view, cx| {
                match res {
                    Ok((frame_path, build_time, render_time)) => {
                        view.state = SetupState::Rendered {
                            frame_path,
                            build_time,
                            render_time,
                        };
                    }
                    Err(err) => {
                        view.fail(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Install only the SDK. Product setup must not regenerate the spike fixture.
    pub fn install_sdk(
        manifest: &CompatibilityManifest,
        sdk_home: &std::path::Path,
        processes: &ProcessTreeManager,
    ) -> Result<PathBuf, String> {
        manifest
            .validate_for_current_app_version(env!("CARGO_PKG_VERSION"))
            .map_err(|error| format!("SDK compatibility rejected before setup: {error}"))?;
        let installer = SdkInstaller::new(sdk_home).with_process_manager(processes.clone());
        installer
            .recover_interrupted_install()
            .map_err(|error| format!("Interrupted SDK install recovery failed: {error}"))?;
        if let Some(active_sdk) = installer
            .reusable_active_for_app(manifest, env!("CARGO_PKG_VERSION"), Some(processes))
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
            .install_from_local_artifacts_for_app(
                manifest,
                &artifact_files,
                env!("CARGO_PKG_VERSION"),
            )
            .map_err(|e| format!("SDK installation failed: {e}"))?;

        Ok(installer.active_sdk_dir())
    }

    /// Explicit import path for a user-selected offline SDK bundle. No network lookup
    /// occurs and a receipt-backed compatible active SDK can still be reused archive-free.
    pub fn install_sdk_from_bundle(
        bundle: &std::path::Path,
        sdk_home: &std::path::Path,
        processes: &ProcessTreeManager,
    ) -> Result<PathBuf, String> {
        let manifest_path = bundle.join("compatibility.json");
        let json = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("SDK bundle manifest {}: {error}", manifest_path.display()))?;
        let manifest = CompatibilityManifest::from_json_str(&json)
            .map_err(|error| format!("SDK bundle manifest is invalid: {error}"))?;
        manifest
            .validate_for_current_app_version(env!("CARGO_PKG_VERSION"))
            .map_err(|error| format!("SDK bundle is incompatible: {error}"))?;

        let installer = SdkInstaller::new(sdk_home).with_process_manager(processes.clone());
        installer
            .recover_interrupted_install()
            .map_err(|error| format!("Interrupted SDK install recovery failed: {error}"))?;
        if let Some(active_sdk) = installer
            .reusable_active_for_app(&manifest, env!("CARGO_PKG_VERSION"), Some(processes))
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
            .install_from_local_artifacts_for_app(&manifest, &artifacts, env!("CARGO_PKG_VERSION"))
            .map_err(|error| format!("SDK bundle installation failed: {error}"))?;
        Ok(installer.active_sdk_dir())
    }

    fn execute_install_and_build_pipeline(
        manifest: CompatibilityManifest,
        sdk_home: PathBuf,
        projects_root: PathBuf,
        process_tree: ProcessTreeManager,
    ) -> Result<(PathBuf, Duration, Duration), String> {
        let active_sdk = Self::install_sdk(&manifest, &sdk_home, &process_tree)?;
        // 3. Generate template project using the managed SDK's standalone framework
        fs::create_dir_all(&projects_root).map_err(|e| e.to_string())?;
        let project_dir = projects_root.join("phase-zero-template-video");
        if project_dir.exists() {
            let _ = fs::remove_dir_all(&project_dir);
        }

        ProjectManager::generate_cpu_project_with_sdk(
            "phase-zero-template-video",
            &project_dir,
            &manifest.fframes_version,
            Some(&active_sdk),
        )
        .map_err(|e| format!("Project generation failed: {e}"))?;

        // 4. Build project with isolated SDK environment in offline mode
        let sdk_env = SdkEnvironment::new(&active_sdk, &project_dir, manifest.clone(), true);
        let build_time = ProjectManager::build_project(&project_dir, &sdk_env, &process_tree)
            .map_err(|e| format!("Project build failed: {e}"))?;

        // 5. Render frame 0
        let frames_dir = project_dir.join("frames");
        let (render_time, _bytes) =
            ProjectManager::render_frame(&project_dir, &sdk_env, 0, &frames_dir, &process_tree)
                .map_err(|e| format!("Frame render failed: {e}"))?;

        let expected_frame = frames_dir.join("0.png");
        let final_frame = if expected_frame.exists() {
            expected_frame
        } else {
            frames_dir
        };

        Ok((final_frame, build_time, render_time))
    }

    /// Same ids and labels as before; the handler runs on this view.
    fn action_button(
        id: &'static str,
        label: &'static str,
        variant: ButtonVariant,
        entity: Entity<Self>,
        action: fn(&mut Self, &mut Context<Self>),
    ) -> Button {
        Button::new(id, label)
            .variant(variant)
            .on_click(move |_window, cx| {
                entity.update(cx, action);
            })
    }
}

/// Progress of one of the four setup steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Active,
    Done,
    Failed,
}

pub const STEP_NAMES: [&str; 4] = ["Preflight", "Install SDK", "Build template", "Render"];

/// Step statuses for a setup state: Preflight, Install SDK, Build template, Render.
///
/// `Failed` carries no record of the stage that failed, so every step stays
/// pending; the view uses [`step_statuses_after`] with the step it remembered.
pub fn step_statuses(state: &SetupState) -> [StepStatus; 4] {
    use StepStatus::{Active, Done, Failed, Pending};
    match state {
        SetupState::Checking => [Active, Pending, Pending, Pending],
        SetupState::PrerequisitesNeeded { .. } => [Failed, Pending, Pending, Pending],
        SetupState::SdkInstallNeeded { .. } => [Done, Pending, Pending, Pending],
        SetupState::Installing { .. } => [Done, Active, Pending, Pending],
        SetupState::SdkReady { .. } => [Done, Done, Pending, Pending],
        SetupState::CreatingProject { .. } | SetupState::Building { .. } => {
            [Done, Done, Active, Pending]
        }
        SetupState::Rendered { .. } => [Done, Done, Done, Done],
        SetupState::Failed { .. } => [Pending, Pending, Pending, Pending],
    }
}

/// The step a failure belongs to when it happens in `previous`: the active step,
/// else the first one not yet done.
fn failing_step(previous: &SetupState) -> Option<usize> {
    let statuses = step_statuses(previous);
    statuses
        .iter()
        .position(|s| *s == StepStatus::Active)
        .or_else(|| statuses.iter().position(|s| *s == StepStatus::Pending))
}

/// [`step_statuses`], with the remembered step marked failed in the `Failed` state:
/// the steps before it are done, the ones after it pending.
pub fn step_statuses_after(state: &SetupState, failed_step: Option<usize>) -> [StepStatus; 4] {
    match (state, failed_step) {
        (SetupState::Failed { .. }, Some(failed)) => {
            std::array::from_fn(|i| match i.cmp(&failed) {
                std::cmp::Ordering::Less => StepStatus::Done,
                std::cmp::Ordering::Equal => StepStatus::Failed,
                std::cmp::Ordering::Greater => StepStatus::Pending,
            })
        }
        _ => step_statuses(state),
    }
}

fn step_row(name: &'static str, status: StepStatus) -> gpui::Div {
    let palette = design_system::tokens().palette;
    let metrics = design_system::tokens().metrics;
    let marker = match status {
        StepStatus::Pending => StatusDot::new(Tone::Neutral).into_any_element(),
        StepStatus::Active => StatusDot::new(Tone::Info).into_any_element(),
        StepStatus::Done => {
            icon(Icon::CircleCheck, metrics.icon_md, palette.success).into_any_element()
        }
        StepStatus::Failed => {
            icon(Icon::AlertTriangle, metrics.icon_md, palette.danger).into_any_element()
        }
    };
    let name_color = match status {
        StepStatus::Pending => palette.text_3,
        _ => palette.text_1,
    };
    div()
        .flex()
        .items_center()
        .gap(px(metrics.space_2))
        .child(
            div()
                .w(px(metrics.icon_md))
                .flex()
                .justify_center()
                .child(marker),
        )
        .child(
            design_system::text_style(div(), TextStyle::Label)
                .text_color(name_color)
                .child(name),
        )
        .children(
            (status == StepStatus::Active).then(|| Badge::new("In progress").tone(Tone::Info)),
        )
}

fn detail_line(text: impl Into<gpui::SharedString>) -> gpui::Div {
    muted(text)
}

impl SetupView {
    /// Status text and extra content for the current state.
    fn state_detail(&self, cx: &mut Context<Self>) -> gpui::Div {
        let palette = design_system::tokens().palette;
        let metrics = design_system::tokens().metrics;
        let entity = cx.entity();
        let column = || div().flex().flex_col().gap(px(metrics.space_2));
        match &self.state {
            SetupState::Checking => column()
                .child(detail_line("Checking Host Prerequisites..."))
                .child(detail_line(
                    "Running read-only preflight probes on compiler, CMake, and graphics libraries...",
                )),
            SetupState::PrerequisitesNeeded {
                missing_packages, ..
            } => column()
                .child(detail_line("System Prerequisites Needed"))
                .child(detail_line(
                    "The following host packages were detected as missing and require user review:",
                ))
                .child(
                    div()
                        .p(px(metrics.space_2))
                        .bg(palette.stage)
                        .rounded(px(metrics.radius_control))
                        .border_1()
                        .border_color(palette.line_strong)
                        .child(
                            design_system::text_style(div(), TextStyle::Small)
                                .text_color(palette.text_1)
                                .child(studio_sdk::install_hint(missing_packages)),
                        ),
                )
                .child(div().flex().child(Self::action_button(
                    "btn-recheck",
                    "Re-run Preflight Check",
                    ButtonVariant::Editor,
                    entity,
                    |this, cx| this.run_preflight(cx),
                ))),
            SetupState::SdkInstallNeeded { manifest_digest } => column()
                .child(detail_line(
                    "Host Preflight Passed — SDK Installation Required",
                ))
                .child(detail_line(
                    "Host environment verified. Ready to install app-managed toolchain & FFmpeg dependencies.",
                ))
                .child(detail_line(format!("Manifest: {:.16}...", manifest_digest)))
                .child(div().flex().child(Self::action_button(
                    "btn-install-sdk",
                    "Install Managed SDK & Build Template",
                    ButtonVariant::Primary,
                    entity,
                    |this, cx| this.start_install_and_build(cx),
                ))),
            SetupState::Installing {
                progress_percent,
                step,
            } => column()
                .child(detail_line(format!(
                    "Installing SDK... ({progress_percent}%)"
                )))
                .child(detail_line(step.clone())),
            SetupState::SdkReady {
                active_path,
                manifest_digest,
            } => column()
                .child(detail_line("Managed SDK Ready"))
                .child(detail_line(format!("Active location: {:?}", active_path)))
                .child(detail_line(format!(
                    "Manifest Digest: {:.16}...",
                    manifest_digest
                )))
                .child(div().flex().child(Self::action_button(
                    "btn-create-proj",
                    "Build and Render Template Project",
                    ButtonVariant::Primary,
                    entity,
                    |this, cx| this.start_install_and_build(cx),
                ))),
            SetupState::CreatingProject { name } => column().child(detail_line(format!(
                "Generating project '{name}' outside workspace..."
            ))),
            SetupState::Building { name } => column().child(detail_line(format!(
                "Building '{name}' with isolated SDK toolchain..."
            ))),
            SetupState::Rendered {
                frame_path,
                build_time,
                render_time,
            } => column()
                .child(detail_line("Project Rendered Successfully!"))
                .child(detail_line(format!("Build duration: {:?}", build_time)))
                .child(detail_line(format!(
                    "Frame render duration: {:?}",
                    render_time
                )))
                .child(detail_line(format!("Output PNG: {:?}", frame_path))),
            SetupState::Failed { error } => column().child(
                Notice::new("setup-failed", Tone::Danger, "Setup / Build Failed")
                    .details(error.clone())
                    .action(Self::action_button(
                        "btn-retry",
                        "Retry",
                        ButtonVariant::Editor,
                        entity,
                        |this, cx| this.run_preflight(cx),
                    )),
            ),
        }
    }
}

const CARD_MAX_W: f32 = 560.;
const CARD_PADDING: f32 = 24.;

impl Render for SetupView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = design_system::tokens().palette;
        let metrics = design_system::tokens().metrics;
        let steps = div().flex().flex_col().gap(px(metrics.space_2)).children(
            STEP_NAMES
                .iter()
                .zip(step_statuses_after(&self.state, self.failed_step))
                .map(|(name, status)| step_row(name, status)),
        );

        div()
            .flex()
            .w_full()
            .justify_center()
            .p(px(metrics.space_6))
            .bg(palette.window)
            .child(
                card()
                    .flex()
                    .flex_col()
                    .w_full()
                    .max_w(px(CARD_MAX_W))
                    .p(px(CARD_PADDING))
                    .gap(px(metrics.space_4))
                    .child(heading("Set up fframes Studio"))
                    .child(muted(
                        "Get the managed SDK ready and render a template project.",
                    ))
                    .child(steps)
                    .child(divider())
                    .child(self.state_detail(cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_state() -> Vec<SetupState> {
        vec![
            SetupState::Checking,
            SetupState::PrerequisitesNeeded {
                missing_packages: vec![],
                summary: String::new(),
            },
            SetupState::SdkInstallNeeded {
                manifest_digest: String::new(),
            },
            SetupState::Installing {
                progress_percent: 20,
                step: String::new(),
            },
            SetupState::SdkReady {
                active_path: PathBuf::new(),
                manifest_digest: String::new(),
            },
            SetupState::CreatingProject {
                name: String::new(),
            },
            SetupState::Building {
                name: String::new(),
            },
            SetupState::Rendered {
                frame_path: PathBuf::new(),
                build_time: Duration::ZERO,
                render_time: Duration::ZERO,
            },
            SetupState::Failed {
                error: String::new(),
            },
        ]
    }

    #[test]
    fn step_statuses_cover_every_state() {
        use StepStatus::*;
        let expected = [
            [Active, Pending, Pending, Pending],
            [Failed, Pending, Pending, Pending],
            [Done, Pending, Pending, Pending],
            [Done, Active, Pending, Pending],
            [Done, Done, Pending, Pending],
            [Done, Done, Active, Pending],
            [Done, Done, Active, Pending],
            [Done, Done, Done, Done],
            [Pending, Pending, Pending, Pending],
        ];
        let states = every_state();
        assert_eq!(states.len(), expected.len());
        for (state, want) in states.iter().zip(expected) {
            assert_eq!(step_statuses(state), want, "{state:?}");
        }
    }

    #[test]
    fn a_failure_marks_the_step_that_was_active() {
        use StepStatus::*;
        let failed = SetupState::Failed {
            error: String::new(),
        };
        let installing = SetupState::Installing {
            progress_percent: 20,
            step: String::new(),
        };
        assert_eq!(failing_step(&SetupState::Checking), Some(0));
        assert_eq!(failing_step(&installing), Some(1));
        assert_eq!(
            step_statuses_after(&failed, failing_step(&installing)),
            [Done, Failed, Pending, Pending]
        );
        assert_eq!(
            step_statuses_after(&failed, Some(0)),
            [Failed, Pending, Pending, Pending]
        );
        assert_eq!(
            step_statuses_after(&failed, Some(3)),
            [Done, Done, Done, Failed]
        );
    }

    #[test]
    fn a_failure_without_a_remembered_step_stays_pending() {
        let failed = SetupState::Failed {
            error: String::new(),
        };
        assert_eq!(step_statuses_after(&failed, None), step_statuses(&failed));
        // The remembered step is ignored outside the failed state.
        assert_eq!(
            step_statuses_after(&SetupState::Checking, Some(2)),
            step_statuses(&SetupState::Checking)
        );
    }

    #[test]
    fn the_first_unfinished_step_takes_the_blame_without_an_active_one() {
        let ready = SetupState::SdkReady {
            active_path: PathBuf::new(),
            manifest_digest: String::new(),
        };
        assert_eq!(failing_step(&ready), Some(2));
        let rendered = SetupState::Rendered {
            frame_path: PathBuf::new(),
            build_time: Duration::ZERO,
            render_time: Duration::ZERO,
        };
        assert_eq!(failing_step(&rendered), None);
    }

    #[test]
    fn at_most_one_step_is_active() {
        for state in every_state() {
            let active = step_statuses(&state)
                .iter()
                .filter(|s| **s == StepStatus::Active)
                .count();
            assert!(active <= 1, "{state:?}");
        }
    }
}
