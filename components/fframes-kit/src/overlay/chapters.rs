use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    motion::{lerp, spring},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// A list of chapters with a highlight that springs to the chapter playing now (an agenda,
/// the steps of a tutorial). Each chapter is `(start seconds, title)`, in order; before the
/// first start the highlight is on the first row. `(x, y)` is the top left of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct ChapterList<'a> {
    pub chapters: Vec<(f32, Cow<'a, str>)>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub row_height: f32,
    pub gap: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub active_fill: Color,
    pub highlight: Color,
    pub radius: f32,
    /// `(stiffness, damping)` of the highlight's mass-1 spring.
    pub spring: (f32, f32),
}

impl Themed for ChapterList<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            chapters: Vec::new(),
            x: 96.0,
            y: 240.0,
            width: 720.0,
            row_height: 72.0,
            gap: 12.0,
            style: t.typography.body,
            fill: t.colors.muted,
            active_fill: t.colors.title,
            highlight: t.colors.surface,
            radius: t.radius.md,
            // Remotion's chapter spring, mass 0.45, stiffness 130, damping 20, as mass 1
            spring: (130.0 / 0.45, 20.0 / 0.45),
        }
    }
}

impl Default for ChapterList<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl ChapterList<'_> {
    /// The chapter playing at `t`.
    pub fn active(&self, t: f32) -> usize {
        self.chapters
            .iter()
            .rposition(|(start, _)| *start <= t)
            .unwrap_or(0)
    }

    /// The top of the highlight at `t`, in rows from the first (between two rows while it
    /// moves).
    pub fn highlight_row(&self, t: f32) -> f32 {
        let active = self.active(t);
        if active == 0 {
            return 0.0;
        }
        let (k, d) = self.spring;
        let moved = spring(t - self.chapters[active].0, k, d);
        lerp((active - 1) as f32, active as f32, moved)
    }
}

impl<'a> Component<'a> for ChapterList<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let active = self.active(t);
        let step = self.row_height + self.gap;
        let top = self.y + self.highlight_row(t) * step;
        let highlight = fframes::svgr!(
            <rect x={self.x} y={top} width={self.width} height={self.row_height} rx={self.radius} fill={self.highlight} />
        );
        let baseline = self.row_height / 2.0 + self.style.size as f32 * 0.35;
        let rows: Vec<Svgr<'a>> = self
            .chapters
            .into_iter()
            .enumerate()
            .map(|(i, (_, title))| {
                let fill = if i == active {
                    self.active_fill
                } else {
                    self.fill
                };
                line(
                    self.x + self.row_height * 0.4,
                    self.y + i as f32 * step + baseline,
                    self.style,
                    fill,
                    TextAnchor::Start,
                    0.0,
                    1.0,
                    title,
                )
            })
            .collect();
        fframes::svgr!(<g>{highlight}{rows}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_highlight_springs_to_the_current_chapter() {
        let list = ChapterList {
            chapters: vec![(0.0, "a".into()), (2.0, "b".into()), (4.0, "c".into())],
            ..ChapterList::default()
        };
        assert_eq!(list.active(-1.0), 0);
        assert_eq!(list.active(3.0), 1);
        assert_eq!(list.highlight_row(1.0), 0.0);
        assert_eq!(list.highlight_row(2.0), 0.0);
        assert!((list.highlight_row(3.5) - 1.0).abs() < 1e-2);
        assert!(list.highlight_row(4.05) > 1.0 && list.highlight_row(4.05) < 2.0);
    }
}
