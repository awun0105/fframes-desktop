//! The look of the 16:9 cut: the type, HUD and palette of the other cuts. Colour
//! is an accent, never a fill: blue for fframes, a lighter blue for speed, red
//! for the old way.

use fframes::{FFramesContext, FFramesSyncedVideoFrame, Frame, Svgr, SyncVideoFrameInput};

use crate::beat::*;
pub use crate::ui::{DISPLAY, MONO, SERIF};

pub const BG: &str = crate::ui::BG;
pub const TEXT: &str = crate::ui::BONE;
pub const ACCENT: &str = crate::ui::ACCENT;
/// The second accent, a lighter blue for speed and the GPU.
pub const SKY: &str = "#8fb8ff";
pub const RED: &str = "#ef4444";
pub const GREEN: &str = "#22c55e";
/// Secondary text.
pub const SLATE: &str = "#a8a39b";
/// Labels and inactive text.
pub const MUTED: &str = crate::ui::GREY;
/// Hairlines and borders.
pub const LINE: &str = crate::ui::DIM;
/// Surfaces on the page.
pub const PANEL: &str = "#141312";

/// Opacity and vertical offset of something that rises into place at beat `at`.
pub fn rise(lb: f32, at: f32, distance: f32) -> (f32, f32) {
    (prog(lb, at, at + 0.75), (1.0 - snap(lb - at)) * distance)
}

/// 1 until beat `at`, then fades to 0 within half a beat.
pub fn leave(lb: f32, at: f32) -> f32 {
    1.0 - prog(lb, at, at + 0.5)
}

/// `translate(x y)`.
pub fn at(x: f32, y: f32) -> String {
    format!("translate({x} {y})")
}

/// Scale around a point.
pub fn zoom(cx: f32, cy: f32, s: f32) -> String {
    format!("translate({cx} {cy}) scale({s}) translate({} {})", -cx, -cy)
}

/// Small spaced-out mono caps, like the labels of an engineering drawing.
pub fn eyebrow(x: f32, y: f32, text: impl Into<String>, color: &'static str) -> Svgr<'static> {
    let text = text.into();
    fframes::svgr!(
        <text x={x} y={y} font-family={MONO} font-weight="600" font-size="22" letter-spacing="4" fill={color}>{text}</text>
    )
}

/// A headline in the display face.
pub fn headline(
    x: f32,
    y: f32,
    text: impl Into<String>,
    size: f32,
    fill: &'static str,
) -> Svgr<'static> {
    let text = text.into();
    fframes::svgr!(
        <text x={x} y={y} font-family={DISPLAY} font-size={size} letter-spacing={-size * 0.03} fill={fill}>{text}</text>
    )
}

/// The act number and name top left, the way every act opens.
pub fn act_title(
    lb: f32,
    number: &'static str,
    name: &'static str,
    title: &'static str,
) -> Svgr<'static> {
    let (o1, d1) = rise(lb, 0.0, 24.0);
    let (o2, d2) = rise(lb, 0.5, 40.0);
    fframes::svgr!(
        <g>
            <g opacity={o1} transform={at(0.0, d1)}>
                <text x="120" y="168" font-family={MONO} font-weight="600" font-size="22" letter-spacing="4" fill={ACCENT}>{number}<tspan fill={MUTED}>" / "</tspan><tspan fill={SLATE}>{name}</tspan></text>
            </g>
            <g opacity={o2} transform={at(0.0, d2)}>{headline(116.0, 250.0, title, 64.0, TEXT)}</g>
        </g>
    )
}

/// The plain-English caption under the picture: a lead in an accent colour and
/// the rest in white, centred at the bottom of the frame.
pub fn caption(
    lb: f32,
    from: f32,
    until: f32,
    lead: &'static str,
    rest: &'static str,
    color: &'static str,
) -> Svgr<'static> {
    if lb < from || lb > until + 0.5 {
        return Svgr::empty();
    }
    let (o, d) = rise(lb, from, 18.0);
    let o = o * leave(lb, until);
    fframes::svgr!(
        <g opacity={o} transform={at(0.0, d)}>
            <text x="960" y="972" text-anchor="middle" font-family={MONO} font-weight="500" font-size="30" fill={TEXT}><tspan font-weight="600" fill={color}>{lead}</tspan>" "{rest}</text>
        </g>
    )
}

/// A surface with a hairline border.
pub fn panel(x: f32, y: f32, w: f32, h: f32) -> Svgr<'static> {
    fframes::svgr!(
        <rect x={x} y={y} width={w} height={h} rx="14" fill={PANEL} stroke={LINE} stroke-width="2" />
    )
}

/// An application window: a panel with a title bar.
pub fn window(x: f32, y: f32, w: f32, h: f32, title: impl Into<String>) -> Svgr<'static> {
    let title = title.into();
    fframes::svgr!(
        <g>
            {panel(x, y, w, h)}
            <path d={format!("M{} {} H{}", x + 1.0, y + 44.0, x + w - 1.0)} stroke={LINE} stroke-width="2" />
            <circle cx={x + 24.0} cy={y + 22.0} r="6" fill="#3a3834" />
            <circle cx={x + 44.0} cy={y + 22.0} r="6" fill="#3a3834" />
            <circle cx={x + 64.0} cy={y + 22.0} r="6" fill="#3a3834" />
            <text x={x + w / 2.0} y={y + 29.0} text-anchor="middle" font-family={MONO} font-weight="500" font-size="17" fill={MUTED}>{title}</text>
        </g>
    )
}

/// A frame of the clip `name` filling the box (cropped to it), playing from
/// `start_from` seconds of the scene. `poster` stands in before that and where
/// frames cannot be decoded (the web editor).
pub fn clip(
    frame: &Frame,
    ctx: &FFramesContext<'_, '_>,
    name: &str,
    poster: &str,
    start_from: f32,
    (x, y, w, h): (f32, f32, f32, f32),
) -> Svgr<'static> {
    let input = SyncVideoFrameInput {
        start_from,
        looping: true,
        editor_fallback_image: ctx.get_image(poster),
    };
    let Some(image) = frame
        .get_synced_video_frame(ctx, name, &input)
        .map(|f| f.into_image())
    else {
        return still(ctx, poster, (x, y, w, h));
    };
    fframes::svgr!(
        <image href={image.href()} x={x} y={y} width={w} height={h} preserveAspectRatio="xMidYMid slice" />
    )
}

/// The image `name` filling the box (cropped to it).
pub fn still(
    ctx: &FFramesContext<'_, '_>,
    name: &str,
    (x, y, w, h): (f32, f32, f32, f32),
) -> Svgr<'static> {
    let Some(image) = ctx.get_image(name) else {
        return fframes::svgr!(<rect x={x} y={y} width={w} height={h} fill={PANEL} />);
    };
    fframes::svgr!(
        <image href={image.href()} x={x} y={y} width={w} height={h} preserveAspectRatio="xMidYMid slice" />
    )
}

/// Clips `body` to a rounded box.
pub fn rounded(
    id: &str,
    (x, y, w, h): (f32, f32, f32, f32),
    r: f32,
    body: Svgr<'static>,
) -> Svgr<'static> {
    let url = format!("url(#{id})");
    fframes::svgr!(
        <g>
            <clipPath id={id.to_owned()}><rect x={x} y={y} width={w} height={h} rx={r} /></clipPath>
            <g clip-path={url}>{body}</g>
        </g>
    )
}
