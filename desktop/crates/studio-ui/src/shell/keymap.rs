//! The shell's action handlers: every studio action dispatched to the window lands here.

use gpui::{Context, InteractiveElement};
use studio_engine::engine::backend::Command;
use studio_engine::presets::PresetCommand;
use studio_engine::services::audio_service::OutputDevice;

use super::{Picker, StudioShell};
use crate::actions;

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
            .on_action(cx.listener(|s, _: &actions::FocusPrompt, window, cx| {
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
    }
}
