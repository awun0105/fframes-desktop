use std::borrow::Cow;

use fframes::{BreakLinesOpts, Color, Svgr, TextAlign};

use crate::{
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line};

/// Text wrapped into lines of at most `width` pixels (`Frame::text_break_lines`), aligned
/// inside that width. `(x, y)` is the start of the first baseline; lines are
/// `style.line_height` apart and `\n` starts a new one. Without a measurable font it is one
/// unwrapped line.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Paragraph<'a> {
    pub text: Cow<'a, str>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub align: TextAlign,
    pub opacity: f32,
}

impl Themed for Paragraph<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            text: Cow::Borrowed(""),
            x: 0.0,
            y: 0.0,
            width: 800.0,
            style: t.typography.body,
            fill: t.colors.text,
            align: TextAlign::Left,
            opacity: 1.0,
        }
    }
}

impl Default for Paragraph<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Paragraph<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let s = self.style;
        let opts: BreakLinesOpts<'a, usize, usize> = BreakLinesOpts {
            font: s.font(),
            width: self.width.max(0.0) as usize,
            line_height: s.line_height,
            align: self.align,
            ..Default::default()
        };
        let Some(wrapped) = ui
            .frame
            .text_break_lines_structure(ui.ctx, &self.text, opts)
        else {
            return line(
                self.x,
                self.y,
                s,
                self.fill,
                TextAnchor::Start,
                0.0,
                self.opacity,
                self.text,
            );
        };
        let lines: Vec<Svgr<'a>> = wrapped
            .lines
            .iter()
            .map(|l| {
                let (x, y) = (self.x + l.dx as f32, self.y + l.dy as f32);
                let words = l.words.join(" ");
                fframes::svgr!(<tspan x={x} y={y}>{words}</tspan>)
            })
            .collect();
        fframes::svgr!(
            <text font-family={s.family} font-size={s.size} font-weight={s.weight} fill={self.fill} opacity={self.opacity}>
                {lines}
            </text>
        )
    }
}
