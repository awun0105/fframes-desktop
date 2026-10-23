//! Motion math shared by the components: progress windows, interpolation, staggering,
//! a closed-form spring and deterministic noise. Everything is a pure function of its inputs,
//! so the same frame always draws the same picture on every render thread.

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
