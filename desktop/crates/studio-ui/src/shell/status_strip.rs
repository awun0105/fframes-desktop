//! The notices under the header and the status strip: errors and the current job.

use gpui::{Context, InteractiveElement, IntoElement, ParentElement, Styled, div, px};
use studio_engine::ProjectMigration;
use studio_engine::engine::backend::Command;
use studio_engine::services::export_service::ExportProgress;
use studio_theme::{METRICS, TextStyle, text_style};

use super::StudioShell;
use crate::kit::{Card, CollapsibleSection, Notice, StatusDot, Tone, selectable};

impl StudioShell {
    /// Warning, error, export and recovery notices; empty when there is nothing to say.
    pub(super) fn render_notices(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let enabled = !self.busy;
        let mut notices = Vec::new();
        if let Some(label) = &self.awaiting_preview {
            notices.push(
                Notice::new("awaiting-preview", Tone::Warning, label.clone()).into_any_element(),
            );
        }
        if let Some(error) = &self.error {
            let (title, details) = error_title_details(error);
            let mut notice = Notice::new("error", Tone::Danger, title);
            if let Some(details) = details {
                notice = notice.details(details);
            }
            if let Some(path) = self.failed_open.clone().filter(|_| {
                error.contains("Duplicate project ID")
                    || error.contains("Relocated project content differs")
            }) {
                notice = notice.action(self.action_button(
                    "independent",
                    "Use this folder as an independent project",
                    enabled,
                    false,
                    None,
                    cx,
                    move |s, cx| s.dispatch(Command::Independent(path.clone()), cx),
                ));
            }
            notices.push(notice.into_any_element());
        }
        let Some(p) = &self.presentation.project else {
            return notices;
        };
        if let Some(status) = &self.export_status {
            let (tone, message) = match status {
                ExportProgress::Started { revision, label } => (
                    Tone::Info,
                    format!("Export snapshot · {label} · {revision}"),
                ),
                ExportProgress::Rendering { done, total } => (
                    Tone::Info,
                    format!("Rendering video · {done}/{total} frames"),
                ),
                ExportProgress::Audio => (Tone::Info, "Writing and verifying audio".into()),
                ExportProgress::Warning(message) => {
                    (Tone::Warning, format!("Export warning · {message}"))
                }
                ExportProgress::Verifying => {
                    (Tone::Info, "Verifying the video before publication".into())
                }
                ExportProgress::Complete {
                    destination,
                    revision,
                } => (
                    Tone::Success,
                    format!("Exported {} · revision {}", destination.display(), revision),
                ),
                ExportProgress::Failed(message) => {
                    (Tone::Danger, format!("Export failed · {message}"))
                }
                ExportProgress::Cancelled => (
                    Tone::Info,
                    "Export cancelled · partial output removed".into(),
                ),
            };
            let mut notice = Notice::new("export-status", tone, message);
            if self.export_cancelled.is_some() {
                notice = notice.action(self.action_button(
                    "cancel-export",
                    "Cancel export",
                    true,
                    false,
                    None,
                    cx,
                    |shell, cx| shell.cancel_export(cx),
                ));
            }
            notices.push(notice.into_any_element());
        }
        if p.interrupted {
            notices.push(
                Notice::new(
                    "interrupted-job",
                    Tone::Warning,
                    if p.draft.is_some() {
                        "Interrupted job · retained draft available"
                    } else {
                        "Interrupted before draft publication · source and saved checkpoint are safe"
                    },
                )
                .into_any_element(),
            );
        }
        if let Some(text) = &p.recovery_notice {
            notices.push(
                Notice::new("recovery-notice", Tone::Warning, text.clone()).into_any_element(),
            );
        }
        match &p.migration {
            ProjectMigration::Current => (),
            ProjectMigration::Available => {
                let source = p.source.clone();
                notices.push(
                    Notice::new(
                        "migration",
                        Tone::Info,
                        "This project uses the earlier framework · migrate it to edit timing and export settings",
                    )
                    .details(
                        "Migration updates Cargo.toml, the Studio worker, src/main.rs, AGENTS.md and studio.json; your scenes in src/lib.rs stay as they are. Undo it from More actions.",
                    )
                    .action(self.action_button(
                        "migrate-project",
                        "Migrate project",
                        enabled,
                        false,
                        None,
                        cx,
                        move |s, cx| s.dispatch(Command::Migrate(Some(source.clone())), cx),
                    ))
                    .into_any_element(),
                );
            }
            ProjectMigration::Blocked(reason) => notices.push(
                Notice::new(
                    "migration",
                    Tone::Warning,
                    format!("This project uses the earlier framework · {reason}"),
                )
                .into_any_element(),
            ),
        }
        notices
    }

    /// Source, checkpoint and job on one line; the other status lines when expanded.
    pub(super) fn render_status_strip(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let p = self.presentation.project.as_ref()?;
        let theme = self.theme;
        let expanded = status_strip_expanded(self.error.is_some(), self.status_open);
        let tone = if p.source == p.accepted {
            Tone::Success
        } else {
            Tone::Warning
        };
        let summary = format!(
            "Source {} · Checkpoint {} · Job {}",
            &p.source.as_str()[..12],
            &p.accepted.as_str()[..12],
            p.job
        );
        let line = |text: gpui::SharedString| {
            text_style(div(), TextStyle::Small)
                .text_color(theme.editor.text_2)
                .child(selectable(text))
        };
        // The summary heads a clickable row, so the full ids are repeated here to copy.
        let lines = div()
            .flex()
            .flex_col()
            .gap(px(METRICS.space_1))
            .pt(px(METRICS.space_1))
            .child(line(format!("Source {}", p.source.as_str()).into()))
            .child(line(format!("Checkpoint {}", p.accepted.as_str()).into()))
            .child(line(
                if p.source == p.accepted {
                    "Current source matches the saved checkpoint"
                } else {
                    "Source changed · saved checkpoint remains available"
                }
                .into(),
            ))
            .child(line(
                if p.history_available {
                    "Local history available"
                } else {
                    "Previous machine history unavailable · source baseline saved locally"
                }
                .into(),
            ))
            .child(line(
                if p.worker_available {
                    "Worker bridge available · build an immutable preview"
                } else {
                    "Worker bridge missing · source remains navigable"
                }
                .into(),
            ));
        let entity = cx.entity();
        Some(
            Card::editor(&self.theme)
                .id("status-strip")
                .flex_none()
                .px(px(METRICS.space_3))
                .py(px(METRICS.space_2))
                .child(
                    CollapsibleSection::new("status-details", expanded)
                        .header(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(METRICS.space_2))
                                .child(StatusDot::new(tone))
                                .child(
                                    text_style(div(), TextStyle::Small)
                                        .text_color(theme.editor.text_2)
                                        .child(summary),
                                ),
                        )
                        .content(lines)
                        .on_toggle(move |open, _, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.status_open = *open;
                                cx.notify();
                            });
                        }),
                )
                .into_any_element(),
        )
    }
}

/// The status strip opens when asked to and on any error. A running job only shows in the
/// one-line summary: the detail lines say nothing about it.
fn status_strip_expanded(has_error: bool, user_open: bool) -> bool {
    user_open || has_error
}

/// First line as the notice title; the full text as details when there is more.
fn error_title_details(text: &str) -> (String, Option<String>) {
    let title = text.lines().find(|l| !l.trim().is_empty()).unwrap_or(text);
    let details = (text.trim() != title.trim()).then(|| text.to_owned());
    (title.trim().to_owned(), details)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_strip_expands_only_when_asked_or_on_errors() {
        assert!(!status_strip_expanded(false, false));
        assert!(status_strip_expanded(true, false));
        assert!(status_strip_expanded(false, true));
    }

    #[test]
    fn error_notice_splits_title_and_details() {
        assert_eq!(error_title_details("boom"), ("boom".into(), None));
        let (title, details) = error_title_details("boom\nmore context");
        assert_eq!(title, "boom");
        assert_eq!(details.as_deref(), Some("boom\nmore context"));
    }
}
