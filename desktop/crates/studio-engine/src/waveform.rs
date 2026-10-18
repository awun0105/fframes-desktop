//! Waveform peaks of prepared PCM, after Cap's `waveform_peaks`: the mean absolute
//! sample of every tenth of a second, in dBFS, with silence at [`WAVEFORM_MIN_DB`].

/// Peaks per second of audio; the view indexes the table at this rate.
pub const WAVEFORM_PEAKS_PER_SECOND: u32 = 10;
/// The level of a silent block, and the floor the view draws against.
pub const WAVEFORM_MIN_DB: f32 = -60.0;

/// Folds interleaved samples into peaks as they stream past, so the pass that
/// validates a PCM file computes its waveform without a second read.
#[derive(Debug, Clone)]
pub struct PeakAccumulator {
    block: u64,
    count: u64,
    sum: f64,
    peaks: Vec<f32>,
}

impl PeakAccumulator {
    pub fn new(sample_rate: u32, channels: u8) -> Self {
        Self {
            block: (u64::from(sample_rate / WAVEFORM_PEAKS_PER_SECOND) * u64::from(channels))
                .max(1),
            count: 0,
            sum: 0.0,
            peaks: Vec::new(),
        }
    }

    pub fn push(&mut self, sample: f32) {
        self.sum += f64::from(sample.abs());
        self.count += 1;
        if self.count == self.block {
            self.flush();
        }
    }

    /// The peaks, including a last partial block.
    pub fn finish(mut self) -> Vec<f32> {
        if self.count > 0 {
            self.flush();
        }
        self.peaks
    }

    fn flush(&mut self) {
        let mean = self.sum / self.count as f64;
        self.peaks.push(if mean > 0.0 {
            (20.0 * mean.log10()) as f32
        } else {
            WAVEFORM_MIN_DB
        });
        self.sum = 0.0;
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_peak_per_tenth_of_a_second_in_dbfs() {
        // 0.25 s of stereo at 40 Hz: blocks of 4 frames, 8 samples.
        let mut peaks = PeakAccumulator::new(40, 2);
        for frame in 0..10 {
            let level = match frame {
                0..4 => 1.0,
                4..8 => 0.0,
                _ => 0.1,
            };
            peaks.push(level);
            peaks.push(-level);
        }
        let peaks = peaks.finish();
        assert_eq!(peaks.len(), 3);
        assert_eq!(peaks[0], 0.0);
        assert_eq!(peaks[1], WAVEFORM_MIN_DB);
        assert!((peaks[2] + 20.0).abs() < 1e-4, "{}", peaks[2]);
    }

    #[test]
    fn no_samples_no_peaks() {
        assert!(PeakAccumulator::new(48_000, 2).finish().is_empty());
        // A rate below the peak rate still makes blocks.
        let mut peaks = PeakAccumulator::new(5, 1);
        peaks.push(0.5);
        assert_eq!(peaks.finish().len(), 1);
    }
}
