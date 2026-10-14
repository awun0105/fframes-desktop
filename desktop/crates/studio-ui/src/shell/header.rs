//! The window header: the project's name and its file, project and export actions.

use gpui::{Context, InteractiveElement, IntoElement, ParentElement, Styled, div, px};
use studio_engine::engine::backend::Command;
use studio_theme::{METRICS, TextStyle, text_style};

use super::{Picker, StudioShell};
use crate::kit::{Card, Icon, icon, with_tooltip};

impl StudioShell {
    pub(super) fn render_header(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let enabled = !self.busy;
        let project = self.presentation.project.clone();
        let mut actions = div().flex().items_center().gap(px(METRICS.space_2));
        for id in header_actions(project.is_some()) {
            let button = match id {
                "create" => self.action_button(
                    "create",
                    "Create…",
                    enabled,
                    false,
                    Some(Icon::Plus),
                    cx,
                    |s, cx| s.pick(Picker::Create, cx),
                ),
                "open" => self.action_button(
                    "open",
                    "Open…",
                    enabled,
                    false,
                    Some(Icon::FolderOpen),
                    cx,
                    |s, cx| s.pick(Picker::Open, cx),
                ),
                "import" => self.action_button(
                    "import",
                    "Import Rust…",
                    enabled,
                    false,
                    Some(Icon::Upload),
                    cx,
                    |s, cx| s.pick(Picker::Import, cx),
                ),
                "export-mp4" => self.action_button(
                    "export-mp4",
                    "Export MP4…",
                    enabled
                        && project.is_some()
                        && self.presentation.agent_sdk.is_some()
                        && self.export_cancelled.is_none(),
                    true,
                    Some(Icon::Download),
                    cx,
                    |s, cx| s.pick(Picker::Export, cx),
                ),
                _ => self.action_button(
                    "close",
                    "Close project",
                    enabled && project.is_some(),
                    false,
                    None,
                    cx,
                    |s, cx| s.dispatch(Command::Close, cx),
                ),
            };
            actions = actions.child(button);
        }
        let title = project
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "fframes Studio".into());
        let mut text = div()
            .flex()
            .flex_col()
            .min_w_0()
            .child(text_style(div().truncate(), TextStyle::Title).child(title));
        if let Some(p) = &project {
            let path = p.root.display().to_string();
            text = text.child(with_tooltip(
                text_style(div().id("project-path").truncate(), TextStyle::Small)
                    .text_color(theme.editor.text_2)
                    .child(path.clone()),
                &theme,
                Some(path.into()),
            ));
        }
        Card::editor(&self.theme)
            .h(px(METRICS.header_h))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(METRICS.space_4))
            .px(px(METRICS.space_4))
            .child(
                div()
                    .flex()
                    .items_center()
                    .min_w_0()
                    .gap(px(METRICS.space_3))
                    .child(icon(Icon::Layers, 20., theme.editor.accent))
                    .child(text),
            )
            .child(actions)
            .into_any_element()
    }
}

/// Ids of the header actions. Create, open and import live in the empty state while
/// no project is open, so each id is rendered once.
fn header_actions(project_open: bool) -> Vec<&'static str> {
    if project_open {
        vec!["create", "open", "import", "export-mp4", "close"]
    } else {
        vec!["export-mp4", "close"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_hides_project_pickers_while_the_empty_state_shows() {
        assert_eq!(header_actions(false), ["export-mp4", "close"]);
        assert_eq!(
            header_actions(true),
            ["create", "open", "import", "export-mp4", "close"]
        );
    }
}
