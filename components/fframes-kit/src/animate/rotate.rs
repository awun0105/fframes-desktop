use std::f32::consts::{PI, TAU};

use kurbo::{Affine, Point};

use crate::{
    anim::RateFunc,
    geometry::{Geometry, Look, Shape},
    ui::Ui,
};

use super::{ShapeAnimation, about, create::themed, enter_alpha};

/// The turn by `angle` radians about `about_point` (or the center of `shape`) at progress
/// `p`. A positive angle turns counterclockwise on screen, as in Manim.
fn turned(shape: &Shape, about_point: Option<(f32, f32)>, angle: f32, p: f32) -> Affine {
    let center = about_point.map_or_else(
        || shape.bbox().center(),
        |(x, y)| Point::new(f64::from(x), f64::from(y)),
    );
    about(center, 1.0, -f64::from(angle * p))
}

/// Turns the shape by `angle` radians about a point and keeps it there (Manim's `Rotate`,
/// a half turn by default). A positive angle turns counterclockwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotate {
    pub angle: f32,
    /// The point to turn about, canvas units; `None` is the center of the shape.
    pub about_point: Option<(f32, f32)>,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(Rotate, |_t| Rotate {
    angle: PI,
    about_point: None,
    delay: 0.0,
    run_time: 1.0,
    rate_func: RateFunc::Smooth,
});

impl ShapeAnimation for Rotate {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        look.then(turned(shape, self.about_point, self.angle, p));
    }
}

/// Turns the shape at an even speed: `angle` radians every `run_time` seconds (Manim's
/// `Rotating`, a full turn in 5 seconds). With `run_time` it stops after one go like Manim;
/// with `forever` it keeps turning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotating {
    pub angle: f32,
    pub about_point: Option<(f32, f32)>,
    pub forever: bool,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(Rotating, |_t| Rotating {
    angle: TAU,
    about_point: None,
    forever: false,
    delay: 0.0,
    run_time: 5.0,
    rate_func: RateFunc::Linear,
});

impl Rotating {
    /// Turns done `t` seconds after the local start, as a fraction of `angle`.
    pub fn turns(&self, t: f32) -> f32 {
        let t = t - self.delay;
        if t <= 0.0 || self.run_time <= 0.0 {
            return 0.0;
        }
        let whole = (t / self.run_time).floor();
        if !self.forever && whole >= 1.0 {
            return 1.0;
        }
        whole
            + self
                .rate_func
                .apply(t / self.run_time - whole, self.run_time)
    }
}

impl ShapeAnimation for Rotating {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let turns = self.turns(ui.seconds());
        // a whole number of full turns is no turn: keep the angle small
        let angle = (self.angle * turns) % TAU;
        look.then(turned(shape, self.about_point, angle, 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Shape {
        Shape {
            path: crate::paths::parse_path("M0 0 L10 0 L10 10 L0 10 Z").unwrap(),
            fill: None,
            stroke: None,
            stroke_width: 1.0,
            dash: None,
            opacity: 1.0,
        }
    }

    #[test]
    fn a_full_turn_comes_back_to_the_start() {
        let a = turned(&square(), None, TAU, 1.0);
        let start = Point::new(3.0, 7.0);
        assert!((a * start - start).hypot() < 1e-4);
        // a quarter turn counterclockwise on screen: right of the center goes up
        let q = turned(&square(), Some((5.0, 5.0)), PI / 2.0, 1.0);
        let moved = q * Point::new(10.0, 5.0);
        assert!((moved - Point::new(5.0, 0.0)).hypot() < 1e-4, "{moved:?}");
    }

    #[test]
    fn rotating_stops_or_keeps_going() {
        let r = Rotating {
            run_time: 2.0,
            ..Rotating::default()
        };
        assert_eq!(r.turns(1.0), 0.5);
        assert_eq!(r.turns(5.0), 1.0);
        let forever = Rotating { forever: true, ..r };
        assert_eq!(forever.turns(5.0), 2.5);
    }
}
