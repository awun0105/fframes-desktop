//! The window's root layout: header, notices, the left column, the player and the agent column.

use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window, div, px,
};
use studio_theme::METRICS;

use super::StudioShell;
use super::keymap::WORKSPACE;
use crate::frame_image::create_render_image;
use crate::kit::Card;

impl Render for StudioShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.theme.refresh(window, cx);
        if self.overflow_open {
            self.sync_overflow();
        }
        if self.clear_images {
            self.images.clear(window);
            self.clear_images = false;
        }
        if let Some(install) = self.pending_ready.take() {
            let ready = &install.ready.clone();
            let audio_ready = install.audio_ready;
            let mut slot = Some(install);
            // No filesystem work here. The controller's nonblocking guard is the source
            // fence and it is held from the final `can_install` check through the
            // coordinator commit and the preview-state install: a workflow publication
            // or a source reconcile cannot interleave, and every check reads the guard's
            // current state, never a copy taken when preparation finished. Unavailable
            // (a job holds the controller, or a source scan is pending) means deferred.
            let engine = self.engine.clone();
            let outcome = audio_ready.then(|| {
                engine.with_install_fence(|source| {
                    let mut committed = false;
                    if self.presentation.state.as_ref().is_some_and(|s| {
                        s.session() == source.session()
                            && s.source() == source.source()
                            && s.generation() == source.generation()
                    }) && self.preview_state.can_install(ready, source).is_ok()
                    {
                        let image = ready
                            .frame
                            .as_ref()
                            .map(|f| create_render_image(&f.response.header, &f.pixels))
                            .transpose();
                        match image {
                            Ok(image) => {
                                let model =
                                    studio_engine::TimelineModel::new(ready.timeline.clone());
                                if let Err(e) = &model {
                                    self.error = Some(e.clone());
                                }
                                if self.preview.as_ref().is_some_and(|p| {
                                    model.is_ok()
                                        && p.commit(ready.identity().clone(), ready.seek_serial)
                                }) && self.preview_state.install(ready, source).is_ok()
                                {
                                    if let Some(image) = image {
                                        self.images.replace_image(image, window);
                                    } else {
                                        self.images.clear(window);
                                    }
                                    let install = slot.take().expect("taken once");
                                    self.transport = install.clock;
                                    if self.resume_install
                                        && ready.position < ready.timeline.total_frames
                                    {
                                        let _ = self
                                            .transport
                                            .toggle(self.clock_start.elapsed().as_secs_f64());
                                    }
                                    self.resume_install = false;
                                    self.output = install.audio;
                                    self.audio_ready_epoch = Some(self.transport.epoch());
                                    self.requested_clock_epoch = self.transport.epoch();
                                    self.presented_serial = ready.seek_serial;
                                    self.painted_frame =
                                        ready.frame.as_ref().map(|f| f.response.frame_index);
                                    if let Some(frame) = ready.frame.as_ref() {
                                        if let Err(error) = self
                                            .canvas
                                            .install_frame(ready.identity().clone(), frame)
                                        {
                                            self.canvas.clear();
                                            self.canvas.message = Some(format!(
                                                "Canvas metadata unavailable: {error}"
                                            ));
                                        }
                                    } else {
                                        self.canvas.clear();
                                    }
                                    self.timeline.update(cx, |v, cx| {
                                        v.install(model.unwrap(), ready.position, cx)
                                    });
                                    self.displayed = Some(ready.clone());
                                    if let Some(workflow) = &self.agent.workflow {
                                        let _ = workflow.set_displayed_preview_identity(Some(
                                            ready.identity().clone(),
                                        ));
                                    }
                                    if ready.tag().base_source == *source.source() {
                                        self.awaiting_preview = None;
                                    }
                                    // Tell the workflow what is on screen so its accepted-awaiting
                                    // label clears exactly when the matching preview shows.
                                    self.agent.displayed =
                                        Some(ready.identity().source_revision.clone());
                                    self.error = None;
                                    // The commit dropped the queued thumbnails of the old preview.
                                    self.thumbnails_sent = None;
                                    committed = true;
                                }
                            }
                            Err(e) => self.error = Some(e.to_string()),
                        }
                    }
                    committed
                })
            });
            match outcome.flatten() {
                // Deferred, not rejected.
                None => self.pending_ready = slot,
                Some(true) => self.report_displayed(),
                Some(false) => {
                    self.audio.stop(self.transport.epoch());
                    self.discard_install(cx);
                }
            }
        }
        if let Some(f) = self.pending_frame.take()
            && self.preview_state.accepts_frame(&f)
            && self.requested_clock_epoch == self.transport.epoch()
        {
            match create_render_image(&f.response.header, &f.pixels) {
                Ok(image) => {
                    self.images.replace_image(image, window);
                    if let Some(preview) = self
                        .displayed
                        .as_ref()
                        .map(|ready| ready.identity().clone())
                    {
                        if let Err(error) = self.canvas.install_frame(preview, &f) {
                            self.canvas.clear();
                            self.canvas.message =
                                Some(format!("Canvas metadata unavailable: {error}"));
                        }
                    } else {
                        self.canvas.clear();
                    }
                    self.presented_serial = f.response.seek_serial;
                    self.painted_frame = Some(f.response.frame_index);
                    self.install_task_scope(self.timeline.read(cx).selected_scope(), cx);
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        self.activate_audio(cx);
        let theme = self.theme;
        let gutter = px(METRICS.gutter);
        let project_open = self.presentation.project.is_some();
        let header = self.render_header(cx);
        let notices = self.render_notices(cx);
        let strip = self.render_status_strip(cx);
        let left = self.render_left_column(cx);
        let main = if project_open {
            let player = self.render_player_area(cx);
            div()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .flex()
                .flex_col()
                .gap(gutter)
                .child(player)
                .child(
                    Card::editor(&self.theme)
                        .flex_none()
                        .overflow_hidden()
                        .child(self.timeline.clone()),
                )
                .into_any_element()
        } else {
            self.render_empty_state(cx)
        };
        let agent = self.render_agent_column();
        Self::on_actions(div(), cx)
            .size_full()
            .key_context(WORKSPACE)
            .track_focus(&self.focus)
            .tab_group()
            .on_key_down(cx.listener(|_, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "tab" {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    cx.stop_propagation();
                }
            }))
            .bg(theme.editor.window)
            .text_color(theme.editor.text_1)
            .flex()
            .flex_col()
            .p(gutter)
            .gap(gutter)
            .child(header)
            .children(notices)
            .children(strip)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(gutter)
                    .child(left)
                    .child(main)
                    .child(agent),
            )
    }
}
