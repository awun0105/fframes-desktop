//! The window header, after Cap's editor header: the project's name, then undo and redo,
//! the project actions, Settings and Export in a row flush with the window's top edge.
//!
//! The header is a cached section. On Windows the row around it (in `layout.rs`) is the
//! window's drag area and holds the caption buttons, because a cached view does not replay
//! window control areas. On macOS the header leaves room for the traffic lights and moves
//! the window from its empty middle.

use gpui::{
    Context, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled, div,
    prelude::FluentBuilder, px,
};
use studio_engine::engine::backend::Command;
use studio_engine::timing::TimelineCommand;
use studio_theme::{METRICS, TextStyle, text_style};

use super::sections::{Section, section_probe};
use super::{Picker, StudioShell};
use crate::kit::{CAPTION_BUTTON_W, Icon, IconButtonSize, selectable};

/// Room for the macOS traffic lights, inset to (20, 20) like Cap's editor.
const TRAFFIC_LIGHTS_W: f32 = 82.;
/// The title keeps at least this much before the actions lose their labels.
const TITLE_MIN_W: f32 = 200.;
/// The action cluster's width with labels, Settings included, with a project open and
/// without one: 652 px (undo, redo and the divider included) and about 282 px in the test
/// platform, plus a margin for other systems' fonts.
const ACTIONS_W: f32 = 680.;
const ACTIONS_CLOSED_W: f32 = 300.;

impl StudioShell {
    pub(super) fn render_header(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let enabled = !self.busy;
        let project = self.presentation.project.clone();
        let compact = compact_header_controls(self.viewport[0], chrome_width(), project.is_some());
        let mut actions = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(METRICS.space_2))
            // Keeps the Windows drag area behind from answering for the buttons.
            .occlude();
        let timeline = self.timeline_controls;
        for id in header_actions(project.is_some()) {
            let button = match id {
                "undo" | "redo" => {
                    let undo = id == "undo";
                    let can = if undo {
                        timeline.can_undo
                    } else {
                        timeline.can_redo
                    };
                    let command = if undo {
                        TimelineCommand::Undo
                    } else {
                        TimelineCommand::Redo
                    };
                    actions = actions.child(self.action_icon_button(
                        if undo {
                            "timeline-undo"
                        } else {
                            "timeline-redo"
                        },
                        if undo { "Undo" } else { "Redo" },
                        if undo { Icon::Undo } else { Icon::Redo },
                        IconButtonSize::Medium,
                        enabled && timeline.editable && can,
                        cx,
                        move |s, cx| s.timeline_history(command.clone(), cx),
                    ));
                    if !undo {
                        actions = actions.child(div().w(px(1.)).h(px(16.)).bg(theme.editor.line));
                    }
                    continue;
                }
                // App-level, so it shows with or without a project; it opens the settings
                // window or brings it to the front.
                "settings" => {
                    actions = actions.child(self.action_icon_button(
                        "settings",
                        "Settings",
                        Icon::Settings,
                        IconButtonSize::Medium,
                        true,
                        cx,
                        |s, cx| s.open_settings(cx),
                    ));
                    continue;
                }
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
                "export" => self.action_button(
                    "export",
                    "Export…",
                    enabled && project.is_some() && self.presentation.agent_sdk.is_some(),
                    true,
                    Some(Icon::Download),
                    cx,
                    |s, cx| s.open_export(cx),
                ),
                // No icon, so it keeps its label in the compact header.
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
            let button = if compact && id != "close" {
                button.icon_only(&theme)
            } else {
                button
            };
            actions = actions.child(button);
        }
        self.header_appearance.set(Some(theme.appearance));
        let title = project
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "fframes Studio".into());
        let mut text = div()
            .flex()
            .items_center()
            .gap(px(METRICS.space_3))
            .min_w_0()
            .child(
                text_style(div().truncate(), TextStyle::BodyStrong)
                    .child(selectable(title).over_drag_area()),
            );
        if let Some(p) = &project {
            let path = studio_bootstrap::external_path(&p.root)
                .display()
                .to_string();
            // Selectable rather than a tooltip: the text blocks the hover a tooltip needs,
            // and a triple click selects the whole path however it is truncated.
            text = text.child(
                text_style(div().id("project-path").truncate(), TextStyle::Small)
                    .text_color(theme.editor.text_2)
                    .child(selectable(path).over_drag_area()),
            );
        }
        div()
            .relative()
            .size_full()
            .flex()
            .items_center()
            .gap(px(METRICS.space_4))
            .px(px(if compact {
                METRICS.space_2
            } else {
                METRICS.space_4
            }))
            .when(cfg!(target_os = "macos"), |header| {
                header.child(div().h_full().w(px(TRAFFIC_LIGHTS_W)).flex_none())
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .min_w_0()
                    .gap(px(METRICS.space_3))
                    .child(
                        gpui::img("branding/studio-icon.png")
                            .w(px(22.))
                            .h(px(22.))
                            .flex_none(),
                    )
                    .child(text),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .when(cfg!(target_os = "macos"), |area| {
                        area.on_mouse_down(MouseButton::Left, |event, window, _| {
                            if event.click_count == 2 {
                                window.titlebar_double_click();
                            } else {
                                window.start_window_move();
                            }
                        })
                    }),
            )
            .child(actions)
            .child(section_probe(Section::Header, cx))
            .into_any_element()
    }
}

/// Width the window's own controls take from the header: the caption buttons on Windows,
/// the traffic lights on macOS.
pub(super) fn chrome_width() -> f32 {
    if cfg!(target_os = "windows") {
        3. * CAPTION_BUTTON_W
    } else if cfg!(target_os = "macos") {
        TRAFFIC_LIGHTS_W
    } else {
        0.
    }
}

/// Whether the header actions drop their labels, after Cap's `compact_header_controls`:
/// when the title, the labelled actions and the window controls do not fit side by side.
fn compact_header_controls(viewport_width: f32, chrome_width: f32, project_open: bool) -> bool {
    let actions = if project_open {
        ACTIONS_W
    } else {
        ACTIONS_CLOSED_W
    };
    viewport_width < TITLE_MIN_W + actions + chrome_width + 2. * METRICS.space_4
}

/// The header actions in Cap's order, Export last. Undo and redo edit the open project's
/// timeline; create, open and import live in the empty state while no project is open, so
/// each id is rendered once.
fn header_actions(project_open: bool) -> Vec<&'static str> {
    if project_open {
        vec![
            "undo", "redo", "create", "open", "import", "close", "settings", "export",
        ]
    } else {
        vec!["close", "settings", "export"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_hides_project_pickers_while_the_empty_state_shows() {
        assert_eq!(header_actions(false), ["close", "settings", "export"]);
        assert_eq!(
            header_actions(true),
            [
                "undo", "redo", "create", "open", "import", "close", "settings", "export"
            ]
        );
    }

    #[test]
    fn the_default_window_keeps_the_labels() {
        for chrome in [0., TRAFFIC_LIGHTS_W, 3. * CAPTION_BUTTON_W] {
            for project_open in [false, true] {
                assert!(!compact_header_controls(1275., chrome, project_open));
            }
        }
    }

    #[test]
    fn the_window_controls_and_an_open_project_need_room() {
        let caption = 3. * CAPTION_BUTTON_W;
        assert!(compact_header_controls(960., caption, true));
        assert!(!compact_header_controls(960., 0., true));
        assert!(!compact_header_controls(960., caption, false));
        assert!(compact_header_controls(400., 0., false));
    }
}
