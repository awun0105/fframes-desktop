//! The physics kit (feature `physics`): mechanics, waves, fields, circuits and optics.
//!
//! Figures stand alone and are placed in units of `ui.space`, with `scale` space units per
//! meter, like the math kit's [`UnitCircle`](crate::kits::math). Physical quantities are SI
//! (meters, seconds, kilograms). A motion without a closed form is stepped once, in the
//! video's constructor, into a [`Sim`](crate::sim::Sim) that the figure borrows:
//! [`PendulumMotion`], [`SpringMotion`]. A figure without its data draws nothing. Every
//! figure runs on its own clock: local time minus `delay`, times `time_scale`.
//! `docs/components/physics.md` has an example of each with its gallery scene.

mod energy;
mod pendulum;
mod projectile;
mod spring;

pub use energy::EnergyBars;
pub use pendulum::{Pendulum, PendulumMotion};
pub use projectile::Projectile;
pub use spring::{SpringMass, SpringMotion};

use fframes::{Color, Svgr};
use kurbo::Point;

use crate::{
    geometry::Shape,
    label::Label,
    layout::Anchor,
    shapes::{Arrow, Line},
    ui::Ui,
};

/// Standard gravity, m/s².
pub const GRAVITY: f64 = 9.81;

/// Steps per second of the simulations.
pub(crate) const RATE: f64 = 240.0;

/// Seconds a motion has run: local time minus `delay`, times `time_scale`, never below 0.
pub(crate) fn clock(ui: &Ui<'_, '_, '_>, delay: f32, time_scale: f32) -> f64 {
    f64::from(((ui.seconds() - delay) * time_scale).max(0.0))
}

/// One classic Runge-Kutta step of `y' = f(y)`.
pub(crate) fn rk4<const N: usize>(
    y: [f64; N],
    dt: f64,
    f: impl Fn([f64; N]) -> [f64; N],
) -> [f64; N] {
    let add = |a: [f64; N], b: [f64; N], s: f64| std::array::from_fn(|i| a[i] + b[i] * s);
    let k1 = f(y);
    let k2 = f(add(y, k1, dt / 2.0));
    let k3 = f(add(y, k2, dt / 2.0));
    let k4 = f(add(y, k3, dt));
    std::array::from_fn(|i| y[i] + dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]))
}

pub(crate) fn line(ui: &Ui<'_, '_, '_>, a: Point, b: Point, color: Color, width: f32) -> Shape {
    Line {
        x1: a.x as f32,
        y1: a.y as f32,
        x2: b.x as f32,
        y2: b.y as f32,
        stroke: Some(color),
        stroke_width: width,
        ..ui.defaults()
    }
    .shape()
}

/// An arrow whose tip shrinks with a short shaft, so a small vector stays readable.
pub(crate) fn arrow(ui: &Ui<'_, '_, '_>, a: Point, b: Point, color: Color, width: f32) -> Shape {
    let tip = ((b - a).hypot() * 0.35).min(22.0) as f32;
    Arrow {
        x1: a.x as f32,
        y1: a.y as f32,
        x2: b.x as f32,
        y2: b.y as f32,
        tip_length: tip,
        tip_width: tip * 0.85,
        fill: Some(color),
        stroke: Some(color),
        stroke_width: width,
        ..ui.defaults()
    }
    .shape()
}

pub(crate) fn label<'a>(
    ui: &mut Ui<'a, '_, '_>,
    text: String,
    at: Point,
    direction: Anchor,
    fill: Color,
) -> Svgr<'a> {
    let label = Label {
        text: text.into(),
        x: at.x as f32,
        y: at.y as f32,
        direction,
        style: ui.theme.typography.caption,
        fill,
        ..ui.defaults()
    };
    ui.show(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rk4_follows_an_exponential() {
        // y' = y from 1: e after one second
        let mut y = [1.0];
        for _ in 0..100 {
            y = rk4(y, 0.01, |[v]| [v]);
        }
        assert!((y[0] - std::f64::consts::E).abs() < 1e-9);
    }
}
