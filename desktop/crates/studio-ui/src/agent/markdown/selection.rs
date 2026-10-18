//! Text selection across the blocks of one rendered markdown row.
//!
//! gpui has no selection that spans several text elements, so the row keeps its own: every text
//! element of the row has an index in document order and registers its layout when it paints. A
//! drag resolves to an `(element, byte offset)` anchor and head; the elements between them are
//! selected whole, and copying joins the pieces with a newline. The state lives in the row, so
//! only one row's selection exists at a time (a press anywhere else ends it).

use std::ops::Range;
use std::rc::Rc;

use gpui::{Bounds, Pixels, Point, SharedString, TextLayout, point};

use super::inline::LinkSpan;

/// A press and its release within this distance (in pixels) is a click, not a drag.
pub const CLICK_SLOP: f32 = 4.0;

/// What a painted text element tells the selection about itself.
pub struct Entry {
    pub layout: TextLayout,
    pub bounds: Bounds<Pixels>,
    pub links: Rc<Vec<LinkSpan>>,
}

#[derive(Default)]
pub struct Selection {
    texts: Vec<SharedString>,
    entries: Vec<Option<Entry>>,
    anchor: Option<(usize, usize)>,
    head: (usize, usize),
    dragging: bool,
    press: Option<Point<Pixels>>,
    /// Whether the pointer was over a link at the last move, to repaint the cursor on change.
    pub hover_link: bool,
}

fn floor_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// The word around `index`; a character that is not part of a word is a word on its own.
pub fn word_at(text: &str, index: usize) -> Range<usize> {
    let is_word = |ch: char| ch.is_alphanumeric() || ch == '_';
    let index = floor_boundary(text, index);
    let Some(ch) = text[index..].chars().next() else {
        return index..index;
    };
    if !is_word(ch) {
        return index..index + ch.len_utf8();
    }
    let start = text[..index]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word(*c))
        .last()
        .map_or(index, |(i, _)| i);
    let end = text[index..]
        .char_indices()
        .find(|(_, c)| !is_word(*c))
        .map_or(text.len(), |(i, _)| index + i);
    start..end
}

impl Selection {
    /// Replaces the document-ordered texts of the row. Called on every build.
    pub fn set_texts(&mut self, texts: Vec<SharedString>) {
        self.texts = texts;
    }

    /// Forgets last frame's geometry; the elements register again while painting.
    pub fn begin_frame(&mut self) {
        self.entries.clear();
        self.entries.resize_with(self.texts.len(), || None);
    }

    pub fn register(&mut self, ix: usize, entry: Entry) {
        if let Some(slot) = self.entries.get_mut(ix) {
            *slot = Some(entry);
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// The selected byte range of element `ix`.
    pub fn range_for(&self, ix: usize) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let (start, end) = if anchor <= self.head {
            (anchor, self.head)
        } else {
            (self.head, anchor)
        };
        if start == end || ix < start.0 || ix > end.0 {
            return None;
        }
        let text = self.texts.get(ix)?;
        let from = if ix == start.0 { start.1 } else { 0 };
        let to = if ix == end.0 { end.1 } else { text.len() };
        let (from, to) = (floor_boundary(text, from), floor_boundary(text, to));
        (from < to || (ix > start.0 && ix < end.0)).then_some(from..to)
    }

    /// The selected text, pieces of different elements joined by a newline.
    pub fn text(&self) -> Option<String> {
        let mut out = String::new();
        let mut any = false;
        for (ix, text) in self.texts.iter().enumerate() {
            if let Some(range) = self.range_for(ix) {
                if any {
                    out.push('\n');
                }
                out.push_str(&text[range]);
                any = true;
            }
        }
        (any && !out.is_empty()).then_some(out)
    }

    /// Ends the selection; whether there was one.
    pub fn clear(&mut self) -> bool {
        let had = self.anchor.is_some() || self.dragging;
        self.anchor = None;
        self.dragging = false;
        self.press = None;
        had
    }

    /// The `(element, byte offset)` nearest to `position`: inside an element when the pointer is
    /// over one, otherwise the closest element vertically.
    pub fn hit(&self, position: Point<Pixels>) -> Option<(usize, usize)> {
        let mut best: Option<(usize, f32)> = None;
        for (ix, entry) in self.entries.iter().enumerate() {
            let Some(entry) = entry else { continue };
            let b = entry.bounds;
            let distance = if position.y < b.top() {
                f32::from(b.top() - position.y)
            } else if position.y > b.bottom() {
                f32::from(position.y - b.bottom())
            } else {
                0.0
            };
            if best.is_none_or(|(_, d)| distance < d) {
                best = Some((ix, distance));
            }
        }
        let (ix, _) = best?;
        let entry = self.entries[ix].as_ref()?;
        let index = entry
            .layout
            .index_for_position(position)
            .unwrap_or_else(|nearest| nearest);
        Some((ix, floor_boundary(self.texts.get(ix)?, index)))
    }

    /// The destination of the link under `position`, if the pointer is exactly on one.
    pub fn link_at(&self, position: Point<Pixels>) -> Option<String> {
        self.entries.iter().flatten().find_map(|entry| {
            if entry.links.is_empty() || !entry.bounds.contains(&position) {
                return None;
            }
            let index = entry.layout.index_for_position(position).ok()?;
            entry
                .links
                .iter()
                .find(|l| l.range.contains(&index))
                .map(|l| l.dest.clone())
        })
    }

    /// A left press at `position`: a click places the caret, a double click selects the word,
    /// a triple click the element. Returns whether the selection changed.
    pub fn press(&mut self, position: Point<Pixels>, click_count: usize) -> bool {
        let Some((ix, offset)) = self.hit(position) else {
            return self.clear();
        };
        self.press = Some(position);
        let text = self.texts[ix].clone();
        match click_count {
            0 | 1 => {
                self.anchor = Some((ix, offset));
                self.head = (ix, offset);
                self.dragging = true;
            }
            2 => {
                let word = word_at(&text, offset);
                self.anchor = Some((ix, word.start));
                self.head = (ix, word.end);
                self.dragging = false;
            }
            _ => {
                self.anchor = Some((ix, 0));
                self.head = (ix, text.len());
                self.dragging = false;
            }
        }
        true
    }

    /// Extends the drag to `position`; whether the head moved.
    pub fn drag_to(&mut self, position: Point<Pixels>) -> bool {
        if !self.dragging {
            return false;
        }
        match self.hit(position) {
            Some(head) if head != self.head => {
                self.head = head;
                true
            }
            _ => false,
        }
    }

    /// A release at `position`. Returns the link destination when the press and release were
    /// one click on a link (no drag between them).
    pub fn release(&mut self, position: Point<Pixels>) -> Option<String> {
        self.dragging = false;
        let press = self.press.take()?;
        let moved = point(position.x - press.x, position.y - press.y);
        let far = f32::from(moved.x).hypot(f32::from(moved.y)) >= CLICK_SLOP;
        if far || self.anchor != Some(self.head) {
            return None;
        }
        self.link_at(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(texts: &[&str], anchor: (usize, usize), head: (usize, usize)) -> Selection {
        Selection {
            texts: texts
                .iter()
                .map(|t| SharedString::from(t.to_string()))
                .collect(),
            anchor: Some(anchor),
            head,
            ..Selection::default()
        }
    }

    #[test]
    fn a_selection_inside_one_element_is_its_slice() {
        let s = selection(&["hello world"], (0, 6), (0, 11));
        assert_eq!(s.range_for(0), Some(6..11));
        assert_eq!(s.text().as_deref(), Some("world"));
    }

    #[test]
    fn a_selection_across_blocks_is_whole_in_between_and_joined_by_newlines() {
        let s = selection(&["first block", "middle", "last block"], (0, 6), (2, 4));
        assert_eq!(s.range_for(0), Some(6..11));
        assert_eq!(s.range_for(1), Some(0..6));
        assert_eq!(s.range_for(2), Some(0..4));
        assert_eq!(s.text().as_deref(), Some("block\nmiddle\nlast"));
    }

    #[test]
    fn a_backwards_drag_selects_the_same_text() {
        let forward = selection(&["alpha", "beta"], (0, 2), (1, 2));
        let backward = selection(&["alpha", "beta"], (1, 2), (0, 2));
        assert_eq!(forward.text(), backward.text());
        assert_eq!(forward.text().as_deref(), Some("pha\nbe"));
    }

    #[test]
    fn a_caret_selects_nothing_and_ranges_follow_char_boundaries() {
        let caret = selection(&["abc"], (0, 1), (0, 1));
        assert_eq!(caret.text(), None);
        let s = selection(&["héllo"], (0, 0), (0, 2));
        assert_eq!(s.range_for(0), Some(0..1));
    }

    #[test]
    fn a_blank_element_between_the_ends_keeps_its_newline() {
        let s = selection(&["one", "", "two"], (0, 0), (2, 3));
        assert_eq!(s.text().as_deref(), Some("one\n\ntwo"));
    }

    #[test]
    fn a_double_click_selects_the_word() {
        let text = "copy héllo_wörld, now";
        assert_eq!(&text[word_at(text, 7)], "héllo_wörld");
        assert_eq!(&text[word_at(text, 4)], " ");
        assert_eq!(word_at(text, text.len()), text.len()..text.len());
    }
}
