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
    shapes::Dot,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{GRAVITY, arrow, clock, label, line};

/// A ball launched at `speed` m/s and `angle` radians above the horizontal from `origin`
/// (units of `ui.space`, `scale` units per meter), without drag, landing at the launch
/// height. With `trace` its path draws behind it, with `vectors` it carries its velocity and
/// the velocity's two components (`vector_scale` units per m/s), and with `peak` and
/// `labels` the top of the arc and the landing point are marked with the height and the
/// range. The flight starts `delay` seconds after the local start and the ball stays where it
/// landed. The ground draws with `enter`, the rest fades with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Projectile {
    pub origin: (f64, f64),
    #[cfg_attr(feature = "studio", props(min = 0.0, max = 40.0, unit = "m/s"))]
    pub speed: f64,
    #[cfg_attr(feature = "studio", props(min = 0.0, max = 1.5708, unit = "rad"))]
    pub angle: f64,
    pub gravity: f64,
    pub scale: f64,
    pub delay: f32,
    pub time_scale: f32,
    pub trace: bool,
    pub vectors: bool,
    pub peak: bool,
    pub labels: bool,
    pub vector_scale: f64,
    pub radius: f32,
    pub color: Color,
    pub velocity_color: Color,
    pub component_color: Color,
    pub trace_color: Color,
    pub ground_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Projectile {
    fn themed(t: &Theme) -> Self {
        Self {
            origin: (-6.0, -2.5),
            speed: 10.0,
            angle: std::f64::consts::FRAC_PI_4,
            gravity: GRAVITY,
            scale: 1.0,
            delay: 0.0,
            time_scale: 1.0,
            trace: true,
            vectors: true,
            peak: true,
            labels: true,
            vector_scale: 0.12,
            radius: 14.0,
            color: t.colors.title,
            velocity_color: t.colors.accent,
            component_color: t.colors.highlight,
            trace_color: t.colors.muted,
            ground_color: t.colors.muted,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Projectile {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Projectile {
    /// Seconds from the launch until the ball is back at the launch height; 0 when it is
    /// not thrown upward or there is no gravity.
    pub fn flight_time(&self) -> f64 {
        let up = self.speed * self.angle.sin();
        if up <= 0.0 || self.gravity <= 0.0 {
            0.0
        } else {
            2.0 * up / self.gravity
        }
    }

    /// Meters from the launch point after `t` seconds of flight (`y` up), stopping at the
    /// landing.
    pub fn position(&self, t: f64) -> (f64, f64) {
        let t = t.clamp(0.0, self.flight_time());
        let (vx, vy) = (self.speed * self.angle.cos(), self.speed * self.angle.sin());
        (vx * t, vy * t - 0.5 * self.gravity * t * t)
    }

    /// Velocity in m/s after `t` seconds of flight.
    pub fn velocity(&self, t: f64) -> (f64, f64) {
        let t = t.clamp(0.0, self.flight_time());
        (
            self.speed * self.angle.cos(),
            self.speed * self.angle.sin() - self.gravity * t,
        )
    }

    /// The time, horizontal distance and height of the top of the arc.
    pub fn apex(&self) -> (f64, f64, f64) {
        let t = self.flight_time() / 2.0;
        let (x, y) = self.position(t);
        (t, x, y)
    }

    /// Meters from the launch point to the landing point.
    pub fn range(&self) -> f64 {
        self.position(self.flight_time()).0
    }

    /// The pixel of a point `meters` from the launch point.
    fn px(&self, space: Space, (x, y): (f64, f64)) -> Point {
        space.to_px(
            self.origin.0 + x * self.scale,
            self.origin.1 + y * self.scale,
        )
    }
}

/// `content` fading in over a quarter second from `since` seconds before now.
fn after(content: Svgr<'_>, since: f64) -> Svgr<'_> {
    let shown = progress(since as f32, 0.0, 0.25);
    if shown <= 0.0 {
        return Svgr::empty();
    }
    fframes::svgr!(<g opacity={shown}>{content}</g>)
}

impl<'a> Component<'a> for Projectile {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let flight = self.flight_time();
        let t = clock(ui, self.delay, self.time_scale).min(flight);
        let muted = self.ground_color;
        let thin = ui.theme.stroke.thin;
        let start = self.px(space, (0.0, 0.0));
        let range = self.range();
        let reach = space.px(0.6);
        let ground = [line(
            ui,
            start - Vec2::new(reach, 0.0),
            self.px(space, (range, 0.0)) + Vec2::new(reach, 0.0),
            muted,
            thin,
        )];
        let ground = draw_animated(ui, &ground, self.enter.as_ref(), self.exit.as_ref(), None);

        let ball = self.px(space, self.position(t));
        let mut shapes: Vec<Shape> = Vec::new();
        if self.trace && t > 0.0 {
            let mut path = BezPath::new();
            path.move_to(start);
            let steps = 64;
            for i in 1..=steps {
                path.line_to(self.px(space, self.position(t * f64::from(i) / f64::from(steps))));
            }
            shapes.push(Shape {
                path,
                fill: None,
                stroke: Some(self.trace_color),
                stroke_width: thin,
                dash: Some((10.0, 8.0)),
                opacity: 1.0,
            });
        }
        // a landed ball is at rest: its vectors fade out
        let landed = clock(ui, self.delay, self.time_scale) - flight;
        let moving = 1.0 - progress(landed as f32, 0.0, 0.25);
        if self.vectors && moving > 0.0 {
            let (vx, vy) = self.velocity(t);
            let unit = space.px(self.vector_scale);
            let along = Vec2::new(vx * unit, 0.0);
            let up = Vec2::new(0.0, -vy * unit);
            let mut vectors = [
                arrow(ui, ball, ball + along, self.component_color, thin),
                arrow(ui, ball, ball + up, self.component_color, thin),
                arrow(
                    ui,
                    ball,
                    ball + along + up,
                    self.velocity_color,
                    self.stroke_width,
                ),
            ];
            for v in &mut vectors {
                v.opacity = moving;
            }
            shapes.extend(vectors);
        }
        shapes.push(
            Dot {
                cx: ball.x as f32,
                cy: ball.y as f32,
                radius: self.radius,
                fill: Some(self.color),
                ..ui.defaults()
            }
            .shape(),
        );
        let mut parts = vec![draw_animated(ui, &shapes, None, None, None)];

        let (apex_t, apex_x, height) = self.apex();
        if self.peak && flight > 0.0 && t >= apex_t {
            let top = self.px(space, (apex_x, height));
            let foot = self.px(space, (apex_x, 0.0));
            let mut drop = line(ui, top, foot, muted, thin);
            drop.dash = Some((8.0, 8.0));
            let marks = [
                drop,
                Dot {
                    cx: top.x as f32,
                    cy: top.y as f32,
                    radius: 7.0,
                    fill: Some(self.component_color),
                    ..ui.defaults()
                }
                .shape(),
            ];
            let mut peak = vec![draw_animated(ui, &marks, None, None, None)];
            if self.labels {
                let text = format!("h = {} m", short(height, 1));
                peak.push(label(ui, text, top, Anchor::Top, self.component_color));
            }
            parts.push(after(peak.into(), t - apex_t));
        }
        if self.labels && flight > 0.0 && t >= flight {
            let at = self.px(space, (range, 0.0)) + Vec2::new(0.0, f64::from(self.radius));
            let text = format!("R = {} m", short(range, 1));
            parts.push(after(
                label(ui, text, at, Anchor::Bottom, ui.theme.colors.text),
                landed,
            ));
        }
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{ground}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flight_matches_the_textbook_formulas() {
        let p = Projectile {
            speed: 10.0,
            angle: std::f64::consts::FRAC_PI_4,
            gravity: 10.0,
            ..Projectile::default()
        };
        // T = 2 v sin(a) / g, R = v² sin(2a) / g, H = v² sin²(a) / (2 g)
        assert!((p.flight_time() - 2.0_f64.sqrt()).abs() < 1e-12);
        assert!((p.range() - 10.0).abs() < 1e-9);
        let (t, x, h) = p.apex();
        assert!((t - p.flight_time() / 2.0).abs() < 1e-12);
        assert!((x - 5.0).abs() < 1e-9 && (h - 2.5).abs() < 1e-9);
        // the vertical speed turns over at the top and comes back mirrored
        assert!(p.velocity(t).1.abs() < 1e-9);
        assert!((p.velocity(p.flight_time()).1 + p.velocity(0.0).1).abs() < 1e-9);
        // after the landing the ball stays put
        assert_eq!(p.position(99.0), p.position(p.flight_time()));
    }

    #[test]
    fn a_ball_thrown_flat_or_down_never_flies() {
        for angle in [0.0, -0.3] {
            let p = Projectile {
                angle,
                ..Projectile::default()
            };
            assert_eq!(p.flight_time(), 0.0);
            assert_eq!(p.position(1.0), (0.0, 0.0));
        }
    }

    #[test]
    fn meters_map_to_space_units_with_scale() {
        let p = Projectile {
            origin: (1.0, 2.0),
            scale: 0.5,
            ..Projectile::default()
        };
        let space = Space {
            origin: (0.0, 0.0),
            unit: 100.0,
        };
        assert_eq!(p.px(space, (4.0, 2.0)), Point::new(300.0, -300.0));
    }
}
