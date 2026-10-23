//! [`Label`]: a line of text placed next to a point, Manim's `label.next_to(point, UR)`.

use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    anim::{Enter, Exit, FadeIn, FadeOut},
    layout::Anchor,
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
}

impl<'a> Component<'a> for Label<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (w, h) = self.size(ui);
        let (left, top) = self.direction.next_to(
            f64::from(self.x),
            f64::from(self.y),
            f64::from(w),
            f64::from(h),
            f64::from(self.buff),
        );
        let mut drawn = line(
            left as f32,
            top as f32 + h,
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
