use std::f64::consts::PI;

use fframes::{Color, Svgr};
use kurbo::{ParamCurve, Point, QuadBez, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    kits::label,
    label::Label,
    layout::Anchor,
    motion::{cubic_in_out, progress},
    shapes::{Circle, Dot},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{shells, symbol};

/// How the two atoms of a [`BondFormation`] bond.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BondType {
    /// The left atom gives electrons to the right one; they become ions.
    Ionic,
    /// The atoms move together and share pairs of electrons.
    Covalent,
}

/// The valence electrons of element `number`: those in its outer shell.
pub fn valence(number: u32) -> u32 {
    shells(number).last().copied().unwrap_or(0)
}

/// The electrons element `number` lacks for a full outer shell: two for hydrogen and helium,
/// eight for the others.
pub fn missing(number: u32) -> u32 {
    let full: u32 = if number <= 2 { 2 } else { 8 };
    full.saturating_sub(valence(number))
}

/// The electrons that move: given by the left atom (ionic), or pairs shared (covalent).
pub fn bond_electrons(kind: BondType, left: u32, right: u32) -> u32 {
    match kind {
        BondType::Ionic => valence(left).min(missing(right)),
        BondType::Covalent => missing(left).min(missing(right)),
    }
}

/// Positions on a ring of `radius` around `center` for `count` electrons, as in a Lewis
/// diagram: the first `facing` on the side at angle `toward`, the rest one per side on the
/// other three sides before they pair up, and what does not fit back on the facing side.
fn ring(center: Point, radius: f64, toward: f64, facing: u32, count: u32) -> Vec<Point> {
    let at = |angle: f64| center + Vec2::new(angle.cos(), -angle.sin()) * radius;
    // `n` electrons on one side: alone in the middle, paired, or spread when more
    let side = |angle: f64, n: u32| -> Vec<Point> {
        let gap = if n <= 2 { 0.4 } else { 0.22 };
        (0..n)
            .map(|i| at(angle + (f64::from(i) - f64::from(n - 1) / 2.0) * gap))
            .collect()
    };
    let facing = facing.min(count);
    let rest = count - facing;
    let mut per_side = [0u32; 3];
    for i in 0..rest.min(6) {
        per_side[(i % 3) as usize] += 1;
    }
    let overflow = rest.saturating_sub(6);
    let mut points = side(toward, facing + overflow);
    // the facing electrons first, the overflow after the other sides
    let extra = points.split_off(facing as usize);
    for (k, angle) in [toward + PI, toward + PI / 2.0, toward - PI / 2.0]
        .into_iter()
        .enumerate()
    {
        points.extend(side(angle, per_side[k]));
    }
    points.extend(extra);
    points
}
/// Two atoms forming a bond, with their valence electrons on rings around their symbols.
/// Ionic: from `at` over `duration` seconds the left atom's electrons arc over to the gaps of
/// the right one, the left ring fades when it is empty, and the charges appear. Covalent:
/// the atoms move together until their rings overlap and the shared electrons pair up
/// between them. `left` and `right` are atomic numbers, placed `distance` pixels apart
/// around `center` (units of `ui.space`). The rings draw with `enter`, the rest with
/// `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct BondFormation {
    pub kind: BondType,
    pub left: u32,
    pub right: u32,
    pub center: (f64, f64),
    pub distance: f32,
    pub radius: f32,
    pub at: f32,
    pub duration: f32,
    pub labels: bool,
    pub nucleus_color: Color,
    pub ring_color: Color,
    pub electron_color: Color,
    pub shared_color: Color,
    pub text_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for BondFormation {
    fn themed(t: &Theme) -> Self {
        Self {
            kind: BondType::Ionic,
            left: 11,
            right: 17,
            center: (0.0, 0.0),
            distance: 520.0,
            radius: 130.0,
            at: 1.0,
            duration: 1.5,
            labels: true,
            nucleus_color: t.colors.accent,
            ring_color: t.colors.muted,
            electron_color: t.colors.highlight,
            shared_color: Color::hex("#69db7c"),
            text_color: t.colors.text,
            stroke_width: t.stroke.thin,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for BondFormation {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

fn charge_text(n: u32, positive: bool) -> String {
    let sign = if positive { "+" } else { "−" };
    if n == 1 {
        sign.to_owned()
    } else {
        format!("{n}{sign}")
    }
}

impl<'a> Component<'a> for BondFormation {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let now = ui.seconds();
        let p = f64::from(cubic_in_out(progress(
            now,
            self.at,
            self.at + self.duration,
        )));
        let radius = f64::from(self.radius);
        let moved = bond_electrons(self.kind, self.left, self.right);
        let (va, vb) = (valence(self.left), valence(self.right));

        // covalent atoms close in until their rings overlap
        let half = f64::from(self.distance) / 2.0;
        let closest = radius * 0.8;
        let half_now = match self.kind {
            BondType::Ionic => half,
            BondType::Covalent => half + (closest - half) * p,
        };
        let a = middle - Vec2::new(half_now, 0.0);
        let b = middle + Vec2::new(half_now, 0.0);

        let shell = |c: Point, opacity: f32| {
            let mut s = Circle {
                cx: c.x as f32,
                cy: c.y as f32,
                radius: self.radius,
                fill: None,
                stroke: Some(self.ring_color),
                stroke_width: self.stroke_width,
                ..ui.defaults()
            }
            .shape();
            s.opacity = opacity;
            s
        };
        // an ionic donor that gave all its electrons loses its ring
        let left_ring = if self.kind == BondType::Ionic && moved == va {
            1.0 - p as f32
        } else {
            1.0
        };
        let rings = [shell(a, left_ring), shell(b, 1.0)];
        let rings = draw_animated(ui, &rings, self.enter.as_ref(), self.exit.as_ref(), None);

        let mut shapes = Vec::new();
        let nucleus = |c: Point| {
            Dot {
                cx: c.x as f32,
                cy: c.y as f32,
                radius: (self.radius * 0.36).max(10.0),
                fill: Some(self.nucleus_color),
                ..ui.defaults()
            }
            .shape()
        };
        shapes.push(nucleus(a));
        shapes.push(nucleus(b));
        let electron = |c: Point, color: Color| {
            Dot {
                cx: c.x as f32,
                cy: c.y as f32,
                radius: (self.radius * 0.075).max(4.0),
                fill: Some(color),
                ..ui.defaults()
            }
            .shape()
        };
        match self.kind {
            BondType::Ionic => {
                let from = ring(a, radius, 0.0, moved, va);
                // the right ring keeps its gaps facing the donor
                let gaps = ring(b, radius, PI, moved, vb + moved);
                for (i, &start) in from.iter().enumerate() {
                    let i = i as u32;
                    if i < moved {
                        let end = gaps[i as usize];
                        let lift = Vec2::new(0.0, -radius * (0.9 + 0.3 * f64::from(i)));
                        let arc = QuadBez::new(start, start.midpoint(end) + lift, end);
                        // one after the other within the transfer
                        let n = f64::from(moved.max(1));
                        let local = (p * n - f64::from(i)).clamp(0.0, 1.0);
                        shapes.push(electron(arc.eval(local), self.electron_color));
                    } else {
                        shapes.push(electron(start, self.electron_color));
                    }
                }
                for &rest in gaps.iter().skip(moved as usize) {
                    shapes.push(electron(rest, self.electron_color));
                }
            }
            BondType::Covalent => {
                for (c, toward, v, side) in [(a, 0.0, va, -1.0), (b, PI, vb, 1.0)] {
                    let spots = ring(c, radius, toward, moved, v);
                    for (i, &start) in spots.iter().enumerate() {
                        if (i as u32) < moved {
                            // pair i sits on the line between the nuclei's middle
                            let row = f64::from(i as u32) - f64::from(moved - 1) / 2.0;
                            let end = middle + Vec2::new(side * radius * 0.07, row * radius * 0.28);
                            let at = start.lerp(end, p);
                            // turning to the shared colour as the pair arrives
                            let color = fframes::animation::Animatable::apply_progress(
                                &self.electron_color,
                                &self.shared_color,
                                ((p - 0.7) / 0.3).clamp(0.0, 1.0) as f32,
                            );
                            shapes.push(electron(at, color));
                        } else {
                            // the lone electrons ride with their atom
                            shapes.push(electron(start, self.electron_color));
                        }
                    }
                }
            }
        }
        let mut parts = vec![draw_animated(ui, &shapes, None, None, None)];
        let mut style = ui.theme.typography.title;
        style.size = (self.radius * 0.3).round().max(8.0) as usize;
        for (c, n) in [(a, self.left), (b, self.right)] {
            parts.push(ui.show(Label {
                text: symbol(n).into(),
                x: c.x as f32,
                y: c.y as f32,
                direction: Anchor::Center,
                buff: 0.0,
                style,
                fill: ui.theme.colors.title,
                ..ui.defaults()
            }));
        }
        if self.labels && p > 0.0 {
            let shown = ((p - 0.7) / 0.3).clamp(0.0, 1.0) as f32;
            let mut texts = Vec::new();
            match self.kind {
                BondType::Ionic if moved > 0 => {
                    // at the nucleus' top right corner, as large as the symbol
                    let corner = Vec2::new(1.0, -1.0) * f64::from(self.radius * 0.36) * 0.75;
                    for (c, positive) in [(a, true), (b, false)] {
                        let at = c + corner;
                        texts.push(ui.show(Label {
                            text: charge_text(moved, positive).into(),
                            x: at.x as f32,
                            y: at.y as f32,
                            direction: Anchor::TopRight,
                            buff: 4.0,
                            style,
                            fill: self.text_color,
                            ..ui.defaults()
                        }));
                    }
                }
                BondType::Covalent if moved > 0 => {
                    let text = match moved {
                        1 => "a shared pair".to_owned(),
                        n => format!("{n} shared pairs"),
                    };
                    let under = middle + Vec2::new(0.0, radius * 1.25);
                    texts.push(label(ui, text, under, Anchor::Bottom, self.shared_color));
                }
                _ => {}
            }
            parts.push(fframes::svgr!(<g opacity={shown}>{texts}</g>));
        }
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{rings}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bonds_move_the_right_electrons() {
        assert_eq!(bond_electrons(BondType::Ionic, 11, 17), 1);
        assert_eq!(bond_electrons(BondType::Ionic, 12, 8), 2);
        assert_eq!(bond_electrons(BondType::Covalent, 1, 1), 1);
        assert_eq!(bond_electrons(BondType::Covalent, 8, 8), 2);
        assert_eq!(bond_electrons(BondType::Covalent, 7, 7), 3);
        assert_eq!(bond_electrons(BondType::Covalent, 1, 17), 1);
    }

    #[test]
    fn rings_place_every_electron_on_the_ring() {
        let c = Point::new(100.0, 100.0);
        let points = ring(c, 50.0, 0.0, 2, 7);
        assert_eq!(points.len(), 7);
        assert!(
            points
                .iter()
                .all(|p| ((*p - c).hypot() - 50.0).abs() < 1e-9)
        );
        // the facing electrons are on the right
        assert!(points[0].x > c.x && points[1].x > c.x);
        // no two share a spot, also on a full ring with its gap facing left
        for points in [points, ring(c, 50.0, PI, 1, 8)] {
            assert_eq!(points.len().max(7), points.len());
            for i in 0..points.len() {
                for j in i + 1..points.len() {
                    assert!((points[i] - points[j]).hypot() > 5.0, "{i} {j}");
                }
            }
        }
        assert!(ring(c, 50.0, PI, 1, 8)[0].x < c.x);
    }
}
