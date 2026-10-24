use std::f64::consts::TAU;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    explain::Space,
    geometry::Shape,
    layout::Anchor,
    motion::progress,
    shapes::{Circle, Dot},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{arrow, label};

/// A point charge at `(x, y)` (units of `ui.space`) of `q` (any unit; only the ratios
/// matter).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Charge {
    pub x: f64,
    pub y: f64,
    pub q: f64,
}

impl Charge {
    pub fn new(x: f64, y: f64, q: f64) -> Self {
        Self { x, y, q }
    }
}

/// Units between two points of a traced line.
const STEP: f64 = 0.02;
/// Units from a charge where a line starts or ends.
const START: f64 = 0.15;
const MAX_STEPS: usize = 4000;

/// The field lines of point charges, traced once: build it in the video's constructor and
/// pass it to [`FieldLines`]. `lines_per_charge` lines leave each positive charge per unit
/// of charge, evenly spaced, and follow the field until they reach a negative charge or leave
/// `bounds` (`(x0, y0, x1, y1)` in units). With only negative charges the lines come in from
/// `bounds` instead. Every line runs along the field from 0.15 units off its charge, with its
/// points 0.02 units apart; a line that reaches a charge ends on its centre.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub charges: Vec<Charge>,
    pub bounds: (f64, f64, f64, f64),
    lines: Vec<Vec<(f64, f64)>>,
}

impl Field {
    pub fn new(
        charges: Vec<Charge>,
        lines_per_charge: usize,
        bounds: (f64, f64, f64, f64),
    ) -> Self {
        let mut field = Self {
            charges,
            bounds,
            lines: Vec::new(),
        };
        let sources = if field.charges.iter().any(|c| c.q > 0.0) {
            1.0
        } else {
            -1.0
        };
        let mut lines = Vec::new();
        for charge in field.charges.iter().filter(|c| c.q * sources > 0.0) {
            let n = (charge.q.abs() * lines_per_charge as f64).round().max(1.0) as usize;
            for i in 0..n {
                let angle = TAU * i as f64 / n as f64;
                let from = (
                    charge.x + START * angle.cos(),
                    charge.y + START * angle.sin(),
                );
                let mut line = field.trace(from, sources);
                if sources < 0.0 {
                    line.reverse();
                }
                lines.push(line);
            }
        }
        field.lines = lines;
        field
    }

    /// The field at `(x, y)`: the sum of `q r̂ / r²`.
    pub fn at(&self, x: f64, y: f64) -> (f64, f64) {
        self.charges.iter().fold((0.0, 0.0), |(ex, ey), c| {
            let (dx, dy) = (x - c.x, y - c.y);
            let r2 = (dx * dx + dy * dy).max(1e-12);
            let s = c.q / (r2 * r2.sqrt());
            (ex + s * dx, ey + s * dy)
        })
    }

    /// The unit direction of the field times `sign`, or `None` where it vanishes.
    fn direction(&self, (x, y): (f64, f64), sign: f64) -> Option<(f64, f64)> {
        let (ex, ey) = self.at(x, y);
        let len = ex.hypot(ey);
        (len > 1e-12 && len.is_finite()).then(|| (sign * ex / len, sign * ey / len))
    }

    /// Points from `from` along the field (`sign` 1) or against it (-1), with the midpoint
    /// method, until a charge of the other sign or the edge of `bounds`.
    fn trace(&self, from: (f64, f64), sign: f64) -> Vec<(f64, f64)> {
        let (x0, y0, x1, y1) = self.bounds;
        let mut line = vec![from];
        let mut p = from;
        for _ in 0..MAX_STEPS {
            let Some((dx, dy)) = self.direction(p, sign) else {
                break;
            };
            let mid = (p.0 + dx * STEP / 2.0, p.1 + dy * STEP / 2.0);
            let Some((dx, dy)) = self.direction(mid, sign) else {
                break;
            };
            p = (p.0 + dx * STEP, p.1 + dy * STEP);
            if let Some(sink) = self
                .charges
                .iter()
                .find(|c| c.q * sign < 0.0 && (p.0 - c.x).hypot(p.1 - c.y) < START)
            {
                line.push((sink.x, sink.y));
                return line;
            }
            line.push(p);
            if p.0 < x0 || p.0 > x1 || p.1 < y0 || p.1 > y1 {
                break;
            }
        }
        line
    }

    /// The traced lines, each from its positive end.
    pub fn lines(&self) -> &[Vec<(f64, f64)>] {
        &self.lines
    }
}

/// The field lines of `field`, drawing on together over `draw` seconds from `delay` seconds
/// after the local start, each with an arrowhead at its middle. Then, with `test_charge`, a
/// small positive charge follows that line at `speed` units a second. The charges are drawn
/// as circles with their sign and draw with `enter`; the lines and the test charge fade with
/// `parts_enter`. Without `field` nothing is drawn.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct FieldLines<'a> {
    pub field: Option<&'a Field>,
    pub delay: f32,
    pub draw: f32,
    pub arrows: bool,
    pub test_charge: Option<usize>,
    pub speed: f64,
    pub charge_radius: f32,
    pub positive_color: Color,
    pub negative_color: Color,
    pub line_color: Color,
    pub test_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for FieldLines<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            field: None,
            delay: 0.0,
            draw: 1.5,
            arrows: true,
            test_charge: None,
            speed: 1.5,
            charge_radius: 30.0,
            positive_color: t.colors.highlight,
            negative_color: t.colors.accent,
            line_color: t.colors.muted,
            test_color: t.colors.title,
            stroke_width: t.stroke.thin,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for FieldLines<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The point `s` units along `line`, which has its points `STEP` apart (the last step and
/// the jump into a charge may be shorter).
fn along(line: &[(f64, f64)], s: f64) -> (f64, f64) {
    if line.is_empty() {
        return (0.0, 0.0);
    }
    let at = (s / STEP).max(0.0);
    let i = at.floor() as usize;
    if i + 1 >= line.len() {
        return line[line.len() - 1];
    }
    let f = at - i as f64;
    let (a, b) = (line[i], line[i + 1]);
    (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
}

fn path(space: Space, points: &[(f64, f64)]) -> BezPath {
    let mut path = BezPath::new();
    for (i, &(x, y)) in points.iter().enumerate() {
        let p = space.to_px(x, y);
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path
}

impl<'a> Component<'a> for FieldLines<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(field) = self.field else {
            return Svgr::empty();
        };
        let space = ui.space;
        let charges: Vec<Shape> = field
            .charges
            .iter()
            .map(|c| {
                let at = space.to_px(c.x, c.y);
                Circle {
                    cx: at.x as f32,
                    cy: at.y as f32,
                    radius: self.charge_radius,
                    fill: Some(if c.q >= 0.0 {
                        self.positive_color
                    } else {
                        self.negative_color
                    }),
                    stroke: None,
                    ..ui.defaults()
                }
                .shape()
            })
            .collect();
        let charges = draw_animated(ui, &charges, self.enter.as_ref(), self.exit.as_ref(), None);
        let mut signs = Vec::new();
        for c in &field.charges {
            let at = space.to_px(c.x, c.y);
            let sign = if c.q >= 0.0 { "+" } else { "−" };
            let fill = ui.theme.colors.background;
            signs.push(label(ui, sign.to_owned(), at, Anchor::Center, fill));
        }
        let signs = fade(ui, signs.into(), self.parts_enter.as_ref(), None);

        let now = ui.seconds();
        let drawn = f64::from(progress(now, self.delay, self.delay + self.draw.max(1e-3)));
        let mut shapes = Vec::new();
        if drawn > 0.0 {
            for line in field.lines() {
                let end = ((line.len() as f64 * drawn).ceil() as usize).clamp(2, line.len().max(2));
                let shown = &line[..end.min(line.len())];
                shapes.push(Shape {
                    path: path(space, shown),
                    fill: None,
                    stroke: Some(self.line_color),
                    stroke_width: self.stroke_width,
                    dash: None,
                    opacity: 1.0,
                });
                let mid = line.len() / 2;
                if self.arrows && line.len() > 24 && end > mid + 3 {
                    let a = space.to_px(line[mid - 10].0, line[mid - 10].1);
                    let b = space.to_px(line[mid + 3].0, line[mid + 3].1);
                    shapes.push(arrow(ui, a, b, self.line_color, self.stroke_width));
                }
            }
        }
        let moving = f64::from(now - self.delay - self.draw);
        if let Some(line) = self.test_charge.and_then(|i| field.lines().get(i))
            && moving > 0.0
        {
            let (x, y) = along(line, moving * self.speed);
            let at: Point = space.to_px(x, y);
            shapes.push(
                Dot {
                    cx: at.x as f32,
                    cy: at.y as f32,
                    radius: self.charge_radius * 0.4,
                    fill: Some(self.test_color),
                    ..ui.defaults()
                }
                .shape(),
            );
        }
        let parts = draw_animated(ui, &shapes, None, None, None);
        let parts = fade(
            ui,
            parts,
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{parts}{charges}{signs}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOX: (f64, f64, f64, f64) = (-5.0, -5.0, 5.0, 5.0);

    #[test]
    fn a_lone_charge_has_straight_radial_lines() {
        let field = Field::new(vec![Charge::new(0.0, 0.0, 2.0)], 4, BOX);
        assert_eq!(field.lines().len(), 8);
        for line in field.lines() {
            let (x, y) = line[line.len() / 2];
            let (ex, ey) = *line.last().unwrap();
            // on the ray from the charge, and out to the box
            assert!((x * ey - y * ex).abs() < 1e-6);
            assert!(ex.abs().max(ey.abs()) > 5.0);
        }
    }

    #[test]
    fn dipole_lines_end_on_the_negative_charge() {
        let field = Field::new(
            vec![Charge::new(-1.0, 0.0, 1.0), Charge::new(1.0, 0.0, -1.0)],
            8,
            BOX,
        );
        // the line that leaves toward the other charge ends on it
        let line = &field.lines()[0];
        assert!(((line[0].0 + 1.0).hypot(line[0].1) - START).abs() < 1e-12);
        assert_eq!(*line.last().unwrap(), (1.0, 0.0));
        // and the field points from + to - between them
        assert!(field.at(0.0, 0.0).0 > 0.0);
    }

    #[test]
    fn negative_charges_alone_get_lines_coming_in() {
        let field = Field::new(vec![Charge::new(0.0, 0.0, -1.0)], 6, BOX);
        assert_eq!(field.lines().len(), 6);
        for line in field.lines() {
            let (x, y) = *line.last().unwrap();
            assert!((x.hypot(y) - START).abs() < 1e-12);
            assert!(line[0].0.abs().max(line[0].1.abs()) > 5.0);
        }
    }

    #[test]
    fn a_test_charge_walks_along_its_line() {
        let line = vec![(0.0, 0.0), (STEP, 0.0), (2.0 * STEP, 0.0)];
        assert_eq!(along(&line, STEP * 1.5), (STEP * 1.5, 0.0));
        assert_eq!(along(&line, 9.0), (2.0 * STEP, 0.0));
        assert_eq!(along(&line, -1.0), (0.0, 0.0));
    }
}
