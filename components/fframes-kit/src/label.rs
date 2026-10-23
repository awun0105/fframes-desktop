//! Labels: [`Label`], a line of text placed next to a point (Manim's
//! `label.next_to(point, UR)`); [`Leader`], a line from a point to a label; [`spread`], which
//! moves labels apart until they no longer overlap. The elbow-line callout with a dot is
//! [`Annotation`](crate::overlay::Annotation).

use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::Rect;

use crate::{
    anim::{Enter, Exit, FadeIn, FadeOut},
    animate::{EnterAnim, ExitAnim, draw_animated},
    layout::{self, Anchor},
    shapes::{Dot, Line},
    text::{TextAnchor, line, width},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// One line of text whose box sits `buff` pixels from `(x, y)` in `direction`: `Top` puts it
/// above the point, `TopRight` up and to the right, `Center` on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Label<'a> {
    pub text: Cow<'a, str>,
    pub x: f32,
    pub y: f32,
    pub direction: Anchor,
    pub buff: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for Label<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            text: Cow::Borrowed(""),
            x: 0.0,
            y: 0.0,
            direction: Anchor::Right,
            buff: t.spacing.sm,
            style: t.typography.body,
            fill: t.colors.title,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Label<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Label<'_> {
    /// Width and height of the text's box: measured with the font when it resolves,
    /// otherwise estimated at 0.55 em per character. The height is the cap height.
    pub fn size(&self, ui: &Ui<'_, '_, '_>) -> (f32, f32) {
        let em = self.style.size as f32;
        let w = width(ui, self.style, &self.text)
            .unwrap_or_else(|| self.text.chars().count() as f32 * em * 0.55);
        (w, em * 0.72)
    }

    /// The box the text is drawn in, in pixels.
    pub fn bounds(&self, ui: &Ui<'_, '_, '_>) -> Rect {
        let (w, h) = self.size(ui);
        self.box_of(w, h)
    }

    /// The box of a `w` x `h` text next to the point.
    fn box_of(&self, w: f32, h: f32) -> Rect {
        let (left, top) = self.direction.next_to(
            f64::from(self.x),
            f64::from(self.y),
            f64::from(w),
            f64::from(h),
            f64::from(self.buff),
        );
        Rect::new(left, top, left + f64::from(w), top + f64::from(h))
    }
}

impl<'a> Component<'a> for Label<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let b = self.bounds(ui);
        let mut drawn = line(
            b.x0 as f32,
            b.y1 as f32,
            self.style,
            self.fill,
            TextAnchor::Start,
            0.0,
            1.0,
            self.text,
        );
        if let Some(enter) = &self.enter {
            drawn = enter.apply(ui, drawn);
        }
        if let Some(exit) = &self.exit {
            drawn = exit.apply(ui, drawn);
        }
        drawn
    }
}

/// Moves labels down until no two boxes overlap, `gap` pixels apart, in the order given:
/// the first keeps its place (see [`layout::separate`]).
pub fn spread(ui: &Ui<'_, '_, '_>, labels: &mut [Label<'_>], gap: f32) {
    let boxes: Vec<Rect> = labels.iter().map(|l| l.bounds(ui)).collect();
    for (label, dy) in labels
        .iter_mut()
        .zip(layout::separate(&boxes, f64::from(gap)))
    {
        label.y += dy as f32;
    }
}

/// A line from `from` to `to` with text at its end, on the side the line points to, and
/// a dot on `from` when `dot` is above 0. The line draws with `enter` (`Create` from
/// `from`), the text fades with `label_enter`.
#[derive(Debug, Clone, PartialEq)]
pub struct Leader<'a> {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub text: Cow<'a, str>,
    pub style: TextStyle,
    pub fill: Color,
    pub color: Color,
    pub stroke_width: f32,
    /// Radius of the dot on `from`, 0 for none.
    pub dot: f32,
    pub buff: f32,
    pub enter: Option<EnterAnim>,
    pub exit: Option<ExitAnim>,
    pub label_enter: Option<FadeIn>,
    pub label_exit: Option<FadeOut>,
}

impl Themed for Leader<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            from: (0.0, 0.0),
            to: (100.0, -60.0),
            text: Cow::Borrowed(""),
            style: t.typography.caption,
            fill: t.colors.title,
            color: t.colors.muted,
            stroke_width: t.stroke.thin,
            dot: 6.0,
            buff: t.spacing.xs,
            enter: None,
            exit: None,
            label_enter: None,
            label_exit: None,
        }
    }
}

impl Default for Leader<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Leader<'a> {
    /// The label at the end, placed on the side the line points to.
    pub fn label(&self) -> Label<'a> {
        let (dx, dy) = (self.to.0 - self.from.0, self.to.1 - self.from.1);
        Label {
            text: self.text.clone(),
            x: self.to.0,
            y: self.to.1,
            direction: Anchor::toward(f64::from(dx), f64::from(dy)),
            buff: self.buff,
            style: self.style,
            fill: self.fill,
            enter: self.label_enter,
            exit: self.label_exit,
        }
    }
}

impl<'a> Component<'a> for Leader<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let mut parts = vec![
            Line {
                x1: self.from.0,
                y1: self.from.1,
                x2: self.to.0,
                y2: self.to.1,
                stroke: Some(self.color),
                stroke_width: self.stroke_width,
                ..ui.defaults()
            }
            .shape(),
        ];
        if self.dot > 0.0 {
            parts.push(
                Dot {
                    cx: self.from.0,
                    cy: self.from.1,
                    radius: self.dot,
                    fill: Some(self.color),
                    ..ui.defaults()
                }
                .shape(),
            );
        }
        let drawn = draw_animated(ui, &parts, self.enter.as_ref(), self.exit.as_ref(), None);
        let label = self.label();
        let label = ui.show(label);
        fframes::svgr!(<g>{drawn}{label}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leader_labels_sit_on_the_far_side_of_the_line() {
        let leader = Leader {
            from: (100.0, 100.0),
            to: (200.0, 40.0),
            text: "peak".into(),
            buff: 4.0,
            ..Leader::default()
        };
        let label = leader.label();
        assert_eq!(label.direction, Anchor::TopRight);
        // a 40 x 10 text: its bottom left corner is 4 px up and right of the line's end
        let b = label.box_of(40.0, 10.0);
        assert_eq!(b, Rect::new(204.0, 26.0, 244.0, 36.0));
        let down = Leader {
            to: (100.0, 200.0),
            ..leader
        }
        .label();
        assert_eq!(down.direction, Anchor::Bottom);
        assert_eq!(
            down.box_of(40.0, 10.0),
            Rect::new(80.0, 204.0, 120.0, 214.0)
        );
    }
}
