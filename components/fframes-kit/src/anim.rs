//! Animated props and Manim-named animations: [`Anim`], [`RateFunc`], the [`Enter`] and
//! [`Exit`] traits, [`FadeIn`] and [`FadeOut`].

use fframes::{
    Svgr, Transform,
    animation::{Animatable, AnimationRuntime, Easing, KeyFramesAnimation},
};

use crate::{
    motion::{clamp01, progress},
    theme::Theme,
    ui::{Themed, Ui},
};

/// A prop that is either a fixed value or a `timeline!` stored in the video.
///
/// `radius: 120.0.into()` is fixed; `radius: (&self.pulse).into()` follows the stored
/// keyframes. Read it with [`Ui::sample`].
#[derive(Debug, Clone, Copy)]
pub enum Anim<'a, T: Animatable + Copy + Default> {
    Fixed(T),
    Keyframes(&'a KeyFramesAnimation<T>),
}

impl<T: Animatable + Copy + Default> From<T> for Anim<'_, T> {
    fn from(value: T) -> Self {
        Anim::Fixed(value)
    }
}

impl<'a, T: Animatable + Copy + Default> From<&'a KeyFramesAnimation<T>> for Anim<'a, T> {
    fn from(keyframes: &'a KeyFramesAnimation<T>) -> Self {
        Anim::Keyframes(keyframes)
    }
}

/// Manim's `ValueTracker`: a number that changes over time and drives other props. Store a
/// `timeline!` of the value in the video (Manim's `tracker.animate.set_value(..)` calls
/// become its keyframes), read it with [`Ui::sample`] where Manim calls `get_value()`, and
/// build what depends on it in `render_frame`, which runs every frame like
/// `always_redraw`. [`DecimalNumber`](crate::text::DecimalNumber) and
/// [`Variable`](crate::text::Variable) show one.
///
/// ```
/// use fframes_kit::prelude::*;
/// use fframes::animation::KeyFramesAnimation;
///
/// fn dot<'a>(ui: &mut Ui<'a, '_, '_>, x: &'a KeyFramesAnimation<f32>) -> Svgr<'a> {
///     let x: ValueTracker = x.into();
///     let value = ui.sample(&x);
///     let at = ui.space.to_px(f64::from(value), 0.0);
///     ui.show(Dot { cx: at.x as f32, cy: at.y as f32, ..ui.defaults() })
/// }
/// ```
pub type ValueTracker<'a> = Anim<'a, f32>;

/// How an animation's progress maps to time, with Manim's names. `Smooth` is the default.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RateFunc {
    /// Slow start and end (Manim's `smooth`, a sigmoid).
    #[default]
    Smooth,
    Linear,
    /// The first half of `Smooth`, sped up: slow start, fast end.
    RushInto,
    /// The second half of `Smooth`, sped up: fast start, slow end.
    RushFrom,
    /// Goes to 1 at the middle and back to 0.
    ThereAndBack,
    /// `Smooth` twice: each half eases in and out.
    DoubleSmooth,
    /// An fframes easing; a spring runs on seconds and may overshoot.
    Easing(Easing),
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Manim's `smooth` with its default inflection 10.
fn smooth(t: f32) -> f32 {
    let error = sigmoid(-5.0);
    clamp01((sigmoid(10.0 * (t - 0.5)) - error) / (1.0 - 2.0 * error))
}

impl RateFunc {
    /// The eased value of linear progress `t` (`0..=1`) of an animation that runs
    /// `run_time` seconds.
    pub fn apply(&self, t: f32, run_time: f32) -> f32 {
        let t = clamp01(t);
        match self {
            RateFunc::Smooth => smooth(t),
            RateFunc::Linear => t,
            RateFunc::RushInto => 2.0 * smooth(t / 2.0),
            RateFunc::RushFrom => 2.0 * smooth(t / 2.0 + 0.5) - 1.0,
            RateFunc::ThereAndBack => smooth(if t < 0.5 { 2.0 * t } else { 2.0 * (1.0 - t) }),
            RateFunc::DoubleSmooth => {
                if t < 0.5 {
                    0.5 * smooth(2.0 * t)
                } else {
                    f32::midpoint(1.0, smooth(2.0 * t - 1.0))
                }
            }
            RateFunc::Easing(easing) => {
                if t >= 1.0 && !matches!(easing, Easing::Spring { .. }) {
                    return 1.0;
                }
                let run_time = run_time.max(f32::EPSILON);
                AnimationRuntime::new(run_time, easing).solve(&(t * run_time))
            }
        }
    }
}

/// Animates content in, with local time starting at the enclosing container's start.
pub trait Enter {
    fn apply<'a>(&self, ui: &Ui<'a, '_, '_>, content: Svgr<'a>) -> Svgr<'a>;
}

/// Animates content out. Times count back from the end of `ui.duration`; without a known
/// end (`ui.duration` is `None`) the content is returned unchanged.
pub trait Exit {
    fn apply<'a>(&self, ui: &Ui<'a, '_, '_>, content: Svgr<'a>) -> Svgr<'a>;
}

/// `content` through the optional enter and exit fades.
pub(crate) fn fade<'a>(
    ui: &Ui<'a, '_, '_>,
    content: Svgr<'a>,
    enter: Option<&FadeIn>,
    exit: Option<&FadeOut>,
) -> Svgr<'a> {
    let content = match enter {
        Some(e) => e.apply(ui, content),
        None => content,
    };
    match exit {
        Some(x) => x.apply(ui, content),
        None => content,
    }
}

/// Fades content in while it moves by `shift` to its place (Manim's `FadeIn(shift=..)`).
///
/// `shift` is in canvas units, `y` down: `(0.0, -40.0)` rises 40 units into place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FadeIn {
    pub shift: (f32, f32),
    /// Seconds after the local start.
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Themed for FadeIn {
    fn themed(theme: &Theme) -> Self {
        Self {
            shift: (0.0, 0.0),
            delay: 0.0,
            run_time: theme.motion.duration.enter,
            rate_func: theme.motion.easing.enter,
        }
    }
}

impl Default for FadeIn {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Fades content out while it moves by `shift` (Manim's `FadeOut(shift=..)`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FadeOut {
    pub shift: (f32, f32),
    /// Seconds between the end of the fade and the end of `ui.duration`.
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Themed for FadeOut {
    fn themed(theme: &Theme) -> Self {
        Self {
            shift: (0.0, 0.0),
            delay: 0.0,
            run_time: theme.motion.duration.exit,
            rate_func: theme.motion.easing.exit,
        }
    }
}

impl Default for FadeOut {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// `content` at `visible` (0 hidden, 1 in place) with the remaining `shift` offset.
fn faded(content: Svgr<'_>, visible: f32, dx: f32, dy: f32) -> Svgr<'_> {
    if visible >= 1.0 && dx == 0.0 && dy == 0.0 {
        return content;
    }
    let opacity = clamp01(visible);
    let transform = Transform::translate(dx, dy);
    fframes::svgr!(<g opacity={opacity} transform={transform}>{content}</g>)
}

impl Enter for FadeIn {
    fn apply<'a>(&self, ui: &Ui<'a, '_, '_>, content: Svgr<'a>) -> Svgr<'a> {
        let t = progress(ui.seconds(), self.delay, self.delay + self.run_time);
        let p = self.rate_func.apply(t, self.run_time);
        let rest = 1.0 - p;
        faded(content, p, -self.shift.0 * rest, -self.shift.1 * rest)
    }
}

impl Exit for FadeOut {
    fn apply<'a>(&self, ui: &Ui<'a, '_, '_>, content: Svgr<'a>) -> Svgr<'a> {
        let Some(end) = ui.duration_seconds() else {
            return content;
        };
        let stop = end - self.delay;
        let t = progress(ui.seconds(), stop - self.run_time, stop);
        if t <= 0.0 {
            return content;
        }
        let p = self.rate_func.apply(t, self.run_time);
        faded(content, 1.0 - p, self.shift.0 * p, self.shift.1 * p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_funcs_start_at_zero_and_end_at_one() {
        for rate in [
            RateFunc::Smooth,
            RateFunc::Linear,
            RateFunc::RushInto,
            RateFunc::RushFrom,
            RateFunc::DoubleSmooth,
            RateFunc::Easing(Easing::EaseInOut),
            RateFunc::Easing(Easing::Linear),
        ] {
            assert!(rate.apply(0.0, 1.0).abs() < 1e-4, "{rate:?} at 0");
            assert!((rate.apply(1.0, 1.0) - 1.0).abs() < 1e-4, "{rate:?} at 1");
            assert!(
                rate.apply(0.3, 1.0) <= rate.apply(0.6, 1.0),
                "{rate:?} rises"
            );
        }
        assert!((RateFunc::Smooth.apply(0.5, 1.0) - 0.5).abs() < 1e-4);
        assert!((RateFunc::ThereAndBack.apply(0.5, 1.0) - 1.0).abs() < 1e-4);
        assert!(RateFunc::ThereAndBack.apply(1.0, 1.0).abs() < 1e-4);
        assert!(RateFunc::RushInto.apply(0.5, 1.0) < 0.5);
        assert!(RateFunc::RushFrom.apply(0.5, 1.0) > 0.5);
    }

    #[test]
    fn spring_easing_settles() {
        let spring = RateFunc::Easing(Easing::Spring {
            mass: 1.0,
            stiffness: 300.0,
            damping: 26.0,
        });
        assert!((spring.apply(1.0, 2.0) - 1.0).abs() < 1e-2);
    }

    #[test]
    fn anim_from_value() {
        let a: Anim<'_, f32> = 3.0.into();
        assert!(matches!(a, Anim::Fixed(v) if v == 3.0));
    }
}
