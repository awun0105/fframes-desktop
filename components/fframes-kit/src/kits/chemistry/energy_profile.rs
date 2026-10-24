use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{Create, EnterAnim, ExitAnim, draw_animated},
    geometry::Shape,
    kits::{label, short},
    layout::Anchor,
    shapes::{Arrow, DoubleArrow, Rectangle},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// A reaction's energy along its progress: a flat start at `reactants`, a hump up to
/// `transition`, a flat end at `products` (energies in `unit`), drawn `width` by `height`
/// pixels around `center` (units of `ui.space`) with an energy axis and a progress axis.
/// With `labels` the levels are named and arrows mark the activation energy and the
/// enthalpy change. A `catalyst` peak adds a lower, dashed path that draws at
/// `catalyst_at` seconds. The axes and the path draw with `enter`; the arrows and labels
/// with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct EnergyProfile<'a> {
    pub reactants: f64,
    pub transition: f64,
    pub products: f64,
    pub catalyst: Option<f64>,
    pub catalyst_at: f32,
    pub unit: Cow<'a, str>,
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub labels: bool,
    pub axis_color: Color,
    pub path_color: Color,
    pub catalyst_color: Color,
    pub arrow_color: Color,
    pub text_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for EnergyProfile<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            reactants: 50.0,
            transition: 150.0,
            products: 10.0,
            catalyst: None,
            catalyst_at: 0.0,
            unit: Cow::Borrowed("kJ/mol"),
            center: (0.0, 0.0),
            width: 1100.0,
            height: 620.0,
            labels: true,
            axis_color: t.colors.muted,
            path_color: t.colors.accent,
            catalyst_color: Color::hex("#69db7c"),
            arrow_color: t.colors.highlight,
            text_color: t.colors.text,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for EnergyProfile<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Where along the progress axis (0 to 1) the levels end and the peak sits.
const FLAT: f64 = 0.18;
const PEAK: f64 = 0.5;

impl EnergyProfile<'_> {
    /// The activation energy: the transition state over the reactants.
    pub fn activation(&self) -> f64 {
        self.transition - self.reactants
    }

    /// The enthalpy change: the products less the reactants, negative when it releases heat.
    pub fn enthalpy(&self) -> f64 {
        self.products - self.reactants
    }

    /// The path through `peak`, in pixels: `x` maps 0 to 1 across the plot, `y` an energy.
    fn path(&self, peak: f64, x: impl Fn(f64) -> f64, y: impl Fn(f64) -> f64) -> BezPath {
        let mut path = BezPath::new();
        let (r, p) = (self.reactants, self.products);
        path.move_to((x(0.0), y(r)));
        path.line_to((x(FLAT), y(r)));
        let rise = (PEAK - FLAT) / 2.0;
        path.curve_to(
            (x(FLAT + rise), y(r)),
            (x(PEAK - rise), y(peak)),
            (x(PEAK), y(peak)),
        );
        path.curve_to(
            (x(PEAK + rise), y(peak)),
            (x(1.0 - FLAT - rise), y(p)),
            (x(1.0 - FLAT), y(p)),
        );
        path.line_to((x(1.0), y(p)));
        path
    }
}

impl<'a> Component<'a> for EnergyProfile<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (w, h) = (f64::from(self.width), f64::from(self.height));
        let origin = middle + Vec2::new(-w / 2.0, h / 2.0);
        let top = [self.reactants, self.transition, self.products]
            .into_iter()
            .chain(self.catalyst)
            .fold(f64::NEG_INFINITY, f64::max);
        let bottom = [self.reactants, self.products]
            .into_iter()
            .fold(f64::INFINITY, f64::min)
            .min(0.0);
        let span = (top - bottom).max(1e-9);
        // the plot leaves room inside the axes: 10 % below the lowest, 15 % above the peak
        let inner_left = origin.x + w * 0.05;
        let inner_width = w * 0.9;
        let x = |f: f64| inner_left + f * inner_width;
        let y = |e: f64| origin.y - h * (0.1 + 0.75 * (e - bottom) / span);

        let stroke = |path: BezPath, color: Color, dash: Option<(f32, f32)>| Shape {
            path,
            fill: None,
            stroke: Some(color),
            stroke_width: self.stroke_width,
            dash,
            opacity: 1.0,
        };
        let axis = |to: Point| {
            Arrow {
                x1: origin.x as f32,
                y1: origin.y as f32,
                x2: to.x as f32,
                y2: to.y as f32,
                tip_length: 18.0,
                tip_width: 14.0,
                fill: Some(self.axis_color),
                stroke: Some(self.axis_color),
                stroke_width: ui.theme.stroke.thin,
                ..ui.defaults()
            }
            .shape()
        };
        let drawn = [
            axis(origin + Vec2::new(0.0, -h)),
            axis(origin + Vec2::new(w, 0.0)),
            stroke(self.path(self.transition, x, y), self.path_color, None),
        ];
        let drawn = draw_animated(ui, &drawn, self.enter.as_ref(), self.exit.as_ref(), None);

        let catalysed = self.catalyst.map_or_else(Svgr::empty, |peak| {
            let create: EnterAnim = Create {
                delay: self.catalyst_at,
                ..ui.defaults()
            }
            .into();
            let path = stroke(
                self.path(peak, x, y),
                self.catalyst_color,
                Some((14.0, 10.0)),
            );
            draw_animated(ui, &[path], Some(&create), self.exit.as_ref(), None)
        });

        let mut parts = Vec::new();
        if self.labels {
            let (r, t, p) = (self.reactants, self.transition, self.products);
            let tip = |a: Point, b: Point, color: Color| {
                DoubleArrow {
                    x1: a.x as f32,
                    y1: a.y as f32,
                    x2: b.x as f32,
                    y2: b.y as f32,
                    tip_length: 14.0,
                    tip_width: 12.0,
                    fill: Some(color),
                    stroke: Some(color),
                    stroke_width: ui.theme.stroke.thin,
                    ..ui.defaults()
                }
                .shape()
            };
            // activation energy over the reactants' level, at the peak
            let ea_x = x(PEAK);
            let mut arrows = vec![tip(
                Point::new(ea_x, y(r)),
                Point::new(ea_x, y(t)),
                self.arrow_color,
            )];
            // enthalpy change at the products' end
            let dh_x = x(1.0 - FLAT / 2.0);
            if (p - r).abs() > 1e-9 {
                arrows.push(tip(
                    Point::new(dh_x, y(r)),
                    Point::new(dh_x, y(p)),
                    self.arrow_color,
                ));
            }
            // the reactants' level carried over as a dashed guide
            let mut guide = BezPath::new();
            guide.move_to((x(FLAT), y(r)));
            guide.line_to((x(1.0), y(r)));
            arrows.push(stroke(guide, self.axis_color, Some((6.0, 8.0))));
            parts.push(draw_animated(ui, &arrows, None, None, None));

            let unit = &self.unit;
            let texts = [
                (
                    format!("Ea = {} {unit}", short(self.activation(), 1)),
                    Point::new(ea_x + 14.0, f64::midpoint(y(r), y(t))),
                    Anchor::Right,
                ),
                (
                    format!("∆H = {} {unit}", short(self.enthalpy(), 1)),
                    Point::new(dh_x + 14.0, f64::midpoint(y(r), y(p))),
                    Anchor::Right,
                ),
                (
                    "reactants".to_owned(),
                    Point::new(x(FLAT / 2.0), y(r) - 12.0),
                    Anchor::Top,
                ),
                (
                    "products".to_owned(),
                    Point::new(x(1.0 - FLAT / 2.0), y(p).max(y(r)) + 12.0),
                    Anchor::Bottom,
                ),
                (
                    "energy".to_owned(),
                    origin + Vec2::new(-12.0, -h),
                    Anchor::Left,
                ),
                (
                    "reaction progress".to_owned(),
                    origin + Vec2::new(w, 14.0),
                    Anchor::BottomLeft,
                ),
            ];
            for (i, (text, at, side)) in texts.into_iter().enumerate() {
                if text.starts_with("∆H") && (p - r).abs() <= 1e-9 {
                    continue;
                }
                // the two energies, right of their arrows, sit on a chip of the background so the
                // paths do not run through them
                if i < 2 {
                    let style = ui.theme.typography.caption;
                    let size = style.size as f32;
                    let width = crate::text::width(ui, style, &text)
                        .unwrap_or(0.55 * size * text.len() as f32);
                    let height = size * style.line_height;
                    let chip = Rectangle {
                        cx: at.x as f32 + width / 2.0,
                        cy: at.y as f32,
                        width: width + 16.0,
                        height: height + 6.0,
                        fill: Some(ui.theme.colors.background),
                        stroke: None,
                        ..ui.defaults()
                    };
                    parts.push(draw_animated(ui, &[chip.shape()], None, None, None));
                }
                parts.push(label(ui, text, at, side, self.text_color));
            }
        }
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{drawn}{catalysed}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energies_and_the_path() {
        let profile = EnergyProfile::default();
        assert_eq!(profile.activation(), 100.0);
        assert_eq!(profile.enthalpy(), -40.0);
        let path = profile.path(150.0, |f| f * 100.0, |e| -e);
        let bbox = kurbo::Shape::bounding_box(&path);
        // from the reactants' level over the peak, ending at the products'
        assert_eq!((bbox.x0, bbox.x1), (0.0, 100.0));
        assert!((bbox.y0 + 150.0).abs() < 1e-9 && (bbox.y1 + 10.0).abs() < 1e-9);
    }
}
