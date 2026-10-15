//! Which frames a validation renders.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveragePlan {
    pub requested_scope: String,
    pub requested_interval: Option<[usize; 2]>,
    pub requested_frames: Vec<usize>,
    pub boundary_frames: Vec<usize>,
    pub sample_frames: Vec<usize>,
    pub rendered_frames: Vec<usize>,
    pub inspect_frames: Vec<usize>,
    pub broadened: Option<BroadenReason>,
    pub complete: bool,
}

/// Why the targeted (boundary + playhead + sample) coverage must widen to the full video.
pub fn broadening(changes: &ChangeSet) -> Option<BroadenReason> {
    if changes.truncated {
        return Some(BroadenReason::ChangeListTruncated);
    }
    if changes.entries.iter().any(|c| {
        matches!(
            c.kind,
            FileKind::Rust
                | FileKind::Cargo
                | FileKind::Style
                | FileKind::Configuration
                // The timing document moves every scene after the first edited clip.
                | FileKind::Timing
        )
    }) {
        return Some(BroadenReason::SharedSourceChanged);
    }
    // An existing non-instruction file that was deleted or modified may be shared by
    // scenes that were not edited.
    if changes.entries.iter().any(|c| {
        matches!(c.change, ChangeKind::Deleted | ChangeKind::Modified)
            && !matches!(c.kind, FileKind::Instructions)
    }) {
        return Some(BroadenReason::ExistingFilesChanged);
    }
    if changes
        .entries
        .iter()
        .any(|c| matches!(c.kind, FileKind::Other))
    {
        return Some(BroadenReason::OtherFilesChanged);
    }
    None
}

/// True when the scenes' frame ranges cover `0..total` without a gap.
pub(super) fn scenes_tile(timeline: &PreviewTimelineResponse) -> bool {
    let total = timeline.total_frames;
    let mut ranges: Vec<(usize, usize)> = timeline
        .scenes
        .iter()
        .map(|s| (s.start_frame, s.end_frame.min(total)))
        .filter(|(start, end)| start < end)
        .collect();
    ranges.sort_unstable();
    let mut cursor = 0;
    for (start, end) in ranges {
        if start > cursor {
            return false;
        }
        cursor = cursor.max(end);
    }
    cursor >= total
}

/// `count` evenly spaced indexes in `0..total` including both ends.
pub fn evenly_spaced(total: usize, count: usize) -> Vec<usize> {
    if total == 0 || count == 0 {
        return vec![];
    }
    let n = count.min(total);
    if n == 1 {
        return vec![0];
    }
    let mut frames: Vec<usize> = (0..n).map(|i| i * (total - 1) / (n - 1)).collect();
    frames.dedup();
    frames
}

pub fn plan_coverage(
    timeline: &PreviewTimelineResponse,
    playhead: usize,
    changes: &ChangeSet,
) -> CoveragePlan {
    plan_coverage_scoped(timeline, playhead, changes, None)
}

/// Plan candidate checks using the frozen submission scope as additional required
/// samples. The compiled identity belongs to the baseline; its absolute frame indexes
/// remain the request, while candidate timing is always taken from `timeline`.
pub fn plan_coverage_scoped(
    timeline: &PreviewTimelineResponse,
    playhead: usize,
    changes: &ChangeSet,
    scope: Option<&TaskScope>,
) -> CoveragePlan {
    let total = timeline.total_frames;
    if total == 0 {
        return CoveragePlan {
            requested_scope: scope.map_or_else(|| "Whole project".into(), TaskScope::label),
            requested_interval: None,
            requested_frames: vec![],
            boundary_frames: vec![],
            sample_frames: vec![],
            rendered_frames: vec![],
            inspect_frames: vec![],
            broadened: None,
            complete: true,
        };
    }
    let playhead = playhead.min(total - 1);
    let mut boundaries = vec![0, total - 1];
    for scene in &timeline.scenes {
        if scene.start_frame < total {
            boundaries.push(scene.start_frame);
        }
        if scene.end_frame > 0 && scene.end_frame <= total {
            boundaries.push(scene.end_frame - 1);
        }
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    let sample = evenly_spaced(total, MAX_SAMPLE_FRAMES);
    let (requested_scope, requested_interval, requested_frames, scope_complete) =
        scoped_samples(scope, total);
    // Render priority: start/end/playhead, scene boundaries, then the sample.
    let mut priority: Vec<usize> = vec![0, total - 1, playhead];
    priority.extend(boundaries.iter().copied());
    priority.extend(requested_frames.iter().copied());
    priority.extend(sample.iter().copied());
    let mut seen = std::collections::BTreeSet::new();
    let mut rendered: Vec<usize> = priority.into_iter().filter(|f| seen.insert(*f)).collect();
    // Only the start/end/playhead and scene-boundary frames are *required*; the sample is
    // best-effort and is what the render cap trims first.
    let required = std::iter::once(playhead)
        .chain(boundaries.iter().copied())
        .chain(requested_frames.iter().copied())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let mut complete = scope_complete && required <= MAX_RENDERED_FRAMES;
    rendered.truncate(MAX_RENDERED_FRAMES);
    rendered.sort_unstable();
    let broadened = broadening(changes).or_else(|| {
        // Targeted coverage cannot be established without scenes that tile the video.
        (!scenes_tile(timeline)).then_some(BroadenReason::ScenesUnavailable)
    });
    let inspect_frames = if broadened.is_some() {
        if total <= MAX_BROADENED_FRAMES {
            (0..total).collect()
        } else {
            complete = false;
            let mut frames = evenly_spaced(total, MAX_BROADENED_FRAMES);
            frames.extend(boundaries.iter().copied());
            frames.push(playhead);
            frames.sort_unstable();
            frames.dedup();
            frames
        }
    } else {
        let mut frames: Vec<usize> = boundaries.clone();
        frames.extend(requested_frames.iter().copied());
        frames.extend(sample.iter().copied());
        frames.push(playhead);
        frames.sort_unstable();
        frames.dedup();
        frames
    };
    CoveragePlan {
        requested_scope,
        requested_interval,
        requested_frames,
        boundary_frames: boundaries,
        sample_frames: sample,
        rendered_frames: rendered,
        inspect_frames,
        broadened,
        complete,
    }
}

pub(super) fn scoped_samples(
    scope: Option<&TaskScope>,
    candidate_total: usize,
) -> (String, Option<[usize; 2]>, Vec<usize>, bool) {
    let Some(scope) = scope else {
        return ("Whole project".into(), None, vec![], true);
    };
    let Some(compiled) = &scope.compiled else {
        return (scope.label(), None, vec![], true);
    };
    if matches!(scope.selection, crate::ScopeSelection::WholeProject) {
        return (
            scope.label(),
            Some([compiled.start_frame, compiled.end_frame]),
            vec![],
            true,
        );
    }

    let mut requested = std::collections::BTreeSet::new();
    requested.extend(compiled.boundary_frames.iter().copied());
    if compiled.start_frame < compiled.end_frame {
        let span = compiled.end_frame - compiled.start_frame;
        requested.extend(
            evenly_spaced(span, MAX_SAMPLE_FRAMES)
                .into_iter()
                .map(|frame| compiled.start_frame + frame),
        );
        requested.insert(compiled.start_frame);
        requested.insert(compiled.end_frame - 1);
    }
    let scope_complete = requested.iter().all(|frame| *frame < candidate_total);
    let requested = requested
        .into_iter()
        .filter(|frame| *frame < candidate_total)
        .collect();
    (
        scope.label(),
        Some([compiled.start_frame, compiled.end_frame]),
        requested,
        scope_complete,
    )
}
