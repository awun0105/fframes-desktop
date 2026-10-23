use fframes::{Color, Svgr};

use crate::{
    anim::{FadeIn, fade},
    animate::{EnterAnim, draw_animated},
    label::Label,
    layout::Anchor,
    shapes::Polygon,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::short;

/// The Pythagorean theorem: a right triangle with legs `a` (along x) and `b` (up), and a
/// square on each side, outward. `corner` is the right angle in units of `ui.space`. Each
/// square draws with its own `squares_enter` (stagger them with delays) and, with `labels`,
/// shows its area at its center (`a² = 9`, `b² = 16`, `c² = 25`).
#[derive(Debug, Clone)]
pub struct Pythagoras {
    pub a: f64,
    pub b: f64,
    pub corner: (f64, f64),
    pub labels: bool,
    pub triangle_color: Color,
    /// The squares on `a`, `b` and the hypotenuse.
    pub colors: [Color; 3],
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub squares_enter: [Option<EnterAnim>; 3],
    pub labels_enter: Option<FadeIn>,
}

impl Themed for Pythagoras {
    fn themed(t: &Theme) -> Self {
        Self {
            a: 3.0,
            b: 4.0,
            corner: (0.0, 0.0),
            labels: true,
            triangle_color: t.colors.title,
            colors: [t.colors.accent, t.colors.highlight, t.colors.caption],
            stroke_width: t.stroke.regular,
            enter: None,
            squares_enter: [None, None, None],
            labels_enter: None,
        }
    }
}

impl Default for Pythagoras {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

type Quad = [(f64, f64); 4];

impl Pythagoras {
    /// The triangle's corners: the right angle, the end of `a`, the end of `b` (units).
    fn triangle(&self) -> [(f64, f64); 3] {
        let (x, y) = self.corner;
        [(x, y), (x + self.a, y), (x, y + self.b)]
    }

    /// The squares on `a` (below), `b` (left) and the hypotenuse (outward), in units.
    fn squares(&self) -> [Quad; 3] {
        let [c, a, b] = self.triangle();
        let (sa, sb) = (self.a, self.b);
        [
            [c, a, (a.0, a.1 - sa), (c.0, c.1 - sa)],
            [c, b, (b.0 - sb, b.1), (c.0 - sb, c.1)],
            // AB turned a quarter clockwise points away from the right angle: (b, a)
            [a, b, (b.0 + sb, b.1 + sa), (a.0 + sb, a.1 + sa)],
        ]
    }

    /// The squares' areas: `a²`, `b²` and `c²`.
    pub fn areas(&self) -> [f64; 3] {
        let c2 = self.a.mul_add(self.a, self.b * self.b);
        [self.a * self.a, self.b * self.b, c2]
    }
}

impl<'a> Component<'a> for Pythagoras {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let px = |p: (f64, f64)| {
            let q = space.to_px(p.0, p.1);
            (q.x as f32, q.y as f32)
        };
        let polygon = |ui: &Ui<'_, '_, '_>, points: Vec<(f32, f32)>, color: Color| {
            Polygon {
                points,
                stroke: Some(color),
                fill: Some(color.with_alpha(64)),
                stroke_width: self.stroke_width,
                ..ui.defaults()
            }
            .shape()
        };
        let squares = self.squares();
        let mut drawn = Vec::new();
        for ((square, &color), enter) in squares.iter().zip(&self.colors).zip(&self.squares_enter) {
            let shape = polygon(ui, square.iter().map(|&p| px(p)).collect(), color);
            drawn.push(draw_animated(ui, &[shape], enter.as_ref(), None, None));
        }
        let mut triangle = polygon(ui, self.triangle().map(px).to_vec(), self.triangle_color);
        triangle.fill = Some(ui.theme.colors.background);
        drawn.push(draw_animated(
            ui,
            &[triangle],
            self.enter.as_ref(),
            None,
            None,
        ));

        let labels = if self.labels {
            let names = ["a²", "b²", "c²"];
            let labels: Vec<Svgr> = squares
                .iter()
                .zip(self.areas())
                .zip(names)
                .map(|((square, area), name)| {
                    let (x, y) = px(center(square));
                    ui.show(Label {
                        text: format!("{name} = {}", short(area, 2)).into(),
                        x,
                        y,
                        direction: Anchor::Center,
                        ..ui.defaults()
                    })
                })
                .collect();
            fade(ui, labels.into(), self.labels_enter.as_ref(), None)
        } else {
            Svgr::empty()
        };
        let drawn: Svgr = drawn.into();
        fframes::svgr!(<g>{drawn}{labels}</g>)
    }
}

fn center(q: &Quad) -> (f64, f64) {
    (f64::midpoint(q[0].0, q[2].0), f64::midpoint(q[0].1, q[2].1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(q: &Quad) -> f64 {
        // the shoelace formula
        let twice: f64 = (0..4)
            .map(|i| {
                let (p, n) = (q[i], q[(i + 1) % 4]);
                p.0 * n.1 - n.0 * p.1
            })
            .sum();
        twice.abs() / 2.0
    }

    #[test]
    fn the_squares_add_up() {
        let p = Pythagoras {
            corner: (1.0, -1.0),
            ..Pythagoras::default()
        };
        let squares = p.squares();
        let areas = squares.map(|q| area(&q));
        assert!((areas[0] - 9.0).abs() < 1e-9);
        assert!((areas[1] - 16.0).abs() < 1e-9);
        assert!((areas[2] - areas[0] - areas[1]).abs() < 1e-9);
        assert_eq!(p.areas(), [9.0, 16.0, 25.0]);
        // every square lies outside the triangle: its center is on the far side of its edge
        let [c, a, b] = p.triangle();
        assert!(center(&squares[0]).1 < c.1);
        assert!(center(&squares[1]).0 < c.0);
        let hyp = center(&squares[2]);
        assert!(hyp.0 > a.0.min(b.0) && hyp.1 > b.1.min(a.1) && hyp.0 + hyp.1 > a.0 + a.1);
    }
}
