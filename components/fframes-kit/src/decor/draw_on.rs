use fframes::Svgr;

use crate::{
    anim::RateFunc,
    animate::{Create, EnterAnim, draw_animated},
    geometry::Shape,
    ui::{Component, Themed, Ui},
};

/// Any [`Shape`] drawn on from the start of its path, the stroke reveal of an underline, a
/// tick or a hand-drawn arrow. It is [`Create`] on a shape you built: give the shape a
/// `stroke` (and no `fill` for a line).
#[derive(Debug, Clone, PartialEq)]
pub struct DrawOn {
    pub shape: Shape,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl DrawOn {
    /// `shape` with the theme's `Create` timing.
    pub fn new(ui: &Ui<'_, '_, '_>, shape: Shape) -> Self {
        let create = Create::themed(ui.theme);
        Self {
            shape,
            delay: create.delay,
            run_time: create.run_time,
            rate_func: create.rate_func,
        }
    }
}

impl<'a> Component<'a> for DrawOn {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let enter: EnterAnim = Create {
            delay: self.delay,
            run_time: self.run_time,
            rate_func: self.rate_func,
        }
        .into();
        draw_animated(ui, &[self.shape], Some(&enter), None, None)
    }
}
