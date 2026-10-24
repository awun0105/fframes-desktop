//! Text components: measured with the fonts of the video's media, with a plain `<text>` as
//! the fallback when a family cannot be measured (system fonts in the editor).
//!
//! [`FitText`], [`Letters`], [`TextReveal`], [`Typewriter`], [`Counter`], [`Paragraph`],
//! [`HudLabel`], and Manim's [`DecimalNumber`], [`Integer`], [`Variable`], [`BulletedList`],
//! [`Title`] and [`MarkupText`].

use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{theme::TextStyle, ui::Ui};

mod bulleted_list;
mod counter;
mod decimal;
mod fit;
mod label;
mod letters;
mod markup;
mod paragraph;
mod reveal;
mod title;
mod typewriter;

pub use bulleted_list::BulletedList;
pub use counter::Counter;
pub(crate) use decimal::format_number;
pub use decimal::{DecimalNumber, Integer, Variable};
pub use fit::FitText;
pub use label::HudLabel;
pub use letters::{LetterLook, Letters};
pub use markup::{MarkupRun, MarkupText, parse_markup};
pub use paragraph::Paragraph;
pub use reveal::{RevealBy, TextReveal};
pub use title::Title;
pub use typewriter::Typewriter;

/// Where `x` is on a line of text (SVG's `text-anchor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAnchor {
    #[default]
    Start,
    Middle,
    End,
}

impl TextAnchor {
    /// The `text-anchor` value.
    pub fn as_str(self) -> &'static str {
        match self {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
        }
    }

    /// The left edge of a line `width` pixels wide whose anchor is at `x`.
    pub fn left(self, x: f32, width: f32) -> f32 {
        match self {
            TextAnchor::Start => x,
            TextAnchor::Middle => x - width / 2.0,
            TextAnchor::End => x - width,
        }
    }
}

/// Width of `text` in `style`, `None` when the family cannot be measured.
pub(crate) fn width(ui: &Ui<'_, '_, '_>, style: TextStyle, text: &str) -> Option<f32> {
    ui.measure(style.font(), text)
}

/// One `<text>` line in `style`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn line<'a>(
    x: f32,
    y: f32,
    style: TextStyle,
    fill: Color,
    anchor: TextAnchor,
    letter_spacing: f32,
    opacity: f32,
    text: Cow<'_, str>,
) -> Svgr<'a> {
    let text = text.into_owned();
    fframes::svgr!(
        <text x={x} y={y} font-family={style.family} font-size={style.size} font-weight={style.weight}
              letter-spacing={letter_spacing} text-anchor={anchor.as_str()} fill={fill} opacity={opacity}>
            {text}
        </text>
    )
}
