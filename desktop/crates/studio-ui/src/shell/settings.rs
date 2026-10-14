//! The settings frame, shown in the window in place of the editor (Cap opens a second
//! window). The header's Settings button opens it and its Back row returns to the editor.
//! The Appearance page picks the theme and saves it with the app's settings.

use std::sync::atomic::{AtomicU64, Ordering};

use gpui::{Context, IntoElement, ParentElement, Styled};
use studio_engine::app_settings::AppSettings;
use studio_proto::AppTheme;

use super::StudioShell;
use super::sections::box_probe;
use crate::kit::{
    Card, Icon, Section, SegmentOption, SegmentedControl, SettingRow, SettingsFrame,
    SettingsNavItem,
};

/// The choices in the order the control shows them; option `i` is `theme-i`.
const THEMES: [(AppTheme, &str); 3] = [
    (AppTheme::System, "System"),
    (AppTheme::Light, "Light"),
    (AppTheme::Dark, "Dark"),
];

/// Saves the theme choices one at a time, in the order they were made: a save that a
/// later choice overtook is skipped.
#[derive(Default)]
pub(super) struct SettingsWriter {
    latest: AtomicU64,
    write: parking_lot::Mutex<()>,
}

impl StudioShell {
    /// Shows the settings frame. The editor is out of view while it shows, so playback
    /// stops and a timeline resize in progress ends.
    pub(super) fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.resume_install = false;
        self.pause_at_painted_frame(cx);
        self.timeline_resize = None;
        self.overflow_open = false;
        self.settings_open = true;
        cx.notify();
    }

    pub(super) fn close_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = false;
        cx.notify();
    }

    /// Applies `choice` to every window and saves it in the background.
    fn choose_theme(&mut self, choice: AppTheme, cx: &mut Context<Self>) {
        if studio_theme::current_preference(cx) == choice {
            return;
        }
        // Refreshes every window, which renders the cached sections again too.
        studio_theme::set_preference(choice, cx);
        let Some(paths) = self.presentation.paths.clone() else {
            self.error = Some("Cannot save the theme: the app's storage is unavailable".into());
            return;
        };
        let writer = self.settings_writer.clone();
        let generation = writer.latest.fetch_add(1, Ordering::AcqRel) + 1;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { save_theme(&writer, generation, choice, &paths) })
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

    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let current = studio_theme::current_preference(cx);
        let entity = cx.entity();
        let options = THEMES
            .iter()
            .map(|(value, label)| SegmentOption::new(*label, *value == current))
            .collect();
        let mut control = SegmentedControl::settings(&theme, "theme", options).on_select({
            let entity = entity.clone();
            move |index, _, cx| {
                if let Some((choice, _)) = THEMES.get(*index) {
                    entity.update(cx, |shell, cx| shell.choose_theme(*choice, cx));
                }
            }
        });
        if self.qualifying {
            let shell = entity.clone();
            control = control.on_option_bounds(move |index, bounds, cx| {
                shell.update(cx, |shell, _| {
                    shell.button_bounds.insert(
                        format!("theme-{index}"),
                        crate::probe::bounds_to_record(bounds),
                    );
                });
            });
        }
        let appearance = Section::settings(
            &theme,
            "Appearance",
            Some("Follow the system's light or dark appearance, or pick one."),
            None,
            vec![
                Card::settings_rows(
                    &theme,
                    vec![
                        SettingRow::settings(&theme, "Theme", None, control.into_any_element())
                            .into_any_element(),
                    ],
                )
                .into_any_element(),
            ],
        );
        let mut frame = SettingsFrame::settings(
            &theme,
            "settings",
            vec![SettingsNavItem::new(
                "settings-appearance",
                "Appearance",
                Icon::Palette,
                true,
            )],
        )
        .on_back({
            let entity = entity.clone();
            move |_, cx| entity.update(cx, |shell, cx| shell.close_settings(cx))
        })
        .children([appearance.into_any_element()]);
        if let Some(probe) = self.bounds_probe(&gpui::ElementId::Name("settings-back".into()), cx) {
            frame = frame.on_back_bounds(probe.into_bounds_handler());
        }
        Card::editor(&theme)
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(frame)
            .child(box_probe("settings", cx))
            .into_any_element()
    }
}

/// Writes `choice` into the app's settings unless a later choice was made meanwhile.
fn save_theme(
    writer: &SettingsWriter,
    generation: u64,
    choice: AppTheme,
    paths: &studio_engine::app_paths::AppPaths,
) -> Result<(), String> {
    let _write = writer.write.lock();
    if writer.latest.load(Ordering::Acquire) != generation {
        return Ok(());
    }
    let mut settings = AppSettings::load(paths);
    settings.theme = choice;
    settings.save(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_overtaken_by_a_later_choice_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let paths = studio_engine::app_paths::AppPaths::new(dir.path()).unwrap();
        let writer = SettingsWriter::default();
        writer.latest.store(2, Ordering::Release);
        save_theme(&writer, 2, AppTheme::Dark, &paths).unwrap();
        save_theme(&writer, 1, AppTheme::Light, &paths).unwrap();
        assert_eq!(AppSettings::load(&paths).theme, AppTheme::Dark);
    }

    #[test]
    fn the_options_are_system_light_and_dark() {
        let order: Vec<_> = THEMES.iter().map(|(theme, _)| *theme).collect();
        assert_eq!(order, [AppTheme::System, AppTheme::Light, AppTheme::Dark]);
    }
}
