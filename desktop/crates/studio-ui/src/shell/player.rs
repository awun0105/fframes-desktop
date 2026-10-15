//! The player: preview frames, playback and audio, canvas selection, and the player card's toolbar
//! and stage.

use gpui::{
    Context, InteractiveElement, IntoElement, MouseButton, ParentElement,
    StatefulInteractiveElement, Styled, canvas, div, px,
};
use studio_engine::TaskScope;
use studio_engine::engine::handle::{PreviewGated, PreviewNews};
use studio_engine::services::audio_service::{AudioEvent, OutputDevice};
use studio_engine::services::preview_coordinator::SeekIntent;
use studio_theme::{METRICS, Shadow, TextStyle, shadow_for, text_style};

use super::keymap::PLAYER;
use super::transport::{show_total_time, transport_mode};
use super::{PreparedInstall, StudioShell};
use crate::actions;
use crate::canvas_view::StageLayout;
use crate::kit::{Badge, Card, Icon, Tone, focus_ring, icon, with_tooltip};

impl StudioShell {
    pub(super) fn detach_preview(&mut self, cx: &mut Context<Self>) {
        if let Some(workflow) = &self.agent.workflow {
            let _ = workflow.set_displayed_preview_identity(None);
        }
        if let Some(preview) = self.preview.take() {
            // Teardown stays off the UI thread; the root token rejects any late spawn.
            std::thread::spawn(move || preview.close());
        }
        self.preview_session = None;
        self.awaiting_preview = None;
        self.preview_state.close();
        self.displayed = None;
        self.pending_ready = None;
        self.pending_frame = None;
        self.canvas.clear();
        self.canvas_pan_position = None;
        self.clear_images = true;
        let _ = self
            .transport
            .reinstall(30, 0, 0, self.clock_start.elapsed().as_secs_f64());
        self.audio.stop(self.transport.epoch());
        self.output = None;
        self.audio_ready_epoch = None;
        self.resume_install = false;
        self.audio_note = "Audio output not prepared".into();
        self.resume_scrub = false;
        self.timeline.update(cx, |v, cx| v.clear(cx));
    }

    /// Takes the preview coordinator's news. Held back while a command runs or earlier
    /// news is with the engine; both drain again when they finish.
    pub(super) fn drain_preview(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.preview_gate_pending {
            return;
        }
        let Some(preview) = &self.preview else {
            return;
        };
        let events = preview.events();
        if let Some((key, frame)) = events.thumbnail {
            self.timeline
                .update(cx, |v, cx| v.thumbnail(key, frame, cx));
            self.thumbnails_sent = None;
        }
        if let Some(frame) = events.frame
            && self.preview_state.accepts_frame(&frame)
            && self.requested_clock_epoch == self.transport.epoch()
        {
            self.pending_frame = Some(frame);
            self.notify_player(cx);
        }
        if events.compiled.is_none() && events.ready.is_none() && events.error.is_none() {
            return;
        }
        self.preview_gate_pending = true;
        self.engine.preview_news(PreviewNews {
            compiled: events.compiled,
            error: events.error,
            ready: events.ready,
        });
    }

    /// Asks the coordinator for the timeline's visible thumbnails when they changed;
    /// nothing while playing.
    pub(super) fn request_thumbnails(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = &self.preview else {
            return;
        };
        let requests = if self.transport.playing() {
            vec![]
        } else {
            self.timeline.read(cx).thumbnail_requests()
        };
        if self.thumbnails_sent.as_ref() != Some(&requests) {
            preview.thumbnails(requests.clone());
            self.thumbnails_sent = Some(requests);
        }
    }

    pub(super) fn preview_gated(&mut self, gated: PreviewGated, cx: &mut Context<Self>) {
        let PreviewGated {
            ready,
            state,
            error,
            presentation,
            ..
        } = gated;
        self.preview_gate_pending = false;
        if let Some(p) = presentation {
            self.presentation = p;
            self.push_preset_view(cx);
        }
        if let Some((tag, e)) = error {
            self.preview_state.fail(&tag, e.clone());
            self.discard_install(cx);
            self.error = Some(e);
        } else if let (Some(ready), Some(state)) = (ready, state) {
            // A staged preview of just-published bytes carries the promotion
            // tag (base source = the published revision, the task's own
            // generation); it is installable only under the engine's current
            // authorization, never as a build of the task base.
            let promoted = state.promotion().is_some_and(|p| p.tag() == ready.tag());
            if !promoted
                && (ready.tag().generation != state.generation()
                    || ready.tag().base_source != *state.source())
            {
                if let Some(preview) = &self.preview {
                    preview.cancel_build();
                }
                self.preview_state.cancel_build();
            } else if self.transport.playing()
                && (promoted
                    || matches!(state.job(), studio_engine::JobState::Succeeded(tag) if tag == ready.tag()))
            {
                // Re-evaluate the audible position, then re-prime the candidate
                // under the newly frozen seek serial. Old playback served the
                // entire compilation/preparation, not the compiler's cursor.
                self.resume_install = true;
                self.pause_playback(cx);
            } else if self.preview_state.can_install(&ready, &state).is_ok() {
                let mut clock = self.transport.clone();
                if let Err(e) = clock.reinstall(
                    ready.timeline.fps,
                    ready.timeline.total_frames,
                    ready.position,
                    self.clock_start.elapsed().as_secs_f64(),
                ) {
                    self.error = Some(e.to_string());
                } else {
                    // Reserve the staged epoch in the active clock as well.
                    // A seek during preparation must advance past this epoch,
                    // never collide with a callback for another revision.
                    let _ = self.transport.wait_for_output();
                    self.audio.stage(
                        ready.clone(),
                        clock.epoch(),
                        ready.position,
                        self.output_device.clone(),
                    );
                    self.output = None;
                    self.audio_ready_epoch = None;
                    self.pending_ready = Some(PreparedInstall {
                        ready,
                        clock,
                        audio: None,
                        audio_ready: false,
                    });
                    self.audio_note = "Priming matching audio and first frame".into();
                }
            }
        }
        cx.notify();
    }

    pub(super) fn seek_preview(&mut self, position: usize, cx: &mut Context<Self>) {
        let resume = std::mem::take(&mut self.resume_install);
        self.pending_ready = None;
        if let Err(e) = self
            .transport
            .seek(position, self.clock_start.elapsed().as_secs_f64())
        {
            self.error = Some(e.to_string());
            return;
        }
        self.audio.stop(self.transport.epoch());
        self.output = None;
        self.audio_ready_epoch = None;
        self.request_frame(self.transport.position(), cx);
        if resume && !self.transport.playing() {
            self.toggle_playback(cx);
        }
        if self.transport.playing() {
            self.prime_playback(cx);
        }
    }

    fn request_frame(&mut self, position: usize, cx: &mut Context<Self>) {
        if self.send_frame_request(position, cx) {
            cx.notify();
        }
    }

    /// A new frame or playhead during playback: only the player section renders again, the
    /// cached sections stay as they are.
    fn notify_player(&mut self, cx: &mut Context<Self>) {
        match &self.sections {
            Some(sections) => sections.player.update(cx, |_, cx| cx.notify()),
            None => cx.notify(),
        }
    }

    /// Asks the coordinator for the frame at `position`; false when nothing is shown or
    /// the seek was refused.
    fn send_frame_request(&mut self, position: usize, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.preview_state.displayed().cloned() else {
            return false;
        };
        let scale = self.displayed.as_ref().map_or(1., |d| {
            crate::frame_image::preview_scale(
                d.timeline.width,
                d.timeline.height,
                self.preview_extent,
            )
        });
        let Ok(serial) = self.preview_state.seek(position, scale) else {
            return false;
        };
        self.requested_clock_epoch = self.transport.epoch();
        if let Some(preview) = &self.preview {
            preview.seek(SeekIntent {
                identity: id,
                serial,
                position: self.preview_state.position(),
                scale: self.preview_state.scale(),
            });
        }
        self.timeline.update(cx, |v, cx| {
            v.set_position(self.preview_state.position(), cx)
        });
        true
    }

    pub(super) fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        if self.transport.playing() || self.resume_install {
            self.resume_install = false;
            self.pause_playback(cx);
        } else {
            self.pending_ready = None;
            if let Err(e) = self
                .transport
                .toggle(self.clock_start.elapsed().as_secs_f64())
            {
                self.error = Some(e.to_string());
            } else if self.transport.playing() {
                // Every start passes through a matching first-frame/PCM epoch.
                let _ = self.transport.wait_for_output();
                self.request_frame(self.transport.position(), cx);
                self.prime_playback(cx);
            }
        }
        cx.notify();
    }

    pub(super) fn pause_playback(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self
            .transport
            .pause(self.clock_start.elapsed().as_secs_f64())
        {
            self.error = Some(e.to_string());
        }
        self.pending_ready = None;
        self.audio.stop(self.transport.epoch());
        self.output = None;
        self.audio_ready_epoch = None;
        self.request_frame(self.transport.position(), cx);
    }

    pub(super) fn pause_at_painted_frame(&mut self, cx: &mut Context<Self>) {
        if !self.transport.playing() {
            return;
        }
        let frame = self
            .painted_frame
            .unwrap_or_else(|| self.transport.position());
        self.pause_playback(cx);
        self.seek_preview(frame, cx);
    }

    pub(super) fn install_task_scope(
        &mut self,
        scope: Result<Option<TaskScope>, String>,
        cx: &mut Context<Self>,
    ) {
        let scope = scope.and_then(|scope| match scope {
            Some(mut scope) => {
                scope.canvas_selection = self.canvas.task_scope_selection();
                scope.validate().map_err(|error| error.to_string())?;
                Ok(Some(scope))
            }
            None if self.canvas.task_scope_selection().is_some() => {
                Err("canvas selection has no matching compiled timeline".into())
            }
            None => Ok(None),
        });
        self.panel
            .update(cx, |panel, cx| panel.set_task_scope(scope, cx));
    }

    fn canvas_point(&self, position: gpui::Point<gpui::Pixels>) -> (f64, f64) {
        (
            f64::from(position.x) - self.canvas_origin.0,
            f64::from(position.y) - self.canvas_origin.1,
        )
    }

    fn canvas_pointer_down(&mut self, event: &gpui::MouseDownEvent, cx: &mut Context<Self>) {
        let point = self.canvas_point(event.position);
        if event.button != MouseButton::Middle {
            self.canvas_pan_position = None;
        }
        if event.button == MouseButton::Left {
            self.pause_at_painted_frame(cx);
        }
        match event.button {
            MouseButton::Middle => self.canvas_pan_position = Some(point),
            MouseButton::Left if event.modifiers.shift => {
                if !self.canvas.begin_rectangle(point.0, point.1) {
                    self.canvas.message =
                        Some("Start a rectangle drag inside the painted video image.".into());
                }
            }
            MouseButton::Left => {
                return self.select_canvas(point, event.modifiers.alt, cx);
            }
            _ => return,
        }
        self.install_task_scope(self.timeline.read(cx).selected_scope(), cx);
        cx.notify();
    }

    /// Selects the object at canvas `point` (`cycle`: the next one under it) and makes the
    /// selection the agent's task scope.
    pub(super) fn select_canvas(&mut self, point: (f64, f64), cycle: bool, cx: &mut Context<Self>) {
        if let Err(error) = self.canvas.select_at(point.0, point.1, cycle) {
            self.canvas.message = Some(error.to_string());
        }
        self.install_task_scope(self.timeline.read(cx).selected_scope(), cx);
        cx.notify();
    }

    /// Where video pixel (`x`, `y`) is painted on the canvas, if the video is laid out.
    pub(super) fn video_to_canvas(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let viewport = self.canvas.viewport.as_ref()?;
        let image = viewport.image_bounds();
        let (width, height) = viewport.video_dimensions();
        Some((
            image.x + x * image.width / f64::from(width),
            image.y + y * image.height / f64::from(height),
        ))
    }

    fn canvas_pointer_move(&mut self, event: &gpui::MouseMoveEvent, cx: &mut Context<Self>) {
        let point = self.canvas_point(event.position);
        if self.canvas.is_drawing_rectangle() {
            self.canvas.update_rectangle(point.0, point.1);
            cx.notify();
        } else if event.pressed_button == Some(MouseButton::Middle)
            && let Some(previous) = self.canvas_pan_position.replace(point)
            && let Some(viewport) = &mut self.canvas.viewport
        {
            let _ = viewport.pan_by(point.0 - previous.0, point.1 - previous.1);
            cx.notify();
        }
    }

    fn canvas_pointer_up(&mut self, button: MouseButton, cx: &mut Context<Self>) {
        if button == MouseButton::Middle {
            self.canvas_pan_position = None;
        } else if button == MouseButton::Left && self.canvas.is_drawing_rectangle() {
            if let Some(rect) = self.canvas.end_rectangle() {
                self.canvas.message = Some(format!(
                    "Rectangle scope · {:.0}×{:.0} video pixels · source identity is not inferred.",
                    rect.width, rect.height
                ));
            }
            self.install_task_scope(self.timeline.read(cx).selected_scope(), cx);
            cx.notify();
        }
    }

    fn canvas_scroll(&mut self, event: &gpui::ScrollWheelEvent, cx: &mut Context<Self>) {
        let point = self.canvas_point(event.position);
        let delta = match event.delta {
            gpui::ScrollDelta::Pixels(pixels) => f64::from(pixels.y),
            gpui::ScrollDelta::Lines(lines) => f64::from(lines.y * 32.0),
        };
        if event.modifiers.control || event.modifiers.platform {
            let factor = if delta > 0.0 {
                1.0 + delta.abs() * 0.01
            } else {
                1.0 / (1.0 + delta.abs() * 0.01)
            };
            if let Some(viewport) = &mut self.canvas.viewport {
                let _ = viewport.zoom_at(point.0, point.1, factor);
            }
        } else if let Some(viewport) = &mut self.canvas.viewport {
            let _ = viewport.pan_by(-delta, 0.0);
        }
        cx.notify();
    }

    pub(super) fn cycle_canvas_selection(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.canvas.cycle_last_hit() {
            self.canvas.message = Some(error.to_string());
        }
        cx.notify();
    }

    pub(super) fn clear_canvas_selection(&mut self, cx: &mut Context<Self>) {
        self.canvas.clear_selection();
        self.install_task_scope(self.timeline.read(cx).selected_scope(), cx);
        cx.notify();
    }

    pub(super) fn reset_canvas_view(&mut self, cx: &mut Context<Self>) {
        if let Some(viewport) = &mut self.canvas.viewport {
            viewport.reset_to_fit();
        }
        cx.notify();
    }

    fn prime_playback(&mut self, cx: &mut Context<Self>) {
        if let Some(ready) = &self.displayed {
            self.output = None;
            self.audio_ready_epoch = None;
            self.audio.stage(
                ready.clone(),
                self.transport.epoch(),
                self.transport.position(),
                self.output_device.clone(),
            );
            self.audio_note = "Buffering · waiting for matching first frame and PCM".into();
            cx.notify();
        }
    }

    pub(super) fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        self.muted = !self.muted;
        self.audio.mute(self.muted);
        cx.notify();
    }

    pub(super) fn change_output(&mut self, next: OutputDevice, cx: &mut Context<Self>) {
        self.output_device = next;
        self.seek_preview(self.transport.position(), cx);
        if !self.transport.playing() {
            self.prime_playback(cx);
        }
    }

    pub(super) fn cycle_output(&mut self, cx: &mut Context<Self>) {
        let next = self.output_device.next(&self.audio.devices());
        self.change_output(next, cx);
    }

    fn poll_audio(&mut self, cx: &mut Context<Self>) {
        match self.audio.event() {
            Some(AudioEvent::Staged {
                epoch,
                identity,
                output,
                unavailable,
            }) => {
                if let Some(pending) = &mut self.pending_ready
                    && pending.clock.epoch() == epoch
                    && pending.ready.identity() == &identity
                {
                    pending.audio = output;
                    pending.audio_ready = true;
                    self.audio_note =
                        unavailable.unwrap_or_else(|| "Matching audio prepared".into());
                } else if epoch == self.transport.epoch()
                    && self.preview_state.displayed() == Some(&identity)
                {
                    self.output = output;
                    self.audio_ready_epoch = Some(epoch);
                    self.audio_note = unavailable.unwrap_or_else(|| "Audio output prepared".into());
                }
                cx.notify();
            }
            Some(AudioEvent::Failed { epoch, error }) => {
                if self
                    .pending_ready
                    .as_ref()
                    .is_some_and(|p| p.clock.epoch() == epoch)
                {
                    let pending = self.pending_ready.take().unwrap();
                    self.preview_state.fail(pending.ready.tag(), error.clone());
                    if let Some(preview) = &self.preview {
                        preview.cancel_build();
                    }
                    self.error = Some(format!(
                        "Audio preparation failed; prior preview retained: {error}"
                    ));
                    if self.resume_install {
                        self.resume_install = false;
                        self.toggle_playback(cx);
                    }
                } else if epoch == self.transport.epoch() {
                    self.output = None;
                    self.audio_ready_epoch = Some(epoch);
                    self.audio_note = format!("Audio unavailable · {error}");
                }
                cx.notify();
            }
            Some(AudioEvent::Lost { epoch }) if epoch == self.transport.epoch() => {
                self.output = None;
                self.audio_ready_epoch = None;
                let _ = self
                    .transport
                    .fallback(self.clock_start.elapsed().as_secs_f64());
                self.audio.stop(self.transport.epoch());
                self.request_frame(self.transport.position(), cx);
                self.audio_note =
                    "Audio device lost · monotonic fallback · Retry audio to reconnect".into();
            }
            _ => {}
        }
    }

    pub(super) fn activate_audio(&mut self, cx: &mut Context<Self>) {
        let epoch = self.transport.epoch();
        if self.audio_ready_epoch == Some(epoch)
            && self.presented_serial == self.preview_state.serial()
        {
            self.audio_ready_epoch = None;
            if self.output.is_some() {
                self.audio.commit(epoch, self.transport.playing());
            } else {
                let _ = self
                    .transport
                    .fallback(self.clock_start.elapsed().as_secs_f64());
                self.audio.stop(self.transport.epoch());
                self.requested_clock_epoch = self.transport.epoch();
                self.audio_note = "Audio unavailable · monotonic fallback".into();
            }
            cx.notify();
        }
    }

    pub(super) fn step_preview(&mut self, delta: isize, cx: &mut Context<Self>) {
        self.resume_install = false;
        self.pause_playback(cx);
        self.seek_preview(self.transport.position().saturating_add_signed(delta), cx);
    }

    pub(super) fn tick_playback(&mut self, cx: &mut Context<Self>) {
        self.poll_audio(cx);
        if let Some(output) = &self.output
            && let Some(snapshot) = output.snapshot()
        {
            self.transport.accept_output(snapshot);
        }
        let was_playing = self.transport.playing();
        let position = self
            .transport
            .tick(self.clock_start.elapsed().as_secs_f64());
        if position != self.preview_state.position() && self.send_frame_request(position, cx) {
            self.notify_player(cx);
        }
        if was_playing != self.transport.playing() {
            if !self.transport.playing() {
                self.audio.stop(self.transport.epoch());
                self.output = None;
                self.audio_ready_epoch = None;
            }
            cx.notify();
        }
    }

    /// The preview, transport and history controls: toolbar, stage and transport
    /// inside one card. The timeline card is placed by `render`.
    pub(super) fn render_player_area(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let entity = cx.entity();
        let mut player = Card::editor(&self.theme)
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.render_player_toolbar(cx))
            .child(self.render_stage(cx))
            .child(self.render_transport(cx))
            .child(super::sections::section_probe(
                super::sections::Section::Player,
                cx,
            ))
            .child(
                // Measures the card so the transport can pick its compact layout.
                canvas(
                    move |bounds, _, cx| {
                        let width = f32::from(bounds.size.width);
                        entity.update(cx, |shell, cx| {
                            if transport_mode(shell.player_width) != transport_mode(width)
                                || show_total_time(shell.player_width) != show_total_time(width)
                            {
                                cx.notify();
                            }
                            shell.player_width = width;
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        if let Some(menu) = self.render_overflow_menu(cx) {
            player = player.child(menu);
        }
        player.into_any_element()
    }

    fn render_player_toolbar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        let (label, tone) = match &self.preview_state.status {
            studio_engine::PreviewStatus::Building => {
                ("Building/preparing · prior preview retained", Tone::Info)
            }
            studio_engine::PreviewStatus::Preparing => {
                ("Preparing matching frame and audio", Tone::Info)
            }
            studio_engine::PreviewStatus::Error(_) => {
                ("Preview failed · prior preview retained", Tone::Warning)
            }
            _ => ("CPU preview · shaders unsupported", Tone::Neutral),
        };
        let audio_label = self.output.as_ref().map_or_else(
            || self.audio_note.clone(),
            |output| {
                let metrics = output.metrics();
                format!(
                    "{} · {} Hz · predicted output clock{}{} · latency {:.1}ms · underruns {}",
                    output.device,
                    output.sample_rate,
                    if self.muted { " · muted" } else { "" },
                    if metrics.invalid_timestamps > 0 {
                        " (latency estimate)"
                    } else {
                        ""
                    },
                    metrics.predicted_latency_ms,
                    metrics.underrun_frames
                )
            },
        );
        let audio_ready = self.displayed.is_some();
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(METRICS.space_2))
            .h(px(METRICS.toolbar_h))
            .px(px(METRICS.space_3))
            .border_b_1()
            .border_color(palette.line)
            .overflow_hidden()
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .child(Badge::new(label).tone(tone)),
            )
            .child(with_tooltip(
                div().id("canvas-hint").flex_none().child(icon(
                    Icon::Info,
                    METRICS.icon_md,
                    palette.text_3,
                )),
                &theme,
                Some(CANVAS_HINT.into()),
            ))
            .child(
                text_style(div(), TextStyle::Small)
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_right()
                    .text_color(palette.text_2)
                    .child(audio_label),
            )
            .child(self.action_button(
                "next-audio-output",
                "Next output",
                audio_ready,
                false,
                None,
                cx,
                |s, cx| s.cycle_output(cx),
            ))
            .child(self.action_button(
                "retry-audio-output",
                "Retry audio",
                audio_ready,
                false,
                None,
                cx,
                |s, cx| s.change_output(OutputDevice::Default, cx),
            ))
            .child(self.action_button(
                "no-audio-output",
                "No audio",
                audio_ready,
                false,
                None,
                cx,
                |s, cx| s.change_output(OutputDevice::Unavailable, cx),
            ))
            .into_any_element()
    }

    /// The stage: the letterboxed frame with its ring, shadow and overlays inside
    /// the `preview-surface` that owns pointer, keyboard and telemetry.
    fn render_stage(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = self.theme;
        let palette = theme.editor;
        let radius = px(METRICS.radius_frame);
        let mut preview_surface = div()
            .id("preview-surface")
            .relative()
            .track_focus(&self.preview_focus)
            .tab_index(0)
            .role(gpui::Role::Group)
            .aria_label("Compiled preview")
            .focus_visible(|s| s.shadow(focus_ring(&theme)))
            .key_context(PLAYER)
            // Home and End on the preview seek the shown preview itself; from anywhere
            // else they go through the timeline (see the window's handlers).
            .on_action(
                cx.listener(|shell, _: &actions::SeekToStart, _, cx| shell.seek_preview(0, cx)),
            )
            .on_action(cx.listener(|shell, _: &actions::SeekToEnd, _, cx| {
                if let Some(d) = &shell.displayed {
                    shell.seek_preview(d.timeline.total_frames, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, event: &gpui::MouseDownEvent, _, cx| {
                    shell.canvas_pointer_down(event, cx)
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|shell, event: &gpui::MouseDownEvent, _, cx| {
                    shell.canvas_pointer_down(event, cx)
                }),
            )
            .on_mouse_move(cx.listener(|shell, event: &gpui::MouseMoveEvent, _, cx| {
                shell.canvas_pointer_move(event, cx)
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|shell, _, _, cx| shell.canvas_pointer_up(MouseButton::Left, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|shell, _, _, cx| shell.canvas_pointer_up(MouseButton::Left, cx)),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|shell, _, _, cx| shell.canvas_pointer_up(MouseButton::Middle, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Middle,
                cx.listener(|shell, _, _, cx| shell.canvas_pointer_up(MouseButton::Middle, cx)),
            )
            .on_scroll_wheel(cx.listener(|shell, event: &gpui::ScrollWheelEvent, _, cx| {
                shell.canvas_scroll(event, cx)
            }))
            .flex_1()
            .min_h(px(120.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2();
        let entity = cx.entity();
        // The surface is the padded stage's content box; its bounds are what the
        // preview telemetry and click mapping report.
        preview_surface = preview_surface.child(
            canvas(
                move |bounds, window, cx| {
                    let dpi = window.scale_factor();
                    let extent = (
                        (f32::from(bounds.size.width) * dpi).round().max(1.) as u32,
                        (f32::from(bounds.size.height) * dpi).round().max(1.) as u32,
                    );
                    entity.update(cx, |shell, cx| {
                        shell.canvas_origin =
                            (f64::from(bounds.origin.x), f64::from(bounds.origin.y));
                        shell.canvas_extent =
                            (f64::from(bounds.size.width), f64::from(bounds.size.height));
                        shell.preview_extent = extent;
                        if let Some(displayed) = &shell.displayed {
                            let before = shell.canvas.viewport.clone();
                            let _ = shell.canvas.resize(
                                f64::from(bounds.size.width),
                                f64::from(bounds.size.height),
                                displayed.timeline.width as u32,
                                displayed.timeline.height as u32,
                            );
                            if before != shell.canvas.viewport {
                                cx.notify();
                            }
                        }
                        if shell.displayed.as_ref().is_some_and(|d| {
                            crate::frame_image::preview_scale(
                                d.timeline.width,
                                d.timeline.height,
                                extent,
                            ) != shell.preview_state.scale()
                        }) {
                            shell.request_frame(shell.transport.position(), cx);
                        }
                    });
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        );
        if let Some(image) = self.images.current_image() {
            let image = gpui::img(image).rounded(radius);
            preview_surface = if let Some(layout) = self
                .canvas
                .viewport
                .as_ref()
                .map(StageLayout::from_viewport)
            {
                let frame = layout.frame_rect;
                let placed = |element: gpui::Div, rect: studio_engine::CanvasRect| {
                    element
                        .absolute()
                        .left(px(rect.x as f32))
                        .top(px(rect.y as f32))
                        .w(px(rect.width as f32))
                        .h(px(rect.height as f32))
                };
                // Overlays live inside the frame, so they clip to it and use frame
                // coordinates.
                let mut content = placed(div(), frame)
                    .overflow_hidden()
                    .child(image.size_full());
                if let Some(bounds) = self.canvas.selected_bounds() {
                    content = content.child(
                        placed(div(), layout.to_frame_space(bounds))
                            .border_2()
                            .border_color(palette.accent)
                            .bg(theme.selection()),
                    );
                }
                if let (Some(viewport), Some(rect)) =
                    (&self.canvas.viewport, &self.canvas.rectangle_scope)
                {
                    content = content.child(
                        placed(
                            div(),
                            layout.to_frame_space(viewport.video_scope_bounds(rect)),
                        )
                        .border_2()
                        .border_color(theme.amber_11),
                    );
                }
                preview_surface
                    .child(
                        placed(div(), frame)
                            .rounded(radius)
                            .shadow(shadow_for(theme.appearance, Shadow::Preview).into_vec()),
                    )
                    .child(content)
                    .child(
                        placed(div(), frame)
                            .rounded(radius)
                            .border_1()
                            .border_color(palette.line),
                    )
            } else {
                preview_surface.child(image.size_full())
            };
        } else {
            preview_surface = preview_surface
                .child(
                    text_style(div(), TextStyle::Title)
                        .text_color(palette.text_1)
                        .child(if self.displayed.is_some() {
                            "Empty video"
                        } else {
                            "No preview available"
                        }),
                )
                .child(
                    text_style(div(), TextStyle::Body)
                        .text_color(palette.text_2)
                        .child("Build preview to compile the project's current source."),
                );
        }
        let mut stage = div()
            .id("stage")
            .relative()
            .flex_1()
            .min_h(px(120.))
            .flex()
            .flex_col()
            .overflow_hidden()
            .p(px(METRICS.space_4))
            .bg(palette.stage)
            .child(preview_surface);
        if let Some(message) = &self.canvas.message {
            stage = stage.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom(px(METRICS.space_4))
                    .flex()
                    .justify_center()
                    .child(
                        text_style(div(), TextStyle::Small)
                            .px(px(METRICS.space_3))
                            .py(px(METRICS.space_1))
                            .rounded(px(METRICS.radius_control))
                            .border_1()
                            .border_color(palette.line)
                            .bg(palette.card)
                            .text_color(palette.text_2)
                            .child(message.clone()),
                    ),
            );
        }
        stage.into_any_element()
    }
}

const CANVAS_HINT: &str = "Select: click · Rectangle: Shift-drag · Cycle: Alt-click/C · Zoom: Ctrl-wheel · Pan: middle-drag · Fit: 0";
