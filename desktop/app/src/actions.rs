//! The studio's commands as gpui actions. The shell handles every one at its root, so an
//! action dispatched anywhere in the window (a key binding, a menu, a host driving the
//! app) reaches the same code as the button that offers it.
use gpui::{Action, actions};

actions!(
    studio,
    [
        /// Choose a project folder and open it.
        OpenProject,
        /// Build the open project's preview.
        BuildPreview,
        /// Stop the preview build in flight.
        CancelPreview,
        /// Choose a destination and export the accepted revision as MP4.
        ExportVideo,
        /// Stop the export in flight.
        CancelExport,
        /// Play from the playhead, or pause.
        TogglePlayback,
        /// Pause and show the next frame.
        StepForward,
        /// Pause and show the previous frame.
        StepBackward,
        /// Mute or unmute the preview's audio.
        ToggleMute,
        /// Play through the next audio output: the default, each device, then none.
        NextAudioOutput,
        /// Play through the system's default audio output again.
        RetryAudioOutput,
        /// Play without audio, on the monotonic clock.
        DisableAudioOutput,
        /// Move the playhead to the first frame.
        SeekToStart,
        /// Move the playhead past the last frame.
        SeekToEnd,
        /// Select the next scene under the playhead on the timeline.
        CycleScene,
        /// Start the timeline range selection at the playhead.
        MarkRangeStart,
        /// End the timeline range selection at the playhead.
        MarkRangeEnd,
        /// Zoom the timeline in, around the playhead.
        ZoomIn,
        /// Zoom the timeline out, around the playhead.
        ZoomOut,
    ]
);

/// Select the canvas object painted at video pixel (`x`, `y`) of the shown frame, as a
/// click there does; `cycle` selects the next object under that point instead.
#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = studio, no_json)]
pub struct SelectCanvasAt {
    pub x: f64,
    pub y: f64,
    pub cycle: bool,
}

/// Move the playhead to `frame`.
#[derive(Clone, PartialEq, Eq, Debug, Action)]
#[action(namespace = studio, no_json)]
pub struct Seek {
    pub frame: usize,
}
