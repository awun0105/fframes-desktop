use fframes::{Color, Svgr};
use kurbo::Point;

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::draw_animated,
    explain::{Range, Space, ramp, sigmoid_length},
    geometry::Shape,
    layout::Anchor,
    shapes::{Circle, straight},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{
    field_lines::{Charge, Field, coulomb, line_shapes},
    label,
};

/// Manim-physics' `Charge` drawn: a disc at the charge with its sign, `radius` pixels for a
/// charge of 1 and growing with the square root of `|q|` (half to twice that), in a soft
/// glow with `glow`. Positive charges are `positive_color`, negative ones `negative_color`.
/// The data is a [`Charge`], which [`Field`] and [`ElectricField`] take as well.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ChargeGlow {
    pub charge: Charge,
    pub radius: f32,
    pub glow: bool,
    pub positive_color: Color,
    pub negative_color: Color,
    pub sign_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for ChargeGlow {
    fn themed(t: &Theme) -> Self {
        Self {
            charge: Charge::new(0.0, 0.0, 1.0),
            radius: 26.0,
            glow: true,
            positive_color: t.colors.highlight,
            negative_color: t.colors.accent,
            sign_color: t.colors.background,
            enter: None,
            exit: None,
        }
    }
}

impl Default for ChargeGlow {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Rings of the glow around a charge.
const GLOW_RINGS: usize = 8;

impl ChargeGlow {
    /// The disc's radius in pixels.
    pub fn disc_radius(&self) -> f32 {
        self.radius * (self.charge.q.abs().sqrt() as f32).clamp(0.5, 2.0)
    }

    fn color(&self) -> Color {
        if self.charge.q >= 0.0 {
            self.positive_color
        } else {
            self.negative_color
        }
    }
}

impl<'a> Component<'a> for ChargeGlow {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let at = ui.space.to_px(self.charge.x, self.charge.y);
        let r = self.disc_radius();
        let color = self.color();
        let mut shapes = Vec::new();
        if self.glow {
            // wide faint rings under narrow stronger ones
            for i in (1..=GLOW_RINGS).rev() {
                let f = i as f32 / GLOW_RINGS as f32;
                shapes.push(Shape {
                    opacity: 0.12 * (1.0 - f) + 0.02,
                    ..disc(ui, at, r * (1.0 + 1.6 * f), color)
                });
            }
        }
        shapes.push(disc(ui, at, r, color));
        let body = draw_animated(ui, &shapes, None, None, None);
        let sign = if self.charge.q >= 0.0 { "+" } else { "−" };
        let sign = label(ui, sign.to_owned(), at, Anchor::Center, self.sign_color);
        fade(
            ui,
            fframes::svgr!(<g>{body}{sign}</g>),
            self.enter.as_ref(),
            self.exit.as_ref(),
        )
    }
}

fn disc(ui: &Ui<'_, '_, '_>, at: Point, radius: f32, color: Color) -> Shape {
    Circle {
        cx: at.x as f32,
        cy: at.y as f32,
        radius,
        fill: Some(color),
        stroke: None,
        ..ui.defaults()
    }
    .shape()
}

/// An arrow on a grid point: `length` units along `(vx, vy)` from `(x, y)`, its tip at most
/// 14 pixels and half the arrow.
pub(super) fn grid_arrow(
    space: Space,
    (x, y): (f64, f64),
    (vx, vy): (f64, f64),
    length: f64,
    color: Color,
    width: f32,
) -> Option<Shape> {
    let norm = vx.hypot(vy);
    if !(norm.is_finite() && norm > 0.0) {
        return None;
    }
    let (from, to): (Point, Point) = (
        space.to_px(x, y),
        space.to_px(x + vx / norm * length, y + vy / norm * length),
    );
    let tip = 14.0f64.min((to - from).hypot() / 2.0);
    Some(Shape {
        path: straight(from, to, tip, tip * 12.0 / 14.0, false),
        fill: Some(color),
        stroke: Some(color),
        stroke_width: width,
        dash: None,
        opacity: 1.0,
    })
}

/// Manim-physics' `ElectricField`: an arrow at every grid point of `x_range` by `y_range`
/// (units of `ui.space`) along the field of `charges`, `length(|E|)` units long and coloured
/// by `|E|` from `colors[0]` at `min_magnitude` to the last colour at `max_magnitude`, like
/// [`ArrowVectorField`](crate::explain::ArrowVectorField). Grid points closer than
/// `clearance` units to a charge get no arrow. With `lines_per_charge` above 0 the field
/// lines of [`Field`] are drawn too, and with `show_charges` every charge as a
/// [`ChargeGlow`].
///
/// `charges` is a plain prop: compute it from `ui.seconds()` and the arrows and lines follow
/// the moving charges. The lines are traced again in every frame, which takes well under a
/// millisecond for a few charges. Everything fades with `enter` and `exit`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
#[allow(unpredictable_function_pointer_comparisons)] // equal props, not equal functions
pub struct ElectricField {
    pub charges: Vec<Charge>,
    pub x_range: Range,
    pub y_range: Range,
    pub arrows: bool,
    pub length: fn(f64) -> f64,
    pub colors: Vec<Color>,
    pub min_magnitude: f64,
    pub max_magnitude: f64,
    pub clearance: f64,
    pub lines_per_charge: usize,
    pub line_color: Color,
    pub show_charges: bool,
    pub glow: bool,
    pub charge_radius: f32,
    pub positive_color: Color,
    pub negative_color: Color,
    pub stroke_width: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for ElectricField {
    fn themed(t: &Theme) -> Self {
        Self {
            charges: Vec::new(),
            x_range: Range::new(-7.0, 7.0, 0.5),
            y_range: Range::new(-4.0, 4.0, 0.5),
            arrows: true,
            length: sigmoid_length,
            colors: vec![t.colors.muted, t.colors.accent, t.colors.highlight],
            min_magnitude: 0.0,
            max_magnitude: 2.0,
            clearance: 0.3,
            lines_per_charge: 0,
            line_color: t.colors.muted,
            show_charges: true,
            glow: true,
            charge_radius: 26.0,
            positive_color: t.colors.highlight,
            negative_color: t.colors.accent,
            stroke_width: t.stroke.thin,
            enter: None,
            exit: None,
        }
    }
}

impl Default for ElectricField {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl ElectricField {
    /// The field at `(x, y)`: the sum of `q r̂ / r²` over `charges`.
    pub fn at(&self, x: f64, y: f64) -> (f64, f64) {
        coulomb(&self.charges, x, y)
    }
}

impl<'a> Component<'a> for ElectricField {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let mut shapes = Vec::new();
        if self.lines_per_charge > 0 && !self.charges.is_empty() {
            let margin = 0.5;
            let bounds = (
                self.x_range.min - margin,
                self.y_range.min - margin,
                self.x_range.max + margin,
                self.y_range.max + margin,
            );
            let field = Field::new(self.charges.clone(), self.lines_per_charge, bounds);
            shapes.extend(line_shapes(
                ui,
                &field,
                1.0,
                true,
                self.line_color,
                self.stroke_width,
            ));
        }
        if self.arrows {
            let ys = self.y_range.ticks();
            for x in self.x_range.ticks() {
                for &y in &ys {
                    let near = self
                        .charges
                        .iter()
                        .any(|c| (x - c.x).hypot(y - c.y) < self.clearance);
                    if near {
                        continue;
                    }
                    let e = self.at(x, y);
                    let norm = e.0.hypot(e.1);
                    let color = ramp(&self.colors, self.min_magnitude, self.max_magnitude, norm);
                    shapes.extend(grid_arrow(
                        space,
                        (x, y),
                        e,
                        (self.length)(norm),
                        color,
                        self.stroke_width,
                    ));
                }
            }
        }
        let field = draw_animated(ui, &shapes, None, None, None);
        let charges: Vec<Svgr<'a>> = if self.show_charges {
            self.charges
                .iter()
                .map(|&charge| {
                    ui.show(ChargeGlow {
                        charge,
                        radius: self.charge_radius,
                        glow: self.glow,
                        positive_color: self.positive_color,
                        negative_color: self.negative_color,
                        ..ui.defaults()
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        fade(
            ui,
            fframes::svgr!(<g>{field}{charges}</g>),
            self.enter.as_ref(),
            self.exit.as_ref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_charge_falls_off_as_one_over_r_squared() {
        let field = ElectricField {
            charges: vec![Charge::new(1.0, -1.0, 3.0)],
            ..ElectricField::default()
        };
        let at = |r: f64| {
            let (ex, ey) = field.at(1.0 + r * 0.6, -1.0 + r * 0.8);
            ex.hypot(ey)
        };
        for r in [0.5, 1.0, 2.0, 4.0] {
            assert!((at(r) - 3.0 / (r * r)).abs() < 1e-9, "{r}");
        }
        // pointing away from a positive charge
        let (ex, ey) = field.at(1.6, -0.2);
        assert!(ex > 0.0 && ey > 0.0);
    }

    #[test]
    fn the_field_matches_the_traced_lines_field() {
        let charges = vec![Charge::new(-1.0, 0.0, 1.0), Charge::new(1.0, 0.5, -2.0)];
        let field = ElectricField {
            charges: charges.clone(),
            ..ElectricField::default()
        };
        let traced = Field::new(charges, 1, (-5.0, -5.0, 5.0, 5.0));
        assert_eq!(field.at(0.3, -0.7), traced.at(0.3, -0.7));
    }

    #[test]
    fn the_disc_grows_with_the_charge() {
        let small = ChargeGlow::default();
        let big = ChargeGlow {
            charge: Charge::new(0.0, 0.0, -4.0),
            ..ChargeGlow::default()
        };
        assert_eq!(big.disc_radius(), 2.0 * small.disc_radius());
    }
}
