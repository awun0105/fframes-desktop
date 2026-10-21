//! First-run setup, shown in the window in place of the workspace like the settings frame:
//! the theme, the managed SDK, a first project, the agent and how the agent's permission
//! requests are answered. Every step reuses the control the workspace has for it (the
//! Appearance section, the Project drawer's SDK actions, the empty state's project actions,
//! the Agent settings page). The agent comes after the project because an agent is set up
//! through an open project's workflow; the choice is saved for every project.

use gpui::{Context, IntoElement, ParentElement, Styled, div, px};
use studio_engine::app_settings::{AppSettings, needs_onboarding};
use studio_proto::PermissionPolicy;
use studio_theme::METRICS;

use super::StudioShell;
use crate::kit::{
    Card, Icon, RadioCard, RadioCards, Section, SettingsFrame, SettingsNavItem, muted,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Welcome,
    Appearance,
    Sdk,
    Project,
    Agent,
    Permissions,
}

impl Step {
    pub(super) const ALL: [Step; 6] = [
        Step::Welcome,
        Step::Appearance,
        Step::Sdk,
        Step::Project,
        Step::Agent,
        Step::Permissions,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|step| *step == self).unwrap_or(0)
    }

    /// The step's id (telemetry, nav row `onboarding-<name>`).
    pub(super) fn name(self) -> &'static str {
        match self {
            Step::Welcome => "welcome",
            Step::Appearance => "appearance",
            Step::Sdk => "sdk",
            Step::Project => "project",
            Step::Agent => "agent",
            Step::Permissions => "permissions",
        }
    }

    fn nav_item(self, selected: bool) -> SettingsNavItem {
        let (label, icon) = match self {
            Step::Welcome => ("Welcome", Icon::Info),
            Step::Appearance => ("Appearance", Icon::Palette),
            Step::Sdk => ("SDK", Icon::Package),
            Step::Project => ("First video", Icon::FolderOpen),
            Step::Agent => ("Agent", Icon::Bot),
            Step::Permissions => ("Permissions", Icon::CircleCheck),
        };
        SettingsNavItem::new(format!("onboarding-{}", self.name()), label, icon, selected)
    }
}

/// The setup while it is open.
pub(super) struct Onboarding {
    pub(super) step: Step,
    /// The permission policy chosen on the Permissions step, saved by Finish.
    pub(super) policy: PermissionPolicy,
}

/// The choices of the Permissions step and the Settings row, in this order.
const POLICY_TEXT: [(PermissionPolicy, &str); 3] = [
    (
        PermissionPolicy::AskEveryTime,
        "The agent stops and asks before every action.",
    ),
    (
        PermissionPolicy::AllowInWorkingCopy,
        "The agent reads and edits files in its private copy of your project without asking. \
         Running commands or using the network still asks. You review or undo every change.",
    ),
    (
        PermissionPolicy::AllowEverything,
        "The agent also runs commands, such as cargo check, and uses the network without \
         asking. Commands run on this computer and are not sandboxed.",
    ),
];

impl StudioShell {
    /// Decides once, when the app's storage is known, whether the setup opens, and reads
    /// the saved permission policy. An install that already has an SDK and an agent and
    /// never recorded the setup is recorded as set up instead.
    pub(super) fn check_onboarding(&mut self, cx: &mut Context<Self>) {
        if self.setup_checked {
            return;
        }
        let Some(paths) = self.presentation.paths.clone() else {
            return;
        };
        self.setup_checked = true;
        let settings = AppSettings::load(&paths);
        self.permission_policy = settings.permission_policy;
        self.onboarding_completed = settings.onboarding_completed;
        self.update_check_disabled = settings.update_check_disabled;
        let agent_selected = studio_engine::providers::host::load_settings(&paths)
            .ok()
            .flatten()
            .is_some();
        let sdk_ready = self.presentation.agent_sdk.is_some();
        if needs_onboarding(&settings, sdk_ready, agent_selected) {
            self.onboarding = Some(Onboarding {
                step: Step::Welcome,
                // A first setup suggests the working-copy policy; it is saved only by Finish.
                policy: if settings.onboarding_completed.is_none() {
                    PermissionPolicy::AllowInWorkingCopy
                } else {
                    settings.permission_policy
                },
            });
        } else if settings.onboarding_completed.is_none() {
            self.onboarding_completed = Some(true);
            self.save_setup_choices(cx);
        }
        self.apply_permission_policy(cx);
        cx.notify();
    }

    /// Settings → Run setup again: opens the setup at its first step with today's choices.
    pub(super) fn run_setup_again(&mut self, cx: &mut Context<Self>) {
        self.onboarding = Some(Onboarding {
            step: Step::Welcome,
            policy: self.permission_policy,
        });
        self.close_settings(cx);
    }

    fn go_to_step(&mut self, step: Step, cx: &mut Context<Self>) {
        if let Some(onboarding) = &mut self.onboarding {
            onboarding.step = step;
            cx.notify();
        }
    }

    /// Finish: records the setup as done with the permission policy it chose.
    fn finish_onboarding(&mut self, cx: &mut Context<Self>) {
        let Some(onboarding) = self.onboarding.take() else {
            return;
        };
        self.permission_policy = onboarding.policy;
        self.onboarding_completed = Some(true);
        self.apply_permission_policy(cx);
        self.save_setup_choices(cx);
        cx.notify();
    }

    /// Skip setup: records the setup as done and keeps the saved permission policy.
    fn skip_onboarding(&mut self, cx: &mut Context<Self>) {
        if self.onboarding.take().is_none() {
            return;
        }
        self.onboarding_completed = Some(true);
        self.save_setup_choices(cx);
        cx.notify();
    }

    /// Settings → Agent: a new permission policy, for the next task.
    pub(super) fn choose_permission_policy(
        &mut self,
        policy: PermissionPolicy,
        cx: &mut Context<Self>,
    ) {
        if self.permission_policy == policy {
            return;
        }
        self.permission_policy = policy;
        self.apply_permission_policy(cx);
        self.save_setup_choices(cx);
        cx.notify();
    }

    /// Hands the policy to the open project's agent and its panel.
    pub(super) fn apply_permission_policy(&mut self, cx: &mut Context<Self>) {
        if let Some(workflow) = &self.agent.workflow {
            let _ = workflow.set_permission_policy(self.permission_policy);
        }
        let policy = self.permission_policy;
        self.panel
            .update(cx, |panel, cx| panel.set_permission_policy(policy, cx));
    }

    /// The three policies as radio cards; `on_choose` gets the one clicked.
    pub(super) fn permission_choice(
        &self,
        selected: PermissionPolicy,
        on_choose: impl Fn(&mut Self, PermissionPolicy, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let entity = cx.entity();
        RadioCards::plain(
            &self.theme,
            "permission-policy",
            POLICY_TEXT
                .iter()
                .map(|(policy, text)| RadioCard::new(policy.label(), Some(text)))
                .collect(),
            POLICY_TEXT
                .iter()
                .position(|(policy, _)| *policy == selected),
        )
        .on_select(move |index, _, cx| {
            if let Some((policy, _)) = POLICY_TEXT.get(*index) {
                entity.update(cx, |shell, cx| on_choose(shell, *policy, cx));
            }
        })
        .into_any_element()
    }

    pub(super) fn render_onboarding(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let Some(onboarding) = &self.onboarding else {
            return div().into_any_element();
        };
        let step = onboarding.step;
        let entity = cx.entity();
        let page = self.onboarding_page(step, onboarding.policy, cx);
        let last = step == Step::Permissions;
        let previous = Step::ALL.get(step.index().wrapping_sub(1)).copied();
        let next = Step::ALL.get(step.index() + 1).copied();
        let footer = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(METRICS.space_2))
            .pt(px(METRICS.space_2))
            .child(self.action_button(
                "onboarding-skip",
                "Skip setup",
                true,
                false,
                None,
                cx,
                |shell, cx| shell.skip_onboarding(cx),
            ))
            .child(div().flex_1())
            .child(self.action_button(
                "onboarding-back",
                "Back",
                previous.is_some(),
                false,
                None,
                cx,
                move |shell, cx| {
                    if let Some(previous) = previous {
                        shell.go_to_step(previous, cx);
                    }
                },
            ))
            .child(self.action_button(
                "onboarding-next",
                if last { "Finish" } else { "Next" },
                true,
                true,
                None,
                cx,
                move |shell, cx| match next {
                    Some(next) => shell.go_to_step(next, cx),
                    None => shell.finish_onboarding(cx),
                },
            ));
        let frame = SettingsFrame::settings(
            &theme,
            "onboarding",
            Step::ALL
                .iter()
                .map(|item| item.nav_item(*item == step))
                .collect(),
        )
        .on_select(move |index, _, cx| {
            if let Some(step) = Step::ALL.get(*index) {
                entity.update(cx, |shell, cx| shell.go_to_step(*step, cx));
            }
        })
        .children([page, footer.into_any_element()]);
        Card::editor(&theme)
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(frame)
            .into_any_element()
    }

    fn onboarding_page(
        &self,
        step: Step,
        policy: PermissionPolicy,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = self.theme;
        let section = |title: &'static str, text: &'static str, children: Vec<gpui::AnyElement>| {
            Section::settings(&theme, title, Some(text), None, children).into_any_element()
        };
        match step {
            Step::Welcome => section(
                "Welcome to fframes Studio",
                "Studio turns a Rust project into a video you can preview, change with an AI \
                 agent and export. This setup takes a minute: the look, the SDK that builds \
                 your videos, a first video, an agent, and how the agent asks for permission. \
                 Every step can be skipped and changed later in Settings.",
                Vec::new(),
            ),
            Step::Appearance => self.render_appearance(cx),
            Step::Sdk => {
                let mut children = vec![
                    Card::settings(&theme, true)
                        .child(self.sdk_controls(cx))
                        .into_any_element(),
                ];
                if self.presentation.agent_sdk.is_none() {
                    children.push(
                        muted(
                            &theme,
                            "Without the SDK, Studio cannot build previews or export. You can \
                             install it later from Settings → SDK.",
                        )
                        .into_any_element(),
                    );
                }
                section(
                    "Video SDK",
                    "Studio builds and previews videos with the fframes SDK. Install it once; \
                     every project uses it.",
                    children,
                )
            }
            Step::Project => {
                let mut children = vec![self.project_actions(cx).into_any_element()];
                if let Some(project) = &self.presentation.project {
                    children.insert(
                        0,
                        muted(
                            &theme,
                            format!(
                                "Open: {} · {}",
                                project.name,
                                studio_bootstrap::external_path(&project.root).display()
                            ),
                        )
                        .into_any_element(),
                    );
                }
                section(
                    "Your first video",
                    "Create a new video project, open one, or import a Rust fframes project. \
                     The agent is set up for the open project next.",
                    children,
                )
            }
            Step::Agent => section(
                "AI agent",
                "Pick the coding agent that edits your videos. Detect agents only looks for \
                 installed agents; nothing starts until you select one. The choice is saved \
                 for every project.",
                vec![self.setup_page.clone().into_any_element()],
            ),
            Step::Permissions => section(
                "Agent permissions",
                "Choose how the agent's requests are answered. Change it anytime in \
                 Settings → Agent.",
                vec![self.permission_choice(
                    policy,
                    |shell, policy, cx| {
                        if let Some(onboarding) = &mut shell.onboarding {
                            onboarding.policy = policy;
                            cx.notify();
                        }
                    },
                    cx,
                )],
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_put_the_project_before_the_agent_and_end_with_permissions() {
        let names: Vec<_> = Step::ALL.iter().map(|step| step.name()).collect();
        assert_eq!(
            names,
            [
                "welcome",
                "appearance",
                "sdk",
                "project",
                "agent",
                "permissions"
            ]
        );
        let policies: Vec<_> = POLICY_TEXT.iter().map(|(policy, _)| *policy).collect();
        assert_eq!(policies, PermissionPolicy::ALL);
    }
}
