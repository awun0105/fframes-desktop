//! Box shadows per appearance.

use gpui::{BoxShadow, hsla, point, px};
use smallvec::{SmallVec, smallvec};

use super::{Appearance, appearance};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shadow {
    Card,
    Pop,
    Preview,
    RaisedPill,
    Thumb,
}

fn black(alpha: f32, y: f32, blur: f32, spread: f32) -> BoxShadow {
    BoxShadow {
        color: hsla(0., 0., 0., alpha),
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(spread),
        inset: false,
    }
}

pub fn shadow_for(appearance: Appearance, kind: Shadow) -> SmallVec<[BoxShadow; 2]> {
    let dark = appearance == Appearance::Dark;
    let pick = |light: f32, dark_value: f32| if dark { dark_value } else { light };
    match kind {
        Shadow::Card => smallvec![black(pick(0.04, 0.30), 1., 2., 0.)],
        Shadow::Pop => smallvec![black(
            pick(0.25, 0.60),
            pick(12., 16.),
            pick(32., 40.),
            -12.
        )],
        Shadow::Preview => smallvec![black(0.35, 12., 32., -8.)],
        Shadow::RaisedPill => smallvec![black(0.12, 1., 2., 0.), black(0.06, 0., 0., 0.5)],
        Shadow::Thumb => smallvec![black(0.25, 1., 3., 0.)],
    }
}

/// Shadow for the current appearance.
pub fn shadow(kind: Shadow) -> SmallVec<[BoxShadow; 2]> {
    shadow_for(appearance(), kind)
}
