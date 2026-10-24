use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::RateFunc,
    geometry::{Geometry, Look, Shape},
    motion::{lerp, progress},
    paths,
    ui::{Component, Ui},
};

use super::{ShapeAnimation, Write, create::themed, enter_alpha};

/// Shows the shape for `time_on`, hides it for `time_off`, `blinks` times, from `delay`
/// seconds in (Manim's `Blink`). It is a step, so it has no rate function; with
/// `hide_at_end` it stays hidden afterwards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blink {
    pub blinks: usize,
    pub time_on: f32,
    pub time_off: f32,
    pub hide_at_end: bool,
    pub delay: f32,
}

themed!(Blink, |_t| Blink {
    blinks: 1,
    time_on: 0.5,
    time_off: 0.5,
    hide_at_end: false,
    delay: 0.0,
});

impl Blink {
    /// Whether the shape shows `t` seconds after the local start.
    pub fn on(&self, t: f32) -> bool {
        let t = t - self.delay;
        let period = self.time_on + self.time_off;
        if t < 0.0 || period <= 0.0 {
            return true;
        }
        if t >= period * self.blinks as f32 {
            return !self.hide_at_end;
        }
        t % period < self.time_on
    }
}

impl ShapeAnimation for Blink {
    fn look(&self, ui: &Ui<'_, '_, '_>, _shape: &Shape, look: &mut Look) {
        if !self.on(ui.seconds()) {
            look.opacity = 0.0;
        }
    }
}

/// A disc as large as the frame that shrinks onto a point while it darkens to `opacity`,
/// then vanishes (Manim's `FocusOn`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusOn {
    /// The point; `None` is the center of the shape.
    pub point: Option<(f32, f32)>,
    pub color: Color,
    pub opacity: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(FocusOn, |t| FocusOn {
    point: None,
    color: t.colors.muted,
    opacity: 0.2,
    delay: 0.0,
    run_time: 2.0,
    rate_func: RateFunc::Smooth,
});

impl ShapeAnimation for FocusOn {
    fn look(&self, _ui: &Ui<'_, '_, '_>, _shape: &Shape, _look: &mut Look) {}

    fn overlay<'a>(&self, ui: &Ui<'a, '_, '_>, shape: &Shape) -> Option<Svgr<'a>> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 || p >= 1.0 {
            return None;
        }
        let target = self.point.map_or_else(
            || shape.bbox().center(),
            |(x, y)| Point::new(f64::from(x), f64::from(y)),
        );
        // Manim's dot starts at the origin, FRAME_X_RADIUS + FRAME_Y_RADIUS wide
        let start = ui.space.to_px(0.0, 0.0);
        let center = start.lerp(target, f64::from(p));
        let r = ui.space.px(64.0 / 9.0 + 4.0) * f64::from(1.0 - p);
        let (cx, cy, r) = (center.x as f32, center.y as f32, r as f32);
        let opacity = self.opacity * p;
        Some(fframes::svgr!(<circle cx={cx} cy={cy} r={r} fill={self.color} opacity={opacity} />))
    }
}

/// The part of the outline a passing flash `time_width` long draws at progress `p`, as
/// fractions of its length (Manim's `ShowPassingFlash`); `None` before and after.
fn flash_window(p: f32, time_width: f32) -> Option<(f32, f32)> {
    if p <= 0.0 || p >= 1.0 {
        return None;
    }
    let upper = p * (1.0 + time_width);
    Some(((upper - time_width).max(0.0), upper.min(1.0)))
}

/// `shape` drawn as an outline only, in `width`, in its stroke or else its fill colour.
fn outline(shape: &Shape, width: f32) -> Shape {
    Shape {
        fill: None,
        stroke: shape.stroke.or(shape.fill),
        stroke_width: width,
        dash: None,
        ..shape.clone()
    }
}

/// A piece of the outline `time_width` long runs along it once; nothing is drawn before or
/// after (Manim's `ShowPassingFlash`). Draw the shape itself separately to keep it. A shape
/// without an outline flashes in its fill colour.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ShowPassingFlash {
    pub shape: Shape,
    pub time_width: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ShowPassingFlash {
    /// Along `shape` in Manim's 1 second, smooth, a tenth of it long.
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            time_width: 0.1,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }
}

impl<'a> Component<'a> for ShowPassingFlash {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let Some(drawn) = flash_window(p, self.time_width) else {
            return Svgr::empty();
        };
        let line = outline(&self.shape, self.shape.stroke_width.max(2.0));
        line.draw_with(&Look {
            drawn,
            ..Look::default()
        })
    }
}

/// `n_segments` passing flashes over each other: the head is wide, the tail thins out
/// behind it (Manim's `ShowPassingFlashWithThinningStrokeWidth`). The widest is the shape's
/// stroke width (at least 2).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ShowPassingFlashWithThinningStrokeWidth {
    pub shape: Shape,
    pub n_segments: usize,
    pub time_width: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ShowPassingFlashWithThinningStrokeWidth {
    /// Along `shape` in Manim's 1 second, smooth, 10 segments, a tenth of it long.
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            n_segments: 10,
            time_width: 0.1,
            delay: 0.0,
            run_time: 1.0,
            rate_func: RateFunc::Smooth,
        }
    }

    /// `(stroke width, time width)` of every flash that draws something: Manim's widths
    /// from 0 to the widest against time widths from `time_width` to 0, without the ends.
    pub fn segments(&self) -> Vec<(f32, f32)> {
        let n = self.n_segments.max(2);
        let widest = self.shape.stroke_width.max(2.0);
        (1..n - 1)
            .map(|i| {
                let f = i as f32 / (n - 1) as f32;
                (widest * f, self.time_width * (1.0 - f))
            })
            .collect()
    }
}

impl<'a> Component<'a> for ShowPassingFlashWithThinningStrokeWidth {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        self.segments()
            .into_iter()
            .filter_map(|(width, time_width)| {
                let drawn = flash_window(p, time_width)?;
                let look = Look {
                    drawn,
                    ..Look::default()
                };
                Some(outline(&self.shape, width).draw_with(&look))
            })
            .collect()
    }
}

/// Rings that grow out of `point` to `radius` one after another while they fade (Manim's
/// `Broadcast`). Each ring runs as a part of a `LaggedStart` of `n_mobs`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Broadcast {
    pub point: (f32, f32),
    /// The radius the rings grow to.
    pub radius: f32,
    pub color: Color,
    pub stroke_width: f32,
    pub n_mobs: usize,
    pub initial_opacity: f32,
    pub final_opacity: f32,
    /// The width the rings start at.
    pub initial_width: f32,
    pub lag_ratio: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(Broadcast, |t| Broadcast {
    point: (960.0, 540.0),
    radius: 270.0,
    color: t.colors.accent,
    stroke_width: t.stroke.regular,
    n_mobs: 5,
    initial_opacity: 1.0,
    final_opacity: 0.0,
    initial_width: 0.0,
    lag_ratio: 0.2,
    delay: 0.0,
    run_time: 3.0,
    rate_func: RateFunc::Smooth,
});

impl<'a> Component<'a> for Broadcast {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let n = self.n_mobs.max(1);
        let lagged = Write {
            delay: self.delay,
            run_time: self.run_time,
            rate_func: self.rate_func,
            lag_ratio: Some(self.lag_ratio),
        };
        let (cx, cy) = self.point;
        (0..n)
            .filter_map(|i| {
                let (start, each) = lagged.window(i, n);
                let t = progress(ui.seconds(), start, start + each);
                if t <= 0.0 || t >= 1.0 {
                    return None;
                }
                let p = self.rate_func.apply(t, each);
                let r = lerp(self.initial_width / 2.0, self.radius, p);
                let opacity = lerp(self.initial_opacity, self.final_opacity, p).clamp(0.0, 1.0);
                Some(fframes::svgr!(
                    <circle cx={cx} cy={cy} r={r} fill="none" stroke={self.color}
                        stroke-width={self.stroke_width} opacity={opacity} />
                ))
            })
            .collect()
    }
}

/// An outline that keeps drawing itself in one colour after another, the previous colour
/// fading out under it (Manim's `AnimatedBoundary`). It never ends; put it in a `Sequence`
/// to stop it.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatedBoundary {
    pub shape: Shape,
    pub colors: Vec<Color>,
    pub max_stroke_width: f32,
    /// Colours per second.
    pub cycle_rate: f32,
    /// Every other colour draws from the end back.
    pub back_and_forth: bool,
    pub draw_rate_func: RateFunc,
    pub fade_rate_func: RateFunc,
    pub delay: f32,
}

impl AnimatedBoundary {
    /// Around `shape` in `colors`, Manim's defaults otherwise.
    pub fn new(shape: Shape, colors: Vec<Color>) -> Self {
        Self {
            shape,
            colors,
            max_stroke_width: 3.0,
            cycle_rate: 0.5,
            back_and_forth: true,
            draw_rate_func: RateFunc::Smooth,
            fade_rate_func: RateFunc::Smooth,
            delay: 0.0,
        }
    }
}

impl<'a> Component<'a> for AnimatedBoundary {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let n = self.colors.len();
        let time = (ui.seconds() - self.delay) * self.cycle_rate;
        if n == 0 || time < 0.0 {
            return Svgr::empty();
        }
        let index = time as usize % n;
        let alpha = time.fract();
        let draw = self.draw_rate_func.apply(alpha, 1.0);
        let fade = self.fade_rate_func.apply(alpha, 1.0);
        let drawn = if self.back_and_forth && time as usize % 2 == 1 {
            (1.0 - draw, 1.0)
        } else {
            (0.0, draw)
        };
        let line = |color: Color, width: f32| Shape {
            stroke: Some(color),
            ..outline(&self.shape, width)
        };
        let growing = line(self.colors[index], self.max_stroke_width).draw_with(&Look {
            drawn,
            ..Look::default()
        });
        if time < 1.0 {
            return growing;
        }
        let previous = self.colors[(index + n - 1) % n];
        let fading = line(previous, (1.0 - fade) * self.max_stroke_width).draw();
        fframes::svgr!(<g>{fading}{growing}</g>)
    }
}

/// Manim's wave: the nudge of a point at `phase` of the passing wave, `ripples` crests.
fn wave(phase: f32, ripples: usize, wave_func: RateFunc) -> f32 {
    let t = 1.0 - phase;
    if t <= 0.0 || t >= 1.0 {
        return 0.0;
    }
    let ripples = ripples.max(1);
    let phases = (2 * ripples) as f32;
    let at = (t * phases) as usize;
    if at == 0 {
        wave_func.apply(t * phases, 1.0)
    } else if at == 2 * ripples - 1 {
        let t = t - at as f32 / phases;
        let sign = if ripples % 2 == 1 { 1.0 } else { -1.0 };
        (1.0 - wave_func.apply(t * phases, 1.0)) * sign
    } else {
        let half = (at - 1) / 2;
        let t = t - (2 * half + 1) as f32 / phases;
        let sign = if half.is_multiple_of(2) { 1.0 } else { -1.0 };
        (1.0 - 2.0 * wave_func.apply(t * ripples as f32, 1.0)) * sign
    }
}

/// A wave runs across the shape from left to right and bends it by `amplitude` pixels in
/// `direction` (Manim's `ApplyWave`). The shape is drawn as built before and after.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ApplyWave {
    pub shape: Shape,
    /// Canvas units, `y` down: `(0.0, -1.0)` is Manim's `UP`.
    pub direction: (f32, f32),
    pub amplitude: f32,
    pub wave_func: RateFunc,
    /// The wave's length as a fraction of the shape's width.
    pub time_width: f32,
    pub ripples: usize,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl ApplyWave {
    /// Up by 27 pixels (Manim's 0.2 units), one ripple in 2 seconds.
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            direction: (0.0, -1.0),
            amplitude: 27.0,
            wave_func: RateFunc::Smooth,
            time_width: 1.0,
            ripples: 1,
            delay: 0.0,
            run_time: 2.0,
            rate_func: RateFunc::Linear,
        }
    }

    /// The path at progress `p`.
    pub fn path_at(&self, p: f32) -> BezPath {
        let b = self.shape.bbox();
        let dir = Vec2::new(f64::from(self.direction.0), f64::from(self.direction.1));
        let nudge = if dir.hypot() > 0.0 {
            dir.normalize() * f64::from(self.amplitude)
        } else {
            Vec2::ZERO
        };
        let upper = p * (1.0 + self.time_width);
        let lower = upper - self.time_width;
        let threshold = (b.width().max(b.height()) / 64.0).max(2.0);
        paths::warp_path(&self.shape.path, threshold, |pt| {
            let relative = if b.width() > 0.0 {
                ((pt.x - b.x0) / b.width()) as f32
            } else {
                0.0
            };
            let phase = if self.time_width > 0.0 {
                (relative - lower) / self.time_width
            } else {
                -1.0
            };
            pt + nudge * f64::from(wave(phase, self.ripples, self.wave_func))
        })
    }
}

impl<'a> Component<'a> for ApplyWave {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        if p <= 0.0 || p >= 1.0 {
            return self.shape.draw();
        }
        Shape {
            path: self.path_at(p),
            ..self.shape.clone()
        }
        .draw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blink_is_on_then_off() {
        let b = Blink {
            blinks: 2,
            ..Blink::default()
        };
        assert!(b.on(-1.0) && b.on(0.2) && !b.on(0.7) && b.on(1.2) && !b.on(1.7) && b.on(2.5));
        assert!(
            !Blink {
                hide_at_end: true,
                ..b
            }
            .on(2.5)
        );
    }

    #[test]
    fn flash_passes_and_thins() {
        assert_eq!(flash_window(0.0, 0.1), None);
        let (lo, hi) = flash_window(0.5, 0.2).unwrap();
        assert!((lo - 0.4).abs() < 1e-6 && (hi - 0.6).abs() < 1e-6);
        let shape = Shape {
            path: paths::parse_path("M0 0 L100 0").unwrap(),
            fill: None,
            stroke: Some(Color::WHITE),
            stroke_width: 4.0,
            dash: None,
            opacity: 1.0,
        };
        let thin = ShowPassingFlashWithThinningStrokeWidth::new(shape);
        let segments = thin.segments();
        assert_eq!(segments.len(), 8);
        assert!(
            segments
                .windows(2)
                .all(|w| w[0].0 < w[1].0 && w[0].1 > w[1].1)
        );
    }

    #[test]
    fn wave_rises_and_settles() {
        assert_eq!(wave(-0.1, 1, RateFunc::Smooth), 0.0);
        assert_eq!(wave(1.1, 1, RateFunc::Smooth), 0.0);
        assert!((wave(0.5, 1, RateFunc::Smooth) - 1.0).abs() < 1e-3);
        assert!(wave(0.9, 1, RateFunc::Smooth) > 0.0);
    }
}
