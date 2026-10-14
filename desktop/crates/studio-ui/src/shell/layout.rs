//! The window's root layout, after Cap's editor: header, notices, then the editor column
//! (the player beside the sidebar, above the timeline and its resize grip) and the agent
//! column.

use gpui::{
    Context, CursorStyle, DispatchPhase, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render, StyleRefinement, Styled,
    Window, WindowControlArea, canvas, div, prelude::FluentBuilder, px,
};
use studio_theme::METRICS;

use super::StudioShell;
use super::keymap::WORKSPACE;
use super::sections::{Sections, box_probe};
use crate::frame_image::create_render_image;
use crate::kit::{Card, windows_caption_controls};

impl Render for StudioShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        self.viewport = [f32::from(viewport.width), f32::from(viewport.height)];
        if self.theme.refresh(window, cx) {
            // The cached sections paint with the theme too.
            cx.notify();
        }
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
                Some(true) => {
                    self.report_displayed();
                    // A render the player alone asked for can commit the install; the
                    // cached sections render again with what it changed.
                    cx.notify();
                }
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
        let sections = self
            .sections
            .get_or_insert_with(|| Sections::new(cx))
            .clone();
        // The cached sections keep their boxes here: a cached view is laid out by this
        // style alone.
        let windows = cfg!(target_os = "windows");
        // The header row is flush with the window's edges and stays uncached: on Windows it
        // is the drag area and holds the caption buttons, and a cached view does not replay
        // window control areas.
        let header = div()
            .flex()
            .flex_none()
            .w_full()
            .h(px(METRICS.header_h))
            .when(windows, |row| {
                row.window_control_area(WindowControlArea::Drag)
            })
            .child(
                sections.header.clone().cached(
                    StyleRefinement::default()
                        .flex_1()
                        .min_w_0()
                        .h(px(METRICS.header_h)),
                ),
            )
            .when(windows, |row| {
                row.child(
                    windows_caption_controls(
                        &theme,
                        window.is_window_active(),
                        window.is_maximized(),
                        true,
                        true,
                    )
                    .relative()
                    .child(box_probe("caption", cx)),
                )
            });
        let notices = self.render_notices(cx);
        let strip = self.render_status_strip(cx);
        let sidebar = sections.sidebar.clone().cached(
            StyleRefinement::default()
                .w(px(METRICS.left_column_w))
                .flex_none()
                .min_h_0(),
        );
        let entity = cx.entity();
        // The editor column measures itself so the split can clamp against its height.
        let measure = canvas(
            move |bounds, _, cx| {
                let height = f32::from(bounds.size.height);
                entity.update(cx, |shell, cx| {
                    if shell.editor_height != height {
                        shell.editor_height = height;
                        cx.notify();
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let mut editor = div()
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(gutter)
            .child(measure);
        if project_open {
            let split = self.split();
            let player = sections.player.clone();
            editor = editor
                .child(
                    div()
                        .flex_1()
                        .min_h(px(split.player_min_height))
                        .flex()
                        .gap(gutter)
                        .child(player)
                        .child(sidebar),
                )
                .child(
                    div()
                        .relative()
                        .flex_none()
                        .w_full()
                        .h(px(split.timeline_height))
                        .child(
                            Card::editor(&self.theme)
                                .size_full()
                                .overflow_hidden()
                                .child(self.timeline.clone()),
                        )
                        .child(self.render_resize_grip(split.timeline_height, cx)),
                );
        } else {
            editor = editor.child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(gutter)
                    .child(self.render_empty_state(cx))
                    .child(sidebar),
            );
        }
        let agent = sections.agent.clone().cached(
            StyleRefinement::default()
                .w(px(METRICS.agent_panel_w))
                .flex_none()
                .min_h_0(),
        );
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
            .children(
                self.timeline_resize
                    .map(|_| self.render_resize_tracking(cx)),
            )
            .bg(theme.editor.window)
            .text_color(theme.editor.text_1)
            .flex()
            .flex_col()
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .px(gutter)
                    .pb(gutter)
                    .gap(gutter)
                    .children(notices)
                    .children(strip)
                    .child(if self.settings_open {
                        self.render_settings(cx)
                    } else {
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .gap(gutter)
                            .child(editor)
                            .child(agent)
                            .into_any_element()
                    }),
            )
    }
}

impl StudioShell {
    /// The split for the timeline height the grip last asked for, or the default.
    pub(super) fn split(&self) -> VerticalSplit {
        self.split_for(self.timeline_height.unwrap_or(METRICS.timeline_default_h))
    }

    fn split_for(&self, preferred: f32) -> VerticalSplit {
        vertical_split(self.editor_height - METRICS.gutter, preferred)
    }

    /// While the grip is held, follows the pointer through the whole window, including past
    /// its edges, where element listeners stop firing.
    fn render_resize_tracking(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let entity = cx.entity();
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let moved = entity.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase != DispatchPhase::Capture {
                        return;
                    }
                    moved.update(cx, |shell, cx| {
                        // A release outside the window never arrives; a move without the
                        // button ends the drag instead.
                        if event.pressed_button != Some(MouseButton::Left) {
                            shell.timeline_resize = None;
                            cx.notify();
                        } else if let Some((start_y, start_height)) = shell.timeline_resize {
                            let preferred = start_height - (f32::from(event.position.y) - start_y);
                            let height = shell.split_for(preferred).timeline_height;
                            if shell.timeline_height != Some(height) {
                                shell.timeline_height = Some(height);
                                cx.notify();
                            }
                        }
                    });
                });
                let released = entity.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                        released.update(cx, |shell, cx| {
                            shell.timeline_resize = None;
                            cx.notify();
                        });
                    }
                });
            },
        )
        .absolute()
        .size_0()
        .into_any_element()
    }

    /// The handle above the timeline that drags its height, as in Cap's editor.
    fn render_resize_grip(&self, timeline_height: f32, cx: &mut Context<Self>) -> gpui::AnyElement {
        let entity = cx.entity();
        div()
            .id("timeline-resize-handle")
            .absolute()
            .left_0()
            .right_0()
            .top(px(-11.))
            .h(px(14.))
            .cursor(CursorStyle::ResizeRow)
            .flex()
            .justify_center()
            .pt(px(5.))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    shell.timeline_resize = Some((f32::from(event.position.y), timeline_height));
                    // Paints the tracker that listens for the release.
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .w(px(36.))
                    .h(px(4.))
                    .rounded(px(2.))
                    .bg(self.theme.editor.line),
            )
            .child(
                canvas(
                    move |bounds, _, cx| {
                        let measured = [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ];
                        entity.update(cx, |shell, _| shell.grip_bounds = Some(measured));
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .into_any_element()
    }
}

/// The player's height below which the timeline gives way, as in Cap's editor.
const PLAYER_MIN_H: f32 = 320.;

/// How the editor column splits between the player row and the timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct VerticalSplit {
    pub player_min_height: f32,
    pub timeline_height: f32,
}

/// Splits `available` (the editor column's height less the gap between player and
/// timeline) the way Cap's `EditorVerticalLayout` does: the timeline takes `preferred`,
/// clamped so neither side drops below its minimum; when both minimums do not fit,
/// they shrink in proportion.
pub(super) fn vertical_split(available: f32, preferred: f32) -> VerticalSplit {
    let available = available.max(0.);
    let scale = (available / (PLAYER_MIN_H + METRICS.timeline_min_h)).min(1.);
    let player_min_height = PLAYER_MIN_H * scale;
    let timeline_min_height = METRICS.timeline_min_h * scale;
    let timeline_max_height = (available - player_min_height).max(timeline_min_height);
    VerticalSplit {
        player_min_height,
        timeline_height: preferred.clamp(timeline_min_height, timeline_max_height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_timeline_fits_a_full_height_window() {
        let split = vertical_split(700., METRICS.timeline_default_h);
        assert_eq!(split.timeline_height, 260.);
        assert_eq!(split.player_min_height, 320.);
    }

    #[test]
    fn the_timeline_is_clamped_between_its_minimum_and_the_players() {
        assert_eq!(vertical_split(700., 100.).timeline_height, 240.);
        assert_eq!(vertical_split(700., 600.).timeline_height, 380.);
    }

    #[test]
    fn both_minimums_shrink_together_when_they_do_not_fit() {
        let split = vertical_split(280., 260.);
        assert_eq!(split.player_min_height, 160.);
        assert_eq!(split.timeline_height, 120.);
    }

    #[test]
    fn no_space_leaves_no_timeline() {
        let split = vertical_split(-10., 260.);
        assert_eq!(split.player_min_height, 0.);
        assert_eq!(split.timeline_height, 0.);
    }
}
