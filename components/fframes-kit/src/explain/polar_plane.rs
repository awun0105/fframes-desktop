//! Manim's `PolarPlane`.

use std::{borrow::Cow, f64::consts::TAU};

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Point, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{Space, axes::number};

/// How the azimuths of a [`PolarPlane`] are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum AzimuthUnits {
    /// `π/2`, `π`, `3π/2`.
    #[default]
    PiRadians,
    /// `τ/4`, `τ/2`.
    TauRadians,
    /// `90°`.
    Degrees,
    /// `100g`.
    Gradians,
    /// No azimuth numbers.
    None,
}

/// Which way the azimuth grows from `azimuth_offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum AzimuthDirection {
    /// Counterclockwise, as in math.
    #[default]
    Ccw,
    Cw,
}

/// A polar grid (Manim `PolarPlane`): circles every `radius_step` up to `radius_max` and
/// `azimuth_step` spokes, `size` units across, with the radii written along the first
/// spoke and the azimuths around the outside. Points are `(r, θ)`, θ in radians.
///
/// The `PiRadians` numbers need a font with `π` and the `TauRadians` numbers one with `τ`
/// (DM Sans has `π` but no `τ`; `inspect` reports the fallback).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct PolarPlane {
    /// The middle, in units of `ui.space`.
    pub center: (f64, f64),
    /// The diameter of the outer circle, in units of `ui.space`.
    pub size: f64,
    pub radius_max: f64,
    pub radius_step: f64,
    /// Number of spokes.
    pub azimuth_step: usize,
    pub azimuth_units: AzimuthUnits,
    /// The angle of θ = 0 from the positive x-axis, in radians.
    pub azimuth_offset: f64,
    pub azimuth_direction: AzimuthDirection,
    pub line_color: Color,
    pub line_width: f32,
    pub line_opacity: f32,
    /// The circle at `radius_max` and the spoke at θ = 0.
    pub axis_color: Color,
    pub radius_numbers: bool,
    pub decimals: usize,
    pub number_style: TextStyle,
    pub number_fill: Color,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    /// How the numbers appear and go; the grid uses `enter` and `exit`.
    pub labels_enter: Option<FadeIn>,
    pub labels_exit: Option<FadeOut>,
}

impl Themed for PolarPlane {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.0, 0.0),
            size: 7.0,
            radius_max: 4.0,
            radius_step: 1.0,
            azimuth_step: 12,
            azimuth_units: AzimuthUnits::PiRadians,
            azimuth_offset: 0.0,
            azimuth_direction: AzimuthDirection::Ccw,
            line_color: t.colors.accent,
            line_width: t.stroke.thin,
            line_opacity: 0.45,
            axis_color: t.colors.text,
            radius_numbers: true,
            decimals: 0,
            number_style: t.typography.caption,
            number_fill: t.colors.text,
            enter: None,
            exit: None,
            labels_enter: None,
            labels_exit: None,
        }
    }
}

impl Default for PolarPlane {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// `p / q` in lowest terms.
fn reduce(p: usize, q: usize) -> (usize, usize) {
    let gcd = |mut a: usize, mut b: usize| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a.max(1)
    };
    let g = gcd(p, q);
    (p / g, q / g)
}

/// `p/q` of `symbol`: `0`, `π`, `3π`, `π/2`, `3π/2`.
fn fraction_of(p: usize, q: usize, symbol: &str) -> String {
    let (p, q) = reduce(p, q);
    let top = match p {
        0 => return "0".to_owned(),
        1 => symbol.to_owned(),
        p => format!("{p}{symbol}"),
    };
    if q == 1 { top } else { format!("{top}/{q}") }
}

impl PolarPlane {
    /// Units of `ui.space` per unit of radius.
    fn scale(&self) -> f64 {
        if self.radius_max == 0.0 {
            0.0
        } else {
            self.size / 2.0 / self.radius_max
        }
    }

    /// The screen angle, counterclockwise from the positive x-axis, of the azimuth `theta`.
    fn screen_angle(&self, theta: f64) -> f64 {
        match self.azimuth_direction {
            AzimuthDirection::Ccw => self.azimuth_offset + theta,
            AzimuthDirection::Cw => self.azimuth_offset - theta,
        }
    }

    /// The pixel of the polar point `(r, θ)` in `space`.
    pub fn pr2pt_in(&self, space: Space, (r, theta): (f64, f64)) -> Point {
        let (a, d) = (self.screen_angle(theta), r * self.scale());
        space.to_px(self.center.0 + d * a.cos(), self.center.1 + d * a.sin())
    }

    /// The pixel of the polar point `(r, θ)` (Manim `polar_to_point`, `pr2pt`).
    pub fn pr2pt(&self, ui: &Ui<'_, '_, '_>, rt: (f64, f64)) -> (f32, f32) {
        let p = self.pr2pt_in(ui.space, rt);
        (p.x as f32, p.y as f32)
    }

    /// The polar point `(r, θ)` at a pixel in `space`, θ in `0..τ`.
    pub fn pt2pr_in(&self, space: Space, p: Point) -> (f64, f64) {
        let (sx, sy) = space.from_px(p);
        let (dx, dy) = (sx - self.center.0, sy - self.center.1);
        let scale = self.scale();
        let r = if scale == 0.0 {
            0.0
        } else {
            dx.hypot(dy) / scale
        };
        let a = dy.atan2(dx) - self.azimuth_offset;
        let theta = match self.azimuth_direction {
            AzimuthDirection::Ccw => a,
            AzimuthDirection::Cw => -a,
        };
        (r, theta.rem_euclid(TAU))
    }

    /// The polar point at a pixel (Manim `point_to_polar`, `pt2pr`).
    pub fn pt2pr(&self, ui: &Ui<'_, '_, '_>, x: f32, y: f32) -> (f64, f64) {
        self.pt2pr_in(ui.space, Point::new(f64::from(x), f64::from(y)))
    }

    /// The azimuth of spoke `k` written in `azimuth_units`, `None` without numbers.
    pub fn azimuth_label(&self, k: usize) -> Option<String> {
        let n = self.azimuth_step.max(1);
        let turn = k as f64 / n as f64;
        Some(match self.azimuth_units {
            AzimuthUnits::PiRadians => fraction_of(2 * k, n, "π"),
            AzimuthUnits::TauRadians => fraction_of(k, n, "τ"),
            AzimuthUnits::Degrees => format!("{}°", number(turn * 360.0, self.decimals)),
            AzimuthUnits::Gradians => format!("{}g", number(turn * 400.0, self.decimals)),
            AzimuthUnits::None => return None,
        })
    }

    /// The radii of the circles, from `radius_step` up to `radius_max`.
    fn radii(&self) -> Vec<f64> {
        if self.radius_step.is_nan() || self.radius_step <= 0.0 {
            return vec![self.radius_max];
        }
        let n = ((self.radius_max / self.radius_step + 1e-9).floor() as usize).min(999);
        (1..=n).map(|i| i as f64 * self.radius_step).collect()
    }

    /// The grid (circles and spokes) and its outer circle and first spoke.
    fn shapes(&self, space: Space) -> [Shape; 2] {
        let center = self.pr2pt_in(space, (0.0, 0.0));
        let px = |r: f64| (self.pr2pt_in(space, (r, 0.0)) - center).hypot();
        let mut grid = BezPath::new();
        for r in self.radii() {
            if (r - self.radius_max).abs() > 1e-9 {
                grid.extend(Circle::new(center, px(r)).path_elements(0.1));
            }
        }
        let n = self.azimuth_step.max(1);
        for k in 1..n {
            let theta = TAU * k as f64 / n as f64;
            grid.move_to(center);
            grid.line_to(self.pr2pt_in(space, (self.radius_max, theta)));
        }
        let mut axis = BezPath::new();
        axis.extend(Circle::new(center, px(self.radius_max)).path_elements(0.1));
        axis.move_to(center);
        axis.line_to(self.pr2pt_in(space, (self.radius_max, 0.0)));
        let shape = |path, color, opacity| Shape {
            path,
            fill: None,
            stroke: Some(color),
            stroke_width: self.line_width,
            dash: None,
            opacity,
        };
        [
            shape(grid, self.line_color, self.line_opacity),
            shape(axis, self.axis_color, 1.0),
        ]
    }
}

impl<'a> Component<'a> for PolarPlane {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let drawn = draw_animated(
            ui,
            &self.shapes(ui.space),
            self.enter.as_ref(),
            self.exit.as_ref(),
            None,
        );
        let size = self.number_style.size as f32;
        let mut texts: Vec<Svgr<'a>> = Vec::new();
        if self.radius_numbers {
            for r in self.radii() {
                // under the first spoke, just past the circle
                let p = self.pr2pt_in(ui.space, (r, 0.0));
                texts.push(line(
                    p.x as f32 + size * 0.45,
                    p.y as f32 + size * 1.2,
                    self.number_style,
                    self.number_fill,
                    TextAnchor::Start,
                    0.0,
                    1.0,
                    Cow::Owned(number(r, self.decimals)),
                ));
            }
        }
        let center = self.pr2pt_in(ui.space, (0.0, 0.0));
        for k in 0..self.azimuth_step.max(1) {
            let Some(text) = self.azimuth_label(k) else {
                break;
            };
            let theta = TAU * k as f64 / self.azimuth_step.max(1) as f64;
            let edge = self.pr2pt_in(ui.space, (self.radius_max, theta));
            let out = (edge - center).normalize() * f64::from(size) * 1.4;
            if !out.is_finite() {
                continue;
            }
            let at = edge + out;
            texts.push(line(
                at.x as f32,
                at.y as f32 + size * 0.36,
                self.number_style,
                self.number_fill,
                TextAnchor::Middle,
                0.0,
                1.0,
                Cow::Owned(text),
            ));
        }
        let texts = fade(
            ui,
            texts.into_iter().collect(),
            self.labels_enter.as_ref(),
            self.labels_exit.as_ref(),
        );
        fframes::svgr!(<g>{drawn}{texts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use super::*;

    #[test]
    fn polar_points_round_trip() {
        let planes = [
            PolarPlane::default(),
            PolarPlane {
                center: (1.0, -0.5),
                azimuth_offset: 0.7,
                azimuth_direction: AzimuthDirection::Cw,
                ..PolarPlane::default()
            },
        ];
        for plane in planes {
            for (r, theta) in [(1.0, 0.3), (3.5, 2.0), (2.0, 5.9), (4.0, PI)] {
                let p = plane.pr2pt_in(Space::FULL_HD, (r, theta));
                let (br, bt) = plane.pt2pr_in(Space::FULL_HD, p);
                assert!(
                    (br - r).abs() < 1e-9 && (bt - theta).abs() < 1e-9,
                    "{r} {theta}"
                );
            }
        }
        // radius 4 is half of 7 units, straight up at π/2
        let p = PolarPlane::default().pr2pt_in(Space::FULL_HD, (4.0, FRAC_PI_2));
        assert!((p.x - 960.0).abs() < 1e-9 && (p.y - (540.0 - 3.5 * 135.0)).abs() < 1e-9);
    }

    #[test]
    fn azimuths_are_written_in_their_units() {
        let plane = PolarPlane::default();
        let pi: Vec<String> = (0..12).filter_map(|k| plane.azimuth_label(k)).collect();
        assert_eq!(
            pi[..7],
            ["0", "π/6", "π/3", "π/2", "2π/3", "5π/6", "π"].map(String::from)
        );
        assert_eq!(pi[9], "3π/2");
        let tau = PolarPlane {
            azimuth_units: AzimuthUnits::TauRadians,
            azimuth_step: 4,
            ..PolarPlane::default()
        };
        assert_eq!(tau.azimuth_label(1).as_deref(), Some("τ/4"));
        assert_eq!(tau.azimuth_label(2).as_deref(), Some("τ/2"));
        let degrees = PolarPlane {
            azimuth_units: AzimuthUnits::Degrees,
            ..PolarPlane::default()
        };
        assert_eq!(degrees.azimuth_label(3).as_deref(), Some("90°"));
        let none = PolarPlane {
            azimuth_units: AzimuthUnits::None,
            ..PolarPlane::default()
        };
        assert_eq!(none.azimuth_label(3), None);
        assert_eq!(plane.radii(), [1.0, 2.0, 3.0, 4.0]);
    }
}
