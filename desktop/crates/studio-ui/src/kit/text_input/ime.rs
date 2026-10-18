//! The platform input handler: composed text, marked text and the IME caret.

use super::*;

// -- The IME seam -------------------------------------------------------------

impl EntityInputHandler for TextInputState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        let range = self.clamp_to_boundary(range.start)..self.clamp_to_boundary(range.end);
        if range.start > range.end {
            return None;
        }
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    /// The composed result -- and every ordinary typed character, because on
    /// macOS a printable key that no binding matched is handed to the input
    /// context and comes back here rather than through `key_char`.
    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16.map(|range| self.range_from_utf16(&range));
        // One character typed coalesces into the previous undo step; anything
        // longer (a paste, a composed run, dictation) is its own.
        let group = if new_text.chars().count() == 1 && self.selected_range.is_empty() {
            UndoGroup::Typing
        } else {
            UndoGroup::Discrete
        };
        self.replace(range, new_text, group, cx);
    }

    /// Marked (composing) text: `Option+e` then `e` arrives here twice, first
    /// as the marked `´`, then as the committed `é`.
    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        let range = self.clamp_to_boundary(range.start)..self.clamp_to_boundary(range.end);
        let range = range.start..range.end.max(range.start);

        // A composition is one undo step, however many keystrokes build it.
        if self.marked_range.is_none() {
            self.push_undo(UndoGroup::Discrete);
        }

        let mut next = String::with_capacity(self.content.len() + new_text.len());
        next.push_str(&self.content[..range.start]);
        next.push_str(new_text);
        next.push_str(&self.content[range.end..]);
        self.content = next.into();

        self.marked_range =
            (!new_text.is_empty()).then(|| range.start..range.start + new_text.len());
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|utf16| self.range_from_utf16(utf16))
            .map(|selected| range.start + selected.start..range.start + selected.end)
            .unwrap_or_else(|| {
                let caret = range.start + new_text.len();
                caret..caret
            });
        self.selection_reversed = false;
        self.invalidate_layout();
        self.goal_x = None;
        cx.emit(TextInputEvent::Changed);
        cx.notify();
    }

    /// Where the IME's candidate window should sit.
    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let layout = self.layout.clone();
        let layout = layout.borrow();
        let layout = layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        let bounds = self.text_bounds.unwrap_or(element_bounds);
        let row = layout.row_for_offset(range.start);
        let align = self.align_offset(layout, row, bounds.size.width);
        let start_x = bounds.left() + align + layout.x_for_offset(range.start, self.line_height)
            - self.scroll_x;
        let end_x = bounds.left() + align + layout.x_for_offset(range.end, self.line_height)
            - self.scroll_x;
        let top = bounds.top() + self.line_height * row as f32;
        Some(Bounds::from_corners(
            point(start_x, top),
            point(end_x.max(start_x), top + self.line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let offset = self.offset_for_position(position);
        Some(self.offset_to_utf16(offset))
    }

    fn text_length_utf16(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.offset_to_utf16(self.content.len()))
    }

    fn accepts_text_input(&self, _window: &mut Window, _cx: &mut Context<Self>) -> bool {
        !self.disabled
    }
}
