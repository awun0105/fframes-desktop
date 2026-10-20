//! The export panel in the shell: opening it, asking the open project's worker what it can
//! export, keeping the user's choice and starting the export with it.

use std::sync::atomic::Ordering;

use fframes_studio_protocol::export::ExportSettings;
use gpui::{Context, IntoElement};
use studio_engine::app_paths::AppPaths;
use studio_engine::app_settings::AppSettings;

use super::StudioShell;
use crate::export::panel::{ExportPanel, ExportPanelModel};
use crate::export::progress::ExportPhase;
use crate::export::settings::ExportSupport;

/// The export panel's state in the shell.
#[derive(Default)]
pub(super) struct ExportPanelState {
    pub open: bool,
    /// The saved choice, read when the panel first opens.
    pub saved: Option<ExportSettings>,
    pub support: ExportSupport,
    /// The session and accepted revision `support` answers for.
    probe_key: Option<(u64, String)>,
}

impl StudioShell {
    /// An export needs an open project and an installed SDK, like the header action.
    fn can_export(&self) -> bool {
        self.presentation.project.is_some()
            && self.presentation.session.is_some()
            && self.presentation.agent_sdk.is_some()
    }

    /// Shows the export panel in place of the editor, and asks the project's worker what it
    /// can export. The editor is out of view while it shows, so playback stops.
    pub(super) fn open_export(&mut self, cx: &mut Context<Self>) {
        if !self.can_export() {
            return;
        }
        self.pause_at_painted_frame(cx);
        self.resize = None;
        self.overflow_open = false;
        self.settings_open = false;
        self.export_panel.open = true;
        if self.export_panel.saved.is_none() {
            self.export_panel.saved = Some(
                self.presentation
                    .paths
                    .as_ref()
                    .map(AppSettings::load)
                    .unwrap_or_default()
                    .export,
            );
        }
        self.sync_export_support(cx);
        cx.notify();
    }

    pub(super) fn close_export(&mut self, cx: &mut Context<Self>) {
        self.export_panel.open = false;
        cx.notify();
    }

    /// What an export would run with now.
    pub(super) fn export_settings(&self) -> ExportSettings {
        self.export_panel
            .support
            .settings_for(self.export_panel.saved.unwrap_or_default())
    }

    /// Asks for the destination of an export with the panel's settings, once the options
    /// are known and no export runs.
    pub(super) fn request_export(&mut self, cx: &mut Context<Self>) {
        if !self.can_export()
            || self.export_cancelled.is_some()
            || !self.export_panel.support.can_start()
        {
            return;
        }
        self.pick(super::Picker::Export, cx);
    }

    /// Brings the answer of the worker up to date with the accepted revision: the worker is
    /// asked once per revision (an unchanged project reuses the preview's build).
    pub(super) fn sync_export_support(&mut self, cx: &mut Context<Self>) {
        if !self.export_panel.open {
            return;
        }
        // The project closed or the SDK went away under the panel.
        if !self.can_export() {
            self.export_panel.open = false;
            self.export_panel.probe_key = None;
            cx.notify();
            return;
        }
        let Some(project) = &self.presentation.project else {
            return;
        };
        let Some(session) = self.presentation.session else {
            return;
        };
        let key = (session.0, project.accepted.as_str().to_owned());
        if self.export_panel.probe_key.as_ref() == Some(&key) {
            return;
        }
        self.export_panel.probe_key = Some(key.clone());
        let Some(sdk) = self
            .presentation
            .agent_sdk
            .as_ref()
            .map(|sdk| sdk.dir.clone())
        else {
            self.export_panel.support =
                ExportSupport::Unavailable("select a compatible installed SDK first".into());
            return;
        };
        self.export_panel.support = ExportSupport::Probing;
        let answer = match self
            .engine
            .probe_export_capabilities(session, sdk, self.builds.clone())
        {
            Ok(answer) => answer,
            Err(error) => {
                self.export_panel.support = ExportSupport::Unavailable(error);
                return;
            }
        };
        cx.spawn(async move |this, cx| {
            let result = answer.recv().await;
            let _ = this.update(cx, |shell, cx| {
                if shell.closed.load(Ordering::Acquire)
                    || shell.export_panel.probe_key.as_ref() != Some(&key)
                {
                    return;
                }
                shell.export_panel.support = match result {
                    Ok(Ok(capabilities)) => ExportSupport::Ready(Box::new(capabilities)),
                    Ok(Err(reason)) => ExportSupport::Unavailable(reason),
                    Err(_) => ExportSupport::Unavailable("the project closed".into()),
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// Keeps `settings` as the choice for every project and saves it in the background.
    fn choose_export_settings(&mut self, settings: ExportSettings, cx: &mut Context<Self>) {
        if self.export_panel.saved == Some(settings) {
            return;
        }
        self.export_panel.saved = Some(settings);
        cx.notify();
        let Some(paths) = self.presentation.paths.clone() else {
            self.error =
                Some("Cannot save the export choice: the app's storage is unavailable".into());
            return;
        };
        let writer = self.settings_writer.clone();
        let generation = writer.next_export_generation();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { save_export(&writer, generation, settings, &paths) })
                .await;
            if let Err(error) = result {
                let _ = this.update(cx, |shell, cx| {
                    shell.error = Some(error);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn render_export(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let entity = cx.entity();
        let model = ExportPanelModel {
            support: self.export_panel.support.clone(),
            settings: self.export_settings(),
            phase: ExportPhase::of(self.export_status.as_ref()),
            seconds: self.timeline.read(cx).duration_seconds(),
        };
        ExportPanel::new(&self.theme, model)
            .on_change({
                let entity = entity.clone();
                move |settings, _, cx| {
                    entity.update(cx, |shell, cx| shell.choose_export_settings(settings, cx));
                }
            })
            .on_start({
                let entity = entity.clone();
                move |_, cx| entity.update(cx, |shell, cx| shell.request_export(cx))
            })
            .on_cancel({
                let entity = entity.clone();
                move |_, cx| entity.update(cx, |shell, cx| shell.cancel_export(cx))
            })
            .on_back({
                let entity = entity.clone();
                move |_, cx| entity.update(cx, |shell, cx| shell.close_export(cx))
            })
            .into_any_element()
    }
}

/// Writes `settings` into the app's settings unless a later choice was made meanwhile.
fn save_export(
    writer: &super::settings::SettingsWriter,
    generation: u64,
    settings: ExportSettings,
    paths: &AppPaths,
) -> Result<(), String> {
    writer.save_if_latest_export(generation, || {
        let mut saved = AppSettings::load(paths);
        saved.export = settings;
        saved.save(paths)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_overtaken_by_a_later_choice_is_skipped_and_the_theme_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path()).unwrap();
        let writer = super::super::settings::SettingsWriter::default();
        let first = writer.next_export_generation();
        let second = writer.next_export_generation();
        let half = ExportSettings {
            scale: 0.5,
            ..ExportSettings::default()
        };
        AppSettings {
            theme: studio_proto::AppTheme::Dark,
            ..AppSettings::default()
        }
        .save(&paths)
        .unwrap();
        save_export(&writer, second, half, &paths).unwrap();
        save_export(&writer, first, ExportSettings::default(), &paths).unwrap();
        let loaded = AppSettings::load(&paths);
        assert_eq!(loaded.export, half);
        assert_eq!(loaded.theme, studio_proto::AppTheme::Dark);
    }
}
