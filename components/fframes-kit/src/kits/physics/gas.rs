use fframes::{Color, Svgr};
use kurbo::{Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    kits::short,
    layout::Anchor,
    motion::Rng,
    shapes::{Dot, Rectangle},
    sim::Sim,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{RATE, clock, label};

/// Heating (or cooling) of a [`GasMotion`]: from `at` seconds the temperature goes to `to`
/// times its start over `over` seconds; the speeds grow with its square root.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Heat {
    pub at: f64,
    pub to: f64,
    pub over: f64,
}

/// What a [`GasMotion`] is built from: `count` hard balls of `radius` meters in a box
/// `width` by `height` meters, started from a grid with seeded jitter, moving in seeded
/// directions with speeds between half and one and a half `speed` m/s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GasSetup {
    pub seed: u64,
    pub count: usize,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub speed: f64,
    pub seconds: f64,
    pub heat: Option<Heat>,
}

impl Default for GasSetup {
    fn default() -> Self {
        Self {
            seed: 1,
            count: 50,
            width: 6.0,
            height: 4.0,
            radius: 0.1,
            speed: 1.5,
            seconds: 8.0,
            heat: None,
        }
    }
}

/// The particles of a gas, stepped once with elastic collisions between them and with the
/// walls: build it in the video's constructor and pass it to [`GasBox`]. The same setup
/// gives the same motion on every run and machine: the steps use only `+ - × ÷` and `√`,
/// and the directions come from the seed without trigonometry.
#[derive(Debug, Clone, PartialEq)]
pub struct GasMotion {
    pub setup: GasSetup,
    /// `x, y, vx, vy` of each particle, every step.
    sim: Sim<Vec<[f64; 4]>>,
}

/// The speed factor at `t` seconds of a heating, `√(temperature factor)`.
fn speed_factor(heat: Option<Heat>, t: f64) -> f64 {
    let Some(h) = heat else {
        return 1.0;
    };
    let f = ((t - h.at) / h.over.max(1e-9)).clamp(0.0, 1.0);
    1.0 + (h.to.max(0.0).sqrt() - 1.0) * f
}

impl GasMotion {
    pub fn new(setup: GasSetup) -> Self {
        let s = setup;
        let (w, h, r) = (s.width, s.height, s.radius);
        let mut rng = Rng::new(s.seed);
        let n = s.count;
        let cols = ((n as f64 * w / h.max(1e-9)).sqrt().ceil() as usize).max(1);
        let rows = n.div_ceil(cols).max(1);
        let (cw, ch) = (w / cols as f64, h / rows as f64);
        let initial: Vec<[f64; 4]> = (0..n)
            .map(|i| {
                let (col, row) = (i % cols, i / cols);
                let jitter_x = ((cw - 2.0 * r) / 2.0).max(0.0);
                let jitter_y = ((ch - 2.0 * r) / 2.0).max(0.0);
                let x = cw * (col as f64 + 0.5) + rng.range(-jitter_x, jitter_x);
                let y = ch * (row as f64 + 0.5) + rng.range(-jitter_y, jitter_y);
                // a direction from a point in the unit disk, without trigonometry
                let (dx, dy) = loop {
                    let (dx, dy) = (rng.range(-1.0, 1.0), rng.range(-1.0, 1.0));
                    let d2 = dx * dx + dy * dy;
                    if d2 > 1e-6 && d2 <= 1.0 {
                        let d = d2.sqrt();
                        break (dx / d, dy / d);
                    }
                };
                let speed = s.speed * rng.range(0.5, 1.5);
                [x, y, dx * speed, dy * speed]
            })
            .collect();
        let steps = (s.seconds.max(0.0) * RATE).ceil() as usize;
        let mut t = 0.0;
        let sim = Sim::precompute(1.0 / RATE, steps, initial, |state, dt| {
            let scale = speed_factor(s.heat, t + dt) / speed_factor(s.heat, t);
            t += dt;
            step(state, dt, w, h, r, scale)
        });
        Self { setup, sim }
    }

    /// Each particle's position in meters (from the box's bottom left corner, `y` up) and
    /// velocity in m/s after `t` seconds.
    pub fn particles(&self, t: f64) -> Vec<[f64; 4]> {
        self.sim.sample(t)
    }

    /// The temperature after `t` seconds over the one at the start: the mean of the squared
    /// speeds over its start.
    pub fn temperature(&self, t: f64) -> f64 {
        let mean = |state: &[[f64; 4]]| {
            state.iter().map(|p| p[2] * p[2] + p[3] * p[3]).sum::<f64>() / state.len().max(1) as f64
        };
        let start = mean(&self.sim.states()[0]);
        if start <= 0.0 {
            return 1.0;
        }
        mean(&self.particles(t)) / start
    }

    /// The root mean square speed at the start, in m/s.
    pub fn rms_speed(&self) -> f64 {
        let first = &self.sim.states()[0];
        let sum: f64 = first.iter().map(|p| p[2] * p[2] + p[3] * p[3]).sum();
        (sum / first.len().max(1) as f64).sqrt()
    }

    /// The highest temperature it reaches, over the start.
    pub fn peak_temperature(&self) -> f64 {
        match self.setup.heat {
            Some(h) => h.to.max(1.0),
            None => 1.0,
        }
    }
}

/// One step: move, bounce off the walls, then let touching particles that approach each
/// other trade the parts of their velocities along the line between them (equal masses),
/// and push overlapping ones apart. `scale` multiplies every velocity (heating).
fn step(state: &[[f64; 4]], dt: f64, w: f64, h: f64, r: f64, scale: f64) -> Vec<[f64; 4]> {
    let mut next: Vec<[f64; 4]> = state
        .iter()
        .map(|&[x, y, vx, vy]| {
            let (mut x, mut y) = (x + vx * dt, y + vy * dt);
            let (mut vx, mut vy) = (vx * scale, vy * scale);
            if x < r {
                x = 2.0 * r - x;
                vx = vx.abs();
            } else if x > w - r {
                x = 2.0 * (w - r) - x;
                vx = -vx.abs();
            }
            if y < r {
                y = 2.0 * r - y;
                vy = vy.abs();
            } else if y > h - r {
                y = 2.0 * (h - r) - y;
                vy = -vy.abs();
            }
            [
                x.clamp(r, (w - r).max(r)),
                y.clamp(r, (h - r).max(r)),
                vx,
                vy,
            ]
        })
        .collect();
    let touch = 2.0 * r;
    for i in 0..next.len() {
        for j in i + 1..next.len() {
            let (dx, dy) = (next[j][0] - next[i][0], next[j][1] - next[i][1]);
            let d2 = dx * dx + dy * dy;
            if d2 >= touch * touch || d2 < 1e-18 {
                continue;
            }
            let d = d2.sqrt();
            let (nx, ny) = (dx / d, dy / d);
            let closing = (next[j][2] - next[i][2]) * nx + (next[j][3] - next[i][3]) * ny;
            if closing < 0.0 {
                next[i][2] += closing * nx;
                next[i][3] += closing * ny;
                next[j][2] -= closing * nx;
                next[j][3] -= closing * ny;
            }
            let push = (touch - d) / 2.0;
            next[i][0] -= nx * push;
            next[i][1] -= ny * push;
            next[j][0] += nx * push;
            next[j][1] += ny * push;
        }
    }
    next
}

/// A box of gas: the particles of `motion` in a box whose middle is at `center` (units of
/// `ui.space`, `scale` units per meter), drawn slow in `slow_color` and faster than the
/// start's root mean square speed in `fast_color`, so heating turns the box to the fast
/// colour. With `thermometer` a bar right of the box shows the temperature against the
/// highest one, and with `labels` its factor over the start. The box and the thermometer's
/// frame draw with `enter`, the particles and the level with `parts_enter`. Without `motion`
/// nothing is drawn.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct GasBox<'a> {
    pub motion: Option<&'a GasMotion>,
    pub center: (f64, f64),
    pub scale: f64,
    pub delay: f32,
    pub time_scale: f32,
    pub thermometer: bool,
    pub labels: bool,
    pub box_color: Color,
    pub slow_color: Color,
    pub fast_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for GasBox<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            motion: None,
            center: (0.0, 0.0),
            scale: 1.0,
            delay: 0.0,
            time_scale: 1.0,
            thermometer: true,
            labels: true,
            box_color: t.colors.text,
            slow_color: t.colors.accent,
            fast_color: t.colors.highlight,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for GasBox<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for GasBox<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(motion) = self.motion else {
            return Svgr::empty();
        };
        let space = ui.space;
        let s = motion.setup;
        let (w, h) = (
            space.px(s.width * self.scale),
            space.px(s.height * self.scale),
        );
        let middle = space.to_px(self.center.0, self.center.1);
        let corner = middle - Vec2::new(w / 2.0, -h / 2.0);
        let wall = Rectangle {
            cx: middle.x as f32,
            cy: middle.y as f32,
            width: w as f32,
            height: h as f32,
            fill: None,
            stroke: Some(self.box_color),
            stroke_width: self.stroke_width,
            ..ui.defaults()
        };
        let mut fixed = vec![wall.shape()];
        let bar = (24.0, h * 0.8);
        let bar_middle = Point::new(middle.x + w / 2.0 + 70.0, middle.y);
        if self.thermometer {
            fixed.push(
                Rectangle {
                    cx: bar_middle.x as f32,
                    cy: bar_middle.y as f32,
                    width: bar.0 as f32,
                    height: bar.1 as f32,
                    fill: None,
                    stroke: Some(self.box_color),
                    stroke_width: ui.theme.stroke.thin,
                    ..ui.defaults()
                }
                .shape(),
            );
        }
        let fixed = draw_animated(ui, &fixed, self.enter.as_ref(), self.exit.as_ref(), None);

        let t = clock(ui, self.delay, self.time_scale);
        let fast = motion.rms_speed();
        let radius = space.px(s.radius * self.scale) as f32;
        let mut shapes: Vec<_> = motion
            .particles(t)
            .iter()
            .map(|&[x, y, vx, vy]| {
                let at = corner + Vec2::new(space.px(x * self.scale), -space.px(y * self.scale));
                let color = if vx.hypot(vy) > fast {
                    self.fast_color
                } else {
                    self.slow_color
                };
                Dot {
                    cx: at.x as f32,
                    cy: at.y as f32,
                    radius,
                    fill: Some(color),
                    ..ui.defaults()
                }
                .shape()
            })
            .collect();
        let mut texts = Vec::new();
        if self.thermometer {
            let temperature = motion.temperature(t);
            let level = (temperature / motion.peak_temperature()).clamp(0.0, 1.0) * bar.1;
            if level > 0.0 {
                let bottom = bar_middle.y + bar.1 / 2.0;
                shapes.push(
                    Rectangle {
                        cx: bar_middle.x as f32,
                        cy: (bottom - level / 2.0) as f32,
                        width: (bar.0 - 8.0) as f32,
                        height: level as f32,
                        fill: Some(self.fast_color),
                        stroke: None,
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
            if self.labels {
                let top = bar_middle - Vec2::new(0.0, bar.1 / 2.0 + 12.0);
                let text = format!("T × {}", short(temperature, 1));
                texts.push(label(ui, text, top, Anchor::Top, self.box_color));
            }
        }
        let mut parts = vec![draw_animated(ui, &shapes, None, None, None)];
        parts.extend(texts);
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{fixed}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(state: &[[f64; 4]]) -> f64 {
        state.iter().map(|p| p[2] * p[2] + p[3] * p[3]).sum()
    }

    #[test]
    fn the_same_seed_gives_the_same_motion_bit_for_bit() {
        let setup = GasSetup {
            seconds: 3.0,
            ..GasSetup::default()
        };
        let (a, b) = (GasMotion::new(setup), GasMotion::new(setup));
        let bits = |m: &GasMotion| -> Vec<u64> {
            m.sim
                .states()
                .iter()
                .flatten()
                .flatten()
                .map(|v| v.to_bits())
                .collect()
        };
        assert_eq!(bits(&a), bits(&b));
        let other = GasMotion::new(GasSetup { seed: 2, ..setup });
        assert_ne!(bits(&a), bits(&other));
    }

    #[test]
    fn collisions_keep_the_energy_and_the_box_holds_them() {
        let motion = GasMotion::new(GasSetup {
            seconds: 4.0,
            ..GasSetup::default()
        });
        let s = motion.setup;
        let start = energy(&motion.sim.states()[0]);
        for state in motion.sim.states() {
            assert!((energy(state) - start).abs() / start < 1e-9);
            for p in state {
                assert!(p[0] >= s.radius - 1e-9 && p[0] <= s.width - s.radius + 1e-9);
                assert!(p[1] >= s.radius - 1e-9 && p[1] <= s.height - s.radius + 1e-9);
            }
        }
        assert!((motion.temperature(4.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn heating_raises_the_temperature_to_its_factor() {
        let motion = GasMotion::new(GasSetup {
            seconds: 4.0,
            heat: Some(Heat {
                at: 1.0,
                to: 3.0,
                over: 1.5,
            }),
            ..GasSetup::default()
        });
        assert!((motion.temperature(0.9) - 1.0).abs() < 1e-9);
        assert!((motion.temperature(3.0) - 3.0).abs() < 1e-9);
        assert_eq!(motion.peak_temperature(), 3.0);
        let middle = motion.temperature(1.75);
        assert!(middle > 1.2 && middle < 2.8, "{middle}");
    }

    #[test]
    fn the_particles_start_apart() {
        let motion = GasMotion::new(GasSetup::default());
        let first = &motion.sim.states()[0];
        let r = motion.setup.radius;
        for i in 0..first.len() {
            for j in i + 1..first.len() {
                let d = (first[i][0] - first[j][0]).hypot(first[i][1] - first[j][1]);
                assert!(d >= 2.0 * r - 1e-9, "{i} {j}: {d}");
            }
        }
    }
}
