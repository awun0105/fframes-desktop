use std::f64::consts::TAU;

use fframes::{Color, Svgr};
use kurbo::Vec2;

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    kits::label,
    label::Label,
    layout::Anchor,
    motion::progress,
    shapes::{Circle, Dot},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::{name, shells, symbol};

/// The Bohr model of element `element` (atomic number): its symbol on the nucleus at
/// `center` (units of `ui.space`) and its electrons on rings `shell_gap` pixels apart, from
/// the ground state configuration. After `delay` the electrons appear one by one, innermost
/// shell first, over `fill` seconds, and every shell turns, the outer ones slower
/// (`speed` radians a second on the first). With `labels` the name and the electrons per
/// shell sit under the atom. The rings draw with `enter`, the nucleus, the electrons and the
/// labels with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct BohrAtom {
    pub element: u32,
    pub center: (f64, f64),
    pub nucleus_radius: f32,
    pub shell_gap: f32,
    pub electron_radius: f32,
    pub delay: f32,
    pub fill: f32,
    pub speed: f32,
    pub labels: bool,
    pub nucleus_color: Color,
    pub shell_color: Color,
    pub electron_color: Color,
    pub text_color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for BohrAtom {
    fn themed(t: &Theme) -> Self {
        Self {
            element: 11,
            center: (0.0, 0.0),
            nucleus_radius: 46.0,
            shell_gap: 56.0,
            electron_radius: 10.0,
            delay: 0.0,
            fill: 1.5,
            speed: 1.2,
            labels: true,
            nucleus_color: t.colors.accent,
            shell_color: t.colors.muted,
            electron_color: t.colors.highlight,
            text_color: t.colors.text,
            stroke_width: t.stroke.thin,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for BohrAtom {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl BohrAtom {
    /// The radius in pixels of shell `index` (0 innermost).
    pub fn shell_radius(&self, index: usize) -> f64 {
        f64::from(self.nucleus_radius) + f64::from(self.shell_gap) * (index as f64 + 1.0)
    }

    /// The angle in radians of electron `slot` of the `count` on shell `index`, `t` seconds
    /// after `delay`: evenly spaced, turning at `speed / (index + 1)`.
    pub fn electron_angle(&self, index: usize, slot: u32, count: u32, t: f64) -> f64 {
        let spin = f64::from(self.speed) / (index as f64 + 1.0) * t;
        // the shells start a little turned against each other
        TAU * f64::from(slot) / f64::from(count.max(1)) + 0.3 * index as f64 + spin
    }
}

impl<'a> Component<'a> for BohrAtom {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let shells = shells(self.element);
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let rings: Vec<_> = (0..shells.len())
            .map(|i| {
                Circle {
                    cx: middle.x as f32,
                    cy: middle.y as f32,
                    radius: self.shell_radius(i) as f32,
                    fill: None,
                    stroke: Some(self.shell_color),
                    stroke_width: self.stroke_width,
                    ..ui.defaults()
                }
                .shape()
            })
            .collect();
        let rings = draw_animated(ui, &rings, self.enter.as_ref(), self.exit.as_ref(), None);

        let now = ui.seconds() - self.delay;
        let t = f64::from(now.max(0.0));
        let total: u32 = shells.iter().sum();
        let mut shapes = vec![
            Dot {
                cx: middle.x as f32,
                cy: middle.y as f32,
                radius: self.nucleus_radius,
                fill: Some(self.nucleus_color),
                ..ui.defaults()
            }
            .shape(),
        ];
        let mut order = 0;
        for (i, &count) in shells.iter().enumerate() {
            for slot in 0..count {
                // each electron grows in over a fifth of a second at its turn
                let at = self.fill * order as f32 / total.max(1) as f32;
                order += 1;
                let grown = progress(now, at, at + 0.2);
                if grown <= 0.0 {
                    continue;
                }
                let angle = self.electron_angle(i, slot, count, t);
                let r = self.shell_radius(i);
                let p = middle + Vec2::new(angle.cos(), -angle.sin()) * r;
                shapes.push(
                    Dot {
                        cx: p.x as f32,
                        cy: p.y as f32,
                        radius: self.electron_radius * grown,
                        fill: Some(self.electron_color),
                        ..ui.defaults()
                    }
                    .shape(),
                );
            }
        }
        let mut parts = vec![draw_animated(ui, &shapes, None, None, None)];
        let mut style = ui.theme.typography.title;
        style.size = (self.nucleus_radius * 0.8).round().max(8.0) as usize;
        parts.push(ui.show(Label {
            text: symbol(self.element).into(),
            x: middle.x as f32,
            y: middle.y as f32,
            direction: Anchor::Center,
            buff: 0.0,
            style,
            fill: ui.theme.colors.title,
            ..ui.defaults()
        }));
        if self.labels && !shells.is_empty() {
            let outer = self.shell_radius(shells.len() - 1) + f64::from(self.electron_radius);
            let below = middle + Vec2::new(0.0, outer + 16.0);
            let counts: Vec<String> = shells.iter().map(u32::to_string).collect();
            let text = format!("{}  ·  {}", name(self.element), counts.join(", "));
            parts.push(label(ui, text, below, Anchor::Bottom, self.text_color));
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
    fn shells_are_spaced_and_electrons_spread() {
        let atom = BohrAtom::default();
        assert_eq!(atom.shell_radius(0), 102.0);
        assert_eq!(atom.shell_radius(2), 214.0);
        // eight electrons a quarter turn apart in pairs, at the start
        let a = atom.electron_angle(1, 0, 8, 0.0);
        let b = atom.electron_angle(1, 2, 8, 0.0);
        assert!((b - a - TAU / 4.0).abs() < 1e-12);
        // the second shell turns at half the speed of the first
        let first = atom.electron_angle(0, 0, 2, 1.0) - atom.electron_angle(0, 0, 2, 0.0);
        let second = atom.electron_angle(1, 0, 8, 1.0) - atom.electron_angle(1, 0, 8, 0.0);
        assert!((first - 2.0 * second).abs() < 1e-12);
    }
}
