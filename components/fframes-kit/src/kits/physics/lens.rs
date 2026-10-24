use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::Space,
    geometry::Shape,
    layout::Anchor,
    motion::progress,
    shapes::Dot,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{
    label, line,
    optics::{partial, stroked, thin_lens_outline},
};

/// What a [`Lens`] is made of.
///
/// Radii follow the Cartesian convention: a radius is positive when the surface's centre of
/// curvature lies to its right, the side light leaves toward. A biconvex lens has `r1 > 0`
/// and `r2 < 0`; a flat surface has an infinite radius (`f64::INFINITY`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LensShape {
    /// A thin lens of focal length `focal` units: converging when positive.
    Thin { focal: f64 },
    /// Two spherical surfaces `thickness` units apart at the axis, of radii `r1` (left) and
    /// `r2` (right) in units, in glass of refractive `index` in air.
    Thick {
        r1: f64,
        r2: f64,
        index: f64,
        thickness: f64,
    },
}

/// Manim-physics' `Lens`: a lens centred at `x` on the axis of an [`OpticalBench`] (units of
/// `ui.space`), `height` units tall. Rays outside the height stop at the lens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lens {
    pub x: f64,
    pub shape: LensShape,
    pub height: f64,
}

impl Lens {
    /// A thin lens at `x` of focal length `focal`, 3 units tall.
    pub fn thin(x: f64, focal: f64) -> Self {
        Self {
            x,
            shape: LensShape::Thin { focal },
            height: 3.0,
        }
    }

    /// A thick lens at `x` from its radii, index and thickness (see [`LensShape`]), 3 units
    /// tall or less: both surfaces fit, and the glass is still a tenth of `thickness` thick
    /// at the edge.
    pub fn thick(x: f64, r1: f64, r2: f64, index: f64, thickness: f64) -> Self {
        let edge = |h: f64| thickness + sag(r2, h / 2.0) - sag(r1, h / 2.0);
        let mut height = 3.0f64.min(1.9 * r1.abs().min(r2.abs()));
        if edge(height) < 0.1 * thickness {
            // the edge thins as the height grows: bisect for the height that leaves a tenth
            let (mut lo, mut hi) = (0.0f64, height);
            for _ in 0..40 {
                let mid = lo.midpoint(hi);
                if edge(mid) < 0.1 * thickness {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            height = lo;
        }
        Self {
            x,
            shape: LensShape::Thick {
                r1,
                r2,
                index,
                thickness,
            },
            height,
        }
    }

    /// The effective focal length: `focal`, or the lensmaker's equation
    /// `1/f = (n - 1) (1/r1 - 1/r2 + (n - 1) d / (n r1 r2))`. `None` for a lens that does not
    /// focus.
    pub fn focal_length(&self) -> Option<f64> {
        let power = match self.shape {
            LensShape::Thin { focal } => {
                return (focal.is_finite() && focal != 0.0).then_some(focal);
            }
            LensShape::Thick {
                r1,
                r2,
                index: n,
                thickness: d,
            } => (n - 1.0) * (1.0 / r1 - 1.0 / r2 + (n - 1.0) * d / (n * r1 * r2)),
        };
        (power.abs() > 1e-12).then(|| 1.0 / power)
    }

    /// The refracting planes of the lens, left to right: one for a thin lens, two for a thick
    /// one.
    fn surfaces(&self) -> Vec<Surface> {
        match self.shape {
            LensShape::Thin { focal } => vec![Surface {
                x: self.x,
                kick: Kick::Thin { focal },
            }],
            LensShape::Thick {
                r1,
                r2,
                index,
                thickness,
            } => vec![
                Surface {
                    x: self.x - thickness / 2.0,
                    kick: Kick::Sphere {
                        radius: r1,
                        before: 1.0,
                        after: index,
                    },
                },
                Surface {
                    x: self.x + thickness / 2.0,
                    kick: Kick::Sphere {
                        radius: r2,
                        before: index,
                        after: 1.0,
                    },
                },
            ],
        }
    }

    /// The lens turned around: light from the right sees what light from the left saw.
    fn reversed(&self) -> Self {
        let shape = match self.shape {
            LensShape::Thick {
                r1,
                r2,
                index,
                thickness,
            } => LensShape::Thick {
                r1: -r2,
                r2: -r1,
                index,
                thickness,
            },
            thin @ LensShape::Thin { .. } => thin,
        };
        Self {
            x: -self.x,
            shape,
            ..*self
        }
    }

    /// Where light coming in parallel to the axis from the left crosses the axis after the
    /// lens, in units along the axis (the back focal point), or `None` when it leaves
    /// parallel.
    pub fn back_focal_point(&self) -> Option<f64> {
        let h = 1e-3;
        let ray = Ray {
            start: (self.x - 1e3, h),
            direction: (1.0, 0.0),
        };
        let Traced { points, slope, .. } = ray.trace_on_axis(std::slice::from_ref(self), 0.0);
        let &(x, y) = points.last()?;
        (slope.abs() > 1e-15).then(|| x - y / slope)
    }

    /// The same for light coming from the right (the front focal point).
    pub fn front_focal_point(&self) -> Option<f64> {
        self.reversed().back_focal_point().map(|x| -x)
    }
}

/// How a refracting plane changes a paraxial ray.
#[derive(Debug, Clone, Copy)]
enum Kick {
    Thin {
        focal: f64,
    },
    Sphere {
        radius: f64,
        before: f64,
        after: f64,
    },
}

#[derive(Debug, Clone, Copy)]
struct Surface {
    x: f64,
    kick: Kick,
}

impl Kick {
    /// The slope after the plane for a ray at height `y` with slope `u`:
    /// `u - y / f` for a thin lens, `n2 u2 = n1 u1 - (n2 - n1) y / R` for a surface.
    fn apply(self, y: f64, u: f64) -> f64 {
        match self {
            Kick::Thin { focal } => u - y / focal,
            Kick::Sphere {
                radius,
                before,
                after,
            } => (before * u - (after - before) * y / radius) / after,
        }
    }
}

/// Manim-physics' `Ray`: light from `start` (units of `ui.space`) heading along
/// `direction`, traced to the right through the lenses of an [`OpticalBench`] with paraxial
/// optics (small angles, every surface bending the ray where it crosses the axis's
/// perpendicular at the surface's vertex).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub start: (f64, f64),
    pub direction: (f64, f64),
}

impl Ray {
    pub fn new(start: (f64, f64), direction: (f64, f64)) -> Self {
        Self { start, direction }
    }

    /// The ray through `lenses` on an axis at height `axis_y`, as its corners in units: the
    /// start, every surface it crosses, and the point at `until` units along the axis. A ray
    /// that meets a lens outside its height stops there. A ray that does not head right is
    /// one straight segment `until - start.x` long.
    pub fn trace(&self, lenses: &[Lens], axis_y: f64, until: f64) -> Vec<(f64, f64)> {
        let (dx, dy) = self.direction;
        if dx <= 1e-12 {
            let length = (until - self.start.0).abs();
            let norm = dx.hypot(dy).max(1e-12);
            return vec![
                self.start,
                (
                    self.start.0 + dx / norm * length,
                    self.start.1 + dy / norm * length,
                ),
            ];
        }
        let Traced {
            mut points,
            slope,
            blocked,
        } = self.trace_on_axis(lenses, axis_y);
        if let Some(&(x, y)) = points.last()
            && !blocked
            && until > x
        {
            points.push((until, y + slope * (until - x)));
        }
        points
    }

    /// The corners up to the last surface and the slope after it.
    fn trace_on_axis(&self, lenses: &[Lens], axis_y: f64) -> Traced {
        let (dx, dy) = self.direction;
        let mut surfaces: Vec<(Surface, f64)> = lenses
            .iter()
            .flat_map(|l| l.surfaces().into_iter().map(move |s| (s, l.height)))
            .filter(|(s, _)| s.x > self.start.0)
            .collect();
        surfaces.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
        let (mut x, mut y) = (self.start.0, self.start.1 - axis_y);
        let mut u = dy / dx;
        let mut points = vec![self.start];
        for (surface, height) in surfaces {
            y += u * (surface.x - x);
            x = surface.x;
            points.push((x, y + axis_y));
            if y.abs() > height / 2.0 {
                return Traced {
                    points,
                    slope: u,
                    blocked: true,
                };
            }
            u = surface.kick.apply(y, u);
        }
        Traced {
            points,
            slope: u,
            blocked: false,
        }
    }
}

struct Traced {
    points: Vec<(f64, f64)>,
    slope: f64,
    /// It met a lens outside the lens's height.
    blocked: bool,
}

/// A spherical surface's offset along the axis from its vertex at height `h`:
/// `R - sign(R) sqrt(R² - h²)`, 0 for a flat one.
fn sag(r: f64, h: f64) -> f64 {
    if r.is_finite() {
        r - r.signum() * (r * r - h * h).max(0.0).sqrt()
    } else {
        0.0
    }
}

/// The outline of a thick lens with vertices `thickness` apart around `(x, y)`: the two
/// spherical surfaces between `±height / 2`, in pixels.
fn thick_outline(space: Space, lens: &Lens, axis_y: f64) -> BezPath {
    let LensShape::Thick {
        r1, r2, thickness, ..
    } = lens.shape
    else {
        return BezPath::new();
    };
    let half = lens.height / 2.0;

    let (left, right) = (lens.x - thickness / 2.0, lens.x + thickness / 2.0);
    let steps = 24u32;
    let mut path = BezPath::new();
    for i in 0..=steps {
        let h = half - lens.height * f64::from(i) / f64::from(steps);
        let p = space.to_px(left + sag(r1, h), axis_y + h);
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    for i in 0..=steps {
        let h = -half + lens.height * f64::from(i) / f64::from(steps);
        path.line_to(space.to_px(right + sag(r2, h), axis_y + h));
    }
    path.close_path();
    path
}

/// Manim-physics' `Lens` and `Ray` on one axis: `lenses` on a horizontal axis at height
/// `axis_y` from `x_range.0` to `x_range.1` (units of `ui.space`), and `rays` traced through
/// them to `x_range.1`. With `focal_points` each lens's front and back focal points are dots
/// labelled `F` and `F'`, found by tracing a parallel ray through the lens, so for a thick
/// lens they sit at its focal distances from its surfaces, not from its middle. The rays draw
/// on one after another from `delay` seconds after the local start (`ray_time` each,
/// `stagger` apart). The axis and lenses draw with `enter`, the rays and labels with
/// `parts_enter`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct OpticalBench {
    pub lenses: Vec<Lens>,
    pub rays: Vec<Ray>,
    pub axis_y: f64,
    pub x_range: (f64, f64),
    pub focal_points: bool,
    pub labels: bool,
    pub delay: f32,
    pub ray_time: f32,
    pub stagger: f32,
    pub lens_color: Color,
    pub ray_color: Color,
    pub guide_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for OpticalBench {
    fn themed(t: &Theme) -> Self {
        Self {
            lenses: Vec::new(),
            rays: Vec::new(),
            axis_y: 0.0,
            x_range: (-7.0, 7.0),
            focal_points: true,
            labels: true,
            delay: 0.0,
            ray_time: 0.8,
            stagger: 0.25,
            lens_color: t.colors.text,
            ray_color: t.colors.highlight,
            guide_color: t.colors.muted,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for OpticalBench {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for OpticalBench {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let thin = ui.theme.stroke.thin;
        let (x0, x1) = self.x_range;
        let mut axis = line(
            ui,
            space.to_px(x0, self.axis_y),
            space.to_px(x1, self.axis_y),
            self.guide_color,
            thin,
        );
        axis.dash = Some((8.0, 8.0));
        let mut fixed = vec![axis];
        let mut foci = Vec::new();
        for lens in &self.lenses {
            let outline = match lens.shape {
                LensShape::Thin { focal } => {
                    thin_lens_outline(space, (lens.x, self.axis_y), lens.height, focal >= 0.0)
                }
                LensShape::Thick { .. } => thick_outline(space, lens, self.axis_y),
            };
            let outline = stroked(outline, self.lens_color, self.stroke_width);
            fixed.push(Shape {
                fill: Some(self.lens_color),
                stroke: None,
                opacity: 0.12,
                ..outline.clone()
            });
            fixed.push(outline);
            if self.focal_points {
                foci.extend(lens.front_focal_point().map(|x| (x, "F")));
                foci.extend(lens.back_focal_point().map(|x| (x, "F'")));
            }
        }
        for &(x, _) in &foci {
            let at = space.to_px(x, self.axis_y);
            fixed.push(
                Dot {
                    cx: at.x as f32,
                    cy: at.y as f32,
                    radius: 7.0,
                    fill: Some(self.lens_color),
                    ..ui.defaults()
                }
                .shape(),
            );
        }
        let drawn = draw_animated(ui, &fixed, self.enter.as_ref(), self.exit.as_ref(), None);
        let mut texts = Vec::new();
        if self.labels {
            for &(x, name) in &foci {
                let at = space.to_px(x, self.axis_y) + Vec2::new(0.0, 8.0);
                texts.push(label(
                    ui,
                    name.to_owned(),
                    at,
                    Anchor::Bottom,
                    self.lens_color,
                ));
            }
        }
        let now = ui.seconds();
        let mut shapes = Vec::new();
        for (k, ray) in self.rays.iter().enumerate() {
            let start = self.delay + k as f32 * self.stagger;
            let shown = f64::from(progress(now, start, start + self.ray_time.max(1e-3)));
            if shown <= 0.0 {
                continue;
            }
            let points: Vec<Point> = ray
                .trace(&self.lenses, self.axis_y, x1)
                .into_iter()
                .map(|(x, y)| space.to_px(x, y))
                .collect();
            shapes.push(stroked(partial(&points, shown), self.ray_color, thin));
        }
        let rays = draw_animated(ui, &shapes, None, None, None);
        let parts = fade(
            ui,
            fframes::svgr!(<g>{rays}{texts}</g>),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{drawn}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where two traced rays cross, by their last segments.
    fn crossing(a: &[(f64, f64)], b: &[(f64, f64)]) -> (f64, f64) {
        let seg = |p: &[(f64, f64)]| {
            let (p0, p1) = (p[p.len() - 2], p[p.len() - 1]);
            (p0, (p1.1 - p0.1) / (p1.0 - p0.0))
        };
        let ((ax, ay), sa) = seg(a);
        let ((bx, by), sb) = seg(b);
        // ay + sa (x - ax) = by + sb (x - bx)
        let x = (by - ay + sa * ax - sb * bx) / (sa - sb);
        (x, ay + sa * (x - ax))
    }

    #[test]
    fn a_thin_lens_images_at_the_lens_equation() {
        let (f, d_o, h) = (1.5, 4.0, 0.8);
        let lenses = [Lens::thin(1.0, f)];
        let object = (1.0 - d_o, h);
        let a = Ray::new(object, (1.0, 0.0)).trace(&lenses, 0.0, 20.0);
        let b = Ray::new(object, (1.0, -0.3)).trace(&lenses, 0.0, 20.0);
        let (x, y) = crossing(&a, &b);
        let d_i = f * d_o / (d_o - f);
        assert!((x - (1.0 + d_i)).abs() < 1e-9, "{x}");
        assert!((y - (-d_i / d_o * h)).abs() < 1e-9, "{y}");
    }

    #[test]
    fn a_thin_lens_has_its_focal_points_at_f() {
        let lens = Lens::thin(2.0, 1.5);
        assert!((lens.back_focal_point().unwrap() - 3.5).abs() < 1e-9);
        assert!((lens.front_focal_point().unwrap() - 0.5).abs() < 1e-9);
        // a diverging lens has them on the other sides
        let lens = Lens::thin(0.0, -2.0);
        assert!((lens.back_focal_point().unwrap() + 2.0).abs() < 1e-9);
    }

    #[test]
    fn a_biconvex_lens_focuses_at_the_lensmakers_distance() {
        // R1 = 2, R2 = -2, n = 1.5, d = 0.4: 1/f = 0.5 (1 + 0.5/3 * ...)
        let lens = Lens::thick(0.0, 2.0, -2.0, 1.5, 0.4);
        let f = lens.focal_length().unwrap();
        let expected = 1.0 / (0.5 * (0.5 + 0.5 - 0.5 * 0.4 / (1.5 * 4.0)));
        assert!((f - expected).abs() < 1e-12);
        // symmetric, so the focal points sit at the same distances from the middle
        let (front, back) = (
            lens.front_focal_point().unwrap(),
            lens.back_focal_point().unwrap(),
        );
        assert!((front + back).abs() < 1e-9);
        // the back focal distance from the rear vertex is f (1 - (n - 1) d / (n R1))
        let bfd = f * (1.0 - 0.5 * 0.4 / (1.5 * 2.0));
        assert!((back - (0.2 + bfd)).abs() < 1e-9, "{back}");
    }

    #[test]
    fn a_plano_convex_lens_depends_on_its_orientation() {
        let (r, n, d) = (1.5, 1.5, 0.3);
        // curved side first and flat side first have one focal length, R / (n - 1)
        let curved_first = Lens::thick(0.0, r, f64::INFINITY, n, d);
        let flat_first = Lens::thick(0.0, f64::INFINITY, -r, n, d);
        let f = r / (n - 1.0);
        assert!((curved_first.focal_length().unwrap() - f).abs() < 1e-12);
        assert!((flat_first.focal_length().unwrap() - f).abs() < 1e-12);
        // but the back focal point is f - d/n past the rear vertex with the curve first,
        // and f past it with the flat side first
        let rear = d / 2.0;
        assert!((curved_first.back_focal_point().unwrap() - (rear + f - d / n)).abs() < 1e-9);
        assert!((flat_first.back_focal_point().unwrap() - (rear + f)).abs() < 1e-9);
    }

    #[test]
    fn rays_outside_a_lens_stop_on_it() {
        let lenses = [Lens::thin(0.0, 2.0)];
        let blocked = Ray::new((-3.0, 2.0), (1.0, 0.0)).trace(&lenses, 0.0, 5.0);
        assert_eq!(blocked, vec![(-3.0, 2.0), (0.0, 2.0)]);
        let passed = Ray::new((-3.0, 1.0), (1.0, 0.0)).trace(&lenses, 0.0, 5.0);
        assert_eq!(passed.len(), 3);
    }

    #[test]
    fn a_thick_lens_keeps_glass_at_its_edge() {
        let lens = Lens::thick(0.0, 2.4, -2.4, 1.5, 0.5);
        let edge = 0.5 + sag(-2.4, lens.height / 2.0) - sag(2.4, lens.height / 2.0);
        assert!(lens.height < 3.0);
        assert!((edge - 0.05).abs() < 1e-6, "{edge}");
        // a thin enough one keeps the full height
        assert_eq!(Lens::thick(0.0, 20.0, -20.0, 1.5, 0.5).height, 3.0);
    }
}
