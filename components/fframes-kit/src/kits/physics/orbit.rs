use std::f64::consts::TAU;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::Space,
    geometry::Shape,
    shapes::{Dot, Ellipse},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::clock;

/// A planet on a Kepler orbit around a star at `focus` (units of `ui.space`): an ellipse of
/// `semi_major` units and `eccentricity` with its closest point (perihelion) to the right of
/// the star, gone around counterclockwise once every `period` seconds. `start` is where the
/// planet is when its clock starts, as a fraction of the period after the perihelion. The
/// planet is fast near the star and slow far from it. With `sweeps`, the first orbit shades
/// that many sectors from the star, each swept in the same `sweep` fraction of the period, at
/// even times around the orbit: Kepler's second law, equal areas in equal times. The orbit
/// and the star draw with `enter`, the planet and the sectors with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Orbit {
    pub focus: (f64, f64),
    pub semi_major: f64,
    #[cfg_attr(feature = "studio", props(min = 0.0, max = 0.95, step = 0.01))]
    pub eccentricity: f64,
    pub period: f64,
    pub start: f64,
    pub delay: f32,
    pub time_scale: f32,
    pub sweeps: usize,
    pub sweep: f64,
    pub star_radius: f32,
    pub planet_radius: f32,
    pub orbit_color: Color,
    pub star_color: Color,
    pub planet_color: Color,
    pub sweep_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Orbit {
    fn themed(t: &Theme) -> Self {
        Self {
            focus: (1.5, 0.0),
            semi_major: 3.5,
            eccentricity: 0.6,
            period: 5.0,
            start: 0.0,
            delay: 0.0,
            time_scale: 1.0,
            sweeps: 3,
            sweep: 0.1,
            star_radius: 24.0,
            planet_radius: 13.0,
            orbit_color: t.colors.muted,
            star_color: t.colors.highlight,
            planet_color: t.colors.accent,
            sweep_color: t.colors.accent,
            stroke_width: t.stroke.thin,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Orbit {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The eccentric anomaly `E` of the mean anomaly `m`: `E - e sin E = m`, by Newton's method.
pub fn eccentric_anomaly(m: f64, e: f64) -> f64 {
    let m = m.rem_euclid(TAU);
    let mut big_e = if e < 0.8 { m } else { std::f64::consts::PI };
    for _ in 0..50 {
        let step = (big_e - e * big_e.sin() - m) / (1.0 - e * big_e.cos());
        big_e -= step;
        if step.abs() < 1e-14 {
            break;
        }
    }
    big_e
}

impl Orbit {
    fn e(&self) -> f64 {
        self.eccentricity.clamp(0.0, 0.99)
    }

    /// The semi-minor axis, `a √(1 - e²)`.
    pub fn semi_minor(&self) -> f64 {
        self.semi_major * (1.0 - self.e() * self.e()).sqrt()
    }

    /// The planet's offset from the star in units after `t` seconds of its clock.
    pub fn position(&self, t: f64) -> (f64, f64) {
        let period = self.period.max(1e-6);
        let e = self.e();
        let big_e = eccentric_anomaly(TAU * (t / period + self.start), e);
        (
            self.semi_major * (big_e.cos() - e),
            self.semi_minor() * big_e.sin(),
        )
    }

    /// Seconds of the planet's clock when sector `i` of [`Orbit::sweeps`] starts.
    pub fn sweep_start(&self, i: usize) -> f64 {
        self.period * i as f64 / self.sweeps.max(1) as f64
    }

    fn px(&self, space: Space, (x, y): (f64, f64)) -> Point {
        space.to_px(self.focus.0 + x, self.focus.1 + y)
    }

    /// The sector swept from `from` to `to` seconds, as a closed path from the star.
    fn sector(&self, space: Space, from: f64, to: f64) -> BezPath {
        let mut path = BezPath::new();
        path.move_to(self.px(space, (0.0, 0.0)));
        let n = 48;
        for i in 0..=n {
            let t = from + (to - from) * f64::from(i) / f64::from(n);
            path.line_to(self.px(space, self.position(t)));
        }
        path.close_path();
        path
    }
}

impl<'a> Component<'a> for Orbit {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let c = self.semi_major * self.e();
        let center = space.to_px(self.focus.0 - c, self.focus.1);
        let star = self.px(space, (0.0, 0.0));
        let fixed = [
            Ellipse {
                cx: center.x as f32,
                cy: center.y as f32,
                width: (2.0 * space.px(self.semi_major)) as f32,
                height: (2.0 * space.px(self.semi_minor())) as f32,
                fill: None,
                stroke: Some(self.orbit_color),
                stroke_width: self.stroke_width,
                dash: Some((8.0, 8.0)),
                ..ui.defaults()
            }
            .shape(),
            Dot {
                cx: star.x as f32,
                cy: star.y as f32,
                radius: self.star_radius,
                fill: Some(self.star_color),
                ..ui.defaults()
            }
            .shape(),
        ];
        let fixed = draw_animated(ui, &fixed, self.enter.as_ref(), self.exit.as_ref(), None);

        let t = clock(ui, self.delay, self.time_scale);
        let mut shapes = Vec::new();
        if self.sweeps > 0 && self.period > 0.0 {
            for i in 0..self.sweeps {
                let from = self.sweep_start(i);
                if t <= from {
                    break;
                }
                let to = (from + self.sweep * self.period).min(t);
                shapes.push(Shape {
                    path: self.sector(space, from, to),
                    fill: Some(self.sweep_color),
                    stroke: None,
                    stroke_width: 0.0,
                    dash: None,
                    opacity: 0.35,
                });
            }
        }
        let planet = self.px(space, self.position(t));
        shapes.push(
            Dot {
                cx: planet.x as f32,
                cy: planet.y as f32,
                radius: self.planet_radius,
                fill: Some(self.planet_color),
                ..ui.defaults()
            }
            .shape(),
        );
        let parts = draw_animated(ui, &shapes, None, None, None);
        let parts = fade(
            ui,
            parts,
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{fixed}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The area swept from `from` to `to` seconds, by the shoelace formula.
    fn swept(orbit: &Orbit, from: f64, to: f64) -> f64 {
        let n = 2000;
        let mut area = 0.0;
        let mut last = orbit.position(from);
        for i in 1..=n {
            let p = orbit.position(from + (to - from) * f64::from(i) / f64::from(n));
            area += last.0 * p.1 - p.0 * last.1;
            last = p;
        }
        area / 2.0
    }

    #[test]
    fn equal_times_sweep_equal_areas() {
        let orbit = Orbit::default();
        let dt = orbit.period * 0.1;
        let first = swept(&orbit, 0.0, dt);
        for t in [0.7, 1.9, 2.5, 3.6] {
            let area = swept(&orbit, t, t + dt);
            assert!(
                (area - first).abs() / first < 1e-4,
                "{t}: {area} vs {first}"
            );
        }
        // a whole orbit sweeps the ellipse, π a b
        let whole = swept(&orbit, 0.0, orbit.period);
        let ellipse = std::f64::consts::PI * orbit.semi_major * orbit.semi_minor();
        assert!((whole - ellipse).abs() / ellipse < 1e-5);
    }

    #[test]
    fn the_planet_closes_its_orbit_and_starts_at_the_perihelion() {
        let orbit = Orbit::default();
        let (x, y) = orbit.position(0.0);
        assert!((x - orbit.semi_major * (1.0 - orbit.eccentricity)).abs() < 1e-12);
        assert!(y.abs() < 1e-12);
        let (x1, y1) = orbit.position(orbit.period);
        assert!((x1 - x).abs() < 1e-9 && (y1 - y).abs() < 1e-9);
        // half an orbit later it is at the aphelion, on the left
        let (xa, _) = orbit.position(orbit.period / 2.0);
        assert!((xa + orbit.semi_major * (1.0 + orbit.eccentricity)).abs() < 1e-9);
    }

    #[test]
    fn keplers_equation_is_solved_for_high_eccentricity() {
        for e in [0.0, 0.3, 0.9, 0.99] {
            for i in 0..12 {
                let m = f64::from(i) * 0.5;
                let big_e = eccentric_anomaly(m, e);
                let back = (big_e - e * big_e.sin()).rem_euclid(TAU);
                assert!((back - m.rem_euclid(TAU)).abs() < 1e-10, "{e} {m}");
            }
        }
    }
}
