use std::borrow::Cow;

use fframes::{Color, Svgr, Transform};

use crate::{
    anim::RateFunc,
    motion::progress,
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line, width};

/// The units [`TextReveal`] staggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RevealBy {
    /// Words, with the space after each word.
    #[default]
    Word,
    /// Every character.
    Letter,
}

/// A line whose words or letters fade in one after another while they move by `shift`
/// into place. Unit `i` starts `delay + i * stagger` seconds into the container and takes
/// `run_time`. `(x, y)` is the start of the baseline. Without a measurable font the whole
/// line fades in as unit 0.
#[derive(Debug, Clone, PartialEq)]
pub struct TextReveal<'a> {
    pub text: Cow<'a, str>,
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub by: RevealBy,
    /// Canvas units, `y` down: `(0.0, 24.0)` rises 24 units into place.
    pub shift: (f32, f32),
    pub delay: f32,
    pub stagger: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Themed for TextReveal<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            text: Cow::Borrowed(""),
            x: 0.0,
            y: 0.0,
            style: t.typography.title,
            fill: t.colors.title,
            by: RevealBy::Word,
            shift: (0.0, 24.0),
            delay: 0.0,
            stagger: t.motion.stagger.tight,
            run_time: t.motion.duration.enter,
            rate_func: t.motion.easing.enter,
        }
    }
}

impl Default for TextReveal<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// `text` split into the units of `by`; joined again they give `text`.
fn units(text: &str, by: RevealBy) -> Vec<&str> {
    match by {
        RevealBy::Letter => text
            .char_indices()
            .map(|(i, c)| &text[i..i + c.len_utf8()])
            .collect(),
        RevealBy::Word => {
            let mut units = Vec::new();
            let mut start = 0;
            let mut in_space = false;
            for (i, c) in text.char_indices() {
                if c.is_whitespace() {
                    in_space = true;
                } else if in_space {
                    units.push(&text[start..i]);
                    start = i;
                    in_space = false;
                }
            }
            if start < text.len() {
                units.push(&text[start..]);
            }
            units
        }
    }
}

impl TextReveal<'_> {
    /// How far unit `i` is in, from 0 to 1.
    fn shown(&self, ui: &Ui<'_, '_, '_>, i: usize) -> f32 {
        let start = self.delay + i as f32 * self.stagger;
        let p = progress(ui.seconds(), start, start + self.run_time);
        self.rate_func.apply(p, self.run_time)
    }

    fn unit<'a>(&self, x: f32, shown: f32, text: &str) -> Svgr<'a> {
        if shown <= 0.0 {
            return Svgr::empty();
        }
        let rest = 1.0 - shown;
        let text = line(
            x,
            self.y,
            self.style,
            self.fill,
            TextAnchor::Start,
            0.0,
            shown.min(1.0),
            Cow::Borrowed(text),
        );
        if rest == 0.0 {
            return text;
        }
        let shift = Transform::translate(self.shift.0 * rest, self.shift.1 * rest);
        fframes::svgr!(<g transform={shift}>{text}</g>)
    }
}

impl<'a> Component<'a> for TextReveal<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let units = units(&self.text, self.by);
        let widths: Option<Vec<f32>> = units.iter().map(|u| width(ui, self.style, u)).collect();
        let Some(widths) = widths else {
            return self.unit(self.x, self.shown(ui, 0), &self.text);
        };
        let mut x = self.x;
        let mut drawn = Vec::with_capacity(units.len());
        for (i, (unit, w)) in units.iter().zip(widths).enumerate() {
            drawn.push(self.unit(x, self.shown(ui, i), unit));
            x += w;
        }
        drawn.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_join_back_into_the_text() {
        let text = "Hello  wide world ";
        assert_eq!(units(text, RevealBy::Word), ["Hello  ", "wide ", "world "]);
        assert_eq!(units(text, RevealBy::Word).concat(), text);
        assert_eq!(units("añb", RevealBy::Letter), ["a", "ñ", "b"]);
        assert_eq!(units("", RevealBy::Word), Vec::<&str>::new());
    }
}
