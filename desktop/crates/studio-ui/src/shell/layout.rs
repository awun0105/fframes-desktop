//! The window's root layout: header and notices, then the workspace row (the rail and its
//! open drawer, the player above the timeline and its resize grip, the agent column with
//! its own grip), then the status bar.

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
                                    let waveform = ready
                                        .audio_source
                                        .as_ref()
                                        .map(|source| source.waveform().clone())
                                        .unwrap_or_else(|| std::sync::Arc::from([]));
                                    self.timeline.update(cx, |v, cx| {
                                        v.install(model.unwrap(), waveform, ready.position, cx)
                                    });
                                    self.displayed = Some(ready.clone());
                                    if let Some(workflow) = &self.agent.workflow {
                                        let _ = workflow.set_displayed_preview_identity(Some(
                                            ready.identity().clone(),
                                        ));
                                    }
                                    if ready.tag().base_source == *source.source()
                                        || source.is_current_build(ready.tag())
                                    {
                                        self.awaiting_preview = None;
                                    }
                                    // Tell the workflow what is on screen so its accepted-awaiting
                                    // label clears exactly when the matching preview shows.
                                    self.agent.displayed =
                                        Some(ready.identity().source_revision.clone());
                                    self.error = None;
                                    // The commit dropped the queued thumbnails of the old preview.
                                    self.thumbnails_sent = None;
                                    self.request_thumbnails(cx);
                                    self.forward_playhead();
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
                    self.images.present(image, f.response.seek_serial, window);
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
        let status_bar = self.render_status_strip(cx);
        let rail_w = METRICS.rail_w
            + if self.drawer.is_some() {
                METRICS.gutter + METRICS.drawer_w
            } else {
                0.
            };
        let rail = sections.rail.clone().cached(
            StyleRefinement::default()
                .w(px(rail_w))
                .flex_none()
                .min_h_0(),
        );
        let agent_w = self.agent_column_width();
        let agent = div()
            .relative()
            .flex_none()
            .w(px(agent_w))
            .flex()
            .child(
                sections
                    .agent
                    .clone()
                    .cached(StyleRefinement::default().flex_1().min_w_0().min_h_0()),
            )
            .child(self.render_agent_grip(agent_w, cx));
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
        // The playhead runs between frames while playing; only this layer repaints for it.
        let playing = self.transport.playing();
        let playhead = self.timeline.update(cx, |timeline, _| {
            timeline.set_playing(playing);
            timeline.playhead()
        });
        if playing && playhead.is_some() {
            window.request_animation_frame();
        }
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
            let player = match self.review.view() {
                Some(review) => div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(review)
                    .into_any_element(),
                None => sections.player.clone().into_any_element(),
            };
            editor = editor
                .child(
                    div()
                        .flex_1()
                        .min_h(px(split.player_min_height))
                        .flex()
                        .child(player),
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
                                .child(
                                    self.timeline
                                        .clone()
                                        .cached(StyleRefinement::default().size_full()),
                                ),
                        )
                        // Over the cached timeline: only this moves while playing.
                        .child(crate::timeline::playhead_overlay(&self.theme, playhead))
                        .child(self.render_resize_grip(split.timeline_height, cx)),
                );
        } else {
            editor = editor.child(self.render_empty_state(cx));
        }
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
            .children(self.resize.map(|_| self.render_resize_tracking(cx)))
            .child(self.render_edit_tracking())
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
                    // Settings and export replace the whole row: the settings Agent page
                    // draws the same panel entity, which renders once per frame.
                    .child(if self.settings_open {
                        self.render_settings(cx)
                    } else if self.onboarding.is_some() {
                        self.render_onboarding(cx)
                    } else if self.export_panel.open {
                        self.render_export(cx)
                    } else {
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .gap(gutter)
                            .child(rail)
                            .child(editor)
                            .child(agent)
                            .into_any_element()
                    }),
            )
            .children(status_bar)
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

    /// The agent column's width: the one its grip asked for, or the default, fitted to the
    /// window.
    pub(super) fn agent_column_width(&self) -> f32 {
        self.agent_width_for(self.agent_width.unwrap_or(METRICS.agent_w))
    }

    fn agent_width_for(&self, preferred: f32) -> f32 {
        agent_width(
            self.viewport[0] - 2. * METRICS.gutter,
            self.drawer.is_some(),
            preferred,
        )
    }

    /// While a grip is held, follows the pointer through the whole window, including past
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
                            shell.resize = None;
                            cx.notify();
                            return;
                        }
                        match shell.resize {
                            Some(Resize::Timeline {
                                start_y,
                                start_height,
                            }) => {
                                let preferred =
                                    start_height - (f32::from(event.position.y) - start_y);
                                let height = shell.split_for(preferred).timeline_height;
                                if shell.timeline_height != Some(height) {
                                    shell.timeline_height = Some(height);
                                    cx.notify();
                                }
                            }
                            // The grip is on the column's left edge: moving left widens it.
                            Some(Resize::Agent {
                                start_x,
                                start_width,
                            }) => {
                                let preferred =
                                    start_width - (f32::from(event.position.x) - start_x);
                                let width = shell.agent_width_for(preferred);
                                if shell.agent_width != Some(width) {
                                    shell.agent_width = Some(width);
                                    cx.notify();
                                }
                            }
                            None => {}
                        }
                    });
                });
                let released = entity.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                        released.update(cx, |shell, cx| {
                            shell.resize = None;
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

    /// The handle on the agent column's left edge that drags its width.
    fn render_agent_grip(&self, width: f32, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .id("agent-resize-handle")
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(-11.))
            .w(px(14.))
            .cursor(CursorStyle::ResizeColumn)
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    shell.resize = Some(Resize::Agent {
                        start_x: f32::from(event.position.x),
                        start_width: width,
                    });
                    // Paints the tracker that listens for the release.
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .w(px(4.))
                    .h(px(36.))
                    .rounded(px(2.))
                    .bg(self.theme.editor.line),
            )
            .child(grip_probe(cx, |shell, measured| {
                shell.agent_grip_bounds = Some(measured)
            }))
            .into_any_element()
    }

    /// The handle above the timeline that drags its height, as in Cap's editor.
    fn render_resize_grip(&self, timeline_height: f32, cx: &mut Context<Self>) -> gpui::AnyElement {
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
                    shell.resize = Some(Resize::Timeline {
                        start_y: f32::from(event.position.y),
                        start_height: timeline_height,
                    });
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
            .child(grip_probe(cx, |shell, measured| {
                shell.grip_bounds = Some(measured)
            }))
            .into_any_element()
    }
}

/// Records where a grip was painted, `[left, top, width, height]`, for the telemetry.
fn grip_probe(
    cx: &mut Context<StudioShell>,
    store: impl Fn(&mut StudioShell, [f32; 4]) + 'static,
) -> impl IntoElement {
    let entity = cx.entity();
    canvas(
        move |bounds, _, cx| {
            let measured = [
                f32::from(bounds.origin.x),
                f32::from(bounds.origin.y),
                f32::from(bounds.size.width),
                f32::from(bounds.size.height),
            ];
            entity.update(cx, |shell, _| store(shell, measured));
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The grip being dragged and what it held at the press.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Resize {
    Timeline { start_y: f32, start_height: f32 },
    Agent { start_x: f32, start_width: f32 },
}

/// The agent column's width in a workspace row `available` wide. The rail, the open
/// drawer and the gaps take their fixed widths; the column takes `preferred`, at most
/// `agent_max_w` and at least `agent_min_w`, and leaves the editor `editor_min_w`. When
/// both minimums do not fit, they shrink in proportion, as the vertical split does.
pub(super) fn agent_width(available: f32, drawer_open: bool, preferred: f32) -> f32 {
    let mut fixed = METRICS.rail_w + 2. * METRICS.gutter;
    if drawer_open {
        fixed += METRICS.drawer_w + METRICS.gutter;
    }
    let space = (available - fixed).max(0.);
    let scale = (space / (METRICS.agent_min_w + METRICS.editor_min_w)).min(1.);
    let agent_min = METRICS.agent_min_w * scale;
    let agent_max = (space - METRICS.editor_min_w * scale)
        .min(METRICS.agent_max_w)
        .max(agent_min);
    preferred.clamp(agent_min, agent_max)
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

    /// The workspace row of a window `width` wide, inside the side gutters.
    fn row(width: f32) -> f32 {
        width - 2. * METRICS.gutter
    }

    #[test]
    fn the_agent_column_keeps_its_default_in_common_windows() {
        for width in [1280., 1536.] {
            assert_eq!(agent_width(row(width), false, METRICS.agent_w), 360.);
        }
        assert_eq!(agent_width(row(1536.), true, METRICS.agent_w), 360.);
    }

    #[test]
    fn the_agent_column_is_clamped_to_its_range_and_the_editor_minimum() {
        assert_eq!(agent_width(row(1920.), false, 100.), 320.);
        assert_eq!(agent_width(row(1920.), false, 900.), 520.);
        // 1264 - 56 - 16 - 320 - 8 leaves 864: the editor keeps 480 of it.
        assert_eq!(agent_width(row(1280.), true, 900.), 384.);
    }

    #[test]
    fn narrow_windows_shrink_the_agent_and_editor_minimums_together() {
        // 784 - 72 = 712 of 800: both minimums at 89 %.
        let width = agent_width(row(800.), false, METRICS.agent_w);
        assert!((width - 284.8).abs() < 0.01, "{width}");
        // The open drawer leaves 384: both minimums at 48 %.
        let width = agent_width(row(800.), true, METRICS.agent_w);
        assert!((width - 153.6).abs() < 0.01, "{width}");
        assert_eq!(agent_width(0., true, METRICS.agent_w), 0.);
    }

    #[test]
    fn no_space_leaves_no_timeline() {
        let split = vertical_split(-10., 260.);
        assert_eq!(split.player_min_height, 0.);
        assert_eq!(split.timeline_height, 0.);
    }
}
