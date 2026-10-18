//! The export panel: container and codec, size, frame rate, the expected file size, the
//! progress of a running export and its Cancel. It is shown in the window in place of the
//! editor, like the settings frame.

use std::rc::Rc;

use fframes_studio_protocol::export::ExportSettings;
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::FluentBuilder, px,
};
use studio_engine::export_options::{
    SCALES, estimate_bytes, format_label, format_size, fps_options, output_size,
};
use studio_theme::Theme;

use super::progress::{ExportPhase, LinearProgress};
use super::settings::ExportSupport;
use crate::kit::{
    Button, ButtonSize, ButtonVariant, Card, Notice, SegmentOption, SegmentedControl, SettingRow,
    Tone,
};

type Change = Rc<dyn Fn(ExportSettings, &mut Window, &mut App)>;
type Click = Rc<dyn Fn(&mut Window, &mut App)>;

/// What the panel shows. `settings` is what an export would run with now.
pub struct ExportPanelModel {
    pub support: ExportSupport,
    pub settings: ExportSettings,
    pub phase: ExportPhase,
    /// The length of the exported video, with edited timing applied.
    pub seconds: Option<f64>,
}

#[derive(IntoElement)]
pub struct ExportPanel {
    theme: Theme,
    model: ExportPanelModel,
    on_change: Option<Change>,
    on_start: Option<Click>,
    on_cancel: Option<Click>,
    on_back: Option<Click>,
}

impl ExportPanel {
    pub fn new(theme: &Theme, model: ExportPanelModel) -> Self {
        Self {
            theme: *theme,
            model,
            on_change: None,
            on_start: None,
            on_cancel: None,
            on_back: None,
        }
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(ExportSettings, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    pub fn on_start(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_start = Some(Rc::new(handler));
        self
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }

    pub fn on_back(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_back = Some(Rc::new(handler));
        self
    }

    fn format_row(&self, locked: bool) -> AnyElement {
        let theme = &self.theme;
        let settings = self.model.settings;
        let formats = self.model.support.formats();
        let options = formats
            .iter()
            .map(|format| {
                SegmentOption::new(
                    format_label(*format),
                    format.container == settings.container && format.codec == settings.codec,
                )
                .disabled(locked)
            })
            .collect();
        let on_change = self.on_change.clone();
        let control = SegmentedControl::settings(theme, "export-format", options).on_select(
            move |index, window, cx| {
                if let (Some(format), Some(change)) = (formats.get(*index), &on_change) {
                    change(
                        ExportSettings {
                            container: format.container,
                            codec: format.codec,
                            ..settings
                        },
                        window,
                        cx,
                    );
                }
            },
        );
        SettingRow::settings(
            theme,
            "Format",
            Some("The container and the codec the video is encoded with."),
            control.into_any_element(),
        )
        .into_any_element()
    }

    fn size_row(&self, locked: bool) -> AnyElement {
        let theme = &self.theme;
        let settings = self.model.settings;
        let capabilities = self.model.support.capabilities().cloned();
        let options = SCALES
            .iter()
            .map(|scale| {
                let percent = (scale * 100.).round() as u32;
                let label = match &capabilities {
                    Some(caps) => {
                        let (width, height) = output_size(caps.width, caps.height, *scale);
                        format!("{percent}% · {width}×{height}")
                    }
                    None => format!("{percent}%"),
                };
                SegmentOption::new(label, settings.scale == *scale).disabled(locked)
            })
            .collect();
        let on_change = self.on_change.clone();
        let control = SegmentedControl::settings(theme, "export-scale", options).on_select(
            move |index, window, cx| {
                if let (Some(scale), Some(change)) = (SCALES.get(*index), &on_change) {
                    change(
                        ExportSettings {
                            scale: *scale,
                            ..settings
                        },
                        window,
                        cx,
                    );
                }
            },
        );
        SettingRow::settings(
            theme,
            "Size",
            Some("Half size renders faster and makes a smaller file."),
            control.into_any_element(),
        )
        .into_any_element()
    }

    fn fps_row(&self, locked: bool) -> AnyElement {
        let theme = &self.theme;
        let settings = self.model.settings;
        let video_fps = self.model.support.capabilities().map(|caps| caps.fps);
        let choices = video_fps.map(fps_options).unwrap_or_else(|| vec![None]);
        let options = choices
            .iter()
            .map(|fps| {
                let label = match (fps, video_fps) {
                    (Some(fps), _) => format!("{fps} fps"),
                    (None, Some(video)) => format!("Video · {video} fps"),
                    (None, None) => "Video".to_owned(),
                };
                SegmentOption::new(label, settings.fps == *fps).disabled(locked)
            })
            .collect();
        let on_change = self.on_change.clone();
        let control = SegmentedControl::settings(theme, "export-fps", options).on_select(
            move |index, window, cx| {
                if let (Some(fps), Some(change)) = (choices.get(*index), &on_change) {
                    change(
                        ExportSettings {
                            fps: *fps,
                            ..settings
                        },
                        window,
                        cx,
                    );
                }
            },
        );
        SettingRow::settings(
            theme,
            "Frame rate",
            Some("A lower rate must divide the video's."),
            control.into_any_element(),
        )
        .into_any_element()
    }

    fn estimate_row(&self) -> AnyElement {
        let theme = &self.theme;
        let text: SharedString = match (self.model.support.capabilities(), self.model.seconds) {
            (Some(capabilities), Some(seconds)) => format!(
                "About {} (estimate) · {seconds:.1} s",
                format_size(estimate_bytes(&self.model.settings, capabilities, seconds))
            )
            .into(),
            _ => "Not known yet".into(),
        };
        SettingRow::settings(
            theme,
            "File size",
            Some("From the encoder's bitrate; the real file differs with the content."),
            div()
                .id("export-estimate")
                .text_color(theme.settings_text())
                .child(text)
                .into_any_element(),
        )
        .into_any_element()
    }
}

impl RenderOnce for ExportPanel {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let theme = self.theme;
        let phase = self.model.phase.clone();
        let busy = phase.is_busy();
        let editable = self.model.support.is_editable() && !busy;
        let can_start = self.model.support.can_start() && !busy;
        let rows = vec![
            self.format_row(!editable),
            self.size_row(!editable),
            self.fps_row(!editable),
            self.estimate_row(),
        ];

        let notice = match &self.model.support {
            ExportSupport::Probing | ExportSupport::Unknown => Some(
                Notice::new(
                    "export-probing",
                    Tone::Info,
                    "Checking what this project's renderer can export",
                )
                .into_any_element(),
            ),
            ExportSupport::Unavailable(reason) => Some(
                Notice::new(
                    "export-unavailable",
                    Tone::Warning,
                    format!("Export options unavailable · {reason}"),
                )
                .into_any_element(),
            ),
            ExportSupport::Ready(_) => None,
        };

        let status = phase.message().map(|message| {
            let tone = match &phase {
                ExportPhase::Failed(_) => theme.settings_text(),
                _ => theme.settings_muted(),
            };
            div()
                .id("export-message")
                .text_color(tone)
                .child(SharedString::from(message))
        });

        let on_start = self.on_start.clone();
        let on_cancel = self.on_cancel.clone();
        let on_back = self.on_back.clone();
        let actions = div()
            .flex()
            .gap(px(8.))
            .child(
                Button::settings(&theme, "export-back", ButtonVariant::Gray, ButtonSize::Sm)
                    .label("Back")
                    .on_click(move |_, window, cx| {
                        if let Some(back) = &on_back {
                            back(window, cx);
                        }
                    }),
            )
            .child(if busy {
                Button::settings(&theme, "export-cancel", ButtonVariant::Gray, ButtonSize::Sm)
                    .label("Cancel export")
                    .on_click(move |_, window, cx| {
                        if let Some(cancel) = &on_cancel {
                            cancel(window, cx);
                        }
                    })
            } else {
                Button::settings(&theme, "export-start", ButtonVariant::Blue, ButtonSize::Sm)
                    .label("Export…")
                    .disabled(!can_start)
                    .on_click(move |_, window, cx| {
                        if let Some(start) = &on_start {
                            start(window, cx);
                        }
                    })
            });

        Card::editor(&theme)
            .id("export-panel")
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .p(px(20.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .text_color(theme.settings_text())
                    .text_size(px(16.))
                    .child("Export"),
            )
            .children(notice)
            .child(Card::settings_rows(&theme, rows))
            .when(phase.shows_progress(), |panel| {
                panel.child(LinearProgress::new(
                    &theme,
                    phase.fraction().unwrap_or_default(),
                ))
            })
            .children(status)
            .child(actions)
    }
}
