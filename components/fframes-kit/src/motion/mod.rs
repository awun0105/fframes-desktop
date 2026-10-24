//! Motion math shared by the components: progress windows, interpolation, staggering,
//! a closed-form spring and deterministic noise. Everything is a pure function of its inputs,
//! so the same frame always draws the same picture on every render thread.

mod beat;
mod format;
mod noise;

pub use beat::Beat;
pub use format::{TimecodeError, thousands, timecode};
pub use noise::{NoiseSeed, noise_2d, noise_3d, noise_4d};

/// Clamps `x` to `0..=1`.
pub fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// Linear progress of `t` from `start` to `end`, clamped to `0..=1`. An empty window
/// (`end <= start`) is a step at `start`.
pub fn progress(t: f32, start: f32, end: f32) -> f32 {
    if end <= start {
        return if t >= start { 1.0 } else { 0.0 };
    }
    clamp01((t - start) / (end - start))
}

/// Interpolates from `a` to `b`; `t` is not clamped.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Delay of the `i`th item of a staggered group, `step` seconds apart.
pub fn stagger(i: usize, step: f32) -> f32 {
    i as f32 * step
}

/// Damped spring from 0 to 1 with mass 1, `t` seconds after release. Under-damped springs
/// overshoot 1 before they settle; `damping >= 2 * sqrt(stiffness)` never overshoots.
pub fn spring(t: f32, stiffness: f32, damping: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    let w0 = stiffness.sqrt();
    let zeta = damping / (2.0 * w0);
    if zeta < 1.0 {
        let wd = w0 * (1.0 - zeta * zeta).sqrt();
        1.0 - (-zeta * w0 * t).exp() * ((wd * t).cos() + zeta * w0 / wd * (wd * t).sin())
    } else {
        1.0 - (1.0 + w0 * t) * (-w0 * t).exp()
    }
}

/// Exponential ease out of `t` in `0..=1`: fast, then a long settle.
pub fn expo_out(t: f32) -> f32 {
    let t = clamp01(t);
    if t >= 1.0 {
        1.0
    } else {
        1.0 - 2f32.powf(-10.0 * t)
    }
}

/// Exponential ease in: a slow start, then fast.
pub fn expo_in(t: f32) -> f32 {
    let t = clamp01(t);
    if t <= 0.0 {
        0.0
    } else {
        2f32.powf(10.0 * t - 10.0)
    }
}

/// Cubic ease in and out.
pub fn cubic_in_out(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Quartic ease out.
pub fn quart_out(t: f32) -> f32 {
    1.0 - (1.0 - clamp01(t)).powi(4)
}

/// Maps an integer to a pseudo-random value in `0..1`. The same input always gives the same
/// value.
pub fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// A seeded random sequence (`SplitMix64`) for data built once in a constructor: particle
/// positions, samples. The same seed gives the same numbers on every machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number in `(0, 1]` from 53 random bits; never 0, so its logarithm is finite.
    pub fn unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 1.0) / (1u64 << 53) as f64
    }

    /// A number between `min` and `max`.
    pub fn range(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.unit()
    }
}

/// Smooth 1D gradient noise in `-1..=1`, the same for the same `seed` and `x`. Each integer
/// `x` has a random slope and the value 0, and a quintic curve blends the two neighbours, so
/// motion driven by it never stops and starts again.
pub fn noise(seed: u32, x: f32) -> f32 {
    let i = x.floor();
    let f = x - i;
    let cell = (i as i32 as u32).wrapping_add(seed.wrapping_mul(1013));
    let g0 = hash(cell) * 2.0 - 1.0;
    let g1 = hash(cell.wrapping_add(1)) * 2.0 - 1.0;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let (a, b) = (g0 * f, g1 * (f - 1.0));
    // the largest value of the blend is 0.5
    (lerp(a, b, u) * 2.0).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_clamps_and_steps() {
        assert_eq!(progress(-1.0, 0.0, 2.0), 0.0);
        assert_eq!(progress(1.0, 0.0, 2.0), 0.5);
        assert_eq!(progress(3.0, 0.0, 2.0), 1.0);
        assert_eq!(progress(0.9, 1.0, 1.0), 0.0);
        assert_eq!(progress(1.0, 1.0, 1.0), 1.0);
    }

    #[test]
    fn rng_is_seeded_and_in_range() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        let first: Vec<u64> = (0..4).map(|_| a.next_u64()).collect();
        assert_eq!(first, (0..4).map(|_| b.next_u64()).collect::<Vec<_>>());
        assert_ne!(Rng::new(8).next_u64(), first[0]);
        // the first output of SplitMix64 for seed 0
        assert_eq!(Rng::new(0).next_u64(), 0xe220_a839_7b1d_cdaf);
        for _ in 0..1000 {
            let u = a.unit();
            assert!(u > 0.0 && u <= 1.0);
            let r = a.range(-2.0, 3.0);
            assert!((-2.0..=3.0).contains(&r));
        }
    }

    #[test]
    fn lerp_and_stagger() {
        assert_eq!(lerp(10.0, 20.0, 0.25), 12.5);
        assert_eq!(stagger(3, 0.05), 0.15_f32);
    }

    #[test]
    fn spring_starts_at_zero_and_settles_to_one() {
        assert_eq!(spring(0.0, 300.0, 26.0), 0.0);
        assert!((spring(3.0, 300.0, 26.0) - 1.0).abs() < 1e-3);
        // critically damped and over-damped springs settle too
        assert!((spring(3.0, 100.0, 20.0) - 1.0).abs() < 1e-3);
        assert!((spring(6.0, 100.0, 40.0) - 1.0).abs() < 1e-2);
        // an under-damped spring overshoots
        let peak = (1..100)
            .map(|i| spring(i as f32 * 0.01, 170.0, 10.0))
            .fold(0.0, f32::max);
        assert!(peak > 1.0);
    }

    #[test]
    fn easing_curves_run_from_zero_to_one() {
        for f in [expo_out, expo_in, cubic_in_out, quart_out] {
            assert!(f(0.0).abs() < 1e-3);
            assert_eq!(f(1.0), 1.0);
            assert!((0.0..=1.0).contains(&f(0.5)));
        }
        assert_eq!(cubic_in_out(0.5), 0.5);
        assert!(expo_out(0.2) > expo_in(0.2));
    }

    #[test]
    fn noise_is_deterministic_and_bounded() {
        for i in 0..200 {
            let x = i as f32 * 0.173;
            assert_eq!(noise(7, x), noise(7, x));
            assert!((-1.0..=1.0).contains(&noise(7, x)));
        }
        assert_ne!(noise(1, 0.5), noise(2, 0.5));
        assert_eq!(noise(3, 4.0), 0.0);
        assert!((0.0..1.0).contains(&hash(42)));
        assert_eq!(hash(42), hash(42));
    }
}
