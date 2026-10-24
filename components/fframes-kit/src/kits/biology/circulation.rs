//! Double circulation: the heart pumping blood through the lungs and round the body.

use std::f64::consts::PI;

use fframes::{Color, Svgr};
use kurbo::{Affine, BezPath, Circle, Point, RoundedRect, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::{Look, Shape},
    kits::draw::{mix, stroke},
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// When the vessels draw, in seconds after the delay: slow enough that the line never jumps.
const DRAW: (f32, f32) = (0.4, 3.6);

/// The chambers, as (name, centre) in pixels from the middle of the heart, `y` down.
const CHAMBERS: [(&str, (f64, f64)); 4] = [
    ("RA", (-70.0, -52.0)),
    ("RV", (-70.0, 57.0)),
    ("LA", (70.0, -52.0)),
    ("LV", (70.0, 57.0)),
];

/// The loop the blood takes, at scale 1 around the middle of the heart: its points, and
/// where (by length) it is in the lungs and in the body.
struct Loop {
    points: Vec<Point>,
    /// Length along the loop at each point.
    at: Vec<f64>,
    lungs: (f64, f64),
    body: (f64, f64),
}

impl Loop {
    fn new() -> Self {
        // a capillary bed: a wave across from `x0` to `x1` at `y`
        let bed = |x0: f64, x1: f64, y: f64| -> Vec<Point> {
            (0..=48)
                .map(|i| {
                    let u = f64::from(i) / 48.0;
                    Point::new(x0 + (x1 - x0) * u, y + 18.0 * (u * 8.0 * PI).sin())
                })
                .collect()
        };
        let p = |x: f64, y: f64| Point::new(x, y);
        let mut points = vec![
            p(-70.0, -52.0), // right atrium
            p(-70.0, 57.0),  // right ventricle
            p(-170.0, 57.0), // pulmonary artery, up to the lungs
            p(-170.0, -330.0),
        ];
        let lungs_from = points.len() - 1;
        points.extend(bed(-170.0, 170.0, -330.0).into_iter().skip(1));
        let lungs_to = points.len() - 1;
        points.extend([
            p(170.0, -52.0), // pulmonary vein, down to the left atrium
            p(70.0, -52.0),
            p(70.0, 57.0),  // left ventricle
            p(320.0, 57.0), // aorta, down to the body
            p(320.0, 330.0),
        ]);
        let body_from = points.len() - 1;
        points.extend(bed(320.0, -320.0, 330.0).into_iter().skip(1));
        let body_to = points.len() - 1;
        points.extend([p(-320.0, -52.0), p(-70.0, -52.0)]); // vena cava, back to the right atrium
        let mut at = vec![0.0];
        for w in points.windows(2) {
            at.push(at[at.len() - 1] + (w[1] - w[0]).hypot());
        }
        Self {
            lungs: (at[lungs_from], at[lungs_to]),
            body: (at[body_from], at[body_to]),
            points,
            at,
        }
    }

    fn length(&self) -> f64 {
        self.at[self.at.len() - 1]
    }

    /// How much oxygen the blood carries at `s` along the loop: 0 from the body to the lungs,
    /// rising through the lungs, 1 from the lungs to the body, falling through the body.
    fn oxygen(&self, s: f64) -> f32 {
        let ramp = |a: f64, b: f64| ((s - a) / (b - a)).clamp(0.0, 1.0) as f32;
        ramp(self.lungs.0, self.lungs.1) - ramp(self.body.0, self.body.1)
    }

    /// The point `s` along the loop.
    fn point(&self, s: f64) -> Point {
        let s = s.rem_euclid(self.length());
        let i = self
            .at
            .partition_point(|&a| a <= s)
            .clamp(1, self.at.len() - 1);
        let (a, b) = (self.at[i - 1], self.at[i]);
        let f = if b > a { (s - a) / (b - a) } else { 0.0 };
        self.points[i - 1].lerp(self.points[i], f)
    }
}

/// Double circulation: the heart's four chambers (right atrium and ventricle on the viewer's
/// left, left atrium and ventricle on the right) with the lungs above and the body below,
/// `scale` times its normal size (about 860 by 820 pixels) around `center` (units of
/// `ui.space`). From `delay` the chambers appear and the vessels draw along the way the blood
/// goes: right ventricle, pulmonary artery, lungs, pulmonary vein, left atrium, left
/// ventricle, aorta, body, vena cava, right atrium. Then blood cells run round the loop at
/// `speed` pixels a second, turning from `deoxygenated` to `oxygenated` in the lungs and back
/// in the body, while the heart beats `bpm` times a minute, atria first. With `labels` the
/// lungs, the body and the vessels are named.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Circulation {
    pub delay: f32,
    pub center: (f64, f64),
    pub scale: f64,
    pub speed: f32,
    pub bpm: f32,
    pub labels: bool,
    pub oxygenated: Color,
    pub deoxygenated: Color,
    pub chamber_color: Color,
    pub bed_color: Color,
    pub gap_color: Color,
    pub label_style: TextStyle,
    pub chamber_style: TextStyle,
    pub text_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for Circulation {
    fn themed(t: &Theme) -> Self {
        let mut chamber_style = t.typography.caption;
        chamber_style.weight = 700;
        Self {
            delay: 0.0,
            center: (0.0, -0.15),
            scale: 1.0,
            speed: 260.0,
            bpm: 70.0,
            labels: true,
            oxygenated: Color::hex("#ff6b6b"),
            deoxygenated: Color::hex("#4dabf7"),
            chamber_color: Color::hex("#2a1c2e"),
            bed_color: Color::hex("#1b2735"),
            gap_color: t.colors.background,
            label_style: t.typography.caption,
            chamber_style,
            text_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Circulation {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Circulation {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let place = Affine::translate(middle.to_vec2()) * Affine::scale(self.scale);
        let blood = Loop::new();
        let t = ui.seconds() - self.delay;
        let color_at = |s: f64| mix(self.deoxygenated, self.oxygenated, blood.oxygen(s));
        let width = (16.0 * self.scale) as f32;

        let mut out = Vec::new();
        // the lungs and the body behind their capillaries
        for (y, name) in [(-330.0, "lungs"), (330.0, "body")] {
            let half = if name == "lungs" { 200.0 } else { 350.0 };
            let shown = progress(t, 0.0, 0.5);
            let bed = Shape {
                path: place * RoundedRect::new(-half, y - 50.0, half, y + 50.0, 30.0).to_path(0.1),
                fill: Some(self.bed_color),
                stroke: None,
                stroke_width: 0.0,
                dash: None,
                opacity: shown,
            };
            out.push(bed.draw());
            if self.labels {
                let text_y = if y < 0.0 {
                    y - 66.0
                } else {
                    y + 66.0 + self.label_style.size as f64 * 0.7
                };
                let at = place * Point::new(0.0, text_y);
                out.push(line(
                    at.x as f32,
                    at.y as f32,
                    self.label_style,
                    self.text_color,
                    TextAnchor::Middle,
                    0.0,
                    shown,
                    name.into(),
                ));
            }
        }

        // the vessels draw along the loop, coloured by the blood in them; the vena cava is
        // drawn over the pulmonary artery where they cross
        let drawn = f64::from(progress(t, DRAW.0, DRAW.1)) * blood.length();
        // pieces of one colour each: the points, the oxygen level in steps of a sixteenth,
        // and where along the loop the piece starts
        let mut pieces: Vec<(Vec<Point>, f32, f64)> = Vec::new();
        for (i, &p) in blood.points.iter().enumerate() {
            let s = blood.at[i];
            if s > drawn {
                if let (Some(last), true) = (pieces.last_mut(), i > 0) {
                    let f = (drawn - blood.at[i - 1]) / (s - blood.at[i - 1]);
                    last.0.push(blood.points[i - 1].lerp(p, f));
                }
                break;
            }
            let level = (blood.oxygen(s) * 16.0).round() / 16.0;
            match pieces.last_mut() {
                Some(last) if last.1 == level => last.0.push(p),
                Some(last) => {
                    last.0.push(p);
                    pieces.push((vec![p], level, s));
                }
                None => pieces.push((vec![p], level, s)),
            }
        }
        for (points, level, s) in pieces {
            if points.len() < 2 {
                continue;
            }
            if s >= blood.body.0 && level == 0.0 {
                // only above the heart's middle, where the crossing is, not over the body bed
                let upper: Vec<Point> = points.iter().copied().filter(|p| p.y < 0.0).collect();
                if upper.len() > 1 {
                    let gap = place * polyline(&upper);
                    out.push(stroke(gap, self.gap_color, width + 10.0, 1.0).draw());
                }
            }
            let path = place * polyline(&points);
            let color = mix(self.deoxygenated, self.oxygenated, level);
            out.push(stroke(path, color, width, 1.0).draw());
        }

        // blood cells round the loop once it is drawn, under the chambers they pass through
        let flowing = progress(t, DRAW.1, DRAW.1 + 0.6);
        if flowing > 0.0 {
            let spacing = 70.0;
            let count = (blood.length() / spacing).floor() as usize;
            let travelled = f64::from(t - DRAW.1) * f64::from(self.speed);
            for k in 0..count {
                let s = (k as f64 * blood.length() / count as f64 + travelled)
                    .rem_euclid(blood.length());
                let cell = Shape {
                    path: place * Circle::new(blood.point(s), 8.0).to_path(0.1),
                    fill: Some(color_at(s)),
                    stroke: Some(Color::WHITE),
                    stroke_width: 1.5,
                    dash: None,
                    opacity: flowing,
                };
                out.push(cell.draw());
            }
        }

        // the chambers, beating: the atria squeeze, then the ventricles
        let beat = (f64::from(t.max(0.0)) * f64::from(self.bpm) / 60.0).fract();
        let squeeze = |from: f64, to: f64| {
            if beat > from && beat < to {
                1.0 - 0.07 * (PI * (beat - from) / (to - from)).sin()
            } else {
                1.0
            }
        };
        let shown = cubic_in_out(progress(t, 0.0, 0.6));
        for (name, (x, y)) in CHAMBERS {
            let atrium = name.ends_with('A');
            let scale = if atrium {
                squeeze(0.0, 0.18)
            } else {
                squeeze(0.18, 0.45)
            };
            let color = if name.starts_with('R') {
                self.deoxygenated
            } else {
                self.oxygenated
            };
            let c = Point::new(x, y);
            let (w, h) = (120.0, if atrium { 95.0 } else { 105.0 });
            let chamber = Shape {
                path: RoundedRect::new(x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0, 22.0)
                    .to_path(0.1),
                fill: Some(self.chamber_color),
                stroke: Some(color),
                stroke_width: 4.0,
                dash: None,
                opacity: 1.0,
            };
            out.push(chamber.draw_with(&Look {
                transform: place
                    * Affine::translate(c.to_vec2())
                    * Affine::scale(scale)
                    * Affine::translate(-c.to_vec2()),
                opacity: shown * 0.92,
                ..Look::default()
            }));
        }

        for (name, (x, y)) in CHAMBERS {
            let at = place * Point::new(x, y + self.chamber_style.size as f64 * 0.35);
            out.push(line(
                at.x as f32,
                at.y as f32,
                self.chamber_style,
                self.text_color,
                TextAnchor::Middle,
                0.0,
                shown,
                name.into(),
            ));
        }

        // the vessels' names, beside them, as the line reaches them
        if self.labels {
            let names = [
                ("pulmonary artery", (-155.0, -220.0), TextAnchor::Start),
                ("pulmonary vein", (155.0, -160.0), TextAnchor::End),
                ("aorta", (338.0, 200.0), TextAnchor::Start),
                ("vena cava", (-338.0, 200.0), TextAnchor::End),
            ];
            for (name, (x, y), anchor) in names {
                let at = Point::new(x, y);
                let nearest = blood
                    .points
                    .iter()
                    .zip(&blood.at)
                    .min_by(|a, b| a.0.distance(at).total_cmp(&b.0.distance(at)))
                    .map_or(0.0, |(_, &s)| s);
                let start = DRAW.0 + (DRAW.1 - DRAW.0) * (nearest / blood.length()) as f32;
                let at = place * at;
                out.push(line(
                    at.x as f32,
                    at.y as f32,
                    self.label_style,
                    self.text_color,
                    anchor,
                    0.0,
                    progress(t, start, start + 0.4),
                    name.into(),
                ));
            }
        }

        fade(ui, out.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

fn polyline(points: &[Point]) -> BezPath {
    let mut path = BezPath::new();
    for (i, &p) in points.iter().enumerate() {
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blood_picks_up_oxygen_in_the_lungs_and_drops_it_in_the_body() {
        let blood = Loop::new();
        assert_eq!(blood.oxygen(0.0), 0.0);
        assert_eq!(blood.oxygen(blood.lungs.0), 0.0);
        assert_eq!(blood.oxygen(blood.lungs.1), 1.0);
        assert_eq!(
            blood.oxygen(f64::midpoint(blood.lungs.1, blood.body.0)),
            1.0
        );
        assert_eq!(blood.oxygen(blood.body.1), 0.0);
        assert!(blood.lungs.1 < blood.body.0);
    }

    #[test]
    fn the_loop_closes_at_the_right_atrium() {
        let blood = Loop::new();
        let start = blood.point(0.0);
        assert!((blood.point(blood.length()) - start).hypot() < 1e-9);
        assert!((start - Point::new(-70.0, -52.0)).hypot() < 1e-9);
        // a point half way along the first leg, from the atrium to the ventricle
        assert!((blood.point(54.5) - Point::new(-70.0, 2.5)).hypot() < 1e-9);
    }
}
