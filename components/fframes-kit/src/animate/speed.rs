use fframes::Svgr;

use crate::{
    anim::{Anim, RateFunc},
    motion::lerp,
    text::DecimalNumber,
    ui::{Children, Component, Ui},
};

use super::enter_alpha;

/// Plays its children faster or slower over the first `run_time` seconds (Manim's
/// `ChangeSpeed`). `speed` is a curve of `(fraction of run_time, speed)` points, straight
/// between them and flat beyond them; no points is speed 1. The children see their time
/// `t` as `run_time · S(t / run_time) / S(1)`, `S` the area under the curve, so they start
/// and end where they would anyway and only the pace in between changes. After `run_time`
/// their time is the real one again.
pub struct ChangeSpeed<'a> {
    pub run_time: f32,
    pub speed: Vec<(f32, f32)>,
    pub children: Children<'a>,
}

impl std::fmt::Debug for ChangeSpeed<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeSpeed")
            .field("run_time", &self.run_time)
            .field("speed", &self.speed)
            .finish_non_exhaustive()
    }
}

impl ChangeSpeed<'_> {
    /// The speed at fraction `u`.
    fn speed_at(&self, u: f32) -> f32 {
        let (Some(first), Some(last)) = (self.speed.first(), self.speed.last()) else {
            return 1.0;
        };
        if u <= first.0 {
            return first.1;
        }
        for w in self.speed.windows(2) {
            let ((u0, s0), (u1, s1)) = (w[0], w[1]);
            if u <= u1 {
                let t = if u1 > u0 { (u - u0) / (u1 - u0) } else { 1.0 };
                return lerp(s0, s1, t);
            }
        }
        last.1
    }

    /// The area under the speed curve from 0 to `u`, exact for the straight pieces.
    fn area(&self, u: f32) -> f32 {
        let mut knots: Vec<f32> = self
            .speed
            .iter()
            .map(|&(k, _)| k)
            .filter(|&k| k > 0.0 && k < u)
            .collect();
        knots.insert(0, 0.0);
        knots.push(u);
        knots
            .windows(2)
            .map(|w| (w[1] - w[0]) * (self.speed_at(w[0]) + self.speed_at(w[1])) / 2.0)
            .sum()
    }

    /// The time the children see at local second `t`.
    pub fn remap(&self, t: f32) -> f32 {
        let r = self.run_time;
        if r <= 0.0 || t <= 0.0 || t >= r {
            return t;
        }
        let total = self.area(1.0);
        if total <= 0.0 {
            return t;
        }
        r * self.area(t / r) / total
    }
}

impl<'a> Component<'a> for ChangeSpeed<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let tau = self.remap(t);
        if (tau - t).abs() < 1e-6 {
            return (self.children)(ui);
        }
        match ui.at_local(tau) {
            Some(mut at) => (self.children)(&mut at),
            None => Svgr::empty(),
        }
    }
}

/// Counts a [`DecimalNumber`] from the value of its tracker to `target` (Manim's
/// `ChangeDecimalToValue`).
#[derive(Debug, Clone, Copy)]
pub struct ChangeDecimalToValue<'a> {
    pub number: DecimalNumber<'a>,
    pub target: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl<'a> ChangeDecimalToValue<'a> {
    /// In Manim's 1 second, smooth.
    pub fn new(number: DecimalNumber<'a>, target: f32) -> Self {
        Self {
            number,
            target,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for ChangeDecimalToValue<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let value = lerp(ui.sample(&self.number.value), self.target, p);
        DecimalNumber {
            value: Anim::Fixed(value),
            ..self.number
        }
        .render(ui)
    }
}

/// Shows `number_update_func(alpha)` in a [`DecimalNumber`] while the animation's progress
/// runs from 0 to 1 (Manim's `ChangingDecimal`).
#[derive(Debug, Clone, Copy)]
pub struct ChangingDecimal<'a> {
    pub number: DecimalNumber<'a>,
    pub number_update_func: fn(f32) -> f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl<'a> ChangingDecimal<'a> {
    /// In Manim's 1 second, smooth.
    pub fn new(number: DecimalNumber<'a>, number_update_func: fn(f32) -> f32) -> Self {
        Self {
            number,
            number_update_func,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for ChangingDecimal<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        DecimalNumber {
            value: Anim::Fixed((self.number_update_func)(p)),
            ..self.number
        }
        .render(ui)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(speed: Vec<(f32, f32)>) -> ChangeSpeed<'static> {
        ChangeSpeed {
            run_time: 4.0,
            speed,
            children: Box::new(|_| Svgr::empty()),
        }
    }

    #[test]
    fn change_speed_keeps_the_ends() {
        for speed in [
            vec![],
            vec![(0.0, 0.2), (1.0, 3.0)],
            vec![(0.3, 0.1), (0.5, 5.0), (0.8, 1.0)],
        ] {
            let c = change(speed);
            assert_eq!(c.remap(0.0), 0.0);
            assert!((c.remap(4.0) - 4.0).abs() < 1e-6);
            assert!((c.remap(3.9999) - 4.0).abs() < 1e-3, "{}", c.remap(3.9999));
            assert_eq!(c.remap(6.0), 6.0);
        }
    }

    #[test]
    fn slow_then_fast_lags_behind_halfway() {
        // speed 0 to 2 in a straight line: S(u) = u², so halfway is a quarter of the way
        let c = change(vec![(0.0, 0.0), (1.0, 2.0)]);
        assert!((c.remap(2.0) - 1.0).abs() < 1e-5, "{}", c.remap(2.0));
        assert_eq!(change(vec![]).remap(1.5), 1.5);
    }
}
