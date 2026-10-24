use fframes::{Color, Svgr};
use kurbo::Point;

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::{EnterAnim, ExitAnim, draw_animated},
    layout::Anchor,
    shapes::Rectangle,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

use super::label;

/// Bars of kinetic energy, potential energy and their sum, standing on `(x, y)` (pixels, the
/// bottom left corner) and `height` pixels tall at `max` joules. Give them the values of the
/// current frame, for example from [`PendulumMotion::energy`](super::PendulumMotion::energy)
/// at the same clock as the figure, and the exchange shows as one bar shrinking while the
/// other grows with the total level. The frames draw with `enter`, the bars and labels fade
/// with `parts_enter`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct EnergyBars {
    pub kinetic: f64,
    pub potential: f64,
    pub max: f64,
    pub x: f32,
    pub y: f32,
    pub bar_width: f32,
    pub gap: f32,
    pub height: f32,
    pub labels: bool,
    pub kinetic_color: Color,
    pub potential_color: Color,
    pub total_color: Color,
    pub frame_color: Color,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub parts_enter: Option<FadeIn>,
    pub parts_exit: Option<FadeOut>,
}

impl Themed for EnergyBars {
    fn themed(t: &Theme) -> Self {
        Self {
            kinetic: 0.0,
            potential: 0.0,
            max: 1.0,
            x: 1400.0,
            y: 820.0,
            bar_width: 80.0,
            gap: 40.0,
            height: 420.0,
            labels: true,
            kinetic_color: t.colors.accent,
            potential_color: t.colors.highlight,
            total_color: t.colors.text,
            frame_color: t.colors.muted,
            enter: None,
            exit: None,
            parts_enter: None,
            parts_exit: None,
        }
    }
}

impl Default for EnergyBars {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl EnergyBars {
    /// The pixel height of `value` joules, between 0 and `height`.
    fn level(&self, value: f64) -> f32 {
        if self.max <= 0.0 || !value.is_finite() {
            return 0.0;
        }
        (value / self.max).clamp(0.0, 1.0) as f32 * self.height
    }

    /// The left edge of bar `i`.
    fn left(&self, i: usize) -> f32 {
        self.x + i as f32 * (self.bar_width + self.gap)
    }
}

fn bar(ui: &Ui<'_, '_, '_>, left: f32, bottom: f32, width: f32, height: f32) -> Rectangle {
    Rectangle {
        cx: left + width / 2.0,
        cy: bottom - height / 2.0,
        width,
        height,
        ..ui.defaults()
    }
}

impl<'a> Component<'a> for EnergyBars {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (w, h) = (self.bar_width, self.height);
        let frames: Vec<_> = (0..3)
            .map(|i| {
                Rectangle {
                    fill: None,
                    stroke: Some(self.frame_color),
                    stroke_width: ui.theme.stroke.thin,
                    ..bar(ui, self.left(i), self.y, w, h)
                }
                .shape()
            })
            .collect();
        let frames = draw_animated(ui, &frames, self.enter.as_ref(), self.exit.as_ref(), None);

        let kinetic = self.level(self.kinetic);
        let potential = self.level(self.potential);
        let filled = |rect: Rectangle, color: Color| {
            Rectangle {
                fill: Some(color),
                stroke: None,
                ..rect
            }
            .shape()
        };
        let mut shapes = vec![
            filled(
                bar(ui, self.left(0), self.y, w, kinetic),
                self.kinetic_color,
            ),
            filled(
                bar(ui, self.left(1), self.y, w, potential),
                self.potential_color,
            ),
            // the total, stacked from the two parts
            filled(
                bar(ui, self.left(2), self.y, w, kinetic),
                self.kinetic_color,
            ),
        ];
        let total = self.level(self.kinetic + self.potential);
        shapes.push(filled(
            bar(
                ui,
                self.left(2),
                self.y - kinetic,
                w,
                (total - kinetic).max(0.0),
            ),
            self.potential_color,
        ));
        shapes.retain(|s| kurbo::Shape::area(&s.path).abs() > 0.0);
        let mut parts = vec![draw_animated(ui, &shapes, None, None, None)];
        if self.labels {
            for (i, text) in ["KE", "PE", "E"].into_iter().enumerate() {
                let at = Point::new(f64::from(self.left(i) + w / 2.0), f64::from(self.y));
                parts.push(label(
                    ui,
                    text.to_owned(),
                    at,
                    Anchor::Bottom,
                    self.total_color,
                ));
            }
        }
        let parts = fade(
            ui,
            parts.into(),
            self.parts_enter.as_ref(),
            self.parts_exit.as_ref(),
        );
        fframes::svgr!(<g>{parts}{frames}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_clamped_to_the_bar() {
        let bars = EnergyBars {
            max: 10.0,
            height: 200.0,
            ..EnergyBars::default()
        };
        assert_eq!(bars.level(5.0), 100.0);
        assert_eq!(bars.level(20.0), 200.0);
        assert_eq!(bars.level(-1.0), 0.0);
        assert_eq!(bars.level(f64::NAN), 0.0);
        let none = EnergyBars { max: 0.0, ..bars };
        assert_eq!(none.level(5.0), 0.0);
    }
}
