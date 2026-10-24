//! An enzyme splitting its substrate: lock and key, or induced fit.

use std::f64::consts::PI;

use fframes::{Color, Svgr};
use kurbo::{Affine, BezPath, Point, Rect, RoundedRect, Shape as _, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::{Look, Shape},
    kits::draw::{mix, stroke},
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// The enzyme's radius, in pixels at scale 1.
const RADIUS: f64 = 150.0;
/// How far the bottom of the active site lies below the top of the enzyme.
const SITE_DEPTH: f64 = 95.0;
/// Half the width of the substrate, and of the active site when it fits it.
const SUBSTRATE_HALF: f64 = 60.0;
const SUBSTRATE_HEIGHT: f64 = 80.0;
/// The space between the two halves of the substrate, which the bond bridges.
const BOND_GAP: f64 = 8.0;
/// Half the width of the open active site under induced fit.
const OPEN_HALF: f64 = 100.0;

/// The stages, in seconds after the delay: the substrate arrives, binds, its bond breaks and
/// the products leave.
const ARRIVE: (f32, f32) = (0.6, 1.8);
const BIND: (f32, f32) = (2.0, 2.8);
const BREAK: (f32, f32) = (3.4, 4.0);
const LEAVE: (f32, f32) = (4.8, 6.2);

/// Half the width of the active site's mouth at `t` seconds after the delay. With lock and key it
/// always fits the substrate; with induced fit it is open, closes round the substrate as it
/// binds, and opens again once the products leave.
fn site_half_width(induced: bool, t: f32) -> f64 {
    let fit = SUBSTRATE_HALF + 2.0;
    if !induced {
        return fit;
    }
    let close = f64::from(cubic_in_out(progress(t, BIND.0, BIND.1)));
    let reopen = f64::from(cubic_in_out(progress(t, LEAVE.0 + 0.3, LEAVE.0 + 0.9)));
    OPEN_HALF + (fit - OPEN_HALF) * close * (1.0 - reopen)
}

/// The enzyme's outline around the origin: a disc with the active site cut into its top,
/// `half` wide on each side and shaped like the substrate at its floor (a square corner on
/// the left, a round one on the right).
fn enzyme_outline(half: f64) -> BezPath {
    let rim = (RADIUS * RADIUS - half * half).sqrt();
    let floor = -RADIUS + SITE_DEPTH;
    // from the right rim, round the bottom, to the left rim
    let from = (-rim).atan2(half);
    let to = (-rim).atan2(-half) + 2.0 * PI;
    let mut path = BezPath::new();
    path.move_to((half, -rim));
    for i in 1..=96 {
        let a = from + (to - from) * f64::from(i) / 96.0;
        path.line_to((RADIUS * a.cos(), RADIUS * a.sin()));
    }
    // down the left wall, across the floor, up the right wall
    path.line_to((-half, floor - 10.0));
    path.quad_to((-half, floor), (-half + 10.0, floor));
    path.line_to((half - 30.0, floor));
    path.quad_to((half, floor), (half, floor - 30.0));
    path.close_path();
    path
}
/// An enzyme meeting its substrate: the substrate drifts in and settles into the active site
/// on top of the enzyme (`scale` times about 300 by 300 pixels, around `center` in units of
/// `ui.space`). With `induced` the site starts wider and closes round the substrate as it
/// binds (induced fit); without it the site already has the substrate's shape (lock and key).
/// Then the bond between the substrate's two halves flashes and breaks, the two products
/// leave, and an induced-fit site opens again. From `delay`: the substrate arrives over
/// 0.6..1.8 s, binds over 2..2.8 s, its bond glows from 3.4 s and breaks at 4.8 s, and the
/// products are gone by 6.2 s.
/// With `labels` the parts are named, and `captions` tells each step underneath.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct EnzymeFit {
    pub delay: f32,
    pub center: (f64, f64),
    pub scale: f64,
    pub induced: bool,
    pub labels: bool,
    pub captions: bool,
    pub enzyme_color: Color,
    pub site_color: Color,
    pub product_colors: [Color; 2],
    pub bond_color: Color,
    pub label_style: TextStyle,
    pub caption_style: TextStyle,
    pub text_color: Color,
    pub muted_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for EnzymeFit {
    fn themed(t: &Theme) -> Self {
        Self {
            delay: 0.0,
            center: (0.0, 0.0),
            scale: 1.0,
            induced: true,
            labels: true,
            captions: true,
            enzyme_color: Color::hex("#7950f2"),
            site_color: Color::hex("#b197fc"),
            product_colors: [Color::hex("#ffa94d"), Color::hex("#38d9a9")],
            bond_color: Color::hex("#ffe066"),
            label_style: t.typography.caption,
            caption_style: t.typography.body,
            text_color: t.colors.text,
            muted_color: t.colors.muted,
            enter: None,
            exit: None,
        }
    }
}

impl Default for EnzymeFit {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for EnzymeFit {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let place = Affine::translate(middle.to_vec2()) * Affine::scale(self.scale);
        let t = ui.seconds() - self.delay;
        let ease = |a: f32, b: f32| f64::from(cubic_in_out(progress(t, a, b)));
        let floor = -RADIUS + SITE_DEPTH;
        let mut out = Vec::new();

        // the enzyme, growing in
        let shown = ease(0.0, 0.6);
        let half = site_half_width(self.induced, t);
        let grow = Affine::scale(0.85 + 0.15 * shown);
        let enzyme = Shape {
            path: enzyme_outline(half).to_path(0.1),
            fill: Some(self.enzyme_color),
            stroke: None,
            stroke_width: 0.0,
            dash: None,
            opacity: 1.0,
        };
        out.push(enzyme.draw_with(&Look {
            transform: place * grow,
            opacity: shown as f32,
            ..Look::default()
        }));
        // the active site's floor, lit while something sits in it
        let lit = ease(ARRIVE.1 - 0.3, ARRIVE.1) * (1.0 - ease(LEAVE.0, LEAVE.0 + 0.4));
        let site = Shape {
            path: Rect::new(-half + 10.0, floor, half - 30.0, floor + 6.0).to_path(0.1),
            fill: Some(mix(self.enzyme_color, self.site_color, lit as f32)),
            stroke: None,
            stroke_width: 0.0,
            dash: None,
            opacity: 1.0,
        };
        out.push(site.draw_with(&Look {
            transform: place * grow,
            opacity: shown as f32,
            ..Look::default()
        }));

        // the substrate: in from the upper right, turning level above the site, then down
        // into it
        let bound = Point::new(0.0, floor - SUBSTRATE_HEIGHT / 2.0 - 1.0);
        let start = Point::new(380.0, -260.0);
        let above = Point::new(0.0, -230.0);
        let mid = f32::midpoint(ARRIVE.0, ARRIVE.1) + 0.2;
        let center = start
            .lerp(above, ease(ARRIVE.0, mid))
            .lerp(bound, ease(mid, ARRIVE.1));
        let turn = (1.0 - ease(ARRIVE.0, mid)) * 0.45;
        let arrived = ease(ARRIVE.0 - 0.4, ARRIVE.0);
        // after the break the halves leave, up and away from each other
        let gone = ease(LEAVE.0, LEAVE.1);
        let leaving = progress(t, LEAVE.0, LEAVE.0 + 0.1);
        let w = SUBSTRATE_HALF - BOND_GAP / 2.0;
        let h = SUBSTRATE_HEIGHT;
        let pieces = [
            // a block on the left, a round-ended piece on the right
            (
                RoundedRect::new(-w, -h / 2.0, 0.0, h / 2.0, 10.0).to_path(0.1),
                Vec2::new(-w / 2.0 - BOND_GAP / 2.0, 0.0),
                Vec2::new(-200.0, -260.0),
                -0.5,
            ),
            (
                RoundedRect::new(0.0, -h / 2.0, w, h / 2.0, (0.0, 30.0, 30.0, 0.0)).to_path(0.1),
                Vec2::new(w / 2.0 + BOND_GAP / 2.0, 0.0),
                Vec2::new(200.0, -260.0),
                0.5,
            ),
        ];
        let mut product_at = Vec::new();
        for (i, (path, offset, away, spin)) in pieces.into_iter().enumerate() {
            // each piece is drawn around its own middle
            let middle = Vec2::new(if i == 0 { w / 2.0 } else { -w / 2.0 }, 0.0);
            let at = center + (Affine::rotate(turn) * offset.to_point()).to_vec2() + away * gone;
            product_at.push(at);
            let piece = Shape {
                path: Affine::translate(middle) * path,
                fill: Some(self.product_colors[i]),
                stroke: None,
                stroke_width: 0.0,
                dash: None,
                opacity: 1.0,
            };
            out.push(piece.draw_with(&Look {
                transform: place
                    * Affine::translate(at.to_vec2())
                    * Affine::rotate(turn + spin * gone),
                opacity: (arrived * (1.0 - ease(LEAVE.1 - 0.4, LEAVE.1))) as f32,
                ..Look::default()
            }));
        }
        // the bond between the halves: it glows while the enzyme works on it, then breaks
        let glow = ease(BREAK.0, BREAK.1);
        let mut bond = BezPath::new();
        bond.move_to(center + (Affine::rotate(turn) * Point::new(-BOND_GAP, 0.0)).to_vec2());
        bond.line_to(center + (Affine::rotate(turn) * Point::new(BOND_GAP, 0.0)).to_vec2());
        out.push(
            stroke(
                place * bond,
                mix(Color::WHITE, self.bond_color, glow as f32),
                (10.0 + 6.0 * glow) as f32,
                (arrived * (1.0 - f64::from(leaving))) as f32,
            )
            .draw(),
        );

        let text = |out: &mut Vec<Svgr<'a>>,
                    at: Point,
                    anchor,
                    style,
                    color,
                    opacity: f64,
                    s: &'static str| {
            let at = place * at;
            out.push(line(
                at.x as f32,
                at.y as f32,
                style,
                color,
                anchor,
                0.0,
                opacity as f32,
                s.into(),
            ));
        };
        if self.labels {
            text(
                &mut out,
                Point::new(-RADIUS - 24.0, 60.0),
                TextAnchor::End,
                self.label_style,
                self.text_color,
                shown,
                "enzyme",
            );
            // the active site, with a leader to its right wall
            let site_shown = ease(0.3, 0.8);
            let rim = (RADIUS * RADIUS - half * half).sqrt();
            let mut leader = BezPath::new();
            leader.move_to((RADIUS + 34.0, -196.0));
            leader.line_to((half + 8.0, (floor - rim) / 2.0));
            out.push(
                stroke(
                    place * grow * leader,
                    self.muted_color,
                    2.0,
                    site_shown as f32,
                )
                .draw(),
            );
            text(
                &mut out,
                Point::new(RADIUS + 40.0, -190.0),
                TextAnchor::Start,
                self.label_style,
                self.text_color,
                site_shown,
                "active site",
            );
            // the substrate's name rides with it until it settles
            let named = arrived * (1.0 - ease(mid, mid + 0.3));
            text(
                &mut out,
                center + Vec2::new(0.0, -h / 2.0 - 18.0),
                TextAnchor::Middle,
                self.label_style,
                self.text_color,
                named,
                "substrate",
            );
            let products =
                ease(LEAVE.0 + 0.5, LEAVE.0 + 0.8) * (1.0 - ease(LEAVE.1 - 0.4, LEAVE.1));
            for at in product_at {
                text(
                    &mut out,
                    at + Vec2::new(0.0, h / 2.0 + 34.0),
                    TextAnchor::Middle,
                    self.label_style,
                    self.text_color,
                    products,
                    "product",
                );
            }
        }
        if self.captions {
            let steps: [(f32, &'static str); 4] = if self.induced {
                [
                    (0.0, "An enzyme and its substrate"),
                    (BIND.0, "The active site closes round the substrate"),
                    (BREAK.0, "The enzyme strains the bond until it breaks"),
                    (LEAVE.0, "The products leave and the site opens again"),
                ]
            } else {
                [
                    (0.0, "An enzyme and its substrate"),
                    (BIND.0, "The substrate fits the active site like a key"),
                    (BREAK.0, "The enzyme strains the bond until it breaks"),
                    (LEAVE.0, "The products leave, the enzyme is unchanged"),
                ]
            };
            let at = Point::new(0.0, RADIUS + 80.0);
            for (k, &(from, caption)) in steps.iter().enumerate() {
                let fade_in = if k == 0 {
                    shown
                } else {
                    ease(from, from + 0.3)
                };
                let fade_out = steps
                    .get(k + 1)
                    .map_or(0.0, |&(next, _)| ease(next - 0.3, next));
                text(
                    &mut out,
                    at,
                    TextAnchor::Middle,
                    self.caption_style,
                    self.text_color,
                    fade_in * (1.0 - fade_out),
                    caption,
                );
            }
            let model = if self.induced {
                "induced fit"
            } else {
                "lock and key"
            };
            text(
                &mut out,
                at + Vec2::new(0.0, 44.0),
                TextAnchor::Middle,
                self.label_style,
                self.muted_color,
                shown,
                model,
            );
        }
        fade(ui, out.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_substrate_always_fits_the_site_it_sits_in() {
        for induced in [false, true] {
            for i in 0..=80 {
                let t = i as f32 * 0.1;
                assert!(
                    site_half_width(induced, t) > SUBSTRATE_HALF,
                    "{induced} {t}"
                );
            }
        }
    }

    #[test]
    fn an_induced_site_closes_round_the_substrate_and_opens_again() {
        let fit = site_half_width(false, 0.0);
        assert_eq!(site_half_width(false, 3.0), fit);
        assert_eq!(site_half_width(true, ARRIVE.1), OPEN_HALF);
        assert_eq!(site_half_width(true, BREAK.0), fit);
        assert_eq!(site_half_width(true, LEAVE.1), OPEN_HALF);
    }

    #[test]
    fn the_site_is_cut_into_the_top_of_the_enzyme() {
        let outline = enzyme_outline(site_half_width(false, 0.0));
        let floor = -RADIUS + SITE_DEPTH;
        assert!(!outline.contains(Point::new(0.0, floor - 5.0)));
        assert!(outline.contains(Point::new(0.0, floor + 5.0)));
        assert!(outline.contains(Point::new(SUBSTRATE_HALF + 20.0, floor - 5.0)));
    }
}
