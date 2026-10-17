//! What the export panel may offer for the open project: the answer of its worker, or the
//! reason there is none.

use fframes_studio_protocol::export::{ExportCapabilities, ExportFormat, ExportSettings};
use studio_engine::export_options::{FORMATS, available_formats, usable_settings};

/// What is known about how the open project can be exported.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ExportSupport {
    /// Nothing asked yet.
    #[default]
    Unknown,
    /// The worker is being asked.
    Probing,
    /// The worker answered: its formats and the video's size and frame rate.
    Ready(Box<ExportCapabilities>),
    /// The worker could not be asked; the export itself reports why it cannot run.
    Unavailable(String),
}

impl ExportSupport {
    /// The worker's answer, when there is one.
    pub fn capabilities(&self) -> Option<&ExportCapabilities> {
        match self {
            Self::Ready(capabilities) => Some(capabilities),
            _ => None,
        }
    }

    /// The panel's controls change the export.
    pub fn is_editable(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// An export may start: the options are known, or the worker could not be asked.
    pub fn can_start(&self) -> bool {
        !matches!(self, Self::Unknown | Self::Probing)
    }

    /// The formats to offer: the worker's, or only the default one without an answer.
    pub fn formats(&self) -> Vec<ExportFormat> {
        match self {
            Self::Ready(capabilities) => available_formats(capabilities),
            _ => vec![FORMATS[0]],
        }
    }

    /// What an export runs with when `saved` is the user's choice.
    pub fn settings_for(&self, saved: ExportSettings) -> ExportSettings {
        match self {
            Self::Ready(capabilities) => usable_settings(saved, capabilities),
            _ => ExportSettings::default(),
        }
    }

    /// A short name for telemetry.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Probing => "probing",
            Self::Ready(_) => "ready",
            Self::Unavailable(_) => "unavailable",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fframes_studio_protocol::export::{ExportCodec, ExportContainer};

    fn ready(formats: &[ExportFormat]) -> ExportSupport {
        ExportSupport::Ready(Box::new(ExportCapabilities {
            version: 1,
            width: 1920,
            height: 1080,
            fps: 60,
            timing: true,
            formats: formats.to_vec(),
        }))
    }

    fn webm() -> ExportSettings {
        ExportSettings {
            container: ExportContainer::Webm,
            codec: ExportCodec::Vp9,
            scale: 0.5,
            fps: Some(30),
        }
    }

    #[test]
    fn an_export_starts_once_the_options_are_known() {
        assert!(!ExportSupport::Unknown.can_start());
        assert!(!ExportSupport::Probing.can_start());
        assert!(ExportSupport::Unavailable("x".into()).can_start());
        assert!(ready(&FORMATS).can_start());
    }

    #[test]
    fn only_a_worker_that_answered_lets_the_settings_change() {
        assert!(ready(&FORMATS).is_editable());
        assert!(!ExportSupport::Unavailable("x".into()).is_editable());
        assert!(!ExportSupport::Probing.is_editable());
    }

    #[test]
    fn without_an_answer_only_the_default_is_exported() {
        let unavailable = ExportSupport::Unavailable("x".into());
        assert_eq!(unavailable.formats(), [FORMATS[0]]);
        assert_eq!(unavailable.settings_for(webm()), ExportSettings::default());
    }

    #[test]
    fn a_saved_choice_the_worker_lacks_falls_back_to_the_default() {
        assert_eq!(ready(&FORMATS).settings_for(webm()), webm());
        assert_eq!(
            ready(&[FORMATS[0]]).settings_for(webm()),
            ExportSettings::default()
        );
    }
}
