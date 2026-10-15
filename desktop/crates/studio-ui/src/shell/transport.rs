//! The transport bar under the player: playback, scrubbing readout, build and cancel.

use gpui::{Context, InteractiveElement, IntoElement, ParentElement, Styled, div, px};
use studio_engine::engine::backend::Command;
use studio_theme::{METRICS, TextStyle, text_style};

use super::StudioShell;
use crate::kit::{Icon, IconButtonSize, with_tooltip};

impl StudioShell {
    pub(super) fn render_transport(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        let enabled = !self.busy;
        let project = self.presentation.project.clone();
        let mode = transport_mode(self.player_width);
        let has_frames = self
            .displayed
            .as_ref()
            .is_some_and(|d| d.timeline.total_frames > 0);
        let playing = self.transport.playing() || self.resume_install;
        let readout = match &self.displayed {
            Some(d) => {
                let fps = d.timeline.fps as f64;
                let current = format_clock(self.preview_state.position() as f64 / fps);
                let text = if show_total_time(self.player_width) {
                    format!(
                        "{current} / {}",
                        format_clock(d.timeline.total_frames as f64 / fps)
                    )
                } else {
                    current
                };
                let tip = format!(
                    "Displayed {} · frame {} / {} · {:.3}s{}",
                    &d.identity().source_revision[..12.min(d.identity().source_revision.len())],
                    self.preview_state.position(),
                    d.timeline.total_frames,
                    self.preview_state.position() as f64 / fps,
                    if project
                        .as_ref()
                        .is_some_and(|p| p.source.as_str() != d.identity().source_revision)
                        && !self
                            .presentation
                            .state
                            .as_ref()
                            .is_some_and(|s| s.is_current_build(d.tag()))
                    {
                        " · prior revision"
                    } else {
                        ""
                    }
                );
                (text, Some(tip))
            }
            None => ("0:00.00 / 0:00.00".to_owned(), None),
        };
        let readout_style = if mode == TransportMode::Compact {
            TextStyle::Label
        } else {
            TextStyle::Body
        };
        let can_build = enabled && project.as_ref().is_some_and(|p| p.worker_available);
        let can_cancel = self.preview.is_some();
        let (build, cancel) = match mode {
            TransportMode::Full => (
                self.action_button(
                    "build-preview",
                    "Build preview",
                    can_build,
                    true,
                    Some(Icon::Hammer),
                    cx,
                    |s, cx| s.dispatch(Command::Build, cx),
                )
                .into_any_element(),
                self.action_button(
                    "cancel-preview",
                    "Cancel build",
                    can_cancel,
                    false,
                    Some(Icon::Square),
                    cx,
                    |s, cx| s.dispatch(Command::CancelBuild, cx),
                )
                .into_any_element(),
            ),
            TransportMode::Compact => (
                self.action_icon_button(
                    "build-preview",
                    "Build preview",
                    Icon::Hammer,
                    IconButtonSize::Medium,
                    can_build,
                    cx,
                    |s, cx| s.dispatch(Command::Build, cx),
                )
                .accent(&self.theme)
                .into_any_element(),
                self.action_icon_button(
                    "cancel-preview",
                    "Cancel build",
                    Icon::Square,
                    IconButtonSize::Medium,
                    can_cancel,
                    cx,
                    |s, cx| s.dispatch(Command::CancelBuild, cx),
                )
                .ctl_fill(&self.theme)
                .into_any_element(),
            ),
        };
        let overflow = self.overflow_trigger(cx);
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(METRICS.space_2))
            .h(px(METRICS.transport_h))
            .px(px(METRICS.space_3))
            .border_t_1()
            .border_color(palette.line)
            .overflow_hidden()
            .child(with_tooltip(
                text_style(div().id("time-readout"), readout_style)
                    .flex_none()
                    .min_w(px(if mode == TransportMode::Compact {
                        60.
                    } else {
                        120.
                    }))
                    .text_color(palette.text_1)
                    .child(readout.0),
                &theme,
                readout.1.map(Into::into),
            ))
            .child(div().flex_1())
            .child(self.action_icon_button(
                "previous-frame",
                "Previous frame",
                Icon::StepBack,
                IconButtonSize::Small,
                has_frames,
                cx,
                |s, cx| s.step_preview(-1, cx),
            ))
            .child(self.action_icon_button(
                "play-pause",
                if playing { "Pause" } else { "Play" },
                if playing { Icon::Pause } else { Icon::Play },
                IconButtonSize::Large,
                has_frames,
                cx,
                |s, cx| s.toggle_playback(cx),
            ))
            .child(self.action_icon_button(
                "next-frame",
                "Next frame",
                Icon::StepForward,
                IconButtonSize::Small,
                has_frames,
                cx,
                |s, cx| s.step_preview(1, cx),
            ))
            .child(self.action_icon_button(
                "mute-audio",
                if self.muted { "Unmute (M)" } else { "Mute (M)" },
                if self.muted {
                    Icon::VolumeX
                } else {
                    Icon::Volume2
                },
                IconButtonSize::Medium,
                self.displayed.is_some(),
                cx,
                |s, cx| s.toggle_mute(cx),
            ))
            .child(div().flex_1())
            .child(build)
            .child(cancel)
            .child(overflow)
            .into_any_element()
    }
}

/// Player width under which the transport uses icon-only Build and Cancel.
const TRANSPORT_COMPACT_BELOW: f32 = 760.;

/// Player width under which the time readout drops the total.
const TRANSPORT_TOTAL_HIDDEN_BELOW: f32 = 620.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransportMode {
    Full,
    Compact,
}

pub(super) fn transport_mode(player_width: f32) -> TransportMode {
    if player_width < TRANSPORT_COMPACT_BELOW {
        TransportMode::Compact
    } else {
        TransportMode::Full
    }
}

pub(super) fn show_total_time(player_width: f32) -> bool {
    player_width >= TRANSPORT_TOTAL_HIDDEN_BELOW
}

/// `m:ss.cc`, truncated to centiseconds.
fn format_clock(seconds: f64) -> String {
    let centis = (seconds.max(0.) * 100.).floor() as u64;
    format!(
        "{}:{:02}.{:02}",
        centis / 6000,
        centis / 100 % 60,
        centis % 100
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_mode_switches_below_760() {
        assert_eq!(transport_mode(1200.), TransportMode::Full);
        assert_eq!(transport_mode(760.), TransportMode::Full);
        assert_eq!(transport_mode(759.9), TransportMode::Compact);
        assert!(show_total_time(620.));
        assert!(!show_total_time(619.));
    }

    #[test]
    fn clock_formats_minutes_seconds_centiseconds() {
        assert_eq!(format_clock(0.), "0:00.00");
        assert_eq!(format_clock(1.2), "0:01.20");
        assert_eq!(format_clock(5.5), "0:05.50");
        assert_eq!(format_clock(125.999), "2:05.99");
        assert_eq!(format_clock(-3.), "0:00.00");
    }
}
