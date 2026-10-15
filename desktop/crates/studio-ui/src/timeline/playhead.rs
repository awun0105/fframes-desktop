//! The playhead. It moves on every playback tick, so the shell paints it over the cached
//! timeline instead of the timeline drawing it: a notify on the timeline, or on any view
//! inside it, would render the whole timeline again.
use gpui::{Bounds, ContentMask, Hsla, IntoElement, Pixels, Styled, canvas, fill, point, px, size};
use studio_theme::{METRICS, Theme};

const PLAYHEAD_HEAD: f32 = 12.;
const PLAYHEAD_RING: f32 = 4.;

/// Where the playhead is, in window pixels: the time axis it is clipped to and its x.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayheadGeometry {
    pub axis: Bounds<Pixels>,
    pub x: f32,
}

/// An overlay that paints the playhead line and its head on the ruler.
pub fn playhead_overlay(theme: &Theme, geometry: Option<PlayheadGeometry>) -> impl IntoElement {
    let line = Hsla::from(theme.editor.playhead);
    let card = Hsla::from(theme.editor.card);
    canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            let Some(PlayheadGeometry { axis, x }) = geometry else {
                return;
            };
            window.with_content_mask(Some(ContentMask { bounds: axis }), |window| {
                window.paint_quad(fill(
                    Bounds::new(point(px(x), axis.top()), size(px(1.), axis.size.height)),
                    line,
                ));
                let ring = PLAYHEAD_HEAD + 2. * PLAYHEAD_RING;
                let ring_origin = point(
                    px(x + 0.5 - ring / 2.),
                    axis.top() + px((METRICS.ruler_h - ring) / 2.),
                );
                window.paint_quad(
                    fill(Bounds::new(ring_origin, size(px(ring), px(ring))), card)
                        .corner_radii(px(ring / 2.)),
                );
                let head_origin = ring_origin + point(px(PLAYHEAD_RING), px(PLAYHEAD_RING));
                window.paint_quad(
                    fill(
                        Bounds::new(head_origin, size(px(PLAYHEAD_HEAD), px(PLAYHEAD_HEAD))),
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
