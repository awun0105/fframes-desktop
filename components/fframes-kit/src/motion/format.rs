//! Numbers as text: timecodes and grouped digits.

use std::fmt;

/// Why [`timecode`] cannot format a frame number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimecodeError {
    /// The frame rate is 0.
    ZeroFps,
}

impl fmt::Display for TimecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimecodeError::ZeroFps => write!(f, "a timecode needs a frame rate above 0"),
        }
    }
}

impl std::error::Error for TimecodeError {}

/// The timecode `hh:mm:ss:ff` of frame `frames` at `fps` frames per second.
pub fn timecode(frames: usize, fps: usize) -> Result<String, TimecodeError> {
    if fps == 0 {
        return Err(TimecodeError::ZeroFps);
    }
    let ff = frames % fps;
    let seconds = frames / fps;
    let (h, m, s) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);
    Ok(format!("{h:02}:{m:02}:{s:02}:{ff:02}"))
}

/// `n` with a comma between groups of three digits: `12480` is `"12,480"`.
pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timecodes() {
        assert_eq!(timecode(0, 60).unwrap(), "00:00:00:00");
        assert_eq!(timecode(61 * 60 + 59, 60).unwrap(), "00:01:01:59");
        assert_eq!(timecode(3600 * 30 + 15, 30).unwrap(), "01:00:00:15");
        assert_eq!(timecode(10, 0), Err(TimecodeError::ZeroFps));
    }

    #[test]
    fn thousands_groups_digits() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(12_480), "12,480");
        assert_eq!(thousands(1_000_000), "1,000,000");
    }
}
