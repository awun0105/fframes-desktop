use std::f64::consts::TAU;

use fframes::{Color, Svgr};
use kurbo::{Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, ValueTracker, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::Space,
    label::Label,
    layout::Anchor,
    shapes::{Arc, Circle, Dot, Line},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::short;

/// The unit circle at a tracked angle: the radius to the point, its cosine along the
/// x-axis and its sine up to the point, the angle's arc and, with `labels`, their values
/// (`cos 0.71`, `sin 0.71`, `45°`). `center` and `radius` are in units of `ui.space`,
/// `angle` in radians counterclockwise. The circle and its axes draw with `enter`, the rest
/// fades with `parts_enter`.
#[derive(Debug, Clone)]
pub struct UnitCircle<'a> {
    pub center: (f64, f64),
    pub radius: f64,
    pub angle: ValueTracker<'a>,
    pub labels: bool,
    pub color: Color,
    pub cos_color: Color,
    pub sin_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for UnitCircle<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.0, 0.0),
            radius: 3.0,
            angle: 0.0.into(),
            labels: true,
            color: t.colors.text,
            cos_color: t.colors.accent,
            sin_color: t.colors.highlight,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for UnitCircle<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Pixels of the circle at one angle.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Pose {
    center: Point,
    radius: f64,
    /// The point on the circle.
    point: Point,
    /// The foot of the sine on the x-axis, `cos` along from the center.
    foot: Point,
}

impl UnitCircle<'_> {
    fn pose(&self, space: Space, angle: f64) -> Pose {
        let center = space.to_px(self.center.0, self.center.1);
        let radius = self.radius * f64::from(space.unit);
        let point = center + Vec2::new(angle.cos(), -angle.sin()) * radius;
        Pose {
            center,
            radius,
            point,
            foot: Point::new(point.x, center.y),
        }
    }
}

fn line(ui: &Ui<'_, '_, '_>, a: Point, b: Point, color: Color, width: f32) -> Line {
    Line {
        x1: a.x as f32,
        y1: a.y as f32,
        x2: b.x as f32,
        y2: b.y as f32,
        stroke: Some(color),
        stroke_width: width,
        ..ui.defaults()
    }
}

fn label<'a>(ui: &Ui<'a, '_, '_>, text: String, at: Point, direction: Anchor) -> Label<'a> {
    Label {
        text: text.into(),
        x: at.x as f32,
        y: at.y as f32,
        direction,
        style: ui.theme.typography.caption,
        ..ui.defaults()
    }
}

impl<'a> Component<'a> for UnitCircle<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let angle = f64::from(ui.sample(&self.angle));
        let p = self.pose(ui.space, angle);
        let (c, r) = (p.center, p.radius);
        let muted = ui.theme.colors.muted;
        let thin = ui.theme.stroke.thin;
        let reach = Vec2::new(r * 1.2, 0.0);
        let up = Vec2::new(0.0, r * 1.2);
        let frame = [
            line(ui, c - reach, c + reach, muted, thin).shape(),
            line(ui, c - up, c + up, muted, thin).shape(),
            Circle {
                cx: c.x as f32,
                cy: c.y as f32,
                radius: r as f32,
                stroke: Some(self.color),
                stroke_width: self.stroke_width,
                ..ui.defaults()
            }
            .shape(),
        ];
        let frame = draw_animated(ui, &frame, self.enter.as_ref(), self.exit.as_ref(), None);

        let turn = angle.rem_euclid(TAU);
        let w = self.stroke_width;
        let parts = [
            line(ui, c, p.foot, self.cos_color, w).shape(),
            line(ui, p.foot, p.point, self.sin_color, w).shape(),
            line(ui, c, p.point, self.color, w).shape(),
            Arc {
                cx: c.x as f32,
                cy: c.y as f32,
                radius: (r * 0.22) as f32,
                start_angle: 0.0,
                angle: turn as f32,
                stroke: Some(self.color),
                stroke_width: thin,
                ..ui.defaults()
            }
            .shape(),
            Dot {
                cx: p.point.x as f32,
                cy: p.point.y as f32,
                radius: 12.0,
                fill: Some(self.color),
                ..ui.defaults()
            }
            .shape(),
        ];
        let mut moving = vec![draw_animated(ui, &parts, None, None, None)];
        if self.labels {
            let (cos, sin) = (angle.cos(), angle.sin());
            let below = if sin >= 0.0 {
                Anchor::Bottom
            } else {
                Anchor::Top
            };
            let side = if cos >= 0.0 {
                Anchor::Right
            } else {
                Anchor::Left
            };
            let half = Vec2::new((turn / 2.0).cos(), -(turn / 2.0).sin());
            let labels = [
                label(
                    ui,
                    format!("cos {}", short(cos, 2)),
                    c.midpoint(p.foot),
                    below,
                ),
                label(
                    ui,
                    format!("sin {}", short(sin, 2)),
                    p.foot.midpoint(p.point),
                    side,
                ),
                label(
                    ui,
                    format!("{}°", short(turn.to_degrees(), 0)),
                    c + half * r * 0.24,
                    Anchor::toward(half.x, half.y),
                ),
            ];
            moving.extend(labels.map(|l| ui.show(l)));
        }
        let moving = fade(
            ui,
            moving.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{frame}{moving}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_point_and_its_foot_follow_the_angle() {
        let space = Space {
            origin: (0.0, 0.0),
            unit: 100.0,
        };
        let circle = UnitCircle {
            center: (1.0, 0.0),
            radius: 2.0,
            ..UnitCircle::default()
        };
        // a quarter turn is straight up on screen
        let p = circle.pose(space, std::f64::consts::FRAC_PI_2);
        assert_eq!((p.center, p.radius), (Point::new(100.0, 0.0), 200.0));
        assert!((p.point - Point::new(100.0, -200.0)).hypot() < 1e-9);
        assert!((p.foot - Point::new(100.0, 0.0)).hypot() < 1e-9);
        let p = circle.pose(space, std::f64::consts::PI / 3.0);
        // cos 60° = 0.5: the foot is one unit right of the center
        assert!((p.foot - Point::new(200.0, 0.0)).hypot() < 1e-9);
    }
}
