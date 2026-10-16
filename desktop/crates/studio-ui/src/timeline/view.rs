//! The timeline entity: Cap's transform (engine-owned), ruler, clip track, overlay and
//! audio rows, the mix waveform and the minimap, and the clip edits: trim handles, drag to
//! reorder and split at the playhead. Geometry and selection are engine-owned, thumbnail
//! textures window-owned. The shell mounts it cached and paints the playhead.
use std::sync::Arc;

use gpui::{
    Bounds, Context, EventEmitter, FocusHandle, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, Render, StatefulInteractiveElement, Styled, Window, canvas, div,
    prelude::FluentBuilder, px,
};
use studio_engine::services::thumbnail_cache::{ThumbnailCache, ThumbnailKey};
use studio_engine::timing::{ResolvedClip, TimelineCommand, TimelineSnapshot, TrimEdge};
use studio_engine::{
    PreviewFrame, TaskScope, TimelineModel, TimelineSelection, TimelineTransform, TimingEditing,
};
use studio_theme::{METRICS, TextStyle, Theme, TrackKind, text_style, track_color};

use super::edit::{
    DRAG_THRESHOLD_PX, SnapTargets, drop_marker, drop_position, split_at, trim_for_edge,
};
use super::minimap::{MINIMAP_MAX_WIDTH, MinimapDrag, render_minimap, zoomed_in};
use super::model::{ClipSegment, clip_segments};
use super::playhead::PlayheadGeometry;
use super::ruler::render_ruler;
use super::segment::{lane_name, row_top, segment, track_row};
use super::waveform::waveform_path;
use crate::frame_image::create_render_image;
use crate::kit::{
    EditorButton, Icon, IconButton, IconButtonSize, Slider, SliderTrack, focus_ring, section_title,
    selectable,
};
use crate::probe::BoundsProbe;

/// The tracks' key context: the shell binds the timeline's keys in it.
pub const KEY_CONTEXT: &str = "Timeline";

#[derive(Debug, Clone)]
pub enum TimelineEvent {
    Seek(usize),
    BeginScrub,
    EndScrub,
    ScopeChanged(Result<Option<Box<TaskScope>>, String>),
    /// An edit of the timing document, for the engine.
    Edit(TimelineCommand),
}

pub struct TimelineView {
    model: Option<TimelineModel>,
    /// The engine's clips; the clip track follows them when present.
    snapshot: Option<TimelineSnapshot>,
    clips: Vec<ClipSegment>,
    /// Peaks of the displayed preview's mix, see `studio_engine::waveform`.
    waveform: Arc<[f32]>,
    viewport: TimelineTransform,
    selection: TimelineSelection,
    /// The playhead frame. Stored without a notify: the shell paints the playhead.
    position: usize,
    focus: FocusHandle,
    bounds: Option<Bounds<Pixels>>,
    /// `[left, top, width, height]` of the Fit button, reported by its probe.
    fit_bounds: Option<[f32; 4]>,
    minimap_bounds: Option<Bounds<Pixels>>,
    minimap_drag: Option<MinimapDrag>,
    drag: Option<(usize, bool)>,
    /// A trim or reorder drag of a clip.
    edit: Option<EditDrag>,
    /// Segments built by the last render, after culling.
    painted_segments: usize,
    /// How often the view rendered; playback must not render it.
    renders: u64,
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
            snapshot: None,
            clips: Vec::new(),
            waveform: Arc::from([]),
            viewport: TimelineTransform::default(),
            selection: TimelineSelection::default(),
            position: 0,
            focus: cx.focus_handle().tab_stop(true),
            bounds: None,
            fit_bounds: None,
            minimap_bounds: None,
            minimap_drag: None,
            drag: None,
            edit: None,
            painted_segments: 0,
            renders: 0,
            cache: ThumbnailCache::default(),
            pending_thumbnail: None,
            clear_cache: false,
            release_failures: 0,
        }
    }
    pub fn install(
        &mut self,
        model: TimelineModel,
        waveform: Arc<[f32]>,
        position: usize,
        cx: &mut Context<Self>,
    ) {
        let first = self.model.is_none();
        let same_preview = self.model.as_ref().is_some_and(|current| {
            current.report().envelope.identity == model.report().envelope.identity
        });
        if same_preview {
            self.selection.clamp(&model);
        } else {
            self.selection = TimelineSelection::default();
            // Another build: the dragged clip belongs to a layout that is gone.
            self.abandon_edit_drag(cx);
        }
        self.viewport.resize(self.viewport.width, &model);
        if first {
            self.viewport.fit(&model);
        }
        self.position = position.min(model.report().total_frames);
        self.model = Some(model);
        self.waveform = waveform;
        self.refresh_clips();
        self.drag = None;
        self.minimap_drag = None;
        self.clear_cache = true;
        self.pending_thumbnail = None;
        self.emit_scope_changed(cx);
        cx.notify();
    }
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.abandon_edit_drag(cx);
        self.model = None;
        self.waveform = Arc::from([]);
        self.refresh_clips();
        self.selection = TimelineSelection::default();
        self.drag = None;
        self.minimap_drag = None;
        self.position = 0;
        self.clear_cache = true;
        self.pending_thumbnail = None;
        self.emit_scope_changed(cx);
        cx.notify();
    }
    /// The engine's timing document and clips, from the presentation.
    pub fn set_snapshot(&mut self, snapshot: Option<TimelineSnapshot>, cx: &mut Context<Self>) {
        if self.snapshot != snapshot {
            self.snapshot = snapshot;
            self.refresh_clips();
            cx.notify();
        }
    }
    fn refresh_clips(&mut self) {
        self.clips = self
            .model
            .as_ref()
            .map(|model| clip_segments(model, self.snapshot.as_ref()))
            .unwrap_or_default();
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
    /// Moves the playhead. The timeline does not render again for it; the shell paints
    /// the playhead from [`Self::playhead`].
    pub fn set_position(&mut self, frame: usize) {
        self.position = frame;
    }
    pub fn renders(&self) -> u64 {
        self.renders
    }
    /// Where the shell paints the playhead, once the tracks have been laid out.
    pub fn playhead(&self) -> Option<PlayheadGeometry> {
        let (model, bounds) = (self.model.as_ref()?, self.bounds?);
        let inset = px(axis_inset());
        let axis = Bounds::new(
            bounds.origin + gpui::point(inset, px(0.)),
            gpui::size((bounds.size.width - inset).max(px(0.)), bounds.size.height),
        );
        let x = f32::from(axis.left()) + self.viewport.x_at_frame(self.position, model) as f32;
        Some(PlayheadGeometry { axis, x })
    }
    pub fn thumbnail_requests(&self) -> Vec<ThumbnailKey> {
        let Some(model) = &self.model else {
            return vec![];
        };
        let scale = thumbnail_scale(model);
        self.viewport
            .thumbnail_frames(model)
            .into_iter()
            .map(|frame| {
                ThumbnailKey::new(
                    model.report().envelope.identity.clone(),
                    model.report().timing_generation,
                    scale,
                    frame,
                )
            })
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
        let rect = |b: Bounds<Pixels>| {
            [
                f32::from(b.left()),
                f32::from(b.top()),
                f32::from(b.size.width),
                f32::from(b.size.height),
            ]
        };
        serde_json::json!({
            "entries": self.cache.len(), "bytes": self.cache.bytes(),
            "high_water_entries": self.cache.high_water_entries(), "high_water_bytes": self.cache.high_water_bytes(),
            "release_failures": self.release_failures,
            "pixels_per_second": self.viewport.pixels_per_second(), "scroll_x": self.viewport.scroll_x(),
            "zoom": self.viewport.zoom, "position_seconds": self.viewport.position,
            "clips": self.clips.len(), "painted_segments": self.painted_segments,
            "waveform_peaks": self.waveform.len(),
            "selection": {
                "scene": self.selection.scene_id,
                "range": self.selection.range.as_ref().map(|r| [r.start, r.end]),
                "position": self.position,
                "total_frames": self.model.as_ref().map(|m| m.report().total_frames),
            },
            "ruler_bounds": self.bounds.map(ruler),
            "fit_bounds": self.fit_bounds,
            "minimap_bounds": self.minimap_bounds.map(rect),
            "edit": self.edit_metrics(),
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
        let (Some(frame), Some(model)) = (self.pointer_frame(event.position.x), &self.model) else {
            return;
        };
        window.focus(&self.focus, cx);
        let range = event.modifiers.shift
            || self.bounds.is_some_and(|b| {
                hit_zone(f32::from(event.position.y), f32::from(b.top())) == HitZone::Lanes
            });
        self.drag = Some((frame, range));
        if range {
            self.selection.select_range(frame, frame, model);
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
            if let Some(model) = &self.model {
                self.selection.select_range(start, frame, model);
            }
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
    fn minimap_down(&mut self, event: &gpui::MouseDownEvent, cx: &mut Context<Self>) {
        let (Some(model), Some(bar)) = (&self.model, self.minimap_bounds) else {
            return;
        };
        self.minimap_drag = MinimapDrag::begin(
            f32::from(event.position.x),
            f32::from(bar.left()),
            f32::from(bar.size.width),
            &mut self.viewport,
            model,
        );
        if self.minimap_drag.is_some() {
            cx.stop_propagation();
            cx.notify();
        }
    }
    fn minimap_move(&mut self, event: &gpui::MouseMoveEvent, cx: &mut Context<Self>) {
        if let (Some(drag), Some(model)) = (&self.minimap_drag, &self.model)
            && event.dragging()
        {
            self.viewport = drag.update(f32::from(event.position.x), model);
            cx.notify();
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
    fn select_scene(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(model) = &self.model {
            self.selection.select_scene(index, model);
        }
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
    /// Zooms by `factor` (`2.` doubles the pixels per second), keeping the playhead
    /// where it is on screen.
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
    /// Shows the whole video.
    pub fn fit(&mut self, cx: &mut Context<Self>) {
        if let Some(model) = &self.model {
            self.viewport.fit(model);
            cx.notify();
        }
    }
    /// Scrolls the tracks by three quarters of their width, back (`-1.`) or forward (`1.`).
    pub fn page(&mut self, direction: f64, cx: &mut Context<Self>) {
        if let Some(model) = &self.model {
            self.viewport
                .scroll_by(direction * self.viewport.width * 0.75, model);
            cx.notify();
        }
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
        let target = fraction_to_zoom(fraction, self.fit_pixels_per_second());
        self.zoom(target / self.viewport.pixels_per_second(), cx);
    }
    /// The scale at which the whole video fills the axis.
    fn fit_pixels_per_second(&self) -> f64 {
        self.model
            .as_ref()
            .filter(|model| model.duration() > 0.)
            .map_or(ZOOM_MAX, |model| self.viewport.width / model.duration())
    }
}

/// What a drag on a clip does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditKind {
    Trim(TrimEdge),
    Reorder,
}

/// A drag on a clip. The boxes stay where they were when it started while the preview
/// retimes under it, and only the dragged clip follows the pointer (Cap's ghost trim).
#[derive(Debug, Clone)]
struct EditDrag {
    kind: EditKind,
    /// The clip's position in the snapshot.
    clip: usize,
    down_x: f32,
    /// Set once the pointer moved [`DRAG_THRESHOLD_PX`]; a press that never moves is a click.
    moved: bool,
    layout: Vec<ClipSegment>,
    /// The clip's frames when the drag started.
    segment: (usize, usize),
    original: ResolvedClip,
    targets: SnapTargets,
    /// The trim last sent, or the position a reorder drops the clip at.
    value: Option<usize>,
    /// The time the dragged edge snapped to, in seconds.
    snapped: Option<f64>,
}

impl EditDrag {
    /// The layout as drawn: the dragged clip at its trim, the others where they were.
    fn draft(&self) -> Vec<ClipSegment> {
        let mut layout = self.layout.clone();
        if let (EditKind::Trim(edge), Some(trim)) = (self.kind, self.value)
            && let Some(segment) = layout.iter_mut().find(|s| s.clip == Some(self.clip))
        {
            let (start, clip) = (self.segment.0, &self.original);
            match edge {
                TrimEdge::Start => {
                    segment.start = (start + trim).saturating_sub(clip.trim_start_frames);
                }
                TrimEdge::End => {
                    segment.end = start + clip.scene_frames - clip.trim_start_frames - trim;
                }
            }
        }
        layout
    }

    fn name(&self) -> &'static str {
        match self.kind {
            EditKind::Trim(TrimEdge::Start) => "trim-start",
            EditKind::Trim(TrimEdge::End) => "trim-end",
            EditKind::Reorder => "reorder",
        }
    }
}

impl TimelineView {
    /// The snapshot when the timeline can be edited now, else why it cannot.
    fn edit_state(&self) -> Result<&TimelineSnapshot, String> {
        match (&self.model, &self.snapshot) {
            (Some(_), Some(snapshot)) => match &snapshot.editing {
                TimingEditing::Enabled => Ok(snapshot),
                TimingEditing::Disabled(reason) => Err(reason.clone()),
            },
            _ => Err("Build the preview to edit the timeline".into()),
        }
    }

    /// Whether a clip drag is live; the shell then follows the pointer through the window.
    pub fn edit_dragging(&self) -> bool {
        self.edit.is_some()
    }

    /// Starts a drag of `clip` pressed at window x `x`. The timing document changes only
    /// once the pointer has moved [`DRAG_THRESHOLD_PX`].
    fn begin_edit(&mut self, kind: EditKind, clip: usize, x: f32, cx: &mut Context<Self>) -> bool {
        let Ok(snapshot) = self.edit_state() else {
            return false;
        };
        let (Some(model), Some(original)) = (&self.model, snapshot.clips.get(clip).cloned()) else {
            return false;
        };
        let Some(segment) = self
            .clips
            .iter()
            .find(|segment| segment.clip == Some(clip) && segment.scene.is_some())
        else {
            return false;
        };
        let fps = model.report().fps as f64;
        let audio = model
            .report()
            .audio_tracks
            .iter()
            .map(|track| (track.start_seconds, track.end_seconds));
        let drag = EditDrag {
            kind,
            clip,
            down_x: x,
            moved: false,
            layout: self.clips.clone(),
            segment: (segment.start, segment.end),
            original,
            targets: SnapTargets::for_trim(
                clip,
                &self.clips,
                audio,
                self.position as f64 / fps,
                fps,
            ),
            value: None,
            snapped: None,
        };
        self.edit = Some(drag);
        cx.notify();
        true
    }

    /// A pointer move while a clip drag is live, anywhere in the window: `x` is the window
    /// x, `snap_disabled` while Shift is held. A trim opens its gesture on the first move
    /// and sends each new trim; a reorder only tracks where the clip would land.
    pub fn edit_drag_move(&mut self, x: f32, snap_disabled: bool, cx: &mut Context<Self>) {
        let pixels_per_second = self.viewport.pixels_per_second();
        let axis_x = self
            .bounds
            .map_or(0., |b| f32::from(px(x) - b.left()) - axis_inset());
        let pointer_seconds = self.viewport.seconds_at_x(f64::from(axis_x));
        let (Some(model), Some(drag)) = (&self.model, &mut self.edit) else {
            return;
        };
        if pixels_per_second <= 0. {
            return;
        }
        if !drag.moved {
            if (x - drag.down_x).abs() < DRAG_THRESHOLD_PX {
                return;
            }
            drag.moved = true;
            if matches!(drag.kind, EditKind::Trim(_)) {
                cx.emit(TimelineEvent::Edit(TimelineCommand::BeginGesture));
            }
        }
        let fps = model.report().fps as f64;
        match drag.kind {
            EditKind::Trim(edge) => {
                let edge_frame = match edge {
                    TrimEdge::Start => drag.segment.0,
                    TrimEdge::End => drag.segment.1,
                };
                let raw = edge_frame as f64 / fps + f64::from(x - drag.down_x) / pixels_per_second;
                // Once a trim lands only the end edge stays where it was dragged (a start
                // trim moves the clip's content, not its box), so only it snaps, as in Cap.
                let (time, snapped) = match edge {
                    TrimEdge::End => drag
                        .targets
                        .snap(raw, 1. / pixels_per_second, snap_disabled),
                    TrimEdge::Start => (raw, None),
                };
                drag.snapped = snapped;
                let to = (time * fps).round().max(0.) as usize;
                let trim = trim_for_edge(&drag.original, drag.segment.0, edge, to);
                if drag.value != Some(trim) {
                    drag.value = Some(trim);
                    cx.emit(TimelineEvent::Edit(TimelineCommand::Trim {
                        clip: drag.clip,
                        edge,
                        frames: trim,
                    }));
                }
            }
            EditKind::Reorder => {
                let at = (pointer_seconds * fps).max(0.) as usize;
                drag.value = Some(drop_position(&drag.layout, drag.clip, at));
            }
        }
        cx.notify();
    }

    /// Ends a clip drag: a trim closes its gesture, which makes it one undo step, and a
    /// reorder moves the clip. Runs on the release anywhere, on a move with no button held
    /// (a release outside the window), when the window loses focus and when the preview is
    /// replaced.
    pub fn end_edit_drag(&mut self, cx: &mut Context<Self>) {
        self.finish_edit_drag(true, cx);
    }

    /// A trim still closes its gesture, but a reorder is dropped: its indices belong to a
    /// layout that was replaced.
    fn abandon_edit_drag(&mut self, cx: &mut Context<Self>) {
        self.finish_edit_drag(false, cx);
    }

    fn finish_edit_drag(&mut self, commit_reorder: bool, cx: &mut Context<Self>) {
        let Some(drag) = self.edit.take() else {
            return;
        };
        if drag.moved {
            match (drag.kind, drag.value) {
                (EditKind::Trim(_), _) => {
                    cx.emit(TimelineEvent::Edit(TimelineCommand::EndGesture));
                }
                (EditKind::Reorder, Some(to)) if commit_reorder && to != drag.clip => {
                    cx.emit(TimelineEvent::Edit(TimelineCommand::Move {
                        clip: drag.clip,
                        to,
                    }));
                }
                (EditKind::Reorder, _) => {}
            }
        }
        cx.notify();
    }

    /// Drops a drag whose gesture the engine refused, asking nothing more of it.
    pub fn cancel_edit_drag(&mut self, cx: &mut Context<Self>) {
        if self.edit.take().is_some() {
            cx.notify();
        }
    }

    /// Splits the clip under the playhead (the selected scene's, when it is there).
    pub fn split_at_playhead(&mut self, cx: &mut Context<Self>) {
        if self.edit_state().is_err() || self.edit.is_some() {
            return;
        }
        let selected = self.model.as_ref().and_then(|model| {
            let id = self.selection.scene_id.as_ref()?;
            model
                .report()
                .scenes
                .iter()
                .position(|scene| &scene.instance_id == id)
        });
        if let Some((clip, at)) = split_at(&self.clips, self.position, selected) {
            cx.emit(TimelineEvent::Edit(TimelineCommand::Split { clip, at }));
        }
    }

    /// Asks the engine to undo (`Undo`) or redo (`Redo`) the last timeline edit.
    fn history(&mut self, command: TimelineCommand, cx: &mut Context<Self>) {
        if self.edit.is_none() {
            cx.emit(TimelineEvent::Edit(command));
        }
    }

    /// The edit state for the telemetry, with the window bounds of every clip's box.
    fn edit_metrics(&self) -> serde_json::Value {
        let state = self.edit_state();
        let clips: Vec<_> = match (&self.model, self.bounds) {
            (Some(model), Some(bounds)) => {
                let axis = f32::from(bounds.left()) + axis_inset();
                self.clips
                    .iter()
                    .filter_map(|segment| {
                        let left = axis + self.viewport.x_at_frame(segment.start, model) as f32;
                        let right = axis + self.viewport.x_at_frame(segment.end, model) as f32;
                        let top = f32::from(bounds.top()) + METRICS.ruler_h + row_top(segment.lane);
                        Some(serde_json::json!({
                            "clip": segment.clip?, "scene": segment.scene,
                            "frames": [segment.start, segment.end],
                            "bounds": [left, top, right - left, METRICS.track_h],
                        }))
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        serde_json::json!({
            "enabled": state.is_ok(),
            "reason": state.err(),
            "can_undo": self.snapshot.as_ref().is_some_and(|s| s.can_undo),
            "can_redo": self.snapshot.as_ref().is_some_and(|s| s.can_redo),
            "notice": self.snapshot.as_ref().and_then(|s| s.notice.clone()),
            "drag": self.edit.as_ref().map(EditDrag::name),
            "drop": self.edit.as_ref().filter(|d| d.kind == EditKind::Reorder).and_then(|d| d.value),
            "snapped": self.edit.as_ref().and_then(|d| d.snapped),
            "clips": clips,
        })
    }

    /// A trim handle on the `edge` of `clip`'s box, Cap's clip handles.
    fn trim_handle(
        &self,
        edge: TrimEdge,
        clip: usize,
        accent: Hsla,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let start = edge == TrimEdge::Start;
        div()
            .id((if start { "trim-start" } else { "trim-end" }, clip))
            .absolute()
            .top_0()
            .bottom_0()
            .w(px(TRIM_HANDLE_W))
            .when(start, |handle| handle.left_0())
            .when(!start, |handle| handle.right_0())
            .cursor(gpui::CursorStyle::ResizeLeftRight)
            .hover(move |handle| handle.bg(accent))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v, e: &gpui::MouseDownEvent, w, cx| {
                    if v.begin_edit(EditKind::Trim(edge), clip, f32::from(e.position.x), cx) {
                        w.focus(&v.focus, cx);
                        cx.stop_propagation();
                    }
                }),
            )
    }

    /// A text button of the toolbar.
    fn text_button(
        theme: &Theme,
        entity: &gpui::Entity<Self>,
        id: &'static str,
        label: &'static str,
        enabled: bool,
        run: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> EditorButton {
        EditorButton::plain(theme, id)
            .label(label)
            .disabled(!enabled)
            .on_click(Self::action(entity, run))
    }
}

fn thumbnail_scale(model: &TimelineModel) -> f64 {
    (160. / model.report().width as f64)
        .min(90. / model.report().height as f64)
        .min(1.)
}

/// The closest zoom (pixels per second); the transform clamps to it.
const ZOOM_MAX: f64 = studio_engine::MAX_PIXELS_PER_SECOND;
const ZOOM_SLIDER_W: f32 = 72.;
const ZOOM_TRACK_H: f32 = 3.;
const ZOOM_THUMB: f32 = 14.;
const CARD_PAD: f32 = 11.;
const CARD_PAD_TOP: f32 = 9.;
/// The mix waveform keeps this much of its row free at the top.
const WAVEFORM_PAD: f32 = 6.;
/// Width of a clip's trim handles.
const TRIM_HANDLE_W: f32 = 6.;

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

/// Zoom (pixels per second) as a 0..=1 slider fraction on a log scale from Fit (`fit`)
/// to the closest zoom.
fn zoom_to_fraction(pixels_per_second: f64, fit: f64) -> f32 {
    if fit >= ZOOM_MAX || fit <= 0. {
        return 0.;
    }
    ((pixels_per_second.clamp(fit, ZOOM_MAX) / fit).ln() / (ZOOM_MAX / fit).ln()) as f32
}

/// Inverse of [`zoom_to_fraction`].
fn fraction_to_zoom(fraction: f32, fit: f64) -> f64 {
    let fit = fit.clamp(f64::MIN_POSITIVE, ZOOM_MAX);
    fit * (ZOOM_MAX / fit).powf(f64::from(fraction.clamp(0., 1.)))
}

impl Render for TimelineView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.renders += 1;
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
        let zoom_fraction = zoom_to_fraction(
            self.viewport.pixels_per_second(),
            self.fit_pixels_per_second(),
        );
        let editable = self.edit_state().is_ok();
        let (can_undo, can_redo) = self
            .snapshot
            .as_ref()
            .map_or((false, false), |s| (s.can_undo, s.can_redo));
        let mut controls = div().flex().items_center().gap(px(metrics.space_1));
        if let Some(model) = &self.model {
            let weak = cx.entity().downgrade();
            let fps = model.report().fps as f64;
            let boundaries = self.clips.iter().map(|clip| clip.start as f64 / fps);
            let interactive = zoomed_in(&self.viewport, model);
            controls = controls.child(
                render_minimap(&theme, &self.viewport, model, boundaries, MINIMAP_MAX_WIDTH)
                    .id("timeline-minimap")
                    .relative()
                    .mr(px(metrics.space_1))
                    // It takes presses only while part of the video is hidden.
                    .when(interactive, |minimap| {
                        minimap.cursor(gpui::CursorStyle::OpenHand).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|v, e, _, cx| v.minimap_down(e, cx)),
                        )
                    })
                    .child(
                        canvas(
                            move |bounds, _, cx| {
                                let _ =
                                    weak.update(cx, |view, _| view.minimap_bounds = Some(bounds));
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    ),
            );
        }
        let toolbar = section_title(&theme, "Timeline").child(
            controls
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
                        .on_click(Self::action(&entity, |v, cx| v.fit(cx))),
                )
                .child(Self::text_button(
                    &theme,
                    &entity,
                    "timeline-split",
                    "Split",
                    editable,
                    |v, cx| v.split_at_playhead(cx),
                ))
                .child(Self::text_button(
                    &theme,
                    &entity,
                    "timeline-undo",
                    "Undo",
                    editable && can_undo,
                    |v, cx| v.history(TimelineCommand::Undo, cx),
                ))
                .child(Self::text_button(
                    &theme,
                    &entity,
                    "timeline-redo",
                    "Redo",
                    editable && can_redo,
                    |v, cx| v.history(TimelineCommand::Redo, cx),
                ))
                .child(Self::icon_button(
                    &theme,
                    "timeline-cycle",
                    Icon::Layers,
                    "Cycle scene (S)",
                    "Cycle scene (S)",
                    enabled,
                    Self::action(&entity, |v, cx| v.cycle_scene(cx)),
                )),
        );
        let root = div()
            .flex()
            .flex_col()
            .gap(px(metrics.space_1 + 2.))
            .min_w_0()
            .flex_shrink_0()
            .size_full()
            .pl(px(CARD_PAD))
            .pr(px(CARD_PAD))
            .pt(px(CARD_PAD_TOP))
            .pb(px(CARD_PAD))
            .on_mouse_move(cx.listener(|v, e, _, cx| v.minimap_move(e, cx)))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|v, _, _, _| v.minimap_drag = None),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|v, _, _, _| v.minimap_drag = None),
            )
            .child(toolbar);
        let Some(model) = &self.model else {
            self.painted_segments = 0;
            return root.child(
                text_style(div(), TextStyle::Body)
                    .text_color(colors.text_2)
                    .child(selectable("No compiled timeline available.")),
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
            .key_context(KEY_CONTEXT)
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
            )
            .child(render_ruler(&theme, geometry.ticks, inset));

        // Rows below the ruler: the clip track, overlay lanes, audio files and the mix.
        // Only segments inside the padded viewport are built (Cap's `isSegmentVisible`).
        let report = model.report();
        let fps = report.fps as f64;
        let pixels_per_second = self.viewport.pixels_per_second();
        let scene_lanes = geometry.scene_lane_count.max(1);
        let audio_count = report.audio_tracks.len();
        let mut painted = 0;
        let mut scene_rows: Vec<Vec<gpui::AnyElement>> =
            (0..scene_lanes).map(|_| Vec::new()).collect();
        let mut audio_rows: Vec<Vec<gpui::AnyElement>> =
            (0..audio_count).map(|_| Vec::new()).collect();
        let drawn = self
            .edit
            .as_ref()
            .map_or_else(|| self.clips.clone(), EditDrag::draft);
        let dragged = self.edit.as_ref().filter(|d| d.moved).map(|d| d.clip);
        for (number, clip) in drawn.iter().enumerate() {
            let (start, end) = (clip.start as f64 / fps, clip.end as f64 / fps);
            if !self.viewport.segment_visible(start, end) {
                continue;
            }
            let scene = clip.scene.map(|index| (index, &report.scenes[index]));
            let selected = scene
                .is_some_and(|(_, s)| self.selection.scene_id.as_ref() == Some(&s.instance_id));
            let element = segment(
                &theme,
                if clip.lane == 0 {
                    TrackKind::Clip
                } else {
                    TrackKind::Overlay
                },
                selected,
                self.viewport.x_at_seconds(start) as f32,
                ((end - start) * pixels_per_second).max(1.) as f32,
                view_width,
                clip.label.clone(),
            )
            .when(dragged.is_some() && dragged == clip.clip, |element| {
                element.opacity(0.7)
            })
            .when_some(
                clip.clip
                    .filter(|_| editable && self.edit.is_none() && scene.is_some()),
                |element, position| {
                    element
                        .child(self.trim_handle(
                            TrimEdge::Start,
                            position,
                            colors.accent.into(),
                            cx,
                        ))
                        .child(self.trim_handle(TrimEdge::End, position, colors.accent.into(), cx))
                },
            );
            painted += 1;
            scene_rows[clip.lane.min(scene_lanes - 1)].push(match scene {
                Some((index, scene)) => self
                    .selectable_scene(element, index, clip.clip, scene, colors.accent.into(), cx)
                    .into_any_element(),
                None => element
                    .id(("clip", number))
                    .aria_label(format!("Clip {}", clip.label))
                    .into_any_element(),
            });
        }
        for rect in geometry.audio {
            let audio = &report.audio_tracks[rect.index];
            painted += 1;
            audio_rows[rect.lane.min(audio_count - 1)].push(
                segment(
                    &theme,
                    TrackKind::Audio,
                    false,
                    rect.x as f32,
                    rect.width.max(1.) as f32,
                    view_width,
                    format!("♫ {} · {:.3}s", audio.file, audio.start_seconds),
                )
                .into_any_element(),
            );
        }
        self.painted_segments = painted;
        let mix = !self.waveform.is_empty() && audio_count > 0;
        let total_rows = scene_lanes + audio_count + usize::from(mix);
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
        if mix {
            content = content.child(track_row(
                &theme,
                row_top(scene_lanes + audio_count),
                "Mix".into(),
                Icon::Volume2,
                vec![self.mix_waveform(model).into_any_element()],
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

        // The selected range over the time axis; the shell paints the playhead.
        if let Some(range) = &self.selection.range {
            let start = self.viewport.x_at_frame(range.start, model);
            let end = self.viewport.x_at_frame(range.end, model);
            tracks = tracks.child(
                div()
                    .absolute()
                    .left(px(inset))
                    .right_0()
                    .top_0()
                    .bottom_0()
                    .overflow_hidden()
                    .child(
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
                    ),
            );
        }

        // Where a dragged clip lands, or the target a dragged edge snapped to.
        let marker = self
            .edit
            .as_ref()
            .filter(|d| d.moved)
            .and_then(|drag| match drag.kind {
                EditKind::Reorder => drag
                    .value
                    .and_then(|to| drop_marker(&drag.layout, drag.clip, to))
                    .map(|frame| self.viewport.x_at_frame(frame, model)),
                EditKind::Trim(_) => drag
                    .snapped
                    .map(|seconds| self.viewport.x_at_seconds(seconds)),
            });
        if let Some(x) = marker {
            tracks = tracks.child(
                div()
                    .absolute()
                    .left(px(inset))
                    .right_0()
                    .top_0()
                    .bottom_0()
                    .overflow_hidden()
                    .child(
                        div()
                            .absolute()
                            .left(px(x as f32 - 1.))
                            .top(px(metrics.ruler_h))
                            .bottom_0()
                            .w(px(2.))
                            .bg(colors.accent),
                    ),
            );
        }

        let mut thumbs = div().relative().flex_none().h(px(38.)).overflow_hidden();
        for frame in self.viewport.thumbnail_frames(model) {
            let key = ThumbnailKey::new(
                report.envelope.identity.clone(),
                report.timing_generation,
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
                .child(selectable(format!(
                    "{selected} · {}",
                    match (
                        self.edit_state(),
                        self.snapshot.as_ref().and_then(|s| s.notice.as_ref())
                    ) {
                        (Err(reason), _) => reason,
                        (Ok(_), Some(notice)) => notice.clone(),
                        (Ok(_), None) =>
                            "drag edges to trim, clips to reorder · C splits · Shift-drag for range"
                                .into(),
                    }
                ))),
        )
    }
}

impl TimelineView {
    /// A scene segment that selects its scene on click or Enter. A press on a clip may
    /// also start dragging it to another place.
    fn selectable_scene(
        &self,
        element: gpui::Div,
        index: usize,
        clip: Option<usize>,
        scene: &fframes_studio_protocol::PreviewSceneInfo,
        accent: Hsla,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        element
            .id(format!("scene-{}", scene.instance_id))
            .role(gpui::Role::Button)
            .aria_label(format!("Scene {} instance {}", scene.name, scene.index))
            .tab_index(0)
            .focus_visible(move |s| s.border_color(accent))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v, e: &gpui::MouseDownEvent, w, cx| {
                    if e.modifiers.shift {
                        return;
                    }
                    v.select_scene(index, cx);
                    if let Some(clip) = clip {
                        v.begin_edit(EditKind::Reorder, clip, f32::from(e.position.x), cx);
                    }
                    w.focus(&v.focus, cx);
                    cx.stop_propagation();
                }),
            )
            .on_key_down(cx.listener(move |v, e: &gpui::KeyDownEvent, _, cx| {
                if e.keystroke.key == "enter" {
                    v.select_scene(index, cx);
                    cx.stop_propagation();
                }
            }))
    }

    /// The mix row's waveform over the visible seconds.
    fn mix_waveform(&self, model: &TimelineModel) -> impl IntoElement {
        let peaks = self.waveform.clone();
        let start = self.viewport.position.max(0.);
        let end = (self.viewport.position + self.viewport.zoom).min(model.duration());
        let x = self.viewport.x_at_seconds(start) as f32;
        let width = ((end - start) * self.viewport.pixels_per_second()) as f32;
        let color = Hsla::from(track_color(TrackKind::Audio)).opacity(0.55);
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let area = Bounds::new(
                    bounds.origin + gpui::point(px(x), px(WAVEFORM_PAD)),
                    gpui::size(px(width), bounds.size.height - px(WAVEFORM_PAD)),
                );
                if end > start
                    && let Some(path) = waveform_path(&peaks, (start, end), area)
                {
                    window.paint_path(path, color);
                }
            },
        )
        .absolute()
        .size_full()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::model::tests::report;

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
    fn zoom_slider_runs_from_fit_to_the_closest_zoom() {
        let fit = 25.;
        assert_eq!(zoom_to_fraction(fit, fit), 0.);
        assert_eq!(zoom_to_fraction(ZOOM_MAX, fit), 1.);
        assert_eq!(zoom_to_fraction(fit / 10., fit), 0.);
        assert_eq!(zoom_to_fraction(ZOOM_MAX * 10., fit), 1.);
        assert_eq!(zoom_to_fraction(10., ZOOM_MAX), 0.);
        assert_eq!(fraction_to_zoom(-1., fit), fit);
        for pps in [30., 100., 4_000.] {
            let back = fraction_to_zoom(zoom_to_fraction(pps, fit), fit);
            assert!((back / pps - 1.).abs() < 1e-3, "{pps} -> {back}");
        }
        assert!(fraction_to_zoom(0.25, fit) < fraction_to_zoom(0.75, fit));
    }

    #[gpui::test]
    fn the_fit_button_reports_its_bounds_for_qualification(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
        cx.run_until_parked();
        let fit = view.read_with(cx, |view, _| view.fit_bounds);
        let [_, _, width, height] = fit.expect("the Fit button reports where it is");
        assert!(width > 0. && height > 0., "{fit:?}");
    }

    /// 30 one-second clips at 10 fps.
    fn thirty_clips() -> TimelineModel {
        let keys: Vec<_> = (0..30).map(|index| format!("clip-{index}")).collect();
        let scenes: Vec<_> = keys
            .iter()
            .enumerate()
            .map(|(index, key)| (key.as_str(), index * 10, index * 10 + 10))
            .collect();
        TimelineModel::new(report(10, &scenes)).unwrap()
    }

    #[gpui::test]
    fn only_segments_near_the_view_are_painted(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
        view.update(cx, |view, cx| {
            view.install(thirty_clips(), Arc::from([]), 0, cx)
        });
        cx.run_until_parked();
        assert_eq!(view.read_with(cx, |view, _| view.painted_segments), 30);
        view.update(cx, |view, cx| {
            let model = view.model.clone().unwrap();
            view.viewport.update_zoom(3., 0., &model);
            view.viewport.set_position(10., &model);
            cx.notify();
        });
        cx.run_until_parked();
        // [10 s, 13 s) and two seconds of padding either side: clips 7 to 15.
        let metrics = view.read_with(cx, |view, _| view.qualification_metrics());
        assert_eq!(metrics["painted_segments"], 9, "{metrics:#}");
        assert_eq!(metrics["clips"], 30);
    }

    #[gpui::test]
    fn the_engine_snapshot_drives_the_clip_track(cx: &mut gpui::TestAppContext) {
        use crate::timeline::model::tests::snapshot;
        let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
        view.update(cx, |view, cx| {
            view.install(thirty_clips(), Arc::from([]), 0, cx)
        });
        cx.run_until_parked();
        // The document plays two clips; until the report catches up the second has no scene.
        view.update(cx, |view, cx| {
            view.set_snapshot(
                Some(snapshot(&[("clip-3", 10, 0, 0), ("clip-3", 10, 5, 0)])),
                cx,
            )
        });
        cx.run_until_parked();
        let (clips, painted) = view.read_with(cx, |view, _| {
            let spans: Vec<_> = view
                .clips
                .iter()
                .map(|c| (c.start, c.end, c.scene))
                .collect();
            (spans, view.painted_segments)
        });
        // The first clip keeps its scene's frames, the second follows it, and the 29
        // scenes no clip plays stay on their rows.
        assert_eq!(clips[..2], [(30, 40, Some(3)), (40, 45, None)]);
        assert_eq!(clips.len(), 31);
        assert!(!clips[2..].iter().any(|(_, _, scene)| *scene == Some(3)));
        assert_eq!(painted, 31);
        view.update(cx, |view, cx| view.set_snapshot(None, cx));
        assert_eq!(view.read_with(cx, |view, _| view.clips.len()), 30);
    }

    #[gpui::test]
    fn a_minimap_press_centres_the_view_and_a_fitted_one_ignores_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
        view.update(cx, |view, cx| {
            view.install(thirty_clips(), Arc::from([]), 0, cx)
        });
        cx.run_until_parked();
        let centre = |cx: &mut gpui::VisualTestContext| {
            let [left, top, width, height] = view
                .read_with(cx, |view, _| view.qualification_metrics())["minimap_bounds"]
                .as_array()
                .expect("the minimap reports where it is")
                .iter()
                .map(|value| value.as_f64().unwrap() as f32)
                .collect::<Vec<_>>()[..]
            else {
                panic!("four numbers")
            };
            gpui::point(px(left + width / 2.), px(top + height / 2.))
        };
        // Fitted: the minimap is hidden and the press changes nothing.
        let at = centre(cx);
        cx.simulate_click(at, gpui::Modifiers::none());
        assert_eq!(view.read_with(cx, |view, _| view.viewport.position), 0.);

        view.update(cx, |view, cx| {
            let model = view.model.clone().unwrap();
            view.viewport.update_zoom(3., 0., &model);
            cx.notify();
        });
        cx.run_until_parked();
        // The bar's middle is 15 s; the 3 s view is centred on it.
        let at = centre(cx);
        cx.simulate_click(at, gpui::Modifiers::none());
        let position = view.read_with(cx, |view, _| view.viewport.position);
        assert!((position - 13.5).abs() < 1e-3, "{position}");
    }

    /// Three three-second clips at 10 fps, editable, with every edit the view asks of the
    /// engine collected.
    fn editable(
        cx: &mut gpui::TestAppContext,
    ) -> (
        gpui::Entity<TimelineView>,
        &mut gpui::VisualTestContext,
        std::rc::Rc<std::cell::RefCell<Vec<TimelineCommand>>>,
    ) {
        use crate::timeline::model::tests::snapshot;
        let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
        let edits = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let log = edits.clone();
        cx.update(|_, app| {
            app.subscribe(&view, move |_, event: &TimelineEvent, _| {
                if let TimelineEvent::Edit(command) = event {
                    log.borrow_mut().push(command.clone());
                }
            })
            .detach();
        });
        view.update(cx, |view, cx| {
            let model =
                TimelineModel::new(report(10, &[("a", 0, 30), ("b", 30, 60), ("c", 60, 90)]))
                    .unwrap();
            view.install(model, Arc::from([]), 0, cx);
            view.set_snapshot(
                Some(snapshot(&[
                    ("a", 30, 0, 0),
                    ("b", 30, 0, 0),
                    ("c", 30, 0, 0),
                ])),
                cx,
            );
        });
        cx.run_until_parked();
        // The first frame measures the track and fits it; the handles are laid out on the next.
        view.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        (view, cx, edits)
    }

    /// The window bounds of clip `clip`'s box: left, top, width, height.
    fn clip_bounds(
        view: &gpui::Entity<TimelineView>,
        cx: &mut gpui::VisualTestContext,
        clip: usize,
    ) -> [f32; 4] {
        let metrics = view.read_with(cx, |view, _| view.qualification_metrics());
        let found = metrics["edit"]["clips"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["clip"] == clip)
            .unwrap_or_else(|| panic!("clip {clip} in {metrics:#}"));
        let at = |i: usize| found["bounds"][i].as_f64().unwrap() as f32;
        [at(0), at(1), at(2), at(3)]
    }

    fn press(cx: &mut gpui::VisualTestContext, x: f32, y: f32) {
        // The first move paints the hover the press is tested against.
        for at in [x + 12., x] {
            cx.simulate_mouse_move(gpui::point(px(at), px(y)), None, gpui::Modifiers::none());
            cx.run_until_parked();
        }
        cx.simulate_mouse_down(
            gpui::point(px(x), px(y)),
            MouseButton::Left,
            gpui::Modifiers::none(),
        );
    }

    #[gpui::test]
    fn dragging_a_clip_edge_trims_in_one_gesture(cx: &mut gpui::TestAppContext) {
        let (view, cx, edits) = editable(cx);
        let [left, top, width, height] = clip_bounds(&view, cx, 1);
        let per_second = width / 3.;
        // A press on the end handle that never moves asks for nothing.
        press(cx, left + width - 2., top + height / 2.);
        assert!(view.read_with(cx, |view, _| view.edit_dragging()));
        view.update(cx, |view, cx| view.end_edit_drag(cx));
        assert!(edits.borrow().is_empty(), "{:?}", edits.borrow());

        // Pulling the end back by a second leaves 20 of the clip's 30 frames.
        press(cx, left + width - 2., top + height / 2.);
        view.update(cx, |view, cx| {
            view.edit_drag_move(left + width - 2. - per_second, true, cx);
            view.edit_drag_move(left + width - 2. - per_second, true, cx);
            view.end_edit_drag(cx);
        });
        assert_eq!(
            *edits.borrow(),
            [
                TimelineCommand::BeginGesture,
                TimelineCommand::Trim {
                    clip: 1,
                    edge: TrimEdge::End,
                    frames: 10
                },
                TimelineCommand::EndGesture,
            ]
        );
        assert!(!view.read_with(cx, |view, _| view.edit_dragging()));
    }

    #[gpui::test]
    fn the_end_edge_snaps_to_the_playhead_unless_shift_is_held(cx: &mut gpui::TestAppContext) {
        let (view, cx, edits) = editable(cx);
        view.update(cx, |view, _| view.set_position(70));
        cx.run_until_parked();
        let [left, top, width, height] = clip_bounds(&view, cx, 2);
        let per_second = width / 3.;
        // The end of the last clip, 9 s, dragged to 7.04 s: within 8 px of the playhead.
        press(cx, left + width - 2., top + height / 2.);
        let near = left + width - 2. - 1.96 * per_second;
        view.update(cx, |view, cx| view.edit_drag_move(near, false, cx));
        let snapped =
            view.read_with(cx, |view, _| view.qualification_metrics())["edit"]["snapped"].clone();
        assert_eq!(snapped, 7.0);
        view.update(cx, |view, cx| view.edit_drag_move(near, true, cx));
        let held =
            view.read_with(cx, |view, _| view.qualification_metrics())["edit"]["snapped"].clone();
        assert!(held.is_null(), "{held}");
        view.update(cx, |view, cx| view.end_edit_drag(cx));
        assert_eq!(edits.borrow().last(), Some(&TimelineCommand::EndGesture));
    }

    #[gpui::test]
    fn dragging_a_clip_along_the_track_moves_it(cx: &mut gpui::TestAppContext) {
        let (view, cx, edits) = editable(cx);
        let [left, top, width, height] = clip_bounds(&view, cx, 0);
        let [last_left, _, last_width, _] = clip_bounds(&view, cx, 2);
        // Press in the first clip's middle, away from its handles.
        press(cx, left + width / 2., top + height / 2.);
        view.update(cx, |view, cx| {
            view.edit_drag_move(last_left + last_width - 4., false, cx)
        });
        let metrics = view.read_with(cx, |view, _| view.qualification_metrics());
        assert_eq!(
            (&metrics["edit"]["drag"], &metrics["edit"]["drop"]),
            (&"reorder".into(), &2.into())
        );
        view.update(cx, |view, cx| view.end_edit_drag(cx));
        assert_eq!(*edits.borrow(), [TimelineCommand::Move { clip: 0, to: 2 }]);
    }

    #[gpui::test]
    fn a_rebuilt_preview_drops_a_reorder_but_closes_a_trim(cx: &mut gpui::TestAppContext) {
        let rebuilt = || {
            let mut report = report(10, &[("a", 0, 30), ("b", 30, 60), ("c", 60, 90)]);
            report.envelope.identity.source_revision = "b".repeat(64);
            TimelineModel::new(report).unwrap()
        };
        let (view, cx, edits) = editable(cx);
        let [left, top, width, height] = clip_bounds(&view, cx, 0);
        let [last_left, _, last_width, _] = clip_bounds(&view, cx, 2);
        press(cx, left + width / 2., top + height / 2.);
        view.update(cx, |view, cx| {
            view.edit_drag_move(last_left + last_width - 4., false, cx)
        });
        view.update(cx, |view, cx| view.install(rebuilt(), Arc::from([]), 0, cx));
        assert!(edits.borrow().is_empty(), "{:?}", edits.borrow());
        assert!(!view.read_with(cx, |view, _| view.edit_dragging()));

        let [left, top, width, height] = clip_bounds(&view, cx, 2);
        press(cx, left + width - 2., top + height / 2.);
        view.update(cx, |view, cx| {
            view.edit_drag_move(left + width - 20., false, cx)
        });
        view.update(cx, |view, cx| view.clear(cx));
        assert_eq!(edits.borrow().last(), Some(&TimelineCommand::EndGesture));
    }

    #[gpui::test]
    fn split_cuts_the_clip_under_the_playhead(cx: &mut gpui::TestAppContext) {
        let (view, cx, edits) = editable(cx);
        view.update(cx, |view, _| view.set_position(40));
        view.update(cx, |view, cx| view.split_at_playhead(cx));
        assert_eq!(
            *edits.borrow(),
            [TimelineCommand::Split { clip: 1, at: 10 }]
        );
        // On a cut between clips nothing is under the playhead.
        view.update(cx, |view, _| view.set_position(30));
        view.update(cx, |view, cx| view.split_at_playhead(cx));
        assert_eq!(edits.borrow().len(), 1);
    }

    #[gpui::test]
    fn edits_are_refused_with_the_engines_reason(cx: &mut gpui::TestAppContext) {
        use crate::timeline::model::tests::snapshot;
        let (view, cx, edits) = editable(cx);
        view.update(cx, |view, cx| {
            let mut locked = snapshot(&[("a", 30, 0, 0), ("b", 30, 0, 0), ("c", 30, 0, 0)]);
            locked.editing = TimingEditing::Disabled("Migrate the project to edit".into());
            view.set_snapshot(Some(locked), cx);
            view.set_position(40);
            view.split_at_playhead(cx);
        });
        cx.run_until_parked();
        let metrics = view.read_with(cx, |view, _| view.qualification_metrics());
        assert_eq!(metrics["edit"]["enabled"], false);
        assert_eq!(metrics["edit"]["reason"], "Migrate the project to edit");
        // A press on a clip edge grabs nothing.
        let [left, top, width, height] = clip_bounds(&view, cx, 1);
        press(cx, left + width - 2., top + height / 2.);
        assert!(!view.read_with(cx, |view, _| view.edit_dragging()));
        assert!(edits.borrow().is_empty());
    }
}
