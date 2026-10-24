use std::f64::consts::TAU;

use kurbo::{Affine, Vec2};

use crate::{
    anim::RateFunc,
    geometry::{Geometry, Look, Shape},
    motion::lerp,
    ui::Ui,
};

use super::{ShapeAnimation, create::themed, enter_alpha};

/// Draws the part of the outline between two fractions of its length that move from `start`
/// to `end`, while the fill fades in (Manim's `ShowPartial`; `Create` is the move from
/// `(0, 0)` to `(0, 1)`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShowPartial {
    /// The drawn window when it starts, as fractions of the outline's length.
    pub start: (f32, f32),
    /// The drawn window when it ends.
    pub end: (f32, f32),
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(ShowPartial, |t| ShowPartial {
    start: (0.0, 0.0),
    end: (0.0, 1.0),
    delay: 0.0,
    run_time: 1.0,
    rate_func: t.motion.easing.standard,
});

impl ShapeAnimation for ShowPartial {
    fn look(&self, ui: &Ui<'_, '_, '_>, _shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        look.draw_window(
            lerp(self.start.0, self.end.0, p),
            lerp(self.start.1, self.end.1, p),
        );
        look.fill_opacity *= p;
    }
}

/// Hides a part that is not on.
fn hide_unless(on: bool, look: &mut Look) {
    if !on {
        look.opacity = 0.0;
    }
}

/// Shows the parts one more at a time, in order, each at once (Manim's
/// `ShowIncreasingSubsets`): part `i` of `n` appears when the progress reaches `(i + 1) / n`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShowIncreasingSubsets {
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(ShowIncreasingSubsets, |_t| ShowIncreasingSubsets {
    delay: 0.0,
    run_time: 1.0,
    rate_func: RateFunc::Smooth,
});

impl ShapeAnimation for ShowIncreasingSubsets {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        self.look_part(ui, shape, shape, 0, 1, look);
    }

    fn per_part(&self) -> bool {
        true
    }

    fn look_part(
        &self,
        ui: &Ui<'_, '_, '_>,
        _whole: &Shape,
        _part: &Shape,
        index: usize,
        count: usize,
        look: &mut Look,
    ) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let shown = (p * count as f32 + 1e-4).floor() as usize;
        hide_unless(index < shown, look);
    }
}

/// Shows one part at a time, in order; the last one stays (Manim's
/// `ShowSubmobjectsOneByOne`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShowSubmobjectsOneByOne {
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(ShowSubmobjectsOneByOne, |_t| ShowSubmobjectsOneByOne {
    delay: 0.0,
    run_time: 1.0,
    rate_func: RateFunc::Smooth,
});

impl ShapeAnimation for ShowSubmobjectsOneByOne {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        self.look_part(ui, shape, shape, 0, 1, look);
    }

    fn per_part(&self) -> bool {
        true
    }

    fn look_part(
        &self,
        ui: &Ui<'_, '_, '_>,
        _whole: &Shape,
        _part: &Shape,
        index: usize,
        count: usize,
        look: &mut Look,
    ) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let current = (p * count as f32 - 1e-4).ceil() as usize;
        hide_unless(current == index + 1, look);
    }
}

/// The parts fly in on a spiral around the center of the whole and fade in during the first
/// `fade_in_fraction` (Manim's `SpiralIn`). Each part starts `scale_factor` times its
/// distance from the center further out, a turn back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiralIn {
    pub scale_factor: f32,
    pub fade_in_fraction: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(SpiralIn, |_t| SpiralIn {
    scale_factor: 8.0,
    fade_in_fraction: 0.3,
    delay: 0.0,
    run_time: 1.0,
    rate_func: RateFunc::Smooth,
});

impl SpiralIn {
    /// Where the center of a part `offset` from the whole's center is at progress `p`,
    /// relative to the whole's center.
    pub(crate) fn offset_at(&self, offset: Vec2, p: f32) -> Vec2 {
        let rest = f64::from(1.0 - p);
        let out = offset * (1.0 + f64::from(self.scale_factor) * rest);
        Affine::rotate(-TAU * rest) * out.to_point() - kurbo::Point::ORIGIN
    }
}

impl ShapeAnimation for SpiralIn {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        self.look_part(ui, shape, shape, 0, 1, look);
    }

    fn per_part(&self) -> bool {
        true
    }

    fn look_part(
        &self,
        ui: &Ui<'_, '_, '_>,
        whole: &Shape,
        part: &Shape,
        _index: usize,
        _count: usize,
        look: &mut Look,
    ) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let center = whole.bbox().center();
        let at = part.bbox().center();
        let moved = self.offset_at(at - center, p);
        look.then(Affine::translate(center + moved - at));
        if self.fade_in_fraction > 0.0 {
            look.opacity *= (p / self.fade_in_fraction).min(1.0);
        }
        if p <= 0.0 {
            look.opacity = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spiral_ends_in_place_and_starts_further_out() {
        let s = SpiralIn::default();
        let offset = Vec2::new(10.0, 0.0);
        assert!((s.offset_at(offset, 1.0) - offset).hypot() < 1e-9);
        assert!((s.offset_at(offset, 0.0).hypot() - 90.0).abs() < 1e-6);
    }
}
