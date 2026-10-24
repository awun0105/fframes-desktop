use std::f64::consts::TAU;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::Space,
    geometry::Shape,
    shapes::Dot,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{clock, line};

/// What a [`Wave`] shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaveKind {
    /// The medium moves up and down while the wave travels right.
    #[default]
    Transverse,
    /// The medium moves back and forth along the direction of travel: a row of particles
    /// that bunch into compressions and spread into rarefactions.
    Longitudinal,
    /// Two equal waves travelling against each other: `2A sin(kx) cos(ωt)`, with its nodes
    /// marked and its envelope dashed.
    Standing,
    /// The wave and a second one (`second_wavelength`, `second_amplitude`) travelling the
    /// same way, drawn thin, with their sum drawn thick.
    Superposition,
}

/// A wave along `length` meters of a medium whose left end is at `origin` (units of
/// `ui.space`, `scale` units per meter), travelling right at `speed` m/s. With `front`, the
/// medium is at rest at the start and the wave leaves the left end on the figure's clock;
/// without it the whole length moves from the start. `marker` (meters from the left end)
/// picks out one point of the medium, which shows that the medium oscillates in place while
/// the wave passes. The axis draws with `enter`, the medium fades with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Wave {
    pub kind: WaveKind,
    pub origin: (f64, f64),
    pub length: f64,
    pub scale: f64,
    pub amplitude: f64,
    pub wavelength: f64,
    pub speed: f64,
    pub second_wavelength: f64,
    pub second_amplitude: f64,
    pub front: bool,
    pub marker: Option<f64>,
    /// Particles of a longitudinal wave.
    pub particles: usize,
    /// Meters tall, the lines that stand for the particles of a longitudinal wave.
    pub band: f64,
    pub delay: f32,
    pub time_scale: f32,
    pub color: Color,
    pub second_color: Color,
    pub marker_color: Color,
    pub guide_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Wave {
    fn themed(t: &Theme) -> Self {
        Self {
            kind: WaveKind::Transverse,
            origin: (-6.0, 0.0),
            length: 12.0,
            scale: 1.0,
            amplitude: 0.8,
            wavelength: 4.0,
            speed: 2.0,
            second_wavelength: 3.0,
            second_amplitude: 0.5,
            front: true,
            marker: None,
            particles: 48,
            band: 1.2,
            delay: 0.0,
            time_scale: 1.0,
            color: t.colors.accent,
            second_color: t.colors.highlight,
            marker_color: t.colors.title,
            guide_color: t.colors.muted,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Wave {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// `amplitude sin(k x - ω t)` of a wave that has travelled `speed t` meters from the left
/// end, or everywhere when `front` is off.
fn travelling(x: f64, t: f64, amplitude: f64, wavelength: f64, speed: f64, front: bool) -> f64 {
    if wavelength <= 0.0 || (front && x > speed * t) {
        return 0.0;
    }
    let k = TAU / wavelength;
    amplitude * (k * x - k * speed * t).sin()
}

impl Wave {
    /// The displacement in meters of the medium `x` meters from the left end after `t`
    /// seconds: up for the transverse kinds, to the right for the longitudinal one.
    pub fn displacement(&self, x: f64, t: f64) -> f64 {
        let (a, l, v) = (self.amplitude, self.wavelength, self.speed);
        match self.kind {
            WaveKind::Transverse | WaveKind::Longitudinal => travelling(x, t, a, l, v, self.front),
            WaveKind::Standing => self.envelope(x) * (TAU / l.max(1e-9) * v * t).cos(),
            WaveKind::Superposition => travelling(x, t, a, l, v, self.front) + self.second(x, t),
        }
    }

    /// The second wave of a superposition.
    fn second(&self, x: f64, t: f64) -> f64 {
        travelling(
            x,
            t,
            self.second_amplitude,
            self.second_wavelength,
            self.speed,
            self.front,
        )
    }

    /// The largest displacement of a standing wave at `x`, `2A sin(kx)`.
    pub fn envelope(&self, x: f64) -> f64 {
        2.0 * self.amplitude * (TAU / self.wavelength.max(1e-9) * x).sin()
    }

    /// The standing wave of the `n`th harmonic of a string `length` meters long, fixed at both
    /// ends: Manim-physics' `StandingWave(n, length)`, `n` half wavelengths along it.
    #[must_use]
    pub fn harmonic(self, n: u32) -> Self {
        Self {
            kind: WaveKind::Standing,
            wavelength: 2.0 * self.length / f64::from(n.max(1)),
            ..self
        }
    }

    /// The nodes of a standing wave, every half wavelength from the left end, in meters.
    pub fn nodes(&self) -> Vec<f64> {
        let half = self.wavelength / 2.0;
        if half <= 0.0 {
            return Vec::new();
        }
        let count = ((self.length.max(0.0) + 1e-9) / half).floor() as usize;
        (0..=count).map(|i| i as f64 * half).collect()
    }

    fn px(&self, space: Space, x: f64, y: f64) -> Point {
        space.to_px(
            self.origin.0 + x * self.scale,
            self.origin.1 + y * self.scale,
        )
    }

    /// The curve of `f` along the medium.
    fn curve(&self, space: Space, f: impl Fn(f64) -> f64) -> BezPath {
        let waves = self.length / self.wavelength.min(self.second_wavelength).max(1e-3);
        let n = (waves * 48.0).clamp(64.0, 2000.0) as usize;
        let mut path = BezPath::new();
        for i in 0..=n {
            let x = self.length * i as f64 / n as f64;
            let p = self.px(space, x, f(x));
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        path
    }

    fn stroke(&self, path: BezPath, color: Color, width: f32) -> Shape {
        Shape {
            path,
            fill: None,
            stroke: Some(color),
            stroke_width: width,
            dash: None,
            opacity: 1.0,
        }
    }
}

impl<'a> Component<'a> for Wave {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let thin = ui.theme.stroke.thin;
        let mut axis = line(
            ui,
            self.px(space, 0.0, 0.0),
            self.px(space, self.length, 0.0),
            self.guide_color,
            thin,
        );
        axis.dash = Some((8.0, 8.0));
        let axis = draw_animated(ui, &[axis], self.enter.as_ref(), self.exit.as_ref(), None);

        let t = clock(ui, self.delay, self.time_scale);
        let mut shapes = Vec::new();
        let dot = |at: Point, radius: f32, color: Color| {
            Dot {
                cx: at.x as f32,
                cy: at.y as f32,
                radius,
                fill: Some(color),
                ..ui.defaults()
            }
            .shape()
        };
        match self.kind {
            WaveKind::Longitudinal => {
                let n = self.particles.max(2);
                let half = self.band / 2.0;
                let marked = self
                    .marker
                    .map(|m| ((m / self.length) * (n - 1) as f64).round() as usize);
                for i in 0..n {
                    let rest = self.length * i as f64 / (n - 1) as f64;
                    let x = rest + self.displacement(rest, t);
                    let (color, width) = if marked == Some(i) {
                        (self.marker_color, self.stroke_width)
                    } else {
                        (self.color, thin)
                    };
                    shapes.push(line(
                        ui,
                        self.px(space, x, -half),
                        self.px(space, x, half),
                        color,
                        width,
                    ));
                }
            }
            WaveKind::Standing => {
                for sign in [1.0, -1.0] {
                    let mut envelope = self.stroke(
                        self.curve(space, |x| sign * self.envelope(x)),
                        self.guide_color,
                        thin,
                    );
                    envelope.dash = Some((6.0, 8.0));
                    shapes.push(envelope);
                }
                shapes.push(self.stroke(
                    self.curve(space, |x| self.displacement(x, t)),
                    self.color,
                    self.stroke_width,
                ));
                for x in self.nodes() {
                    shapes.push(dot(self.px(space, x, 0.0), 7.0, self.second_color));
                }
            }
            WaveKind::Superposition => {
                let first = |x| {
                    travelling(
                        x,
                        t,
                        self.amplitude,
                        self.wavelength,
                        self.speed,
                        self.front,
                    )
                };
                shapes.push(self.stroke(self.curve(space, first), self.color, thin));
                shapes.push(self.stroke(
                    self.curve(space, |x| self.second(x, t)),
                    self.second_color,
                    thin,
                ));
                shapes.push(self.stroke(
                    self.curve(space, |x| self.displacement(x, t)),
                    self.marker_color,
                    self.stroke_width,
                ));
            }
            WaveKind::Transverse => {
                shapes.push(self.stroke(
                    self.curve(space, |x| self.displacement(x, t)),
                    self.color,
                    self.stroke_width,
                ));
            }
        }
        if let (Some(m), false) = (self.marker, self.kind == WaveKind::Longitudinal) {
            let m = m.clamp(0.0, self.length);
            shapes.push(dot(
                self.px(space, m, self.displacement(m, t)),
                10.0,
                self.marker_color,
            ));
        }
        let parts = draw_animated(ui, &shapes, None, None, None);
        let parts = fade(
            ui,
            parts,
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{axis}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_travelling_wave_moves_at_its_speed() {
        let wave = Wave {
            front: false,
            ..Wave::default()
        };
        // what is at x now is at x + v dt after dt
        for i in 0..20 {
            let x = f64::from(i) * 0.3;
            let later = wave.displacement(x + wave.speed * 0.7, 0.7);
            assert!((wave.displacement(x, 0.0) - later).abs() < 1e-9);
        }
    }

    #[test]
    fn the_front_leaves_the_medium_at_rest_ahead_of_it() {
        let wave = Wave::default();
        assert_eq!(wave.displacement(3.0, 1.0), 0.0);
        assert!(wave.displacement(1.0, 1.0).abs() > 0.1);
        // the medium starts from rest at the front: no jump there
        assert!(wave.displacement(2.0 - 1e-6, 1.0).abs() < 1e-5);
    }

    #[test]
    fn a_standing_wave_never_moves_at_its_nodes() {
        let wave = Wave {
            kind: WaveKind::Standing,
            length: 10.0,
            wavelength: 4.0,
            ..Wave::default()
        };
        assert_eq!(wave.nodes(), vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        for node in wave.nodes() {
            for i in 0..10 {
                assert!(wave.displacement(node, f64::from(i) * 0.37).abs() < 1e-9);
            }
        }
        // an antinode reaches twice the amplitude
        assert!((wave.displacement(1.0, 0.0) - 2.0 * wave.amplitude).abs() < 1e-12);
    }

    #[test]
    fn a_harmonic_has_n_loops_between_fixed_ends() {
        let wave = Wave {
            length: 9.0,
            ..Wave::default()
        }
        .harmonic(3);
        assert_eq!(wave.kind, WaveKind::Standing);
        assert_eq!(wave.nodes(), vec![0.0, 3.0, 6.0, 9.0]);
    }

    #[test]
    fn a_superposition_is_the_sum_of_its_waves() {
        let wave = Wave {
            kind: WaveKind::Superposition,
            front: false,
            ..Wave::default()
        };
        let first = Wave {
            kind: WaveKind::Transverse,
            ..wave.clone()
        };
        let second = Wave {
            amplitude: wave.second_amplitude,
            wavelength: wave.second_wavelength,
            ..first.clone()
        };
        for i in 0..20 {
            let (x, t) = (f64::from(i) * 0.4, f64::from(i) * 0.1);
            let sum = first.displacement(x, t) + second.displacement(x, t);
            assert!((wave.displacement(x, t) - sum).abs() < 1e-12);
        }
    }
}
