//! The tabbed sidebar right of the player (Agent, Project, Assets and Style), and the empty
//! state with recent projects.

use std::path::PathBuf;

use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div, prelude::FluentBuilder, px,
};
use studio_engine::engine::backend::Command;
use studio_theme::{METRICS, TextStyle, text_style};

use super::sections::{Section, box_probe, section_probe};
use super::settings::SettingsPage;
use super::{Picker, StudioShell};
use crate::kit::{
    Badge, Card, CollapsibleSection, DISABLED_OPACITY, EditorButton, Handler, Icon, IconButton,
    IconButtonSize, StatusDot, TabRail, TabRailItem, Tone, divider, focusable, heading, icon,
    muted, on_activate, section_title, selectable, with_tooltip,
};

impl StudioShell {
    /// The sidebar card right of the player, after Cap's config sidebar: the tab rail
    /// above the selected tab's body. It is not cached itself; the agent panel and the
    /// drawer contents are separate cached sections, and only the selected one is drawn.
    pub(super) fn render_sidebar(
        &self,
        sections: &super::sections::Sections,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = self.theme;
        let entity = cx.entity();
        let selected = self.sidebar_tab;
        let attention = self.agent_attention.waiting && selected != SidebarTab::Agent;
        let rail = TabRail::editor(
            &theme,
            "sidebar-tabs",
            theme.editor.card.into(),
            SidebarTab::ALL
                .iter()
                .map(|tab| {
                    TabRailItem::new(tab.icon().path(), tab.name(), selected == *tab, false)
                        .id(format!("sidebar-tab-{}", tab.name()))
                        .dot(*tab == SidebarTab::Agent && attention)
                })
                .collect(),
        )
        .height(px(METRICS.tab_bar_h))
        .on_select(move |index, _, cx| {
            entity.update(cx, |shell, cx| {
                shell.select_sidebar_tab(SidebarTab::from_index(*index), cx)
            });
        });
        let body = match selected {
            SidebarTab::Agent => sections.agent.clone(),
            _ => sections.drawer.clone(),
        };
        Card::editor(&theme)
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(box_probe("agent-column", cx))
            .child(rail)
            .child(body.cached(gpui::StyleRefinement::default().flex_1().min_h_0().w_full()))
            .into_any_element()
    }

    pub(super) fn select_sidebar_tab(&mut self, tab: SidebarTab, cx: &mut Context<Self>) {
        if self.sidebar_tab != tab {
            self.sidebar_tab = tab;
            cx.notify();
        }
    }

    /// The selected Project, Assets, Style or Props tab's contents (a cached section).
    pub(super) fn render_drawer(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let content = match self.sidebar_tab {
            SidebarTab::Assets => self.render_assets_tab(cx),
            SidebarTab::Style => self.render_style_tab(cx),
            SidebarTab::Props => self.props_panel.clone().into_any_element(),
            // The Agent tab draws the agent section instead; this one is not in the tree.
            SidebarTab::Project | SidebarTab::Agent => self.render_project_tab(cx),
        };
        div()
            .id("drawer")
            .relative()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .p(px(METRICS.space_4))
            .child(content)
            .child(section_probe(Section::Drawer, cx))
            .into_any_element()
    }

    /// A 28 px file row: icon, truncated name, hover fill. The full name is the tooltip.
    fn list_row(
        &self,
        id: String,
        label: String,
        glyph: Icon,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        let entity = cx.entity();
        let handler: Handler = std::rc::Rc::new(move |_, cx| {
            entity.update(cx, |shell, cx| action(shell, cx));
        });
        let row = div()
            .id(id)
            .role(gpui::Role::Button)
            .aria_label(label.clone())
            .flex()
            .flex_none()
            .items_center()
            .gap(px(METRICS.space_2))
            .h(px(LIST_ROW_H))
            .px(px(METRICS.space_2))
            .min_w_0()
            .rounded(px(METRICS.radius_item))
            .hover(move |style| style.bg(theme.editor.ctl_hover))
            .child(icon(glyph, METRICS.icon_sm, theme.editor.text_2))
            .child(
                text_style(div().min_w_0().flex_1().truncate(), TextStyle::Body)
                    .text_color(theme.editor.text_1)
                    .child(label.clone()),
            );
        let row = on_activate(
            focusable(row, &theme, false).cursor_pointer(),
            false,
            Some(handler),
        );
        with_tooltip(row, &theme, Some(label.into()))
    }

    /// The file list of a project tab, or the hint shown while no project is open.
    fn file_list(
        &self,
        files: Option<&[String]>,
        root: Option<PathBuf>,
        glyph: Icon,
        empty: &'static str,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        let mut list = div()
            .id("inventory")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_y_scroll();
        let (Some(files), Some(root)) = (files, root) else {
            return list.child(
                text_style(div(), TextStyle::Body)
                    .text_color(theme.editor.text_2)
                    .child(selectable("Create or open a project to browse its files.")),
            );
        };
        if files.is_empty() {
            list = list.child(
                text_style(div(), TextStyle::Body)
                    .text_color(theme.editor.text_2)
                    .child(selectable(empty)),
            );
        }
        for file in files {
            let path = root.join(file);
            list = list.child(self.list_row(
                format!("file-{file}"),
                file.clone(),
                glyph,
                cx,
                move |_, cx| cx.reveal_path(&path),
            ));
        }
        list
    }

    fn render_project_tab(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let project = self.presentation.project.clone();
        let mut title = section_title(&theme, "Files");
        if let Some(p) = &project {
            title = title.child(Badge::new(p.file_count.to_string()));
        }
        let files = self.file_list(
            project.as_ref().map(|p| p.files.as_slice()),
            project.as_ref().map(|p| p.root.clone()),
            Icon::File,
            "No files to show",
            cx,
        );
        let mut tab = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_2))
            .child(title)
            .child(files);
        if let Some(p) = &project
            && p.file_count > p.files.len()
        {
            tab = tab.child(
                text_style(div(), TextStyle::Small)
                    .flex_none()
                    .text_color(theme.editor.text_3)
                    .child(selectable(format!(
                        "Showing {} of {} files",
                        p.files.len(),
                        p.file_count
                    ))),
            );
        }

        if project.is_some() {
            let entity = cx.entity();
            let library = div()
                .id("left-library")
                .flex_none()
                .max_h(px(360.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(METRICS.space_3))
                .child(divider(&self.theme))
                .child(
                    CollapsibleSection::new("recents-section", self.recents_open)
                        .header(section_title(&self.theme, "Recent projects"))
                        .content(div().pt(px(METRICS.space_2)).child(self.render_recents(cx)))
                        .on_toggle(move |open, _, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.recents_open = *open;
                                cx.notify();
                            });
                        }),
                );
            tab = tab.child(library);
        }
        tab.into_any_element()
    }

    fn render_assets_tab(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let project = self.presentation.project.clone();
        let files = self.file_list(
            project.as_ref().map(|p| p.assets.as_slice()),
            project.as_ref().map(|p| p.root.clone()),
            Icon::Image,
            "No assets imported",
            cx,
        );
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_2))
            .child(section_title(&self.theme, "Assets"))
            .child(files)
            .child(self.action_button(
                "asset",
                "Copy asset…",
                !self.busy && project.is_some(),
                false,
                Some(Icon::Plus),
                cx,
                |s, cx| s.pick(Picker::Asset, cx),
            ))
            .into_any_element()
    }

    fn render_style_tab(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let mut tab = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_2));
        if let Some(p) = &self.presentation.project {
            if p.styles.is_empty() {
                tab = tab.child(
                    text_style(div(), TextStyle::Small)
                        .text_color(theme.editor.text_3)
                        .child(selectable("No style preset configured")),
                );
            }
            for file in &p.styles {
                let path = p.root.join(file);
                tab = tab.child(self.list_row(
                    format!("file-{file}"),
                    file.clone(),
                    Icon::File,
                    cx,
                    move |_, cx| cx.reveal_path(&path),
                ));
            }
        }
        // The presets themselves are chosen in Settings → Presets.
        tab.child(self.action_button(
            "open-presets",
            "Style presets…",
            true,
            false,
            Some(Icon::Palette),
            cx,
            |s, cx| s.open_settings_page(SettingsPage::Presets, cx),
        ))
        .into_any_element()
    }

    /// The recent-projects list. Rendered once: in the left column while a project is
    /// open, in the empty state otherwise, so its element ids stay unique.
    fn render_recents(&self, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        let enabled = !self.busy;
        let mut recents = div()
            .id("recents")
            .flex()
            .flex_col()
            .gap(px(METRICS.space_1));
        if self.presentation.recents.is_empty() {
            recents = recents.child(
                text_style(div(), TextStyle::Body)
                    .text_color(theme.editor.text_2)
                    .child(selectable("No recent projects")),
            );
        }
        for recent in self.presentation.recents.clone() {
            let id = String::from(recent.id.clone());
            let path = recent.path.clone();
            let project_id = recent.id.clone();
            let group = format!("recent-group-{id}");
            let label = format!(
                "{}{}",
                recent.name,
                if recent.missing { " · Missing" } else { "" }
            );
            let open_enabled = enabled && !recent.missing;
            let entity = cx.entity();
            let open: Handler = std::rc::Rc::new(move |_, cx| {
                entity.update(cx, |shell, cx| {
                    shell.dispatch(Command::Open(path.clone()), cx)
                });
            });
            let mut name_line = div().flex().items_center().gap(px(METRICS.space_2)).child(
                text_style(div().min_w_0().flex_1().truncate(), TextStyle::Body)
                    .text_color(theme.editor.text_1)
                    .child(recent.name.clone()),
            );
            if recent.missing {
                name_line = name_line.child(Badge::new("Missing").tone(Tone::Warning));
            }
            let main = div()
                .id(format!("recent-{id}"))
                .role(gpui::Role::Button)
                .aria_label(label)
                .flex()
                .flex_col()
                .w_full()
                .min_w_0()
                .px(px(METRICS.space_2))
                .py(px(METRICS.space_1))
                .rounded(px(METRICS.radius_item))
                .child(name_line)
                .child(
                    text_style(div().truncate(), TextStyle::Small)
                        .text_color(theme.editor.text_3)
                        .child(
                            studio_bootstrap::external_path(&recent.path)
                                .display()
                                .to_string(),
                        ),
                );
            let main = if open_enabled {
                main.hover(move |style| style.bg(theme.editor.ctl_hover))
                    .cursor_pointer()
            } else {
                main.opacity(DISABLED_OPACITY)
            };
            let main = on_activate(
                focusable(main, &theme, !open_enabled),
                !open_enabled,
                Some(open),
            );
            let main = with_tooltip(
                main,
                &theme,
                Some(
                    studio_bootstrap::external_path(&recent.path)
                        .display()
                        .to_string()
                        .into(),
                ),
            );

            // Row actions appear on pointer hover and while keyboard focus is inside.
            let mut actions = div()
                .id(format!("recent-actions-{id}"))
                .focusable()
                .absolute()
                .top(px(METRICS.space_1))
                .right(px(METRICS.space_1))
                .flex()
                .items_center()
                .gap(px(METRICS.space_1))
                .rounded(px(METRICS.radius_item))
                .bg(theme.editor.ctl_hover)
                .opacity(0.)
                .group_hover(group.clone(), |style| style.opacity(1.))
                .in_focus(|style| style.opacity(1.));
            for action in recent_row_actions(recent.missing) {
                let entity = cx.entity();
                let target = project_id.clone();
                actions = actions.child(match action {
                    "locate" => EditorButton::plain(&theme, format!("locate-{id}"))
                        .label("Locate…")
                        .filled(&theme)
                        .disabled(!enabled)
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.pick(Picker::Locate(target.clone()), cx)
                            });
                        })
                        .into_any_element(),
                    _ => IconButton::editor(&theme, format!("remove-{id}"), Icon::X.path())
                        .editor_size(IconButtonSize::Small)
                        .label("Remove recent")
                        .tooltip(&theme, "Remove recent")
                        .disabled(!enabled)
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.dispatch(Command::RemoveRecent(target.clone()), cx)
                            });
                        })
                        .into_any_element(),
                });
            }
            recents = recents.child(div().relative().group(group).child(main).child(actions));
        }
        recents
    }

    /// The managed SDK's status and its Install, Import and Select actions (Settings → SDK
    /// and the onboarding's SDK step).
    pub(super) fn sdk_controls(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = self.theme;
        let enabled = !self.busy;
        let sdk_ready = self.presentation.agent_sdk.is_some();
        let status = if self.busy {
            "Operation in progress…".to_owned()
        } else {
            self.presentation.sdk.clone()
        };
        div()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_2))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(METRICS.space_2))
                    .child(
                        div()
                            .pt(px(5.))
                            .child(StatusDot::new(sdk_status_tone(self.busy, sdk_ready))),
                    )
                    .child(
                        text_style(div().min_w_0().flex_1(), TextStyle::Small)
                            .text_color(theme.editor.text_2)
                            .child(selectable(status)),
                    ),
            )
            .child(self.action_button(
                "install",
                "Install managed SDK",
                enabled,
                false,
                None,
                cx,
                |s, cx| s.dispatch(Command::InstallSdk, cx),
            ))
            .child(self.action_button(
                "import-sdk-bundle",
                "Import offline SDK bundle…",
                enabled,
                false,
                None,
                cx,
                |s, cx| s.pick(Picker::SdkBundle, cx),
            ))
            .when(self.presentation.sdk_repin.is_some(), |controls| {
                controls.child(self.action_button(
                    "use-installed-sdk",
                    "Use installed SDK",
                    enabled,
                    false,
                    None,
                    cx,
                    |s, cx| s.dispatch(Command::UseInstalledSdk, cx),
                ))
            })
            .child(self.action_button(
                "sdk",
                "Select installed SDK…",
                enabled && self.presentation.project.is_some(),
                false,
                None,
                cx,
                |s, cx| s.pick(Picker::Sdk, cx),
            ))
    }

    /// Create, Open and Import Rust (the empty state and the onboarding's Project step).
    pub(super) fn project_actions(&self, cx: &mut Context<Self>) -> gpui::Div {
        let enabled = !self.busy;
        div()
            .flex()
            .flex_wrap()
            .gap(px(METRICS.space_2))
            .child(self.action_button(
                "create",
                "Create…",
                enabled,
                true,
                Some(Icon::Plus),
                cx,
                |s, cx| s.pick(Picker::Create, cx),
            ))
            .child(self.action_button(
                "open",
                "Open…",
                enabled,
                false,
                Some(Icon::FolderOpen),
                cx,
                |s, cx| s.pick(Picker::Open, cx),
            ))
            .child(self.action_button(
                "import",
                "Import Rust…",
                enabled,
                false,
                Some(Icon::Upload),
                cx,
                |s, cx| s.pick(Picker::Import, cx),
            ))
    }

    pub(super) fn render_empty_state(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let actions = self.project_actions(cx);
        div()
            .id("empty-state")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                Card::editor(&self.theme)
                    .w_full()
                    .max_w(px(520.))
                    .max_h_full()
                    .flex()
                    .flex_col()
                    .gap(px(METRICS.space_4))
                    .p(px(METRICS.space_6))
                    .child(heading(&self.theme, "fframes Studio"))
                    .child(muted(
                        &self.theme,
                        "A native workspace for your video projects",
                    ))
                    .child(actions)
                    .child(section_title(&self.theme, "Recent projects"))
                    .child(self.render_recents(cx).max_h(px(260.)).overflow_y_scroll()),
            )
            .into_any_element()
    }

    /// The sidebar's Agent tab: the panel, which pads its own rows (a cached section).
    pub(super) fn render_agent_column(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .relative()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.panel.clone())
            .child(box_probe("agent", cx))
            .into_any_element()
    }
}

/// The sidebar's tabs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SidebarTab {
    #[default]
    Agent,
    Project,
    Assets,
    Style,
    Props,
}

/// Height of a file or asset row.
const LIST_ROW_H: f32 = 28.;

/// The actions of a recent-project row, as the id prefix of each button: a missing
/// project can be located, every project can be removed.
fn recent_row_actions(missing: bool) -> Vec<&'static str> {
    if missing {
        vec!["locate", "remove"]
    } else {
        vec!["remove"]
    }
}

/// Dot tone of the managed SDK line: busy, ready or missing.
fn sdk_status_tone(busy: bool, ready: bool) -> Tone {
    if busy {
        Tone::Info
    } else if ready {
        Tone::Success
    } else {
        Tone::Warning
    }
}

impl SidebarTab {
    const ALL: [SidebarTab; 5] = [
        SidebarTab::Agent,
        SidebarTab::Project,
        SidebarTab::Assets,
        SidebarTab::Style,
        SidebarTab::Props,
    ];

    /// Also the tab's element id, `sidebar-tab-{name}`.
    pub(super) fn name(self) -> &'static str {
        match self {
            SidebarTab::Agent => "Agent",
            SidebarTab::Project => "Project",
            SidebarTab::Assets => "Assets",
            SidebarTab::Style => "Style",
            SidebarTab::Props => "Props",
        }
    }

    fn icon(self) -> Icon {
        match self {
            SidebarTab::Agent => Icon::Bot,
            SidebarTab::Project => Icon::Folder,
            SidebarTab::Assets => Icon::Image,
            SidebarTab::Style => Icon::Palette,
            SidebarTab::Props => Icon::Layers,
        }
    }

    fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_rows_offer_locate_only_when_missing() {
        assert_eq!(recent_row_actions(true), ["locate", "remove"]);
        assert_eq!(recent_row_actions(false), ["remove"]);
    }

    #[test]
    fn sdk_status_tone_prefers_busy_then_ready() {
        assert_eq!(sdk_status_tone(true, true), Tone::Info);
        assert_eq!(sdk_status_tone(false, true), Tone::Success);
        assert_eq!(sdk_status_tone(false, false), Tone::Warning);
    }

    #[test]
    fn sidebar_tabs_keep_their_element_ids() {
        let names: Vec<_> = SidebarTab::ALL.iter().map(|t| t.name()).collect();
        assert_eq!(names, ["Agent", "Project", "Assets", "Style", "Props"]);
        for (index, tab) in SidebarTab::ALL.into_iter().enumerate() {
            assert_eq!(SidebarTab::from_index(index), tab);
        }
        assert_eq!(SidebarTab::from_index(9), SidebarTab::Agent);
    }
}
