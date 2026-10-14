//! Pure text helpers: graphemes, words and lines.

use super::*;

// -- Pure text helpers, unit-tested -------------------------------------------

pub(super) fn previous_grapheme(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .rev()
        .find_map(|(index, _)| (index < offset).then_some(index))
        .unwrap_or(0)
}

pub(super) fn next_grapheme(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .find_map(|(index, _)| (index > offset).then_some(index))
        .unwrap_or(text.len())
}

pub(super) fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// `alt-left`: skip any run of non-word characters, then the word itself.
pub(crate) fn word_start_before(text: &str, offset: usize) -> usize {
    let mut index = offset.min(text.len());
    while let Some((start, ch)) = text[..index].char_indices().next_back() {
        if is_word_char(ch) {
            break;
        }
        index = start;
    }
    while let Some((start, ch)) = text[..index].char_indices().next_back() {
        if !is_word_char(ch) {
            break;
        }
        index = start;
    }
    index
}

/// `alt-right`: skip any run of non-word characters, then the word itself.
pub(crate) fn word_end_after(text: &str, offset: usize) -> usize {
    let mut index = offset.min(text.len());
    while let Some(ch) = text[index..].chars().next() {
        if is_word_char(ch) {
            break;
        }
        index += ch.len_utf8();
    }
    while let Some(ch) = text[index..].chars().next() {
        if !is_word_char(ch) {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

/// The word a double-click selects. An offset inside whitespace selects that
/// run of whitespace, which is what `NSTextView` does.
pub(crate) fn word_at(text: &str, offset: usize) -> Range<usize> {
    if text.is_empty() {
        return 0..0;
    }
    let offset = offset.min(text.len());
    // Sitting at the very end selects the last word rather than nothing.
    let probe = if offset == text.len() {
        previous_grapheme(text, offset)
    } else {
        offset
    };
    let Some(ch) = text[probe..].chars().next() else {
        return offset..offset;
    };
    let word = is_word_char(ch);
    let mut start = probe;
    while let Some((index, ch)) = text[..start].char_indices().next_back() {
        if is_word_char(ch) != word {
            break;
        }
        start = index;
    }
    let mut end = probe;
    while let Some(ch) = text[end..].chars().next() {
        if is_word_char(ch) != word {
            break;
        }
        end += ch.len_utf8();
    }
    start..end
}

/// The start of the hard line (paragraph) containing `offset`.
///
/// `cmd-left`, `cmd-backspace` and a triple-click work on the *paragraph*, not
/// on the soft-wrapped visual row: the two agree in every single-line field,
/// and in the teleprompter the paragraph is the unit a script is written in.
pub(crate) fn paragraph_start(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    text[..offset]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0)
}

/// The end of the hard line containing `offset`, excluding the newline.
pub(crate) fn paragraph_end(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    text[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(text.len())
}

/// Shift-extend: move whichever end of the selection the caret is on, and flip
/// the direction when it crosses the anchor.
pub(crate) fn extend_selection(
    range: Range<usize>,
    reversed: bool,
    to: usize,
) -> (Range<usize>, bool) {
    let mut range = range;
    let mut reversed = reversed;
    if reversed {
        range.start = to;
    } else {
        range.end = to;
    }
    if range.end < range.start {
        reversed = !reversed;
        range = range.end..range.start;
    }
    (range, reversed)
}

/// A single-line field never holds a newline, the way an `<input>` never does:
/// a pasted line break becomes a space.
pub(crate) fn flatten_single_line(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

/// utf-16 is the input protocol's unit; the content is utf-8.
pub(crate) fn offset_from_utf16(text: &str, offset: usize) -> usize {
    let mut utf8 = 0;
    let mut utf16 = 0;
    for ch in text.chars() {
        if utf16 >= offset {
            break;
        }
        utf16 += ch.len_utf16();
        utf8 += ch.len_utf8();
    }
    utf8
}

pub(crate) fn offset_to_utf16(text: &str, offset: usize) -> usize {
    let mut utf16 = 0;
    let mut utf8 = 0;
    for ch in text.chars() {
        if utf8 >= offset {
            break;
        }
        utf8 += ch.len_utf8();
        utf16 += ch.len_utf16();
    }
    utf16
}
