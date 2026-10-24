//! The physics kit (feature `physics`): mechanics, waves, fields, circuits and optics.
//!
//! Figures stand alone and are placed in units of `ui.space`, with `scale` space units per
//! meter, like the math kit's [`UnitCircle`](crate::kits::math). Physical quantities are SI
//! (meters, seconds, kilograms). A motion without a closed form is stepped once, in the
//! video's constructor, into a [`Sim`](crate::sim::Sim) that the figure borrows:
//! [`PendulumMotion`], [`SpringMotion`], [`Field`]. A figure without its data draws nothing. Every
//! figure runs on its own clock: local time minus `delay`, times `time_scale`.
//! `docs/components/physics.md` has an example of each with its gallery scene.

mod circuit;
mod collision;
mod doppler;
mod electro;
mod energy;
mod field_lines;
mod free_body;
mod gas;
mod lens;
mod magneto;
mod optics;
mod orbit;
mod pendulum;
mod projectile;
mod schematic;
mod spring;
mod wave;
mod waves_ii;

pub use circuit::Circuit;
pub use collision::Collision1D;
pub use doppler::Doppler;
pub use electro::{ChargeGlow, ElectricField};
pub use energy::EnergyBars;
pub use field_lines::{Charge, Field, FieldLines};
pub use free_body::{Force, FreeBody};
pub use gas::{GasBox, GasMotion, GasSetup, Heat};
pub use lens::{Lens, LensShape, OpticalBench, Ray};
pub use magneto::{MagneticField, Wire, magnetic_field};
pub use optics::{Optic, RayDiagram, Refraction};
pub use orbit::{Orbit, eccentric_anomaly};
pub use pendulum::{Pendulum, PendulumMotion};
pub use projectile::Projectile;
pub use schematic::{Connection, Pin, Schematic, Symbol, SymbolKind};
pub use spring::{SpringMass, SpringMotion};
pub use wave::{Wave, WaveKind};
pub use waves_ii::RadialWave;

pub(crate) use super::label;

use fframes::Color;
use kurbo::{BezPath, Point, Vec2};

use crate::{
    geometry::Shape,
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

/// A zigzag from `a` to `b` with `coils` turns `amplitude` pixels to each side, straight
/// for the first and last tenth.
pub(crate) fn zigzag(a: Point, b: Point, coils: usize, amplitude: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(a);
    let d = b - a;
    let length = d.hypot();
    if length < 1e-9 {
        return path;
    }
    let along = d / length;
    let side = Vec2::new(-along.y, along.x) * amplitude;
    let lead = length * 0.1;
    let body = length - 2.0 * lead;
    let points = coils.max(1) * 2;
    path.line_to(a + along * lead);
    for i in 0..points {
        let f = (i as f64 + 0.5) / points as f64;
        let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
        path.line_to(a + along * (lead + body * f) + side * sign);
    }
    path.line_to(b - along * lead);
    path.line_to(b);
    path
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
