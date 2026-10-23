use kurbo::{PathEl, Point};

use crate::{
    anim::RateFunc,
    geometry::{Geometry, Look, Shape},
    ui::Ui,
};

use super::{ShapeAnimation, about, create::themed, enter_alpha, grow_scale};

/// Grows the shape from nothing about the center of its bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrowFromCenter {
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(GrowFromCenter, |t| GrowFromCenter {
    delay: 0.0,
    run_time: 1.0,
    rate_func: t.motion.easing.standard,
});

impl ShapeAnimation for GrowFromCenter {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        look.then(about(shape.bbox().center(), grow_scale(p), 0.0));
    }
}

/// Grows the shape from nothing about `point` (canvas units).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrowFromPoint {
    pub point: (f32, f32),
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(GrowFromPoint, |t| GrowFromPoint {
    point: (0.0, 0.0),
    delay: 0.0,
    run_time: 1.0,
    rate_func: t.motion.easing.standard,
});

impl ShapeAnimation for GrowFromPoint {
    fn look(&self, ui: &Ui<'_, '_, '_>, _shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let point = Point::new(f64::from(self.point.0), f64::from(self.point.1));
        look.then(about(point, grow_scale(p), 0.0));
    }
}

/// Grows the shape from the start of its path: an arrow grows from its tail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrowArrow {
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(GrowArrow, |t| GrowArrow {
    delay: 0.0,
    run_time: 1.0,
    rate_func: t.motion.easing.standard,
});

impl ShapeAnimation for GrowArrow {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let start = match shape.path.elements().first() {
            Some(PathEl::MoveTo(p)) => *p,
            _ => shape.bbox().center(),
        };
        look.then(about(start, grow_scale(p), 0.0));
    }
}

/// Grows the shape from its center while it turns by `angle` radians into place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinInFromNothing {
    pub angle: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(SpinInFromNothing, |t| SpinInFromNothing {
    angle: std::f32::consts::FRAC_PI_2,
    delay: 0.0,
    run_time: 1.0,
    rate_func: t.motion.easing.standard,
});

impl ShapeAnimation for SpinInFromNothing {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let angle = f64::from(self.angle * (1.0 - p));
        look.then(about(shape.bbox().center(), grow_scale(p), angle));
    }
}
