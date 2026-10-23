use crate::{anim::RateFunc, motion, theme::Theme, ui::Themed};

/// How a transition's progress runs from 0 to 1 (Remotion's `linearTiming` and
/// `springTiming`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timing {
    /// `duration` seconds through `easing`.
    Linear { duration: f32, easing: RateFunc },
    /// A damped spring. Without a `duration` the transition lasts until the spring settles;
    /// with one, the spring is stretched or squeezed to settle at that time.
    Spring {
        mass: f32,
        stiffness: f32,
        damping: f32,
        duration: Option<f32>,
    },
}

/// The spring counts as settled once it stays within this of 1 (Remotion's threshold).
const SETTLED: f32 = 0.005;
/// Settle times are measured in steps of this many seconds.
const STEP: f32 = 1.0 / 240.0;
/// A spring that has not settled after this many seconds is cut there.
const LONGEST: f32 = 10.0;

/// A spring of `mass` behaves like a mass-1 spring with stiffness and damping divided by it.
fn unit_spring(mass: f32, stiffness: f32, damping: f32) -> (f32, f32) {
    let mass = mass.max(1e-3);
    (stiffness / mass, damping / mass)
}

/// Seconds until a mass-1 spring stays within [`SETTLED`] of 1.
fn settle_time(stiffness: f32, damping: f32) -> f32 {
    let steps = (LONGEST / STEP) as usize;
    let mut last_out = 0.0;
    for i in 0..=steps {
        let t = i as f32 * STEP;
        if (1.0 - motion::spring(t, stiffness, damping)).abs() > SETTLED {
            last_out = t;
        }
    }
    (last_out + STEP).min(LONGEST)
}

impl Timing {
    /// Length in seconds.
    pub fn duration(&self) -> f32 {
        match *self {
            Timing::Linear { duration, .. } => duration.max(0.0),
            Timing::Spring {
                mass,
                stiffness,
                damping,
                duration,
            } => duration.unwrap_or_else(|| {
                let (k, d) = unit_spring(mass, stiffness, damping);
                settle_time(k, d)
            }),
        }
    }

    /// Progress `t` seconds into a transition that lasts `length` seconds (its
    /// [`duration`](Self::duration), or less when the series had to shorten it): 0 at the
    /// start, 1 from `length` on. A spring may overshoot 1 on the way.
    pub fn progress(&self, t: f32, length: f32) -> f32 {
        if length <= 0.0 || t >= length {
            return 1.0;
        }
        if t <= 0.0 {
            return 0.0;
        }
        match *self {
            Timing::Linear { easing, .. } => easing.apply(t / length, length),
            Timing::Spring {
                mass,
                stiffness,
                damping,
                ..
            } => {
                let (k, d) = unit_spring(mass, stiffness, damping);
                motion::spring(t * settle_time(k, d) / length, k, d)
            }
        }
    }
}

impl Themed for Timing {
    fn themed(t: &Theme) -> Self {
        Timing::Linear {
            duration: t.motion.duration.slow,
            easing: t.motion.easing.standard,
        }
    }
}

impl Default for Timing {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_runs_from_zero_to_one() {
        let t = Timing::Linear {
            duration: 1.0,
            easing: RateFunc::Linear,
        };
        assert_eq!(t.progress(0.0, 1.0), 0.0);
        assert!((t.progress(0.25, 1.0) - 0.25).abs() < 1e-6);
        assert_eq!(t.progress(1.0, 1.0), 1.0);
        // shortened by the series: the same curve over the shorter window
        assert!((t.progress(0.25, 0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn spring_without_duration_ends_when_it_settles() {
        let spring = Timing::Spring {
            mass: 1.0,
            stiffness: 100.0,
            damping: 10.0,
            duration: None,
        };
        let length = spring.duration();
        assert!(length > 0.3 && length < 3.0, "{length}");
        // from the settle time on, the spring itself stays within the threshold
        for i in 0..100 {
            let t = length + i as f32 * 0.05;
            assert!(
                (1.0 - motion::spring(t, 100.0, 10.0)).abs() <= SETTLED,
                "{t}"
            );
        }
        // and just before it, it is still moving
        assert!((1.0 - motion::spring(length - 2.0 * STEP, 100.0, 10.0)).abs() > SETTLED);
        assert_eq!(spring.progress(length, length), 1.0);
        assert!((spring.progress(length - 0.001, length) - 1.0).abs() <= SETTLED);
    }

    #[test]
    fn mass_scales_like_stiffness_and_damping() {
        let heavy = Timing::Spring {
            mass: 2.0,
            stiffness: 200.0,
            damping: 20.0,
            duration: None,
        };
        let light = Timing::Spring {
            mass: 1.0,
            stiffness: 100.0,
            damping: 10.0,
            duration: None,
        };
        assert_eq!(heavy.duration(), light.duration());
    }

    #[test]
    fn spring_with_a_duration_settles_at_it() {
        let spring = Timing::Spring {
            mass: 1.0,
            stiffness: 100.0,
            damping: 10.0,
            duration: Some(0.5),
        };
        assert_eq!(spring.duration(), 0.5);
        assert!((spring.progress(0.499, 0.5) - 1.0).abs() < 0.01);
    }
}
