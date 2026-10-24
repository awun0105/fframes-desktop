//! Source code with colours from a small tokenizer and a highlighted line that follows the
//! steps of another figure.

use std::borrow::Cow;

use fframes::{Color, Svgr};
use kurbo::RoundedRect;

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::keyed::locate;

/// The language of a [`CodeBlock`], for its keywords and comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    /// Pseudocode: `if`, `then`, `for`, `while`, `return`… and `//` comments.
    Pseudo,
    /// No colours.
    Plain,
}

impl Language {
    fn keywords(self) -> &'static [&'static str] {
        match self {
            Language::Rust => &[
                "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
                "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match",
                "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
                "super", "trait", "true", "type", "unsafe", "use", "where", "while",
            ],
            Language::Python => &[
                "and", "as", "assert", "async", "await", "break", "class", "continue", "def",
                "del", "elif", "else", "except", "False", "finally", "for", "from", "global", "if",
                "import", "in", "is", "lambda", "None", "nonlocal", "not", "or", "pass", "raise",
                "return", "True", "try", "while", "with", "yield",
            ],
            Language::JavaScript => &[
                "async",
                "await",
                "break",
                "case",
                "catch",
                "class",
                "const",
                "continue",
                "default",
                "delete",
                "do",
                "else",
                "export",
                "extends",
                "false",
                "finally",
                "for",
                "function",
                "if",
                "import",
                "in",
                "instanceof",
                "let",
                "new",
                "null",
                "of",
                "return",
                "super",
                "switch",
                "this",
                "throw",
                "true",
                "try",
                "typeof",
                "undefined",
                "var",
                "void",
                "while",
                "yield",
            ],
            Language::Pseudo => &[
                "if",
                "then",
                "else",
                "end",
                "for",
                "each",
                "to",
                "in",
                "do",
                "while",
                "repeat",
                "until",
                "return",
                "function",
                "procedure",
                "and",
                "or",
                "not",
                "true",
                "false",
                "nil",
            ],
            Language::Plain => &[],
        }
    }

    fn comment(self) -> Option<&'static str> {
        match self {
            Language::Python => Some("#"),
            Language::Plain => None,
            _ => Some("//"),
        }
    }
}

/// What a piece of a line is, for its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Text,
    Keyword,
    String,
    Number,
    Comment,
    /// A name called like a function, or a Rust macro.
    Function,
    /// A capitalised name.
    Type,
}

/// Splits one line into coloured pieces. Strings and comments end with the line; a Rust `'`
/// opens a character only when it closes within the next three characters, so lifetimes stay
/// text.
pub fn tokenize(source: &str, language: Language) -> Vec<(Token, &str)> {
    if language == Language::Plain {
        return vec![(Token::Text, source)];
    }
    let keywords = language.keywords();
    let comment = language.comment();
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let at = |i: usize| chars.get(i).map_or(source.len(), |&(b, _)| b);
    // byte ranges; neighbouring plain text joins into one piece
    let mut out: Vec<(Token, usize, usize)> = Vec::new();
    let mut push = |kind: Token, from: usize, to: usize| match out.last_mut() {
        Some((Token::Text, _, end)) if kind == Token::Text => *end = to,
        _ => out.push((kind, from, to)),
    };
    let mut i = 0;
    while i < chars.len() {
        let (byte, c) = chars[i];
        if comment.is_some_and(|m| source[byte..].starts_with(m)) {
            push(Token::Comment, byte, source.len());
            break;
        }
        let quote = c == '"'
            || (c == '\'' && language != Language::Rust)
            || (c == '\'' && (2..=3).any(|k| chars.get(i + k).is_some_and(|&(_, q)| q == '\'')));
        if quote {
            let mut j = i + 1;
            while j < chars.len() && chars[j].1 != c {
                j += if chars[j].1 == '\\' { 2 } else { 1 };
            }
            let end = (j + 1).min(chars.len());
            push(Token::String, byte, at(end));
            i = end;
        } else if c.is_ascii_digit() {
            let mut j = i;
            while j < chars.len()
                && (chars[j].1.is_ascii_alphanumeric() || matches!(chars[j].1, '.' | '_'))
            {
                j += 1;
            }
            push(Token::Number, byte, at(j));
            i = j;
        } else if c.is_alphabetic() || c == '_' {
            let mut j = i;
            while j < chars.len() && (chars[j].1.is_alphanumeric() || chars[j].1 == '_') {
                j += 1;
            }
            let word = &source[byte..at(j)];
            let next = chars[j..].iter().find(|(_, c)| *c != ' ').map(|&(_, c)| c);
            let kind = if keywords.contains(&word) {
                Token::Keyword
            } else if next == Some('(') || (language == Language::Rust && next == Some('!')) {
                Token::Function
            } else if word.starts_with(char::is_uppercase) {
                Token::Type
            } else {
                Token::Text
            };
            push(kind, byte, at(j));
            i = j;
        } else {
            push(Token::Text, byte, at(i + 1));
            i += 1;
        }
    }
    out.into_iter()
        .map(|(kind, from, to)| (kind, &source[from..to]))
        .collect()
}

/// The colours of a [`CodeBlock`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CodeColors {
    pub text: Color,
    pub keyword: Color,
    pub string: Color,
    pub number: Color,
    pub comment: Color,
    pub function: Color,
    pub kind: Color,
    pub line_number: Color,
    pub background: Color,
    /// The bar behind the highlighted line.
    pub highlight: Color,
}

impl Themed for CodeColors {
    fn themed(t: &Theme) -> Self {
        Self {
            text: t.colors.text,
            keyword: Color::hex("#c678dd"),
            string: Color::hex("#98c379"),
            number: Color::hex("#d19a66"),
            comment: Color::hex("#7f848e"),
            function: Color::hex("#61afef"),
            kind: Color::hex("#e5c07b"),
            line_number: t.colors.muted,
            background: t.colors.surface,
            highlight: t.colors.accent,
        }
    }
}

impl Default for CodeColors {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl CodeColors {
    fn of(&self, token: Token) -> Color {
        match token {
            Token::Text => self.text,
            Token::Keyword => self.keyword,
            Token::String => self.string,
            Token::Number => self.number,
            Token::Comment => self.comment,
            Token::Function => self.function,
            Token::Type => self.kind,
        }
    }
}

/// The characters of `source` that fit in `room` columns, ending with `…` when it is cut.
fn fit(source: &str, room: usize) -> Cow<'_, str> {
    if source.chars().count() <= room {
        return Cow::Borrowed(source);
    }
    let kept: String = source.chars().take(room.saturating_sub(1)).collect();
    Cow::Owned(kept + "…")
}

/// Source code on a panel, coloured by [`tokenize`] for `language`, in a monospaced `style`,
/// with line numbers, centred on `center` (units of `ui.space`). The panel is as wide as the
/// longest line, or `width` pixels when set, and longer lines end with `…`. Tabs count as four
/// spaces. `lines` lights one line (counted from 1, 0 for none) a step, one step every
/// `step_time` seconds from `delay`, so it can follow another figure's trace with the same
/// timing: the bar slides to the line and the other lines dim to `dim`. The whole figure
/// fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct CodeBlock<'a> {
    pub code: Cow<'a, str>,
    pub language: Language,
    pub lines: Vec<usize>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub width: f32,
    pub padding: f32,
    pub line_numbers: bool,
    pub dim: f32,
    pub style: TextStyle,
    pub colors: CodeColors,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for CodeBlock<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            code: Cow::Borrowed(""),
            language: Language::Rust,
            lines: Vec::new(),
            delay: 0.0,
            step_time: 0.8,
            center: (0.0, 0.0),
            width: 0.0,
            padding: 28.0,
            line_numbers: true,
            dim: 0.55,
            style: TextStyle {
                family: "JetBrains Mono",
                size: 28,
                weight: 400,
                line_height: 1.5,
            },
            colors: CodeColors::themed(t),
            enter: None,
            exit: None,
        }
    }
}

impl Default for CodeBlock<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl CodeBlock<'_> {
    /// The line lit `seconds` into the block's time and the one before it, with how far
    /// into the step.
    fn lit(&self, seconds: f32) -> Option<(usize, usize, f32)> {
        let (index, into) = locate(self.lines.len(), seconds - self.delay, self.step_time)?;
        let now = self.lines[index];
        let before = if index == 0 {
            now
        } else {
            self.lines[index - 1]
        };
        Some((before, now, into))
    }
}

impl<'a> Component<'a> for CodeBlock<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let source: Vec<String> = self.code.lines().map(|l| l.replace('\t', "    ")).collect();
        if source.is_empty() {
            return Svgr::empty();
        }
        let size = self.style.size as f64;
        // measured at a large size, as widths come in whole pixels
        let big = TextStyle {
            size: 1000,
            ..self.style
        };
        let advance = ui
            .measure(big.font(), "m")
            .map_or(size * 0.6, |w| f64::from(w) * size / 1000.0);
        let row = size * f64::from(self.style.line_height);
        let pad = f64::from(self.padding);
        let gutter = if self.line_numbers {
            (source.len().to_string().len() as f64 + 1.5) * advance
        } else {
            0.0
        };
        let longest = source.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let inner = if self.width > 0.0 {
            (f64::from(self.width) - 2.0 * pad - gutter).max(advance)
        } else {
            longest as f64 * advance
        };
        let room = (inner / advance).floor() as usize;
        let (w, h) = (
            inner + gutter + 2.0 * pad,
            source.len() as f64 * row + 2.0 * pad,
        );
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (left, top) = (middle.x - w / 2.0, middle.y - h / 2.0);
        let baseline = |k: usize| top + pad + k as f64 * row + row / 2.0 + size * 0.35;

        let panel = Shape {
            path: kurbo::Shape::to_path(&RoundedRect::new(left, top, left + w, top + h, 16.0), 0.1),
            fill: Some(self.colors.background),
            stroke: Some(self.colors.text.with_alpha(28)),
            stroke_width: 1.5,
            dash: None,
            opacity: 1.0,
        }
        .draw();

        // the bar slides from the line before to the lit one; 0 is no line
        let lit = self.lit(ui.seconds()).map(|(before, now, into)| {
            let move_time = (self.step_time * 0.4).min(0.3);
            (before, now, cubic_in_out(progress(into, 0.0, move_time)))
        });
        // the other lines dim while a line is lit
        let focus = |k: usize| {
            lit.map_or(1.0, |(before, now, p)| {
                let on = |line: usize| {
                    if line == 0 || line == k + 1 {
                        1.0
                    } else {
                        self.dim
                    }
                };
                on(before) + (on(now) - on(before)) * p
            })
        };
        let mut bar = Svgr::empty();
        if let Some((before, now, p)) = lit {
            let shown = |line: usize| if line > 0 { 1.0 } else { 0.0 };
            let glow = shown(before) + (shown(now) - shown(before)) * p;
            let row_of = |line: usize, other: usize| {
                let line = if line == 0 { other } else { line };
                line.saturating_sub(1) as f64
            };
            let k =
                row_of(before, now) + (row_of(now, before) - row_of(before, now)) * f64::from(p);
            let y = top + pad + k * row;
            let (x0, x1) = (left + 8.0, left + w - 8.0);
            let back = Shape {
                path: kurbo::Shape::to_path(&RoundedRect::new(x0, y, x1, y + row, 8.0), 0.1),
                fill: Some(self.colors.highlight.with_alpha(56)),
                stroke: None,
                stroke_width: 0.0,
                dash: None,
                opacity: glow,
            };
            let edge = Shape {
                path: kurbo::Shape::to_path(&RoundedRect::new(x0, y, x0 + 6.0, y + row, 3.0), 0.1),
                fill: Some(self.colors.highlight),
                stroke: None,
                stroke_width: 0.0,
                dash: None,
                opacity: glow,
            };
            bar = fframes::svgr!(<g>{back.draw()}{edge.draw()}</g>);
        }

        let mut text = Vec::new();
        for (k, source_line) in source.iter().enumerate() {
            let y = baseline(k) as f32;
            let opacity = focus(k);
            if self.line_numbers {
                text.push(line(
                    (left + pad + gutter - advance * 1.5) as f32,
                    y,
                    self.style,
                    self.colors.line_number,
                    TextAnchor::End,
                    0.0,
                    opacity,
                    (k + 1).to_string().into(),
                ));
            }
            let shown = fit(source_line, room);
            let mut column = 0_usize;
            // every run without spaces at its own column, so no renderer collapses the spaces
            for (token, piece) in tokenize(&shown, self.language) {
                let mut run = String::new();
                let mut start = column;
                for c in piece.chars().chain([' ']) {
                    if c == ' ' {
                        if !run.is_empty() {
                            text.push(line(
                                (left + pad + gutter + start as f64 * advance) as f32,
                                y,
                                self.style,
                                self.colors.of(token),
                                TextAnchor::Start,
                                0.0,
                                opacity,
                                std::mem::take(&mut run).into(),
                            ));
                        }
                        start = column + 1;
                    } else {
                        run.push(c);
                    }
                    column += 1;
                }
                // the space added to flush the last run is not in the line
                column -= 1;
            }
        }
        let parts: Svgr<'a> = vec![panel, bar, text.into()].into();
        fade(ui, parts, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str, language: Language) -> Vec<(Token, &str)> {
        tokenize(line, language)
            .into_iter()
            .filter(|(_, piece)| !piece.trim().is_empty())
            .collect()
    }

    #[test]
    fn rust_lines() {
        let t = kinds("let x: Vec<u8> = vec![1, 2]; // two", Language::Rust);
        assert_eq!(t[0], (Token::Keyword, "let"));
        assert!(t.contains(&(Token::Type, "Vec")));
        assert!(t.contains(&(Token::Function, "vec")));
        assert!(t.contains(&(Token::Number, "1")));
        assert_eq!(t.last(), Some(&(Token::Comment, "// two")));
        // a lifetime is not a character
        let t = kinds("fn f(s: &'a str) -> char { 'x' }", Language::Rust);
        assert!(t.contains(&(Token::Function, "f")));
        assert!(t.contains(&(Token::String, "'x'")));
        assert!(
            !t.iter()
                .any(|(k, p)| *k == Token::String && p.contains("a>"))
        );
    }

    #[test]
    fn python_lines() {
        let t = kinds(
            "    if a[mid] == \"x\": return mid  # done",
            Language::Python,
        );
        assert_eq!(t[0], (Token::Keyword, "if"));
        assert!(t.contains(&(Token::String, "\"x\"")));
        assert!(t.contains(&(Token::Keyword, "return")));
        assert_eq!(t.last(), Some(&(Token::Comment, "# done")));
        // pieces cover the whole line
        let line = "def f(a, b='q\\'s'):";
        let joined: String = tokenize(line, Language::Python)
            .iter()
            .map(|(_, p)| *p)
            .collect();
        assert_eq!(joined, line);
    }

    #[test]
    fn long_lines_are_cut() {
        assert_eq!(fit("abcdef", 6), "abcdef");
        assert_eq!(fit("abcdefg", 6), "abcde…");
    }

    #[test]
    fn the_lit_line_follows_the_steps() {
        let block = CodeBlock {
            lines: vec![1, 3, 0],
            step_time: 1.0,
            ..CodeBlock::default()
        };
        assert_eq!(block.lit(-1.0), Some((1, 1, 0.0)));
        assert_eq!(block.lit(1.5), Some((1, 3, 0.5)));
        assert_eq!(block.lit(9.0).map(|l| (l.0, l.1)), Some((3, 0)));
        assert_eq!(CodeBlock::default().lit(1.0), None);
    }
}
