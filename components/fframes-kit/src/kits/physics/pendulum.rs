use fframes::{Color, Svgr};
use kurbo::{Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::Space,
    kits::short,
    layout::Anchor,
    shapes::{Arc, Dot},
    sim::Sim,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{GRAVITY, RATE, clock, label, line, rk4};

/// A pendulum's swing, stepped once: `length` meters, released at rest from `angle0` radians
/// (0 hangs straight down, positive to the right), slowed by `damping` per second
/// (`θ'' = -g/L sin θ - damping θ'`), under standard gravity. Build it in the video's
/// constructor and pass it to [`Pendulum`] and, through [`PendulumMotion::energy`], to
/// [`EnergyBars`](super::EnergyBars).
#[derive(Debug, Clone, PartialEq)]
pub struct PendulumMotion {
    pub length: f64,
    pub damping: f64,
    sim: Sim<[f64; 2]>,
}

impl PendulumMotion {
    /// `seconds` of an undamped swing.
    pub fn new(length: f64, angle0: f64, seconds: f64) -> Self {
        Self::damped(length, angle0, 0.0, seconds)
    }

    /// `seconds` of a swing that loses speed with `damping` per second.
    pub fn damped(length: f64, angle0: f64, damping: f64, seconds: f64) -> Self {
        let length = length.max(1e-6);
        let steps = (seconds.max(0.0) * RATE).ceil() as usize;
        let sim = Sim::precompute(1.0 / RATE, steps, [angle0, 0.0], |&y, dt| {
            rk4(y, dt, |[a, w]| {
                [w, -GRAVITY / length * a.sin() - damping * w]
            })
        });
        Self {
            length,
            damping,
            sim,
        }
    }

    /// The angle in radians and the angular speed in rad/s after `t` seconds.
    pub fn state(&self, t: f64) -> (f64, f64) {
        let [a, w] = self.sim.sample(t);
        (a, w)
    }

    /// The kinetic and the potential energy in joules of a bob of `mass` kg after `t`
    /// seconds; the potential is 0 at the lowest point.
    pub fn energy(&self, t: f64, mass: f64) -> (f64, f64) {
        let (a, w) = self.state(t);
        let speed = self.length * w;
        (
            0.5 * mass * speed * speed,
            mass * GRAVITY * self.length * (1.0 - a.cos()),
        )
    }

    /// The angle it was released from.
    pub fn amplitude(&self) -> f64 {
        self.sim.states()[0][0]
    }

    /// The period of small swings, `2π √(L / g)`.
    pub fn small_angle_period(&self) -> f64 {
        std::f64::consts::TAU * (self.length / GRAVITY).sqrt()
    }

    /// Seconds simulated.
    pub fn duration(&self) -> f64 {
        self.sim.duration()
    }
}

/// A pendulum hanging from `pivot` (units of `ui.space`, `scale` units per meter) that
/// swings as `motion` says, from `delay` seconds after the local start. With `arc` the
/// swing's reach is a dashed arc, with `angle_label` the angle from the vertical is
/// written next to the pivot. The ceiling and the arc draw with `enter`, the rod and bob
/// fade with `parts_enter`. Without `motion` nothing is drawn.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Pendulum<'a> {
    pub motion: Option<&'a PendulumMotion>,
    pub pivot: (f64, f64),
    pub scale: f64,
    pub delay: f32,
    pub time_scale: f32,
    pub bob_radius: f32,
    pub arc: bool,
    pub angle_label: bool,
    pub color: Color,
    pub bob_color: Color,
    pub guide_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Pendulum<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            motion: None,
            pivot: (0.0, 2.5),
            scale: 1.0,
            delay: 0.0,
            time_scale: 1.0,
            bob_radius: 28.0,
            arc: true,
            angle_label: true,
            color: t.colors.text,
            bob_color: t.colors.accent,
            guide_color: t.colors.muted,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Pendulum<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Pendulum<'_> {
    /// The pixel of the bob at `angle` for a rod `length` meters long.
    fn bob(&self, space: Space, length: f64, angle: f64) -> Point {
        let r = length * self.scale;
        space.to_px(
            self.pivot.0 + r * angle.sin(),
            self.pivot.1 - r * angle.cos(),
        )
    }
}

impl<'a> Component<'a> for Pendulum<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(motion) = self.motion else {
            return Svgr::empty();
        };
        let space = ui.space;
        let pivot = space.to_px(self.pivot.0, self.pivot.1);
        let rod = space.px(motion.length * self.scale);
        let thin = ui.theme.stroke.thin;
        let width = space.px(0.5);
        let mut guides = vec![line(
            ui,
            pivot - Vec2::new(width, 0.0),
            pivot + Vec2::new(width, 0.0),
            self.color,
            self.stroke_width,
        )];
        let mut rest = line(
            ui,
            pivot,
            pivot + Vec2::new(0.0, rod),
            self.guide_color,
            thin,
        );
        rest.dash = Some((8.0, 8.0));
        guides.push(rest);
        let reach = motion.amplitude().abs();
        if self.arc && reach > 1e-3 {
            let mut swing = Arc {
                cx: pivot.x as f32,
                cy: pivot.y as f32,
                radius: rod as f32,
                // counterclockwise on screen: straight down is minus a quarter turn
                start_angle: (-std::f64::consts::FRAC_PI_2 - reach) as f32,
                angle: (2.0 * reach) as f32,
                stroke: Some(self.guide_color),
                stroke_width: thin,
                ..ui.defaults()
            }
            .shape();
            swing.dash = Some((8.0, 8.0));
            guides.push(swing);
        }
        let guides = draw_animated(ui, &guides, self.enter.as_ref(), self.exit.as_ref(), None);

        let (angle, _) = motion.state(clock(ui, self.delay, self.time_scale));
        let bob = self.bob(space, motion.length, angle);
        let shapes = [
            line(ui, pivot, bob, self.color, self.stroke_width),
            Dot {
                cx: pivot.x as f32,
                cy: pivot.y as f32,
                radius: 7.0,
                fill: Some(self.color),
                ..ui.defaults()
            }
            .shape(),
            Dot {
                cx: bob.x as f32,
                cy: bob.y as f32,
                radius: self.bob_radius,
                fill: Some(self.bob_color),
                ..ui.defaults()
            }
            .shape(),
        ];
        let mut parts = vec![draw_animated(ui, &shapes, None, None, None)];
        if self.angle_label {
            // the arc from the vertical to the rod, the value past its middle; both move
            // with the angle, so the label never jumps sides
            let down = -std::f64::consts::FRAC_PI_2;
            let sweep = Arc {
                cx: pivot.x as f32,
                cy: pivot.y as f32,
                radius: (rod * 0.25) as f32,
                start_angle: down as f32,
                angle: angle as f32,
                stroke: Some(self.guide_color),
                stroke_width: thin,
                ..ui.defaults()
            }
            .shape();
            parts.push(draw_animated(ui, &[sweep], None, None, None));
            let half = down + angle / 2.0;
            let at = pivot + Vec2::new(half.cos(), -half.sin()) * (rod * 0.4);
            let text = format!("{}°", short(angle.to_degrees(), 0));
            parts.push(label(ui, text, at, Anchor::Center, self.color));
        }
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{guides}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_undamped_swing_keeps_its_energy() {
        let motion = PendulumMotion::new(2.0, 1.2, 10.0);
        let (k0, p0) = motion.energy(0.0, 1.0);
        assert_eq!(k0, 0.0);
        for i in 0..=100 {
            let (k, p) = motion.energy(f64::from(i) * 0.1, 1.0);
            assert!(((k + p) - p0).abs() / p0 < 1e-6, "at {i}: {}", k + p);
        }
    }

    #[test]
    fn a_small_swing_has_the_textbook_period() {
        let motion = PendulumMotion::new(1.0, 0.05, 5.0);
        let period = motion.small_angle_period();
        // back at the start after one period, at the far side after half of one
        assert!((motion.state(period).0 - 0.05).abs() < 1e-3);
        assert!((motion.state(period / 2.0).0 + 0.05).abs() < 1e-3);
    }

    #[test]
    fn damping_takes_energy_out() {
        let motion = PendulumMotion::damped(1.5, 0.8, 0.4, 12.0);
        let total = |t| {
            let (k, p) = motion.energy(t, 2.0);
            k + p
        };
        assert!(total(12.0) < total(0.0) * 0.1);
        assert!(total(6.0) < total(3.0));
        assert_eq!(motion.duration(), 12.0);
    }

    #[test]
    fn the_bob_hangs_below_the_pivot() {
        let p = Pendulum {
            pivot: (0.0, 0.0),
            scale: 0.5,
            ..Pendulum::default()
        };
        let space = Space {
            origin: (0.0, 0.0),
            unit: 100.0,
        };
        assert!((p.bob(space, 2.0, 0.0) - Point::new(0.0, 100.0)).hypot() < 1e-9);
        let side = p.bob(space, 2.0, std::f64::consts::FRAC_PI_2);
        assert!((side - Point::new(100.0, 0.0)).hypot() < 1e-9);
    }
}
