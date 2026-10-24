use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line};

/// Manim's `BulletedList`: one line per item, each after a round bullet. `(x, y)` is the
/// left of the first bullet on the first baseline; lines are `line_spacing` times the font
/// size apart. The bullets are circles, so they need no glyph in the font.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct BulletedList<'a> {
    pub items: Vec<Cow<'a, str>>,
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub bullet_fill: Color,
    /// Pixels from the bullet's left to the text.
    pub indent: f32,
    /// Baseline distance as a multiple of the font size.
    pub line_spacing: f32,
    pub opacity: f32,
}

impl Themed for BulletedList<'_> {
    fn themed(t: &Theme) -> Self {
        let style = t.typography.body;
        Self {
            items: Vec::new(),
            x: 0.0,
            y: 0.0,
            style,
            fill: t.colors.text,
            bullet_fill: t.colors.accent,
            indent: style.size as f32 * 1.1,
            line_spacing: style.line_height * 1.25,
            opacity: 1.0,
        }
    }
}

impl Default for BulletedList<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl BulletedList<'_> {
    /// The baseline of item `index`.
    pub fn baseline(&self, index: usize) -> f32 {
        self.y + index as f32 * self.line_spacing * self.style.size as f32
    }
}

impl<'a> Component<'a> for BulletedList<'a> {
    fn render(self, _ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let size = self.style.size as f32;
        let r = size * 0.11;
        let lines: Vec<Svgr<'a>> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let y = self.baseline(i);
                // the middle of the lowercase letters
                let (cx, cy) = (self.x + r, y - size * 0.3);
                let fill = self.bullet_fill;
                let text = line(
                    self.x + self.indent,
                    y,
                    self.style,
                    self.fill,
                    TextAnchor::Start,
                    0.0,
                    1.0,
                    Cow::Borrowed(item),
                );
                fframes::svgr!(<g><circle cx={cx} cy={cy} r={r} fill={fill} />{text}</g>)
            })
            .collect();
        fframes::svgr!(<g opacity={self.opacity}>{lines}</g>)
    }
}
