//! [`Sim`]: a simulation stepped once, ahead of time, and sampled at any time.
//!
//! `render_frame` runs on several threads in any order, so a simulation cannot advance one
//! frame at a time. Step it in the video's constructor with [`Sim::precompute`] and read it
//! per frame with [`Sim::sample`], which interpolates between the two steps around `t`.

use kurbo::{Point, Vec2};

/// A state that can be blended with another: positions, angles, lists of them.
pub trait Lerp: Clone {
    /// `self` at `t = 0`, `other` at `t = 1`.
    fn lerp(&self, other: &Self, t: f64) -> Self;
}

impl Lerp for f64 {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        self + (other - self) * t
    }
}

impl Lerp for f32 {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        self + (other - self) * t as f32
    }
}

impl Lerp for Point {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        Point::lerp(*self, *other, t)
    }
}

impl Lerp for Vec2 {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        Vec2::lerp(*self, *other, t)
    }
}

impl<A: Lerp, B: Lerp> Lerp for (A, B) {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        (self.0.lerp(&other.0, t), self.1.lerp(&other.1, t))
    }
}

impl<T: Lerp, const N: usize> Lerp for [T; N] {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        std::array::from_fn(|i| self[i].lerp(&other[i], t))
    }
}

impl<T: Lerp> Lerp for Vec<T> {
    /// Element by element; a longer list keeps its extra elements from `self`.
    fn lerp(&self, other: &Self, t: f64) -> Self {
        self.iter()
            .enumerate()
            .map(|(i, a)| other.get(i).map_or_else(|| a.clone(), |b| a.lerp(b, t)))
            .collect()
    }
}

/// The states of a simulation every `dt` seconds from 0, computed once.
#[derive(Debug, Clone, PartialEq)]
pub struct Sim<S> {
    dt: f64,
    states: Vec<S>,
}

impl<S: Lerp> Sim<S> {
    /// Steps `initial` `steps` times: `step(state, dt)` returns the next state. The result
    /// holds `steps + 1` states, at `0, dt, 2 dt, ..`. A `dt` that is not positive gives a
    /// simulation that stays at `initial`.
    pub fn precompute(
        dt: f64,
        steps: usize,
        initial: S,
        mut step: impl FnMut(&S, f64) -> S,
    ) -> Self {
        let steps = if dt > 0.0 && dt.is_finite() { steps } else { 0 };
        let mut states = Vec::with_capacity(steps + 1);
        states.push(initial);
        for i in 0..steps {
            let next = step(&states[i], dt);
            states.push(next);
        }
        Self { dt, states }
    }

    /// The state at `t` seconds, interpolated between the two steps around it, and clamped
    /// to the first state before 0 and the last after [`Sim::duration`].
    pub fn sample(&self, t: f64) -> S {
        let last = self.states.len() - 1;
        if last == 0 || t.is_nan() || t <= 0.0 {
            return self.states[0].clone();
        }
        let at = t / self.dt;
        if at >= last as f64 {
            return self.states[last].clone();
        }
        let i = at.floor() as usize;
        self.states[i].lerp(&self.states[i + 1], at - i as f64)
    }

    /// Seconds from the first state to the last.
    pub fn duration(&self) -> f64 {
        (self.states.len() - 1) as f64 * self.dt.max(0.0)
    }

    /// Seconds between two states.
    pub fn dt(&self) -> f64 {
        self.dt
    }

    /// Every computed state, the first at 0.
    pub fn states(&self) -> &[S] {
        &self.states
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampling_interpolates_and_clamps() {
        // a body falling from rest, explicit Euler on (position, velocity)
        let sim = Sim::precompute(0.5, 4, (0.0_f64, 0.0_f64), |&(x, v), dt| {
            (x + v * dt, v + 10.0 * dt)
        });
        assert_eq!(sim.states().len(), 5);
        assert_eq!(sim.duration(), 2.0);
        assert_eq!(sim.states()[2], (2.5, 10.0));
        // halfway between the states at 0.5 s (0, 5) and 1 s (2.5, 10)
        assert_eq!(sim.sample(0.75), (1.25, 7.5));
        assert_eq!(sim.sample(-1.0), (0.0, 0.0));
        assert_eq!(sim.sample(f64::NAN), (0.0, 0.0));
        assert_eq!(sim.sample(99.0), sim.states()[4]);
        assert_eq!(sim.sample(2.0), sim.states()[4]);
    }

    #[test]
    fn a_degenerate_step_stays_put() {
        let sim = Sim::precompute(0.0, 10, 3.0_f32, |x, _| x + 1.0);
        assert_eq!(sim.states(), &[3.0]);
        assert_eq!(sim.sample(5.0), 3.0);
        assert_eq!(sim.duration(), 0.0);
    }

    #[test]
    fn lists_and_points_blend() {
        let a = vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
        let b = vec![Point::new(2.0, 2.0), Point::new(10.0, 4.0)];
        assert_eq!(
            a.lerp(&b, 0.5),
            vec![Point::new(1.0, 1.0), Point::new(10.0, 2.0)]
        );
        assert_eq!([1.0_f64, 3.0].lerp(&[3.0, 5.0], 0.5), [2.0, 4.0]);
    }
}
