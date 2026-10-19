//! What a preview worker reports about a candidate, and PCM scanning.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeError {
    pub kind: FailureKind,
    pub message: String,
}

impl ProbeError {
    pub fn source(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Source,
            message: message.into(),
        }
    }
    pub fn environment(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Environment,
            message: message.into(),
        }
    }
}

/// Statistics of prepared PCM, in sample frames, gathered in one streaming pass.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PcmStats {
    pub sample_frames: u64,
    pub non_finite: u64,
    pub peak: f32,
    pub clipped: u64,
    /// Merged ranges `[start, end)` of 10 ms buckets whose peak exceeds the silence floor
    /// (bounded; only used to tell which tracks are audible).
    pub active: Vec<(u64, u64)>,
    /// More ranges existed than are retained: per-track audibility is incomplete.
    pub activity_truncated: bool,
    /// The merged track windows the scan checked every active bucket against. Placement
    /// is only verified when these equal the windows the compiled timeline implies
    /// ([`placement_windows`]); a scan that was given none cannot vouch for placement.
    pub checked_windows: Vec<(u64, u64)>,
    /// First active `[start, end)` bucket outside every checked window, retained
    /// regardless of how fragmented the activity map became (sticky).
    pub first_outside: Option<(u64, u64)>,
}

/// Sorted, merged `[start, end)` sample-frame windows of the compiled audio tracks at
/// `sample_rate`, padded by 100 ms on each side for fades and rounding.
pub fn placement_windows(timeline: &PreviewTimelineResponse, sample_rate: u32) -> Vec<(u64, u64)> {
    let rate = f64::from(sample_rate);
    merge_windows(
        timeline
            .audio_tracks
            .iter()
            .filter(|t| t.end_seconds > t.start_seconds)
            .map(|t| {
                (
                    ((t.start_seconds - 0.1).max(0.) * rate) as u64,
                    ((t.end_seconds + 0.1) * rate) as u64,
                )
            })
            .collect(),
    )
}

pub(super) fn merge_windows(mut windows: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    windows.retain(|(start, end)| start < end);
    windows.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::with_capacity(windows.len());
    for (start, end) in windows {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

#[derive(Debug, Clone)]
pub struct AudioProbe {
    pub descriptor: PreparedAudioDescriptor,
    pub stats: PcmStats,
}

/// The worker-facing operations validation needs. The app implements it over an owned
/// preview worker; tests use deterministic fakes.
pub trait CandidateProbe {
    fn timeline(&mut self) -> Result<PreviewTimelineResponse, ProbeError>;
    /// Inspect at most [`MAX_INSPECT_FRAMES`] frames.
    fn inspect(&mut self, frames: &[usize]) -> Result<InspectResponse, ProbeError>;
    fn render(&mut self, frame: usize) -> Result<PreviewFrame, ProbeError>;
    /// Prepare the mix and scan its PCM locally.
    fn audio(&mut self) -> Result<AudioProbe, ProbeError>;
    /// The renderer the worker reported (`cpu`, `skia-cpu`, `skia-gpu`). Defaults to `cpu`.
    fn backend(&self) -> String {
        "cpu".into()
    }
    /// Capabilities the worker's hello declared unavailable. Defaults to none.
    fn capability_gaps(&self) -> Vec<String> {
        Vec::new()
    }
    /// True once the run was stopped (Stop, project close, shutdown); polled between
    /// worker requests. Defaults to never.
    fn cancelled(&self) -> bool {
        false
    }
}

/// Stream prepared PCM (`f32le` stereo interleaved) once and gather [`PcmStats`]. Every
/// active 10 ms bucket is checked against `windows` (sample-frame ranges, see
/// [`placement_windows`]) while streaming, so audible content outside the compiled
/// tracks is recorded even when the bounded activity map overflows.
pub fn scan_pcm(
    mut reader: impl Read,
    sample_rate: u32,
    windows: &[(u64, u64)],
    cancelled: &dyn Fn() -> bool,
) -> std::io::Result<PcmStats> {
    let bucket = (sample_rate as u64 / 100).max(1);
    let windows = merge_windows(windows.to_vec());
    let mut stats = PcmStats {
        checked_windows: windows.clone(),
        ..PcmStats::default()
    };
    let mut buffer = vec![0u8; 64 * 1024];
    let mut carry = 0usize;
    let mut bucket_peak = 0f32;
    let mut bucket_fill = 0u64;
    let mut bucket_start = 0u64;
    let mut window = 0usize;
    let mut flush = |stats: &mut PcmStats, start: u64, end: u64, peak: f32| {
        if peak <= SILENCE_PEAK {
            return;
        }
        // Buckets arrive in order: skip windows that ended before this one.
        while window < windows.len() && windows[window].1 <= start {
            window += 1;
        }
        let inside = windows
            .get(window)
            .is_some_and(|&(ws, we)| ws <= start && end <= we);
        if !inside && stats.first_outside.is_none() {
            stats.first_outside = Some((start, end));
        }
        if let Some(last) = stats.active.last_mut()
            && last.1 == start
        {
            last.1 = end;
        } else if stats.active.len() >= MAX_ACTIVITY_RANGES {
            stats.activity_truncated = true;
        } else {
            stats.active.push((start, end));
        }
    };
    loop {
        if cancelled() {
            return Err(std::io::Error::other("PCM scan cancelled"));
        }
        let read = reader.read(&mut buffer[carry..])?;
        if read == 0 {
            break;
        }
        let available = carry + read;
        let whole = available - available % 8;
        for frame in buffer[..whole].as_chunks::<8>().0 {
            let l = f32::from_le_bytes(frame[0..4].try_into().expect("4 bytes"));
            let r = f32::from_le_bytes(frame[4..8].try_into().expect("4 bytes"));
            let mut peak = 0f32;
            for v in [l, r] {
                if !v.is_finite() {
                    stats.non_finite += 1;
                    continue;
                }
                let a = v.abs();
                peak = peak.max(a);
                if a >= 1.0 {
                    stats.clipped += 1;
                }
            }
            stats.peak = stats.peak.max(peak);
            bucket_peak = bucket_peak.max(peak);
            bucket_fill += 1;
            stats.sample_frames += 1;
            if bucket_fill == bucket {
                flush(
                    &mut stats,
                    bucket_start,
                    bucket_start + bucket_fill,
                    bucket_peak,
                );
                bucket_start += bucket_fill;
                bucket_fill = 0;
                bucket_peak = 0.;
            }
        }
        buffer.copy_within(whole..available, 0);
        carry = available - whole;
    }
    if bucket_fill > 0 {
        flush(
            &mut stats,
            bucket_start,
            bucket_start + bucket_fill,
            bucket_peak,
        );
    }
    Ok(stats)
}
