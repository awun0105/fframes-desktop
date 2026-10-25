//! manim-voiceover's bookmarks: `<bookmark mark="A"/>` tags in a block's text, timed from
//! the block's word timings.

use crate::text::CaptionWord;

/// A bookmark tag of a text: its mark and the number of letters and digits before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mark {
    pub name: String,
    pub offset: usize,
}

/// `text` without its `<bookmark mark="..."/>` tags and with single spaces, and the marks in
/// the order they appear. A tag without a `mark` is dropped; an unclosed tag is kept as text.
pub fn strip_bookmarks(text: &str) -> (String, Vec<Mark>) {
    let (mut plain, mut marks) = (String::new(), Vec::new());
    let mut rest = text;
    while let Some(open) = rest.find("<bookmark") {
        let Some(close) = rest[open..].find('>').map(|c| open + c) else {
            break;
        };
        plain.push_str(&rest[..open]);
        if let Some(name) = attribute(&rest[open..close], "mark") {
            marks.push(Mark {
                name: name.to_owned(),
                offset: letters(&plain),
            });
        }
        rest = &rest[close + 1..];
    }
    plain.push_str(rest);
    (
        plain.split_whitespace().collect::<Vec<_>>().join(" "),
        marks,
    )
}

/// The start in milliseconds of the first word at or after `offset` letters and digits of
/// `words`, or the end of the last word for a mark after all of them. Counting letters
/// rather than words keeps the time right when the transcriber splits or joins words
/// differently from the text. `None` without words.
pub fn bookmark_ms(words: &[CaptionWord], offset: usize) -> Option<f64> {
    let mut before = 0;
    for word in words {
        if before >= offset {
            return Some(word.start_ms);
        }
        before += letters(&word.text);
    }
    words.last().map(|word| word.end_ms)
}

/// The value of `name="..."` (or single quotes) inside a tag.
fn attribute<'t>(tag: &'t str, name: &str) -> Option<&'t str> {
    let at = tag.find(&format!("{name}="))? + name.len() + 1;
    let quote = tag[at..]
        .chars()
        .next()
        .filter(|q| *q == '"' || *q == '\'')?;
    let value = &tag[at + 1..];
    value.find(quote).map(|end| &value[..end])
}

fn letters(text: &str) -> usize {
    text.chars().filter(|c| c.is_alphanumeric()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_stripped_and_counted() {
        let (plain, marks) = strip_bookmarks(
            "Watch the <bookmark mark=\"A\"/>circle,  then <bookmark mark='B' />the square.<bookmark mark=\"end\"/>",
        );
        assert_eq!(plain, "Watch the circle, then the square.");
        let found: Vec<_> = marks.iter().map(|m| (m.name.as_str(), m.offset)).collect();
        assert_eq!(found, [("A", 8), ("B", 18), ("end", 27)]);
    }

    #[test]
    fn broken_tags_do_not_lose_text() {
        let (plain, marks) = strip_bookmarks("a <bookmark/> b <bookmark mark=\"x\"");
        assert_eq!(plain, "a b <bookmark mark=\"x\"");
        assert_eq!(marks, []);
    }

    #[test]
    fn marks_take_the_time_of_the_next_word() {
        let words = [
            CaptionWord::new(" Watch", 0.0, 300.0),
            CaptionWord::new(" the", 300.0, 450.0),
            // the transcriber joined two words of the text
            CaptionWord::new(" circle,then", 450.0, 1200.0),
            CaptionWord::new(" the", 1300.0, 1450.0),
            CaptionWord::new(" square.", 1450.0, 2000.0),
        ];
        assert_eq!(bookmark_ms(&words, 0), Some(0.0));
        assert_eq!(bookmark_ms(&words, 8), Some(450.0));
        assert_eq!(bookmark_ms(&words, 18), Some(1300.0));
        assert_eq!(bookmark_ms(&words, 27), Some(2000.0), "after the last word");
        assert_eq!(bookmark_ms(&[], 3), None);
    }
}
