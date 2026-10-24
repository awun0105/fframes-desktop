use std::f64::consts::FRAC_PI_2;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::Space,
    geometry::Shape,
    kits::short,
    layout::Anchor,
    motion::progress,
    shapes::{Arc, Dot, Ellipse, Rectangle},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{arrow, label, line};

/// The element of a [`RayDiagram`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Optic {
    /// A thin lens: converging for a positive focal length, diverging for a negative one.
    #[default]
    Lens,
    /// A spherical mirror facing left: concave for a positive focal length, convex for a
    /// negative one.
    Mirror,
}

/// The principal rays of a thin lens or a mirror at `center` (units of `ui.space`) on a
/// horizontal axis, from the tip of an object arrow `object_distance` units to its left and
/// `object_height` tall: parallel to the axis then through the focal point, through the
/// middle, and through the focal point then parallel. They draw on one after another from
/// `delay` seconds after the local start (`ray_time` each, `stagger` apart) and run `reach`
/// units past the optic; then the image forms where they meet, dashed with the rays'
/// backward extensions when it is virtual. Lengths are units of `ui.space` (`focal` is the
/// focal length). The axis, optic, focal points and object draw with `enter`, the rays and
/// the image and the labels with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct RayDiagram {
    pub optic: Optic,
    pub center: (f64, f64),
    pub focal: f64,
    pub object_distance: f64,
    pub object_height: f64,
    pub reach: f64,
    pub optic_height: f64,
    pub delay: f32,
    pub ray_time: f32,
    pub stagger: f32,
    pub labels: bool,
    pub optic_color: Color,
    pub object_color: Color,
    pub image_color: Color,
    pub ray_color: Color,
    pub guide_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for RayDiagram {
    fn themed(t: &Theme) -> Self {
        Self {
            optic: Optic::Lens,
            center: (0.0, 0.0),
            focal: 1.8,
            object_distance: 4.0,
            object_height: 1.2,
            reach: 6.0,
            optic_height: 4.0,
            delay: 0.0,
            ray_time: 0.8,
            stagger: 0.6,
            labels: true,
            optic_color: t.colors.text,
            object_color: t.colors.title,
            image_color: t.colors.highlight,
            ray_color: t.colors.accent,
            guide_color: t.colors.muted,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for RayDiagram {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The part of the polyline `points` that is `shown` (0 to 1) of its length from the start.
pub(super) fn partial(points: &[Point], shown: f64) -> BezPath {
    let mut path = BezPath::new();
    let Some(&first) = points.first() else {
        return path;
    };
    path.move_to(first);
    let total: f64 = points.windows(2).map(|w| (w[1] - w[0]).hypot()).sum();
    let mut left = total * shown.clamp(0.0, 1.0);
    for w in points.windows(2) {
        let length = (w[1] - w[0]).hypot();
        if left >= length {
            path.line_to(w[1]);
            left -= length;
        } else {
            if length > 0.0 {
                path.line_to(w[0] + (w[1] - w[0]) * (left / length));
            }
            break;
        }
    }
    path
}

pub(super) fn stroked(path: BezPath, color: Color, width: f32) -> Shape {
    Shape {
        path,
        fill: None,
        stroke: Some(color),
        stroke_width: width,
        dash: None,
        opacity: 1.0,
    }
}

/// The outline of a thin lens `height` units tall at `center` (units of `ui.space`), in
/// pixels: a narrow ellipse when it converges, a concave slab when it diverges.
pub(super) fn thin_lens_outline(
    space: Space,
    center: (f64, f64),
    height: f64,
    converging: bool,
) -> BezPath {
    let p = |x: f64, y: f64| space.to_px(center.0 + x, center.1 + y);
    if converging {
        let ellipse = Ellipse {
            cx: p(0.0, 0.0).x as f32,
            cy: p(0.0, 0.0).y as f32,
            width: space.px(0.4) as f32,
            height: space.px(height) as f32,
            ..Ellipse::default()
        };
        return crate::geometry::Geometry::path(&ellipse);
    }
    let (half, w) = (height / 2.0, 0.22);
    let mut path = BezPath::new();
    path.move_to(p(-w, half));
    path.line_to(p(w, half));
    path.quad_to(p(w * 0.15, 0.0), p(w, -half));
    path.line_to(p(-w, -half));
    path.quad_to(p(-w * 0.15, 0.0), p(-w, half));
    path.close_path();
    path
}

impl RayDiagram {
    /// The image distance from `1/d_o + 1/d_i = 1/f`: positive for a real image (right of
    /// a lens, left of a mirror), negative for a virtual one, `None` when the object is at
    /// the focal point and the rays leave parallel.
    pub fn image_distance(&self) -> Option<f64> {
        let (f, d) = (self.focal, self.object_distance);
        ((d - f).abs() > 1e-9).then(|| f * d / (d - f))
    }

    /// The image's height over the object's, `-d_i / d_o`; negative when it is upside down.
    pub fn magnification(&self) -> Option<f64> {
        self.image_distance().map(|di| -di / self.object_distance)
    }

    /// The image's tip, in units from the optic, `y` up.
    pub fn image(&self) -> Option<(f64, f64)> {
        let di = self.image_distance()?;
        let m = self.magnification()?;
        let x = match self.optic {
            Optic::Lens => di,
            Optic::Mirror => -di,
        };
        Some((x, m * self.object_height))
    }

    /// Each principal ray as the object's tip, the point where it meets the optic and its
    /// direction after it, in units from the optic.
    fn rays(&self) -> Vec<[(f64, f64); 3]> {
        let (f, d, h) = (self.focal, self.object_distance, self.object_height);
        let tip = (-d, h);
        let mut rays = vec![
            [tip, (0.0, h), (f.abs(), -h * f.signum())],
            [tip, (0.0, 0.0), (d, -h)],
        ];
        if (d - f).abs() > 1e-9 {
            rays.push([tip, (0.0, -h * f / (d - f)), (1.0, 0.0)]);
        }
        if self.optic == Optic::Mirror {
            for ray in &mut rays {
                ray[2].0 = -ray[2].0;
            }
        }
        rays
    }

    fn px(&self, space: Space, (x, y): (f64, f64)) -> Point {
        space.to_px(self.center.0 + x, self.center.1 + y)
    }

    /// The outline of the optic, in pixels.
    fn optic_path(&self, space: Space) -> BezPath {
        let half = self.optic_height / 2.0;
        let mut path = BezPath::new();
        let p = |x, y| self.px(space, (x, y));
        match (self.optic, self.focal >= 0.0) {
            (Optic::Lens, converging) => {
                return thin_lens_outline(space, self.center, self.optic_height, converging);
            }
            (Optic::Mirror, concave) => {
                // the edges bend toward the side the mirror faces when it is concave
                let bend = if concave { -0.25 } else { 0.25 };
                path.move_to(p(bend, half));
                path.quad_to(p(-bend, 0.0), p(bend, -half));
            }
        }
        path
    }
}

impl<'a> Component<'a> for RayDiagram {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let thin = ui.theme.stroke.thin;
        let d = self.object_distance;
        let (left, right) = match self.optic {
            Optic::Lens => (-d - 0.8, self.reach),
            Optic::Mirror => (-(d.max(self.reach)) - 0.5, 1.0),
        };
        let mut axis = line(
            ui,
            self.px(space, (left, 0.0)),
            self.px(space, (right, 0.0)),
            self.guide_color,
            thin,
        );
        axis.dash = Some((8.0, 8.0));
        let optic = stroked(self.optic_path(space), self.optic_color, self.stroke_width);
        let mut fixed = vec![axis];
        if self.optic == Optic::Lens {
            fixed.push(Shape {
                fill: Some(self.optic_color),
                stroke: None,
                opacity: 0.12,
                ..optic.clone()
            });
        }
        fixed.push(optic);
        if self.optic == Optic::Mirror {
            // hatching behind the mirror
            let half = self.optic_height / 2.0;
            for i in 0..9 {
                let y = -half + self.optic_height * (f64::from(i) + 0.5) / 9.0;
                let bend = if self.focal >= 0.0 { -0.25 } else { 0.25 };
                // on the mirror's curve, a parabola through its ends
                let x = bend * (y / half).powi(2);
                let at = self.px(space, (x + 0.05, y));
                fixed.push(line(
                    ui,
                    at,
                    at + Vec2::new(18.0, -14.0),
                    self.guide_color,
                    thin,
                ));
            }
        }
        let foci: Vec<f64> = match self.optic {
            Optic::Lens => vec![-self.focal, self.focal],
            Optic::Mirror => vec![-self.focal],
        };
        for &x in &foci {
            let at = self.px(space, (x, 0.0));
            fixed.push(
                Dot {
                    cx: at.x as f32,
                    cy: at.y as f32,
                    radius: 7.0,
                    fill: Some(self.optic_color),
                    ..ui.defaults()
                }
                .shape(),
            );
        }
        let base = self.px(space, (-d, 0.0));
        let tip = self.px(space, (-d, self.object_height));
        fixed.push(arrow(ui, base, tip, self.object_color, self.stroke_width));
        let drawn = draw_animated(ui, &fixed, self.enter.as_ref(), self.exit.as_ref(), None);
        let mut texts = Vec::new();
        if self.labels {
            for &x in &foci {
                let at = self.px(space, (x, 0.0)) + Vec2::new(0.0, 8.0);
                texts.push(label(
                    ui,
                    "F".to_owned(),
                    at,
                    Anchor::Bottom,
                    self.optic_color,
                ));
            }
        }
        let texts = fade(
            ui,
            texts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        let fixed = fframes::svgr!(<g>{drawn}{texts}</g>);

        let now = ui.seconds();
        let image = self.image();
        let mut shapes = Vec::new();
        let rays = self.rays();
        for (k, [from, hit, (dx, dy)]) in rays.iter().copied().enumerate() {
            let start = self.delay + k as f32 * self.stagger;
            let shown = f64::from(progress(now, start, start + self.ray_time.max(1e-3)));
            if shown <= 0.0 {
                continue;
            }
            // out to `reach` past the optic, or to the top or bottom of the optic's space
            let limit = self.optic_height;
            let along = (self.reach / dx.abs()).min(if dy.abs() > 1e-9 {
                ((limit * dy.signum()) - hit.1) / dy
            } else {
                f64::INFINITY
            });
            let end = (hit.0 + dx * along, hit.1 + dy * along);
            let points = [from, hit, end].map(|p| self.px(space, p));
            shapes.push(stroked(partial(&points, shown), self.ray_color, thin));
            if let (Some(image), true) = (image, shown >= 1.0)
                && self.image_distance().is_some_and(|di| di < 0.0)
            {
                let mut back = line(
                    ui,
                    self.px(space, hit),
                    self.px(space, image),
                    self.ray_color,
                    thin,
                );
                back.dash = Some((6.0, 6.0));
                back.opacity = 0.7;
                shapes.push(back);
            }
        }
        if let Some((x, y)) = image {
            let done = self.delay + (rays.len() as f32 - 1.0) * self.stagger + self.ray_time;
            let formed = progress(now, done, done + 0.4);
            if formed > 0.0 && y.abs() > 1e-6 {
                let mut shape = arrow(
                    ui,
                    self.px(space, (x, 0.0)),
                    self.px(space, (x, y)),
                    self.image_color,
                    self.stroke_width,
                );
                if self.image_distance().is_some_and(|di| di < 0.0) {
                    shape.dash = Some((8.0, 6.0));
                }
                shape.opacity = formed;
                shapes.push(shape);
            }
        }
        let parts = draw_animated(ui, &shapes, None, None, None);
        let parts = fade(
            ui,
            parts,
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{fixed}{parts}</g>)
    }
}

/// A ray meeting the flat boundary between two media at `point` (units of `ui.space`): it
/// comes in from the upper left at `angle` radians from the normal through the upper medium
/// (index `n1`) and bends into the lower one (`n2`) by Snell's law,
/// `n1 sin θ1 = n2 sin θ2`. Past the critical angle it is reflected whole instead. With
/// `reflection` the weak reflected ray is drawn as well. The rays draw on from `delay`
/// seconds after the local start, the incoming one over `ray_time` seconds and then the
/// others; with `labels` the angles and indices are written. Rays are `length` units long.
/// The boundary, the normal and the media draw with `enter`, the rays and labels with
/// `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Refraction {
    pub point: (f64, f64),
    pub n1: f64,
    pub n2: f64,
    #[cfg_attr(feature = "studio", props(min = 0.0, max = 1.55, unit = "rad"))]
    pub angle: f64,
    pub length: f64,
    pub reflection: bool,
    pub labels: bool,
    pub delay: f32,
    pub ray_time: f32,
    pub ray_color: Color,
    pub medium_color: Color,
    pub guide_color: Color,
    pub text_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Refraction {
    fn themed(t: &Theme) -> Self {
        Self {
            point: (0.0, 0.0),
            n1: 1.0,
            n2: 1.5,
            angle: 0.7,
            length: 3.0,
            reflection: true,
            labels: true,
            delay: 0.0,
            ray_time: 0.8,
            ray_color: t.colors.accent,
            medium_color: t.colors.surface,
            guide_color: t.colors.muted,
            text_color: t.colors.text,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Refraction {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Refraction {
    /// The angle of the refracted ray from the normal, or `None` past the critical angle.
    pub fn refraction_angle(&self) -> Option<f64> {
        let s = self.n1 / self.n2.max(1e-9) * self.angle.sin();
        (s.abs() <= 1.0).then(|| s.asin())
    }

    /// The angle of incidence past which the ray is reflected whole; only when `n1 > n2`.
    pub fn critical_angle(&self) -> Option<f64> {
        (self.n1 > self.n2).then(|| (self.n2 / self.n1).asin())
    }
}

impl<'a> Component<'a> for Refraction {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let thin = ui.theme.stroke.thin;
        let o = space.to_px(self.point.0, self.point.1);
        let reach = space.px(self.length);
        let medium = Rectangle {
            cx: o.x as f32,
            cy: (o.y + reach / 2.0) as f32,
            width: (2.2 * reach) as f32,
            height: reach as f32,
            fill: Some(self.medium_color),
            stroke: None,
            ..ui.defaults()
        };
        let boundary = line(
            ui,
            o - Vec2::new(1.1 * reach, 0.0),
            o + Vec2::new(1.1 * reach, 0.0),
            self.guide_color,
            self.stroke_width,
        );
        let mut normal = line(
            ui,
            o - Vec2::new(0.0, reach * 0.9),
            o + Vec2::new(0.0, reach * 0.9),
            self.guide_color,
            thin,
        );
        normal.dash = Some((8.0, 8.0));
        let drawn = draw_animated(
            ui,
            &[medium.shape(), boundary, normal],
            self.enter.as_ref(),
            self.exit.as_ref(),
            None,
        );
        let mut fixed = Vec::new();
        if self.labels {
            let at = o + Vec2::new(1.05 * reach, -12.0);
            let text = format!("n = {}", short(self.n1, 2));
            fixed.push(label(ui, text, at, Anchor::TopLeft, self.text_color));
            let at = o + Vec2::new(1.05 * reach, 12.0);
            let text = format!("n = {}", short(self.n2, 2));
            fixed.push(label(ui, text, at, Anchor::BottomLeft, self.text_color));
        }
        let texts = fade(
            ui,
            fixed.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        let fixed = fframes::svgr!(<g>{drawn}{texts}</g>);

        let now = ui.seconds();
        let (s1, c1) = self.angle.sin_cos();
        let from = o + Vec2::new(-s1, -c1) * reach;
        let incoming = f64::from(progress(
            now,
            self.delay,
            self.delay + self.ray_time.max(1e-3),
        ));
        let after = self.delay + self.ray_time;
        let outgoing = f64::from(progress(now, after, after + self.ray_time.max(1e-3)));
        let mut shapes = Vec::new();
        let mut parts = Vec::new();
        if incoming > 0.0 {
            shapes.push(stroked(
                partial(&[from, o], incoming),
                self.ray_color,
                self.stroke_width,
            ));
        }
        let refracted = self.refraction_angle();
        let reflected = o + Vec2::new(s1, -c1) * reach;
        if outgoing > 0.0 {
            match refracted {
                Some(a2) => {
                    let to = o + Vec2::new(a2.sin(), a2.cos()) * reach;
                    shapes.push(stroked(
                        partial(&[o, to], outgoing),
                        self.ray_color,
                        self.stroke_width,
                    ));
                    if self.reflection {
                        let mut weak =
                            stroked(partial(&[o, reflected], outgoing), self.ray_color, thin);
                        weak.opacity = 0.35;
                        shapes.push(weak);
                    }
                }
                None => {
                    shapes.push(stroked(
                        partial(&[o, reflected], outgoing),
                        self.ray_color,
                        self.stroke_width,
                    ));
                }
            }
            // the angles from the normal: incoming above, refracted below
            let r = 70.0;
            let mut arcs = vec![(FRAC_PI_2, self.angle)];
            if let Some(a2) = refracted {
                arcs.push((-FRAC_PI_2, a2));
            }
            for &(start, sweep) in &arcs {
                let mut arc = Arc {
                    cx: o.x as f32,
                    cy: o.y as f32,
                    radius: r as f32,
                    start_angle: start as f32,
                    angle: sweep as f32,
                    fill: None,
                    stroke: Some(self.guide_color),
                    stroke_width: thin,
                    ..ui.defaults()
                }
                .shape();
                arc.opacity = outgoing as f32;
                shapes.push(arc);
                if self.labels && sweep > 0.05 {
                    let half = start + sweep / 2.0;
                    let at = o + Vec2::new(half.cos(), -half.sin()) * (r + 40.0);
                    let text = format!("{}°", short(sweep.to_degrees(), 0));
                    let text = label(ui, text, at, Anchor::Center, self.text_color);
                    parts.push(fframes::svgr!(<g opacity={outgoing}>{text}</g>));
                }
            }
        }
        parts.insert(0, draw_animated(ui, &shapes, None, None, None));
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{fixed}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_object_at_twice_the_focal_length_images_upside_down_at_the_same_size() {
        let lens = RayDiagram {
            focal: 2.0,
            object_distance: 4.0,
            ..RayDiagram::default()
        };
        assert!((lens.image_distance().unwrap() - 4.0).abs() < 1e-12);
        assert!((lens.magnification().unwrap() + 1.0).abs() < 1e-12);
        // inside the focal length: virtual, upright, larger, on the object's side
        let close = RayDiagram {
            object_distance: 1.0,
            ..lens.clone()
        };
        assert!(close.image_distance().unwrap() < 0.0);
        assert!(close.magnification().unwrap() > 1.0);
        let at_focus = RayDiagram {
            object_distance: 2.0,
            ..lens
        };
        assert_eq!(at_focus.image_distance(), None);
    }

    #[test]
    fn the_principal_rays_meet_at_the_image() {
        for optic in [Optic::Lens, Optic::Mirror] {
            for (focal, d) in [(1.8, 4.0), (1.8, 1.0), (-1.5, 3.0)] {
                let diagram = RayDiagram {
                    optic,
                    focal,
                    object_distance: d,
                    ..RayDiagram::default()
                };
                let (ix, iy) = diagram.image().unwrap();
                for [_, (hx, hy), (dx, dy)] in diagram.rays() {
                    // the image's tip is on the line through the hit point along the ray
                    let cross = (ix - hx) * dy - (iy - hy) * dx;
                    assert!(cross.abs() < 1e-9, "{optic:?} f {focal} d {d}: {cross}");
                }
            }
        }
    }

    #[test]
    fn a_mirror_sends_the_rays_back() {
        let mirror = RayDiagram {
            optic: Optic::Mirror,
            ..RayDiagram::default()
        };
        assert!(mirror.rays().iter().all(|r| r[2].0 < 0.0));
        // a real image of a concave mirror is in front of it
        assert!(mirror.image().unwrap().0 < 0.0);
    }

    #[test]
    fn snells_law_and_total_internal_reflection() {
        let glass = Refraction {
            n1: 1.0,
            n2: 1.5,
            angle: 0.6,
            ..Refraction::default()
        };
        let a2 = glass.refraction_angle().unwrap();
        assert!((glass.n1 * glass.angle.sin() - glass.n2 * a2.sin()).abs() < 1e-12);
        assert_eq!(glass.critical_angle(), None);
        let out = Refraction {
            n1: 1.5,
            n2: 1.0,
            angle: 0.8,
            ..Refraction::default()
        };
        let critical = out.critical_angle().unwrap();
        assert!((critical - (1.0f64 / 1.5).asin()).abs() < 1e-12);
        assert_eq!(out.refraction_angle(), None);
    }

    #[test]
    fn a_partial_line_stops_on_the_way() {
        let points = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ];
        let half = partial(&points, 0.75);
        assert_eq!(half.elements().len(), 3);
        assert!(
            matches!(half.elements()[2], kurbo::PathEl::LineTo(p) if p == Point::new(10.0, 5.0))
        );
        assert_eq!(partial(&points, 0.0).elements().len(), 2);
    }
}
