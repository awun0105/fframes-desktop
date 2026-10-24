use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line, width};

/// Types its text one character at a time, `chars_per_second` from `delay` seconds into the
/// container, with a cursor after the last character (Manim's `TypeWithCursor`). The cursor
/// is steady while typing and blinks every `blink` seconds once the text is complete. With
/// `erase` it deletes the text again at the same rate, from the end, finishing `erase`
/// seconds before the end of the container (Manim's `UntypeWithCursor`). `(x, y)` is the
/// start of the baseline. Without a measurable font there is no cursor.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Typewriter<'a> {
    pub text: Cow<'a, str>,
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub delay: f32,
    pub chars_per_second: f32,
    /// `None` draws no cursor.
    pub cursor: Option<Color>,
    /// Cursor width in pixels; its height is the font size.
    pub cursor_width: f32,
    /// Seconds the cursor is on, then off, after typing; 0 keeps it on.
    pub blink: f32,
    /// Seconds before the end of the container when the erasing is done; `None` keeps the
    /// text.
    pub erase: Option<f32>,
}

impl Themed for Typewriter<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            text: Cow::Borrowed(""),
            x: 0.0,
            y: 0.0,
            style: t.typography.body,
            fill: t.colors.text,
            delay: 0.0,
            chars_per_second: 24.0,
            cursor: Some(t.colors.accent),
            cursor_width: 3.0,
            blink: 0.5,
            erase: None,
        }
    }
}

impl Default for Typewriter<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Typewriter<'_> {
    /// Characters typed `t` seconds into the container.
    pub fn typed(&self, t: f32) -> usize {
        let typed = ((t - self.delay) * self.chars_per_second).max(0.0) as usize;
        typed.min(self.text.chars().count())
    }

    /// The window in seconds in which the text is erased in a container that ends at `end`.
    fn erasing(&self, end: Option<f32>) -> Option<(f32, f32)> {
        let stop = end? - self.erase?;
        let count = self.text.chars().count() as f32;
        Some((stop - count / self.chars_per_second.max(f32::EPSILON), stop))
    }

    /// Characters shown `t` seconds into a container that ends at `end` seconds: typed, less
    /// the erased ones.
    pub fn shown(&self, t: f32, end: Option<f32>) -> usize {
        let typed = self.typed(t);
        let Some((start, _)) = self.erasing(end) else {
            return typed;
        };
        let count = self.text.chars().count();
        let erased = ((t - start) * self.chars_per_second).max(0.0).ceil() as usize;
        typed.min(count.saturating_sub(erased))
    }

    /// Whether the cursor shows `t` seconds into the container; it is steady while erasing.
    pub fn cursor_on(&self, t: f32) -> bool {
        self.cursor_on_until(t, None)
    }

    /// [`cursor_on`](Self::cursor_on) in a container that ends at `end` seconds.
    pub fn cursor_on_until(&self, t: f32, end: Option<f32>) -> bool {
        if t < self.delay {
            return false;
        }
        if let Some((start, stop)) = self.erasing(end)
            && t >= start
        {
            return t < stop;
        }
        let count = self.text.chars().count();
        let done = self.delay + count as f32 / self.chars_per_second.max(f32::EPSILON);
        if t < done || self.blink <= 0.0 {
            return true;
        }
        (((t - done) / self.blink) as usize).is_multiple_of(2)
    }
}

impl<'a> Component<'a> for Typewriter<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let container_end = ui.duration_seconds();
        let end = self
            .text
            .char_indices()
            .nth(self.shown(t, container_end))
            .map_or(self.text.len(), |(i, _)| i);
        let typed = &self.text[..end];
        let text = line(
            self.x,
            self.y,
            self.style,
            self.fill,
            TextAnchor::Start,
            0.0,
            1.0,
            Cow::Borrowed(typed),
        );
        let cursor = match (self.cursor, width(ui, self.style, typed)) {
            (Some(color), Some(w)) if self.cursor_on_until(t, container_end) => {
                let size = self.style.size as f32;
                let x = self.x + w + size * 0.08;
                let y = self.y - size * 0.8;
                fframes::svgr!(<rect x={x} y={y} width={self.cursor_width} height={size} fill={color} />)
            }
            _ => Svgr::empty(),
        };
        fframes::svgr!(<g>{text}{cursor}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_then_blinks() {
        let t = Typewriter {
            text: "abcd".into(),
            delay: 1.0,
            chars_per_second: 4.0,
            blink: 0.5,
            ..Typewriter::default()
        };
        assert_eq!(t.typed(0.5), 0);
        assert_eq!(t.typed(1.5), 2);
        assert_eq!(t.typed(9.0), 4);
        assert!(!t.cursor_on(0.5));
        assert!(t.cursor_on(1.9));
        // typed at 2.0: on for half a second, then off
        assert!(t.cursor_on(2.2));
        assert!(!t.cursor_on(2.7));
        assert!(t.cursor_on(3.1));
    }

    #[test]
    fn erases_from_the_end_before_the_container_ends() {
        let t = Typewriter {
            text: "abcd".into(),
            chars_per_second: 4.0,
            erase: Some(1.0),
            ..Typewriter::default()
        };
        // a 5 second container: erasing runs from 3.0 to 4.0
        let end = Some(5.0);
        assert_eq!(t.shown(2.9, end), 4);
        assert_eq!(t.shown(3.1, end), 3);
        assert_eq!(t.shown(3.6, end), 1);
        assert_eq!(t.shown(4.5, end), 0);
        assert_eq!(t.shown(4.5, None), 4);
        assert!(t.cursor_on_until(3.9, end));
        assert!(!t.cursor_on_until(4.2, end));
    }
}
