//! The shell's keymap and action handlers: every studio action dispatched to the window
//! lands here.
//!
//! Key contexts, most specific first: `TextInput` (a focused field, bound by the kit), then
//! `Player` (the preview surface) and `Timeline` (its tracks), then `Workspace` (the window).
//! Plain keys are bound only in `Player` and `Timeline`, which hold no text field, so typing
//! in a field never reaches them; `Workspace` has no bindings of its own.

use gpui::{App, Context, InteractiveElement, KeyBinding};
use studio_engine::engine::backend::Command;
use studio_engine::presets::PresetCommand;
use studio_engine::services::audio_service::OutputDevice;

use super::left_column::LeftTab;
use super::{Picker, StudioShell};
use crate::actions;

/// The preview surface's key context.
pub(super) const PLAYER: &str = "Player";
/// The timeline tracks' key context.
const TIMELINE: &str = crate::timeline::KEY_CONTEXT;
/// The window's key context, around every other one.
pub(super) const WORKSPACE: &str = "Workspace";

/// Binds the studio window's keys: the text fields' editing keys, then the player's and
/// the timeline's. Call once at startup, before the window opens.
pub fn bind_keys(cx: &mut App) {
    crate::kit::bind_text_input_keys(cx);
    crate::kit::bind_selectable_text_keys(cx);
    cx.bind_keys(key_bindings());
}

fn key_bindings() -> Vec<KeyBinding> {
    let player = Some(PLAYER);
    let timeline = Some(TIMELINE);
    vec![
        KeyBinding::new("space", actions::TogglePlayback, player),
        KeyBinding::new("left", actions::StepBackward, player),
        KeyBinding::new("right", actions::StepForward, player),
        KeyBinding::new("home", actions::SeekToStart, player),
        KeyBinding::new("end", actions::SeekToEnd, player),
        KeyBinding::new("c", actions::CycleCanvasSelection, player),
        KeyBinding::new("0", actions::ResetCanvasView, player),
        KeyBinding::new("escape", actions::ClearCanvasSelection, player),
        KeyBinding::new("space", actions::TogglePlayback, timeline),
        KeyBinding::new("m", actions::ToggleMute, timeline),
        KeyBinding::new("left", actions::StepBackward, timeline),
        KeyBinding::new("right", actions::StepForward, timeline),
        KeyBinding::new("home", actions::SeekToStart, timeline),
        KeyBinding::new("end", actions::SeekToEnd, timeline),
        KeyBinding::new("+", actions::ZoomIn, timeline),
        KeyBinding::new("=", actions::ZoomIn, timeline),
        KeyBinding::new("shift-=", actions::ZoomIn, timeline),
        KeyBinding::new("-", actions::ZoomOut, timeline),
        KeyBinding::new("pageup", actions::PageTimelineBack, timeline),
        KeyBinding::new("pagedown", actions::PageTimelineForward, timeline),
        KeyBinding::new("s", actions::CycleScene, timeline),
        KeyBinding::new("[", actions::MarkRangeStart, timeline),
        KeyBinding::new("]", actions::MarkRangeEnd, timeline),
    ]
}

impl StudioShell {
    /// Every studio action, handled at the root so it reaches the shell from anywhere in
    /// the window.
    pub(super) fn on_actions(root: gpui::Div, cx: &mut Context<Self>) -> gpui::Div {
        root.on_action(cx.listener(|s, _: &actions::OpenProject, _, cx| s.pick(Picker::Open, cx)))
            .on_action(
                cx.listener(|s, _: &actions::BuildPreview, _, cx| s.dispatch(Command::Build, cx)),
            )
            .on_action(cx.listener(|s, _: &actions::CancelPreview, _, cx| {
                s.dispatch(Command::CancelBuild, cx)
            }))
            .on_action(cx.listener(|s, _: &actions::ExportVideo, _, cx| s.pick(Picker::Export, cx)))
            .on_action(cx.listener(|s, _: &actions::CancelExport, _, cx| s.cancel_export(cx)))
            .on_action(cx.listener(|s, action: &actions::ApplyPreset, _, cx| {
                let key = action.key.clone();
                s.run_preset(PresetCommand::Apply { key }, cx)
            }))
            .on_action(cx.listener(|s, action: &actions::ResetPreset, _, cx| {
                let key = action.key.clone();
                s.run_preset(PresetCommand::Reset { key }, cx)
            }))
            .on_action(cx.listener(|s, action: &actions::SetStyleOverride, _, cx| {
                let (token, value) = (action.token.clone(), action.value.clone());
                s.run_preset(PresetCommand::SetOverride { token, value }, cx)
            }))
            .on_action(cx.listener(|s, _: &actions::SaveCheckpoint, _, cx| {
                s.dispatch(Command::Checkpoint, cx)
            }))
            .on_action(
                cx.listener(|s, _: &actions::RestoreCheckpoint, _, cx| s.pick(Picker::Restore, cx)),
            )
            .on_action(cx.listener(|s, _: &actions::SelectSdk, _, cx| s.pick(Picker::Sdk, cx)))
            .on_action(
                cx.listener(|s, _: &actions::ImportSdkBundle, _, cx| s.pick(Picker::SdkBundle, cx)),
            )
            .on_action(cx.listener(|s, _: &actions::DetectAgents, _, cx| {
                s.panel.update(cx, |panel, cx| panel.detect_agents(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::OpenAgentSetup, _, cx| s.open_agent_setup(cx)))
            .on_action(cx.listener(|s, _: &actions::FocusPrompt, window, cx| {
                if s.left_tab != LeftTab::Agent {
                    s.left_tab = LeftTab::Agent;
                    cx.notify();
                }
                s.panel
                    .update(cx, |panel, cx| panel.focus_prompt(window, cx))
            }))
            .on_action(cx.listener(|s, _: &actions::FocusPreview, window, cx| {
                window.focus(&s.preview_focus, cx)
            }))
            .on_action(cx.listener(|s, _: &actions::FocusTimeline, window, cx| {
                s.timeline.update(cx, |t, cx| t.focus(window, cx))
            }))
            .on_action(cx.listener(|s, _: &actions::SubmitPrompt, _, cx| {
                s.panel.update(cx, |panel, cx| panel.submit(cx))
            }))
            .on_action(cx.listener(|s, action: &actions::AnswerPermission, _, cx| {
                let option = action.option.clone();
                s.panel
                    .update(cx, |panel, cx| panel.answer_oldest_permission(option, cx))
            }))
            .on_action(cx.listener(|s, _: &actions::ApplyCandidate, _, cx| {
                s.panel.update(cx, |panel, cx| panel.apply(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::ToggleReviewPolicy, _, cx| {
                s.panel
                    .update(cx, |panel, cx| panel.toggle_review_policy(cx))
            }))
            .on_action(cx.listener(|s, action: &actions::UseDetectedAgent, _, cx| {
                s.panel
                    .update(cx, |panel, cx| panel.use_detected(&action.profile, cx))
            }))
            .on_action(cx.listener(|s, action: &actions::SelectCanvasAt, _, cx| {
                s.pause_at_painted_frame(cx);
                match s.video_to_canvas(action.x, action.y) {
                    Some(point) => s.select_canvas(point, action.cycle, cx),
                    None => {
                        s.canvas.message = Some("The video is not on the canvas yet.".into());
                        cx.notify();
                    }
                }
            }))
            .on_action(cx.listener(|s, _: &actions::TogglePlayback, _, cx| s.toggle_playback(cx)))
            .on_action(cx.listener(|s, _: &actions::StepForward, _, cx| s.step_preview(1, cx)))
            .on_action(cx.listener(|s, _: &actions::StepBackward, _, cx| s.step_preview(-1, cx)))
            .on_action(cx.listener(|s, _: &actions::ToggleMute, _, cx| s.toggle_mute(cx)))
            .on_action(cx.listener(|s, _: &actions::NextAudioOutput, _, cx| s.cycle_output(cx)))
            .on_action(cx.listener(|s, _: &actions::RetryAudioOutput, _, cx| {
                s.change_output(OutputDevice::Default, cx)
            }))
            .on_action(cx.listener(|s, _: &actions::DisableAudioOutput, _, cx| {
                s.change_output(OutputDevice::Unavailable, cx)
            }))
            .on_action(cx.listener(|s, action: &actions::Seek, _, cx| {
                s.timeline.update(cx, |t, cx| t.seek(action.frame, cx))
            }))
            .on_action(cx.listener(|s, _: &actions::SeekToStart, _, cx| {
                s.timeline.update(cx, |t, cx| t.seek(0, cx))
            }))
            .on_action(cx.listener(|s, _: &actions::SeekToEnd, _, cx| {
                s.timeline.update(cx, |t, cx| t.seek_to_end(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::CycleScene, _, cx| {
                s.timeline.update(cx, |t, cx| t.cycle_scene(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::MarkRangeStart, _, cx| {
                s.timeline.update(cx, |t, cx| t.mark_range_start(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::MarkRangeEnd, _, cx| {
                s.timeline.update(cx, |t, cx| t.mark_range_end(cx))
            }))
            .on_action(cx.listener(|s, _: &actions::ZoomIn, _, cx| {
                s.timeline.update(cx, |t, cx| t.zoom(2., cx))
            }))
            .on_action(cx.listener(|s, _: &actions::ZoomOut, _, cx| {
                s.timeline.update(cx, |t, cx| t.zoom(0.5, cx))
            }))
            .on_action(cx.listener(|s, _: &actions::PageTimelineBack, _, cx| {
                s.timeline.update(cx, |t, cx| t.page(-1., cx))
            }))
            .on_action(cx.listener(|s, _: &actions::PageTimelineForward, _, cx| {
                s.timeline.update(cx, |t, cx| t.page(1., cx))
            }))
            .on_action(cx.listener(|s, _: &actions::CycleCanvasSelection, _, cx| {
                s.cycle_canvas_selection(cx)
            }))
            .on_action(cx.listener(|s, _: &actions::ClearCanvasSelection, _, cx| {
                s.clear_canvas_selection(cx)
            }))
            .on_action(
                cx.listener(|s, _: &actions::ResetCanvasView, _, cx| s.reset_canvas_view(cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bound() -> Vec<(String, String, &'static str)> {
        key_bindings()
            .iter()
            .map(|binding| {
                let context = binding
                    .predicate()
                    .map(|predicate| predicate.to_string())
                    .unwrap_or_default();
                let keys = binding
                    .keystrokes()
                    .iter()
                    .map(|keystroke| keystroke.inner().unparse())
                    .collect::<Vec<_>>()
                    .join(" ");
                (context, keys, binding.action().name())
            })
            .collect()
    }

    #[test]
    fn every_shell_binding_is_scoped_to_the_player_or_the_timeline() {
        // A binding in a wider context would win over a focused text field's typing.
        for (context, keys, action) in bound() {
            assert!(
                context == PLAYER || context == TIMELINE,
                "{keys} -> {action} is bound in {context:?}"
            );
        }
    }

    #[test]
    fn the_player_and_the_timeline_answer_their_keys() {
        let bindings = bound();
        for (context, keys, action) in [
            (PLAYER, "space", "studio::TogglePlayback"),
            (PLAYER, "left", "studio::StepBackward"),
            (PLAYER, "right", "studio::StepForward"),
            (PLAYER, "home", "studio::SeekToStart"),
            (PLAYER, "end", "studio::SeekToEnd"),
            (PLAYER, "c", "studio::CycleCanvasSelection"),
            (PLAYER, "0", "studio::ResetCanvasView"),
            (PLAYER, "escape", "studio::ClearCanvasSelection"),
            (TIMELINE, "space", "studio::TogglePlayback"),
            (TIMELINE, "m", "studio::ToggleMute"),
            (TIMELINE, "left", "studio::StepBackward"),
            (TIMELINE, "right", "studio::StepForward"),
            (TIMELINE, "home", "studio::SeekToStart"),
            (TIMELINE, "end", "studio::SeekToEnd"),
            (TIMELINE, "+", "studio::ZoomIn"),
            (TIMELINE, "=", "studio::ZoomIn"),
            (TIMELINE, "shift-=", "studio::ZoomIn"),
            (TIMELINE, "-", "studio::ZoomOut"),
            (TIMELINE, "pageup", "studio::PageTimelineBack"),
            (TIMELINE, "pagedown", "studio::PageTimelineForward"),
            (TIMELINE, "s", "studio::CycleScene"),
            (TIMELINE, "[", "studio::MarkRangeStart"),
            (TIMELINE, "]", "studio::MarkRangeEnd"),
        ] {
            let keys = gpui::Keystroke::parse(keys).unwrap().unparse();
            assert!(
                bindings.contains(&(context.to_owned(), keys.clone(), action)),
                "{context} {keys} -> {action} missing from {bindings:#?}"
            );
        }
    }
}
