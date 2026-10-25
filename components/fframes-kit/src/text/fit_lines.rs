//! Remotion's `@remotion/layout-utils` text fitters, measured with the fonts of the video's
//! media: the largest size that keeps a text within some lines, and a box that takes words
//! until it is full.

use crate::{theme::TextStyle, ui::Ui};

use super::width;

/// A text broken into lines at a font size.
#[derive(Debug, Clone, PartialEq)]
pub struct FitLines {
    pub size: usize,
    pub lines: Vec<String>,
}

/// The largest font size up to `style.size` at which `text` breaks into at most `max_lines`
/// lines of `max_width` pixels, in the family and weight of `style`, with those lines
/// (Remotion's `fitTextOnNLines`).
///
/// Lines break at whitespace and words are never split. `None` when the family cannot be
/// measured (system fonts in the editor; draw the text at `style.size` then), or when a word
/// is wider than the box even at size 1.
pub fn fit_text_on_n_lines(
    ui: &Ui<'_, '_, '_>,
    text: &str,
    style: TextStyle,
    max_width: f32,
    max_lines: usize,
) -> Option<FitLines> {
    let measure = |text: &str, size: usize| width(ui, TextStyle { size, ..style }, text);
    fit_with(measure, text, max_width, max_lines, style.size)
}

fn fit_with(
    measure: impl Fn(&str, usize) -> Option<f32>,
    text: &str,
    max_width: f32,
    max_lines: usize,
    max_size: usize,
) -> Option<FitLines> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let fits = |size: usize| -> Option<Option<Vec<String>>> {
        let (lines, widest) = break_lines(&words, |word| measure(word, size), max_width)?;
        Some((lines.len() <= max_lines.max(1) && widest <= max_width).then_some(lines))
    };
    // the fit only gets worse with the size, so search for the last size that fits
    let (mut low, mut high) = (1, max_size.max(1));
    let mut best = fits(low)?.map(|lines| FitLines { size: low, lines })?;
    while low < high {
        let mid = (low + high).div_ceil(2);
        match fits(mid)? {
            Some(lines) => {
                best = FitLines { size: mid, lines };
                low = mid;
            }
            None => high = mid - 1,
        }
    }
    Some(best)
}

/// `words` broken greedily into lines of `max_width`, and the width of the widest line
/// measured whole, as it is drawn.
fn break_lines(
    words: &[&str],
    measure: impl Fn(&str) -> Option<f32>,
    max_width: f32,
) -> Option<(Vec<String>, f32)> {
    let space = measure(" ")?;
    let mut lines = Vec::new();
    let (mut current, mut used) = (String::new(), 0.0);
    for word in words {
        let w = measure(word)?;
        if !current.is_empty() && used + space + w > max_width {
            lines.push(std::mem::take(&mut current));
            used = 0.0;
        }
        if !current.is_empty() {
            current.push(' ');
            used += space;
        }
        current.push_str(word);
        used += w;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    let mut widest = 0.0_f32;
    for line in &lines {
        widest = widest.max(measure(line)?);
    }
    Some((lines, widest))
}

/// What [`TextBox::add`] did with a word (Remotion's `fillTextBox` result).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxFit {
    /// The word did not fit and was not added.
    pub exceeds_box: bool,
    /// The word starts a new line, or would have.
    pub new_line: bool,
}

/// A box `max_width` pixels wide and `max_lines` lines tall that takes words one after the
/// other (Remotion's `fillTextBox`). Give words with their leading space (`" world"`), as
/// transcripts do; a word that starts a line loses it.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBox {
    pub max_width: f32,
    pub max_lines: usize,
    lines: Vec<String>,
    line_width: f32,
}

/// An empty [`TextBox`].
pub fn fill_text_box(max_width: f32, max_lines: usize) -> TextBox {
    TextBox {
        max_width,
        max_lines,
        lines: Vec::new(),
        line_width: 0.0,
    }
}

impl TextBox {
    /// Adds `text` in `style` to the last line, or to a new one when it does not fit there.
    /// A word that needs a line more than `max_lines`, or is wider than the box, is not
    /// added. `None` when the family cannot be measured.
    pub fn add(&mut self, ui: &Ui<'_, '_, '_>, style: TextStyle, text: &str) -> Option<BoxFit> {
        self.add_with(text, |text| width(ui, style, text))
    }

    /// The lines so far.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    fn add_with(&mut self, text: &str, measure: impl Fn(&str) -> Option<f32>) -> Option<BoxFit> {
        if let Some(last) = self.lines.last_mut() {
            let w = measure(text)?;
            if self.line_width + w <= self.max_width {
                last.push_str(text);
                self.line_width += w;
                return Some(BoxFit {
                    exceeds_box: false,
                    new_line: false,
                });
            }
        }
        let new_line = !self.lines.is_empty();
        let text = text.trim_start();
        let w = measure(text)?;
        if self.lines.len() >= self.max_lines || w > self.max_width {
            return Some(BoxFit {
                exceeds_box: true,
                new_line,
            });
        }
        self.lines.push(text.to_owned());
        self.line_width = w;
        Some(BoxFit {
            exceeds_box: false,
            new_line,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A monospaced font: every character is 0.6 of the size wide.
    fn mono(text: &str, size: usize) -> f32 {
        text.chars().count() as f32 * size as f32 * 0.6
    }

    /// mono as a measure that always finds the font.
    fn font() -> impl Fn(&str, usize) -> Option<f32> {
        |text, size| Some(mono(text, size))
    }

    const TEXT: &str =
        "Remotion measures text so that captions never leave the frame they are given";

    #[test]
    fn the_largest_size_never_overflows() {
        for (max_width, max_lines) in [(300.0, 1), (600.0, 2), (900.0, 3), (1600.0, 2), (120.0, 9)]
        {
            let fit = fit_with(font(), TEXT, max_width, max_lines, 400).expect("fits at size 1");
            assert!(
                fit.lines.len() <= max_lines,
                "{max_width} {max_lines}: {fit:?}"
            );
            for line in &fit.lines {
                let w = mono(line, fit.size);
                assert!(w <= max_width, "{line:?} is {w} wide at {}", fit.size);
            }
            assert_eq!(fit.lines.join(" "), TEXT);
            let bigger = fit_with(font(), TEXT, max_width, max_lines, fit.size + 1);
            assert_eq!(
                bigger.map(|f| f.size),
                Some(fit.size),
                "size + 1 does not fit"
            );
        }
    }

    #[test]
    fn the_size_stops_at_the_style() {
        let fit = fit_with(font(), "Short", 1600.0, 1, 48).expect("fits");
        assert_eq!((fit.size, fit.lines), (48, vec!["Short".to_owned()]));
    }

    #[test]
    fn nothing_fits_without_a_font_or_room() {
        assert_eq!(fit_with(|_, _| None, TEXT, 600.0, 2, 80), None);
        assert_eq!(fit_with(font(), "unbreakable", 3.0, 4, 80), None);
    }

    #[test]
    fn a_box_takes_words_until_it_is_full() {
        // ten characters to a line
        let measure = |text: &str| font()(text, 10);
        let fit = |exceeds_box, new_line| BoxFit {
            exceeds_box,
            new_line,
        };
        let mut text_box = fill_text_box(60.0, 2);
        let mut add = |word| text_box.add_with(word, measure).expect("measured");
        assert_eq!(add("Hello"), fit(false, false));
        assert_eq!(add(" big"), fit(false, false));
        assert_eq!(add(" wide"), fit(false, true));
        assert_eq!(add(" worlds"), fit(true, true));
        assert_eq!(add(" x"), fit(false, false), "room left on the line");
        assert_eq!(text_box.lines(), ["Hello big", "wide x"]);

        let mut narrow = fill_text_box(60.0, 2);
        assert_eq!(
            narrow.add_with("abcdefghijk", measure),
            Some(fit(true, false))
        );
        assert_eq!(narrow.lines(), [] as [String; 0]);
    }
}
