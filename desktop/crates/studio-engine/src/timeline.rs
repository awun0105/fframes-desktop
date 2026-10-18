//! Pure compiled-timeline validation, geometry and selection arithmetic.

use std::ops::Range;
use std::{cmp::Reverse, collections::BinaryHeap};

use fframes_studio_protocol::{PREVIEW_CONTRACT_VERSION, PreviewTimelineResponse};

pub const DEFAULT_PIXELS_PER_SECOND: f64 = 100.0;
pub const MAX_PIXELS_PER_SECOND: f64 = 100_000.0;
const MAX_VIEWPORT_WIDTH: f64 = 1_000_000.0;
const MAX_GEOMETRY_ITEMS: usize = 100_000;
const MAX_TICKS: usize = 2_048;

#[derive(Debug, Clone)]
pub struct TimelineModel {
    report: PreviewTimelineResponse,
    scene_lanes: Vec<usize>,
}

impl TimelineModel {
    pub fn new(report: PreviewTimelineResponse) -> Result<Self, String> {
        validate_report(&report)?;
        let mut scene_lanes = vec![0; report.scenes.len()];
        let mut order: Vec<_> = (0..report.scenes.len())
            .filter(|index| report.scenes[*index].start_frame < report.scenes[*index].end_frame)
            .collect();
        order.sort_unstable_by_key(|index| {
            let scene = &report.scenes[*index];
            (scene.start_frame, scene.end_frame, *index)
        });
        let mut active = BinaryHeap::<Reverse<(usize, usize)>>::new();
        let mut available = BinaryHeap::<Reverse<usize>>::new();
        let mut lane_count = 0;
        for index in order {
            let scene = &report.scenes[index];
            while active
                .peek()
                .is_some_and(|Reverse((end, _))| *end <= scene.start_frame)
            {
                let Reverse((_, lane)) = active.pop().expect("peeked active lane");
                available.push(Reverse(lane));
            }
            let lane = available.pop().map_or_else(
                || {
                    let lane = lane_count;
                    lane_count += 1;
                    lane
                },
                |Reverse(lane)| lane,
            );
            scene_lanes[index] = lane;
            active.push(Reverse((scene.end_frame, lane)));
        }
        Ok(Self {
            report,
            scene_lanes,
        })
    }

    pub fn report(&self) -> &PreviewTimelineResponse {
        &self.report
    }

    /// The row a scene is packed into: 0 unless it overlaps an earlier scene.
    pub fn scene_lane(&self, index: usize) -> usize {
        self.scene_lanes.get(index).copied().unwrap_or(0)
    }

    /// Seconds from the start to the end of the video.
    pub fn duration(&self) -> f64 {
        self.seconds(self.report.total_frames)
    }

    /// Seconds at a cursor frame. The cursor may be exactly at the video end.
    pub fn seconds(&self, frame: usize) -> f64 {
        frame.min(self.report.total_frames) as f64 / self.report.fps as f64
    }

    /// Convert seconds to the nearest inclusive-end cursor frame.
    pub fn frame_at_seconds(&self, seconds: f64) -> Option<usize> {
        if !seconds.is_finite() || self.report.total_frames == 0 {
            return None;
        }
        if seconds <= 0.0 {
            return Some(0);
        }
        if seconds >= self.seconds(self.report.total_frames) {
            return Some(self.report.total_frames);
        }
        let frame = (seconds.max(0.0) * self.report.fps as f64).round();
        if !frame.is_finite() {
            return None;
        }
        Some(frame.clamp(0.0, self.report.total_frames as f64) as usize)
    }

    pub fn time_label(&self, frame: usize) -> String {
        let frame = frame.min(self.report.total_frames);
        format!("{:.3}s / frame {frame}", self.seconds(frame))
    }

    /// Scene indices containing `frame`, in compiled report order.
    pub fn scene_hits(&self, frame: usize) -> Vec<usize> {
        self.report
            .scenes
            .iter()
            .enumerate()
            .filter_map(|(index, scene)| {
                (scene.start_frame <= frame && frame < scene.end_frame).then_some(index)
            })
            .collect()
    }
}

fn validate_report(report: &PreviewTimelineResponse) -> Result<(), String> {
    crate::validate_preview_timeline(report)?;
    if report.envelope.contract_version != PREVIEW_CONTRACT_VERSION
        || report.envelope.request_id == 0
        || report
            .scenes
            .len()
            .saturating_add(report.audio_tracks.len())
            > MAX_GEOMETRY_ITEMS
    {
        return Err("Invalid compiled timeline timebase/dimensions".into());
    }
    for audio in &report.audio_tracks {
        if !(audio.start_seconds * MAX_PIXELS_PER_SECOND).is_finite()
            || !((audio.end_seconds - audio.start_seconds) * MAX_PIXELS_PER_SECOND).is_finite()
        {
            return Err("Compiled audio geometry overflows".into());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineRect {
    pub index: usize,
    pub x: f64,
    pub width: f64,
    pub lane: usize,
}

/// One ruler mark: `major` marks carry `label`, the others subdivide them.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineTick {
    pub x: f64,
    pub seconds: f64,
    pub major: bool,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineGeometry {
    pub scenes: Vec<TimelineRect>,
    pub audio: Vec<TimelineRect>,
    pub ticks: Vec<TimelineTick>,
    /// Reserve every compiled scene lane, including lanes outside the horizontal viewport.
    pub scene_lane_count: usize,
    pub width: f64,
}

/// Seconds of slack either side of the viewport within which a segment is still
/// built, so a short pan never shows an empty edge (Cap's `SEGMENT_RENDER_PADDING`).
pub const SEGMENT_RENDER_PADDING: f64 = 2.0;
/// At most this many ruler marks are visible (Cap's `MAX_TIMELINE_MARKINGS`).
const MAX_TIMELINE_MARKINGS: f64 = 20.0;
/// Mark spacings in seconds. Cap's ladder starts at 0.5 s; the finer steps keep the
/// ruler readable down to single frames.
const TIMELINE_MARKING_RESOLUTIONS: [f64; 16] = [
    0.01, 0.02, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 150.0, 300.0, 600.0, 1800.0,
];

/// The timeline's viewport, after Cap's `Transform`: `zoom` seconds are visible across
/// `width` pixels and `position` is the second at the left edge. Zoom is kept between
/// [`MAX_PIXELS_PER_SECOND`] and the whole video (Fit), the position so the viewport
/// never passes the end. Non-finite input never changes it.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineTransform {
    pub width: f64,
    pub zoom: f64,
    pub position: f64,
}

impl Default for TimelineTransform {
    fn default() -> Self {
        Self {
            width: 0.0,
            zoom: 0.0,
            position: 0.0,
        }
    }
}

impl TimelineTransform {
    pub fn pixels_per_second(&self) -> f64 {
        if self.zoom > 0.0 && self.width > 0.0 {
            self.width / self.zoom
        } else {
            DEFAULT_PIXELS_PER_SECOND
        }
    }

    /// The left edge in pixels from the start of the video.
    pub fn scroll_x(&self) -> f64 {
        self.position * self.pixels_per_second()
    }

    pub fn resize(&mut self, width: f64, model: &TimelineModel) {
        if width.is_finite() && width >= 0.0 {
            self.width = width.min(MAX_VIEWPORT_WIDTH);
        }
        if self.zoom <= 0.0 {
            self.fit(model);
        } else {
            self.clamp(model);
        }
    }

    /// Shows the whole video.
    pub fn fit(&mut self, model: &TimelineModel) {
        self.zoom = Self::max_zoom(self.width, model);
        self.position = 0.0;
        self.clamp(model);
    }

    /// Cap's `updateZoom`: shows `zoom` seconds, keeping the second at `origin` at the
    /// same fraction of the viewport.
    pub fn update_zoom(&mut self, zoom: f64, origin: f64, model: &TimelineModel) {
        if !zoom.is_finite() || !origin.is_finite() || self.zoom <= 0.0 {
            return;
        }
        let zoom = zoom
            .min(Self::max_zoom(self.width, model))
            .max(self.min_zoom());
        let fraction = ((origin - self.position) / self.zoom).clamp(0.0, 1.0);
        self.zoom = zoom;
        self.set_position(origin - zoom * fraction, model);
    }

    /// Zooms in by `factor` (`2.` doubles the pixels per second), keeping the time
    /// under `anchor_x` (pixels from the left edge) where it is.
    pub fn zoom(&mut self, factor: f64, anchor_x: f64, model: &TimelineModel) {
        if !factor.is_finite() || factor <= 0.0 || !anchor_x.is_finite() {
            return;
        }
        let origin = self.seconds_at_x(anchor_x.clamp(0.0, self.width));
        self.update_zoom(self.zoom / factor, origin, model);
    }

    /// Cap's `setPosition`, clamped to the video.
    pub fn set_position(&mut self, position: f64, model: &TimelineModel) {
        if position.is_finite() {
            self.position = position;
        }
        self.clamp(model);
    }

    pub fn scroll_by(&mut self, dx: f64, model: &TimelineModel) {
        if dx.is_finite() {
            let position = self.position + dx / self.pixels_per_second();
            self.set_position(
                if position.is_finite() {
                    position
                } else if dx.is_sign_positive() {
                    f64::MAX
                } else {
                    0.0
                },
                model,
            );
        }
    }

    pub fn seconds_at_x(&self, x: f64) -> f64 {
        self.position + x / self.pixels_per_second()
    }

    pub fn x_at_seconds(&self, seconds: f64) -> f64 {
        (seconds - self.position) * self.pixels_per_second()
    }

    pub fn frame_at_x(&self, x: f64, model: &TimelineModel) -> Option<usize> {
        if !x.is_finite() {
            return None;
        }
        let seconds = self.seconds_at_x(x);
        if !seconds.is_finite() {
            return Some(if x.is_sign_positive() {
                model.report.total_frames
            } else {
                0
            });
        }
        model.frame_at_seconds(seconds)
    }

    pub fn x_at_frame(&self, frame: usize, model: &TimelineModel) -> f64 {
        self.x_at_seconds(model.seconds(frame))
    }

    /// Cap's `isSegmentVisible`: whether `start..end` seconds overlaps the viewport
    /// widened by [`SEGMENT_RENDER_PADDING`].
    pub fn segment_visible(&self, start: f64, end: f64) -> bool {
        end >= (self.position - SEGMENT_RENDER_PADDING).max(0.0)
            && start <= self.position + self.zoom + SEGMENT_RENDER_PADDING
    }

    pub fn geometry(&self, model: &TimelineModel) -> TimelineGeometry {
        let pixels_per_second = self.pixels_per_second();
        let fps = model.report.fps as f64;
        let scenes = model
            .report
            .scenes
            .iter()
            .enumerate()
            .filter_map(|(index, scene)| {
                let start = scene.start_frame as f64 / fps;
                let end = scene.end_frame as f64 / fps;
                self.segment_visible(start, end).then(|| TimelineRect {
                    index,
                    x: self.x_at_seconds(start),
                    width: (end - start) * pixels_per_second,
                    lane: model.scene_lanes[index],
                })
            })
            .collect();
        let audio = model
            .report
            .audio_tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| self.segment_visible(track.start_seconds, track.end_seconds))
            .map(|(index, track)| TimelineRect {
                index,
                x: self.x_at_seconds(track.start_seconds),
                width: (track.end_seconds - track.start_seconds) * pixels_per_second,
                lane: index,
            })
            .collect();
        TimelineGeometry {
            scenes,
            audio,
            ticks: self.ticks(model),
            scene_lane_count: model
                .scene_lanes
                .iter()
                .map(|lane| lane + 1)
                .max()
                .unwrap_or(1),
            width: model.seconds(model.report.total_frames) * pixels_per_second,
        }
    }

    pub fn thumbnail_frames(&self, model: &TimelineModel) -> Vec<usize> {
        if model.report.total_frames == 0 || self.width <= 0.0 {
            return Vec::new();
        }
        let first = self.frame_at_x(0.0, model).unwrap_or(0).saturating_sub(1);
        let last = self
            .frame_at_x(self.width, model)
            .unwrap_or(model.report.total_frames)
            .saturating_add(1)
            .min(model.report.total_frames - 1);
        if first >= last {
            return vec![first.min(model.report.total_frames - 1)];
        }
        let count = (last - first + 1).min(12);
        (0..count)
            .map(|i| {
                first + (i as u128 * (last - first) as u128 / (count - 1).max(1) as u128) as usize
            })
            .collect()
    }

    /// Cap's `markingResolution`: the first spacing that shows at most
    /// [`MAX_TIMELINE_MARKINGS`] marks.
    pub fn marking_resolution(&self) -> f64 {
        TIMELINE_MARKING_RESOLUTIONS
            .into_iter()
            .find(|candidate| self.zoom / candidate <= MAX_TIMELINE_MARKINGS)
            .unwrap_or(TIMELINE_MARKING_RESOLUTIONS[TIMELINE_MARKING_RESOLUTIONS.len() - 1])
    }

    /// Cap's `TimelineMarkings`: marks on the resolution grid across the viewport, up to
    /// the end of the video. Whole seconds are major and labelled; below a quarter-second
    /// spacing every fifth mark is.
    fn ticks(&self, model: &TimelineModel) -> Vec<TimelineTick> {
        if self.width <= 0.0 || self.zoom <= 0.0 || model.report.total_frames == 0 {
            return Vec::new();
        }
        let resolution = self.marking_resolution();
        let label_steps = if resolution < 0.25 {
            5.0
        } else {
            (1..=4)
                .map(f64::from)
                .find(|steps| is_whole(steps * resolution))
                .unwrap_or(1.0)
        };
        let end = (self.position + self.zoom).min(model.seconds(model.report.total_frames));
        let first = (self.position / resolution).ceil();
        let count = (((end / resolution).floor() - first + 1.0).max(0.0) as usize).min(MAX_TICKS);
        (0..count)
            .map(|index| {
                let step = first + index as f64;
                let seconds = step * resolution;
                // Marks are compared in whole steps, not seconds, so rounding never
                // moves a label.
                let major = step % label_steps == 0.0;
                TimelineTick {
                    x: self.x_at_seconds(seconds),
                    seconds,
                    major,
                    label: major.then(|| format_ruler_time(seconds, resolution)),
                }
            })
            .collect()
    }

    /// The narrowest view: [`MAX_PIXELS_PER_SECOND`] across the width.
    fn min_zoom(&self) -> f64 {
        (self.width / MAX_PIXELS_PER_SECOND).max(f64::MIN_POSITIVE)
    }

    /// The widest view: the whole video, or the default scale when it is empty.
    fn max_zoom(width: f64, model: &TimelineModel) -> f64 {
        let duration = model.seconds(model.report.total_frames);
        if duration > 0.0 {
            duration
        } else {
            (width / DEFAULT_PIXELS_PER_SECOND).max(f64::MIN_POSITIVE)
        }
    }

    fn clamp(&mut self, model: &TimelineModel) {
        let max_zoom = Self::max_zoom(self.width, model);
        if !self.zoom.is_finite() || self.zoom <= 0.0 {
            self.zoom = max_zoom;
        }
        self.zoom = self.zoom.min(max_zoom).max(self.min_zoom());
        let last = (model.seconds(model.report.total_frames) - self.zoom).max(0.0);
        self.position = if self.position.is_finite() {
            self.position.clamp(0.0, last)
        } else {
            0.0
        };
    }
}

/// Cap's `formatTime` (`M:SS`), with the fraction a sub-second resolution needs.
pub fn format_ruler_time(seconds: f64, resolution: f64) -> String {
    let seconds = seconds.max(0.0);
    if resolution >= 1.0 || is_whole(seconds) {
        let total = seconds.round() as u64;
        return format!("{}:{:02}", total / 60, total % 60);
    }
    let decimals = if is_whole(resolution * 10.0) { 1 } else { 2 };
    let minutes = (seconds / 60.0).floor();
    let rest = seconds - minutes * 60.0;
    format!(
        "{}:{rest:0width$.decimals$}",
        minutes as u64,
        width = decimals + 3
    )
}

fn is_whole(value: f64) -> bool {
    (value - value.round()).abs() < 1e-6
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimelineSelection {
    pub scene_id: Option<String>,
    pub range: Option<Range<usize>>,
}

impl TimelineSelection {
    pub fn select_scene(&mut self, index: usize, model: &TimelineModel) {
        if let Some(scene) = model.report.scenes.get(index) {
            self.scene_id = Some(scene.instance_id.clone());
            self.range = Some(scene.start_frame..scene.end_frame);
        }
    }

    pub fn select_range(&mut self, a: usize, b: usize, model: &TimelineModel) {
        let start = a.min(b).min(model.report.total_frames);
        let end = a.max(b).min(model.report.total_frames);
        self.scene_id = None;
        self.range = Some(start..end);
    }

    pub fn clamp(&mut self, model: &TimelineModel) {
        if self.scene_id.as_ref().is_some_and(|id| {
            !model
                .report
                .scenes
                .iter()
                .any(|scene| &scene.instance_id == id)
        }) {
            self.scene_id = None;
        }
        if let Some(range) = &mut self.range {
            range.start = range.start.min(model.report.total_frames);
            range.end = range.end.min(model.report.total_frames);
            if range.start > range.end {
                std::mem::swap(&mut range.start, &mut range.end);
            }
        }
    }

    pub fn cycle_scene_at(&mut self, frame: usize, model: &TimelineModel) {
        let hits = model.scene_hits(frame);
        if hits.is_empty() {
            self.scene_id = None;
            return;
        }
        let current = self.scene_id.as_deref();
        let next_position = hits
            .iter()
            .position(|index| Some(model.report.scenes[*index].instance_id.as_str()) == current)
            .map_or(0, |position| (position + 1) % hits.len());
        self.select_scene(hits[next_position], model);
    }
}
