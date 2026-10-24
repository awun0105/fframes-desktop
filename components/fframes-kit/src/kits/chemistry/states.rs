use fframes::{Color, Svgr};
use kurbo::{Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    kits::label,
    layout::Anchor,
    motion::{Rng, cubic_in_out, progress},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// A state of matter for [`StatesOfMatter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatterState {
    Solid,
    Liquid,
    Gas,
}

impl MatterState {
    /// 0, 1 and 2: the particles' energy, from which their motion blends.
    fn level(self) -> f64 {
        match self {
            Self::Solid => 0.0,
            Self::Liquid => 1.0,
            Self::Gas => 2.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Liquid => "liquid",
            Self::Gas => "gas",
        }
    }
}

/// The change between two neighbouring states while it happens.
fn change(level: f64, rising: bool) -> &'static str {
    match (level < 1.0, rising) {
        (true, true) => "melting",
        (true, false) => "freezing",
        (false, true) => "boiling",
        (false, false) => "condensing",
    }
}

/// `x` bouncing between `low` and `high`: a triangle wave, so a straight flight reflects off
/// the walls without stepping a simulation.
fn bounce(x: f64, low: f64, high: f64) -> f64 {
    let span = high - low;
    if span <= 0.0 {
        return low;
    }
    let m = (x - low).rem_euclid(2.0 * span);
    low + if m > span { 2.0 * span - m } else { m }
}

/// What the seed gives one particle.
#[derive(Debug, Clone, Copy)]
struct Particle {
    home: Vec2,
    pool: Vec2,
    phase: [f64; 4],
    pace: [f64; 4],
    gas_start: Vec2,
    gas_velocity: Vec2,
}

/// Particles in a box changing state, from `from` to `to` over `duration` seconds after
/// `at`: in a solid they vibrate on a lattice at the bottom, in a liquid they wander past
/// each other and in a gas they fly across the whole box and bounce off its walls. Between
/// two states the motions blend, and the particles turn from `cold_color` to `hot_color`.
/// `columns` by `rows` particles of `radius` pixels sit in a `width` by `height` pixel box
/// around `center` (units of `ui.space`). Positions are a function of the time and `seed`
/// only, so every frame renders on its own and the same everywhere. With `labels` the
/// state, or the change under way, is written under the box. The box draws with `enter`,
/// the particles and the label fade with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct StatesOfMatter {
    pub seed: u64,
    pub columns: usize,
    pub rows: usize,
    pub from: MatterState,
    pub to: MatterState,
    pub at: f32,
    pub duration: f32,
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub labels: bool,
    pub box_color: Color,
    pub cold_color: Color,
    pub hot_color: Color,
    pub text_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for StatesOfMatter {
    fn themed(t: &Theme) -> Self {
        Self {
            seed: 1,
            columns: 8,
            rows: 4,
            from: MatterState::Solid,
            to: MatterState::Gas,
            at: 1.0,
            duration: 4.0,
            center: (0.0, 0.0),
            width: 760.0,
            height: 520.0,
            radius: 18.0,
            labels: true,
            box_color: t.colors.muted,
            cold_color: Color::hex("#4dabf7"),
            hot_color: Color::hex("#ff6b6b"),
            text_color: t.colors.text,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for StatesOfMatter {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl StatesOfMatter {
    /// The particles' level at `seconds`: 0 a solid, 1 a liquid, 2 a gas.
    pub fn level(&self, seconds: f32) -> f64 {
        let p = cubic_in_out(progress(seconds, self.at, self.at + self.duration));
        let (a, b) = (self.from.level(), self.to.level());
        a + (b - a) * f64::from(p)
    }

    fn particles(&self) -> Vec<Particle> {
        let mut rng = Rng::new(self.seed);
        let (w, h, r) = (
            f64::from(self.width),
            f64::from(self.height),
            f64::from(self.radius),
        );
        let spacing = 2.15 * r;
        let lattice_width = spacing * (self.columns.max(1) - 1) as f64;
        let speed = 0.55 * w;
        // a liquid spreads over the floor: rows across the whole box, every other one shifted
        // by half a place, and the seed decides which particle flows to which place
        let count = self.rows * self.columns;
        let pool_spacing = 2.3 * r;
        let across = (((w - 2.0 * r) / pool_spacing).floor() as usize).max(1);
        let pool_width = pool_spacing * (across - 1) as f64;
        let mut places: Vec<Vec2> = (0..count)
            .map(|i| {
                let (row, col) = (i / across, i % across);
                let shift = if row % 2 == 1 { 0.5 } else { 0.0 };
                Vec2::new(
                    (-pool_width / 2.0 + pool_spacing * (col as f64 + shift)).min(w / 2.0 - r),
                    h / 2.0 - r - 2.0 - pool_spacing * 0.87 * row as f64,
                )
            })
            .collect();
        for i in (1..places.len()).rev() {
            let j = (rng.next_u64() % (i as u64 + 1)) as usize;
            places.swap(i, j);
        }
        (0..self.rows)
            .flat_map(|row| (0..self.columns).map(move |col| (row, col)))
            .zip(places)
            .map(|((row, col), pool)| {
                // from the box's middle, y down; the bottom row rests on the floor
                let home = Vec2::new(
                    -lattice_width / 2.0 + spacing * col as f64,
                    h / 2.0 - r - 2.0 - spacing * row as f64,
                );
                let mut four = || {
                    [
                        rng.range(0.0, std::f64::consts::TAU),
                        rng.range(0.0, std::f64::consts::TAU),
                        rng.range(0.0, std::f64::consts::TAU),
                        rng.range(0.0, std::f64::consts::TAU),
                    ]
                };
                let phase = four();
                let pace = four().map(|p| 0.6 + p / std::f64::consts::TAU);
                let gas_start = Vec2::new(
                    rng.range(-w / 2.0 + r, w / 2.0 - r),
                    rng.range(-h / 2.0 + r, h / 2.0 - r),
                );
                let (dx, dy) = (rng.range(-1.0, 1.0), rng.range(-1.0, 1.0));
                let norm = dx.hypot(dy).max(1e-6);
                let gas_velocity = Vec2::new(dx, dy) / norm * speed * rng.range(0.6, 1.4);
                Particle {
                    home,
                    pool,
                    phase,
                    pace,
                    gas_start,
                    gas_velocity,
                }
            })
            .collect()
    }

    /// The particles' centres at `seconds`, from the box's middle in pixels (y down).
    pub fn positions(&self, seconds: f32) -> Vec<Vec2> {
        let t = f64::from(seconds);
        let level = self.level(seconds);
        let (w, h, r) = (
            f64::from(self.width),
            f64::from(self.height),
            f64::from(self.radius),
        );
        let spacing = 2.15 * r;
        let smooth = |x: f64| {
            let x = x.clamp(0.0, 1.0);
            x * x * (3.0 - 2.0 * x)
        };
        self.particles()
            .iter()
            .map(|p| {
                let wave = |i: usize, speed: f64| (speed * p.pace[i] * t + p.phase[i]).sin();
                let solid = p.home + Vec2::new(wave(0, 19.0), wave(1, 23.0)) * spacing * 0.07;
                let liquid = p.pool
                    + Vec2::new(wave(2, 1.3), 0.5 * wave(3, 1.1)) * spacing * 0.2
                    + Vec2::new(wave(0, 9.0), wave(1, 11.0)) * spacing * 0.04;
                let flight = p.gas_start + p.gas_velocity * t;
                let gas = Vec2::new(
                    bounce(flight.x, -w / 2.0 + r, w / 2.0 - r),
                    bounce(flight.y, -h / 2.0 + r, h / 2.0 - r),
                );
                let at = if level <= 1.0 {
                    solid.lerp(liquid, smooth(level))
                } else {
                    liquid.lerp(gas, smooth(level - 1.0))
                };
                // the wandering and vibrating particles stay off the walls too
                Vec2::new(
                    at.x.clamp(-w / 2.0 + r, w / 2.0 - r),
                    at.y.clamp(-h / 2.0 + r, h / 2.0 - r),
                )
            })
            .collect()
    }
}

impl<'a> Component<'a> for StatesOfMatter {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (w, h) = (f64::from(self.width), f64::from(self.height));
        let now = ui.seconds();
        let level = self.level(now);
        let container = Shape {
            path: kurbo::Shape::to_path(
                &kurbo::Rect::from_center_size(middle, (w + 8.0, h + 8.0)),
                0.1,
            ),
            fill: None,
            stroke: Some(self.box_color),
            stroke_width: self.stroke_width,
            dash: None,
            opacity: 1.0,
        };
        let container = draw_animated(
            ui,
            &[container],
            self.enter.as_ref(),
            self.exit.as_ref(),
            None,
        );

        let color = fframes::animation::Animatable::apply_progress(
            &self.cold_color,
            &self.hot_color,
            (level / 2.0) as f32,
        );
        let particles: Vec<_> = self
            .positions(now)
            .into_iter()
            .map(|offset| {
                let circle = kurbo::Circle::new(middle + offset, f64::from(self.radius));
                Shape {
                    path: kurbo::Shape::to_path(&circle, 0.1),
                    fill: Some(color),
                    stroke: None,
                    stroke_width: 0.0,
                    dash: None,
                    opacity: 1.0,
                }
            })
            .collect();
        let mut parts = vec![draw_animated(ui, &particles, None, None, None)];
        if self.labels {
            // the state the particles are in, or the change between two
            let nearest = level.round();
            let text = if (level - nearest).abs() < 0.1 {
                [MatterState::Solid, MatterState::Liquid, MatterState::Gas][nearest as usize].name()
            } else {
                change(level, self.to.level() > self.from.level())
            };
            let under = Point::new(middle.x, middle.y + h / 2.0 + 24.0);
            parts.push(label(
                ui,
                text.to_owned(),
                under,
                Anchor::Bottom,
                self.text_color,
            ));
        }
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{container}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounces_stay_inside() {
        assert_eq!(bounce(5.0, 0.0, 10.0), 5.0);
        assert_eq!(bounce(12.0, 0.0, 10.0), 8.0);
        assert_eq!(bounce(-3.0, 0.0, 10.0), 3.0);
        assert_eq!(bounce(23.0, 0.0, 10.0), 3.0);
    }

    #[test]
    fn states_blend_and_stay_in_the_box() {
        let matter = StatesOfMatter::default();
        assert_eq!(matter.level(0.0), 0.0);
        assert_eq!(matter.level(10.0), 2.0);
        let (hw, hh) = (
            f64::from(matter.width) / 2.0,
            f64::from(matter.height) / 2.0,
        );
        for t in [0.0, 1.5, 3.0, 4.2, 9.0] {
            let positions = matter.positions(t);
            assert_eq!(positions.len(), 32);
            assert!(
                positions
                    .iter()
                    .all(|p| p.x.abs() <= hw + 1e-9 && p.y.abs() <= hh + 1e-9),
                "inside the box at {t}"
            );
        }
        // the same seed gives the same frame, another seed another one
        assert_eq!(matter.positions(6.0), matter.positions(6.0));
        let other = StatesOfMatter {
            seed: 2,
            ..StatesOfMatter::default()
        };
        assert_ne!(matter.positions(6.0), other.positions(6.0));
    }

    #[test]
    fn a_solid_keeps_its_lattice() {
        let matter = StatesOfMatter::default();
        let home = matter.positions(0.0);
        let later = matter.positions(0.9);
        let spacing = 2.15 * f64::from(matter.radius);
        assert!(
            home.iter()
                .zip(&later)
                .all(|(a, b)| (*a - *b).hypot() < spacing * 0.2)
        );
    }
}
