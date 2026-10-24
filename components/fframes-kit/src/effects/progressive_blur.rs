use fframes::Svgr;

use super::{Region, filtered, patterns::masked};
use crate::{
    anim::Anim,
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// The most blurred copies a progressive blur draws.
pub const MAX_BLUR_LEVELS: usize = 8;

/// How much the blur radius may grow from one copy of a progressive blur to the next.
const LEVEL_STEP: f32 = 6.0;

/// The children blurred like Remotion's radius `radius`: a Gaussian of a third of it.
fn blurred(content: Svgr<'_>, id: String, radius: f32, (w, h): (f32, f32)) -> Svgr<'_> {
    if radius <= 0.0 {
        return content;
    }
    let primitives = fframes::svgr!(<feGaussianBlur stdDeviation={radius / 3.0} />);
    filtered(content, id, Region::Area(w, h), primitives)
}

/// The blur radii of the copies of a progressive blur from `start` to `end`: the first
/// is drawn whole, every later one over it through a ramp. At most [`MAX_BLUR_LEVELS`]
/// copies after the first, and only the first when the blur stays the same.
pub fn blur_levels(start: f32, end: f32) -> Vec<f32> {
    let (start, end) = (start.max(0.0), end.max(0.0));
    if start == end {
        return vec![start];
    }
    let steps = ((end - start).abs() / LEVEL_STEP).ceil() as usize;
    let steps = steps.clamp(1, MAX_BLUR_LEVELS);
    (0..=steps)
        .map(|k| start + (end - start) * k as f32 / steps as f32)
        .collect()
}

/// The copies of `children` at the radii of [`blur_levels`], each after the first shown
/// through the ramp `ramp(id, from, to)` returns: transparent before `from`, opaque from `to`
/// on (both 0 to 1 along the blur).
fn progressive<'a>(
    ui: &mut Ui<'a, '_, '_>,
    children: &Children<'a>,
    id: &str,
    (start, end): (f32, f32),
    area: (f32, f32),
    ramp: impl Fn(String, f32, f32) -> Svgr<'a>,
) -> Svgr<'a> {
    let levels = blur_levels(start, end);
    let last = (levels.len() - 1) as f32;
    let mut layers = Vec::with_capacity(levels.len());
    for (k, &radius) in levels.iter().enumerate() {
        let copy = blurred(children(ui), format!("{id}-blur-{k}"), radius, area);
        if k == 0 {
            layers.push(copy);
            continue;
        }
        let (from, to) = ((k as f32 - 1.0) / last, k as f32 / last);
        let mask = ramp(format!("{id}-ramp-{k}"), from, to);
        layers.push(masked(copy, mask, &format!("{id}-mask-{k}"), area));
    }
    Svgr::from(layers)
}

/// A white rectangle over the area filled with the gradient `id`, defined by `gradient`.
fn ramp_rect<'a>(id: &str, gradient: Svgr<'a>, (w, h): (f32, f32)) -> Svgr<'a> {
    let fill = format!("url(#{id})");
    fframes::svgr!(
        <g>
            <defs>{gradient}</defs>
            <rect x="0" y="0" width={w} height={h} fill={fill} />
        </g>
    )
}

/// Its children blurred inside a rectangle (Remotion's `regionBlur()`): from `top_left` to
/// `bottom_right` (0 to 1 across the area), blurred by `radius` like Remotion's (a Gaussian
/// of a third of it), the edge softened over `feather` units and the corners rounded by
/// `roundness` (0 square to 1 fully round). The blurred copy is laid over the sharp
/// children, so the children render twice.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct RegionBlur<'a> {
    pub top_left: (f32, f32),
    pub bottom_right: (f32, f32),
    pub radius: Anim<'a, f32>,
    pub feather: f32,
    pub roundness: f32,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for RegionBlur<'_> {
    /// Remotion's defaults: a radius of 40, sharp square edges, over the middle half.
    fn themed(_: &Theme) -> Self {
        Self {
            top_left: (0.25, 0.25),
            bottom_right: (0.75, 0.75),
            radius: Anim::Fixed(40.0),
            feather: 0.0,
            roundness: 0.0,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for RegionBlur<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let radius = ui.sample(&self.radius);
        let sharp = (self.children)(ui);
        if radius <= 0.0 {
            return sharp;
        }
        let id = ui.id(self.key, 0, "region-blur");
        let (w, h) = self.area;
        let (x0, y0) = (self.top_left.0 * w, self.top_left.1 * h);
        let (x1, y1) = (self.bottom_right.0 * w, self.bottom_right.1 * h);
        let (x, y) = (x0.min(x1), y0.min(y1));
        let (rw, rh) = ((x1 - x0).abs(), (y1 - y0).abs());
        let corner = rw.min(rh) / 2.0 * self.roundness.clamp(0.0, 1.0);
        let blur = blurred((self.children)(ui), format!("{id}-blur"), radius, self.area);
        let region = fframes::svgr!(
            <rect x={x} y={y} width={rw} height={rh} rx={corner} ry={corner} fill="#ffffff" />
        );
        // a smoothstep over `feather` is close to a Gaussian of a quarter of it
        let region = if self.feather > 0.0 {
            let soft = fframes::svgr!(<feGaussianBlur stdDeviation={self.feather / 4.0} />);
            filtered(region, format!("{id}-feather"), Region::Area(w, h), soft)
        } else {
            region
        };
        let blur = masked(blur, region, &format!("{id}-mask"), self.area);
        fframes::svgr!(<g>{sharp}{blur}</g>)
    }
}

/// Its children blurred more and more along a line (Remotion's `linearProgressiveBlur()`):
/// `start_blur` at `start`, growing to `end_blur` at `end` (both 0 to 1 across the area),
/// radii like Remotion's (a Gaussian of a third of them). Drawn as blurred copies under
/// graded masks, one for every 6 units of radius and at most [`MAX_BLUR_LEVELS`], so the
/// children render up to nine times.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LinearProgressiveBlur<'a> {
    pub start: (f32, f32),
    pub end: (f32, f32),
    pub start_blur: f32,
    pub end_blur: f32,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for LinearProgressiveBlur<'_> {
    /// Remotion's defaults: sharp on the left, a radius of 50 on the right.
    fn themed(_: &Theme) -> Self {
        Self {
            start: (0.0, 0.5),
            end: (1.0, 0.5),
            start_blur: 0.0,
            end_blur: 50.0,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for LinearProgressiveBlur<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let id = ui.id(self.key, 0, "linear-progressive-blur");
        let area = self.area;
        let (w, h) = area;
        let (x1, y1) = (self.start.0 * w, self.start.1 * h);
        let (x2, y2) = (self.end.0 * w, self.end.1 * h);
        let ramp = |name: String, from: f32, to: f32| {
            let gradient = fframes::svgr!(
                <linearGradient id={name.clone()} gradientUnits="userSpaceOnUse"
                                x1={x1} y1={y1} x2={x2} y2={y2}>
                    <stop offset={from} stop-color="#000000" />
                    <stop offset={to} stop-color="#ffffff" />
                </linearGradient>
            );
            ramp_rect(&name, gradient, area)
        };
        progressive(
            ui,
            &self.children,
            &id,
            (self.start_blur, self.end_blur),
            area,
            ramp,
        )
    }
}

/// Its children blurred more and more away from a point (Remotion's
/// `radialProgressiveBlur()`): `start_blur` inside `start` (0 the centre to 1 the ellipse),
/// growing to `end_blur` on and outside an ellipse round `center`, `width` by `height`
/// (shares of the area) and turned `rotation` degrees. Radii are like Remotion's (a Gaussian
/// of a third of them). Drawn as blurred copies under graded masks, one for every 6 units of
/// radius and at most [`MAX_BLUR_LEVELS`], so the children render up to nine times.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct RadialProgressiveBlur<'a> {
    pub center: (f32, f32),
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub start: f32,
    pub start_blur: f32,
    pub end_blur: f32,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for RadialProgressiveBlur<'_> {
    /// Remotion's defaults: sharp in the middle, a radius of 50 at the ellipse that touches
    /// the edges of the area.
    fn themed(_: &Theme) -> Self {
        Self {
            center: (0.5, 0.5),
            width: 1.0,
            height: 1.0,
            rotation: 0.0,
            start: 0.0,
            start_blur: 0.0,
            end_blur: 50.0,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl<'a> Component<'a> for RadialProgressiveBlur<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let id = ui.id(self.key, 0, "radial-progressive-blur");
        let area = self.area;
        let (w, h) = area;
        let (cx, cy) = (self.center.0 * w, self.center.1 * h);
        let (rx, ry) = (
            (self.width * w / 2.0).max(1e-3),
            (self.height * h / 2.0).max(1e-3),
        );
        let place = format!(
            "translate({cx} {cy}) rotate({}) scale({rx} {ry})",
            self.rotation
        );
        let start = self.start.clamp(0.0, 1.0);
        let ramp = |name: String, from: f32, to: f32| {
            let along = |t: f32| start + (1.0 - start) * t;
            let gradient = fframes::svgr!(
                <radialGradient id={name.clone()} gradientUnits="userSpaceOnUse"
                                cx="0" cy="0" r="1" gradientTransform={place.clone()}>
                    <stop offset={along(from)} stop-color="#000000" />
                    <stop offset={along(to)} stop-color="#ffffff" />
                </radialGradient>
            );
            ramp_rect(&name, gradient, area)
        };
        progressive(
            ui,
            &self.children,
            &id,
            (self.start_blur, self.end_blur),
            area,
            ramp,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_step_by_at_most_six_and_stay_below_the_cap() {
        assert_eq!(blur_levels(0.0, 12.0), [0.0, 6.0, 12.0]);
        assert_eq!(blur_levels(0.0, 50.0).len(), MAX_BLUR_LEVELS + 1);
        assert_eq!(blur_levels(0.0, 500.0).len(), MAX_BLUR_LEVELS + 1);
        // a blur that falls off, and one that stays even
        assert_eq!(blur_levels(12.0, 0.0), [12.0, 6.0, 0.0]);
        assert_eq!(blur_levels(10.0, 10.0), [10.0]);
    }
}
