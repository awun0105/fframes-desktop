use gpui::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use studio_bootstrap::ProcessTreeManager;
use studio_sdk::{
    Doctor, ProbeStatus, environment::SdkEnvironment, manifest::CompatibilityManifest,
    project::ProjectManager,
};
use studio_theme::{METRICS, TextStyle, Theme, text_style};
use studio_ui::kit::{
    Badge, Card, EditorButton, Icon, Notice, StatusDot, Tone, divider, heading, icon, muted,
};

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
        /// The step that was active when setup failed (index into [`STEP_NAMES`]).
        step: Option<usize>,
    },
}

pub struct SetupView {
    pub state: SetupState,
    pub manifest: CompatibilityManifest,
    pub sdk_home: PathBuf,
    pub projects_root: PathBuf,
    pub process_tree: ProcessTreeManager,
}

impl SetupView {
    /// Shared discovery for the spike and product shell; callers run this off the UI thread.
    pub fn defaults() -> Result<(CompatibilityManifest, PathBuf), String> {
        studio_engine::sdk_setup::defaults_in(studio_engine::sdk_setup::default_sdk_home())
    }

    pub fn new(cx: &mut Context<Self>) -> Self {
        let defaults = Self::defaults();
        let (manifest, sdk_home) = defaults.as_ref().cloned().unwrap_or_else(|_| {
            (
                CompatibilityManifest::default_linux_x64(),
                studio_engine::sdk_setup::default_sdk_home(),
            )
        });
        let mut view = Self {
            state: SetupState::Checking,
            manifest,
            sdk_home,
            projects_root: default_projects_root(),
            process_tree: ProcessTreeManager::new(),
        };
        if let Err(error) = defaults {
            view.fail(error);
        } else {
            view.run_preflight(cx);
        }
        view
    }

    /// Enters `Failed`, recording which step was active so it can be marked.
    fn fail(&mut self, error: String) {
        self.state = failed_from(&self.state, error);
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

    fn execute_install_and_build_pipeline(
        manifest: CompatibilityManifest,
        sdk_home: PathBuf,
        projects_root: PathBuf,
        process_tree: ProcessTreeManager,
    ) -> Result<(PathBuf, Duration, Duration), String> {
        let active_sdk = studio_engine::sdk_setup::install_sdk(
            &manifest,
            &sdk_home,
            &process_tree,
            env!("CARGO_PKG_VERSION"),
        )?;
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

    /// Same ids and labels as before; the handler runs on this view. `primary` is the
    /// accent fill, otherwise the control fill.
    fn action_button(
        theme: &Theme,
        id: &'static str,
        label: &'static str,
        primary: bool,
        entity: Entity<Self>,
        action: fn(&mut Self, &mut Context<Self>),
    ) -> EditorButton {
        let button = EditorButton::plain(theme, id)
            .label(label)
            .on_click(move |_, _, cx| {
                entity.update(cx, action);
            });
        if primary {
            button.accent(theme)
        } else {
            button.filled(theme)
        }
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
/// In `Failed` the recorded step is marked failed, the steps before it done and the
/// ones after it pending; without a recorded step every step stays pending.
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
        SetupState::Failed {
            step: Some(failed), ..
        } => std::array::from_fn(|i| match i.cmp(failed) {
            std::cmp::Ordering::Less => Done,
            std::cmp::Ordering::Equal => Failed,
            std::cmp::Ordering::Greater => Pending,
        }),
        SetupState::Failed { step: None, .. } => [Pending, Pending, Pending, Pending],
    }
}

/// The `Failed` state entered from `previous`: a repeat failure keeps the original step.
fn failed_from(previous: &SetupState, error: String) -> SetupState {
    let step = match previous {
        SetupState::Failed { step, .. } => *step,
        other => failing_step(other),
    };
    SetupState::Failed { error, step }
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

fn step_row(theme: &Theme, name: &'static str, status: StepStatus) -> gpui::Div {
    let palette = theme.editor;
    let metrics = METRICS;
    let marker = match status {
        StepStatus::Pending => StatusDot::new(Tone::Neutral).into_any_element(),
        StepStatus::Active => StatusDot::new(Tone::Info).into_any_element(),
        StepStatus::Done => {
            icon(Icon::CircleCheck, metrics.icon_md, theme.green_11).into_any_element()
        }
        StepStatus::Failed => {
            icon(Icon::AlertTriangle, metrics.icon_md, theme.red_11).into_any_element()
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
            text_style(div(), TextStyle::Label)
                .text_color(name_color)
                .child(name),
        )
        .children(
            (status == StepStatus::Active).then(|| Badge::new("In progress").tone(Tone::Info)),
        )
}

fn detail_line(theme: &Theme, text: impl Into<gpui::SharedString>) -> gpui::Div {
    muted(theme, text)
}

impl SetupView {
    /// Status text and extra content for the current state.
    fn state_detail(&self, theme: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let palette = theme.editor;
        let metrics = METRICS;
        let entity = cx.entity();
        let column = || div().flex().flex_col().gap(px(metrics.space_2));
        match &self.state {
            SetupState::Checking => column()
                .child(detail_line(theme, "Checking Host Prerequisites..."))
                .child(detail_line(theme,
                    "Running read-only preflight probes on compiler, CMake, and graphics libraries...",
                )),
            SetupState::PrerequisitesNeeded {
                missing_packages, ..
            } => column()
                .child(detail_line(theme, "System Prerequisites Needed"))
                .child(detail_line(theme,
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
                            text_style(div(), TextStyle::Small)
                                .text_color(palette.text_1)
                                .child(studio_sdk::install_hint(missing_packages)),
                        ),
                )
                .child(div().flex().child(Self::action_button(
                    theme,
                    "btn-recheck",
                    "Re-run Preflight Check",
                    false,
                    entity,
                    |this, cx| this.run_preflight(cx),
                ))),
            SetupState::SdkInstallNeeded { manifest_digest } => column()
                .child(detail_line(theme,
                    "Host Preflight Passed — SDK Installation Required",
                ))
                .child(detail_line(theme,
                    "Host environment verified. Ready to install app-managed toolchain & FFmpeg dependencies.",
                ))
                .child(detail_line(theme, format!("Manifest: {:.16}...", manifest_digest)))
                .child(div().flex().child(Self::action_button(
                    theme,
                    "btn-install-sdk",
                    "Install Managed SDK & Build Template",
                    true,
                    entity,
                    |this, cx| this.start_install_and_build(cx),
                ))),
            SetupState::Installing {
                progress_percent,
                step,
            } => column()
                .child(detail_line(theme, format!(
                    "Installing SDK... ({progress_percent}%)"
                )))
                .child(detail_line(theme, step.clone())),
            SetupState::SdkReady {
                active_path,
                manifest_digest,
            } => column()
                .child(detail_line(theme, "Managed SDK Ready"))
                .child(detail_line(theme, format!("Active location: {:?}", active_path)))
                .child(detail_line(theme, format!(
                    "Manifest Digest: {:.16}...",
                    manifest_digest
                )))
                .child(div().flex().child(Self::action_button(
                    theme,
                    "btn-create-proj",
                    "Build and Render Template Project",
                    true,
                    entity,
                    |this, cx| this.start_install_and_build(cx),
                ))),
            SetupState::CreatingProject { name } => column().child(detail_line(theme, format!(
                "Generating project '{name}' outside workspace..."
            ))),
            SetupState::Building { name } => column().child(detail_line(theme, format!(
                "Building '{name}' with isolated SDK toolchain..."
            ))),
            SetupState::Rendered {
                frame_path,
                build_time,
                render_time,
            } => column()
                .child(detail_line(theme, "Project Rendered Successfully!"))
                .child(detail_line(theme, format!("Build duration: {:?}", build_time)))
                .child(detail_line(theme, format!(
                    "Frame render duration: {:?}",
                    render_time
                )))
                .child(detail_line(theme, format!("Output PNG: {:?}", frame_path))),
            SetupState::Failed { error, .. } => column().child(
                Notice::new("setup-failed", Tone::Danger, "Setup / Build Failed")
                    .details(error.clone())
                    .action(Self::action_button(
                    theme,
                        "btn-retry",
                        "Retry",
                        false,
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_window(window, cx);
        let palette = theme.editor;
        let metrics = METRICS;
        let steps = div().flex().flex_col().gap(px(metrics.space_2)).children(
            STEP_NAMES
                .iter()
                .zip(step_statuses(&self.state))
                .map(|(name, status)| step_row(&theme, name, status)),
        );

        div()
            .flex()
            .w_full()
            .justify_center()
            .p(px(metrics.space_6))
            .bg(palette.window)
            .child(
                Card::editor(&theme)
                    .flex()
                    .flex_col()
                    .w_full()
                    .max_w(px(CARD_MAX_W))
                    .p(px(CARD_PADDING))
                    .gap(px(metrics.space_4))
                    .child(heading(&theme, "Set up fframes Studio"))
                    .child(muted(
                        &theme,
                        "Get the managed SDK ready and render a template project.",
                    ))
                    .child(steps)
                    .child(divider(&theme))
                    .child(self.state_detail(&theme, cx)),
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
                step: None,
            },
        ]
    }

    #[gpui::test]
    fn every_state_renders(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| SetupView {
            state: SetupState::Checking,
            manifest: CompatibilityManifest::default_linux_x64(),
            sdk_home: PathBuf::new(),
            projects_root: PathBuf::new(),
            process_tree: ProcessTreeManager::new(),
        });
        for state in every_state() {
            view.update(cx, |view, cx| {
                view.state = state;
                cx.notify();
            });
            cx.run_until_parked();
        }
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

    fn failed_step(state: &SetupState) -> Option<usize> {
        match state {
            SetupState::Failed { step, .. } => *step,
            _ => panic!("not failed: {state:?}"),
        }
    }

    #[test]
    fn a_second_failure_keeps_the_original_step() {
        let installing = SetupState::Installing {
            progress_percent: 20,
            step: String::new(),
        };
        let first = failed_from(&installing, "first".into());
        assert_eq!(failed_step(&first), Some(1));
        let second = failed_from(&first, "second".into());
        assert_eq!(failed_step(&second), Some(1));
        assert_eq!(
            failed_step(&failed_from(&SetupState::Checking, String::new())),
            Some(0)
        );
    }

    #[test]
    fn a_failure_marks_the_step_that_was_active() {
        use StepStatus::*;
        let failed = |step| SetupState::Failed {
            error: String::new(),
            step,
        };
        let installing = SetupState::Installing {
            progress_percent: 20,
            step: String::new(),
        };
        assert_eq!(failing_step(&SetupState::Checking), Some(0));
        assert_eq!(failing_step(&installing), Some(1));
        assert_eq!(
            step_statuses(&failed_from(&installing, String::new())),
            [Done, Failed, Pending, Pending]
        );
        assert_eq!(
            step_statuses(&failed(Some(0))),
            [Failed, Pending, Pending, Pending]
        );
        assert_eq!(step_statuses(&failed(Some(3))), [Done, Done, Done, Failed]);
        assert_eq!(
            step_statuses(&failed(None)),
            [Pending, Pending, Pending, Pending]
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
