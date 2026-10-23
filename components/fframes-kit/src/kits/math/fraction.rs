//! Fractions as parts of a whole: [`FractionBar`] and [`FractionPie`].

use std::f32::consts::{FRAC_PI_2, TAU};

use fframes::{Color, Svgr};

use crate::{
    anim::RateFunc,
    animate::{EnterAnim, draw_animated},
    geometry::Shape,
    label::Label,
    layout::Anchor,
    motion::progress,
    shapes::{Rectangle, Sector},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// How far part `i` of the filled ones is filled `t` seconds in: the parts fill one after
/// another, `stagger` seconds apart, each over `run_time`.
fn part_fill(t: f32, i: usize, delay: f32, stagger: f32, run_time: f32, rate: RateFunc) -> f32 {
    let start = delay + i as f32 * stagger;
    rate.apply(progress(t, start, start + run_time), run_time)
}

/// The fraction written as `n/d`.
fn fraction_text(numerator: usize, denominator: usize) -> String {
    format!("{numerator}/{denominator}")
}

/// A bar of `width` x `height` pixels at `(x, y)` (top left) cut into `denominator` equal
/// parts, of which the first `numerator` fill one after another. Bars of the same width
/// stacked under each other show equal fractions lining up (`1/2`, `2/4`, `4/8`). The outline
/// and the cuts draw with `enter`; with `label` the fraction is written left of the bar.
#[derive(Debug, Clone)]
pub struct FractionBar {
    pub numerator: usize,
    pub denominator: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
    pub stroke: Color,
    pub stroke_width: f32,
    pub delay: f32,
    pub stagger: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
    pub label: bool,
    pub label_style: TextStyle,
    pub enter: Option<EnterAnim>,
}

impl Themed for FractionBar {
    fn themed(t: &Theme) -> Self {
        Self {
            numerator: 1,
            denominator: 2,
            x: 560.0,
            y: 500.0,
            width: 800.0,
            height: 80.0,
            color: t.colors.accent,
            stroke: t.colors.text,
            stroke_width: t.stroke.regular,
            delay: 0.0,
            stagger: 0.15,
            run_time: t.motion.duration.base,
            rate_func: RateFunc::default(),
            label: true,
            label_style: t.typography.body,
            enter: None,
        }
    }
}

impl Default for FractionBar {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl FractionBar {
    /// Part `i`'s rectangle (center, width) in pixels.
    fn part(&self, i: usize) -> (f32, f32, f32) {
        let w = self.width / self.denominator.max(1) as f32;
        (self.x + w * (i as f32 + 0.5), self.y + self.height / 2.0, w)
    }
}

impl<'a> Component<'a> for FractionBar {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        if self.denominator == 0 {
            return Svgr::empty();
        }
        let t = ui.seconds();
        let filled: Vec<Shape> = (0..self.numerator.min(self.denominator))
            .filter_map(|i| {
                let p = part_fill(
                    t,
                    i,
                    self.delay,
                    self.stagger,
                    self.run_time,
                    self.rate_func,
                );
                let (cx, cy, w) = self.part(i);
                (p > 0.0).then(|| {
                    Rectangle {
                        cx,
                        cy,
                        width: w,
                        height: self.height,
                        fill: Some(self.color),
                        stroke: None,
                        opacity: p,
                        ..ui.defaults()
                    }
                    .shape()
                })
            })
            .collect();
        let outlines: Vec<Shape> = (0..self.denominator)
            .map(|i| {
                let (cx, cy, w) = self.part(i);
                Rectangle {
                    cx,
                    cy,
                    width: w,
                    height: self.height,
                    stroke: Some(self.stroke),
                    stroke_width: self.stroke_width,
                    ..ui.defaults()
                }
                .shape()
            })
            .collect();
        let filled = draw_animated(ui, &filled, None, None, None);
        let outlines = draw_animated(ui, &outlines, self.enter.as_ref(), None, None);
        let label = if self.label {
            ui.show(Label {
                text: fraction_text(self.numerator, self.denominator).into(),
                x: self.x,
                y: self.y + self.height / 2.0,
                direction: Anchor::Left,
                style: self.label_style,
                enter: Some(crate::anim::FadeIn {
                    delay: self.delay,
                    ..ui.defaults()
                }),
                ..ui.defaults()
            })
        } else {
            Svgr::empty()
        };
        fframes::svgr!(<g>{filled}{outlines}{label}</g>)
    }
}

/// A circle of `radius` pixels around `(cx, cy)` cut into `denominator` equal slices from
/// the top, clockwise, of which the first `numerator` fill one after another. With `label`
/// the fraction is written under the circle.
#[derive(Debug, Clone)]
pub struct FractionPie {
    pub numerator: usize,
    pub denominator: usize,
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
    pub color: Color,
    pub stroke: Color,
    pub stroke_width: f32,
    pub delay: f32,
    pub stagger: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
    pub label: bool,
    pub label_style: TextStyle,
    pub enter: Option<EnterAnim>,
}

impl Themed for FractionPie {
    fn themed(t: &Theme) -> Self {
        Self {
            numerator: 1,
            denominator: 4,
            cx: 960.0,
            cy: 540.0,
            radius: 200.0,
            color: t.colors.highlight,
            stroke: t.colors.text,
            stroke_width: t.stroke.regular,
            delay: 0.0,
            stagger: 0.15,
            run_time: t.motion.duration.base,
            rate_func: RateFunc::default(),
            label: true,
            label_style: t.typography.body,
            enter: None,
        }
    }
}

impl Default for FractionPie {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl FractionPie {
    /// Slice `i`'s start angle and angle (radians, counterclockwise on screen): slice 0
    /// ends at the top, so the slices run clockwise from there.
    fn slice(&self, i: usize) -> (f32, f32) {
        let step = TAU / self.denominator.max(1) as f32;
        (FRAC_PI_2 - (i as f32 + 1.0) * step, step)
    }

    fn sector(&self, ui: &Ui<'_, '_, '_>, i: usize) -> Sector {
        let (start_angle, angle) = self.slice(i);
        Sector {
            cx: self.cx,
            cy: self.cy,
            radius: self.radius,
            start_angle,
            angle,
            ..ui.defaults()
        }
    }
}

impl<'a> Component<'a> for FractionPie {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        if self.denominator == 0 {
            return Svgr::empty();
        }
        let t = ui.seconds();
        let filled: Vec<Shape> = (0..self.numerator.min(self.denominator))
            .filter_map(|i| {
                let p = part_fill(
                    t,
                    i,
                    self.delay,
                    self.stagger,
                    self.run_time,
                    self.rate_func,
                );
                (p > 0.0).then(|| {
                    Sector {
                        fill: Some(self.color),
                        stroke: None,
                        opacity: p,
                        ..self.sector(ui, i)
                    }
                    .shape()
                })
            })
            .collect();
        let outlines: Vec<Shape> = (0..self.denominator)
            .map(|i| {
                Sector {
                    fill: None,
                    stroke: Some(self.stroke),
                    stroke_width: self.stroke_width,
                    ..self.sector(ui, i)
                }
                .shape()
            })
            .collect();
        let filled = draw_animated(ui, &filled, None, None, None);
        let outlines = draw_animated(ui, &outlines, self.enter.as_ref(), None, None);
        let label = if self.label {
            ui.show(Label {
                text: fraction_text(self.numerator, self.denominator).into(),
                x: self.cx,
                y: self.cy + self.radius,
                direction: Anchor::Bottom,
                style: self.label_style,
                enter: Some(crate::anim::FadeIn {
                    delay: self.delay,
                    ..ui.defaults()
                }),
                ..ui.defaults()
            })
        } else {
            Svgr::empty()
        };
        fframes::svgr!(<g>{filled}{outlines}{label}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_fill_one_after_another() {
        let fill = |t, i| part_fill(t, i, 1.0, 0.5, 1.0, RateFunc::Linear);
        assert_eq!(fill(0.5, 0), 0.0);
        assert_eq!(fill(1.5, 0), 0.5);
        assert_eq!(fill(1.5, 1), 0.0);
        assert_eq!(fill(2.0, 1), 0.5);
        assert_eq!(fill(9.0, 3), 1.0);
    }

    #[test]
    fn equal_fractions_line_up() {
        let bar = |d| FractionBar {
            denominator: d,
            x: 100.0,
            width: 800.0,
            ..FractionBar::default()
        };
        // the right edge of 1/2 is the right edge of 2/4 and 4/8
        let edge = |b: FractionBar, n: usize| {
            let (cx, _, w) = b.part(n - 1);
            cx + w / 2.0
        };
        assert_eq!(edge(bar(2), 1), 500.0);
        assert_eq!(edge(bar(4), 2), 500.0);
        assert_eq!(edge(bar(8), 4), 500.0);
    }

    #[test]
    fn slices_run_clockwise_from_the_top() {
        let pie = FractionPie {
            denominator: 4,
            ..FractionPie::default()
        };
        let (start, angle) = pie.slice(0);
        assert!((start - 0.0).abs() < 1e-6 && (angle - FRAC_PI_2).abs() < 1e-6);
        let (start, _) = pie.slice(1);
        assert!((start + FRAC_PI_2).abs() < 1e-6);
    }
}
