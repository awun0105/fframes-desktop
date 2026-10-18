//! Animation durations.

pub const FAST_MS: u64 = 120;
/// Collapsible height animation: 0.2 s ease-out.
pub const NORMAL_MS: u64 = 200;

/// Reduced-motion preference. Not wired to the OS yet; animations may consult
/// it once they exist.
pub const fn reduced_motion() -> bool {
    false
}
