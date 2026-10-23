use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line, width};

/// How one letter of [`Letters`] is drawn: moved by `(dx, dy)`, faded to `opacity` and
/// scaled by `scale` about its center.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LetterLook {
    pub dx: f32,
    pub dy: f32,
    pub opacity: f32,
    pub scale: f32,
}

impl Default for LetterLook {
    fn default() -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            opacity: 1.0,
            scale: 1.0,
        }
    }
}

/// A line of text drawn letter by letter, each letter with its own [`LetterLook`] from
/// `look(index)`. `(x, y)` is the start of the baseline and `tracking` adds pixels after
/// every letter. Letters at opacity 0 are left out. Without a measurable font the line is
/// one `<text>` with the look of letter 0.
pub struct Letters<'a> {
    pub text: Cow<'a, str>,
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub tracking: f32,
    pub look: Box<dyn Fn(usize) -> LetterLook + 'a>,
}

impl Themed for Letters<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            text: Cow::Borrowed(""),
            x: 0.0,
            y: 0.0,
            style: t.typography.title,
            fill: t.colors.title,
            tracking: 0.0,
            look: Box::new(|_| LetterLook::default()),
        }
    }
}

impl Default for Letters<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Letters<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let s = self.style;
        let Some(widths) = self
            .text
            .chars()
            .map(|c| width(ui, s, c.encode_utf8(&mut [0; 4])))
            .collect::<Option<Vec<f32>>>()
        else {
            let look = (self.look)(0);
            return line(
                self.x + look.dx,
                self.y + look.dy,
                s,
                self.fill,
                TextAnchor::Start,
                self.tracking,
                look.opacity,
                self.text,
            );
        };
        let mut advance = 0.0;
        let mut letters = Vec::with_capacity(widths.len());
        for (i, (c, w)) in self.text.chars().zip(widths).enumerate() {
            let left = advance + self.tracking * i as f32;
            advance += w;
            let look = (self.look)(i);
            if look.opacity <= 0.001 {
                continue;
            }
            let center = self.x + left + w / 2.0;
            let transform = format!(
                "translate({} {}) scale({}) translate({} 0)",
                center + look.dx,
                self.y + look.dy,
                look.scale.max(0.001),
                -w / 2.0
            );
            let c = c.to_string();
            letters.push(fframes::svgr!(
                <text transform={transform} font-family={s.family} font-weight={s.weight} font-size={s.size}
                      fill={self.fill} opacity={look.opacity}>{c}</text>
            ));
        }
        fframes::svgr!(<g>{letters}</g>)
    }
}
