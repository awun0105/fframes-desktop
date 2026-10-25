//! Mouse cursors drawn as SVG paths, and the motions of a screen recording: [`Cursor`],
//! [`CursorPath`], [`Click`], [`Drag`] and [`TypeAt`].

use std::f32::consts::PI;

use fframes::{Color, Svgr};

use crate::{
    anim::RateFunc,
    motion::{clamp01, lerp, progress, quart_out, spring},
    text::Typewriter,
    theme::Theme,
    ui::{Children, Component, Themed, Ui},
};

/// The shape of a [`Cursor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum CursorShape {
    /// The arrow; the hotspot is its tip.
    #[default]
    Arrow,
    /// The hand over a link; the hotspot is the tip of the finger.
    Pointer,
    /// The text cursor; the hotspot is its middle.
    IBeam,
    /// An open hand, over something that can be dragged.
    Grab,
    /// A closed hand, while dragging.
    Grabbing,
    /// Two crossed lines; the hotspot is where they cross.
    Crosshair,
}

/// The colours of a [`Cursor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum CursorStyle {
    /// Black with a white outline, the usual cursor.
    #[default]
    Dark,
    /// White with a black outline.
    Light,
}

impl CursorStyle {
    /// `(body, outline)`.
    fn colors(self) -> (Color, Color) {
        let (black, white) = (Color::hex("#111111"), Color::hex("#ffffff"));
        match self {
            CursorStyle::Dark => (black, white),
            CursorStyle::Light => (white, black),
        }
    }
}

/// The paths of a shape, in units of a 24 unit tall cursor.
struct Outline {
    /// The body, filled; for a stroked shape the lines.
    body: &'static str,
    /// Lines drawn in the outline colour over the body (the gaps between fingers).
    details: &'static str,
    hotspot: (f32, f32),
    /// The body is a set of lines, not an area.
    stroked: bool,
}

impl CursorShape {
    /// Every shape, in the order of the enum.
    pub const ALL: [CursorShape; 6] = [
        CursorShape::Arrow,
        CursorShape::Pointer,
        CursorShape::IBeam,
        CursorShape::Grab,
        CursorShape::Grabbing,
        CursorShape::Crosshair,
    ];

    fn outline(self) -> Outline {
        match self {
            CursorShape::Arrow => Outline {
                body: "M0 0 L0 17.5 L4.2 13.6 L7 20.2 L10.1 18.9 L7.4 12.4 L12.8 12.4 Z",
                details: "",
                hotspot: (0.0, 0.0),
                stroked: false,
            },
            CursorShape::Pointer => Outline {
                body: "M6 1.5 C6 0.7 6.7 0 7.5 0 C8.3 0 9 0.7 9 1.5 L9 8.5 C9 7.7 9.7 7 10.5 7 C11.3 7 12 7.7 12 8.5 L12 9.5 C12 8.7 12.7 8 13.5 8 C14.3 8 15 8.7 15 9.5 L15 10.5 C15 9.7 15.7 9 16.5 9 C17.3 9 18 9.7 18 10.5 L18 16 C18 19.5 15.5 22 12 22 L10.5 22 C8.5 22 7.2 21 6 19.5 L2.2 14.5 C1.6 13.7 1.8 12.6 2.6 12.1 C3.4 11.6 4.4 11.8 5 12.5 L6 13.8 Z",
                details: "M12 9.5 V13 M15 10.5 V13",
                hotspot: (7.5, 0.6),
                stroked: false,
            },
            CursorShape::IBeam => Outline {
                body: "M-4 -11 H-1.5 Q0 -11 0 -9.8 Q0 -11 1.5 -11 H4 V-9.4 H2.2 Q1 -9.4 1 -8.2 V8.2 Q1 9.4 2.2 9.4 H4 V11 H1.5 Q0 11 0 9.8 Q0 11 -1.5 11 H-4 V9.4 H-2.2 Q-1 9.4 -1 8.2 V-8.2 Q-1 -9.4 -2.2 -9.4 H-4 Z",
                details: "",
                hotspot: (0.0, 0.0),
                stroked: false,
            },
            CursorShape::Grab => Outline {
                body: "M5 11 V5 C5 4.2 5.7 3.5 6.5 3.5 C7.3 3.5 8 4.2 8 5 V3 C8 2.2 8.7 1.5 9.5 1.5 C10.3 1.5 11 2.2 11 3 V3.5 C11 2.7 11.7 2 12.5 2 C13.3 2 14 2.7 14 3.5 V5.5 C14 4.7 14.7 4 15.5 4 C16.3 4 17 4.7 17 5.5 V14 C17 18 14.5 21 11 21 H10 C7.5 21 6 19.8 4.8 18 L1.6 13 C1.1 12.2 1.3 11.2 2.1 10.8 C2.9 10.4 3.8 10.6 4.3 11.3 L5 12.3 Z",
                details: "M8 10.5 V5.5 M11 10.5 V4.5 M14 10.5 V6",
                hotspot: (10.0, 11.0),
                stroked: false,
            },
            CursorShape::Grabbing => Outline {
                body: "M4.5 12 V10.5 C4.5 9.7 5.2 9 6 9 C6.8 9 7.5 9.7 7.5 10.5 V9.5 C7.5 8.7 8.2 8 9 8 C9.8 8 10.5 8.7 10.5 9.5 V9 C10.5 8.2 11.2 7.5 12 7.5 C12.8 7.5 13.5 8.2 13.5 9 V9.5 C13.5 8.7 14.2 8 15 8 C15.8 8 16.5 8.7 16.5 9.5 V14 C16.5 18 14 20.5 10.8 20.5 H10 C7.5 20.5 5.8 19.3 5 17.5 L4.5 16 Z",
                details: "M7.5 10.5 V12.5 M10.5 9.5 V12 M13.5 9.5 V12",
                hotspot: (10.0, 13.0),
                stroked: false,
            },
            CursorShape::Crosshair => Outline {
                body: "M0 -10 V-3 M0 3 V10 M-10 0 H-3 M3 0 H10 M-0.6 0 H0.6",
                details: "",
                hotspot: (0.0, 0.0),
                stroked: true,
            },
        }
    }

    /// The hotspot in units of a 24 unit tall cursor, from the origin of its paths.
    pub fn hotspot(self) -> (f32, f32) {
        self.outline().hotspot
    }
}

/// A mouse cursor with its hotspot at `(x, y)` (the tip of the arrow, the middle of the
/// I-beam). `size` is the height in pixels of the 24 unit design, `scale` grows or shrinks
/// it about the hotspot (a press), `shadow` adds a soft shadow below it. The kit draws its own
/// shapes; they are not the artwork of any operating system.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Cursor {
    pub shape: CursorShape,
    pub style: CursorStyle,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub scale: f32,
    pub opacity: f32,
    pub shadow: bool,
}

impl Themed for Cursor {
    fn themed(_: &Theme) -> Self {
        Self {
            shape: CursorShape::Arrow,
            style: CursorStyle::Dark,
            x: 960.0,
            y: 540.0,
            size: 40.0,
            scale: 1.0,
            opacity: 1.0,
            shadow: true,
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Cursor {
    /// The cursor without a `Ui`; nothing when it is invisible.
    pub fn draw(&self) -> Svgr<'static> {
        if self.opacity <= 0.0 || self.scale <= 0.0 {
            return Svgr::empty();
        }
        let outline = self.shape.outline();
        let (body, edge) = self.style.colors();
        let (hx, hy) = outline.hotspot;
        let k = self.size / 24.0 * self.scale;
        let place = format!(
            "translate({} {}) scale({k}) translate({} {})",
            self.x, self.y, -hx, -hy
        );
        let d = outline.body;
        let shadow = if self.shadow {
            let shadow = if outline.stroked {
                fframes::svgr!(<path d={d} fill="none" stroke="#000" stroke-width="3.6" stroke-linecap="round" />)
            } else {
                fframes::svgr!(<path d={d} fill="#000" stroke="#000" stroke-width="2.6" stroke-linejoin="round" />)
            };
            fframes::svgr!(<g transform="translate(0.5 1.2)" opacity="0.28">{shadow}</g>)
        } else {
            Svgr::empty()
        };
        let shape = if outline.stroked {
            fframes::svgr!(
                <g fill="none" stroke-linecap="round">
                    <path d={d} stroke={edge} stroke-width="3.6" />
                    <path d={d} stroke={body} stroke-width="1.6" />
                </g>
            )
        } else {
            let details = if outline.details.is_empty() {
                Svgr::empty()
            } else {
                let lines = outline.details;
                fframes::svgr!(<path d={lines} fill="none" stroke={edge} stroke-width="0.9" stroke-linecap="round" />)
            };
            fframes::svgr!(
                <g>
                    <path d={d} fill={edge} stroke={edge} stroke-width="2.6" stroke-linejoin="round" />
                    <path d={d} fill={body} />
                    {details}
                </g>
            )
        };
        fframes::svgr!(<g transform={place} opacity={self.opacity}>{shadow}{shape}</g>)
    }
}

impl<'a> Component<'a> for Cursor {
    fn render(self, _ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        self.draw()
    }
}

/// How a click looks: the cursor shrinks to `depth` and back over `press` seconds, then a
/// ring of `color` grows to `radius` and fades over `run_time` seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClickLook {
    pub color: Color,
    pub radius: f32,
    pub stroke_width: f32,
    pub press: f32,
    pub depth: f32,
    pub run_time: f32,
}

impl Themed for ClickLook {
    fn themed(t: &Theme) -> Self {
        Self {
            color: t.colors.accent,
            radius: 34.0,
            stroke_width: 3.0,
            press: 0.18,
            depth: 0.82,
            run_time: 0.55,
        }
    }
}

impl Default for ClickLook {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl ClickLook {
    /// The scale of the cursor `dt` seconds after a press started.
    pub fn scale(&self, dt: f32) -> f32 {
        if dt <= 0.0 || dt >= self.press {
            return 1.0;
        }
        1.0 - (1.0 - self.depth) * (PI * dt / self.press).sin()
    }

    /// How far the ring has got `dt` seconds after the press started, from 0 to 1; `None`
    /// before the bottom of the press and after the ring faded.
    pub fn ripple_progress(&self, dt: f32) -> Option<f32> {
        let dt = dt - self.press / 2.0;
        (dt > 0.0 && dt < self.run_time).then(|| dt / self.run_time)
    }

    /// The ring around `(x, y)`, `dt` seconds after the press started. It starts at the
    /// bottom of the press, small and transparent.
    pub fn ripple(&self, dt: f32, x: f32, y: f32) -> Svgr<'static> {
        let Some(p) = self.ripple_progress(dt) else {
            return Svgr::empty();
        };
        let r = lerp(4.0, self.radius, quart_out(p));
        let opacity = clamp01(p / 0.15) * (1.0 - p);
        fframes::svgr!(
            <circle cx={x} cy={y} r={r} fill="none" stroke={self.color}
                    stroke-width={self.stroke_width} opacity={opacity} />
        )
    }
}

/// How a [`CursorPath`] or a [`Drag`] travels to the next point. Both arrive exactly on time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorMotion {
    /// Eased in and out, passing the point by `overshoot` (about 0.8 is 3 %) just before it
    /// arrives.
    Smooth { overshoot: f32 },
    /// A damped spring stretched or squeezed from its settle time to the travel time, so it
    /// overshoots when under-damped.
    Spring { stiffness: f32, damping: f32 },
}

impl Default for CursorMotion {
    fn default() -> Self {
        CursorMotion::Spring {
            stiffness: 170.0,
            damping: 20.0,
        }
    }
}

impl CursorMotion {
    /// The share of the way travelled at linear progress `u` of a move: 0 at the start,
    /// exactly 1 at the end.
    pub fn apply(&self, u: f32) -> f32 {
        if u <= 0.0 {
            return 0.0;
        }
        if u >= 1.0 {
            return 1.0;
        }
        match *self {
            CursorMotion::Smooth { overshoot } => {
                let s = RateFunc::Smooth.apply(u, 1.0) - 1.0;
                1.0 + (overshoot + 1.0) * s * s * s + overshoot * s * s
            }
            CursorMotion::Spring { stiffness, damping } => {
                // the spring's own settle time, stretched over the travel
                let w0 = stiffness.max(f32::EPSILON).sqrt();
                let zeta = damping / (2.0 * w0);
                let decay = if zeta < 1.0 {
                    zeta * w0
                } else {
                    w0 * (zeta - (zeta * zeta - 1.0).sqrt())
                };
                // to within 1 %
                let settle = 4.6 / decay.max(f32::EPSILON);
                let end = spring(settle, stiffness, damping);
                spring(u * settle, stiffness, damping) + u * (1.0 - end)
            }
        }
    }
}

/// A point of a [`CursorPath`]: the cursor is at `(x, y)` at `at` seconds, and clicks there
/// when `click` is set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorPoint {
    pub at: f32,
    pub x: f32,
    pub y: f32,
    pub click: bool,
}

impl CursorPoint {
    /// The cursor at `(x, y)` at `at` seconds.
    pub const fn new(at: f32, x: f32, y: f32) -> Self {
        Self {
            at,
            x,
            y,
            click: false,
        }
    }

    /// The same point with a click when the cursor arrives.
    pub const fn click(self) -> Self {
        Self {
            click: true,
            ..self
        }
    }
}

/// A cursor that moves through timed points, in the order of their times. It leaves a point
/// `travel` seconds before the next one is due (or at once when they are closer) and arrives
/// on time, with `motion`. A point with `click` presses the cursor and sends out a ring
/// when it arrives; [`sfx::on_clicks`](super::sfx::on_clicks) gives the sounds. The cursor
/// fades in over `fade` seconds from the first point's time.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct CursorPath {
    pub points: Vec<CursorPoint>,
    pub shape: CursorShape,
    pub style: CursorStyle,
    pub size: f32,
    pub travel: f32,
    pub motion: CursorMotion,
    pub fade: f32,
    pub click: ClickLook,
}

impl Themed for CursorPath {
    fn themed(t: &Theme) -> Self {
        Self {
            points: Vec::new(),
            shape: CursorShape::Arrow,
            style: CursorStyle::Dark,
            size: 40.0,
            travel: 0.6,
            motion: CursorMotion::default(),
            fade: t.motion.duration.fast,
            click: ClickLook::themed(t),
        }
    }
}

impl Default for CursorPath {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl CursorPath {
    /// Where the hotspot is at `t` seconds: the first point before it, the last after.
    pub fn position(&self, t: f32) -> (f32, f32) {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else {
            return (0.0, 0.0);
        };
        if t <= first.at {
            return (first.x, first.y);
        }
        for pair in self.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if t <= b.at {
                let start = (b.at - self.travel).max(a.at);
                let v = self.motion.apply(progress(t, start, b.at));
                return (lerp(a.x, b.x, v), lerp(a.y, b.y, v));
            }
        }
        (last.x, last.y)
    }

    /// The times of the clicks, in seconds.
    pub fn clicks(&self) -> impl Iterator<Item = f32> + '_ {
        self.points.iter().filter(|p| p.click).map(|p| p.at)
    }
}

impl<'a> Component<'a> for CursorPath {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(first) = self.points.first() else {
            return Svgr::empty();
        };
        let t = ui.seconds();
        let opacity = progress(t, first.at, first.at + self.fade);
        if opacity <= 0.0 {
            return Svgr::empty();
        }
        let (x, y) = self.position(t);
        let scale = self.clicks().map(|at| self.click.scale(t - at)).product();
        let rings: Vec<Svgr<'static>> = self
            .points
            .iter()
            .filter(|p| p.click)
            .map(|p| self.click.ripple(t - p.at, p.x, p.y))
            .collect();
        let cursor = Cursor {
            shape: self.shape,
            style: self.style,
            x,
            y,
            size: self.size,
            scale,
            opacity,
            shadow: true,
        }
        .draw();
        fframes::svgr!(<g>{rings}{cursor}</g>)
    }
}

/// A cursor at `(x, y)` that clicks at `at` seconds: it presses, then a ring goes out. It
/// fades in over `fade` seconds from the start of its container.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Click {
    pub x: f32,
    pub y: f32,
    pub at: f32,
    pub shape: CursorShape,
    pub style: CursorStyle,
    pub size: f32,
    pub fade: f32,
    pub look: ClickLook,
}

impl Themed for Click {
    fn themed(t: &Theme) -> Self {
        Self {
            x: 960.0,
            y: 540.0,
            at: 0.5,
            shape: CursorShape::Pointer,
            style: CursorStyle::Dark,
            size: 40.0,
            fade: t.motion.duration.fast,
            look: ClickLook::themed(t),
        }
    }
}

impl Default for Click {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Click {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let dt = t - self.at;
        let ring = self.look.ripple(dt, self.x, self.y);
        let cursor = Cursor {
            shape: self.shape,
            style: self.style,
            x: self.x,
            y: self.y,
            size: self.size,
            scale: self.look.scale(dt),
            opacity: progress(t, 0.0, self.fade),
            shadow: true,
        }
        .draw();
        fframes::svgr!(<g>{ring}{cursor}</g>)
    }
}

/// The grabbing hand picks up its children at `from` at `start` seconds and carries them to
/// `to` in `run_time` seconds. The open hand closes over `swap` seconds before the move and
/// opens again after it; the children move with the cursor. The cursor fades in over `fade`
/// seconds from the start of the container.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Drag<'a> {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub start: f32,
    pub run_time: f32,
    pub motion: CursorMotion,
    pub style: CursorStyle,
    pub size: f32,
    pub swap: f32,
    pub fade: f32,
    pub children: Children<'a>,
}

impl Themed for Drag<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            from: (760.0, 540.0),
            to: (1160.0, 540.0),
            start: 0.6,
            run_time: 0.9,
            motion: CursorMotion::default(),
            style: CursorStyle::Dark,
            size: 40.0,
            swap: 0.12,
            fade: t.motion.duration.fast,
            children: Box::new(|_| Svgr::empty()),
        }
    }
}

impl Default for Drag<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Drag<'_> {
    /// Where the cursor, and the offset of the children from where they were, is at `t`.
    pub fn position(&self, t: f32) -> (f32, f32) {
        let v = self
            .motion
            .apply(progress(t, self.start, self.start + self.run_time));
        (
            lerp(self.from.0, self.to.0, v),
            lerp(self.from.1, self.to.1, v),
        )
    }

    /// How closed the hand is at `t`, from 0 (open) to 1.
    pub fn grip(&self, t: f32) -> f32 {
        let end = self.start + self.run_time;
        progress(t, self.start - self.swap, self.start) * (1.0 - progress(t, end, end + self.swap))
    }
}

impl<'a> Component<'a> for Drag<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let (x, y) = self.position(t);
        let shift = format!("translate({} {})", x - self.from.0, y - self.from.1);
        let content = (self.children)(ui);
        let opacity = progress(t, 0.0, self.fade);
        let grip = self.grip(t);
        let hand = |shape, share: f32| {
            Cursor {
                shape,
                style: self.style,
                x,
                y,
                size: self.size,
                scale: 1.0,
                opacity: opacity * share,
                shadow: true,
            }
            .draw()
        };
        let open = hand(CursorShape::Grab, 1.0 - grip);
        let closed = hand(CursorShape::Grabbing, grip);
        fframes::svgr!(<g><g transform={shift}>{content}</g>{open}{closed}</g>)
    }
}

/// A text cursor that clicks into a field and the [`Typewriter`] that then types there. The
/// cursor sits just left of the text's start, fades in over `fade` seconds from the start of
/// the container and clicks `lead` seconds before the typing starts (the typewriter's
/// `delay`), so the delay should be at least `fade + lead`.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct TypeAt<'a> {
    pub typewriter: Typewriter<'a>,
    pub shape: CursorShape,
    pub style: CursorStyle,
    pub size: f32,
    pub lead: f32,
    pub fade: f32,
    pub click: ClickLook,
}

impl Themed for TypeAt<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            typewriter: Typewriter {
                delay: 0.9,
                ..Typewriter::themed(t)
            },
            shape: CursorShape::IBeam,
            style: CursorStyle::Dark,
            size: 40.0,
            lead: 0.4,
            fade: t.motion.duration.fast,
            click: ClickLook::themed(t),
        }
    }
}

impl Default for TypeAt<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl TypeAt<'_> {
    /// Where the cursor's hotspot is: left of the start of the text, at its middle.
    pub fn cursor_at(&self) -> (f32, f32) {
        let font = self.typewriter.style.size as f32;
        (
            self.typewriter.x - self.size * 0.4,
            self.typewriter.y - font * 0.35,
        )
    }

    /// The second of the click.
    pub fn click_at(&self) -> f32 {
        self.typewriter.delay - self.lead
    }
}

impl<'a> Component<'a> for TypeAt<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let (x, y) = self.cursor_at();
        let dt = t - self.click_at();
        let ring = self.click.ripple(dt, x, y);
        let cursor = Cursor {
            shape: self.shape,
            style: self.style,
            x,
            y,
            size: self.size,
            scale: self.click.scale(dt),
            opacity: progress(t, 0.0, self.fade),
            shadow: true,
        }
        .draw();
        let text = ui.show(self.typewriter);
        fframes::svgr!(<g>{text}{ring}{cursor}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn cursor_path_reaches_each_point_at_its_time() {
        let points = vec![
            CursorPoint::new(0.2, 100.0, 100.0),
            CursorPoint::new(1.0, 900.0, 300.0).click(),
            CursorPoint::new(1.3, 950.0, 320.0),
            CursorPoint::new(2.5, 400.0, 800.0).click(),
        ];
        for motion in [
            CursorMotion::default(),
            CursorMotion::Smooth { overshoot: 0.8 },
        ] {
            let path = CursorPath {
                points: points.clone(),
                motion,
                ..CursorPath::default()
            };
            for p in &points {
                assert!(
                    close(path.position(p.at), (p.x, p.y)),
                    "{motion:?} at {}",
                    p.at
                );
            }
            // waits at a point until it leaves for the next one
            assert!(close(path.position(1.8), (950.0, 320.0)));
            assert!(close(path.position(0.0), (100.0, 100.0)));
            assert!(close(path.position(9.0), (400.0, 800.0)));
            assert_eq!(path.clicks().collect::<Vec<_>>(), vec![1.0, 2.5]);
        }
    }

    #[test]
    fn motions_overshoot_slightly_before_arriving() {
        for motion in [
            CursorMotion::default(),
            CursorMotion::Smooth { overshoot: 0.8 },
        ] {
            let peak = (0..=100)
                .map(|i| motion.apply(i as f32 / 100.0))
                .fold(0.0_f32, f32::max);
            assert!(peak > 1.0 && peak < 1.08, "{motion:?} peaks at {peak}");
            assert_eq!(motion.apply(0.0), 0.0);
            assert!((motion.apply(1.0) - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn a_click_presses_then_ripples() {
        let look = ClickLook::default();
        assert_eq!(look.scale(-0.1), 1.0);
        assert!((look.scale(look.press / 2.0) - look.depth).abs() < 1e-4);
        assert_eq!(look.scale(look.press), 1.0);
        // no ring before the bottom of the press or after it faded
        assert_eq!(look.ripple_progress(look.press / 2.0), None);
        assert!(look.ripple_progress(look.press).is_some());
        assert_eq!(look.ripple_progress(look.press + look.run_time), None);
    }

    #[test]
    fn drag_carries_and_grips() {
        let drag = Drag {
            from: (0.0, 0.0),
            to: (100.0, 50.0),
            start: 1.0,
            run_time: 1.0,
            ..Drag::default()
        };
        assert!(close(drag.position(0.5), (0.0, 0.0)));
        assert!(close(drag.position(2.0), (100.0, 50.0)));
        assert_eq!(drag.grip(0.5), 0.0);
        assert_eq!(drag.grip(1.5), 1.0);
        assert_eq!(drag.grip(2.5), 0.0);
    }
}
