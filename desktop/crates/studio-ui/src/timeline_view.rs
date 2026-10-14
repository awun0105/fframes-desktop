//! Native compiled tracks. Geometry/selection are engine-owned; textures are window-owned.
use gpui::{
    Bounds, Context, EventEmitter, FocusHandle, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, Render, StatefulInteractiveElement, Styled, Window, canvas, div, px,
};
use studio_engine::services::thumbnail_cache::{ThumbnailCache, ThumbnailKey};
use studio_engine::{PreviewFrame, TaskScope, TimelineModel, TimelineSelection, TimelineViewport};
use studio_theme::{
    METRICS, TextStyle, Theme, TrackKind, segment_colors_for, text_style, track_color,
};

use crate::frame_image::create_render_image;
use crate::kit::{
    EditorButton, Icon, IconButton, IconButtonSize, Slider, SliderTrack, focus_ring, icon,
    section_title,
};
use crate::probe::BoundsProbe;

#[derive(Debug, Clone)]
pub enum TimelineEvent {
    Seek(usize),
    Step(isize),
    TogglePlayback,
    ToggleMute,
    BeginScrub,
    EndScrub,
    ScopeChanged(Result<Option<Box<TaskScope>>, String>),
}

pub struct TimelineView {
    model: Option<TimelineModel>,
    viewport: TimelineViewport,
    selection: TimelineSelection,
    position: usize,
    focus: FocusHandle,
    bounds: Option<Bounds<Pixels>>,
    /// `[left, top, width, height]` of the Fit button, reported by its probe.
    fit_bounds: Option<[f32; 4]>,
    drag: Option<(usize, bool)>,
    cache: ThumbnailCache<gpui::RenderImage>,
    pending_thumbnail: Option<(ThumbnailKey, PreviewFrame)>,
    clear_cache: bool,
    release_failures: usize,
}
impl EventEmitter<TimelineEvent> for TimelineView {}

impl TimelineView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            model: None,
            viewport: TimelineViewport::default(),
            selection: TimelineSelection::default(),
            position: 0,
            focus: cx.focus_handle().tab_stop(true),
            bounds: None,
            fit_bounds: None,
            drag: None,
            cache: ThumbnailCache::default(),
            pending_thumbnail: None,
            clear_cache: false,
            release_failures: 0,
        }
    }
    pub fn install(&mut self, model: TimelineModel, position: usize, cx: &mut Context<Self>) {
        let first = self.model.is_none();
        let same_preview = self.model.as_ref().is_some_and(|current| {
            current.report().envelope.identity == model.report().envelope.identity
        });
        if same_preview {
            self.selection.clamp(&model);
        } else {
            self.selection = TimelineSelection::default();
        }
        self.viewport.resize(self.viewport.width, &model);
        if first {
            self.viewport.fit(&model);
        }
        self.position = position.min(model.report().total_frames);
        self.model = Some(model);
        self.drag = None;
        self.clear_cache = true;
        self.pending_thumbnail = None;
        self.emit_scope_changed(cx);
        cx.notify();
    }
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.model = None;
        self.selection = TimelineSelection::default();
        self.drag = None;
        self.position = 0;
        self.clear_cache = true;
        self.pending_thumbnail = None;
        self.emit_scope_changed(cx);
        cx.notify();
    }
    pub fn selected_scope(&self) -> Result<Option<TaskScope>, String> {
        self.model
            .as_ref()
            .map(|model| TaskScope::from_timeline(model.report(), &self.selection).map(Some))
            .unwrap_or(Ok(None))
            .map_err(|error| error.to_string())
    }
    fn emit_scope_changed(&self, cx: &mut Context<Self>) {
        let scope = self.selected_scope().map(|scope| scope.map(Box::new));
        cx.emit(TimelineEvent::ScopeChanged(scope));
    }
    pub fn set_position(&mut self, frame: usize, cx: &mut Context<Self>) {
        if self.position != frame {
            self.position = frame;
            cx.notify();
        }
    }
    pub fn thumbnail_requests(&self) -> Vec<ThumbnailKey> {
        let Some(model) = &self.model else {
            return vec![];
        };
        let scale = thumbnail_scale(model);
        self.viewport
            .thumbnail_frames(model)
            .into_iter()
            .map(|frame| ThumbnailKey::new(model.report().envelope.identity.clone(), scale, frame))
            .filter(|key| !self.cache.contains(key))
            .collect()
    }
    pub fn qualification_metrics(&self) -> serde_json::Value {
        // The ruler covers the time axis only: the track gutter sits to its left.
        let ruler = |b: Bounds<Pixels>| {
            [
                f32::from(b.left()) + axis_inset(),
                f32::from(b.top()),
                (f32::from(b.size.width) - axis_inset()).max(0.),
                METRICS.ruler_h,
            ]
        };
        serde_json::json!({
            "entries": self.cache.len(), "bytes": self.cache.bytes(),
            "high_water_entries": self.cache.high_water_entries(), "high_water_bytes": self.cache.high_water_bytes(),
            "release_failures": self.release_failures,
            "pixels_per_second": self.viewport.pixels_per_second, "scroll_x": self.viewport.scroll_x,
            "selection": {
                "scene": self.selection.scene_id,
                "range": self.selection.range.as_ref().map(|r| [r.start, r.end]),
                "position": self.position,
                "total_frames": self.model.as_ref().map(|m| m.report().total_frames),
            },
            "ruler_bounds": self.bounds.map(ruler),
            "fit_bounds": self.fit_bounds,
        })
    }
    pub fn thumbnail(&mut self, key: ThumbnailKey, frame: PreviewFrame, cx: &mut Context<Self>) {
        if self.model.as_ref().is_some_and(|model| {
            key.identity == model.report().envelope.identity
                && key.scale_bits == thumbnail_scale(model).to_bits()
                && key.frame_index == frame.response.frame_index
                && frame.response.scale.to_bits() == key.scale_bits
                && frame.validate(&key.identity).is_ok()
                && self
                    .viewport
                    .thumbnail_frames(model)
                    .contains(&key.frame_index)
        }) {
            self.pending_thumbnail = Some((key, frame));
            cx.notify();
        }
    }
    /// Moves the keyboard focus to the tracks, as a click on them does.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
    }
    /// Moves the playhead to `frame` (as a click on the ruler does).
    pub fn seek(&mut self, frame: usize, cx: &mut Context<Self>) {
        if self
            .model
            .as_ref()
            .is_some_and(|m| m.report().total_frames > 0)
        {
            cx.emit(TimelineEvent::Seek(frame));
        }
    }
    fn pointer_frame(&self, x: Pixels) -> Option<usize> {
        let axis_x = f32::from(x - self.bounds?.left()) - axis_inset();
        if axis_x < 0. {
            return None;
        }
        self.viewport
            .frame_at_x(f64::from(axis_x), self.model.as_ref()?)
    }
    fn pointer_down(
        &mut self,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(frame) = self.pointer_frame(event.position.x) else {
            return;
        };
        window.focus(&self.focus, cx);
        let range = event.modifiers.shift
            || self.bounds.is_some_and(|b| {
                hit_zone(f32::from(event.position.y), f32::from(b.top())) == HitZone::Lanes
            });
        self.drag = Some((frame, range));
        if range {
            self.selection
                .select_range(frame, frame, self.model.as_ref().unwrap());
            self.emit_scope_changed(cx);
        } else {
            cx.emit(TimelineEvent::BeginScrub);
            self.seek(frame, cx);
        }
        cx.notify();
    }
    fn pointer_move(&mut self, event: &gpui::MouseMoveEvent, cx: &mut Context<Self>) {
        if !event.dragging() {
            return;
        }
        let (Some((start, range)), Some(frame)) = (self.drag, self.pointer_frame(event.position.x))
        else {
            return;
        };
        if range {
            self.selection
                .select_range(start, frame, self.model.as_ref().unwrap());
            self.emit_scope_changed(cx);
            cx.notify();
        } else {
            self.seek(frame, cx);
        }
    }
    fn pointer_up(&mut self, cx: &mut Context<Self>) {
        if let Some((_, false)) = self.drag.take() {
            cx.emit(TimelineEvent::EndScrub);
        }
    }
    /// Moves the playhead past the last frame.
    pub fn seek_to_end(&mut self, cx: &mut Context<Self>) {
        if let Some(total) = self.model.as_ref().map(|m| m.report().total_frames) {
            self.seek(total, cx);
        }
    }
    /// Selects the next scene under the playhead.
    pub fn cycle_scene(&mut self, cx: &mut Context<Self>) {
        let Some(model) = &self.model else {
            return;
        };
        self.selection.cycle_scene_at(self.position, model);
        self.emit_scope_changed(cx);
        cx.notify();
    }
    /// Starts the selected range at the playhead (keeping its end, if any).
    pub fn mark_range_start(&mut self, cx: &mut Context<Self>) {
        let Some(model) = &self.model else {
            return;
        };
        let end = self
            .selection
            .range
            .as_ref()
            .map_or(self.position, |r| r.end);
        self.selection.select_range(self.position, end, model);
        self.emit_scope_changed(cx);
        cx.notify();
    }
    /// Ends the selected range at the playhead (keeping its start, if any).
    pub fn mark_range_end(&mut self, cx: &mut Context<Self>) {
        let Some(model) = &self.model else {
            return;
        };
        let start = self
            .selection
            .range
            .as_ref()
            .map_or(self.position, |r| r.start);
        self.selection.select_range(start, self.position, model);
        self.emit_scope_changed(cx);
        cx.notify();
    }
    /// Zooms by `factor`, keeping the playhead where it is on screen.
    pub fn zoom(&mut self, factor: f64, cx: &mut Context<Self>) {
        if let Some(model) = &self.model {
            self.viewport.zoom(
                factor,
                self.viewport.x_at_frame(self.position, model),
                model,
            );
            cx.notify();
        }
    }
    fn key(&mut self, event: &gpui::KeyDownEvent, cx: &mut Context<Self>) {
        let Some(model) = &self.model else {
            return;
        };
        match event.keystroke.key.as_str() {
            "space" => cx.emit(TimelineEvent::TogglePlayback),
            "m" => cx.emit(TimelineEvent::ToggleMute),
            "left" => cx.emit(TimelineEvent::Step(-1)),
            "right" => cx.emit(TimelineEvent::Step(1)),
            "home" => self.seek(0, cx),
            "end" => self.seek_to_end(cx),
            "+" | "=" => self.zoom(2., cx),
            "-" => self.zoom(0.5, cx),
            "pageup" => {
                self.viewport.scroll_by(-self.viewport.width * 0.75, model);
                cx.notify();
            }
            "pagedown" => {
                self.viewport.scroll_by(self.viewport.width * 0.75, model);
                cx.notify();
            }
            "s" => self.cycle_scene(cx),
            "[" => self.mark_range_start(cx),
            "]" => self.mark_range_end(cx),
            _ => return,
        }
        cx.stop_propagation();
    }
    fn action(
        entity: &gpui::Entity<Self>,
        run: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static {
        let entity = entity.clone();
        move |_, _, cx| entity.update(cx, |view, cx| run(view, cx))
    }

    /// A small toolbar icon button: `label` names it, `tooltip` explains it.
    fn icon_button(
        theme: &Theme,
        id: &'static str,
        glyph: Icon,
        label: &'static str,
        tooltip: &'static str,
        enabled: bool,
        on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    ) -> IconButton {
        IconButton::editor(theme, id, glyph.path())
            .editor_size(IconButtonSize::Small)
            .label(label)
            .tooltip(theme, tooltip)
            .disabled(!enabled)
            .on_click(on_click)
    }
    /// Zoom to the slider `fraction`, keeping the playhead where it is on screen.
    fn set_zoom_fraction(&mut self, fraction: f32, cx: &mut Context<Self>) {
        let target = fraction_to_zoom(fraction);
        if self.viewport.pixels_per_second > 0. {
            self.zoom(target / self.viewport.pixels_per_second, cx);
        }
    }
}

fn thumbnail_scale(model: &TimelineModel) -> f64 {
    (160. / model.report().width as f64)
        .min(90. / model.report().height as f64)
        .min(1.)
}

/// Zoom range of `TimelineViewport` (pixels per second); the viewport clamps to it.
const ZOOM_MIN: f64 = 0.01;
const ZOOM_MAX: f64 = 100_000.;
/// Left inset of the segment label and the room kept on its right.
const LABEL_PAD_LEFT: f32 = 13.;
const LABEL_PAD_RIGHT: f32 = 10.;
const ACCENT_BAR_W: f32 = 3.;
const PLAYHEAD_HEAD: f32 = 12.;
const PLAYHEAD_RING: f32 = 4.;
const MAJOR_TICK_H: f32 = 8.;
const MINOR_TICK_H: f32 = 5.;
const MINOR_PER_MAJOR: usize = 5;
const ZOOM_SLIDER_W: f32 = 72.;
const ZOOM_TRACK_H: f32 = 3.;
const ZOOM_THUMB: f32 = 14.;
const CARD_PAD: f32 = 11.;
const CARD_PAD_TOP: f32 = 9.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitZone {
    Ruler,
    Lanes,
}

/// Which part of the timeline a pointer at window `y` hits, given the top of the
/// timeline area. The ruler scrubs, the lanes select.
fn hit_zone(y: f32, top: f32) -> HitZone {
    if y - top < METRICS.ruler_h {
        HitZone::Ruler
    } else {
        HitZone::Lanes
    }
}

/// Width left of the time axis: the track gutter plus its gap.
fn axis_inset() -> f32 {
    let metrics = METRICS;
    metrics.track_gutter_w + metrics.space_2
}

/// Zoom (pixels per second) as a 0..=1 slider fraction on a log scale.
fn zoom_to_fraction(pixels_per_second: f64) -> f32 {
    ((pixels_per_second.clamp(ZOOM_MIN, ZOOM_MAX) / ZOOM_MIN).ln() / (ZOOM_MAX / ZOOM_MIN).ln())
        as f32
}

/// Inverse of [`zoom_to_fraction`].
fn fraction_to_zoom(fraction: f32) -> f64 {
    ZOOM_MIN * (ZOOM_MAX / ZOOM_MIN).powf(f64::from(fraction.clamp(0., 1.)))
}

/// Where a segment label sits: `left` from the segment's left edge, `width` available.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LabelSlice {
    left: f32,
    width: f32,
}

/// Anchors a label to the part of a segment (at `x`, `width` wide) that is inside
/// a viewport `view_width` wide.
fn label_slice(x: f32, width: f32, view_width: f32) -> LabelSlice {
    let visible_start = (-x).clamp(0., width);
    let visible_end = (view_width - x).min(width).max(visible_start);
    LabelSlice {
        left: visible_start + LABEL_PAD_LEFT,
        width: (visible_end - visible_start - LABEL_PAD_LEFT - LABEL_PAD_RIGHT).max(0.),
    }
}

/// Minor tick positions between the major ticks at `majors` (x, ascending).
fn minor_tick_xs(majors: &[f64], view_width: f64) -> Vec<f64> {
    let [first, second, ..] = majors else {
        return Vec::new();
    };
    let step = (second - first) / MINOR_PER_MAJOR as f64;
    if !(step.is_finite() && step > 1.) {
        return Vec::new();
    }
    let start = first - (first / step).floor().max(0.) * step;
    let count = ((view_width - start) / step).floor().max(0.) as usize + 1;
    (0..count)
        .map(|index| start + index as f64 * step)
        .filter(|x| majors.iter().all(|major| (major - x).abs() > 0.5))
        .collect()
}

fn lane_name(base: &str, index: usize, count: usize) -> String {
    if count > 1 {
        format!("{base} {}", index + 1)
    } else {
        base.to_owned()
    }
}

/// A track row: gutter with icon and name, then the clipped band for segments.
fn track_row(
    theme: &Theme,
    top: f32,
    name: String,
    glyph: Icon,
    segments: Vec<gpui::AnyElement>,
) -> gpui::Div {
    let metrics = METRICS;
    div()
        .absolute()
        .left_0()
        .right_0()
        .top(px(top))
        .h(px(metrics.track_h))
        .flex()
        .items_center()
        .gap(px(metrics.space_2))
        .child(
            text_style(div(), TextStyle::Small)
                .font_weight(gpui::FontWeight::MEDIUM)
                .w(px(metrics.track_gutter_w))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.))
                .overflow_hidden()
                .text_color(theme.editor.text_2)
                .child(icon(glyph, metrics.icon_sm, theme.editor.text_2))
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(name),
                ),
        )
        .child(
            div()
                .relative()
                .flex_1()
                .min_w_0()
                .h_full()
                .overflow_hidden()
                .rounded(px(metrics.radius_segment))
                .bg(theme.editor.card_2)
                .children(segments),
        )
}

/// Segment box with its accent bar and clipped label. The caller adds ids and handlers.
fn segment(
    theme: &Theme,
    kind: TrackKind,
    selected: bool,
    x: f32,
    width: f32,
    view_width: f32,
    label: String,
) -> gpui::Div {
    let metrics = METRICS;
    let colors = segment_colors_for(theme.appearance, kind, selected);
    let slice = label_slice(x, width, view_width);
    div()
        .absolute()
        .left(px(x))
        .top_0()
        .w(px(width))
        .h(px(metrics.track_h))
        .overflow_hidden()
        .rounded(px(metrics.radius_segment))
        .border_1()
        .border_color(colors.border)
        .bg(colors.fill)
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(ACCENT_BAR_W))
                .bg(track_color(kind)),
        )
        .child(
            text_style(div(), TextStyle::Small)
                .absolute()
                .left(px(slice.left))
                .top_0()
                .bottom_0()
                .w(px(slice.width))
                .flex()
                .items_center()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(colors.label)
                .child(label),
        )
}

impl Render for TimelineView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.clear_cache {
            for image in self.cache.clear() {
                self.release_failures += usize::from(window.drop_image(image).is_err());
            }
            self.clear_cache = false;
        }
        if let Some((key, frame)) = self.pending_thumbnail.take()
            && let Ok(image) = create_render_image(&frame.response.header, &frame.pixels)
        {
            let bytes =
                frame.response.header.width as usize * frame.response.header.height as usize * 4;
            for image in self.cache.insert(key, image, bytes) {
                self.release_failures += usize::from(window.drop_image(image).is_err());
            }
        }
        let theme = Theme::for_window(window, cx);
        let colors = theme.editor;
        let metrics = METRICS;
        let enabled = self
            .model
            .as_ref()
            .is_some_and(|m| m.report().total_frames > 0);
        let entity = cx.entity();
        let zoom_fraction = zoom_to_fraction(self.viewport.pixels_per_second);
        let toolbar = section_title(&theme, "Timeline").child(
            div()
                .flex()
                .items_center()
                .gap(px(metrics.space_1))
                .child(Self::icon_button(
                    &theme,
                    "timeline-zoom-out",
                    Icon::ZoomOut,
                    "−",
                    "Zoom out",
                    enabled,
                    Self::action(&entity, |v, cx| v.zoom(0.5, cx)),
                ))
                .child(
                    // The old slider's half-thumb inset, so the knob stays inside the 72px.
                    div()
                        .flex()
                        .w(px(ZOOM_SLIDER_W))
                        .px(px(ZOOM_THUMB / 2.))
                        .child(
                            Slider::new(
                                "timeline-zoom-slider",
                                zoom_fraction,
                                SliderTrack::default(),
                            )
                            .flex()
                            .track(px(ZOOM_TRACK_H), colors.ctl_active.into())
                            .fill(colors.accent.into())
                            .thumb(px(ZOOM_THUMB), colors.thumb.into(), None)
                            .thumb_shadow()
                            .label("Timeline zoom")
                            .disabled(!enabled)
                            .on_change(0., {
                                let entity = entity.clone();
                                move |fraction, _, cx| {
                                    entity.update(cx, |v, cx| v.set_zoom_fraction(*fraction, cx));
                                }
                            }),
                        ),
                )
                .child(Self::icon_button(
                    &theme,
                    "timeline-zoom-in",
                    Icon::ZoomIn,
                    "+",
                    "Zoom in",
                    enabled,
                    Self::action(&entity, |v, cx| v.zoom(2., cx)),
                ))
                .child(
                    EditorButton::plain(&theme, "timeline-fit")
                        .label("Fit")
                        .filled(&theme)
                        .disabled(!enabled)
                        .on_bounds(
                            BoundsProbe::new({
                                let entity = entity.clone();
                                move |rect, cx| entity.update(cx, |v, _| v.fit_bounds = Some(rect))
                            })
                            .into_bounds_handler(),
                        )
                        .on_click(Self::action(&entity, |v, cx| {
                            if let Some(m) = &v.model {
                                v.viewport.fit(m);
                                cx.notify();
                            }
                        })),
                )
                .child(Self::icon_button(
                    &theme,
                    "timeline-cycle",
                    Icon::Layers,
                    "Cycle scene (S)",
                    "Cycle scene (S)",
                    enabled,
                    Self::action(&entity, |v, cx| {
                        if let Some(m) = &v.model {
                            v.selection.cycle_scene_at(v.position, m);
                            v.emit_scope_changed(cx);
                            cx.notify();
                        }
                    }),
                )),
        );
        let root = div()
            .flex()
            .flex_col()
            .gap(px(metrics.space_1 + 2.))
            .min_w_0()
            .flex_shrink_0()
            .h(px(metrics.timeline_default_h - 2.))
            .pl(px(CARD_PAD))
            .pr(px(CARD_PAD))
            .pt(px(CARD_PAD_TOP))
            .pb(px(CARD_PAD))
            .child(toolbar);
        let Some(model) = &self.model else {
            return root.child(
                text_style(div(), TextStyle::Body)
                    .text_color(colors.text_2)
                    .child("No compiled timeline available."),
            );
        };
        let inset = axis_inset();
        let view_width = self.viewport.width as f32;
        let geometry = self.viewport.geometry(model);
        let weak = cx.entity().downgrade();
        let mut tracks = div()
            .id("compiled-timeline")
            .track_focus(&self.focus)
            .role(gpui::Role::Group)
            .aria_label("Compiled timeline: ruler scrubs, tracks select ranges")
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .rounded(px(metrics.radius_segment))
            .focus_visible(|s| s.shadow(focus_ring(&theme)))
            .on_key_down(cx.listener(|view, event, _, cx| view.key(event, cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|v, e, w, cx| v.pointer_down(e, w, cx)),
            )
            .on_mouse_move(cx.listener(|v, e, _, cx| v.pointer_move(e, cx)))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|v, _, _, cx| v.pointer_up(cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|v, _, _, cx| v.pointer_up(cx)),
            )
            .on_scroll_wheel(cx.listener(|v, e: &gpui::ScrollWheelEvent, _, cx| {
                let Some(model) = &v.model else {
                    return;
                };
                let delta = match e.delta {
                    gpui::ScrollDelta::Pixels(p) => f32::from(p.x) + f32::from(p.y),
                    gpui::ScrollDelta::Lines(p) => (p.x + p.y) * 32.,
                } as f64;
                if e.modifiers.control {
                    let anchor = v.bounds.map_or(0., |b| {
                        f32::from(e.position.x - b.left() - px(axis_inset())) as f64
                    });
                    v.viewport.zoom((delta * 0.01).exp(), anchor.max(0.), model);
                } else {
                    v.viewport.scroll_by(-delta, model);
                }
                cx.notify();
                cx.stop_propagation();
            }))
            .child(
                canvas(
                    move |bounds, _, cx| {
                        let _ = weak.update(cx, |view, cx| {
                            view.bounds = Some(bounds);
                            if let Some(model) = &view.model {
                                let width =
                                    (f32::from(bounds.size.width) - axis_inset()).max(0.) as f64;
                                if (view.viewport.width - width).abs() > 0.5 {
                                    let first = view.viewport.width == 0.;
                                    view.viewport.resize(width, model);
                                    if first {
                                        view.viewport.fit(model);
                                    }
                                    cx.notify();
                                }
                            }
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            );

        // Ruler: major ticks carry the labels, minor ticks subdivide them.
        let major_xs: Vec<f64> = geometry.ticks.iter().map(|tick| tick.x).collect();
        let mut ruler = div()
            .absolute()
            .left(px(inset))
            .right_0()
            .top_0()
            .h(px(metrics.ruler_h))
            .overflow_hidden();
        for x in minor_tick_xs(&major_xs, self.viewport.width) {
            ruler = ruler.child(
                div()
                    .absolute()
                    .left(px(x as f32))
                    .bottom_0()
                    .w(px(1.))
                    .h(px(MINOR_TICK_H))
                    .bg(colors.line_strong),
            );
        }
        for tick in geometry.ticks {
            let x = tick.x as f32;
            ruler = ruler
                .child(
                    div()
                        .absolute()
                        .left(px(x))
                        .bottom_0()
                        .w(px(1.))
                        .h(px(MAJOR_TICK_H))
                        .bg(colors.line_strong),
                )
                .child(
                    text_style(div(), TextStyle::Small)
                        .absolute()
                        .left(px(x + 4.))
                        .top(px(3.))
                        .whitespace_nowrap()
                        .text_color(colors.text_3)
                        .child(tick.label),
                );
        }
        tracks = tracks.child(ruler);

        // Lanes scroll vertically independently of the ruler. Only horizontal visible geometry is rendered.
        let scene_lanes = geometry.scene_lane_count.max(1);
        let audio_count = model.report().audio_tracks.len();
        let row_top = |row: usize| row as f32 * (metrics.track_h + metrics.track_gap);
        let mut scene_rows: Vec<Vec<gpui::AnyElement>> =
            (0..scene_lanes).map(|_| Vec::new()).collect();
        let mut audio_rows: Vec<Vec<gpui::AnyElement>> =
            (0..audio_count).map(|_| Vec::new()).collect();
        for rect in geometry.scenes {
            let scene = &model.report().scenes[rect.index];
            if scene.start_frame == scene.end_frame {
                continue;
            }
            let selected = self.selection.scene_id.as_ref() == Some(&scene.instance_id);
            let index = rect.index;
            let kind = if rect.lane == 0 {
                TrackKind::Clip
            } else {
                TrackKind::Overlay
            };
            let element = segment(
                &theme,
                kind,
                selected,
                rect.x as f32,
                rect.width.max(1.) as f32,
                view_width,
                format!("{} #{}", scene.name, scene.index),
            )
            .id(format!("scene-{}", scene.instance_id))
            .role(gpui::Role::Button)
            .aria_label(format!("Scene {} instance {}", scene.name, scene.index))
            .tab_index(0)
            .focus_visible(move |s| s.border_color(colors.accent))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v, e: &gpui::MouseDownEvent, w, cx| {
                    if e.modifiers.shift {
                        return;
                    }
                    if let Some(m) = &v.model {
                        v.selection.select_scene(index, m);
                    }
                    v.emit_scope_changed(cx);
                    w.focus(&v.focus, cx);
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .on_key_down(cx.listener(move |v, e: &gpui::KeyDownEvent, _, cx| {
                if e.keystroke.key == "enter" {
                    if let Some(m) = &v.model {
                        v.selection.select_scene(index, m);
                    }
                    v.emit_scope_changed(cx);
                    cx.notify();
                    cx.stop_propagation();
                }
            }));
            scene_rows[rect.lane.min(scene_lanes - 1)].push(element.into_any_element());
        }
        for rect in geometry.audio {
            let audio = &model.report().audio_tracks[rect.index];
            let element = segment(
                &theme,
                TrackKind::Audio,
                false,
                rect.x as f32,
                rect.width.max(1.) as f32,
                view_width,
                format!("♫ {} · {:.3}s", audio.file, audio.start_seconds),
            );
            audio_rows[rect.lane.min(audio_count - 1)].push(element.into_any_element());
        }
        let total_rows = scene_lanes + audio_count;
        let mut content = div().relative().h(px(
            (row_top(total_rows) - metrics.track_gap).max(metrics.track_h)
        ));
        for (lane, segments) in scene_rows.into_iter().enumerate() {
            content = content.child(track_row(
                &theme,
                row_top(lane),
                lane_name("Scenes", lane, scene_lanes),
                Icon::Layers,
                segments,
            ));
        }
        for (track, segments) in audio_rows.into_iter().enumerate() {
            content = content.child(track_row(
                &theme,
                row_top(scene_lanes + track),
                lane_name("Audio", track, audio_count),
                Icon::Volume2,
                segments,
            ));
        }
        let lanes = div()
            .id("timeline-lanes")
            .absolute()
            .top(px(metrics.ruler_h))
            .bottom_0()
            .w_full()
            .overflow_y_scroll()
            .child(content);
        tracks = tracks.child(lanes);

        // Selection range and playhead share one clip box over the time axis.
        let mut overlay = div()
            .absolute()
            .left(px(inset))
            .right_0()
            .top_0()
            .bottom_0()
            .overflow_hidden();
        if let Some(range) = &self.selection.range {
            let start = self.viewport.x_at_frame(range.start, model);
            let end = self.viewport.x_at_frame(range.end, model);
            overlay = overlay.child(
                div()
                    .absolute()
                    .left(px(start as f32))
                    .top(px(metrics.ruler_h))
                    .bottom_0()
                    .w(px((end - start) as f32))
                    .bg(theme.selection())
                    .border_l_1()
                    .border_r_1()
                    .border_color(colors.accent),
            );
        }
        let playhead_x = self.viewport.x_at_frame(self.position, model) as f32;
        let ring = PLAYHEAD_HEAD + 2. * PLAYHEAD_RING;
        overlay = overlay
            .child(
                div()
                    .absolute()
                    .left(px(playhead_x))
                    .top_0()
                    .bottom_0()
                    .w(px(1.))
                    .bg(colors.playhead),
            )
            .child(
                div()
                    .absolute()
                    .left(px(playhead_x + 0.5 - ring / 2.))
                    .top(px((metrics.ruler_h - ring) / 2.))
                    .size(px(ring))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(colors.card)
                    .child(
                        div()
                            .size(px(PLAYHEAD_HEAD))
                            .rounded_full()
                            .bg(colors.playhead),
                    ),
            );
        tracks = tracks.child(overlay);

        let mut thumbs = div().relative().flex_none().h(px(38.)).overflow_hidden();
        for frame in self.viewport.thumbnail_frames(model) {
            let key = ThumbnailKey::new(
                model.report().envelope.identity.clone(),
                thumbnail_scale(model),
                frame,
            );
            if let Some(image) = self.cache.get(&key) {
                thumbs = thumbs.child(
                    div()
                        .absolute()
                        .left(px(self.viewport.x_at_frame(frame, model) as f32 + inset))
                        .w(px(64.))
                        .h(px(36.))
                        .overflow_hidden()
                        .child(gpui::img(image).w(px(64.)).h(px(36.))),
                );
            }
        }
        let selected = self
            .selection
            .range
            .as_ref()
            .map_or("No range selected".into(), |r| {
                format!(
                    "Range [{}..{}){}",
                    r.start,
                    r.end,
                    self.selection
                        .scene_id
                        .as_ref()
                        .map_or(String::new(), |_| " · scene selected".into())
                )
            });
        root.child(tracks).child(thumbs).child(
            text_style(div(), TextStyle::Small)
                .flex_none()
                .text_color(colors.text_3)
                .child(format!(
                    "{} · {selected} · drag tracks / Shift-drag for range",
                    model.time_label(self.position)
                )),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruler_click_scrubs_and_lane_click_selects() {
        let ruler_h = METRICS.ruler_h;
        assert_eq!(ruler_h, 26.);
        assert_eq!(hit_zone(100., 100.), HitZone::Ruler);
        assert_eq!(hit_zone(100. + ruler_h - 0.5, 100.), HitZone::Ruler);
        assert_eq!(hit_zone(100. + ruler_h, 100.), HitZone::Lanes);
        assert_eq!(hit_zone(300., 100.), HitZone::Lanes);
    }

    #[test]
    fn zoom_slider_maps_to_zoom_range_and_back() {
        assert_eq!(zoom_to_fraction(ZOOM_MIN), 0.);
        assert_eq!(zoom_to_fraction(ZOOM_MAX), 1.);
        assert_eq!(zoom_to_fraction(ZOOM_MIN / 10.), 0.);
        assert_eq!(zoom_to_fraction(ZOOM_MAX * 10.), 1.);
        assert_eq!(fraction_to_zoom(-1.), ZOOM_MIN);
        for pps in [0.5, 12., 100., 4_000.] {
            let back = fraction_to_zoom(zoom_to_fraction(pps));
            assert!((back / pps - 1.).abs() < 1e-3, "{pps} -> {back}");
        }
        assert!(fraction_to_zoom(0.25) < fraction_to_zoom(0.75));
    }

    #[test]
    fn segment_label_anchors_to_visible_slice() {
        let inner = LABEL_PAD_LEFT + LABEL_PAD_RIGHT;
        // Fully visible: label starts after the left padding.
        let whole = label_slice(10., 100., 500.);
        assert_eq!(whole.left, LABEL_PAD_LEFT);
        assert_eq!(whole.width, 100. - inner);
        // Scrolled past the left edge: label follows the visible part.
        let left_cut = label_slice(-50., 100., 500.);
        assert_eq!(left_cut.left, 50. + LABEL_PAD_LEFT);
        assert_eq!(left_cut.width, 50. - inner);
        // Cut by the right edge of the viewport.
        let right_cut = label_slice(450., 100., 500.);
        assert_eq!(right_cut.left, LABEL_PAD_LEFT);
        assert_eq!(right_cut.width, 50. - inner);
        // Too little visible: no width, never negative.
        assert_eq!(label_slice(490., 100., 500.).width, 0.);
        // Longer than the viewport on both sides.
        let both = label_slice(-1000., 5000., 400.);
        assert_eq!(both.left, 1000. + LABEL_PAD_LEFT);
        assert_eq!(both.width, 400. - inner);
    }

    #[test]
    fn minor_ticks_subdivide_majors_without_overlap() {
        assert!(minor_tick_xs(&[], 300.).is_empty());
        assert!(minor_tick_xs(&[10.], 300.).is_empty());
        let minors = minor_tick_xs(&[20., 120., 220.], 300.);
        assert!(minors.iter().all(|x| (0. ..=300.).contains(x)));
        assert!(!minors.iter().any(|x| [20., 120., 220.].contains(x)));
        assert_eq!(minors.iter().filter(|x| **x > 20. && **x < 120.).count(), 4);
    }

    #[gpui::test]
    fn the_fit_button_reports_its_bounds_for_qualification(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
        cx.run_until_parked();
        let fit = view.read_with(cx, |view, _| view.fit_bounds);
        let [_, _, width, height] = fit.expect("the Fit button reports where it is");
        assert!(width > 0. && height > 0., "{fit:?}");
    }

    #[test]
    fn axis_inset_is_gutter_plus_gap() {
        assert_eq!(axis_inset(), 112.);
        assert_eq!(lane_name("Audio", 0, 1), "Audio");
        assert_eq!(lane_name("Audio", 1, 2), "Audio 2");
    }
}
