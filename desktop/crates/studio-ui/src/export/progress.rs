//! Where an export is, as the panel shows it, and the linear bar that draws it.

use std::path::PathBuf;

use gpui::{
    IntoElement, ParentElement, RenderOnce, Styled, Window, div, prelude::FluentBuilder, px,
    relative,
};
use studio_engine::services::export_service::ExportProgress;
use studio_theme::Theme;

/// The stage of the export, folded from the engine's progress events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportPhase {
    /// No export ran since the project opened, or the last one was dismissed.
    Idle,
    /// Freezing the source, compiling the worker, starting the encoder.
    Preparing,
    Rendering {
        done: usize,
        total: usize,
    },
    /// The frames are written; the audio and the file are being checked.
    Finishing,
    Complete {
        destination: PathBuf,
    },
    Failed(String),
    Cancelled,
}

impl ExportPhase {
    pub fn of(status: Option<&ExportProgress>) -> Self {
        match status {
            None => Self::Idle,
            Some(ExportProgress::Started { .. } | ExportProgress::Warning(_)) => Self::Preparing,
            Some(ExportProgress::Rendering { done, total }) => Self::Rendering {
                done: *done,
                total: *total,
            },
            Some(ExportProgress::Audio | ExportProgress::Verifying) => Self::Finishing,
            Some(ExportProgress::Complete { destination, .. }) => Self::Complete {
                destination: destination.clone(),
            },
            Some(ExportProgress::Failed(reason)) => Self::Failed(reason.clone()),
            Some(ExportProgress::Cancelled) => Self::Cancelled,
        }
    }

    /// An export is running: settings are locked and Cancel is offered.
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::Rendering { .. } | Self::Finishing
        )
    }

    /// The bar has a position to show.
    pub fn shows_progress(&self) -> bool {
        matches!(self, Self::Rendering { .. } | Self::Finishing)
    }

    /// How far along the export is, in `0..=1`; `None` before there is a position.
    pub fn fraction(&self) -> Option<f32> {
        match self {
            Self::Rendering { done, total } if *total > 0 => {
                Some((*done as f32 / *total as f32).clamp(0., 1.))
            }
            Self::Rendering { .. } => Some(0.),
            Self::Finishing => Some(1.),
            _ => None,
        }
    }

    /// The line under the bar, or under the buttons once the export ended.
    pub fn message(&self) -> Option<String> {
        Some(match self {
            Self::Idle => return None,
            Self::Preparing => "Preparing the export".into(),
            Self::Rendering { done, total } => format!("Rendering · {done}/{total} frames"),
            Self::Finishing => "Writing and verifying the file".into(),
            Self::Complete { destination } => format!(
                "Exported {}",
                studio_bootstrap::external_path(destination).display()
            ),
            Self::Failed(reason) => format!("Export failed · {reason}"),
            Self::Cancelled => "Export cancelled · partial output removed".into(),
        })
    }
}

/// A thin horizontal bar: a track with a rounded fill up to `fraction`.
#[derive(IntoElement)]
pub struct LinearProgress {
    fraction: f32,
    track: gpui::Hsla,
    fill: gpui::Hsla,
}

impl LinearProgress {
    pub fn new(theme: &Theme, fraction: f32) -> Self {
        Self {
            fraction: fraction.clamp(0., 1.),
            track: theme.settings_border(),
            fill: gpui::rgb(Theme::SETTINGS_ACCENT).into(),
        }
    }
}

impl RenderOnce for LinearProgress {
    fn render(self, _: &mut Window, _: &mut gpui::App) -> impl IntoElement {
        div()
            .h(px(6.))
            .w_full()
            .rounded(px(3.))
            .overflow_hidden()
            .bg(self.track)
            .child(
                div()
                    .h_full()
                    .rounded(px(3.))
                    .bg(self.fill)
                    .when(self.fraction > 0., |bar| bar.w(relative(self.fraction))),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_fold_into_phases() {
        let started = ExportProgress::Started {
            revision: "r".into(),
            label: "l".into(),
        };
        assert_eq!(ExportPhase::of(None), ExportPhase::Idle);
        assert_eq!(ExportPhase::of(Some(&started)), ExportPhase::Preparing);
        assert_eq!(
            ExportPhase::of(Some(&ExportProgress::Warning("w".into()))),
            ExportPhase::Preparing
        );
        assert_eq!(
            ExportPhase::of(Some(&ExportProgress::Rendering { done: 3, total: 9 })),
            ExportPhase::Rendering { done: 3, total: 9 }
        );
        assert_eq!(
            ExportPhase::of(Some(&ExportProgress::Audio)),
            ExportPhase::Finishing
        );
        assert_eq!(
            ExportPhase::of(Some(&ExportProgress::Verifying)),
            ExportPhase::Finishing
        );
        assert_eq!(
            ExportPhase::of(Some(&ExportProgress::Cancelled)),
            ExportPhase::Cancelled
        );
    }

    #[test]
    fn only_a_running_export_is_busy() {
        let busy = [
            ExportPhase::Preparing,
            ExportPhase::Rendering { done: 0, total: 1 },
            ExportPhase::Finishing,
        ];
        assert!(busy.iter().all(ExportPhase::is_busy));
        let idle = [
            ExportPhase::Idle,
            ExportPhase::Complete {
                destination: "a.mp4".into(),
            },
            ExportPhase::Failed("x".into()),
            ExportPhase::Cancelled,
        ];
        assert!(!idle.iter().any(ExportPhase::is_busy));
    }

    #[test]
    fn the_bar_follows_the_frames() {
        assert_eq!(ExportPhase::Preparing.fraction(), None);
        assert!(!ExportPhase::Preparing.shows_progress());
        let half = ExportPhase::Rendering {
            done: 45,
            total: 90,
        };
        assert!(half.shows_progress());
        assert_eq!(half.fraction(), Some(0.5));
        assert_eq!(
            ExportPhase::Rendering { done: 5, total: 0 }.fraction(),
            Some(0.)
        );
        assert_eq!(ExportPhase::Finishing.fraction(), Some(1.));
    }

    #[test]
    fn a_failure_carries_its_reason() {
        let message = ExportPhase::Failed("no encoder".into()).message().unwrap();
        assert!(message.contains("no encoder"), "{message}");
        assert_eq!(ExportPhase::Idle.message(), None);
    }
}
