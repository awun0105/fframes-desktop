use fframes::{Color, Svgr};
use kurbo::Vec2;

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    kits::short,
    layout::Anchor,
    motion::{cubic_in_out, progress},
    shapes::Rectangle,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{arrow, label};

/// One force on a [`FreeBody`]: its name and its components in newtons (`y` up), drawn in
/// `color` or the figure's force colour.
#[derive(Debug, Clone, PartialEq)]
pub struct Force<'a> {
    pub name: &'a str,
    pub x: f64,
    pub y: f64,
    pub color: Option<Color>,
}

impl<'a> Force<'a> {
    pub fn new(name: &'a str, x: f64, y: f64) -> Self {
        Self {
            name,
            x,
            y,
            color: None,
        }
    }

    /// Newtons.
    pub fn magnitude(&self) -> f64 {
        self.x.hypot(self.y)
    }
}

/// A free-body diagram: a block at `center` (units of `ui.space`) with its `forces` drawn
/// from its middle, `scale` units per newton. The arrows grow one after another from `delay`
/// seconds after the local start, each over `grow` seconds and `stagger` seconds after the
/// one before, with their names (and magnitudes, with `magnitudes`) at the tips. With `net`
/// the sum of the forces follows as a dashed arrow, or `net = 0` under the block when they
/// balance; `net_name` names it. The block draws with `enter`, the arrows and labels with
/// `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct FreeBody<'a> {
    pub forces: Vec<Force<'a>>,
    pub center: (f64, f64),
    pub scale: f64,
    pub size: f32,
    pub delay: f32,
    pub grow: f32,
    pub stagger: f32,
    pub net: bool,
    /// The name in the net force's label, "net = 4 N".
    pub net_name: &'a str,
    pub labels: bool,
    pub magnitudes: bool,
    pub block_color: Color,
    pub force_color: Color,
    pub net_color: Color,
    pub text_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for FreeBody<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            forces: Vec::new(),
            center: (0.0, 0.0),
            scale: 0.5,
            size: 160.0,
            delay: 0.0,
            grow: 0.6,
            stagger: 0.5,
            net: true,
            net_name: "net",
            labels: true,
            magnitudes: true,
            block_color: t.colors.muted,
            force_color: t.colors.accent,
            net_color: t.colors.highlight,
            text_color: t.colors.text,
            stroke_width: t.stroke.regular,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for FreeBody<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// Where a label goes at the tip of a vector `d` pixels long (`y` down): past the tip along
/// the larger component.
fn beyond(d: Vec2) -> Anchor {
    if d.x.abs() >= d.y.abs() {
        if d.x >= 0.0 {
            Anchor::Right
        } else {
            Anchor::Left
        }
    } else if d.y >= 0.0 {
        Anchor::Bottom
    } else {
        Anchor::Top
    }
}

/// Where the label of a vector `d` pixels long (`y` down) goes beside its tip: across the
/// larger component, so it stays off the line of forces that point the same way.
fn beside(d: Vec2) -> Anchor {
    if d.x.abs() >= d.y.abs() {
        Anchor::Top
    } else if d.x >= 0.0 {
        Anchor::Right
    } else {
        Anchor::Left
    }
}

impl FreeBody<'_> {
    /// The sum of the forces in newtons.
    pub fn net_force(&self) -> (f64, f64) {
        self.forces
            .iter()
            .fold((0.0, 0.0), |(x, y), f| (x + f.x, y + f.y))
    }

    /// Seconds after the local start when force `i` starts to grow; `forces.len()` gives the
    /// start of the net force.
    pub fn start_of(&self, i: usize) -> f32 {
        if i < self.forces.len() {
            self.delay + i as f32 * self.stagger
        } else {
            let last = self.forces.len().saturating_sub(1) as f32;
            self.delay + last * self.stagger + self.grow + self.stagger.min(0.3)
        }
    }

    /// How far a vector starting at `start` has grown, eased.
    fn grown(&self, now: f32, start: f32) -> f64 {
        f64::from(cubic_in_out(progress(
            now,
            start,
            start + self.grow.max(1e-3),
        )))
    }
}

impl<'a> Component<'a> for FreeBody<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let space = ui.space;
        let middle = space.to_px(self.center.0, self.center.1);
        let block = Rectangle {
            cx: middle.x as f32,
            cy: middle.y as f32,
            width: self.size,
            height: self.size,
            fill: Some(self.block_color),
            stroke: None,
            ..ui.defaults()
        };
        let block = draw_animated(
            ui,
            &[block.shape()],
            self.enter.as_ref(),
            self.exit.as_ref(),
            None,
        );

        let now = ui.seconds();
        let unit = space.px(self.scale);
        let vector = |x: f64, y: f64| Vec2::new(x * unit, -y * unit);
        let (net_x, net_y) = self.net_force();
        let net = vector(net_x, net_y);
        let net_grown = if self.net {
            self.grown(now, self.start_of(self.forces.len()))
        } else {
            0.0
        };
        // the forces step back while their sum grows
        let dim = 1.0 - 0.5 * net_grown;
        let mut forces = Vec::new();
        for (i, force) in self.forces.iter().enumerate() {
            let grown = self.grown(now, self.start_of(i));
            let d = vector(force.x, force.y);
            if grown <= 0.0 || d.hypot() < 1e-6 {
                continue;
            }
            let color = force.color.unwrap_or(self.force_color);
            let tip = middle + d * grown;
            forces.push(draw_animated(
                ui,
                &[arrow(ui, middle, tip, color, self.stroke_width)],
                None,
                None,
                None,
            ));
            if self.labels {
                let text = if self.magnitudes {
                    format!("{} = {} N", force.name, short(force.magnitude(), 1))
                } else {
                    force.name.to_owned()
                };
                let text = label(ui, text, tip, beyond(d), color);
                forces.push(fframes::svgr!(<g opacity={grown}>{text}</g>));
            }
        }
        let forces: Svgr<'a> = forces.into();
        let mut parts = vec![fframes::svgr!(<g opacity={dim}>{forces}</g>)];
        if net_grown > 0.0 {
            let text = format!("{} = {} N", self.net_name, short(net_x.hypot(net_y), 1));
            if net.hypot() > 1.0 {
                let tip = middle + net * net_grown;
                let mut shape = arrow(ui, middle, tip, self.net_color, self.stroke_width);
                shape.dash = Some((10.0, 8.0));
                parts.push(draw_animated(ui, &[shape], None, None, None));
                if self.labels {
                    // beside its tip, off the line the other forces may share
                    let side = beside(net);
                    let off = match side {
                        Anchor::Top => Vec2::new(0.0, -10.0),
                        Anchor::Left => Vec2::new(-10.0, 0.0),
                        _ => Vec2::new(10.0, 0.0),
                    };
                    let text = label(ui, text, tip + off, side, self.net_color);
                    parts.push(fframes::svgr!(<g opacity={net_grown}>{text}</g>));
                }
            } else if self.labels {
                let under = middle + Vec2::new(0.0, f64::from(self.size) / 2.0);
                let text = format!("{} = 0", self.net_name);
                let text = label(ui, text, under, Anchor::Bottom, self.net_color);
                parts.push(fframes::svgr!(<g opacity={net_grown}>{text}</g>));
            }
        }
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{block}{parts}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_net_force_is_the_sum() {
        let body = FreeBody {
            forces: vec![
                Force::new("W", 0.0, -10.0),
                Force::new("N", 0.0, 10.0),
                Force::new("F", 6.0, 0.0),
                Force::new("f", -2.0, 0.0),
            ],
            ..FreeBody::default()
        };
        assert_eq!(body.net_force(), (4.0, 0.0));
        assert_eq!(Force::new("F", 3.0, 4.0).magnitude(), 5.0);
    }

    #[test]
    fn the_arrows_grow_in_turn_and_the_net_force_last() {
        let body = FreeBody {
            forces: vec![Force::new("a", 1.0, 0.0), Force::new("b", 0.0, 1.0)],
            delay: 1.0,
            grow: 0.5,
            stagger: 0.4,
            ..FreeBody::default()
        };
        assert_eq!(body.start_of(0), 1.0);
        assert_eq!(body.start_of(1), 1.4);
        // after the last arrow has grown
        assert!(body.start_of(2) >= body.start_of(1) + body.grow);
        assert_eq!(body.grown(0.9, body.start_of(0)), 0.0);
        assert_eq!(body.grown(1.5, body.start_of(0)), 1.0);
    }

    #[test]
    fn labels_sit_past_the_tip() {
        assert_eq!(beyond(Vec2::new(10.0, 2.0)), Anchor::Right);
        assert_eq!(beyond(Vec2::new(-10.0, 2.0)), Anchor::Left);
        assert_eq!(beyond(Vec2::new(1.0, 10.0)), Anchor::Bottom);
        assert_eq!(beyond(Vec2::new(1.0, -10.0)), Anchor::Top);
    }
}
