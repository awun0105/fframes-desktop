//! Cap's `render_ruler`: marks on the engine's resolution grid, taller and labelled on
//! whole seconds, the origin's label anchored left so it stays inside the axis.
use gpui::{ParentElement, Styled, div, prelude::FluentBuilder, px};
use studio_engine::TimelineTick;
use studio_theme::{METRICS, TextStyle, Theme, text_style};

const MAJOR_TICK_H: f32 = 8.;
const MINOR_TICK_H: f32 = 5.;
/// A box wide enough for `M:SS.ss`, centred on its mark.
const LABEL_W: f32 = 52.;
/// Labels sit this far above the ruler's bottom edge, over their mark.
const LABEL_BOTTOM: f32 = 10.;

/// The ruler over the time axis, `inset` from the left of the tracks.
pub(super) fn render_ruler(theme: &Theme, ticks: Vec<TimelineTick>, inset: f32) -> gpui::Div {
    let colors = theme.editor;
    let mut ruler = div()
        .absolute()
        .left(px(inset))
        .right_0()
        .top_0()
        .h(px(METRICS.ruler_h))
        .overflow_hidden();
    for tick in ticks {
        let x = tick.x as f32;
        ruler = ruler.child(
            div()
                .absolute()
                .left(px(x))
                .bottom_0()
                .w(px(1.))
                .h(px(if tick.major {
                    MAJOR_TICK_H
                } else {
                    MINOR_TICK_H
                }))
                .bg(colors.line_strong),
        );
        if let Some(label) = tick.label {
            let origin = tick.seconds == 0.;
            ruler = ruler.child(
                text_style(div(), TextStyle::Small)
                    .absolute()
                    .left(px(if origin { x } else { x - LABEL_W / 2. }))
                    .bottom(px(LABEL_BOTTOM))
                    .w(px(LABEL_W))
                    .flex()
                    .when(!origin, |label| label.justify_center())
                    .whitespace_nowrap()
                    .text_color(colors.text_3)
                    .child(label),
            );
        }
    }
    ruler
}
