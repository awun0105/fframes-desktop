use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    layout::Anchor,
    motion::{cubic_in_out, progress},
    shapes::{Circle, Dot},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{label, line, zigzag};

/// A series circuit on a rectangular loop centered on `center`, `width` by `height` units of
/// `ui.space`: a battery on the left side, a switch on the top, a bulb on the right and a
/// resistor on the bottom, each left out when its flag is off (the wire runs on). The switch
/// closes `on_at` seconds after the local start; then the current flows as dots, clockwise
/// out of the battery's long plate (+) at `current_speed` pixels a second, and the bulb glows.
/// Without a switch the current starts at `on_at`. A label left empty is not drawn. The wire
/// and the parts draw with `enter`, the labels, dots and glow fade with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Circuit<'a> {
    pub center: (f64, f64),
    pub width: f64,
    pub height: f64,
    pub battery: bool,
    pub switch: bool,
    pub bulb: bool,
    pub resistor: bool,
    pub on_at: f32,
    pub current_speed: f32,
    pub dots: usize,
    pub battery_label: &'a str,
    pub switch_label: &'a str,
    pub bulb_label: &'a str,
    pub resistor_label: &'a str,
    pub wire_color: Color,
    pub dot_color: Color,
    pub glow_color: Color,
    pub label_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for Circuit<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.0, 0.0),
            width: 8.0,
            height: 4.5,
            battery: true,
            switch: true,
            bulb: true,
            resistor: true,
            on_at: 1.0,
            current_speed: 160.0,
            dots: 28,
            battery_label: "9 V",
            switch_label: "",
            bulb_label: "",
            resistor_label: "R",
            wire_color: t.colors.text,
            dot_color: t.colors.accent,
            glow_color: t.colors.highlight,
            label_color: t.colors.text,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for Circuit<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Seconds the switch takes to close.
const CLOSING: f32 = 0.3;
/// Half the wire each part takes, in pixels: battery, switch, bulb, resistor.
const HALF_GAPS: [f64; 4] = [8.0, 40.0, 30.0, 50.0];

/// The loop in pixels, walked clockwise on screen from the top left corner: `s` pixels along
/// it is a point on the wire.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Loop {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl Loop {
    fn w(&self) -> f64 {
        self.right - self.left
    }

    fn h(&self) -> f64 {
        self.bottom - self.top
    }

    fn perimeter(&self) -> f64 {
        2.0 * (self.w() + self.h())
    }

    /// Distances of the corners from the top left one.
    fn corners(&self) -> [f64; 4] {
        [
            0.0,
            self.w(),
            self.w() + self.h(),
            2.0 * self.w() + self.h(),
        ]
    }

    fn at(&self, s: f64) -> Point {
        let (w, h) = (self.w(), self.h());
        let s = s.rem_euclid(self.perimeter().max(1e-9));
        if s < w {
            Point::new(self.left + s, self.top)
        } else if s < w + h {
            Point::new(self.right, self.top + s - w)
        } else if s < 2.0 * w + h {
            Point::new(self.right - (s - w - h), self.bottom)
        } else {
            Point::new(self.left, self.bottom - (s - 2.0 * w - h))
        }
    }

    /// The middle of the left, top, right and bottom side: where the battery, switch, bulb
    /// and resistor sit.
    fn middles(&self) -> [f64; 4] {
        let (w, h) = (self.w(), self.h());
        [2.0 * w + 1.5 * h, w / 2.0, w + h / 2.0, 1.5 * w + h]
    }

    /// The wire from `a` to `b` pixels along (`b` may pass the perimeter), around the corners.
    fn wire(&self, a: f64, b: f64) -> BezPath {
        let p = self.perimeter();
        let mut path = BezPath::new();
        path.move_to(self.at(a));
        let mut corners: Vec<f64> = (0..3)
            .flat_map(|k| self.corners().map(|c| c + f64::from(k) * p))
            .filter(|&c| c > a && c < b)
            .collect();
        corners.sort_by(f64::total_cmp);
        for c in corners {
            path.line_to(self.at(c));
        }
        path.line_to(self.at(b));
        path
    }
}

/// Cyclic distance between two points along a loop `p` long.
fn apart(a: f64, b: f64, p: f64) -> f64 {
    let d = (a - b).rem_euclid(p);
    d.min(p - d)
}

impl Circuit<'_> {
    fn parts(&self) -> [bool; 4] {
        [self.battery, self.switch, self.bulb, self.resistor]
    }

    /// The angle of the switch's lever, from open (35°) to closed (0).
    pub fn lever_angle(&self, now: f32) -> f64 {
        let closing = cubic_in_out(progress(now, self.on_at, self.on_at + CLOSING));
        35f64.to_radians() * f64::from(1.0 - closing)
    }

    /// Seconds the current has flowed at `now` seconds of local time, 0 before it starts.
    pub fn flowing(&self, now: f32) -> f32 {
        let start = if self.switch {
            self.on_at + CLOSING
        } else {
            self.on_at
        };
        (now - start).max(0.0)
    }

    /// Where the current's dots are, in pixels along `wire`, with the ones inside a part
    /// left out.
    fn dot_positions(&self, wire: &Loop, now: f32) -> Vec<f64> {
        let p = wire.perimeter();
        let n = self.dots.max(1);
        let offset = f64::from(self.flowing(now) * self.current_speed);
        let gaps: Vec<(f64, f64)> = wire
            .middles()
            .into_iter()
            .zip(HALF_GAPS)
            .zip(self.parts())
            .filter(|(_, on)| *on)
            .map(|(gap, _)| gap)
            .collect();
        (0..n)
            .map(|i| (p * i as f64 / n as f64 + offset).rem_euclid(p))
            .filter(|&s| gaps.iter().all(|&(at, half)| apart(s, at, p) > half + 6.0))
            .collect()
    }
}

impl<'a> Component<'a> for Circuit<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let top_left = space.to_px(
            self.center.0 - self.width / 2.0,
            self.center.1 + self.height / 2.0,
        );
        let bottom_right = space.to_px(
            self.center.0 + self.width / 2.0,
            self.center.1 - self.height / 2.0,
        );
        let wire = Loop {
            left: top_left.x,
            top: top_left.y,
            right: bottom_right.x,
            bottom: bottom_right.y,
        };
        let p = wire.perimeter();
        let now = ui.seconds();
        let (color, width) = (self.wire_color, self.stroke_width);
        let stroke = |path: BezPath, width: f32| Shape {
            path,
            fill: None,
            stroke: Some(color),
            stroke_width: width,
            dash: None,
            opacity: 1.0,
        };

        // the wire between the parts, in order along the loop
        let mut gaps: Vec<(f64, f64)> = wire
            .middles()
            .into_iter()
            .zip(HALF_GAPS)
            .zip(self.parts())
            .filter(|(_, on)| *on)
            .map(|((at, half), _)| (at - half, at + half))
            .collect();
        gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut shapes = Vec::new();
        if gaps.is_empty() {
            let mut path = wire.wire(0.0, p);
            path.close_path();
            shapes.push(stroke(path, width));
        }
        for (i, &(_, end)) in gaps.iter().enumerate() {
            let next = gaps[(i + 1) % gaps.len()].0;
            let next = if next <= end { next + p } else { next };
            shapes.push(stroke(wire.wire(end, next), width));
        }

        let [battery, switch, bulb, resistor] = wire.middles().map(|s| wire.at(s));
        let mut labels = Vec::new();
        if self.battery {
            // the long plate is +, toward the top where the current leaves
            let plate = |y: f64, half: f64, w: f32| {
                let (a, b) = (
                    Point::new(battery.x - half, y),
                    Point::new(battery.x + half, y),
                );
                line(ui, a, b, color, w)
            };
            shapes.push(plate(battery.y - 8.0, 30.0, width));
            shapes.push(plate(battery.y + 8.0, 15.0, width * 2.5));
            labels.push((
                self.battery_label,
                battery - Vec2::new(40.0, 0.0),
                Anchor::Left,
            ));
        }
        if self.switch {
            let a = switch - Vec2::new(40.0, 0.0);
            let b = switch + Vec2::new(40.0, 0.0);
            for at in [a, b] {
                shapes.push(
                    Dot {
                        cx: at.x as f32,
                        cy: at.y as f32,
                        radius: 6.0,
                        fill: Some(color),
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
            let angle = self.lever_angle(now);
            let tip = a + Vec2::new(angle.cos(), -angle.sin()) * 80.0;
            shapes.push(line(ui, a, tip, color, width));
            labels.push((
                self.switch_label,
                switch - Vec2::new(0.0, 40.0),
                Anchor::Top,
            ));
        }
        let mut glow_shapes = Vec::new();
        if self.bulb {
            let r = HALF_GAPS[2];
            shapes.push(
                Circle {
                    cx: bulb.x as f32,
                    cy: bulb.y as f32,
                    radius: r as f32,
                    fill: None,
                    stroke: Some(color),
                    stroke_width: width,
                    ..ui.defaults()
                }
                .shape(),
            );
            let d = r * std::f64::consts::FRAC_1_SQRT_2;
            for (dx, dy) in [(d, d), (d, -d)] {
                let (a, b) = (bulb + Vec2::new(-dx, -dy), bulb + Vec2::new(dx, dy));
                shapes.push(line(ui, a, b, color, width));
            }
            let glow = progress(self.flowing(now), 0.0, 0.3);
            if glow > 0.0 {
                for (radius, opacity) in [(r * 2.2, 0.2), (r, 0.85)] {
                    let mut dot = Dot {
                        cx: bulb.x as f32,
                        cy: bulb.y as f32,
                        radius: radius as f32,
                        fill: Some(self.glow_color),
                        ..ui.defaults()
                    }
                    .shape();
                    dot.opacity = glow * opacity;
                    glow_shapes.push(dot);
                }
            }
            labels.push((
                self.bulb_label,
                bulb + Vec2::new(r + 12.0, 0.0),
                Anchor::Right,
            ));
        }
        if self.resistor {
            let half = HALF_GAPS[3];
            let path = zigzag(
                resistor + Vec2::new(half, 0.0),
                resistor - Vec2::new(half, 0.0),
                5,
                14.0,
            );
            shapes.push(stroke(path, width));
            labels.push((
                self.resistor_label,
                resistor + Vec2::new(0.0, 24.0),
                Anchor::Bottom,
            ));
        }
        let drawn = draw_animated(ui, &shapes, self.enter.as_ref(), self.exit.as_ref(), None);
        let mut texts = Vec::new();
        for (text, at, direction) in labels {
            if !text.is_empty() {
                texts.push(label(ui, text.to_owned(), at, direction, self.label_color));
            }
        }
        let texts = fade(
            ui,
            texts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        let fixed = fframes::svgr!(<g>{drawn}{texts}</g>);

        let mut moving = glow_shapes;
        if self.flowing(now) > 0.0 {
            for s in self.dot_positions(&wire, now) {
                let at = wire.at(s);
                moving.push(
                    Dot {
                        cx: at.x as f32,
                        cy: at.y as f32,
                        radius: 7.0,
                        fill: Some(self.dot_color),
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
        }
        let moving = draw_animated(ui, &moving, None, None, None);
        let moving = fade(
            ui,
            moving,
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{moving}{fixed}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIRE: Loop = Loop {
        left: 0.0,
        top: 0.0,
        right: 400.0,
        bottom: 200.0,
    };

    #[test]
    fn the_loop_runs_clockwise_from_the_top_left() {
        assert_eq!(WIRE.perimeter(), 1200.0);
        assert_eq!(WIRE.at(100.0), Point::new(100.0, 0.0));
        assert_eq!(WIRE.at(500.0), Point::new(400.0, 100.0));
        assert_eq!(WIRE.at(700.0), Point::new(300.0, 200.0));
        assert_eq!(WIRE.at(1100.0), Point::new(0.0, 100.0));
        assert_eq!(WIRE.at(1300.0), WIRE.at(100.0));
        // the battery sits on the left side, the bulb on the right
        let [battery, switch, bulb, resistor] = WIRE.middles().map(|s| WIRE.at(s));
        assert_eq!(battery, Point::new(0.0, 100.0));
        assert_eq!(switch, Point::new(200.0, 0.0));
        assert_eq!(bulb, Point::new(400.0, 100.0));
        assert_eq!(resistor, Point::new(200.0, 200.0));
    }

    #[test]
    fn a_wire_turns_its_corners_and_wraps() {
        let path = WIRE.wire(1100.0, 1300.0);
        let els = path.elements();
        // start, the top left corner, the end
        assert_eq!(els.len(), 3);
        assert!(matches!(els[1], kurbo::PathEl::LineTo(p) if p == Point::new(0.0, 0.0)));
    }

    #[test]
    fn no_current_flows_before_the_switch_closes() {
        let circuit = Circuit {
            on_at: 1.0,
            ..Circuit::default()
        };
        assert_eq!(circuit.flowing(1.2), 0.0);
        assert!((circuit.lever_angle(0.5) - 35f64.to_radians()).abs() < 1e-12);
        assert_eq!(circuit.lever_angle(2.0), 0.0);
        assert!((circuit.flowing(2.0) - 0.7).abs() < 1e-6);
        let wired = Circuit {
            switch: false,
            ..circuit
        };
        assert!((wired.flowing(2.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn dots_move_on_and_skip_the_parts() {
        let circuit = Circuit {
            dots: 60,
            ..Circuit::default()
        };
        let p = WIRE.perimeter();
        for now in [1.5, 2.0, 3.7] {
            for s in circuit.dot_positions(&WIRE, now) {
                for (at, half) in WIRE.middles().into_iter().zip(HALF_GAPS) {
                    assert!(apart(s, at, p) > half, "{s} in the part at {at}");
                }
            }
        }
        let without = Circuit {
            battery: false,
            switch: false,
            bulb: false,
            resistor: false,
            ..circuit
        };
        assert_eq!(without.dot_positions(&WIRE, 0.0).len(), 60);
        // clockwise: the first dot moves on by the speed times the time
        let later = without.dot_positions(&WIRE, without.on_at + 0.5)[0];
        assert!((later - f64::from(without.current_speed) * 0.5).abs() < 1e-3);
    }
}
