use std::borrow::Cow;

use fframes::{Color, Svgr, TextAlign, Transform};

use crate::{
    anim::RateFunc,
    motion::progress,
    text::{Paragraph, TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// The cue of a subtitle (`.vtt`) file that plays now, wrapped into `width` pixels. Cue times are
/// seconds of the video, or of the container with `video_time: false`. A missing file draws
/// nothing (and `inspect` reports it).
#[derive(Debug, Clone, PartialEq)]
pub struct Captions<'a> {
    /// A subtitle file of the video's media.
    pub file: &'a str,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub align: TextAlign,
    pub video_time: bool,
}

impl Themed for Captions<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            file: "",
            x: 160.0,
            y: 900.0,
            width: 1600.0,
            style: t.typography.heading,
            fill: t.colors.caption,
            align: TextAlign::Center,
            video_time: true,
        }
    }
}

impl Default for Captions<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Captions<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(subtitles) = ui.ctx.get_subtitles(self.file) else {
            return Svgr::empty();
        };
        let frame = if self.video_time {
            ui.frame.clone().into_global()
        } else {
            ui.frame.clone()
        };
        let Some(phrase) = frame.get_subtitle_phrase(subtitles) else {
            return Svgr::empty();
        };
        ui.show(Paragraph {
            text: Cow::Owned(phrase.to_owned()),
            x: self.x,
            y: self.y,
            width: self.width,
            style: self.style,
            fill: self.fill,
            align: self.align,
            opacity: 1.0,
        })
    }
}

/// One card of a [`CaptionStack`]: a small `tag` over a `line`, from `at` seconds on.
#[derive(Debug, Clone, PartialEq)]
pub struct Caption<'a> {
    pub at: f32,
    pub tag: Cow<'a, str>,
    pub line: Cow<'a, str>,
}

impl<'a> Caption<'a> {
    pub fn new(at: f32, tag: impl Into<Cow<'a, str>>, line: impl Into<Cow<'a, str>>) -> Self {
        Self {
            at,
            tag: tag.into(),
            line: line.into(),
        }
    }
}

/// Timed captions without a subtitle file: each [`Caption`] rises `rise` units into place
/// at its `at` seconds and leaves upward while the next one comes in. The last stays.
/// `(x, y)` is the baseline of the tag; the line is `gap` pixels below it.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptionStack<'a> {
    pub items: Vec<Caption<'a>>,
    pub x: f32,
    pub y: f32,
    pub gap: f32,
    pub tag_style: TextStyle,
    pub line_style: TextStyle,
    pub tag_fill: Color,
    pub line_fill: Color,
    pub tag_spacing: f32,
    pub rise: f32,
    pub enter: f32,
    /// Seconds before the next caption's `at` that this one starts to leave.
    pub leave: f32,
    pub rate_func: RateFunc,
}

impl Themed for CaptionStack<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            items: Vec::new(),
            x: 96.0,
            y: 860.0,
            gap: 88.0,
            tag_style: t.typography.caption,
            line_style: t.typography.heading,
            tag_fill: t.colors.accent,
            line_fill: t.colors.title,
            tag_spacing: 4.0,
            rise: 40.0,
            enter: t.motion.duration.enter,
            leave: t.motion.duration.exit,
            rate_func: t.motion.easing.enter,
        }
    }
}

impl Default for CaptionStack<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl CaptionStack<'_> {
    /// `(opacity, dy)` of item `i` at `t`.
    fn state(&self, i: usize, t: f32) -> (f32, f32) {
        let at = self.items[i].at;
        let shown = self
            .rate_func
            .apply(progress(t, at, at + self.enter), self.enter);
        let left = self
            .items
            .get(i + 1)
            .map_or(0.0, |next| progress(t, next.at - self.leave, next.at));
        let opacity = progress(t, at, at + self.enter) * (1.0 - left);
        (opacity, (1.0 - shown) * self.rise - left * self.rise * 0.75)
    }
}

impl<'a> Component<'a> for CaptionStack<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let states: Vec<(f32, f32)> = (0..self.items.len()).map(|i| self.state(i, t)).collect();
        self.items
            .into_iter()
            .zip(states)
            .filter(|(_, (opacity, _))| *opacity > 0.0)
            .map(|(item, (opacity, dy))| {
                let tag = line(
                    self.x,
                    self.y,
                    self.tag_style,
                    self.tag_fill,
                    TextAnchor::Start,
                    self.tag_spacing,
                    1.0,
                    item.tag,
                );
                let text = line(
                    self.x,
                    self.y + self.gap,
                    self.line_style,
                    self.line_fill,
                    TextAnchor::Start,
                    0.0,
                    1.0,
                    item.line,
                );
                let shift = Transform::translate(0.0, dy);
                fframes::svgr!(<g opacity={opacity} transform={shift}>{tag}{text}</g>)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_caption_hands_over_to_the_next() {
        let stack = CaptionStack {
            items: vec![Caption::new(0.0, "1", "one"), Caption::new(2.0, "2", "two")],
            enter: 0.5,
            leave: 0.4,
            ..CaptionStack::default()
        };
        assert_eq!(stack.state(0, 0.0).0, 0.0);
        assert_eq!(stack.state(0, 1.0), (1.0, 0.0));
        assert_eq!(stack.state(0, 2.0).0, 0.0);
        assert_eq!(stack.state(1, 1.9).0, 0.0);
        // the last one stays
        assert_eq!(stack.state(1, 60.0), (1.0, 0.0));
    }
}
