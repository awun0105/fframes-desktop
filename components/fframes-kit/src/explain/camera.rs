//! Manim's 2D cameras: `MovingCameraScene` and `ZoomedScene`, as group transforms around the
//! children.

use fframes::{Color, Svgr};
use kurbo::{Affine, Rect};

use crate::{
    anim::{Anim, RateFunc},
    animate::enter_alpha,
    explain::Space,
    theme::Theme,
    ui::{Children, Component, Themed, Ui},
};

/// The width in units of the frame that shows the whole canvas of `space`, whose origin is
/// the middle of the canvas.
pub fn full_frame_width(space: Space) -> f32 {
    if space.unit > 0.0 {
        2.0 * space.origin.0 / space.unit
    } else {
        0.0
    }
}

/// Manim's `camera.auto_zoom`: the frame center and width in units that shows every box
/// (pixels) with `margin` units around them, keeping the canvas's aspect ratio. `None`
/// without boxes.
pub fn auto_zoom(space: Space, boxes: &[Rect], margin: f32) -> Option<((f32, f32), f32)> {
    let all = boxes.iter().copied().reduce(|a, b| a.union(b))?;
    let (x0, y0) = space.from_px(all.origin());
    let (x1, y1) = space.from_px(kurbo::Point::new(all.x1, all.y1));
    let center = (x0.midpoint(x1) as f32, y0.midpoint(y1) as f32);
    let (w, h) = ((x1 - x0).abs() as f32, (y1 - y0).abs() as f32);
    let aspect = space.origin.0 / space.origin.1.max(1e-3);
    let width = w.max(h * aspect) + 2.0 * margin;
    Some((center, width))
}

/// The canvas seen through a camera frame (Manim's `MovingCameraScene`): `center` and
/// `width` in units of `ui.space` say which part of the canvas fills the picture, `width`
/// of [`full_frame_width`] showing it all. Both follow an [`Anim`], so the camera can pan and
/// zoom; [`auto_zoom`] gives the frame around some boxes. The children draw in canvas
/// pixels as always.
pub struct MovingCamera<'a> {
    pub center: Anim<'a, (f32, f32)>,
    pub width: Anim<'a, f32>,
    pub children: Children<'a>,
}

impl std::fmt::Debug for MovingCamera<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MovingCamera")
            .field("center", &self.center)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}

impl MovingCamera<'_> {
    /// The map from canvas pixels to the picture for a frame at `center` `width` units wide.
    pub fn view(space: Space, center: (f32, f32), width: f32) -> Affine {
        let full = f64::from(full_frame_width(space));
        let scale = if width > 0.0 {
            full / f64::from(width)
        } else {
            1.0
        };
        let at = space.to_px(f64::from(center.0), f64::from(center.1));
        let middle = space.to_px(0.0, 0.0);
        Affine::translate(middle.to_vec2())
            * Affine::scale(scale)
            * Affine::translate(-at.to_vec2())
    }
}

impl<'a> Component<'a> for MovingCamera<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let view = Self::view(ui.space, ui.sample(&self.center), ui.sample(&self.width));
        let transform = svg_matrix(view);
        let children = (self.children)(ui);
        fframes::svgr!(<g transform={transform}>{children}</g>)
    }
}

/// An affine as an SVG `matrix(..)`.
fn svg_matrix(a: Affine) -> String {
    let [a, b, c, d, e, f] = a.as_coeffs();
    format!("matrix({a} {b} {c} {d} {e} {f})")
}

/// Manim's `ZoomedScene`: the children, and a second, magnified copy of what is inside
/// `frame` drawn in `display` (both pixel boxes), with the frame outlined and the display
/// bordered. The display pops out of the frame over `run_time` after `delay` (Manim's
/// `get_zoomed_display_pop_out_animation`); a `run_time` of 0 shows it at once. The children
/// render twice.
pub struct ZoomedInset<'a> {
    pub key: Option<&'a str>,
    /// The part of the canvas that is magnified.
    pub frame: Rect,
    /// Where the magnified copy is drawn.
    pub display: Rect,
    pub frame_color: Color,
    pub border_color: Color,
    pub stroke_width: f32,
    /// Fills the display under the copy.
    pub background: Color,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
    pub children: Children<'a>,
}

impl std::fmt::Debug for ZoomedInset<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZoomedInset")
            .field("frame", &self.frame)
            .field("display", &self.display)
            .finish_non_exhaustive()
    }
}

impl Themed for ZoomedInset<'_> {
    /// Manim's defaults on a 1920x1080 canvas: a 3 by 3 unit display half a unit from the top
    /// right corner and a frame 0.15 times its size in the middle; no children yet.
    fn themed(t: &Theme) -> Self {
        Self {
            key: None,
            frame: Rect::new(929.625, 509.625, 990.375, 570.375),
            display: Rect::new(1447.5, 67.5, 1852.5, 472.5),
            frame_color: t.colors.highlight,
            border_color: t.colors.highlight,
            stroke_width: t.stroke.regular,
            background: t.colors.background,
            delay: 0.0,
            run_time: 0.0,
            rate_func: RateFunc::Smooth,
            children: Box::new(|_| Svgr::empty()),
        }
    }
}

/// The box `t` of the way from `a` to `b`.
fn lerp_rect(a: Rect, b: Rect, t: f64) -> Rect {
    let l = |x: f64, y: f64| x + (y - x) * t;
    Rect::new(l(a.x0, b.x0), l(a.y0, b.y0), l(a.x1, b.x1), l(a.y1, b.y1))
}

impl ZoomedInset<'_> {
    /// Where the display is at progress `p` of the pop-out.
    pub fn display_at(&self, p: f32) -> Rect {
        lerp_rect(self.frame, self.display, f64::from(p))
    }

    /// The map from canvas pixels to the display at progress `p`: `frame` onto the display.
    pub fn zoom(&self, p: f32) -> Affine {
        let (f, d) = (self.frame, self.display_at(p));
        let sx = d.width() / f.width().max(1e-6);
        let sy = d.height() / f.height().max(1e-6);
        Affine::translate(d.origin().to_vec2())
            * Affine::scale_non_uniform(sx, sy)
            * Affine::translate(-f.origin().to_vec2())
    }
}

impl<'a> Component<'a> for ZoomedInset<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = if self.run_time <= 0.0 {
            if ui.seconds() >= self.delay { 1.0 } else { 0.0 }
        } else {
            enter_alpha(ui, self.delay, self.run_time, self.rate_func)
        };
        let scene = (self.children)(ui);
        let f = self.frame;
        let w = self.stroke_width;
        let outline = fframes::svgr!(
            <rect x={f.x0 as f32} y={f.y0 as f32} width={f.width() as f32} height={f.height() as f32}
                  fill="none" stroke={self.frame_color} stroke-width={w} />
        );
        if p <= 0.0 {
            return fframes::svgr!(<g>{scene}{outline}</g>);
        }
        let d = self.display_at(p);
        let (x, y, dw, dh) = (
            d.x0 as f32,
            d.y0 as f32,
            d.width() as f32,
            d.height() as f32,
        );
        let id = ui.id(self.key, 0, "zoomed-inset");
        let clip = format!("url(#{id})");
        let zoom = svg_matrix(self.zoom(p));
        let copy = (self.children)(ui);
        fframes::svgr!(
            <g>
                {scene}
                {outline}
                <defs>
                    <clipPath id={id} clipPathUnits="userSpaceOnUse">
                        <rect x={x} y={y} width={dw} height={dh} />
                    </clipPath>
                </defs>
                <rect x={x} y={y} width={dw} height={dh} fill={self.background} />
                <g clip-path={clip}>
                    <g transform={zoom}>{copy}</g>
                </g>
                <rect x={x} y={y} width={dw} height={dh} fill="none"
                      stroke={self.border_color} stroke-width={w} />
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_full_frame_is_the_identity() {
        let space = Space::FULL_HD;
        let width = full_frame_width(space);
        assert!((width - 1920.0 / 135.0).abs() < 1e-4);
        let v = MovingCamera::view(space, (0.0, 0.0), width);
        let p = v * kurbo::Point::new(100.0, 900.0);
        assert!((p - kurbo::Point::new(100.0, 900.0)).hypot() < 1e-6);
        // half the width: the point (1, 1) unit lands twice as far from the middle
        let zoomed = MovingCamera::view(space, (0.0, 0.0), width / 2.0);
        let q = zoomed * space.to_px(1.0, 1.0);
        assert!((q - kurbo::Point::new(960.0 + 270.0, 540.0 - 270.0)).hypot() < 1e-6);
    }

    #[test]
    fn auto_zoom_fits_wide_and_tall_boxes() {
        let space = Space::FULL_HD;
        let wide = Rect::new(960.0, 540.0, 1230.0, 567.0);
        let ((cx, cy), w) = auto_zoom(space, &[wide], 0.0).unwrap();
        assert!((cx - 1.0).abs() < 1e-5 && (cy + 0.1).abs() < 1e-5 && (w - 2.0).abs() < 1e-5);
        // a tall box is fitted by its height times the aspect ratio
        let tall = Rect::new(960.0, 405.0, 987.0, 675.0);
        let (_, w) = auto_zoom(space, &[tall], 0.5).unwrap();
        assert!((w - (2.0 * 16.0 / 9.0 + 1.0)).abs() < 1e-4, "{w}");
        assert!(auto_zoom(space, &[], 0.0).is_none());
    }

    #[test]
    fn the_inset_maps_the_frame_onto_the_display() {
        let inset = ZoomedInset::themed(&Theme::default());
        let z = inset.zoom(1.0);
        let corner = z * inset.frame.origin();
        assert!((corner - inset.display.origin()).hypot() < 1e-9);
        let far = z * kurbo::Point::new(inset.frame.x1, inset.frame.y1);
        assert!((far - kurbo::Point::new(inset.display.x1, inset.display.y1)).hypot() < 1e-9);
        assert_eq!(inset.display_at(0.0), inset.frame);
    }
}
