//! The playhead. It moves on every playback tick, so the shell paints it over the cached
//! timeline instead of the timeline drawing it: a notify on the timeline, or on any view
//! inside it, would render the whole timeline again.
use gpui::{Bounds, ContentMask, Hsla, IntoElement, Pixels, Styled, canvas, fill, point, px, size};
use studio_theme::{METRICS, Theme};

const PLAYHEAD_HEAD: f32 = 10.;
const PLAYHEAD_LINE: f32 = 1.5;

/// Where the playhead is, in window pixels: the time axis it is clipped to and its x.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayheadGeometry {
    pub axis: Bounds<Pixels>,
    pub x: f32,
}

/// An overlay that paints the playhead line and its head on the ruler.
pub fn playhead_overlay(theme: &Theme, geometry: Option<PlayheadGeometry>) -> impl IntoElement {
    let line = Hsla::from(theme.editor.playhead);
    canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            let Some(PlayheadGeometry { axis, x }) = geometry else {
                return;
            };
            window.with_content_mask(Some(ContentMask { bounds: axis }), |window| {
                // Cap's playhead: a 1.5 px line under a round head centred on the ruler.
                let center = x + 0.5;
                window.paint_quad(fill(
                    Bounds::new(
                        point(px(center - PLAYHEAD_LINE / 2.), axis.top()),
                        size(px(PLAYHEAD_LINE), axis.size.height),
                    ),
                    line,
                ));
                let head = point(
                    px(center - PLAYHEAD_HEAD / 2.),
                    axis.top() + px((METRICS.ruler_h - PLAYHEAD_HEAD) / 2.),
                );
                window.paint_quad(
                    fill(
                        Bounds::new(head, size(px(PLAYHEAD_HEAD), px(PLAYHEAD_HEAD))),
                        line,
                    )
                    .corner_radii(px(PLAYHEAD_HEAD / 2.)),
                );
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}
