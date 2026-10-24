use fframes::Svgr;

use crate::ui::Ui;

/// Which of the two items of a transition a presentation draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The item that comes in; it is drawn on top.
    Entering,
    /// The item that goes away.
    Exiting,
}

/// Where a `Slide` or `Wipe` comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    FromLeft,
    FromRight,
    FromTop,
    FromBottom,
}

impl Direction {
    /// The way the entering item moves, as a unit vector (`y` down).
    pub fn travel(self) -> (f32, f32) {
        match self {
            Direction::FromLeft => (1.0, 0.0),
            Direction::FromRight => (-1.0, 0.0),
            Direction::FromTop => (0.0, 1.0),
            Direction::FromBottom => (0.0, -1.0),
        }
    }
}

/// How a transition draws its two items (Remotion's presentations). Both sides get the
/// same `progress`, 0 when the transition starts and 1 when it ends; a spring timing may
/// overshoot 1. `id` is unique to this transition in the frame, for clip paths, masks and
/// gradients.
pub trait Presentation {
    fn apply<'a>(
        &self,
        progress: f32,
        side: Side,
        content: Svgr<'a>,
        ui: &Ui<'a, '_, '_>,
        id: &str,
    ) -> Svgr<'a>;
}

/// The distance from `center` to the farthest corner of a `size` area: the radius of a circle
/// that covers it.
pub(crate) fn reach(center: (f32, f32), size: (f32, f32)) -> f32 {
    let dx = center.0.max(size.0 - center.0);
    let dy = center.1.max(size.1 - center.1);
    dx.hypot(dy)
}

/// `content` clipped to `shape`, a clip path with `id`.
pub(crate) fn clipped<'a>(content: Svgr<'a>, shape: Svgr<'a>, id: &str) -> Svgr<'a> {
    let url = format!("url(#{id})");
    let id = id.to_owned();
    fframes::svgr!(
        <g>
            <defs>
                <clipPath id={id}>{shape}</clipPath>
            </defs>
            <g clip-path={url}>{content}</g>
        </g>
    )
}

/// `content` scaled by `scale` and turned by `degrees` about `origin`, unchanged when both are
/// neutral.
pub(crate) fn zoomed(content: Svgr<'_>, scale: f32, degrees: f32, origin: (f32, f32)) -> Svgr<'_> {
    if scale == 1.0 && degrees == 0.0 {
        return content;
    }
    let (cx, cy) = (f64::from(origin.0), f64::from(origin.1));
    let affine = kurbo::Affine::translate((cx, cy))
        * kurbo::Affine::rotate(f64::from(degrees).to_radians())
        * kurbo::Affine::scale(f64::from(scale))
        * kurbo::Affine::translate((-cx, -cy));
    let transform = crate::geometry::matrix(affine);
    fframes::svgr!(<g transform={transform}>{content}</g>)
}

/// `content` blurred by a Gaussian with standard deviations `sx` and `sy` (view-box units)
/// through a filter and a clip path named from `id` and `side` (both sides of a transition
/// may blur in one frame), unchanged when both are 0.
///
/// The content is first clipped to `area` (from the origin, in the caller's units), so every
/// backend blurs the same pixels: the CPU backend never sees what lies past the canvas, the
/// Skia backend would. The blur then fades the alpha towards the clip edge, to a half at a
/// straight edge in theory and to about a quarter with the CPU backend's box blur at large
/// deviations; four times the alpha keeps opaque content opaque up to the edge on both, so
/// the canvas does not darken at its sides. The presentations expect items that cover the
/// canvas: translucent content draws opaque wherever the blur leaves it a quarter opaque.
pub(crate) fn blurred<'a>(
    content: Svgr<'a>,
    sx: f32,
    sy: f32,
    area: (f32, f32),
    id: &str,
    side: Side,
) -> Svgr<'a> {
    let (sx, sy) = (sx.max(0.0), sy.max(0.0));
    if sx == 0.0 && sy == 0.0 {
        return content;
    }
    let (w, h) = area;
    let id = match side {
        Side::Entering => format!("{id}-blur-in"),
        Side::Exiting => format!("{id}-blur-out"),
    };
    let clip = format!("{id}-area");
    let clip_url = format!("url(#{clip})");
    let url = format!("url(#{id})");
    let deviation = format!("{sx} {sy}");
    fframes::svgr!(
        <g>
            <defs>
                <clipPath id={clip}>
                    <rect x="0" y="0" width={w} height={h} />
                </clipPath>
                <filter id={id} x="0" y="0" width="100%" height="100%"
                        color-interpolation-filters="sRGB">
                    <feGaussianBlur stdDeviation={deviation} />
                    <feColorMatrix type="matrix"
                                   values="1 0 0 0 0 0 1 0 0 0 0 0 1 0 0 0 0 0 4 0" />
                </filter>
            </defs>
            <g filter={url}>
                <g clip-path={clip_url}>{content}</g>
            </g>
        </g>
    )
}

/// 0 at the ends of a transition and 1 in its middle.
pub(crate) fn bell(progress: f32) -> f32 {
    1.0 - (2.0 * crate::motion::clamp01(progress) - 1.0).abs()
}
