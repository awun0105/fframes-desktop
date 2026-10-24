use fframes::{Color, Svgr};
use kurbo::{Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::Space,
    kits::short,
    layout::Anchor,
    shapes::{Dot, Rectangle, RoundedRectangle},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{arrow, clock, label, line};

/// Two carts, A and B, on a straight track that collide head on, in closed form: masses
/// `m1` and `m2` kg, velocities `u1` and `u2` m/s before (positive to the right), centers
/// `x1` and `x2` meters from the track's left end. `restitution` is 1 for an elastic
/// collision and 0 for one where they stick together. The track starts at `track` (units of
/// `ui.space`, `scale` units per meter) and is `track_length` meters long; a cart stops at
/// its end. With `vectors` each cart carries its velocity, and with `momentum` bars under
/// the track show both momenta and their sum, which stays the same through the collision.
/// The track draws with `enter`, the carts and bars with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Collision1D {
    pub m1: f64,
    pub m2: f64,
    pub u1: f64,
    pub u2: f64,
    pub x1: f64,
    pub x2: f64,
    #[cfg_attr(feature = "studio", props(min = 0.0, max = 1.0, step = 0.05))]
    pub restitution: f64,
    pub track: (f64, f64),
    pub track_length: f64,
    pub scale: f64,
    pub delay: f32,
    pub time_scale: f32,
    pub vectors: bool,
    pub momentum: bool,
    /// Pixels of bar per kg·m/s.
    pub bar_scale: f32,
    pub color_a: Color,
    pub color_b: Color,
    pub total_color: Color,
    pub track_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Collision1D {
    fn themed(t: &Theme) -> Self {
        Self {
            m1: 2.0,
            m2: 1.0,
            u1: 3.0,
            u2: -1.0,
            x1: 2.0,
            x2: 8.0,
            restitution: 1.0,
            track: (-6.0, 0.0),
            track_length: 12.0,
            scale: 1.0,
            delay: 0.0,
            time_scale: 1.0,
            vectors: true,
            momentum: true,
            bar_scale: 40.0,
            color_a: t.colors.accent,
            color_b: t.colors.highlight,
            total_color: t.colors.text,
            track_color: t.colors.muted,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Collision1D {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Pixels from the track to the first momentum bar, and from one bar to the next.
const BARS: (f64, f64) = (150.0, 44.0);

impl Collision1D {
    /// Meters long, a cart of `mass` kg: heavier carts are longer.
    pub fn cart_length(mass: f64) -> f64 {
        0.4 + 0.5 * mass.max(0.0).sqrt()
    }

    /// Seconds until the carts touch, or `None` when they never close in.
    pub fn collision_time(&self) -> Option<f64> {
        let gap = (self.x2 - self.x1)
            - f64::midpoint(Self::cart_length(self.m1), Self::cart_length(self.m2));
        let closing = self.u1 - self.u2;
        (gap >= 0.0 && closing > 0.0).then(|| gap / closing)
    }

    /// The velocities after the collision, from momentum and the restitution.
    pub fn final_velocities(&self) -> (f64, f64) {
        let (m1, m2, u1, u2) = (self.m1, self.m2, self.u1, self.u2);
        let e = self.restitution.clamp(0.0, 1.0);
        let total = (m1 + m2).max(1e-9);
        let p = m1 * u1 + m2 * u2;
        (
            (p + m2 * e * (u2 - u1)) / total,
            (p + m1 * e * (u1 - u2)) / total,
        )
    }

    /// Each cart's center in meters and velocity in m/s after `t` seconds.
    pub fn state(&self, t: f64) -> [(f64, f64); 2] {
        let lengths = [Self::cart_length(self.m1), Self::cart_length(self.m2)];
        let (raw, v) = match self.collision_time() {
            Some(tc) if t > tc => {
                let (v1, v2) = self.final_velocities();
                let after = t - tc;
                (
                    [
                        self.x1 + self.u1 * tc + v1 * after,
                        self.x2 + self.u2 * tc + v2 * after,
                    ],
                    [v1, v2],
                )
            }
            _ => (
                [self.x1 + self.u1 * t, self.x2 + self.u2 * t],
                [self.u1, self.u2],
            ),
        };
        std::array::from_fn(|i| {
            let half = lengths[i] / 2.0;
            let x = raw[i].clamp(half, (self.track_length - half).max(half));
            // a cart held at the end of the track has stopped
            let v = if (x - raw[i]).abs() > 1e-9 { 0.0 } else { v[i] };
            (x, v)
        })
    }

    /// The momenta of A and B in kg·m/s after `t` seconds.
    pub fn momenta(&self, t: f64) -> (f64, f64) {
        let [(_, v1), (_, v2)] = self.state(t);
        (self.m1 * v1, self.m2 * v2)
    }

    fn px(&self, space: Space, x: f64, y: f64) -> Point {
        space.to_px(self.track.0 + x * self.scale, self.track.1 + y)
    }
}

impl<'a> Component<'a> for Collision1D {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let thin = ui.theme.stroke.thin;
        let start = self.px(space, 0.0, 0.0);
        let end = self.px(space, self.track_length, 0.0);
        let mut fixed = vec![line(ui, start, end, self.track_color, self.stroke_width)];
        let zero = Point::new(f64::midpoint(start.x, end.x), start.y);
        if self.momentum {
            let top = zero.y + BARS.0 - BARS.1 / 2.0;
            let mut axis = line(
                ui,
                Point::new(zero.x, top),
                Point::new(zero.x, top + 3.0 * BARS.1),
                self.track_color,
                thin,
            );
            axis.dash = Some((6.0, 6.0));
            fixed.push(axis);
        }
        let fixed = draw_animated(ui, &fixed, self.enter.as_ref(), self.exit.as_ref(), None);

        let t = clock(ui, self.delay, self.time_scale);
        let state = self.state(t);
        let mut shapes = Vec::new();
        let mut texts = Vec::new();
        let height = 70.0;
        for (i, ((x, v), (mass, color))) in state
            .iter()
            .zip([(self.m1, self.color_a), (self.m2, self.color_b)])
            .enumerate()
        {
            let length = space.px(Self::cart_length(mass) * self.scale);
            let middle = self.px(space, *x, 0.0) - Vec2::new(0.0, 14.0 + height / 2.0);
            shapes.push(
                RoundedRectangle {
                    cx: middle.x as f32,
                    cy: middle.y as f32,
                    width: length as f32,
                    height: height as f32,
                    corner_radius: 10.0,
                    fill: Some(color),
                    stroke: None,
                    ..ui.defaults()
                }
                .shape(),
            );
            for side in [-1.0, 1.0] {
                let wheel = middle + Vec2::new(side * length * 0.3, height / 2.0 + 2.0);
                shapes.push(
                    Dot {
                        cx: wheel.x as f32,
                        cy: wheel.y as f32,
                        radius: 12.0,
                        fill: Some(self.track_color),
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
            let name = if i == 0 { "A" } else { "B" };
            let text = format!("{name} {} kg", short(mass, 1));
            let background = ui.theme.colors.background;
            texts.push(label(ui, text, middle, Anchor::Center, background));
            if self.vectors && v.abs() > 1e-6 {
                let from = middle - Vec2::new(0.0, height / 2.0 + 30.0);
                let to = from + Vec2::new(space.px(v * self.scale * 0.35), 0.0);
                shapes.push(arrow(ui, from, to, color, self.stroke_width));
                let text = format!("{} m/s", short(*v, 1));
                let above = Point::new(f64::midpoint(from.x, to.x), from.y - 12.0);
                texts.push(label(ui, text, above, Anchor::Top, color));
            }
        }
        if self.momentum {
            let (p1, p2) = self.momenta(t);
            let rows = [("A", p1, self.color_a), ("B", p2, self.color_b)];
            let mut y = zero.y + BARS.0;
            for (name, p, color) in rows {
                let w = f64::from(self.bar_scale) * p;
                shapes.push(
                    Rectangle {
                        cx: (zero.x + w / 2.0) as f32,
                        cy: y as f32,
                        width: w.abs() as f32,
                        height: 26.0,
                        fill: Some(color),
                        stroke: None,
                        ..ui.defaults()
                    }
                    .shape(),
                );
                let at = Point::new(zero.x - 240.0, y);
                texts.push(label(ui, format!("p {name}"), at, Anchor::Left, color));
                y += BARS.1;
            }
            // the sum, stacked from the two
            let (w1, w2) = (
                f64::from(self.bar_scale) * p1,
                f64::from(self.bar_scale) * p2,
            );
            for (from, w, color) in [(0.0, w1, self.color_a), (w1, w2, self.color_b)] {
                shapes.push(
                    Rectangle {
                        cx: (zero.x + from + w / 2.0) as f32,
                        cy: y as f32,
                        width: w.abs() as f32,
                        height: 26.0,
                        fill: Some(color),
                        stroke: None,
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
            let at = Point::new(zero.x - 240.0, y);
            texts.push(label(
                ui,
                "total".to_owned(),
                at,
                Anchor::Left,
                self.total_color,
            ));
            let tip = Point::new(zero.x + (w1 + w2).max(w1).max(0.0) + 16.0, y);
            let text = format!("{} kg m/s", short(p1 + p2, 1));
            texts.push(label(ui, text, tip, Anchor::Right, self.total_color));
        }
        shapes.retain(|s| kurbo::Shape::area(&s.path).abs() > 0.0 || s.fill.is_none());
        let mut parts = vec![draw_animated(ui, &shapes, None, None, None)];
        parts.extend(texts);
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

    fn kinetic(c: &Collision1D, (v1, v2): (f64, f64)) -> f64 {
        0.5 * c.m1 * v1 * v1 + 0.5 * c.m2 * v2 * v2
    }

    #[test]
    fn an_elastic_collision_keeps_momentum_and_energy() {
        let c = Collision1D::default();
        let (v1, v2) = c.final_velocities();
        assert!((c.m1 * v1 + c.m2 * v2 - (c.m1 * c.u1 + c.m2 * c.u2)).abs() < 1e-12);
        assert!((kinetic(&c, (v1, v2)) - kinetic(&c, (c.u1, c.u2))).abs() < 1e-12);
        // equal masses swap their velocities
        let equal = Collision1D {
            m1: 1.0,
            m2: 1.0,
            ..c
        };
        let (a, b) = equal.final_velocities();
        assert!((a - equal.u2).abs() < 1e-12 && (b - equal.u1).abs() < 1e-12);
    }

    #[test]
    fn a_perfectly_inelastic_collision_leaves_them_together() {
        let c = Collision1D {
            restitution: 0.0,
            ..Collision1D::default()
        };
        let (v1, v2) = c.final_velocities();
        assert!((v1 - v2).abs() < 1e-12);
        assert!(kinetic(&c, (v1, v2)) < kinetic(&c, (c.u1, c.u2)));
    }

    #[test]
    fn the_carts_touch_at_the_collision_time() {
        let c = Collision1D::default();
        let tc = c.collision_time().unwrap();
        let [(x1, _), (x2, _)] = c.state(tc);
        let touching = f64::midpoint(
            Collision1D::cart_length(c.m1),
            Collision1D::cart_length(c.m2),
        );
        assert!((x2 - x1 - touching).abs() < 1e-9);
        // the total momentum is the same before and after
        let before = c.momenta(tc * 0.5);
        let after = c.momenta(tc + 0.2);
        assert!((before.0 + before.1 - after.0 - after.1).abs() < 1e-9);
        // carts moving apart never meet
        let apart = Collision1D {
            u1: -1.0,
            u2: 1.0,
            ..c
        };
        assert_eq!(apart.collision_time(), None);
    }

    #[test]
    fn a_cart_stops_at_the_end_of_the_track() {
        let c = Collision1D {
            u1: -5.0,
            u2: 5.0,
            ..Collision1D::default()
        };
        let [(x1, v1), (x2, v2)] = c.state(10.0);
        assert_eq!((v1, v2), (0.0, 0.0));
        assert!((x1 - Collision1D::cart_length(c.m1) / 2.0).abs() < 1e-12);
        assert!((x2 - (c.track_length - Collision1D::cart_length(c.m2) / 2.0)).abs() < 1e-12);
    }
}
