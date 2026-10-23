use crate::{
    anim::RateFunc,
    geometry::{Look, Shape},
    ui::Ui,
};

use super::{ShapeAnimation, enter_alpha, exit_alpha};

/// `Themed` and `Default` (the default theme) for an animation.
macro_rules! themed {
    ($name:ident, |$t:ident| $body:expr) => {
        impl $crate::ui::Themed for $name {
            fn themed($t: &$crate::theme::Theme) -> Self {
                $body
            }
        }

        impl Default for $name {
            fn default() -> Self {
                <Self as $crate::ui::Themed>::themed(&$crate::theme::Theme::default())
            }
        }
    };
}
pub(crate) use themed;

/// Draws the outline from its start while the fill fades in (Manim's `Create`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Create {
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(Create, |t| Create {
    delay: 0.0,
    run_time: 1.0,
    rate_func: t.motion.easing.standard,
});

impl ShapeAnimation for Create {
    fn look(&self, ui: &Ui<'_, '_, '_>, _shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        look.draw_window(0.0, p);
        look.fill_opacity *= p;
    }
}

/// `Create` backwards before the end (Manim's `Uncreate`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Uncreate {
    /// Seconds between the end of the animation and the end of `ui.duration`.
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

themed!(Uncreate, |t| Uncreate {
    delay: 0.0,
    run_time: 1.0,
    rate_func: t.motion.easing.standard,
});

impl ShapeAnimation for Uncreate {
    fn look(&self, ui: &Ui<'_, '_, '_>, _shape: &Shape, look: &mut Look) {
        let Some(p) = exit_alpha(ui, self.delay, self.run_time, self.rate_func) else {
            return;
        };
        look.draw_window(0.0, 1.0 - p);
        look.fill_opacity *= 1.0 - p;
    }
}

/// Draws the border in the first half, then fades the fill in (Manim's
/// `DrawBorderThenFill`). A shape without an outline gets one in its fill colour while it is
/// drawn, which fades out as the fill arrives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawBorderThenFill {
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
    /// Width of the temporary border of a shape without an outline.
    pub stroke_width: f32,
}

themed!(DrawBorderThenFill, |t| DrawBorderThenFill {
    delay: 0.0,
    run_time: 2.0,
    rate_func: t.motion.easing.standard,
    stroke_width: t.stroke.thin,
});

impl DrawBorderThenFill {
    /// The look at eased progress `p`.
    pub(crate) fn at(&self, p: f32, shape: &Shape, look: &mut Look) {
        let border = (2.0 * p).min(1.0);
        let fill = (2.0 * p - 1.0).max(0.0);
        look.draw_window(0.0, border);
        look.fill_opacity *= fill;
        if shape.stroke.is_none() {
            look.stroke = shape.fill;
            look.stroke_width = Some(self.stroke_width);
            look.stroke_opacity *= 1.0 - fill;
        }
    }
}

impl ShapeAnimation for DrawBorderThenFill {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        let p = enter_alpha(ui, self.delay, self.run_time, self.rate_func);
        self.at(p, shape, look);
    }
}
