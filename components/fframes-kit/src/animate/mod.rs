//! Manim animations that need a shape's geometry, and the timing groups.
//!
//! Shape components take plain-data animation props: `enter: Some(Create::default().into())`,
//! `exit`, `emphasis`. Each animation changes the shape's [`Look`] (transform, opacities, the
//! drawn part of the outline) and never its path, so a static shape keeps its `d`. Only the
//! morphs ([`Transform`], [`ReplacementTransform`]) build a new path per frame.
//!
//! Times are local: enter and emphasis animations count from the start of the enclosing
//! container plus `delay`, exit animations back from its end.

mod apply;
mod create;
mod emphasis;
mod group;
mod grow;
mod homotopy;
mod indicate_ii;
mod rotate;
mod subsets;
mod text_by;
mod traced_path;
mod transform;
mod transform_ii;
mod write;

pub use apply::{
    ApplyComplexFunction, ApplyFunction, ApplyMatrix, ApplyPointwiseFunction,
    ApplyPointwiseFunctionToCenter,
};
pub use create::{Create, DrawBorderThenFill, Uncreate};
pub use emphasis::{Circumscribe, Flash, Indicate, Wiggle};
pub use group::{AnimationGroup, LaggedStart, Succession};
pub use grow::{GrowArrow, GrowFromCenter, GrowFromEdge, GrowFromPoint, SpinInFromNothing};
pub use homotopy::{
    ComplexHomotopy, HOMOTOPY_SEGMENTS, Homotopy, PHASE_FLOW_STEPS, PhaseFlow,
    SmoothedVectorizedHomotopy,
};
pub use indicate_ii::{
    AnimatedBoundary, ApplyWave, Blink, Broadcast, FocusOn, ShowPassingFlash,
    ShowPassingFlashWithThinningStrokeWidth,
};
pub use rotate::{Rotate, Rotating};
pub use subsets::{ShowIncreasingSubsets, ShowPartial, ShowSubmobjectsOneByOne, SpiralIn};
pub use text_by::{AddTextLetterByLetter, RemoveTextLetterByLetter};
pub use traced_path::{MAX_TRACE_SAMPLES, TracedPath};
#[cfg(feature = "latex")]
pub(crate) use transform::{MORPH_SEGMENTS, lerp_paint, morph};
pub use transform::{MoveAlongPath, ReplacementTransform, Transform};
pub use transform_ii::{
    ClockwiseTransform, CounterclockwiseTransform, CyclicReplace, FadeToColor, FadeTransform,
    FadeTransformPieces, ScaleInPlace, ShrinkToCenter, Swap, TransformAnimations,
    TransformFromCopy, TransformMatchingShapes, match_shapes, same_shape, shape_key,
};
pub use write::{Unwrite, Write};

use fframes::Svgr;
use kurbo::{Affine, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, RateFunc},
    geometry::{Look, Shape},
    motion::progress,
    ui::Ui,
};

/// An animation that changes how a shape is drawn.
pub trait ShapeAnimation {
    /// Changes `look` for the current time.
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look);

    /// Extra drawing on top of the shape (`Flash` lines, the `Circumscribe` outline).
    fn overlay<'a>(&self, _ui: &Ui<'a, '_, '_>, _shape: &Shape) -> Option<Svgr<'a>> {
        None
    }

    /// Whether every part (a glyph of a text) gets its own [`look_part`](Self::look_part),
    /// as in `Write`, instead of the whole sharing one look.
    fn per_part(&self) -> bool {
        false
    }

    /// Changes `look` of part `index` of `count`; `whole` holds every part. By default the
    /// part looks like the whole.
    fn look_part(
        &self,
        ui: &Ui<'_, '_, '_>,
        whole: &Shape,
        _part: &Shape,
        _index: usize,
        _count: usize,
        look: &mut Look,
    ) {
        self.look(ui, whole, look);
    }
}

/// Eased progress of an animation that starts `delay` seconds after the local start.
pub(crate) fn enter_alpha(ui: &Ui<'_, '_, '_>, delay: f32, run_time: f32, rate: RateFunc) -> f32 {
    let t = progress(ui.seconds(), delay, delay + run_time);
    rate.apply(t, run_time)
}

/// Eased progress of an animation that ends `delay` seconds before the end of `ui.duration`;
/// `None` before it starts or without a known end.
pub(crate) fn exit_alpha(
    ui: &Ui<'_, '_, '_>,
    delay: f32,
    run_time: f32,
    rate: RateFunc,
) -> Option<f32> {
    let stop = ui.duration_seconds()? - delay;
    let t = progress(ui.seconds(), stop - run_time, stop);
    (t > 0.0).then(|| rate.apply(t, run_time))
}

/// Scales by `s` (and rotates by `angle` radians) about `center`.
pub(crate) fn about(center: Point, s: f64, angle: f64) -> Affine {
    let c = center.to_vec2();
    Affine::translate(c) * Affine::rotate(angle) * Affine::scale(s) * Affine::translate(-c)
}

/// A scale that never collapses the matrix to zero.
pub(crate) fn grow_scale(alpha: f32) -> f64 {
    f64::from(alpha).max(1e-3)
}

impl ShapeAnimation for FadeIn {
    fn look(&self, ui: &Ui<'_, '_, '_>, _shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        let rest = f64::from(1.0 - p);
        look.opacity *= p;
        look.then(Affine::translate(Vec2::new(
            -f64::from(self.shift.0) * rest,
            -f64::from(self.shift.1) * rest,
        )));
    }
}

impl ShapeAnimation for FadeOut {
    fn look(&self, ui: &Ui<'_, '_, '_>, _shape: &Shape, look: &mut Look) {
        let Some(p) = exit_alpha(ui, self.delay, self.run_time, self.rate_func) else {
            return;
        };
        look.opacity *= 1.0 - p;
        look.then(Affine::translate(Vec2::new(
            f64::from(self.shift.0 * p),
            f64::from(self.shift.1 * p),
        )));
    }
}

/// The `enter` prop of shapes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnterAnim {
    FadeIn(FadeIn),
    Create(Create),
    DrawBorderThenFill(DrawBorderThenFill),
    Write(Write),
    GrowFromCenter(GrowFromCenter),
    GrowFromPoint(GrowFromPoint),
    GrowArrow(GrowArrow),
    SpinInFromNothing(SpinInFromNothing),
    ShowPartial(ShowPartial),
    ShowIncreasingSubsets(ShowIncreasingSubsets),
    ShowSubmobjectsOneByOne(ShowSubmobjectsOneByOne),
    SpiralIn(SpiralIn),
    AddTextLetterByLetter(AddTextLetterByLetter),
    GrowFromEdge(GrowFromEdge),
}

/// The `exit` prop of shapes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExitAnim {
    FadeOut(FadeOut),
    Uncreate(Uncreate),
    Unwrite(Unwrite),
    RemoveTextLetterByLetter(RemoveTextLetterByLetter),
    ShrinkToCenter(ShrinkToCenter),
}

/// The `emphasis` prop of shapes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Emphasis {
    Indicate(Indicate),
    Circumscribe(Circumscribe),
    Flash(Flash),
    Wiggle(Wiggle),
    Blink(Blink),
    FocusOn(FocusOn),
    Rotate(Rotate),
    Rotating(Rotating),
    FadeToColor(FadeToColor),
    ScaleInPlace(ScaleInPlace),
}

macro_rules! slot {
    ($slot:ident: $($anim:ident),*) => {
        $(impl From<$anim> for $slot {
            fn from(a: $anim) -> Self {
                $slot::$anim(a)
            }
        })*

        impl $slot {
            fn as_animation(&self) -> &dyn ShapeAnimation {
                match self {
                    $($slot::$anim(a) => a,)*
                }
            }
        }

        impl ShapeAnimation for $slot {
            fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
                self.as_animation().look(ui, shape, look);
            }

            fn overlay<'a>(&self, ui: &Ui<'a, '_, '_>, shape: &Shape) -> Option<Svgr<'a>> {
                self.as_animation().overlay(ui, shape)
            }

            fn per_part(&self) -> bool {
                self.as_animation().per_part()
            }

            fn look_part(
                &self,
                ui: &Ui<'_, '_, '_>,
                whole: &Shape,
                part: &Shape,
                index: usize,
                count: usize,
                look: &mut Look,
            ) {
                self.as_animation().look_part(ui, whole, part, index, count, look);
            }
        }
    };
}

slot!(EnterAnim: FadeIn, Create, DrawBorderThenFill, Write, GrowFromCenter, GrowFromPoint, GrowArrow, SpinInFromNothing, ShowPartial, ShowIncreasingSubsets, ShowSubmobjectsOneByOne, SpiralIn, AddTextLetterByLetter, GrowFromEdge);
slot!(ExitAnim: FadeOut, Uncreate, Unwrite, RemoveTextLetterByLetter, ShrinkToCenter);
slot!(Emphasis: Indicate, Circumscribe, Flash, Wiggle, Blink, FocusOn, Rotate, Rotating, FadeToColor, ScaleInPlace);

/// Draws `parts` (the glyphs of a text, or one shape) with the animation props applied.
/// Animations whose [`ShapeAnimation::per_part`] is set (`Write`) give each part its own
/// look; the others treat the parts as one.
pub fn draw_animated<'a>(
    ui: &Ui<'a, '_, '_>,
    parts: &[Shape],
    enter: Option<&EnterAnim>,
    exit: Option<&ExitAnim>,
    emphasis: Option<&Emphasis>,
) -> Svgr<'a> {
    let Some(first) = parts.first() else {
        return Svgr::empty();
    };
    let joined;
    let whole = if parts.len() == 1 {
        first
    } else {
        joined = whole_shape(parts);
        &joined
    };
    let mut look = Look::default();
    if let Some(e) = emphasis {
        e.look(ui, whole, &mut look);
    }
    let overlay = emphasis.and_then(|e| e.overlay(ui, whole));
    let per_part =
        enter.is_some_and(ShapeAnimation::per_part) || exit.is_some_and(ShapeAnimation::per_part);
    let drawn: Svgr<'a> = if per_part {
        let count = parts.len();
        parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                let mut part_look = look;
                if let Some(x) = exit {
                    x.look_part(ui, whole, part, i, count, &mut part_look);
                }
                if let Some(e) = enter {
                    e.look_part(ui, whole, part, i, count, &mut part_look);
                }
                part.draw_with(&part_look)
            })
            .collect()
    } else {
        if let Some(x) = exit {
            x.look(ui, whole, &mut look);
        }
        match enter {
            Some(e) => {
                e.look(ui, whole, &mut look);
                parts.iter().map(|p| p.draw_with(&look)).collect()
            }
            None if parts.len() == 1 => first.draw_with(&look),
            None => parts.iter().map(|p| p.draw_with(&look)).collect(),
        }
    };
    match overlay {
        Some(overlay) => fframes::svgr!(<g>{drawn}{overlay}</g>),
        None => drawn,
    }
}

/// One shape whose path holds every part, for animations that need the whole bounding box.
fn whole_shape(parts: &[Shape]) -> Shape {
    let mut whole = parts[0].clone();
    for p in &parts[1..] {
        whole.path.extend(p.path.iter());
    }
    whole
}
