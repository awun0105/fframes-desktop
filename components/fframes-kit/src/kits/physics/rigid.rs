use std::f64::consts::{PI, TAU};

use fframes::{Color, Svgr};
use kurbo::{Affine, BezPath, Circle as KCircle, Point, Shape as _};
use rapier2d::prelude::{ColliderBuilder, PhysicsWorld, RigidBodyBuilder, Vector};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::draw_animated,
    geometry::Shape,
    sim::{Lerp, Sim},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::clock;

/// Seconds of one physics step; [`Sim::sample`] blends between them.
const STEP: f64 = 1.0 / 60.0;

/// The outline of a [`Body`], in units of `ui.space` around its position.
#[derive(Debug, Clone, PartialEq)]
pub enum BodyShape {
    Circle {
        radius: f64,
    },
    Box {
        width: f64,
        height: f64,
    },
    /// The convex hull of the points.
    Polygon {
        points: Vec<(f64, f64)>,
    },
}

/// A rigid body of a [`RigidMotion`], Manim-physics' bodies in a `Space`: its shape, where it
/// starts (units of `ui.space`, `y` up; `angle` in radians, counterclockwise) and how it
/// moves then. A `fixed` body never moves: floors, walls, ramps. `restitution` is how much
/// of its speed a bounce keeps (0 to 1), `density` gives its mass from its area. `color`
/// fills it instead of the world's colour.
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub shape: BodyShape,
    pub position: (f64, f64),
    pub angle: f64,
    pub velocity: (f64, f64),
    pub angular_velocity: f64,
    pub fixed: bool,
    pub restitution: f64,
    pub friction: f64,
    pub density: f64,
    pub color: Option<Color>,
}

impl Body {
    fn new(shape: BodyShape, x: f64, y: f64) -> Self {
        Self {
            shape,
            position: (x, y),
            angle: 0.0,
            velocity: (0.0, 0.0),
            angular_velocity: 0.0,
            fixed: false,
            restitution: 0.5,
            friction: 0.5,
            density: 1.0,
            color: None,
        }
    }

    pub fn circle(x: f64, y: f64, radius: f64) -> Self {
        Self::new(BodyShape::Circle { radius }, x, y)
    }

    pub fn rect(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self::new(BodyShape::Box { width, height }, x, y)
    }

    /// The convex hull of `points`, which are relative to `(x, y)`.
    pub fn polygon(x: f64, y: f64, points: impl IntoIterator<Item = (f64, f64)>) -> Self {
        Self::new(
            BodyShape::Polygon {
                points: points.into_iter().collect(),
            },
            x,
            y,
        )
    }

    /// A fixed wall from `a` to `b`, `thickness` units thick, the line through its middle.
    pub fn wall(a: (f64, f64), b: (f64, f64), thickness: f64) -> Self {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        Self {
            angle: dy.atan2(dx),
            fixed: true,
            ..Self::rect(
                a.0.midpoint(b.0),
                a.1.midpoint(b.1),
                dx.hypot(dy),
                thickness,
            )
        }
    }

    #[must_use]
    pub fn velocity(self, vx: f64, vy: f64) -> Self {
        Self {
            velocity: (vx, vy),
            ..self
        }
    }

    /// Radians a second, counterclockwise.
    #[must_use]
    pub fn spin(self, angular_velocity: f64) -> Self {
        Self {
            angular_velocity,
            ..self
        }
    }

    #[must_use]
    pub fn angle(self, angle: f64) -> Self {
        Self { angle, ..self }
    }

    #[must_use]
    pub fn fixed(self) -> Self {
        Self {
            fixed: true,
            ..self
        }
    }

    #[must_use]
    pub fn restitution(self, restitution: f64) -> Self {
        Self {
            restitution,
            ..self
        }
    }

    #[must_use]
    pub fn friction(self, friction: f64) -> Self {
        Self { friction, ..self }
    }

    #[must_use]
    pub fn density(self, density: f64) -> Self {
        Self { density, ..self }
    }

    #[must_use]
    pub fn color(self, color: Color) -> Self {
        Self {
            color: Some(color),
            ..self
        }
    }

    /// The hull of a polygon body counterclockwise, empty for the other shapes.
    fn hull(&self) -> Vec<(f64, f64)> {
        match &self.shape {
            BodyShape::Polygon { points } => convex_hull(points),
            _ => Vec::new(),
        }
    }
}

/// The convex hull of `points` counterclockwise (Andrew's monotone chain), without points on
/// its edges.
fn convex_hull(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut sorted: Vec<(f64, f64)> = points
        .iter()
        .copied()
        .filter(|p| p.0.is_finite() && p.1.is_finite())
        .collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    sorted.dedup();
    if sorted.len() < 3 {
        return sorted;
    }
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut hull: Vec<(f64, f64)> = Vec::with_capacity(2 * sorted.len());
    for pass in [true, false] {
        let start = hull.len();
        let order: Vec<(f64, f64)> = if pass {
            sorted.clone()
        } else {
            sorted.iter().rev().copied().collect()
        };
        for p in order {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

/// Where a body is at one step: `angle` counts whole turns instead of wrapping at ±π, so
/// blending two steps never spins the wrong way round.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}

impl Lerp for Pose {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        Self {
            x: self.x.lerp(&other.x, t),
            y: self.y.lerp(&other.y, t),
            angle: self.angle.lerp(&other.angle, t),
        }
    }
}

/// `a` wrapped into `(-π, π]`.
fn wrap(a: f64) -> f64 {
    let w = (a + PI).rem_euclid(TAU) - PI;
    if w <= -PI { w + TAU } else { w }
}

/// The motion of `bodies` under `gravity` (units per second², `y` up), Manim-physics'
/// `Space` and `SpaceScene`: simulated by [rapier](https://rapier.rs) at 60 steps a second
/// for `seconds`, once, in the constructor; keep it in the video and pass it to
/// [`RigidWorld`]. The same bodies give the same motion every time (rapier's
/// `enhanced-determinism`). A polygon with fewer than three corners in its hull collides as
/// a small circle.
#[derive(Debug, Clone, PartialEq)]
pub struct RigidMotion {
    pub bodies: Vec<Body>,
    sim: Sim<Vec<Pose>>,
}

impl RigidMotion {
    pub fn new(bodies: Vec<Body>, gravity: (f64, f64), seconds: f64) -> Self {
        let mut world = PhysicsWorld::new();
        world.gravity = Vector::new(gravity.0 as f32, gravity.1 as f32);
        world.integration_parameters.dt = STEP as f32;
        let handles: Vec<_> = bodies
            .iter()
            .map(|body| {
                let builder = if body.fixed {
                    RigidBodyBuilder::fixed()
                } else {
                    RigidBodyBuilder::dynamic().ccd_enabled(true)
                };
                let rigid = builder
                    .translation(Vector::new(body.position.0 as f32, body.position.1 as f32))
                    .rotation(body.angle as f32)
                    .linvel(Vector::new(body.velocity.0 as f32, body.velocity.1 as f32))
                    .angvel(body.angular_velocity as f32);
                let collider = match &body.shape {
                    BodyShape::Circle { radius } => ColliderBuilder::ball(radius.max(1e-3) as f32),
                    BodyShape::Box { width, height } => ColliderBuilder::cuboid(
                        (width.max(2e-3) / 2.0) as f32,
                        (height.max(2e-3) / 2.0) as f32,
                    ),
                    BodyShape::Polygon { .. } => {
                        let hull: Vec<Vector> = body
                            .hull()
                            .iter()
                            .map(|&(x, y)| Vector::new(x as f32, y as f32))
                            .collect();
                        (hull.len() >= 3)
                            .then(|| ColliderBuilder::convex_hull(&hull))
                            .flatten()
                            .unwrap_or_else(|| ColliderBuilder::ball(0.05))
                    }
                }
                .restitution(body.restitution as f32)
                .friction(body.friction as f32)
                .density(body.density.max(1e-6) as f32);
                world.insert(rigid, collider).0
            })
            .collect();
        let initial: Vec<Pose> = bodies
            .iter()
            .map(|b| Pose {
                x: b.position.0,
                y: b.position.1,
                angle: b.angle,
            })
            .collect();
        let steps = (seconds.max(0.0) / STEP).ceil() as usize;
        let sim = Sim::precompute(STEP, steps, initial, |previous, _| {
            world.step();
            handles
                .iter()
                .zip(previous)
                .map(|(&h, before)| {
                    let body = &world.bodies[h];
                    let at = body.translation();
                    let turned = f64::from(body.position().rotation.angle());
                    Pose {
                        x: f64::from(at.x),
                        y: f64::from(at.y),
                        angle: before.angle + wrap(turned - before.angle),
                    }
                })
                .collect()
        });
        Self { bodies, sim }
    }

    /// Where every body is after `t` seconds, in the order of `bodies`.
    pub fn poses(&self, t: f64) -> Vec<Pose> {
        self.sim.sample(t)
    }

    /// Seconds simulated.
    pub fn duration(&self) -> f64 {
        self.sim.duration()
    }
}

/// A body where it is in a frame, for the `draw` closure of [`RigidWorld`]: its index in
/// `bodies`, its center in pixels, its rotation in degrees for SVG's `rotate()` (clockwise on
/// screen) and the pixels of one unit of `ui.space`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyView {
    pub index: usize,
    pub center: Point,
    pub rotation: f64,
    pub unit: f64,
}

/// Draws one body of a [`RigidWorld`] its own way, or leaves it to the default with `None`.
pub type DrawBody<'a> = &'a dyn Fn(&BodyView) -> Option<Svgr<'a>>;

/// The bodies of `motion` where they are `delay` seconds after the local start (times
/// `time_scale`), Manim-physics' `SpaceScene`. Each body is its own shape, filled with
/// `fill_color` (its own `color` if it has one, `fixed_color` when fixed) and outlined, a
/// circle with a radius that shows it turning. `draw` replaces that for the bodies it returns
/// something for. Everything fades with `enter` and `exit`. Without `motion` nothing is
/// drawn.
#[derive(Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct RigidWorld<'a> {
    pub motion: Option<&'a RigidMotion>,
    pub delay: f32,
    pub time_scale: f32,
    pub draw: Option<DrawBody<'a>>,
    pub fill_color: Color,
    pub fixed_color: Color,
    pub stroke_color: Color,
    pub stroke_width: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl std::fmt::Debug for RigidWorld<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RigidWorld")
            .field("motion", &self.motion)
            .field("delay", &self.delay)
            .field("time_scale", &self.time_scale)
            .field("fill_color", &self.fill_color)
            .field("fixed_color", &self.fixed_color)
            .finish_non_exhaustive()
    }
}

impl Themed for RigidWorld<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            motion: None,
            delay: 0.0,
            time_scale: 1.0,
            draw: None,
            fill_color: t.colors.accent,
            fixed_color: t.colors.muted,
            stroke_color: t.colors.text,
            stroke_width: t.stroke.thin,
            enter: None,
            exit: None,
        }
    }
}

impl Default for RigidWorld<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The outline of `body` in its own units, and for a circle the radius that turns with it.
fn outline(body: &Body) -> (BezPath, Option<BezPath>) {
    match &body.shape {
        BodyShape::Circle { radius } => {
            let mut spoke = BezPath::new();
            spoke.move_to((0.0, 0.0));
            spoke.line_to((*radius, 0.0));
            (
                KCircle::new((0.0, 0.0), *radius).to_path(0.001),
                Some(spoke),
            )
        }
        BodyShape::Box { width, height } => (
            kurbo::Rect::new(-width / 2.0, -height / 2.0, width / 2.0, height / 2.0).to_path(0.1),
            None,
        ),
        BodyShape::Polygon { .. } => {
            let mut path = BezPath::new();
            for (i, p) in body.hull().into_iter().enumerate() {
                if i == 0 {
                    path.move_to(p);
                } else {
                    path.line_to(p);
                }
            }
            path.close_path();
            (path, None)
        }
    }
}

impl<'a> Component<'a> for RigidWorld<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(motion) = self.motion else {
            return Svgr::empty();
        };
        let space = ui.space;
        let unit = f64::from(space.unit);
        let poses = motion.poses(clock(ui, self.delay, self.time_scale));
        let mut drawn = Vec::new();
        let mut shapes = Vec::new();
        for (index, (body, pose)) in motion.bodies.iter().zip(&poses).enumerate() {
            let center = space.to_px(pose.x, pose.y);
            let view = BodyView {
                index,
                center,
                rotation: -pose.angle.to_degrees(),
                unit,
            };
            if let Some(own) = self.draw.and_then(|draw| draw(&view)) {
                drawn.push(own);
                continue;
            }
            let place = Affine::translate(center.to_vec2())
                * Affine::scale_non_uniform(unit, -unit)
                * Affine::rotate(pose.angle);
            let fill = if body.fixed {
                self.fixed_color
            } else {
                body.color.unwrap_or(self.fill_color)
            };
            let (path, spoke) = outline(body);
            shapes.push(Shape {
                path: place * path,
                fill: Some(fill),
                stroke: Some(self.stroke_color),
                stroke_width: self.stroke_width,
                dash: None,
                opacity: 1.0,
            });
            if let Some(spoke) = spoke {
                shapes.push(Shape {
                    path: place * spoke,
                    fill: None,
                    stroke: Some(self.stroke_color),
                    stroke_width: self.stroke_width,
                    dash: None,
                    opacity: 1.0,
                });
            }
        }
        let bodies = draw_animated(ui, &shapes, None, None, None);
        fade(
            ui,
            fframes::svgr!(<g>{bodies}{drawn}</g>),
            self.enter.as_ref(),
            self.exit.as_ref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pile() -> Vec<Body> {
        vec![
            Body::wall((-5.0, -3.0), (5.0, -3.0), 0.4),
            Body::wall((-5.0, -3.0), (-5.0, 3.0), 0.4),
            Body::wall((5.0, -3.0), (5.0, 3.0), 0.4),
            Body::circle(-1.0, 2.0, 0.5).velocity(2.0, 0.0),
            Body::rect(0.5, 1.0, 1.0, 0.6).angle(0.3),
            Body::polygon(1.5, 2.5, [(0.0, 0.6), (-0.5, -0.4), (0.5, -0.4)]).spin(3.0),
        ]
    }

    #[test]
    fn a_run_is_the_same_twice() {
        let a = RigidMotion::new(pile(), (0.0, -9.81), 3.0);
        let b = RigidMotion::new(pile(), (0.0, -9.81), 3.0);
        assert_eq!(a, b);
        // and something moved
        assert_ne!(a.poses(0.0), a.poses(3.0));
    }

    #[test]
    fn a_free_body_falls_with_gravity() {
        let motion = RigidMotion::new(vec![Body::circle(0.0, 4.0, 0.2)], (0.0, -9.81), 1.0);
        let fallen = 4.0 - motion.poses(1.0)[0].y;
        // 4.905 m exactly, the step integrator adds g dt² n / 2
        assert!((fallen - 0.5 * 9.81).abs() < 0.1, "{fallen}");
    }

    #[test]
    fn bodies_stay_inside_the_walls() {
        let motion = RigidMotion::new(pile(), (0.0, -9.81), 4.0);
        for i in 0..=40 {
            for pose in &motion.poses(f64::from(i) * 0.1)[3..] {
                // the walls are 0.2 thick on each side of their line
                assert!(pose.y > -2.8 && pose.x > -4.8 && pose.x < 4.8, "{pose:?}");
            }
        }
    }

    #[test]
    fn a_spinning_body_turns_on_without_jumps() {
        let motion = RigidMotion::new(
            vec![Body::rect(0.0, 0.0, 1.0, 0.2).spin(20.0)],
            (0.0, 0.0),
            2.0,
        );
        let mut before = motion.poses(0.0)[0].angle;
        for i in 1..=400 {
            let now = motion.poses(f64::from(i) * 0.005)[0].angle;
            assert!(
                (now - before - 20.0 * 0.005).abs() < 1e-3,
                "at {i}: {before} {now}"
            );
            before = now;
        }
        // six turns and more, counted on; rapier's rotation step turns 0.2% short at a
        // third of a radian per step
        let end = motion.poses(2.0)[0].angle;
        assert!((end - 40.0).abs() < 0.2, "{end}");
    }

    #[test]
    fn a_flat_polygon_does_not_break_the_run() {
        let motion = RigidMotion::new(
            vec![Body::polygon(
                0.0,
                0.0,
                [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)],
            )],
            (0.0, -9.81),
            0.5,
        );
        assert!(motion.poses(0.5)[0].y.is_finite());
        assert_eq!(convex_hull(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]).len(), 2);
        let square = convex_hull(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.5, 0.5)]);
        assert_eq!(square.len(), 4);
    }

    #[test]
    fn angles_wrap_into_a_half_turn() {
        assert!((wrap(3.0 * PI / 2.0) + PI / 2.0).abs() < 1e-12);
        assert_eq!(wrap(PI), PI);
        assert!((wrap(-PI) - PI).abs() < 1e-12);
    }
}
